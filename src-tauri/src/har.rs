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

/// Export recorded HTTP requests to HAR 1.2 format.
#[tauri::command]
pub fn export_har(state: State<'_, Arc<DbState>>, session_name: String) -> Result<HarFile, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
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
            Ok((timestamp, method, scheme, host, path, req_headers, req_body, resp_status, resp_headers, resp_body, duration_ms))
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
        let req_headers: Vec<(String, String)> = serde_json::from_str(&req_headers_json)
            .unwrap_or_default();
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
        let req_body_text = req_body.as_ref().map(|b| String::from_utf8_lossy(b).to_string());
        let req_content_type = req_headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.clone());

        let post_data = if req_body_text.is_some() {
            Some(HarPostData {
                mime_type: req_content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
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
        let resp_headers: Vec<(String, String)> = serde_json::from_str(&resp_headers_json)
            .unwrap_or_default();
        let resp_content_type = resp_headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.clone());

        // Parse response body
        let resp_body_text = resp_body.as_ref().map(|b| String::from_utf8_lossy(b).to_string());
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
                    mime_type: resp_content_type.unwrap_or_else(|| "application/octet-stream".to_string()),
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
        "Exported HAR file '{}' with {} entries (session: {})",
        format!("{}.har", session_name),
        entries.len(),
        session_name
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

/// Parse timestamp string (Unix epoch with milliseconds) to ISO 8601 format.
fn parse_timestamp_to_iso(timestamp: &str) -> String {
    // Format: "1234567890.123"
    let parts: Vec<&str> = timestamp.split('.').collect();
    let secs: i64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let millis: u32 = parts
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

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

    (year, month, day as u32, hour as u32, minute as u32, second as u32)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Save HAR file to disk.
#[tauri::command]
pub fn save_har_file(har_json: String, session_name: String) -> Result<String, String> {
    let har: HarFile = serde_json::from_str(&har_json).map_err(|e| e.to_string())?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home).join(".proxybot").join("exports");

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let filename = format!("{}.har", session_name);
    let path = dir.join(&filename);

    let json = serde_json::to_string_pretty(&har).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;

    log::info!("Saved HAR file to {:?}", path);
    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(json.contains("\"version\": \"1.2\""));
        assert!(json.contains("\"method\": \"GET\""));
        assert!(json.contains("\"status\": 200"));
    }
}
