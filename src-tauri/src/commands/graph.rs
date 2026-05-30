use std::collections::HashMap;
use std::sync::Arc;

use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::DbState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestNode {
    pub id: String,
    pub host: String,
    pub path: String,
    pub method: String,
    pub status: Option<u16>,
    #[serde(rename = "duration_ms")]
    pub duration_ms: u64,
    pub timestamp: i64,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub requests: Vec<RequestNode>,
    pub edges: Vec<Edge>,
}

struct RawRequest {
    id: i64,
    host: String,
    path: String,
    method: String,
    resp_status: Option<u16>,
    duration_ms: Option<u64>,
    timestamp: String,
    req_headers: String,
}

/// Parse a timestamp string into a sortable i64 (seconds since epoch or 0).
fn parse_timestamp(ts: &str) -> i64 {
    // Try parsing as float seconds (e.g. "1713000000.123")
    if let Ok(f) = ts.parse::<f64>() {
        return f as i64;
    }
    // Try parsing as ISO-like "YYYY-MM-DD HH:MM:SS"
    chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0)
}

fn extract_referer_host(req_headers: &str) -> Option<String> {
    let headers: Vec<(String, String)> = serde_json::from_str(req_headers).ok()?;
    for (name, value) in &headers {
        if name.eq_ignore_ascii_case("referer") || name.eq_ignore_ascii_case("referrer") {
            // Parse host from URL like "https://example.com/path"
            let rest = value.strip_prefix("https://").or_else(|| value.strip_prefix("http://"))?;
            let host = rest.split('/').next()?;
            // Strip port if present
            let host = host.split(':').next()?;
            return Some(host.to_string());
        }
    }
    None
}

#[tauri::command]
pub fn get_graph_data(
    db_state: State<'_, Arc<DbState>>,
    max_requests: usize,
) -> Result<GraphData, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    let limit = max_requests.min(500) as i64;

    let mut stmt = conn
        .prepare(
            r#"SELECT id, host, path, method, resp_status, duration_ms, timestamp, req_headers
               FROM http_requests
               ORDER BY id DESC
               LIMIT ?1"#,
        )
        .map_err(|e| e.to_string())?;

    let raw: Vec<RawRequest> = stmt
        .query_map(params![limit], |row| {
            Ok(RawRequest {
                id: row.get(0)?,
                host: row.get(1)?,
                path: row.get(2)?,
                method: row.get(3)?,
                resp_status: row.get(4)?,
                duration_ms: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                timestamp: row.get(6)?,
                req_headers: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    drop(stmt);

    // Build a lookup: host+path → most recent request id (for referer resolution)
    let mut host_path_to_id: HashMap<String, String> = HashMap::new();
    for r in &raw {
        let key = format!("{}{}", r.host, r.path);
        host_path_to_id.entry(key).or_insert_with(|| r.id.to_string());
    }

    // Build nodes and edges
    let mut edges = Vec::new();
    let mut parent_map: HashMap<String, Option<String>> = HashMap::new();

    for r in &raw {
        let id_str = r.id.to_string();

        // Find parent via Referer header
        if let Some(ref_host) = extract_referer_host(&r.req_headers) {
            // Try to find a matching request by referer host
            // Use the most recent request to that host as parent
            if let Some(parent_id) = raw
                .iter()
                .find(|other| other.id != r.id && other.host == ref_host)
                .map(|p| p.id.to_string())
            {
                edges.push(Edge {
                    from: parent_id.clone(),
                    to: id_str.clone(),
                });
                parent_map.insert(id_str.clone(), Some(parent_id));
            }
        }
    }

    // Also add edges for same-host sequential requests (navigations)
    let mut by_host: HashMap<String, Vec<&RawRequest>> = HashMap::new();
    for r in &raw {
        by_host.entry(r.host.clone()).or_default().push(r);
    }
    for (_host, reqs) in &by_host {
        if reqs.len() < 2 {
            continue;
        }
        // Connect sequential requests on the same host if no parent already assigned
        for window in reqs.windows(2) {
            let child_id = window[1].id.to_string();
            if !parent_map.contains_key(&child_id) {
                let parent_id = window[0].id.to_string();
                edges.push(Edge {
                    from: parent_id.clone(),
                    to: child_id.clone(),
                });
                parent_map.insert(child_id, Some(parent_id));
            }
        }
    }

    let requests: Vec<RequestNode> = raw
        .iter()
        .map(|r| RequestNode {
            id: r.id.to_string(),
            host: r.host.clone(),
            path: r.path.clone(),
            method: r.method.clone(),
            status: r.resp_status,
            duration_ms: r.duration_ms.unwrap_or(0),
            timestamp: parse_timestamp(&r.timestamp),
            parent_id: parent_map.get(&r.id.to_string()).cloned().flatten(),
        })
        .collect();

    Ok(GraphData { requests, edges })
}

#[tauri::command]
pub fn test_debug_command() -> String {
    "debug ok".to_string()
}
