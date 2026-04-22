//! Traffic replay module for ProxyBot.
//!
//! Replays recorded HTTP requests against a local mock server and computes diffs.

use crate::db::DbState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::sleep;

/// Replay target - a host that can be replayed.
#[derive(Serialize, Deserialize, Clone)]
pub struct ReplayTarget {
    pub host: String,
    pub request_count: usize,
    pub path_count: usize,
}

/// A single request to replay.
#[derive(Serialize, Deserialize, Clone)]
pub struct ReplayRequest {
    pub id: i64,
    pub method: String,
    pub url: String,
    pub path: String,
    pub req_headers: Vec<(String, String)>,
    pub req_body: Option<String>,
}

/// The recorded response for a request.
#[derive(Serialize, Deserialize, Clone)]
pub struct RecordedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// Result of replaying a single request.
#[derive(Serialize, Deserialize, Clone)]
pub struct ReplayResult {
    pub request_id: i64,
    pub method: String,
    pub url: String,
    pub recorded_response: RecordedResponse,
    pub mock_response: Option<MockResponse>,
    pub diff: Option<DiffResult>,
    pub delay_ms: u64,
    pub error: Option<String>,
}

/// The mock server's response.
#[derive(Serialize, Deserialize, Clone)]
pub struct MockResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// Diff result for headers and body.
#[derive(Serialize, Deserialize, Clone)]
pub struct DiffResult {
    pub header_diffs: Vec<HeaderDiff>,
    pub body_diff: Option<BodyDiff>,
    pub has_changes: bool,
}

/// Difference in a header.
#[derive(Serialize, Deserialize, Clone)]
pub struct HeaderDiff {
    pub header: String,
    pub recorded: Option<String>,
    pub mock: Option<String>,
    pub diff_type: DiffType,
}

/// Body diff with line-by-line comparison.
#[derive(Serialize, Deserialize, Clone)]
pub struct BodyDiff {
    pub recorded: Option<String>,
    pub mock: Option<String>,
    pub recorded_lines: Vec<String>,
    pub mock_lines: Vec<String>,
    pub line_diffs: Vec<LineDiff>,
}

/// Type of difference.
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum DiffType {
    Added,
    Removed,
    Modified,
    Unchanged,
}

/// A single line diff.
#[derive(Serialize, Deserialize, Clone)]
pub struct LineDiff {
    pub line_number_recorded: Option<usize>,
    pub line_number_mock: Option<usize>,
    pub recorded_text: Option<String>,
    pub mock_text: Option<String>,
    pub diff_type: DiffType,
}

/// Replay session state.
pub struct ReplayState {
    #[allow(dead_code)]
    pub mock_port: u16,
    #[allow(dead_code)]
    pub is_running: bool,
    #[allow(dead_code)]
    pub results: Vec<ReplayResult>,
}

impl Default for ReplayState {
    fn default() -> Self {
        Self {
            mock_port: 19998,
            is_running: false,
            results: Vec::new(),
        }
    }
}

/// Get all hosts that have recorded requests.
#[tauri::command]
pub fn get_replay_targets(state: State<'_, Arc<DbState>>) -> Result<Vec<ReplayTarget>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT host, COUNT(*) as cnt, COUNT(DISTINCT path) as path_cnt
             FROM http_requests
             GROUP BY host
             ORDER BY cnt DESC",
        )
        .map_err(|e| e.to_string())?;

    let targets = stmt
        .query_map([], |row| {
            Ok(ReplayTarget {
                host: row.get(0)?,
                request_count: row.get::<_, i64>(1)? as usize,
                path_count: row.get::<_, i64>(2)? as usize,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(targets)
}

/// Get requests for a specific host.
#[tauri::command]
pub fn get_requests_for_replay(
    state: State<'_, Arc<DbState>>,
    host: String,
) -> Result<Vec<ReplayRequest>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, method, host, path, req_headers, req_body
             FROM http_requests
             WHERE host = ?1
             ORDER BY timestamp ASC",
        )
        .map_err(|e| e.to_string())?;

    let requests = stmt
        .query_map([&host], |row| {
            let req_headers_json: String = row.get(4)?;
            let req_headers: Vec<(String, String)> =
                serde_json::from_str(&req_headers_json).unwrap_or_default();
            let req_body: Option<Vec<u8>> = row.get(5)?;
            let req_body_str = req_body.as_ref().map(|b| String::from_utf8_lossy(b).to_string());

            let path: String = row.get(3)?;
            let method: String = row.get(1)?;
            let host: String = row.get(2)?;
            let url = format!("{}{}", host, path);

            Ok(ReplayRequest {
                id: row.get(0)?,
                method,
                url,
                path,
                req_headers,
                req_body: req_body_str,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(requests)
}

/// Get recorded responses for requests.
#[tauri::command]
pub fn get_recorded_responses(
    state: State<'_, Arc<DbState>>,
    request_ids: Vec<i64>,
) -> Result<HashMap<i64, RecordedResponse>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut responses = HashMap::new();

    for id in request_ids {
        let mut stmt = conn
            .prepare(
                "SELECT resp_status, resp_headers, resp_body
                 FROM http_requests
                 WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;

        if let Ok(result) = stmt.query_row([id], |row| {
            let resp_status: Option<u16> = row.get(0)?;
            let resp_headers_json: String = row.get(1)?;
            let resp_headers: Vec<(String, String)> =
                serde_json::from_str(&resp_headers_json).unwrap_or_default();
            let resp_body: Option<Vec<u8>> = row.get(2)?;
            let resp_body_str = resp_body.as_ref().map(|b| String::from_utf8_lossy(b).to_string());

            Ok(RecordedResponse {
                status: resp_status.unwrap_or(0),
                headers: resp_headers,
                body: resp_body_str,
            })
        }) {
            responses.insert(id, result);
        }
    }

    Ok(responses)
}

/// Start the mock server and replay requests.
#[tauri::command]
pub async fn start_replay(
    state: State<'_, Arc<DbState>>,
    host: String,
    delay_ms: u64,
) -> Result<Vec<ReplayResult>, String> {
    // Get requests for this host
    let requests = get_requests_for_replay(state.clone(), host.clone())?;
    if requests.is_empty() {
        return Err("No requests found for this host".to_string());
    }

    let request_ids: Vec<i64> = requests.iter().map(|r| r.id).collect();
    let recorded_responses = get_recorded_responses(state, request_ids)?;

    // Start mock server
    let mock_port = 19998;
    let mock_responses = recorded_responses.clone();

    // Spawn mock server
    let server_handle = tokio::spawn(async move {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", mock_port)).await;
        if listener.is_err() {
            return;
        }
        let listener = listener.unwrap();

        loop {
            match listener.accept().await {
                Ok((mut stream, _)) => {
                    let responses = mock_responses.clone();
                    tokio::spawn(async move {
                        let mut buf = [0u8; 8192];
                        if let Ok(n) = stream.read(&mut buf).await {
                            let request = String::from_utf8_lossy(&buf[..n]).to_string();
                            let lines: Vec<&str> = request.lines().collect();
                            if let Some(request_line) = lines.first() {
                                let parts: Vec<&str> = request_line.split_whitespace().collect();
                                if parts.len() >= 2 {
                                    let _method = parts[0];
                                    let path = parts[1];

                                    // Find matching response by path
                                    let mut response_body = String::new();
                                    let mut response_status = 404;
                                    let mut response_headers = Vec::new();

                                    for (_req_id, resp) in &responses {
                                        // Try to find request with this path
                                        // For simplicity, use first response if path matches
                                        if path == "/" || path.starts_with("/") {
                                            response_status = resp.status;
                                            response_headers = resp.headers.clone();
                                            if let Some(ref body) = resp.body {
                                                response_body = body.clone();
                                            }
                                            break;
                                        }
                                    }

                                    // Build HTTP response
                                    let body_len = response_body.len();
                                    let headers_str = response_headers
                                        .iter()
                                        .map(|(k, v)| format!("{}: {}\r\n", k, v))
                                        .collect::<String>();
                                    let response = format!(
                                        "HTTP/1.1 {} OK\r\n{}Content-Length: {}\r\n\r\n{}",
                                        response_status, headers_str, body_len, response_body
                                    );
                                    let _ = stream.write_all(response.as_bytes()).await;
                                }
                            }
                        }
                    });
                }
                Err(_) => break,
            }
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(100)).await;

    // Replay requests
    let mut results = Vec::new();
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| e.to_string())?;

    for request in &requests {
        if delay_ms > 0 {
            sleep(Duration::from_millis(delay_ms)).await;
        }

        // Make request to mock server
        let mock_url = format!("http://127.0.0.1:{}{}", mock_port, request.path);
        let mock_response = client
            .request(
                reqwest::Method::from_bytes(request.method.as_bytes()).unwrap_or(reqwest::Method::GET),
                &mock_url,
            )
            .headers(
                request
                    .req_headers
                    .iter()
                    .fold(reqwest::header::HeaderMap::new(), |mut headers, (k, v)| {
                        if let (Ok(name), Ok(value)) = (
                            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                            reqwest::header::HeaderValue::from_str(v),
                        ) {
                            headers.insert(name, value);
                        }
                        headers
                    }),
            )
            .body(request.req_body.clone().unwrap_or_default())
            .send()
            .await;

        let recorded = recorded_responses.get(&request.id);

        match mock_response {
            Ok(resp) => {
                let mock_status = resp.status().as_u16();
                let mock_headers: Vec<(String, String)> = resp
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                let mock_body = resp.text().await.ok();

                let mock_resp = MockResponse {
                    status: mock_status,
                    headers: mock_headers.clone(),
                    body: mock_body.clone(),
                };

                // Compute diff
                let diff = if let Some(recorded) = recorded {
                    Some(compute_diff(
                        &recorded.status,
                        &recorded.headers,
                        &recorded.body,
                        &mock_status,
                        &mock_headers,
                        &mock_body,
                    ))
                } else {
                    None
                };

                results.push(ReplayResult {
                    request_id: request.id,
                    method: request.method.clone(),
                    url: request.url.clone(),
                    recorded_response: recorded.cloned().unwrap_or(RecordedResponse {
                        status: 0,
                        headers: vec![],
                        body: None,
                    }),
                    mock_response: Some(mock_resp),
                    diff,
                    delay_ms,
                    error: None,
                });
            }
            Err(e) => {
                results.push(ReplayResult {
                    request_id: request.id,
                    method: request.method.clone(),
                    url: request.url.clone(),
                    recorded_response: recorded.cloned().unwrap_or(RecordedResponse {
                        status: 0,
                        headers: vec![],
                        body: None,
                    }),
                    mock_response: None,
                    diff: None,
                    delay_ms,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Shutdown mock server
    drop(server_handle);

    Ok(results)
}

/// Compute diff between recorded and mock responses.
fn compute_diff(
    _recorded_status: &u16,
    recorded_headers: &[(String, String)],
    recorded_body: &Option<String>,
    _mock_status: &u16,
    mock_headers: &[(String, String)],
    mock_body: &Option<String>,
) -> DiffResult {
    let mut header_diffs = Vec::new();
    let mut all_headers: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (k, _) in recorded_headers {
        all_headers.insert(k.clone());
    }
    for (k, _) in mock_headers {
        all_headers.insert(k.clone());
    }

    for header in all_headers {
        let recorded_val = recorded_headers.iter().find(|(k, _)| k == &header).map(|(_, v)| v.clone());
        let mock_val = mock_headers.iter().find(|(k, _)| k == &header).map(|(_, v)| v.clone());

        let diff_type = match (&recorded_val, &mock_val) {
            (Some(r), Some(m)) if r == m => DiffType::Unchanged,
            (Some(_), Some(_)) => DiffType::Modified,
            (Some(_), None) => DiffType::Removed,
            (None, Some(_)) => DiffType::Added,
            (None, None) => continue,
        };

        header_diffs.push(HeaderDiff {
            header,
            recorded: recorded_val,
            mock: mock_val,
            diff_type,
        });
    }

    // Body diff
    let body_diff = compute_body_diff(recorded_body, mock_body);

    let has_changes = header_diffs.iter().any(|d| d.diff_type != DiffType::Unchanged)
        || body_diff.as_ref().map(|b| b.line_diffs.iter().any(|l| l.diff_type != DiffType::Unchanged)).unwrap_or(false);

    DiffResult {
        header_diffs,
        body_diff,
        has_changes,
    }
}

/// Compute line-by-line body diff.
fn compute_body_diff(recorded: &Option<String>, mock: &Option<String>) -> Option<BodyDiff> {
    let recorded_lines: Vec<String> = recorded
        .as_ref()
        .map(|s| s.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default();
    let mock_lines: Vec<String> = mock
        .as_ref()
        .map(|s| s.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default();

    let max_len = recorded_lines.len().max(mock_lines.len());
    let mut line_diffs = Vec::new();

    for i in 0..max_len {
        let recorded_line = recorded_lines.get(i).cloned();
        let mock_line = mock_lines.get(i).cloned();

        let diff_type = match (&recorded_line, &mock_line) {
            (Some(r), Some(m)) if r == m => DiffType::Unchanged,
            (Some(_), Some(_)) => DiffType::Modified,
            (Some(_), None) => DiffType::Removed,
            (None, Some(_)) => DiffType::Added,
            (None, None) => continue,
        };

        line_diffs.push(LineDiff {
            line_number_recorded: recorded_line.as_ref().map(|_| i + 1),
            line_number_mock: mock_line.as_ref().map(|_| i + 1),
            recorded_text: recorded_line,
            mock_text: mock_line,
            diff_type,
        });
    }

    Some(BodyDiff {
        recorded: recorded.clone(),
        mock: mock.clone(),
        recorded_lines,
        mock_lines,
        line_diffs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_diff() {
        let recorded_status = 200;
        let recorded_headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Custom".to_string(), "value1".to_string()),
        ];
        let recorded_body = Some(r#"{"key": "value"}"#.to_string());

        let mock_status = 200;
        let mock_headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Custom".to_string(), "different".to_string()),
        ];
        let mock_body = Some(r#"{"key": "different"}"#.to_string());

        let diff = compute_diff(
            &recorded_status,
            &recorded_headers,
            &recorded_body,
            &mock_status,
            &mock_headers,
            &mock_body,
        );

        assert!(diff.has_changes);
        assert_eq!(diff.header_diffs.len(), 2);
        assert!(diff.body_diff.is_some());
    }

    #[test]
    fn test_compute_body_diff() {
        let recorded = Some("line1\nline2\nline3".to_string());
        let mock = Some("line1\nmodified\nline3".to_string());

        let diff = compute_body_diff(&recorded, &mock);
        assert!(diff.is_some());
        let diff = diff.unwrap();
        assert_eq!(diff.line_diffs.len(), 3);
    }
}
