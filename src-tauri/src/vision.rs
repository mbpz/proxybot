//! Vision screenshot UI analyzer module.
//!
//! Calls Claude Vision API to analyze mobile app screenshots
//! and produce component structure JSON for scaffold generation.

use crate::db::DbState;
use base64::Engine;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

// ============================================================================
// Types
// ============================================================================

/// A UI component extracted from Vision analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionComponent {
    pub component_type: String,
    pub text: Option<String>,
    pub position: VisionPosition,
    pub children: Vec<VisionComponent>,
}

/// Position of a component in the screenshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionPosition {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Vision analysis result for a screenshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionAnalysis {
    pub id: i64,
    pub session_id: String,
    pub filename: String,
    pub components: Vec<VisionComponent>,
    pub raw_response: String,
    pub score: f64,
    pub created_at: String,
}

/// Component tree used by scaffold generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentTree {
    pub components: Vec<VisionComponent>,
    pub layout_json: String,
    pub suggested_routes: Vec<String>,
}

// ============================================================================
// API Key
// ============================================================================

fn get_anthropic_api_key() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("CLAUDE_API_KEY"))
        .ok()
}

// ============================================================================
// Vision API Call
// ============================================================================

async fn call_vision_api(image_base64: &str, api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": "claude-sonnet-4-7-20251101",
        "max_tokens": 4096,
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": image_base64
                    }
                },
                {
                    "type": "text",
                    "text": "Analyze this mobile app screenshot. For each UI component, identify its type (button, text, image, card, list, nav, input, etc.), the text content if any, and its approximate position on screen. Output ONLY valid JSON in this exact format with no markdown code blocks or extra text: {\"components\": [{\"component_type\": \"...\", \"text\": \"...\", \"position\": {\"x\": 0, \"y\": 0, \"width\": 0, \"height\": 0}, \"children\": []}]}. Do not include any explanation, only the JSON."
                }
            ]
        }]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Vision API request failed: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("Vision API error {}: {}", status, body));
    }

    #[derive(Deserialize)]
    struct ApiResponse {
        content: Vec<ContentBlock>,
    }
    #[derive(Deserialize)]
    struct ContentBlock {
        #[serde(rename = "type")]
        block_type: String,
        text: Option<String>,
    }

    let api_resp: ApiResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse Vision response: {}", e))?;

    for block in api_resp.content {
        if block.block_type == "text" {
            if let Some(text) = block.text {
                return Ok(text);
            }
        }
    }
    Err("No text content in Vision API response".to_string())
}

fn parse_vision_response(raw: &str) -> Result<Vec<VisionComponent>, String> {
    // Try to extract JSON from the response (may have markdown code blocks)
    let json_str = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_start_matches("\n")
        .trim_end_matches("```")
        .trim_end_matches("\n")
        .trim();

    // Handle case where AI returns raw object instead of array-wrapped
    // e.g. {"components": {"component_type": ...}} -> {"components": [{"component_type": ...}]}
    let normalized = if json_str.starts_with("{\"components\":{")
        || json_str.starts_with("{\"components\": {")
    {
        json_str
            .replace("{\"components\":{", "{\"components\":[{")
            .replace("\"components\": {", "\"components\": [{")
            .replacen("}}", "}]}", 1)
    } else {
        json_str.to_string()
    };

    #[derive(Deserialize)]
    struct VisionResponse {
        components: Vec<VisionComponent>,
    }

    match serde_json::from_str::<VisionResponse>(&normalized) {
        Ok(vr) => Ok(vr.components),
        Err(_) => {
            // Try direct Vec<VisionComponent> parse in case components key is missing
            serde_json::from_str(&normalized)
                .map_err(|e| format!("Failed to parse components JSON: {}", e))
        }
    }
}

// ============================================================================
// Database
// ============================================================================

pub fn init_vision_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS vision_analyses (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id      TEXT NOT NULL,
            filename        TEXT NOT NULL,
            components_json TEXT NOT NULL,
            raw_response    TEXT NOT NULL,
            score           REAL NOT NULL DEFAULT 0.0,
            created_at      TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_vision_analyses_session ON vision_analyses(session_id);
        "#,
    )?;
    Ok(())
}

// ============================================================================
// Internal helpers (testable with a bare &Connection)
// ============================================================================

/// Store a vision analysis row in the database. Returns the created VisionAnalysis.
fn store_vision_analysis_internal(
    conn: &rusqlite::Connection,
    session_id: &str,
    filename: &str,
    components: &[VisionComponent],
    raw_response: &str,
) -> Result<VisionAnalysis, String> {
    let now = crate::db::chrono_lite_timestamp();
    let components_json = serde_json::to_string(components).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO vision_analyses (session_id, filename, components_json, raw_response, score, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![session_id, filename, components_json, raw_response, 0.0, now],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();

    let display_filename = PathBuf::from(filename)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| filename.to_string());

    Ok(VisionAnalysis {
        id,
        session_id: session_id.to_string(),
        filename: display_filename,
        components: components.to_vec(),
        raw_response: raw_response.to_string(),
        score: 0.0,
        created_at: now,
    })
}

/// Get all vision analyses for a session (internal, takes &Connection).
fn get_vision_analyses_internal(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<Vec<VisionAnalysis>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, filename, components_json, raw_response, score, created_at
             FROM vision_analyses WHERE session_id = ?1 ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let analyses = stmt
        .query_map(params![session_id], |row| {
            let components_json: String = row.get(3)?;
            let raw_response: String = row.get(4)?;
            let components: Vec<VisionComponent> =
                serde_json::from_str(&components_json).unwrap_or_default();
            Ok(VisionAnalysis {
                id: row.get(0)?,
                session_id: row.get(1)?,
                filename: row.get(2)?,
                components,
                raw_response,
                score: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(analyses)
}

/// Delete a vision analysis by id (internal, takes &Connection).
fn delete_vision_analysis_internal(
    conn: &rusqlite::Connection,
    id: i64,
) -> Result<(), String> {
    conn.execute("DELETE FROM vision_analyses WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Fuse vision component tree with inferred API to produce enhanced scaffold data (internal).
fn fuse_vision_with_api_internal(
    conn: &rusqlite::Connection,
    session_id: &str,
) -> Result<ComponentTree, String> {
    // Get latest vision analysis
    let vision_components = {
        let mut stmt = conn
            .prepare(
                "SELECT components_json FROM vision_analyses WHERE session_id = ?1 ORDER BY created_at DESC LIMIT 1",
            )
            .map_err(|e| e.to_string())?;

        let result: Result<String, _> = stmt.query_row(params![session_id], |row| row.get(0));

        match result {
            Ok(json) => serde_json::from_str::<Vec<VisionComponent>>(&json).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    };

    // Get inferred APIs for route suggestions
    let suggested_routes = {
        let mut stmt = conn
            .prepare("SELECT path FROM inferred_apis WHERE session_id = ?1 ORDER BY id")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![session_id], |row| {
                let path: String = row.get(0)?;
                Ok(path)
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    // Build layout JSON from component tree
    let layout_json = serde_json::to_string_pretty(&vision_components).unwrap_or_default();

    Ok(ComponentTree {
        components: vision_components,
        layout_json,
        suggested_routes,
    })
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Analyze a screenshot using Claude Vision API.
#[tauri::command]
pub async fn analyze_screenshot(
    db: State<'_, Arc<DbState>>,
    session_id: String,
    image_path: String,
) -> Result<VisionAnalysis, String> {
    let api_key = get_anthropic_api_key().ok_or("ANTHROPIC_API_KEY not set")?;

    // Read image file
    let image_data = fs::read(&image_path).map_err(|e| format!("Failed to read image: {}", e))?;
    let image_base64 = base64::engine::general_purpose::STANDARD.encode(&image_data);

    // Call Vision API
    let raw_response = call_vision_api(&image_base64, &api_key).await?;

    // Parse response
    let components = parse_vision_response(&raw_response)?;

    // Store in database
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    store_vision_analysis_internal(&conn, &session_id, &image_path, &components, &raw_response)
}

/// Analyze screenshot from base64-encoded image data.
#[tauri::command]
pub async fn analyze_screenshot_base64(
    db: State<'_, Arc<DbState>>,
    session_id: String,
    image_data_base64: String,
    filename: String,
) -> Result<VisionAnalysis, String> {
    let api_key = get_anthropic_api_key().ok_or("ANTHROPIC_API_KEY not set")?;

    // Call Vision API
    let raw_response = call_vision_api(&image_data_base64, &api_key).await?;

    // Parse response
    let components = parse_vision_response(&raw_response)?;

    // Store in database
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    store_vision_analysis_internal(&conn, &session_id, &filename, &components, &raw_response)
}

/// Get all vision analyses for a session.
#[tauri::command]
pub fn get_vision_analyses(
    db: State<'_, Arc<DbState>>,
    session_id: String,
) -> Result<Vec<VisionAnalysis>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    get_vision_analyses_internal(&conn, &session_id)
}

/// Delete a vision analysis.
#[tauri::command]
pub fn delete_vision_analysis(db: State<'_, Arc<DbState>>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    delete_vision_analysis_internal(&conn, id)
}

/// Fuse vision component tree with inferred API to produce enhanced scaffold data.
#[tauri::command]
pub fn fuse_vision_with_api(
    db: State<'_, Arc<DbState>>,
    session_id: String,
) -> Result<ComponentTree, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    fuse_vision_with_api_internal(&conn, &session_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Helper: open an in-memory DB with the full schema (including vision_analyses + inferred_apis).
    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::DbState::init_schema(&conn).unwrap();
        conn
    }

    fn sample_component(component_type: &str, text: &str) -> VisionComponent {
        VisionComponent {
            component_type: component_type.to_string(),
            text: Some(text.to_string()),
            position: VisionPosition {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 40.0,
            },
            children: vec![],
        }
    }

    // ------------------------------------------------------------------
    // parse_vision_response — pure function tests
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_vision_response() {
        let raw = r#"{"components": [{"component_type": "button", "text": "Submit", "position": {"x": 10, "y": 20, "width": 100, "height": 40}, "children": []}]}"#;
        let components = parse_vision_response(raw).unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, "button");
        assert_eq!(components[0].text, Some("Submit".to_string()));
        assert_eq!(components[0].position.x, 10.0);
        assert_eq!(components[0].position.y, 20.0);
        assert_eq!(components[0].position.width, 100.0);
        assert_eq!(components[0].position.height, 40.0);
    }

    #[test]
    fn test_parse_vision_response_with_markdown() {
        let raw = "```json\n{\"components\": [{\"component_type\": \"text\", \"text\": \"Hello\", \"position\": {\"x\": 0, \"y\": 0, \"width\": 100, \"height\": 20}, \"children\": []}]}\n```";
        let components = parse_vision_response(raw).unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, "text");
        assert_eq!(components[0].text, Some("Hello".to_string()));
    }

    #[test]
    fn test_parse_vision_response_empty_components_array() {
        let raw = r#"{"components": []}"#;
        let components = parse_vision_response(raw).unwrap();
        assert_eq!(components.len(), 0, "Empty components array should parse to empty vec");
    }

    #[test]
    fn test_parse_vision_response_nested_children() {
        let raw = r#"{"components": [{"component_type": "card", "text": null, "position": {"x": 0, "y": 0, "width": 300, "height": 200}, "children": [{"component_type": "text", "text": "Title", "position": {"x": 10, "y": 10, "width": 280, "height": 30}, "children": []}]}]}"#;
        let components = parse_vision_response(raw).unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, "card");
        assert_eq!(components[0].text, None);
        assert_eq!(components[0].children.len(), 1);
        assert_eq!(components[0].children[0].component_type, "text");
        assert_eq!(components[0].children[0].text, Some("Title".to_string()));
    }

    #[test]
    fn test_parse_vision_response_raw_object_normalization() {
        // When AI returns {"components": {...}} (object instead of array) the normalizer wraps it
        let raw = r#"{"components": {"component_type": "button", "text": "OK", "position": {"x": 5, "y": 5, "width": 50, "height": 20}, "children": []}}"#;
        let components = parse_vision_response(raw).unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, "button");
        assert_eq!(components[0].text, Some("OK".to_string()));
    }

    #[test]
    fn test_parse_vision_response_raw_object_with_space_normalization() {
        // Same normalization but with a space after "components":
        let raw = r#"{"components": {"component_type": "input", "text": "Search", "position": {"x": 0, "y": 0, "width": 200, "height": 40}, "children": []}}"#;
        let components = parse_vision_response(raw).unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, "input");
    }

    #[test]
    fn test_parse_vision_response_direct_array_fallback() {
        // When the response is a bare array without the "components" wrapper
        let raw = r#"[{"component_type": "image", "text": null, "position": {"x": 0, "y": 0, "width": 100, "height": 100}, "children": []}]"#;
        let components = parse_vision_response(raw).unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, "image");
        assert_eq!(components[0].text, None);
    }

    #[test]
    fn test_parse_vision_response_invalid_json_returns_error() {
        let raw = "not json at all {{{";
        let result = parse_vision_response(raw);
        assert!(result.is_err(), "Invalid JSON should return Err");
    }

    #[test]
    fn test_parse_vision_response_multiple_components() {
        let raw = r#"{"components": [
            {"component_type": "nav", "text": "Header", "position": {"x": 0, "y": 0, "width": 375, "height": 60}, "children": []},
            {"component_type": "list", "text": null, "position": {"x": 0, "y": 60, "width": 375, "height": 600}, "children": []},
            {"component_type": "button", "text": "Submit", "position": {"x": 20, "y": 680, "width": 335, "height": 50}, "children": []}
        ]}"#;
        let components = parse_vision_response(raw).unwrap();
        assert_eq!(components.len(), 3);
        assert_eq!(components[0].component_type, "nav");
        assert_eq!(components[1].component_type, "list");
        assert_eq!(components[2].component_type, "button");
        assert_eq!(components[2].text, Some("Submit".to_string()));
    }

    // ------------------------------------------------------------------
    // init_vision_schema
    // ------------------------------------------------------------------

    #[test]
    fn test_init_vision_schema_creates_table_and_index() {
        // Use a bare connection with only the vision schema to verify the DDL is self-contained
        let conn = Connection::open_in_memory().unwrap();
        init_vision_schema(&conn).unwrap();

        // Should be able to insert a row
        conn.execute(
            "INSERT INTO vision_analyses (session_id, filename, components_json, raw_response, score, created_at)
             VALUES ('s1', 'test.png', '[]', 'raw', 0.5, '2026-01-01 00:00:00')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM vision_analyses", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // Verify the index exists by querying with session_id (uses the index)
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM vision_analyses WHERE session_id = 's1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    // ------------------------------------------------------------------
    // store_vision_analysis_internal
    // ------------------------------------------------------------------

    #[test]
    fn test_store_vision_analysis_inserts_row() {
        let conn = test_db();
        let components = vec![sample_component("button", "Click me")];

        let result = store_vision_analysis_internal(
            &conn,
            "session-1",
            "/tmp/screenshot.png",
            &components,
            "raw api response",
        )
        .unwrap();

        assert!(result.id > 0, "Auto-increment id should be > 0");
        assert_eq!(result.session_id, "session-1");
        assert_eq!(result.filename, "screenshot.png", "Should extract just the filename from path");
        assert_eq!(result.components.len(), 1);
        assert_eq!(result.components[0].component_type, "button");
        assert_eq!(result.raw_response, "raw api response");
        assert_eq!(result.score, 0.0);
        assert!(!result.created_at.is_empty());
    }

    #[test]
    fn test_store_vision_analysis_full_path_as_filename() {
        let conn = test_db();
        let result = store_vision_analysis_internal(
            &conn,
            "session-2",
            "no_path_separator.png",
            &[] as &[VisionComponent],
            "raw",
        )
        .unwrap();

        assert_eq!(result.filename, "no_path_separator.png");
    }

    // ------------------------------------------------------------------
    // get_vision_analyses_internal
    // ------------------------------------------------------------------

    #[test]
    fn test_get_vision_analyses_returns_by_session() {
        let conn = test_db();

        store_vision_analysis_internal(&conn, "s1", "a.png", &[], "raw-a").unwrap();
        store_vision_analysis_internal(&conn, "s1", "b.png", &[], "raw-b").unwrap();
        store_vision_analysis_internal(&conn, "s2", "c.png", &[], "raw-c").unwrap();

        let analyses = get_vision_analyses_internal(&conn, "s1").unwrap();
        assert_eq!(analyses.len(), 2, "Session s1 should have 2 analyses");
        // Ordered by created_at DESC — both have the same second-level timestamp so
        // we just check they belong to the right session
        for a in &analyses {
            assert_eq!(a.session_id, "s1");
        }
    }

    #[test]
    fn test_get_vision_analyses_empty_session() {
        let conn = test_db();
        let analyses = get_vision_analyses_internal(&conn, "nonexistent").unwrap();
        assert_eq!(analyses.len(), 0, "Nonexistent session should return empty vec");
    }

    #[test]
    fn test_get_vision_analyses_preserves_components() {
        let conn = test_db();
        let components = vec![
            sample_component("button", "OK"),
            sample_component("text", "Label"),
        ];
        store_vision_analysis_internal(&conn, "s1", "test.png", &components, "raw").unwrap();

        let analyses = get_vision_analyses_internal(&conn, "s1").unwrap();
        assert_eq!(analyses.len(), 1);
        assert_eq!(analyses[0].components.len(), 2);
        assert_eq!(analyses[0].components[0].component_type, "button");
        assert_eq!(analyses[0].components[1].text, Some("Label".to_string()));
    }

    // ------------------------------------------------------------------
    // delete_vision_analysis_internal
    // ------------------------------------------------------------------

    #[test]
    fn test_delete_vision_analysis_removes_row() {
        let conn = test_db();

        let stored = store_vision_analysis_internal(&conn, "s1", "a.png", &[], "raw").unwrap();
        let analyses = get_vision_analyses_internal(&conn, "s1").unwrap();
        assert_eq!(analyses.len(), 1, "Should have 1 analysis before delete");

        delete_vision_analysis_internal(&conn, stored.id).unwrap();

        let analyses = get_vision_analyses_internal(&conn, "s1").unwrap();
        assert_eq!(analyses.len(), 0, "Should have 0 analyses after delete");
    }

    #[test]
    fn test_delete_vision_analysis_nonexistent_id_does_not_error() {
        let conn = test_db();
        // Deleting a non-existent id should succeed silently (0 rows affected)
        delete_vision_analysis_internal(&conn, 99999).unwrap();
    }

    // ------------------------------------------------------------------
    // fuse_vision_with_api_internal
    // ------------------------------------------------------------------

    #[test]
    fn test_fuse_vision_with_api_returns_components_and_routes() {
        let conn = test_db();

        let components = vec![
            sample_component("button", "Login"),
            sample_component("input", "Username"),
        ];
        store_vision_analysis_internal(&conn, "s1", "screen.png", &components, "raw").unwrap();

        // Insert an inferred API to get route suggestions
        conn.execute(
            "INSERT INTO inferred_apis (session_id, name, method, path, params, auth_required, request_ids, score, created_at)
             VALUES ('s1', 'login', 'POST', '/api/login', '{}', 0, '[]', 0.9, '2026-01-01 00:00:00')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inferred_apis (session_id, name, method, path, params, auth_required, request_ids, score, created_at)
             VALUES ('s1', 'users', 'GET', '/api/users', '{}', 1, '[]', 0.8, '2026-01-01 00:00:00')",
            [],
        )
        .unwrap();

        let tree = fuse_vision_with_api_internal(&conn, "s1").unwrap();

        assert_eq!(tree.components.len(), 2);
        assert_eq!(tree.components[0].component_type, "button");
        assert_eq!(tree.suggested_routes.len(), 2);
        assert_eq!(tree.suggested_routes[0], "/api/login");
        assert_eq!(tree.suggested_routes[1], "/api/users");
        assert!(!tree.layout_json.is_empty(), "layout_json should be a non-empty pretty-printed JSON");
    }

    #[test]
    fn test_fuse_vision_with_api_empty_session() {
        let conn = test_db();

        let tree = fuse_vision_with_api_internal(&conn, "nonexistent").unwrap();

        assert_eq!(tree.components.len(), 0, "No vision data should yield empty components");
        assert_eq!(tree.suggested_routes.len(), 0, "No inferred APIs should yield empty routes");
        assert_eq!(tree.layout_json, "[]", "layout_json should be empty array literal");
    }

    #[test]
    fn test_fuse_vision_with_api_returns_latest_analysis() {
        let conn = test_db();

        // Insert two analyses with explicitly different created_at timestamps
        // (chrono_lite_timestamp has second resolution, so same-second inserts
        // would be indeterminate under ORDER BY created_at DESC).
        conn.execute(
            "INSERT INTO vision_analyses (session_id, filename, components_json, raw_response, score, created_at)
             VALUES ('s1', 'old.png', ?1, 'raw-old', 0.0, '2026-01-01 00:00:00')",
            params![serde_json::to_string(&[sample_component("text", "Old")]).unwrap()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO vision_analyses (session_id, filename, components_json, raw_response, score, created_at)
             VALUES ('s1', 'new.png', ?1, 'raw-new', 0.0, '2026-01-02 00:00:00')",
            params![serde_json::to_string(&[sample_component("button", "New")]).unwrap()],
        )
        .unwrap();

        let tree = fuse_vision_with_api_internal(&conn, "s1").unwrap();
        assert_eq!(tree.components.len(), 1);
        assert_eq!(tree.components[0].text, Some("New".to_string()));
    }

    // NOTE: fuse_vision_with_api_internal uses `ORDER BY created_at DESC LIMIT 1`
    // with chrono_lite_timestamp (second resolution). Two inserts in the same
    // second produce the same created_at, making the "latest" selection
    // indeterminate. Production impact is low (sequential user actions rarely
    // land within one second), but a higher-resolution timestamp or id-based
    // ordering would be more robust. See task #89.
}
