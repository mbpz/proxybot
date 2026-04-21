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

    let api_resp: ApiResponse =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse Vision response: {}", e))?;

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
        .trim_end_matches("```")
        .trim();

    #[derive(Deserialize)]
    struct VisionResponse {
        components: Vec<VisionComponent>,
    }

    serde_json::from_str(json_str).map_err(|e| format!("Failed to parse components JSON: {}", e))
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
    let now = crate::db::chrono_lite_timestamp();
    let components_json = serde_json::to_string(&components).map_err(|e| e.to_string())?;

    let id = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO vision_analyses (session_id, filename, components_json, raw_response, score, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, image_path, components_json, raw_response.clone(), 0.0, now],
        )
        .map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };

    let filename = PathBuf::from(&image_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| image_path.clone());

    Ok(VisionAnalysis {
        id,
        session_id,
        filename,
        components,
        raw_response,
        score: 0.0,
        created_at: now,
    })
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
    let now = crate::db::chrono_lite_timestamp();
    let components_json = serde_json::to_string(&components).map_err(|e| e.to_string())?;

    let id = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO vision_analyses (session_id, filename, components_json, raw_response, score, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, filename, components_json, raw_response.clone(), 0.0, now],
        )
        .map_err(|e| e.to_string())?;
        conn.last_insert_rowid()
    };

    Ok(VisionAnalysis {
        id,
        session_id,
        filename,
        components,
        raw_response,
        score: 0.0,
        created_at: now,
    })
}

/// Get all vision analyses for a session.
#[tauri::command]
pub fn get_vision_analyses(
    db: State<'_, Arc<DbState>>,
    session_id: String,
) -> Result<Vec<VisionAnalysis>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
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

/// Delete a vision analysis.
#[tauri::command]
pub fn delete_vision_analysis(db: State<'_, Arc<DbState>>, id: i64) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM vision_analyses WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Fuse vision component tree with inferred API to produce enhanced scaffold data.
#[tauri::command]
pub fn fuse_vision_with_api(
    db: State<'_, Arc<DbState>>,
    session_id: String,
) -> Result<ComponentTree, String> {
    // Get latest vision analysis
    let vision_components = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
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
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT path FROM inferred_apis WHERE session_id = ?1 ORDER BY id")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![session_id], |row| {
                let path: String = row.get(0)?;
                Ok(path)
            })
            .map_err(|e| e.to_string())?;
        let result: Vec<String> = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        result
    };

    // Build layout JSON from component tree
    let layout_json = serde_json::to_string_pretty(&vision_components).unwrap_or_default();

    Ok(ComponentTree {
        components: vision_components,
        layout_json,
        suggested_routes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vision_response() {
        let raw = r#"{"components": [{"component_type": "button", "text": "Submit", "position": {"x": 10, "y": 20, "width": 100, "height": 40}, "children": []}]}"#;
        let components = parse_vision_response(raw).unwrap();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, "button");
        assert_eq!(components[0].text, Some("Submit".to_string()));
    }

    #[test]
    fn test_parse_vision_response_with_markdown() {
        let raw = "```json\n{\"components\": [{\"component_type\": \"text\", \"text\": \"Hello\", \"position\": {\"x\": 0, \"y\": 0, \"width\": 100, \"height\": 20}, \"children\": []}]}\n```";
        let components = parse_vision_response(raw).unwrap();
        assert_eq!(components.len(), 1);
    }
}