//! Traffic replay module for ProxyBot.
//!
//! Replays recorded HTTP requests against a local mock server and computes diffs.

use crate::db::DbState;
use rusqlite::Connection;
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
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
    pub mock_port: u16,
    pub is_running: std::sync::Mutex<bool>,
    pub results: std::sync::Mutex<Vec<ReplayResult>>,
}

impl Default for ReplayState {
    fn default() -> Self {
        Self {
            mock_port: 19998,
            is_running: std::sync::Mutex::new(false),
            results: std::sync::Mutex::new(Vec::new()),
        }
    }
}

/// Get all hosts that have recorded requests.
#[tauri::command]
pub fn get_replay_targets(state: State<'_, Arc<DbState>>) -> Result<Vec<ReplayTarget>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    get_replay_targets_internal(&conn)
}

/// Get all hosts that have recorded requests (takes `&Connection` for testability).
pub(crate) fn get_replay_targets_internal(conn: &Connection) -> Result<Vec<ReplayTarget>, String> {
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
    get_requests_for_replay_internal(&conn, &host)
}

/// Get requests for a specific host (takes `&Connection` for testability).
///
/// NOTE: The `url` field is constructed as `format!("{}{}", host, path)` — a
/// scheme-less, authority+path string like `api.example.com/v1/users`. This is
/// a display field only (the real HTTP target in `start_replay` is built from
/// the loopback `mock_url` constant), but a UI consumer may expect a full URL.
pub(crate) fn get_requests_for_replay_internal(
    conn: &Connection,
    host: &str,
) -> Result<Vec<ReplayRequest>, String> {
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
            let req_body_str = req_body
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).to_string());

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
    get_recorded_responses_internal(&conn, &request_ids)
}

/// Get recorded responses for requests (takes `&Connection` for testability).
///
/// NOTE: For unknown `request_ids`, the function silently omits them from the
/// returned map (no error, no entry). The `unwrap_or_default()` calls for
/// header JSON parsing also silently swallow malformed JSON.
pub(crate) fn get_recorded_responses_internal(
    conn: &Connection,
    request_ids: &[i64],
) -> Result<HashMap<i64, RecordedResponse>, String> {
    let mut responses: HashMap<i64, RecordedResponse> = HashMap::new();

    for id in request_ids {
        // `request_ids: &[i64]` iterates as `&i64`; deref to `i64` for the
        // rusqlite parameter binding below.
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
            let resp_body_str = resp_body
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).to_string());

            Ok(RecordedResponse {
                status: resp_status.unwrap_or(0),
                headers: resp_headers,
                body: resp_body_str,
            })
        }) {
            responses.insert(*id, result);
        }
    }

    Ok(responses)
}

/// Start the mock server and replay requests.
#[tauri::command]
pub async fn start_replay(
    state: State<'_, Arc<DbState>>,
    replay_state: State<'_, Arc<ReplayState>>,
    host: String,
    delay_ms: u64,
) -> Result<Vec<ReplayResult>, String> {
    // Mark replay as running
    *replay_state.is_running.lock().unwrap() = true;

    // Get requests for this host
    let requests = get_requests_for_replay(state.clone(), host.clone())?;
    if requests.is_empty() {
        *replay_state.is_running.lock().unwrap() = false;
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
                                        // NOTE: BUG — the path check
                                        //   `if path == "/" || path.starts_with("/")`
                                        // is ALWAYS true (every HTTP path starts with
                                        // "/"), so this loop always returns the FIRST
                                        // response in the HashMap regardless of which
                                        // path the client requested. Replay diffs will
                                        // be incorrect when multiple distinct paths
                                        // exist for the same host. See task #80.
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
                reqwest::Method::from_bytes(request.method.as_bytes())
                    .unwrap_or(reqwest::Method::GET),
                &mock_url,
            )
            .headers(request.req_headers.iter().fold(
                reqwest::header::HeaderMap::new(),
                |mut headers, (k, v)| {
                    if let (Ok(name), Ok(value)) = (
                        reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                        reqwest::header::HeaderValue::from_str(v),
                    ) {
                        headers.insert(name, value);
                    }
                    headers
                },
            ))
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

    // Store results and mark replay as done
    *replay_state.results.lock().unwrap() = results.clone();
    *replay_state.is_running.lock().unwrap() = false;

    Ok(results)
}

/// Compute diff between recorded and mock responses.
pub fn compute_diff(
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
        let recorded_val = recorded_headers
            .iter()
            .find(|(k, _)| k == &header)
            .map(|(_, v)| v.clone());
        let mock_val = mock_headers
            .iter()
            .find(|(k, _)| k == &header)
            .map(|(_, v)| v.clone());

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

    let has_changes = header_diffs
        .iter()
        .any(|d| d.diff_type != DiffType::Unchanged)
        || body_diff
            .as_ref()
            .map(|b| {
                b.line_diffs
                    .iter()
                    .any(|l| l.diff_type != DiffType::Unchanged)
            })
            .unwrap_or(false);

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
    use crate::db::{record_http_request, DbState};

    // Missing coverage: see tombstone NOTE at the bottom of this module for
    // start_replay / task #80.

    /// Helper: open an in-memory database with the full schema and seed
    /// `http_requests` rows so replay queries have something to read.
    fn seeded_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();
        conn
    }

    // ------------------------------------------------------------------
    // compute_diff — pure function tests
    // ------------------------------------------------------------------

    #[test]
    fn test_compute_diff_detects_modified_value_and_modified_body() {
        let recorded_headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Custom".to_string(), "value1".to_string()),
        ];
        let mock_headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Custom".to_string(), "different".to_string()),
        ];
        let recorded_body = Some(r#"{"key": "value"}"#.to_string());
        let mock_body = Some(r#"{"key": "different"}"#.to_string());

        let diff = compute_diff(&200, &recorded_headers, &recorded_body, &200, &mock_headers, &mock_body);

        assert!(diff.has_changes, "modified header + modified body must set has_changes");

        // Two distinct header keys, both with values present on both sides.
        assert_eq!(diff.header_diffs.len(), 2);

        // Per-header: Content-Type unchanged, X-Custom modified.
        let content_type = diff.header_diffs.iter().find(|h| h.header == "Content-Type").unwrap();
        assert_eq!(content_type.diff_type, DiffType::Unchanged);

        let x_custom = diff.header_diffs.iter().find(|h| h.header == "X-Custom").unwrap();
        assert_eq!(x_custom.diff_type, DiffType::Modified);
        assert_eq!(x_custom.recorded.as_deref(), Some("value1"));
        assert_eq!(x_custom.mock.as_deref(), Some("different"));

        // Body diff present and shows modification.
        let body = diff.body_diff.as_ref().expect("body diff should be Some");
        assert_eq!(body.line_diffs.len(), 1);
        assert_eq!(body.line_diffs[0].diff_type, DiffType::Modified);
    }

    #[test]
    fn test_compute_diff_identical_inputs_have_no_changes() {
        let headers = vec![("Content-Type".to_string(), "text/plain".to_string())];
        let body = Some("hello\nworld".to_string());

        let diff = compute_diff(&200, &headers, &body, &200, &headers, &body);

        assert!(!diff.has_changes, "Identical inputs must NOT set has_changes");
        assert_eq!(diff.header_diffs.len(), 1);
        assert_eq!(diff.header_diffs[0].diff_type, DiffType::Unchanged);

        let body = diff.body_diff.as_ref().unwrap();
        assert!(body.line_diffs.iter().all(|l| l.diff_type == DiffType::Unchanged));
    }

    #[test]
    fn test_compute_diff_header_added_on_mock_side() {
        let recorded_headers = vec![("Content-Type".to_string(), "application/json".to_string())];
        let mock_headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Request-Id".to_string(), "abc-123".to_string()),
        ];

        let diff = compute_diff(&200, &recorded_headers, &None, &200, &mock_headers, &None);

        assert!(diff.has_changes, "An added header should set has_changes");
        let added = diff
            .header_diffs
            .iter()
            .find(|h| h.header == "X-Request-Id")
            .expect("X-Request-Id should appear in the union");
        assert_eq!(added.diff_type, DiffType::Added);
        assert_eq!(added.recorded, None);
        assert_eq!(added.mock.as_deref(), Some("abc-123"));
    }

    #[test]
    fn test_compute_diff_header_removed_on_mock_side() {
        let recorded_headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer xyz".to_string()),
        ];
        let mock_headers = vec![("Content-Type".to_string(), "application/json".to_string())];

        let diff = compute_diff(&200, &recorded_headers, &None, &200, &mock_headers, &None);

        assert!(diff.has_changes);
        let removed = diff
            .header_diffs
            .iter()
            .find(|h| h.header == "Authorization")
            .expect("Authorization should still appear as Removed");
        assert_eq!(removed.diff_type, DiffType::Removed);
        assert_eq!(removed.recorded.as_deref(), Some("Bearer xyz"));
        assert_eq!(removed.mock, None);
    }

    #[test]
    fn test_compute_diff_empty_header_lists_produce_no_header_diffs() {
        let empty: Vec<(String, String)> = vec![];
        let diff = compute_diff(&200, &empty, &None, &200, &empty, &None);

        assert_eq!(diff.header_diffs.len(), 0);
        // has_changes should be false since headers and body are both empty/none.
        assert!(!diff.has_changes);
    }

    #[test]
    fn test_compute_diff_body_only_change_keeps_headers_unchanged() {
        let recorded_headers = vec![("Content-Type".to_string(), "text/plain".to_string())];
        let mock_headers = vec![("Content-Type".to_string(), "text/plain".to_string())];
        let recorded_body = Some("a\nb\nc".to_string());
        let mock_body = Some("a\nB\nc".to_string());

        let diff = compute_diff(&200, &recorded_headers, &recorded_body, &200, &mock_headers, &mock_body);

        // Headers match; only body has a change.
        assert!(diff.has_changes, "Body-only change must still set has_changes");
        assert_eq!(diff.header_diffs.len(), 1);
        assert_eq!(diff.header_diffs[0].diff_type, DiffType::Unchanged);
    }

    // ------------------------------------------------------------------
    // compute_body_diff — pure helper tests
    // ------------------------------------------------------------------

    #[test]
    fn test_compute_body_diff_three_lines_one_modified() {
        let recorded = Some("line1\nline2\nline3".to_string());
        let mock = Some("line1\nmodified\nline3".to_string());

        let diff = compute_body_diff(&recorded, &mock).expect("always Some");
        assert_eq!(diff.recorded_lines, vec!["line1", "line2", "line3"]);
        assert_eq!(diff.mock_lines, vec!["line1", "modified", "line3"]);
        assert_eq!(diff.line_diffs.len(), 3);
        assert_eq!(diff.line_diffs[0].diff_type, DiffType::Unchanged);
        assert_eq!(diff.line_diffs[1].diff_type, DiffType::Modified);
        assert_eq!(diff.line_diffs[1].recorded_text.as_deref(), Some("line2"));
        assert_eq!(diff.line_diffs[1].mock_text.as_deref(), Some("modified"));
        assert_eq!(diff.line_diffs[2].diff_type, DiffType::Unchanged);
        // 1-based line numbers.
        assert_eq!(diff.line_diffs[0].line_number_recorded, Some(1));
        assert_eq!(diff.line_diffs[1].line_number_mock, Some(2));
    }

    #[test]
    fn test_compute_body_diff_mock_shorter_marks_trailing_as_removed() {
        let recorded = Some("a\nb\nc".to_string());
        let mock = Some("a".to_string());

        let diff = compute_body_diff(&recorded, &mock).expect("always Some");
        // max_len = 3, so we get 3 entries; the ones past the end of mock are Removed.
        assert_eq!(diff.line_diffs.len(), 3);
        assert_eq!(diff.line_diffs[0].diff_type, DiffType::Unchanged);
        assert_eq!(diff.line_diffs[1].diff_type, DiffType::Removed);
        assert_eq!(diff.line_diffs[1].recorded_text.as_deref(), Some("b"));
        assert_eq!(diff.line_diffs[1].mock_text, None);
        assert_eq!(diff.line_diffs[2].diff_type, DiffType::Removed);
    }

    #[test]
    fn test_compute_body_diff_mock_longer_marks_trailing_as_added() {
        let recorded = Some("a".to_string());
        let mock = Some("a\nb\nc".to_string());

        let diff = compute_body_diff(&recorded, &mock).expect("always Some");
        assert_eq!(diff.line_diffs.len(), 3);
        assert_eq!(diff.line_diffs[0].diff_type, DiffType::Unchanged);
        assert_eq!(diff.line_diffs[1].diff_type, DiffType::Added);
        assert_eq!(diff.line_diffs[1].recorded_text, None);
        assert_eq!(diff.line_diffs[1].mock_text.as_deref(), Some("b"));
        assert_eq!(diff.line_diffs[2].diff_type, DiffType::Added);
    }

    #[test]
    fn test_compute_body_diff_both_none_yields_empty_lines() {
        let diff = compute_body_diff(&None, &None).expect("always Some");
        assert_eq!(diff.recorded_lines.len(), 0);
        assert_eq!(diff.mock_lines.len(), 0);
        assert_eq!(diff.line_diffs.len(), 0);
        assert_eq!(diff.recorded, None);
        assert_eq!(diff.mock, None);
    }

    #[test]
    fn test_compute_body_diff_one_none_one_some_yields_all_removed() {
        let recorded = Some("only-on-recorded".to_string());
        let diff = compute_body_diff(&recorded, &None).expect("always Some");
        assert_eq!(diff.line_diffs.len(), 1);
        assert_eq!(diff.line_diffs[0].diff_type, DiffType::Removed);
        assert_eq!(diff.line_diffs[0].recorded_text.as_deref(), Some("only-on-recorded"));
        assert_eq!(diff.line_diffs[0].mock_text, None);
    }

    // ------------------------------------------------------------------
    // get_replay_targets_internal — DB CRUD tests
    // ------------------------------------------------------------------

    #[test]
    fn test_get_replay_targets_internal_empty_db() {
        let conn = seeded_db();
        let targets = get_replay_targets_internal(&conn).unwrap();
        assert!(targets.is_empty(), "Empty http_requests should yield no targets");
    }

    #[test]
    fn test_get_replay_targets_internal_groups_by_host_and_counts_paths() {
        let conn = seeded_db();
        let empty_headers: Vec<(String, String)> = vec![];

        // api.example.com: 3 requests, 2 distinct paths
        record_http_request(&conn, "2026-06-04 10:00:00", "GET", "https", "api.example.com", "/v1/users", &empty_headers, None, Some(200), &empty_headers, None, None, None, None).unwrap();
        record_http_request(&conn, "2026-06-04 10:00:01", "GET", "https", "api.example.com", "/v1/users", &empty_headers, None, Some(200), &empty_headers, None, None, None, None).unwrap();
        record_http_request(&conn, "2026-06-04 10:00:02", "POST", "https", "api.example.com", "/v1/login", &empty_headers, None, Some(201), &empty_headers, None, None, None, None).unwrap();

        // cdn.example.com: 1 request, 1 path
        record_http_request(&conn, "2026-06-04 10:00:03", "GET", "https", "cdn.example.com", "/img.png", &empty_headers, None, Some(200), &empty_headers, None, None, None, None).unwrap();

        let targets = get_replay_targets_internal(&conn).unwrap();
        assert_eq!(targets.len(), 2, "Should have 2 distinct hosts");

        // ORDER BY cnt DESC → api.example.com first.
        assert_eq!(targets[0].host, "api.example.com");
        assert_eq!(targets[0].request_count, 3);
        assert_eq!(targets[0].path_count, 2, "Two distinct paths under api.example.com");

        assert_eq!(targets[1].host, "cdn.example.com");
        assert_eq!(targets[1].request_count, 1);
        assert_eq!(targets[1].path_count, 1);
    }

    #[test]
    fn test_get_replay_targets_internal_sorts_by_count_descending() {
        let conn = seeded_db();
        let empty_headers: Vec<(String, String)> = vec![];

        // a.com: 1 row
        record_http_request(&conn, "2026-06-04 10:00:00", "GET", "https", "a.com", "/", &empty_headers, None, Some(200), &empty_headers, None, None, None, None).unwrap();
        // b.com: 5 rows
        for i in 0..5 {
            record_http_request(&conn, "2026-06-04 10:00:01", "GET", "https", "b.com", &format!("/p{}", i), &empty_headers, None, Some(200), &empty_headers, None, None, None, None).unwrap();
        }
        // c.com: 3 rows
        for i in 0..3 {
            record_http_request(&conn, "2026-06-04 10:00:02", "GET", "https", "c.com", &format!("/p{}", i), &empty_headers, None, Some(200), &empty_headers, None, None, None, None).unwrap();
        }

        let targets = get_replay_targets_internal(&conn).unwrap();
        assert_eq!(targets.len(), 3);
        // Expected sort: b.com (5), c.com (3), a.com (1)
        assert_eq!(targets[0].host, "b.com");
        assert_eq!(targets[0].request_count, 5);
        assert_eq!(targets[1].host, "c.com");
        assert_eq!(targets[1].request_count, 3);
        assert_eq!(targets[2].host, "a.com");
        assert_eq!(targets[2].request_count, 1);
    }

    // ------------------------------------------------------------------
    // get_requests_for_replay_internal — DB CRUD tests
    // ------------------------------------------------------------------

    #[test]
    fn test_get_requests_for_replay_internal_unknown_host_returns_empty() {
        let conn = seeded_db();
        let requests = get_requests_for_replay_internal(&conn, "no-such-host.example").unwrap();
        assert!(requests.is_empty());
    }

    #[test]
    fn test_get_requests_for_replay_internal_round_trip() {
        let conn = seeded_db();
        let req_headers = vec![("User-Agent".to_string(), "proxybot-test".to_string())];
        let empty_headers: Vec<(String, String)> = vec![];

        let id1 = record_http_request(
            &conn,
            "2026-06-04 10:00:00",
            "GET",
            "https",
            "api.example.com",
            "/v1/users",
            &req_headers,
            None,
            Some(200),
            &empty_headers,
            Some("[]"),
            Some(50),
            None,
            Some("wechat"),
        )
        .unwrap();

        let id2 = record_http_request(
            &conn,
            "2026-06-04 10:00:01",
            "POST",
            "https",
            "api.example.com",
            "/v1/login",
            &empty_headers,
            Some(r#"{"u":"x"}"#),
            Some(201),
            &empty_headers,
            Some(r#"{"ok":true}"#),
            Some(120),
            None,
            None,
        )
        .unwrap();

        let requests = get_requests_for_replay_internal(&conn, "api.example.com").unwrap();
        assert_eq!(requests.len(), 2);

        // First request (by timestamp ASC)
        let r1 = &requests[0];
        assert_eq!(r1.id, id1);
        assert_eq!(r1.method, "GET");
        assert_eq!(r1.path, "/v1/users");
        // NOTE: url is scheme-less `host + path` (existing quirky behavior — see NOTE
        // in the source). We document the current behavior rather than fix it.
        assert_eq!(r1.url, "api.example.com/v1/users");
        assert_eq!(r1.req_headers, req_headers);
        assert_eq!(r1.req_body, None);

        let r2 = &requests[1];
        assert_eq!(r2.id, id2);
        assert_eq!(r2.method, "POST");
        assert_eq!(r2.path, "/v1/login");
        assert_eq!(r2.url, "api.example.com/v1/login");
        assert_eq!(r2.req_body.as_deref(), Some(r#"{"u":"x"}"#));
    }

    #[test]
    fn test_get_requests_for_replay_internal_filters_by_host() {
        let conn = seeded_db();
        let empty_headers: Vec<(String, String)> = vec![];

        record_http_request(&conn, "2026-06-04 10:00:00", "GET", "https", "a.com", "/", &empty_headers, None, Some(200), &empty_headers, None, None, None, None).unwrap();
        record_http_request(&conn, "2026-06-04 10:00:01", "GET", "https", "b.com", "/", &empty_headers, None, Some(200), &empty_headers, None, None, None, None).unwrap();
        record_http_request(&conn, "2026-06-04 10:00:02", "GET", "https", "a.com", "/v2", &empty_headers, None, Some(200), &empty_headers, None, None, None, None).unwrap();

        let a = get_requests_for_replay_internal(&conn, "a.com").unwrap();
        let b = get_requests_for_replay_internal(&conn, "b.com").unwrap();
        assert_eq!(a.len(), 2, "Only a.com rows should be returned");
        assert_eq!(b.len(), 1, "Only b.com rows should be returned");
        assert!(a.iter().all(|r| r.url.starts_with("a.com")));
        assert!(b.iter().all(|r| r.url.starts_with("b.com")));
    }

    #[test]
    fn test_get_requests_for_replay_internal_handles_malformed_header_json() {
        let conn = seeded_db();
        // Seed a row with intentionally malformed JSON in req_headers.
        conn.execute(
            "INSERT INTO http_requests (timestamp, method, scheme, host, path, req_headers, req_body, resp_status, resp_headers, resp_body)
             VALUES ('2026-06-04 10:00:00', 'GET', 'https', 'broken.com', '/', 'this-is-not-json', NULL, 200, '[]', NULL)",
            [],
        )
        .unwrap();

        let requests = get_requests_for_replay_internal(&conn, "broken.com").unwrap();
        assert_eq!(requests.len(), 1, "Malformed JSON must not skip the row");
        // req_headers is silently default-empty (current behavior — see NOTE in source).
        assert!(requests[0].req_headers.is_empty());
    }

    // ------------------------------------------------------------------
    // get_recorded_responses_internal — DB CRUD tests
    // ------------------------------------------------------------------

    #[test]
    fn test_get_recorded_responses_internal_empty_ids_returns_empty_map() {
        let conn = seeded_db();
        let responses = get_recorded_responses_internal(&conn, &[]).unwrap();
        assert!(responses.is_empty());
    }

    #[test]
    fn test_get_recorded_responses_internal_round_trip() {
        let conn = seeded_db();
        let req_headers: Vec<(String, String)> = vec![];
        let resp_headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Server".to_string(), "nginx/1.18".to_string()),
        ];

        let id1 = record_http_request(
            &conn,
            "2026-06-04 10:00:00",
            "GET",
            "https",
            "api.example.com",
            "/v1/users",
            &req_headers,
            None,
            Some(200),
            &resp_headers,
            Some(r#"[{"id":1}]"#),
            Some(42),
            None,
            None,
        )
        .unwrap();

        let id2 = record_http_request(
            &conn,
            "2026-06-04 10:00:01",
            "POST",
            "https",
            "api.example.com",
            "/v1/login",
            &req_headers,
            Some("{}"),
            Some(404),
            &resp_headers,
            None,
            Some(11),
            None,
            None,
        )
        .unwrap();

        let responses = get_recorded_responses_internal(&conn, &[id1, id2]).unwrap();
        assert_eq!(responses.len(), 2, "Both IDs should be in the returned map");

        let r1 = responses.get(&id1).expect("id1 must be present");
        assert_eq!(r1.status, 200);
        assert_eq!(r1.headers, resp_headers);
        assert_eq!(r1.body.as_deref(), Some(r#"[{"id":1}]"#));

        let r2 = responses.get(&id2).expect("id2 must be present");
        assert_eq!(r2.status, 404);
        assert_eq!(r2.body, None, "Body was inserted as NULL");
    }

    #[test]
    fn test_get_recorded_responses_internal_skips_unknown_ids_silently() {
        let conn = seeded_db();
        let empty_headers: Vec<(String, String)> = vec![];
        let id = record_http_request(
            &conn,
            "2026-06-04 10:00:00",
            "GET",
            "https",
            "api.example.com",
            "/v1/users",
            &empty_headers,
            None,
            Some(200),
            &empty_headers,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        // Pass [id, 99999] — id 99999 does not exist.
        let responses = get_recorded_responses_internal(&conn, &[id, 99_999]).unwrap();
        assert_eq!(responses.len(), 1, "Unknown ids are silently dropped");
        assert!(responses.contains_key(&id));
        assert!(!responses.contains_key(&99_999));
    }

    #[test]
    fn test_get_recorded_responses_internal_normalizes_null_status_to_zero() {
        let conn = seeded_db();
        // Insert a row with resp_status = NULL (response was incomplete).
        conn.execute(
            "INSERT INTO http_requests (timestamp, method, scheme, host, path, req_headers, resp_status, resp_headers)
             VALUES ('2026-06-04 10:00:00', 'GET', 'https', 'partial.com', '/', '[]', NULL, '[]')",
            [],
        )
        .unwrap();
        let id: i64 = conn
            .query_row("SELECT id FROM http_requests WHERE host='partial.com'", [], |row| row.get(0))
            .unwrap();

        let responses = get_recorded_responses_internal(&conn, &[id]).unwrap();
        let r = responses.get(&id).expect("Row should still be returned");
        assert_eq!(r.status, 0, "NULL resp_status must be normalized to 0");
    }

    #[test]
    fn test_get_recorded_responses_internal_handles_binary_body() {
        let conn = seeded_db();
        // Non-UTF8 body: 0xFF 0xFE 0x00 0x41 ("\xFF\xFE\x00A")
        let empty_headers: Vec<(String, String)> = vec![];
        let id = record_http_request(
            &conn,
            "2026-06-04 10:00:00",
            "GET",
            "https",
            "bin.example.com",
            "/blob",
            &empty_headers,
            None,
            Some(200),
            &empty_headers,
            None, // body = None
            None,
            None,
            None,
        )
        .unwrap();

        // Now write some binary data directly via raw SQL so we don't go through
        // record_http_request's UTF-8 assumption.
        conn.execute(
            "UPDATE http_requests SET resp_body = X'FFFE0041' WHERE id = ?1",
            [id],
        )
        .unwrap();

        let responses = get_recorded_responses_internal(&conn, &[id]).unwrap();
        let r = responses.get(&id).unwrap();
        let body = r.body.as_ref().expect("body should be Some after raw bytes update");
        // Body bytes are lossy-decoded; the result starts with U+FFFD replacements.
        assert!(body.starts_with('\u{FFFD}'), "Non-UTF8 bytes must be lossy-decoded to replacement chars");
    }

    // ------------------------------------------------------------------
    // ReplayState defaults — pure constructor test
    // ------------------------------------------------------------------

    #[test]
    fn test_replay_state_default_values() {
        let state = ReplayState::default();
        assert_eq!(state.mock_port, 19998, "Default mock_port should be 19998");
        assert!(!*state.is_running.lock().unwrap(), "Default is_running should be false");
        assert!(state.results.lock().unwrap().is_empty(), "Default results should be empty");
    }

    // ------------------------------------------------------------------
    // NOTE: start_replay (the mock-server path-matching logic) is NOT
    // covered by a unit test. The path check inside the request loop
    // (`path == "/" || path.starts_with("/")`) is broken — see task #80.
    // start_replay spins up a real tokio listener and binds to a port;
    // exercising it from a unit test would require fixtures (a live port,
    // a tokio runtime, request fakes) that are beyond this audit's
    // scope. The bug is characterized in the source comment at
    // `start_replay`; whoever picks up task #80 should also add the
    // first test that exercises that path.
    // ------------------------------------------------------------------
}
