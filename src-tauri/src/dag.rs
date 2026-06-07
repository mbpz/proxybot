//! DAG (Directed Acyclic Graph) builder module for traffic analysis.
//!
//! Builds a dependency graph of HTTP requests based on token passing.
//! Edges are created when a later request uses a token that was returned by an earlier request.

use crate::db::DbState;
use regex::Regex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

// ============================================================================
// Token Extraction
// ============================================================================

/// Token patterns to extract from request/response bodies and headers.
/// JSON format: "access_token": "value" or access_token: value
const TOKEN_PATTERNS: &[&str] = &[
    // JSON format: "access_token": "value"
    r#"(?i)"access_token":\s*["']([^"']+)["']"#,
    // Key=Value format: access_token=value or access_token: value
    r#"(?i)access_token[=:]\s*['"]?([a-zA-Z0-9_\-\.+/=]+)['"]?"#,
    // JSON format: "sessionId": "value"
    r#"(?i)"sessionId":\s*["']([^"']+)["']"#,
    // Key=Value format: sessionId=value or sessionId: value
    r#"(?i)sessionId[=:]\s*['"]?([a-zA-Z0-9_\-\.+/=]+)['"]?"#,
    // JSON format: "auth_token": "value"
    r#"(?i)"auth_token":\s*["']([^"']+)["']"#,
    // Key=Value format: auth_token=value or auth_token: value
    r#"(?i)auth_token[=:]\s*['"]?([a-zA-Z0-9_\-\.+/=]+)['"]?"#,
    // JSON format: "id": "value"
    r#"(?i)"id":\s*["']([^"']+)["']"#,
    // JSON format: "uid": "value"
    r#"(?i)"uid":\s*["']([^"']+)["']"#,
    // Key=Value format: uid=value or uid: value
    r#"(?i)uid[=:]\s*['"]?([a-zA-Z0-9_\-\.+/=]+)['"]?"#,
    // JSON format: "refresh_token": "value"
    r#"(?i)"refresh_token":\s*["']([^"']+)["']"#,
    // Key=Value format: refresh_token=value or refresh_token: value
    r#"(?i)refresh_token[=:]\s*['"]?([a-zA-Z0-9_\-\.+/=]+)['"]?"#,
];

/// Extracted token with its source location.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ExtractedToken {
    pub value: String,
    pub source: TokenSource,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum TokenSource {
    RequestHeader,
    ResponseHeader,
    RequestBody,
    ResponseBody,
}

/// DAG node representing a request in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub host: String,
    pub device_id: Option<i64>,
}

/// DAG edge representing a token dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagEdge {
    pub from_node_id: i64,
    pub to_node_id: i64,
    pub token_value: String,
}

/// Complete DAG structure for API response.
#[derive(Debug, Clone, Serialize)]
pub struct TrafficDag {
    pub nodes: Vec<DagNode>,
    pub edges: Vec<DagEdge>,
    pub adjacency_list: HashMap<i64, Vec<i64>>,
}

// ============================================================================
// Token Extraction Functions
// ============================================================================

/// Extract tokens from a string (header value or body text).
fn extract_tokens_from_text(text: &str, source: TokenSource) -> Vec<ExtractedToken> {
    let mut tokens = Vec::new();

    for pattern in TOKEN_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(text) {
                if let Some(matched) = cap.get(1) {
                    let value = matched.as_str().to_string();
                    // Filter out very short tokens or obvious non-tokens
                    if value.len() >= 8 {
                        tokens.push(ExtractedToken {
                            value,
                            source: source.clone(),
                        });
                    }
                }
            }
        }
    }

    tokens
}

/// Extract tokens from JSON value (recursive).
fn extract_tokens_from_json(value: &serde_json::Value, source: TokenSource) -> Vec<ExtractedToken> {
    let mut tokens = Vec::new();

    match value {
        serde_json::Value::String(s) => {
            tokens.extend(extract_tokens_from_text(s, source.clone()));
        }
        serde_json::Value::Object(obj) => {
            for (key, val) in obj {
                // Skip keys that are clearly not tokens
                let key_lower = key.to_lowercase();
                if key_lower.contains("password")
                    || key_lower.contains("secret")
                    || key_lower.contains("key") && !key_lower.contains("token")
                {
                    continue;
                }

                // Check if this key is a token name and extract the value
                if let Some(token_value) = extract_token_from_key_value(key, val) {
                    if token_value.len() >= 8 {
                        tokens.push(ExtractedToken {
                            value: token_value,
                            source: source.clone(),
                        });
                    }
                }

                tokens.extend(extract_tokens_from_json(val, source.clone()));
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                tokens.extend(extract_tokens_from_json(item, source.clone()));
            }
        }
        _ => {}
    }

    tokens
}

/// Check if a key is a token name and extract the value if so.
fn extract_token_from_key_value(key: &str, value: &serde_json::Value) -> Option<String> {
    let key_lower = key.to_lowercase();
    let token_names = [
        "access_token",
        "sessionid",
        "auth_token",
        "id",
        "uid",
        "refresh_token",
    ];

    if token_names.iter().any(|t| key_lower.contains(t)) {
        if let serde_json::Value::String(s) = value {
            return Some(s.clone());
        }
    }
    None
}

/// Extract all tokens from request/response data.
pub fn extract_tokens(
    req_headers: &serde_json::Value,
    req_body: Option<&serde_json::Value>,
    resp_status: u16,
    resp_headers: &serde_json::Value,
    resp_body: Option<&serde_json::Value>,
) -> (Vec<ExtractedToken>, Vec<ExtractedToken>) {
    let mut request_tokens = Vec::new();
    let mut response_tokens = Vec::new();

    // Extract from request headers
    if let Some(obj) = req_headers.as_object() {
        for (name, value) in obj {
            if let Some(s) = value.as_str() {
                // Common auth headers
                let name_lower = name.to_lowercase();
                if name_lower.contains("authorization")
                    || name_lower.contains("cookie")
                    || name_lower.contains("x-token")
                    || name_lower.contains("x-session")
                {
                    request_tokens.extend(extract_tokens_from_text(s, TokenSource::RequestHeader));
                }
            }
        }
    }

    // Extract from request body (only for non-GET requests)
    if let Some(body) = req_body {
        if !body.is_null() {
            request_tokens.extend(extract_tokens_from_json(body, TokenSource::RequestBody));
        }
    }

    // Extract from response headers (only successful responses)
    if (200..400).contains(&resp_status) {
        if let Some(obj) = resp_headers.as_object() {
            for (name, value) in obj {
                if let Some(s) = value.as_str() {
                    let name_lower = name.to_lowercase();
                    if name_lower.contains("set-cookie") || name_lower.contains("authorization") {
                        response_tokens
                            .extend(extract_tokens_from_text(s, TokenSource::ResponseHeader));
                    }
                }
            }
        }

        // Extract from response body
        if let Some(body) = resp_body {
            if !body.is_null() {
                response_tokens.extend(extract_tokens_from_json(body, TokenSource::ResponseBody));
            }
        }
    }

    // Deduplicate
    request_tokens.sort_by(|a, b| a.value.cmp(&b.value));
    request_tokens.dedup();

    response_tokens.sort_by(|a, b| a.value.cmp(&b.value));
    response_tokens.dedup();

    (request_tokens, response_tokens)
}

// ============================================================================
// DAG Building
// ============================================================================

/// Build DAG from all HTTP requests in the database.
pub fn build_dag_from_requests(
    requests: &[(
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        i64,
        Option<i64>,
    )],
) -> TrafficDag {
    let mut nodes: Vec<DagNode> = Vec::new();
    let mut node_by_id: HashMap<i64, usize> = HashMap::new();

    // Collect request data for token extraction
    let mut node_request_data: HashMap<i64, (String, Option<String>, u16)> = HashMap::new();

    // First pass: create nodes and collect request data
    for (id, timestamp, method, host, path, req_headers, req_body, resp_status, device_id) in
        requests
    {
        let node = DagNode {
            id: *id,
            timestamp: timestamp.clone(),
            method: method.clone(),
            path: path.clone(),
            host: host.clone(),
            device_id: *device_id,
        };

        node_by_id.insert(*id, nodes.len());
        nodes.push(node);

        node_request_data.insert(
            *id,
            (
                req_headers.clone().unwrap_or_default(),
                req_body.clone(),
                *resp_status as u16,
            ),
        );
    }

    // Second pass: build edges based on token passing
    let mut edges: Vec<DagEdge> = Vec::new();
    let mut adjacency_list: HashMap<i64, Vec<i64>> = HashMap::new();

    // Token to node mapping (which node produced which token)
    let mut token_producers: HashMap<String, i64> = HashMap::new();

    // Sort nodes by timestamp to ensure proper ordering
    let mut node_indices: Vec<usize> = (0..nodes.len()).collect();
    node_indices.sort_by(|&a, &b| {
        let node_a = &nodes[a];
        let node_b = &nodes[b];
        node_a.timestamp.cmp(&node_b.timestamp)
    });

    // First pass: identify token producers (responses that return tokens)
    // NOTE: Bug — the producer pass hardcodes `resp_body_json: None` and an
    // empty `resp_headers_json`, then only uses the `resp_tokens` half of
    // the returned tuple. Combined with the fact that the `requests` tuple
    // returned by `get_all_requests` does NOT include `resp_body` or
    // `resp_headers` (only `req_headers`, `req_body`, `resp_status`), no
    // request can ever be identified as a token producer. The consumer
    // pass below therefore never finds a producer to link against, and
    // `dag.edges` is always empty. Tracked as task #77.
    for &idx in &node_indices {
        let node = &nodes[idx];
        if let Some((req_headers_str, req_body_str, resp_status)) = node_request_data.get(&node.id)
        {
            let req_headers_json: serde_json::Value = serde_json::from_str(req_headers_str)
                .ok()
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let req_body_json: Option<serde_json::Value> = req_body_str
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok());
            let resp_headers_json: serde_json::Value =
                serde_json::Value::Object(serde_json::Map::new());
            let resp_body_json: Option<serde_json::Value> = None;

            let (_, resp_tokens) = extract_tokens(
                &req_headers_json,
                req_body_json.as_ref(),
                *resp_status,
                &resp_headers_json,
                resp_body_json.as_ref(),
            );

            for token in resp_tokens {
                // Only store if this is the first time we've seen this token
                token_producers.entry(token.value).or_insert(node.id);
            }
        }
    }

    // Second pass: create edges for token consumers
    for &idx in &node_indices {
        let node = &nodes[idx];
        if let Some((req_headers_str, req_body_str, _)) = node_request_data.get(&node.id) {
            let req_headers_json: serde_json::Value = serde_json::from_str(req_headers_str)
                .ok()
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let req_body_json: Option<serde_json::Value> = req_body_str
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok());

            let (req_tokens, _) = extract_tokens(
                &req_headers_json,
                req_body_json.as_ref(),
                0, // No response status needed for request token extraction
                &serde_json::Value::Object(serde_json::Map::new()),
                None,
            );

            for token in req_tokens {
                if let Some(&producer_id) = token_producers.get(&token.value) {
                    if producer_id != node.id {
                        // Found a dependency
                        edges.push(DagEdge {
                            from_node_id: producer_id,
                            to_node_id: node.id,
                            token_value: token.value.clone(),
                        });

                        adjacency_list
                            .entry(producer_id)
                            .or_insert_with(Vec::new)
                            .push(node.id);
                    }
                }
            }
        }
    }

    TrafficDag {
        nodes,
        edges,
        adjacency_list,
    }
}

// ============================================================================
// Database Operations
// ============================================================================

/// Store DAG in the database.
pub fn store_dag(db_state: &DbState, dag: &TrafficDag) -> Result<(), String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    store_dag_internal(&conn, dag)
}

/// Store DAG in the database (takes &Connection directly for testability).
fn store_dag_internal(conn: &Connection, dag: &TrafficDag) -> Result<(), String> {
    // Clear existing DAG data
    conn.execute("DELETE FROM dag_edges", [])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM dag_nodes", [])
        .map_err(|e| e.to_string())?;

    // Insert nodes
    for node in &dag.nodes {
        conn.execute(
            "INSERT INTO dag_nodes (id, timestamp, method, path, host, device_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![node.id, node.timestamp, node.method, node.path, node.host, node.device_id],
        ).map_err(|e| e.to_string())?;
    }

    // Insert edges
    for edge in &dag.edges {
        conn.execute(
            "INSERT INTO dag_edges (from_node_id, to_node_id, token_value) VALUES (?1, ?2, ?3)",
            params![edge.from_node_id, edge.to_node_id, edge.token_value],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Get stored DAG from database.
pub fn get_stored_dag(db_state: &DbState) -> Result<TrafficDag, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    get_stored_dag_internal(&conn)
}

/// Get stored DAG from database (takes &Connection directly for testability).
fn get_stored_dag_internal(conn: &Connection) -> Result<TrafficDag, String> {
    // Get nodes
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, method, path, host, device_id FROM dag_nodes ORDER BY timestamp",
        )
        .map_err(|e| e.to_string())?;

    let nodes: Vec<DagNode> = stmt
        .query_map([], |row| {
            Ok(DagNode {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                method: row.get(2)?,
                path: row.get(3)?,
                host: row.get(4)?,
                device_id: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Get edges
    let mut stmt = conn
        .prepare("SELECT from_node_id, to_node_id, token_value FROM dag_edges")
        .map_err(|e| e.to_string())?;

    let edges: Vec<DagEdge> = stmt
        .query_map([], |row| {
            Ok(DagEdge {
                from_node_id: row.get(0)?,
                to_node_id: row.get(1)?,
                token_value: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Build adjacency list
    let mut adjacency_list: HashMap<i64, Vec<i64>> = HashMap::new();
    for edge in &edges {
        adjacency_list
            .entry(edge.from_node_id)
            .or_insert_with(Vec::new)
            .push(edge.to_node_id);
    }

    Ok(TrafficDag {
        nodes,
        edges,
        adjacency_list,
    })
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Get all HTTP requests for DAG building.
fn get_all_requests(
    conn: &rusqlite::Connection,
) -> Result<
    Vec<(
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        i64,
        Option<i64>,
    )>,
    rusqlite::Error,
> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, method, host, path, req_headers, req_body, resp_status, device_id
         FROM http_requests ORDER BY timestamp",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
        ))
    })?;

    rows.collect()
}

/// Build and store the DAG from current traffic data.
#[tauri::command]
pub fn build_traffic_dag(db_state: State<'_, Arc<DbState>>) -> Result<TrafficDag, String> {
    let requests = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        get_all_requests(&conn).map_err(|e| e.to_string())?
    };

    let dag = build_dag_from_requests(&requests);
    store_dag(&db_state, &dag)?;

    Ok(dag)
}

/// Get the stored traffic DAG.
#[tauri::command]
pub fn get_traffic_dag(db_state: State<'_, Arc<DbState>>) -> Result<TrafficDag, String> {
    get_stored_dag(&db_state)
}

/// Get DAG for a specific device.
#[tauri::command]
pub fn get_device_dag(
    db_state: State<'_, Arc<DbState>>,
    device_id: i64,
) -> Result<TrafficDag, String> {
    let requests: Vec<(
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        i64,
        Option<i64>,
    )> = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, method, host, path, req_headers, req_body, resp_status, device_id
             FROM http_requests WHERE device_id = ?1 ORDER BY timestamp"
        ).map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![device_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let result: Vec<_> = rows.collect();
        result
            .into_iter()
            .map(|r| r.map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, String>>()?
    };

    let dag = build_dag_from_requests(&requests);
    Ok(dag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tokens_from_text() {
        let text = r#"{"access_token": "abc123xyz789", "refresh_token": "refresh456token"}"#;

        // Test what extract_tokens_from_text produces
        let tokens = extract_tokens_from_text(text, TokenSource::ResponseBody);
        assert!(!tokens.is_empty(), "Expected tokens but got empty");
        assert!(tokens.iter().any(|t| t.value == "abc123xyz789"));
        assert!(tokens.iter().any(|t| t.value == "refresh456token"));
    }

    #[test]
    fn test_extract_tokens_from_json() {
        let json = serde_json::json!({
            "data": {
                "access_token": "token123abc",
                "user": {
                    "uid": "user456def"
                }
            }
        });
        let tokens = extract_tokens_from_json(&json, TokenSource::ResponseBody);

        assert!(tokens.iter().any(|t| t.value == "token123abc"));
        assert!(tokens.iter().any(|t| t.value == "user456def"));
    }

    #[test]
    fn test_build_dag_simple() {
        // Two requests: first returns a token, second uses it
        let requests = vec![
            (
                1,
                "2024-01-01T00:00:00".to_string(),
                "POST".to_string(),
                "api.example.com".to_string(),
                "/login".to_string(),
                Some(r#"{" Content-Type":"application/json"}"#.to_string()),
                Some(r#"{"username":"test"}"#.to_string()),
                200,
                None,
            ),
            (
                2,
                "2024-01-01T00:00:01".to_string(),
                "GET".to_string(),
                "api.example.com".to_string(),
                "/profile".to_string(),
                Some(r#"{"Authorization":"Bearer token123"}"#.to_string()),
                None,
                200,
                None,
            ),
        ];

        let dag = build_dag_from_requests(&requests);
        assert_eq!(dag.nodes.len(), 2);
    }

    // ------------------------------------------------------------------
    // extract_tokens tests
    // ------------------------------------------------------------------

    #[test]
    fn test_extract_tokens_from_request_headers() {
        // The Authorization header value contains a `access_token=...`
        // key=value pair, which the `access_token[=:]` regex pattern picks
        // up. The captured value is 18 chars, well over the 8-char minimum.
        let req_headers = serde_json::json!({
            "Authorization": "Bearer access_token=abc123def456ghi789"
        });
        let (request_tokens, _) = extract_tokens(
            &req_headers,
            None,
            200,
            &serde_json::json!({}),
            None,
        );
        assert!(
            !request_tokens.is_empty(),
            "Expected at least one token extracted from the Authorization header"
        );
        assert!(
            request_tokens.iter().any(|t| t.value == "abc123def456ghi789"),
            "Expected the token 'abc123def456ghi789' in request_tokens, got {:?}",
            request_tokens
        );
    }

    #[test]
    fn test_extract_tokens_from_response_body() {
        // A response body containing a JSON `access_token` field — the
        // recursive JSON walker should pick this up via
        // `extract_token_from_key_value`.
        let resp_body = serde_json::json!({"access_token": "xyz123abc456def789"});
        let (_, response_tokens) = extract_tokens(
            &serde_json::json!({}),
            None,
            200,
            &serde_json::json!({}),
            Some(&resp_body),
        );
        assert!(
            response_tokens.iter().any(|t| t.value == "xyz123abc456def789"),
            "Expected 'xyz123abc456def789' to be extracted from the response body, got {:?}",
            response_tokens
        );
    }

    #[test]
    fn test_extract_tokens_skips_short_tokens() {
        // The regex pattern matches `access_token=ab` and captures "ab",
        // but the 8-char minimum length filter must drop it. This verifies
        // the length guard in `extract_tokens_from_text`.
        let req_headers = serde_json::json!({
            "Authorization": "Bearer access_token=ab"
        });
        let (request_tokens, _) = extract_tokens(
            &req_headers,
            None,
            200,
            &serde_json::json!({}),
            None,
        );
        assert_eq!(
            request_tokens.len(),
            0,
            "Tokens shorter than 8 chars must be filtered out; got {:?}",
            request_tokens
        );
    }

    #[test]
    fn test_extract_tokens_returns_empty_for_clean_request() {
        // No header keys match the auth-header set, and the JSON bodies
        // use keys ('user', 'role', 'status', 'message') that are not in
        // the token-names allowlist, so no tokens should be extracted.
        let req_body = serde_json::json!({"user": "alice", "role": "admin"});
        let resp_body = serde_json::json!({"status": "ok", "message": "logged in"});
        let (request_tokens, response_tokens) = extract_tokens(
            &serde_json::json!({}),
            Some(&req_body),
            200,
            &serde_json::json!({}),
            Some(&resp_body),
        );
        assert_eq!(request_tokens.len(), 0, "Clean request body should yield no tokens");
        assert_eq!(response_tokens.len(), 0, "Clean response body should yield no tokens");
    }

    // ------------------------------------------------------------------
    // build_dag_from_requests tests
    // ------------------------------------------------------------------

    #[test]
    fn test_build_dag_from_requests_empty() {
        let dag = build_dag_from_requests(&[]);
        assert_eq!(dag.nodes.len(), 0, "Empty requests should produce no nodes");
        assert_eq!(dag.edges.len(), 0, "Empty requests should produce no edges");
        assert_eq!(dag.adjacency_list.len(), 0, "Empty requests should produce empty adjacency_list");
    }

    #[test]
    fn test_build_dag_from_requests_single_request() {
        let requests = vec![(
            1,
            "2024-01-01T00:00:00".to_string(),
            "GET".to_string(),
            "api.example.com".to_string(),
            "/test".to_string(),
            Some("{}".to_string()),
            None,
            200,
            None,
        )];
        let dag = build_dag_from_requests(&requests);
        assert_eq!(dag.nodes.len(), 1, "One request should produce one node");
        assert_eq!(dag.edges.len(), 0, "A single request cannot produce any edges");
    }

    // ------------------------------------------------------------------
    // NOTE: A test asserting that `build_dag_from_requests` links two
    // requests via a shared token is intentionally OMITTED. The current
    // implementation cannot produce any edges: the producer pass hardcodes
    // `None` for `resp_body_json` and an empty `resp_headers_json`, and the
    // 9-tuple returned by `get_all_requests` does not include response
    // data at all. Until the tuple is widened to include `resp_body` /
    // `resp_headers` and the producer pass is fixed to consume them, no
    // request can be identified as a token producer and no edges are
    // created. Tracked as task #77. See the NOTE comment in
    // `build_dag_from_requests`.
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // store_dag_internal / get_stored_dag_internal tests
    // ------------------------------------------------------------------

    #[test]
    fn test_store_dag_internal_persists_nodes_and_edges() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let dag = TrafficDag {
            nodes: vec![
                DagNode {
                    id: 1,
                    timestamp: "2024-01-01T00:00:00".to_string(),
                    method: "GET".to_string(),
                    path: "/a".to_string(),
                    host: "x.com".to_string(),
                    device_id: None,
                },
                DagNode {
                    id: 2,
                    timestamp: "2024-01-01T00:00:01".to_string(),
                    method: "GET".to_string(),
                    path: "/b".to_string(),
                    host: "x.com".to_string(),
                    device_id: None,
                },
            ],
            edges: vec![DagEdge {
                from_node_id: 1,
                to_node_id: 2,
                token_value: "abc123def456".to_string(),
            }],
            adjacency_list: HashMap::new(),
        };

        store_dag_internal(&conn, &dag).unwrap();
        let loaded = get_stored_dag_internal(&conn).unwrap();

        assert_eq!(loaded.nodes.len(), 2, "Should persist both nodes");
        assert_eq!(loaded.edges.len(), 1, "Should persist the single edge");
        assert_eq!(loaded.edges[0].from_node_id, 1);
        assert_eq!(loaded.edges[0].to_node_id, 2);
        assert_eq!(loaded.edges[0].token_value, "abc123def456");
        // Field-level roundtrip checks guard against column-swap bugs in the
        // INSERT/SELECT pair (e.g., path <-> host transposition).
        let n0 = &loaded.nodes[0];
        assert_eq!(n0.id, 1);
        assert_eq!(n0.timestamp, "2024-01-01T00:00:00");
        assert_eq!(n0.method, "GET");
        assert_eq!(n0.path, "/a");
        assert_eq!(n0.host, "x.com");
        let n1 = &loaded.nodes[1];
        assert_eq!(n1.id, 2);
        assert_eq!(n1.path, "/b");
        assert_eq!(n1.host, "x.com");
        // Adjacency list is rebuilt from the loaded edges.
        assert_eq!(loaded.adjacency_list.get(&1).unwrap(), &vec![2]);
    }

    #[test]
    fn test_store_dag_internal_clears_existing() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let dag_a = TrafficDag {
            nodes: vec![DagNode {
                id: 1,
                timestamp: "2024-01-01T00:00:00".to_string(),
                method: "GET".to_string(),
                path: "/a".to_string(),
                host: "x.com".to_string(),
                device_id: None,
            }],
            edges: vec![],
            adjacency_list: HashMap::new(),
        };
        store_dag_internal(&conn, &dag_a).unwrap();

        let dag_b = TrafficDag {
            nodes: vec![DagNode {
                id: 99,
                timestamp: "2024-01-01T00:00:01".to_string(),
                method: "GET".to_string(),
                path: "/b".to_string(),
                host: "y.com".to_string(),
                device_id: None,
            }],
            edges: vec![],
            adjacency_list: HashMap::new(),
        };
        store_dag_internal(&conn, &dag_b).unwrap();

        let loaded = get_stored_dag_internal(&conn).unwrap();
        assert_eq!(loaded.nodes.len(), 1, "Second store must replace (not append) first");
        assert_eq!(loaded.nodes[0].id, 99, "Loaded node must reflect the second dag, not the first");
    }

    #[test]
    fn test_get_stored_dag_internal_empty() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let loaded = get_stored_dag_internal(&conn).unwrap();
        assert_eq!(loaded.nodes.len(), 0, "Fresh DB should have no nodes");
        assert_eq!(loaded.edges.len(), 0, "Fresh DB should have no edges");
        assert_eq!(loaded.adjacency_list.len(), 0, "Fresh DB should have empty adjacency list");
    }
}
