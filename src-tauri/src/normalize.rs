//! Traffic normalizer module.
//!
//! Converts HTTP traffic into structured records for AI analysis.
//! Body parser detects JSON, Protobuf (base64), and GraphQL variants.

use crate::db::DbState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::State;

// ============================================================================
// Normalized Record Types
// ============================================================================

/// Normalized HTTP exchange record for AI analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedRecord {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub query: Value,
    pub request_headers: Value,
    pub request_body: Value,
    pub response_status: u16,
    pub response_headers: Value,
    pub response_body: Value,
    pub timing_ms: i64,
    pub device_id: Option<i64>,
}

/// Parsed body content with detected format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedBody {
    pub format: BodyFormat,
    pub parsed: Value,
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BodyFormat {
    Json,
    Protobuf,
    GraphQL,
    FormData,
    Text,
    Binary,
}

/// Paginated traffic response.
#[derive(Debug, Clone, Serialize)]
pub struct TrafficPage {
    pub records: Vec<NormalizedRecord>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub has_more: bool,
}

// ============================================================================
// Body Parsing Functions
// ============================================================================

/// Parse query string into JSON object.
pub fn parse_query_params(query: &str) -> Value {
    let mut obj = serde_json::Map::new();
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            obj.insert(
                url_decode(key),
                url_decode_value(value),
            );
        } else if !pair.is_empty() {
            obj.insert(pair.to_string(), Value::Null);
        }
    }
    Value::Object(obj)
}

fn url_decode(s: &str) -> String {
    // Simple URL decode - replace %XX with character
    s.replace("%20", " ")
        .replace("%3D", "=")
        .replace("%26", "&")
}

fn url_decode_value(s: &str) -> Value {
    Value::String(url_decode(s))
}

/// Parse request/response body and detect format.
pub fn parse_body(body: &[u8], content_type: Option<&str>) -> ParsedBody {
    // First try to parse as UTF-8 string
    let text = match String::from_utf8(body.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            // Binary data - try base64 for Protobuf
            return ParsedBody {
                format: BodyFormat::Binary,
                parsed: Value::Null,
                raw: Some(base64_encode(body)),
            };
        }
    };

    // Check content type hint first
    if let Some(ct) = content_type {
        let ct_lower = ct.to_lowercase();
        if ct_lower.contains("application/json") || ct_lower.contains("+json") {
            if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                if is_graphql(&parsed) {
                    return ParsedBody {
                        format: BodyFormat::GraphQL,
                        parsed,
                        raw: None,
                    };
                }
                return ParsedBody {
                    format: BodyFormat::Json,
                    parsed,
                    raw: None,
                };
            }
        } else if ct_lower.contains("application/x-protobuf") || ct_lower.contains("application/protobuf") {
            return ParsedBody {
                format: BodyFormat::Protobuf,
                parsed: Value::String(base64_encode(body)),
                raw: None,
            };
        } else if ct_lower.contains("application/x-www-form-urlencoded") {
            if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                return ParsedBody {
                    format: BodyFormat::FormData,
                    parsed,
                    raw: None,
                };
            }
        } else if ct_lower.contains("text/") {
            return ParsedBody {
                format: BodyFormat::Text,
                parsed: Value::String(text),
                raw: None,
            };
        }
    }

    // Try JSON parsing
    if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
        if is_graphql(&parsed) {
            return ParsedBody {
                format: BodyFormat::GraphQL,
                parsed,
                raw: None,
            };
        }
        return ParsedBody {
            format: BodyFormat::Json,
            parsed,
            raw: None,
        };
    }

    // Check for Protobuf binary markers
    if is_probably_protobuf(body) {
        return ParsedBody {
            format: BodyFormat::Protobuf,
            parsed: Value::String(base64_encode(body)),
            raw: None,
        };
    }

    // Default to text
    ParsedBody {
        format: BodyFormat::Text,
        parsed: Value::String(text),
        raw: None,
    }
}

/// Check if parsed JSON represents a GraphQL query.
fn is_graphql(parsed: &Value) -> bool {
    if let Some(obj) = parsed.as_object() {
        return obj.contains_key("query") || obj.contains_key("variables") || obj.contains_key("operationName");
    }
    false
}

/// Heuristic check for Protobuf binary data.
fn is_probably_protobuf(body: &[u8]) -> bool {
    if body.len() < 2 || body.len() > 1024 * 1024 {
        return false;
    }

    let mut has_varint = false;
    let mut i = 0;
    let mut count = 0;

    while i < body.len() && count < 100 {
        let byte = body[i];

        if byte & 0x80 == 0 {
            has_varint = true;
            i += 1;
        } else {
            let mut j = i;
            while j < body.len() && j < i + 10 && body[j] & 0x80 != 0 {
                j += 1;
            }
            if j < body.len() {
                has_varint = true;
            }
            i = j + 1;
        }
        count += 1;
    }

    has_varint && body.len() < 10000
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3F] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Parse headers into JSON object.
#[allow(dead_code)]
pub fn parse_headers(headers: &[(String, String)]) -> Value {
    let mut obj = serde_json::Map::new();
    for (name, value) in headers {
        obj.insert(name.clone(), Value::String(value.clone()));
    }
    Value::Object(obj)
}

/// Normalize a single HTTP request record from the database.
pub fn normalize_http_record(
    id: i64,
    timestamp: &str,
    method: &str,
    path: &str,
    req_headers: &str,
    req_body: Option<&[u8]>,
    resp_status: Option<i64>,
    resp_headers: &str,
    resp_body: Option<&[u8]>,
    duration_ms: Option<i64>,
    device_id: Option<i64>,
) -> NormalizedRecord {
    let query_str = path.split('?').nth(1).unwrap_or("");
    let query = parse_query_params(query_str);

    let req_headers_parsed: Value = serde_json::from_str(req_headers).unwrap_or(Value::Object(serde_json::Map::new()));
    let resp_headers_parsed: Value = serde_json::from_str(resp_headers).unwrap_or(Value::Object(serde_json::Map::new()));

    let req_ct = req_headers_parsed.get("Content-Type")
        .or_else(|| req_headers_parsed.get("content-type"))
        .and_then(|v| v.as_str());
    let resp_ct = resp_headers_parsed.get("Content-Type")
        .or_else(|| resp_headers_parsed.get("content-type"))
        .and_then(|v| v.as_str());

    let req_body_parsed = req_body
        .map(|b| parse_body(b, req_ct))
        .unwrap_or(ParsedBody {
            format: BodyFormat::Text,
            parsed: Value::Null,
            raw: None,
        });

    let resp_body_parsed = resp_body
        .map(|b| parse_body(b, resp_ct))
        .unwrap_or(ParsedBody {
            format: BodyFormat::Text,
            parsed: Value::Null,
            raw: None,
        });

    NormalizedRecord {
        id,
        timestamp: timestamp.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        query,
        request_headers: req_headers_parsed,
        request_body: req_body_parsed.parsed,
        response_status: resp_status.unwrap_or(0) as u16,
        response_headers: resp_headers_parsed,
        response_body: resp_body_parsed.parsed,
        timing_ms: duration_ms.unwrap_or(0),
        device_id,
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Get all normalized traffic records (for AI analysis).
#[tauri::command]
pub fn get_normalized_traffic(
    db_state: State<'_, Arc<DbState>>,
    limit: Option<i64>,
) -> Result<Vec<NormalizedRecord>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(1000);

    let mut stmt = conn.prepare(
        "SELECT id, timestamp, method, path, req_headers, req_body, resp_status, resp_headers, resp_body, duration_ms, device_id
         FROM http_requests ORDER BY id DESC LIMIT ?1"
    ).map_err(|e| e.to_string())?;

    let records = stmt.query_map(params![limit], |row| {
        let id: i64 = row.get(0)?;
        let timestamp: String = row.get(1)?;
        let method: String = row.get(2)?;
        let path: String = row.get(3)?;
        let req_headers: String = row.get(4)?;
        let req_body: Option<Vec<u8>> = row.get(5)?;
        let resp_status: Option<i64> = row.get(6)?;
        let resp_headers: String = row.get(7)?;
        let resp_body: Option<Vec<u8>> = row.get(8)?;
        let duration_ms: Option<i64> = row.get(9)?;
        let device_id: Option<i64> = row.get(10)?;

        Ok(normalize_http_record(
            id,
            &timestamp,
            &method,
            &path,
            &req_headers,
            req_body.as_deref(),
            resp_status,
            &resp_headers,
            resp_body.as_deref(),
            duration_ms,
            device_id,
        ))
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;

    Ok(records)
}

/// Get paginated traffic records.
#[tauri::command]
pub fn get_traffic_page(
    db_state: State<'_, Arc<DbState>>,
    page: i64,
    page_size: i64,
) -> Result<TrafficPage, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let offset = page * page_size;

    let mut stmt = conn.prepare(
        "SELECT id, timestamp, method, path, req_headers, req_body, resp_status, resp_headers, resp_body, duration_ms, device_id
         FROM http_requests ORDER BY id DESC LIMIT ?1 OFFSET ?2"
    ).map_err(|e| e.to_string())?;

    let records = stmt.query_map(params![page_size, offset], |row| {
        let id: i64 = row.get(0)?;
        let timestamp: String = row.get(1)?;
        let method: String = row.get(2)?;
        let path: String = row.get(3)?;
        let req_headers: String = row.get(4)?;
        let req_body: Option<Vec<u8>> = row.get(5)?;
        let resp_status: Option<i64> = row.get(6)?;
        let resp_headers: String = row.get(7)?;
        let resp_body: Option<Vec<u8>> = row.get(8)?;
        let duration_ms: Option<i64> = row.get(9)?;
        let device_id: Option<i64> = row.get(10)?;

        Ok(normalize_http_record(
            id,
            &timestamp,
            &method,
            &path,
            &req_headers,
            req_body.as_deref(),
            resp_status,
            &resp_headers,
            resp_body.as_deref(),
            duration_ms,
            device_id,
        ))
    }).map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;

    let has_more = (page + 1) * page_size < total;

    Ok(TrafficPage {
        records,
        total,
        page,
        page_size,
        has_more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_params() {
        let result = parse_query_params("foo=bar&baz=qux");
        assert_eq!(result["foo"], "bar");
        assert_eq!(result["baz"], "qux");
    }

    #[test]
    fn test_parse_json_body() {
        let body = b"{\"name\": \"test\", \"value\": 123}";
        let result = parse_body(body, Some("application/json"));
        assert_eq!(result.format, BodyFormat::Json);
        assert_eq!(result.parsed["name"], "test");
    }

    #[test]
    fn test_parse_graphql_body() {
        let body = b"{\"query\": \"{ users { id } }\", \"variables\": {}}";
        let result = parse_body(body, None);
        assert_eq!(result.format, BodyFormat::GraphQL);
    }

    #[test]
    fn test_parse_headers() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer token".to_string()),
        ];
        let result = parse_headers(&headers);
        assert_eq!(result["Content-Type"], "application/json");
        assert_eq!(result["Authorization"], "Bearer token");
    }
}