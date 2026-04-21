//! DAG (Directed Acyclic Graph) builder module for traffic analysis.
//!
//! Builds a dependency graph of HTTP requests based on token passing.
//! Edges are created when a later request uses a token that was returned by an earlier request.

use crate::db::DbState;
use regex::Regex;
use rusqlite::params;
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
                if key_lower.contains("password") || key_lower.contains("secret") || key_lower.contains("key") && !key_lower.contains("token") {
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
    let token_names = ["access_token", "sessionid", "auth_token", "id", "uid", "refresh_token"];

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
                if name_lower.contains("authorization") || name_lower.contains("cookie") || name_lower.contains("x-token") || name_lower.contains("x-session") {
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
    if resp_status >= 200 && resp_status < 400 {
        if let Some(obj) = resp_headers.as_object() {
            for (name, value) in obj {
                if let Some(s) = value.as_str() {
                    let name_lower = name.to_lowercase();
                    if name_lower.contains("set-cookie") || name_lower.contains("authorization") {
                        response_tokens.extend(extract_tokens_from_text(s, TokenSource::ResponseHeader));
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
    requests: &[(i64, String, String, String, String, Option<String>, Option<String>, i64, Option<i64>)],
) -> TrafficDag {
    let mut nodes: Vec<DagNode> = Vec::new();
    let mut node_by_id: HashMap<i64, usize> = HashMap::new();

    // Collect request data for token extraction
    let mut node_request_data: HashMap<i64, (String, Option<String>, u16)> = HashMap::new();

    // First pass: create nodes and collect request data
    for (id, timestamp, method, host, path, req_headers, req_body, resp_status, device_id) in requests {
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

        node_request_data.insert(*id, (
            req_headers.clone().unwrap_or_default(),
            req_body.clone(),
            *resp_status as u16,
        ));
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
    for &idx in &node_indices {
        let node = &nodes[idx];
        if let Some((req_headers_str, req_body_str, resp_status)) = node_request_data.get(&node.id) {
            let req_headers_json: serde_json::Value = serde_json::from_str(req_headers_str).ok().unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let req_body_json: Option<serde_json::Value> = req_body_str.as_ref().and_then(|s| serde_json::from_str(s).ok());
            let resp_headers_json: serde_json::Value = serde_json::Value::Object(serde_json::Map::new());
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
            let req_headers_json: serde_json::Value = serde_json::from_str(req_headers_str).ok().unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let req_body_json: Option<serde_json::Value> = req_body_str.as_ref().and_then(|s| serde_json::from_str(s).ok());

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

    // Clear existing DAG data
    conn.execute("DELETE FROM dag_edges", []).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM dag_nodes", []).map_err(|e| e.to_string())?;

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
        ).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Get stored DAG from database.
pub fn get_stored_dag(db_state: &DbState) -> Result<TrafficDag, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    // Get nodes
    let mut stmt = conn
        .prepare("SELECT id, timestamp, method, path, host, device_id FROM dag_nodes ORDER BY timestamp")
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
fn get_all_requests(conn: &rusqlite::Connection) -> Result<Vec<(i64, String, String, String, String, Option<String>, Option<String>, i64, Option<i64>)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, method, host, path, req_headers, req_body, resp_status, device_id
         FROM http_requests ORDER BY timestamp"
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
pub fn get_device_dag(db_state: State<'_, Arc<DbState>>, device_id: i64) -> Result<TrafficDag, String> {
    let requests: Vec<(i64, String, String, String, String, Option<String>, Option<String>, i64, Option<i64>)> = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, method, host, path, req_headers, req_body, resp_status, device_id
             FROM http_requests WHERE device_id = ?1 ORDER BY timestamp"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![device_id], |row| {
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
        }).map_err(|e| e.to_string())?;

        let result: Vec<_> = rows.collect();
        result.into_iter().map(|r| r.map_err(|e| e.to_string())).collect::<Result<Vec<_>, String>>()?
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
            (1, "2024-01-01T00:00:00".to_string(), "POST".to_string(), "api.example.com".to_string(), "/login".to_string(),
             Some(r#"{" Content-Type":"application/json"}"#.to_string()), Some(r#"{"username":"test"}"#.to_string()), 200, None),
            (2, "2024-01-01T00:00:01".to_string(), "GET".to_string(), "api.example.com".to_string(), "/profile".to_string(),
             Some(r#"{"Authorization":"Bearer token123"}"#.to_string()), None, 200, None),
        ];

        let dag = build_dag_from_requests(&requests);
        assert_eq!(dag.nodes.len(), 2);
    }
}