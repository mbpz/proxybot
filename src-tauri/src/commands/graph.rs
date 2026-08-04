use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::{parse_captured_timestamp, CapturedRequestQuery, DbState};

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

fn extract_referer_host(req_headers: &str) -> Option<String> {
    let headers: Vec<(String, String)> = serde_json::from_str(req_headers).ok()?;
    for (name, value) in &headers {
        if name.eq_ignore_ascii_case("referer") || name.eq_ignore_ascii_case("referrer") {
            // Parse host from URL like "https://example.com/path"
            let rest = value
                .strip_prefix("https://")
                .or_else(|| value.strip_prefix("http://"))?;
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
    let raw: Vec<RawRequest> = db_state
        .captured_requests(&CapturedRequestQuery {
            limit: Some(max_requests.min(500)),
            ..Default::default()
        })?
        .into_iter()
        .map(|record| RawRequest {
            id: record.id,
            host: record.host,
            path: record.path,
            method: record.method,
            resp_status: record.response_status,
            duration_ms: record
                .duration_ms
                .and_then(|value| u64::try_from(value).ok()),
            timestamp: record.timestamp,
            req_headers: serde_json::to_string(&record.request_headers).unwrap_or_default(),
        })
        .collect();

    // Build a lookup: host+path → most recent request id (for referer resolution)
    let mut host_path_to_id: HashMap<String, String> = HashMap::new();
    for r in &raw {
        let key = format!("{}{}", r.host, r.path);
        host_path_to_id
            .entry(key)
            .or_insert_with(|| r.id.to_string());
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
    for reqs in by_host.values() {
        if reqs.len() < 2 {
            continue;
        }
        // Connect sequential requests on the same host if no parent already assigned
        for window in reqs.windows(2) {
            let child_id = window[1].id.to_string();
            parent_map.entry(child_id.clone()).or_insert_with(|| {
                let parent_id = window[0].id.to_string();
                edges.push(Edge {
                    from: parent_id.clone(),
                    to: child_id.clone(),
                });
                Some(parent_id)
            });
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
            timestamp: parse_captured_timestamp(&r.timestamp)
                .map(|timestamp| timestamp.timestamp())
                .unwrap_or(0),
            parent_id: parent_map.get(&r.id.to_string()).cloned().flatten(),
        })
        .collect();

    Ok(GraphData { requests, edges })
}

// Non-command test function
pub fn test_graph_helper() -> i32 {
    42
}
