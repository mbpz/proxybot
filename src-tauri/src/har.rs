//! HAR (HTTP Archive) export module for ProxyBot.
//!
//! Generates HAR 1.2 format files from recorded HTTP requests.

use crate::db::DbState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

/// HAR 1.2 specification structures
#[derive(Serialize, Deserialize)]
pub struct HarFile {
    #[serde(rename = "log")]
    pub log: HarLog,
}

#[derive(Serialize, Deserialize)]
pub struct HarLog {
    pub version: String,
    pub creator: HarCreator,
    pub entries: Vec<HarEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct HarEntry {
    #[serde(rename = "startedDateTime")]
    pub started_date_time: String,
    pub time: f64,
    pub request: HarRequest,
    pub response: HarResponse,
    pub timings: HarTimings,
}

#[derive(Serialize, Deserialize)]
pub struct HarRequest {
    pub method: String,
    pub url: String,
    #[serde(rename = "httpVersion")]
    pub http_version: String,
    pub headers: Vec<HarHeader>,
    #[serde(rename = "queryString")]
    pub query_string: Vec<HarQueryParam>,
    #[serde(rename = "postData")]
    pub post_data: Option<HarPostData>,
    #[serde(rename = "headersSize")]
    pub headers_size: i64,
    #[serde(rename = "bodySize")]
    pub body_size: i64,
}

#[derive(Serialize, Deserialize)]
pub struct HarResponse {
    pub status: u16,
    #[serde(rename = "statusText")]
    pub status_text: String,
    #[serde(rename = "httpVersion")]
    pub http_version: String,
    pub headers: Vec<HarHeader>,
    pub content: HarContent,
    #[serde(rename = "headersSize")]
    pub headers_size: i64,
    #[serde(rename = "bodySize")]
    pub body_size: i64,
}

#[derive(Serialize, Deserialize)]
pub struct HarHeader {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct HarQueryParam {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct HarPostData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct HarContent {
    pub size: i64,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub text: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct HarTimings {
    pub blocked: f64,
    pub dns: f64,
    pub connect: f64,
    pub send: f64,
    pub wait: f64,
    pub receive: f64,
}

/// Internal helper: export recorded HTTP requests to HAR 1.2 format from a raw connection.
fn export_har_internal(conn: &rusqlite::Connection) -> Result<HarFile, String> {
    let mut stmt = conn
        .prepare(
            "SELECT timestamp, method, scheme, host, path, req_headers, req_body,
                    resp_status, resp_headers, resp_body, duration_ms
             FROM http_requests
             ORDER BY timestamp ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let timestamp: String = row.get(0)?;
            let method: String = row.get(1)?;
            let scheme: String = row.get(2)?;
            let host: String = row.get(3)?;
            let path: String = row.get(4)?;
            let req_headers: String = row.get(5)?;
            let req_body: Option<Vec<u8>> = row.get(6)?;
            let resp_status: Option<u16> = row.get(7)?;
            let resp_headers: String = row.get(8)?;
            let resp_body: Option<Vec<u8>> = row.get(9)?;
            let duration_ms: Option<i64> = row.get(10)?;
            Ok((
                timestamp,
                method,
                scheme,
                host,
                path,
                req_headers,
                req_body,
                resp_status,
                resp_headers,
                resp_body,
                duration_ms,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut entries = Vec::new();

    for row in rows {
        let (
            timestamp,
            method,
            scheme,
            host,
            path,
            req_headers_json,
            req_body,
            resp_status,
            resp_headers_json,
            resp_body,
            duration_ms,
        ) = row.map_err(|e| e.to_string())?;

        // Parse request headers
        let req_headers: Vec<(String, String)> =
            serde_json::from_str(&req_headers_json).unwrap_or_default();
        let request_url = format!("{}://{}{}", scheme, host, path);

        // Build request query string from path
        let query_string: Vec<HarQueryParam> = if let Some(query) = path.split('?').nth(1) {
            query
                .split('&')
                .filter_map(|param| {
                    let mut parts = param.splitn(2, '=');
                    Some(HarQueryParam {
                        name: parts.next().unwrap_or("").to_string(),
                        value: parts.next().unwrap_or("").to_string(),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        // Parse request body
        let req_body_text = req_body
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).to_string());
        let req_content_type = req_headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.clone());

        let post_data = if req_body_text.is_some() {
            Some(HarPostData {
                mime_type: req_content_type
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
                text: req_body_text,
            })
        } else {
            None
        };

        let req_headers_size = req_headers
            .iter()
            .map(|(n, v)| n.len() + v.len() + 4) // "name: value\r\n"
            .sum::<usize>() as i64;
        let req_body_size = req_body.map(|b| b.len() as i64).unwrap_or(-1);

        // Parse response headers
        let resp_headers: Vec<(String, String)> =
            serde_json::from_str(&resp_headers_json).unwrap_or_default();
        let resp_content_type = resp_headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.clone());

        // Parse response body
        let resp_body_text = resp_body
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).to_string());
        let resp_body_size = resp_body.map(|b| b.len() as i64).unwrap_or(-1);

        let resp_status_text = match resp_status.unwrap_or(0) {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "",
        };

        let resp_headers_size = resp_headers
            .iter()
            .map(|(n, v)| n.len() + v.len() + 4)
            .sum::<usize>() as i64;

        // Convert timestamp to HAR format (ISO 8601)
        let started_date_time = parse_timestamp_to_iso(&timestamp);

        // Calculate time in milliseconds
        let time = duration_ms.unwrap_or(0) as f64;

        // Build HAR entry
        let entry = HarEntry {
            started_date_time,
            time,
            request: HarRequest {
                method,
                url: request_url,
                http_version: "HTTP/1.1".to_string(),
                headers: req_headers
                    .into_iter()
                    .map(|(n, v)| HarHeader { name: n, value: v })
                    .collect(),
                query_string,
                post_data,
                headers_size: req_headers_size,
                body_size: req_body_size,
            },
            response: HarResponse {
                status: resp_status.unwrap_or(0),
                status_text: resp_status_text.to_string(),
                http_version: "HTTP/1.1".to_string(),
                headers: resp_headers
                    .into_iter()
                    .map(|(n, v)| HarHeader { name: n, value: v })
                    .collect(),
                content: HarContent {
                    size: resp_body_size.max(0),
                    mime_type: resp_content_type
                        .unwrap_or_else(|| "application/octet-stream".to_string()),
                    text: resp_body_text,
                },
                headers_size: resp_headers_size,
                body_size: resp_body_size,
            },
            timings: HarTimings {
                blocked: -1.0,
                dns: -1.0,
                connect: -1.0,
                send: 0.0,
                wait: time,
                receive: 0.0,
            },
        };

        entries.push(entry);
    }

    log::info!(
        "Exported HAR with {} entries",
        entries.len()
    );

    Ok(HarFile {
        log: HarLog {
            version: "1.2".to_string(),
            creator: HarCreator {
                name: "ProxyBot".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            entries,
        },
    })
}

/// Export recorded HTTP requests to HAR 1.2 format.
#[tauri::command]
pub fn export_har(state: State<'_, Arc<DbState>>, session_name: String) -> Result<HarFile, String> {
    log::info!("Exporting HAR for session: {}", session_name);
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    export_har_internal(&conn)
}

/// Parse timestamp string (Unix epoch with milliseconds) to ISO 8601 format.
fn parse_timestamp_to_iso(timestamp: &str) -> String {
    // Format: "1234567890.123"
    let parts: Vec<&str> = timestamp.split('.').collect();
    let secs: i64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let millis: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    // Convert to ISO 8601
    let dt = chrono_lite_to_datetime(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        dt.0, dt.1, dt.2, dt.3, dt.4, dt.5, millis
    )
}

/// Convert Unix timestamp to (year, month, day, hour, minute, second).
fn chrono_lite_to_datetime(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let mut remaining = secs as u64;

    // Years
    let mut year = 1970i64;
    loop {
        let days_in_year = if is_leap_year(year as u64) { 366 } else { 365 };
        if remaining < days_in_year * 86400 {
            break;
        }
        remaining -= days_in_year * 86400;
        year += 1;
    }

    // Months
    let days_in_months: &[u64] = if is_leap_year(year as u64) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u32;
    for days in days_in_months {
        if remaining < days * 86400 {
            break;
        }
        remaining -= days * 86400;
        month += 1;
    }

    // Days, hours, minutes, seconds
    let day = (remaining / 86400) + 1;
    remaining %= 86400;
    let hour = remaining / 3600;
    remaining %= 3600;
    let minute = remaining / 60;
    let second = remaining % 60;

    (
        year,
        month,
        day as u32,
        hour as u32,
        minute as u32,
        second as u32,
    )
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Internal helper: save HAR JSON to a given directory.
fn save_har_file_internal(
    har_json: &str,
    session_name: &str,
    dir: &std::path::Path,
) -> Result<String, String> {
    let har: HarFile = serde_json::from_str(har_json).map_err(|e| e.to_string())?;

    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let filename = format!("{}.har", session_name);
    let path = dir.join(&filename);

    let json = serde_json::to_string_pretty(&har).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;

    log::info!("Saved HAR file to {:?}", path);
    Ok(path.to_string_lossy().to_string())
}

/// Save HAR file to disk.
#[tauri::command]
pub fn save_har_file(har_json: String, session_name: String) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home).join(".proxybot").join("exports");
    save_har_file_internal(&har_json, &session_name, &dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{record_http_request, DbState};
    use rusqlite::Connection;

    /// Helper: create an in-memory DB with schema initialized.
    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();
        conn
    }

    /// Helper: build a minimal valid HarFile JSON for save_har_file_internal tests.
    fn sample_har_json() -> String {
        let har = HarFile {
            log: HarLog {
                version: "1.2".to_string(),
                creator: HarCreator {
                    name: "ProxyBot".to_string(),
                    version: "1.0.0".to_string(),
                },
                entries: vec![],
            },
        };
        serde_json::to_string(&har).unwrap()
    }

    // ------------------------------------------------------------------
    // Pre-existing tests (kept verbatim)
    // ------------------------------------------------------------------

    #[test]
    fn test_timestamp_to_iso() {
        // 2024-01-01 00:00:00 UTC
        let result = parse_timestamp_to_iso("1704067200.000");
        assert!(result.starts_with("2024-01-01T"));
    }

    #[test]
    fn test_har_structure() {
        let har = HarFile {
            log: HarLog {
                version: "1.2".to_string(),
                creator: HarCreator {
                    name: "ProxyBot".to_string(),
                    version: "1.0.0".to_string(),
                },
                entries: vec![HarEntry {
                    started_date_time: "2024-01-01T00:00:00.000Z".to_string(),
                    time: 123.0,
                    request: HarRequest {
                        method: "GET".to_string(),
                        url: "https://example.com/path".to_string(),
                        http_version: "HTTP/1.1".to_string(),
                        headers: vec![HarHeader {
                            name: "Host".to_string(),
                            value: "example.com".to_string(),
                        }],
                        query_string: vec![],
                        post_data: None,
                        headers_size: 10,
                        body_size: -1,
                    },
                    response: HarResponse {
                        status: 200,
                        status_text: "OK".to_string(),
                        http_version: "HTTP/1.1".to_string(),
                        headers: vec![],
                        content: HarContent {
                            size: 0,
                            mime_type: "text/html".to_string(),
                            text: None,
                        },
                        headers_size: 0,
                        body_size: -1,
                    },
                    timings: HarTimings {
                        blocked: -1.0,
                        dns: -1.0,
                        connect: -1.0,
                        send: 0.0,
                        wait: 123.0,
                        receive: 0.0,
                    },
                }],
            },
        };

        let json = serde_json::to_string(&har).unwrap();
        assert!(json.contains("\"version\":\"1.2\""), "version missing");
        assert!(json.contains("\"method\":\"GET\""), "method missing");
        assert!(json.contains("\"status\":200"), "status missing");
    }

    // ------------------------------------------------------------------
    // Pure helper tests: chrono_lite_to_datetime
    // ------------------------------------------------------------------

    #[test]
    fn test_chrono_lite_to_datetime_epoch() {
        let (y, m, d, h, min, s) = chrono_lite_to_datetime(0);
        assert_eq!((y, m, d, h, min, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn test_chrono_lite_to_datetime_2024_01_01() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        let (y, m, d, h, min, s) = chrono_lite_to_datetime(1704067200);
        assert_eq!(y, 2024);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
        assert_eq!(h, 0);
        assert_eq!(min, 0);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_chrono_lite_to_datetime_with_time() {
        // 2024-06-15 12:30:45 UTC
        // 2024-01-01 = 1704067200
        // Jan=31, Feb=29(leap), Mar=31, Apr=30, May=31 = 152 days to Jun 1
        // Jun 15 = 152 + 14 = 166 days from Jan 1
        // 166 * 86400 = 14342400
        // 12*3600 + 30*60 + 45 = 45045
        let ts = 1704067200 + 14342400 + 45045;
        let (y, m, d, h, min, s) = chrono_lite_to_datetime(ts);
        assert_eq!(y, 2024);
        assert_eq!(m, 6);
        assert_eq!(d, 15);
        assert_eq!(h, 12);
        assert_eq!(min, 30);
        assert_eq!(s, 45);
    }

    // ------------------------------------------------------------------
    // Pure helper tests: is_leap_year
    // ------------------------------------------------------------------

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000), "divisible by 400");
        assert!(!is_leap_year(1900), "divisible by 100 but not 400");
        assert!(is_leap_year(2024), "divisible by 4 and not 100");
        assert!(!is_leap_year(2023), "not divisible by 4");
        assert!(!is_leap_year(1970), "epoch year");
    }

    // ------------------------------------------------------------------
    // Pure helper tests: parse_timestamp_to_iso
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_timestamp_to_iso_full_format() {
        // 1704067200 = 2024-01-01 00:00:00 UTC
        let result = parse_timestamp_to_iso("1704067200.500");
        assert_eq!(result, "2024-01-01T00:00:00.500Z");
    }

    #[test]
    fn test_parse_timestamp_to_iso_no_millis() {
        let result = parse_timestamp_to_iso("1704067200");
        assert_eq!(result, "2024-01-01T00:00:00.000Z");
    }

    #[test]
    fn test_parse_timestamp_to_iso_epoch_zero() {
        let result = parse_timestamp_to_iso("0.000");
        assert_eq!(result, "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn test_parse_timestamp_to_iso_empty_string() {
        // Malformed input: empty string -> defaults to epoch 0
        let result = parse_timestamp_to_iso("");
        assert_eq!(result, "1970-01-01T00:00:00.000Z");
    }

    // ------------------------------------------------------------------
    // DB tests: export_har_internal
    // ------------------------------------------------------------------

    #[test]
    fn test_export_har_internal_empty_db() {
        let conn = test_db();
        let har = export_har_internal(&conn).unwrap();
        assert_eq!(har.log.version, "1.2");
        assert_eq!(har.log.creator.name, "ProxyBot");
        assert!(har.log.entries.is_empty(), "Empty DB should produce zero entries");
    }

    #[test]
    fn test_export_har_internal_single_get_request() {
        let conn = test_db();
        let req_headers: Vec<(String, String)> = vec![
            ("Host".into(), "example.com".into()),
            ("User-Agent".into(), "test/1.0".into()),
        ];
        let resp_headers: Vec<(String, String)> = vec![
            ("Content-Type".into(), "text/html".into()),
        ];

        record_http_request(
            &conn,
            "1704067200.000",
            "GET",
            "https",
            "example.com",
            "/index.html",
            &req_headers,
            None,
            Some(200),
            &resp_headers,
            Some("<html></html>"),
            Some(42),
            None,
            None,
        )
        .unwrap();

        let har = export_har_internal(&conn).unwrap();
        assert_eq!(har.log.entries.len(), 1);

        let entry = &har.log.entries[0];
        assert_eq!(entry.request.method, "GET");
        assert_eq!(entry.request.url, "https://example.com/index.html");
        assert_eq!(entry.request.http_version, "HTTP/1.1");
        assert_eq!(entry.request.headers.len(), 2);
        assert_eq!(entry.request.headers[0].name, "Host");
        assert_eq!(entry.request.query_string.len(), 0);
        assert!(entry.request.post_data.is_none(), "GET should have no post_data");
        assert_eq!(entry.request.body_size, -1, "No body -> -1");

        assert_eq!(entry.response.status, 200);
        assert_eq!(entry.response.status_text, "OK");
        assert_eq!(entry.response.content.text.as_deref(), Some("<html></html>"));
        assert_eq!(entry.response.content.mime_type, "text/html");
        assert_eq!(entry.response.body_size, "<html></html>".len() as i64);

        assert_eq!(entry.time, 42.0);
        assert_eq!(entry.started_date_time, "2024-01-01T00:00:00.000Z");
        assert_eq!(entry.timings.wait, 42.0);
        assert_eq!(entry.timings.blocked, -1.0);
    }

    #[test]
    fn test_export_har_internal_query_string_parsing() {
        let conn = test_db();
        let empty: Vec<(String, String)> = vec![];

        record_http_request(
            &conn,
            "1704067200.000",
            "GET",
            "https",
            "api.example.com",
            "/search?q=rust&lang=en",
            &empty,
            None,
            Some(200),
            &empty,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let har = export_har_internal(&conn).unwrap();
        let entry = &har.log.entries[0];

        assert_eq!(entry.request.url, "https://api.example.com/search?q=rust&lang=en");
        assert_eq!(entry.request.query_string.len(), 2);
        assert_eq!(entry.request.query_string[0].name, "q");
        assert_eq!(entry.request.query_string[0].value, "rust");
        assert_eq!(entry.request.query_string[1].name, "lang");
        assert_eq!(entry.request.query_string[1].value, "en");
    }

    #[test]
    fn test_export_har_internal_post_with_body() {
        let conn = test_db();
        let req_headers: Vec<(String, String)> = vec![
            ("Content-Type".into(), "application/json".into()),
        ];
        let resp_headers: Vec<(String, String)> = vec![];

        record_http_request(
            &conn,
            "1704067200.000",
            "POST",
            "https",
            "api.example.com",
            "/submit",
            &req_headers,
            Some(r#"{"key":"value"}"#),
            Some(201),
            &resp_headers,
            Some(r#"{"ok":true}"#),
            Some(100),
            None,
            None,
        )
        .unwrap();

        let har = export_har_internal(&conn).unwrap();
        let entry = &har.log.entries[0];

        // Request post_data
        let pd = entry.request.post_data.as_ref().expect("POST should have post_data");
        assert_eq!(pd.mime_type, "application/json");
        assert_eq!(pd.text.as_deref(), Some(r#"{"key":"value"}"#));
        assert_eq!(entry.request.body_size, r#"{"key":"value"}"#.len() as i64);

        // Response status
        assert_eq!(entry.response.status, 201);
        assert_eq!(entry.response.status_text, "Created");
        assert_eq!(entry.response.content.text.as_deref(), Some(r#"{"ok":true}"#));
    }

    #[test]
    fn test_export_har_internal_utf8_body() {
        let conn = test_db();
        let empty: Vec<(String, String)> = vec![];
        let body = "Hello \u{4e16}\u{754c}!"; // "Hello World!" in Chinese

        record_http_request(
            &conn,
            "1704067200.000",
            "POST",
            "https",
            "i18n.example.com",
            "/text",
            &empty,
            Some(body),
            Some(200),
            &empty,
            Some(body),
            Some(50),
            None,
            None,
        )
        .unwrap();

        let har = export_har_internal(&conn).unwrap();
        let entry = &har.log.entries[0];

        assert_eq!(
            entry.request.post_data.as_ref().unwrap().text.as_deref(),
            Some(body),
            "UTF-8 body should survive round-trip"
        );
        assert_eq!(
            entry.response.content.text.as_deref(),
            Some(body),
            "UTF-8 response body should survive round-trip"
        );
    }

    #[test]
    fn test_export_har_internal_missing_resp_status_defaults_to_zero() {
        let conn = test_db();
        let empty: Vec<(String, String)> = vec![];

        record_http_request(
            &conn,
            "1704067200.000",
            "GET",
            "http",
            "example.com",
            "/timeout",
            &empty,
            None,
            None, // no response status
            &empty,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let har = export_har_internal(&conn).unwrap();
        let entry = &har.log.entries[0];

        assert_eq!(entry.response.status, 0, "Missing status should default to 0");
        assert_eq!(entry.response.status_text, "", "Status 0 -> empty status text");
        assert_eq!(entry.time, 0.0, "Missing duration -> 0.0");
    }

    #[test]
    fn test_export_har_internal_multiple_entries_ordered_by_timestamp() {
        let conn = test_db();
        let empty: Vec<(String, String)> = vec![];

        // Insert in reverse chronological order
        record_http_request(&conn, "1704067202.000", "GET", "https", "c.com", "/", &empty, None, Some(200), &empty, None, None, None, None).unwrap();
        record_http_request(&conn, "1704067200.000", "GET", "https", "a.com", "/", &empty, None, Some(200), &empty, None, None, None, None).unwrap();
        record_http_request(&conn, "1704067201.000", "GET", "https", "b.com", "/", &empty, None, Some(200), &empty, None, None, None, None).unwrap();

        let har = export_har_internal(&conn).unwrap();
        assert_eq!(har.log.entries.len(), 3);
        assert_eq!(har.log.entries[0].request.url, "https://a.com/");
        assert_eq!(har.log.entries[1].request.url, "https://b.com/");
        assert_eq!(har.log.entries[2].request.url, "https://c.com/");
    }

    #[test]
    fn test_export_har_internal_post_without_content_type_defaults_octet_stream() {
        let conn = test_db();
        let empty: Vec<(String, String)> = vec![];

        record_http_request(
            &conn,
            "1704067200.000",
            "POST",
            "https",
            "example.com",
            "/upload",
            &empty, // no Content-Type header
            Some("raw bytes"),
            Some(200),
            &empty,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let har = export_har_internal(&conn).unwrap();
        let entry = &har.log.entries[0];

        let pd = entry.request.post_data.as_ref().unwrap();
        assert_eq!(pd.mime_type, "application/octet-stream", "Missing Content-Type -> octet-stream default");

        assert_eq!(
            entry.response.content.mime_type, "application/octet-stream",
            "Missing response Content-Type -> octet-stream default"
        );
    }

    #[test]
    fn test_export_har_internal_status_text_mapping() {
        let conn = test_db();
        let empty: Vec<(String, String)> = vec![];

        let cases: Vec<(u16, &str)> = vec![
            (200, "OK"),
            (201, "Created"),
            (204, "No Content"),
            (301, "Moved Permanently"),
            (302, "Found"),
            (304, "Not Modified"),
            (400, "Bad Request"),
            (401, "Unauthorized"),
            (403, "Forbidden"),
            (404, "Not Found"),
            (500, "Internal Server Error"),
            (502, "Bad Gateway"),
            (503, "Service Unavailable"),
        ];

        // Use zero-padded indices so string sort == numeric sort
        for (i, (status, expected_text)) in cases.iter().enumerate() {
            record_http_request(
                &conn,
                &format!("17040672{:02}.000", i),
                "GET",
                "https",
                "example.com",
                &format!("/status/{}", status),
                &empty,
                None,
                Some(*status),
                &empty,
                None,
                None,
                None,
                None,
            )
            .unwrap();
        }

        // Insert an unknown status code too (higher timestamp so it sorts last)
        record_http_request(
            &conn,
            "1704067299.000",
            "GET",
            "https",
            "example.com",
            "/status/418",
            &empty,
            None,
            Some(418),
            &empty,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let har = export_har_internal(&conn).unwrap();
        assert_eq!(har.log.entries.len(), cases.len() + 1);

        // Look up by status code instead of assuming index order
        for (status, expected_text) in &cases {
            let entry = har.log.entries.iter()
                .find(|e| e.response.status == *status)
                .unwrap_or_else(|| panic!("No entry found for status {}", status));
            assert_eq!(
                entry.response.status_text, *expected_text,
                "Status {} should map to '{}'",
                status, expected_text
            );
        }
        // Unknown status -> empty string
        let unknown = har.log.entries.iter().find(|e| e.response.status == 418).unwrap();
        assert_eq!(unknown.response.status_text, "");
    }

    // ------------------------------------------------------------------
    // FS tests: save_har_file_internal
    // ------------------------------------------------------------------

    #[test]
    fn test_save_har_file_internal_writes_to_temp_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let har_json = sample_har_json();

        let path = save_har_file_internal(&har_json, "test-session", tmp.path()).unwrap();

        assert!(path.ends_with("test-session.har"), "Path should end with session_name.har");
        assert!(std::path::Path::new(&path).exists(), "File should exist on disk");

        let contents = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["log"]["version"], "1.2");
    }

    #[test]
    fn test_save_har_file_internal_invalid_json_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let result = save_har_file_internal("not valid json", "test", tmp.path());
        assert!(result.is_err(), "Invalid JSON should return Err");
    }
}
