//! Traffic normalizer module.
//!
//! Converts HTTP traffic into structured records for AI analysis.
//! Body parser detects JSON, Protobuf (base64), and GraphQL variants.

use crate::db::DbState;
use rusqlite::{params, Connection, Row};
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
            obj.insert(url_decode(key), url_decode_value(value));
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
        } else if ct_lower.contains("application/x-protobuf")
            || ct_lower.contains("application/protobuf")
        {
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
        return obj.contains_key("query")
            || obj.contains_key("variables")
            || obj.contains_key("operationName");
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

    let req_headers_parsed: Value =
        serde_json::from_str(req_headers).unwrap_or(Value::Object(serde_json::Map::new()));
    let resp_headers_parsed: Value =
        serde_json::from_str(resp_headers).unwrap_or(Value::Object(serde_json::Map::new()));

    let req_ct = req_headers_parsed
        .get("Content-Type")
        .or_else(|| req_headers_parsed.get("content-type"))
        .and_then(|v| v.as_str());
    let resp_ct = resp_headers_parsed
        .get("Content-Type")
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
    get_normalized_traffic_internal(&conn, limit)
}

/// Get all normalized traffic records (takes `&Connection` for testability).
fn get_normalized_traffic_internal(
    conn: &Connection,
    limit: Option<i64>,
) -> Result<Vec<NormalizedRecord>, String> {
    let limit = limit.unwrap_or(1000);

    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, method, path, req_headers, req_body, resp_status, resp_headers, resp_body, duration_ms, device_id
             FROM http_requests ORDER BY id DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let records = stmt
        .query_map(params![limit], row_to_normalized)
        .map_err(|e| e.to_string())?
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
    get_traffic_page_internal(&conn, page, page_size)
}

/// Get paginated traffic records (takes `&Connection` for testability).
fn get_traffic_page_internal(
    conn: &Connection,
    page: i64,
    page_size: i64,
) -> Result<TrafficPage, String> {
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let offset = page * page_size;

    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, method, path, req_headers, req_body, resp_status, resp_headers, resp_body, duration_ms, device_id
             FROM http_requests ORDER BY id DESC LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| e.to_string())?;

    let records = stmt
        .query_map(params![page_size, offset], row_to_normalized)
        .map_err(|e| e.to_string())?
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

/// Decode a `http_requests` row into a [`NormalizedRecord`]. Shared by
/// `get_normalized_traffic_internal` and `get_traffic_page_internal`.
fn row_to_normalized(row: &Row<'_>) -> rusqlite::Result<NormalizedRecord> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbState;
    use rusqlite::Connection;

    // ------------------------------------------------------------------
    // parse_query_params — additional edge cases
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_query_params_empty_returns_empty_object() {
        let result = parse_query_params("");
        assert!(
            result.as_object().unwrap().is_empty(),
            "Empty query string should produce an empty object, got {}",
            result
        );
    }

    #[test]
    fn test_parse_query_params_key_without_value_is_null() {
        let result = parse_query_params("flag");
        assert_eq!(result["flag"], Value::Null, "Bare key (no '=') should map to null");
    }

    #[test]
    fn test_parse_query_params_mixed_keys_and_pairs() {
        let result = parse_query_params("a=1&flag&b=2");
        assert_eq!(result["a"], "1");
        assert_eq!(result["flag"], Value::Null);
        assert_eq!(result["b"], "2");
    }

    #[test]
    fn test_parse_query_params_decodes_url_encoded_values() {
        // url_decode replaces %20 with space, %3D with '=', %26 with '&'
        let result = parse_query_params("q=hello%20world&eq=a%3Db&combined=x%26y");
        assert_eq!(result["q"], "hello world", "%20 should decode to space");
        assert_eq!(result["eq"], "a=b", "%3D should decode to '='");
        assert_eq!(result["combined"], "x&y", "%26 should decode to '&'");
    }

    #[test]
    fn test_parse_query_params_empty_value_is_empty_string() {
        let result = parse_query_params("empty=&filled=v");
        assert_eq!(result["empty"], "", "Empty value (a=) should be empty string");
        assert_eq!(result["filled"], "v");
    }

    // ------------------------------------------------------------------
    // parse_body — format detection matrix
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_body_protobuf_by_content_type() {
        // Body bytes that look like garbage — the content-type hint should
        // win and force the Protobuf path (which base64-encodes the raw bytes).
        let body: &[u8] = &[0x00, 0x01, 0x02, 0x03];
        let result = parse_body(body, Some("application/x-protobuf"));
        assert_eq!(result.format, BodyFormat::Protobuf);
        assert!(
            result.parsed.is_string(),
            "Protobuf body should be base64 string in `parsed`, got {:?}",
            result.parsed
        );
        // base64 of [0x00, 0x01, 0x02, 0x03] is "AAECAw=="
        assert_eq!(result.parsed.as_str().unwrap(), "AAECAw==");
    }

    #[test]
    fn test_parse_body_protobuf_alt_content_type() {
        let body: &[u8] = &[0x00, 0x01, 0x02, 0x03];
        let result = parse_body(body, Some("application/protobuf"));
        assert_eq!(result.format, BodyFormat::Protobuf);
    }

    #[test]
    fn test_parse_body_form_urlencoded() {
        // The content-type branch forces FormData ONLY when the body is
        // ALSO valid JSON. A real urlencoded body like "key=value" fails
        // JSON parsing — see the characterization test below for that
        // case (tracked as task #84).
        let body = b"{\"a\": 1}";
        let result = parse_body(body, Some("application/x-www-form-urlencoded"));
        assert_eq!(result.format, BodyFormat::FormData);
    }

    #[test]
    fn test_parse_body_form_urlencoded_real_form_body_is_misclassified() {
        // Characterization of a real bug: the form-urlencoded branch only
        // returns FormData if the body is ALSO valid JSON. A real
        // urlencoded body like "key=value&other=2" fails JSON parsing
        // and falls through to the (also buggy) Protobuf heuristic,
        // landing in Protobuf.
        //
        // This test pins the current (buggy) behavior. Tracked as
        // task #84 — the form-urlencoded branch needs its own parser
        // (or to fall back to a string-valued parsed field).
        let body = b"key=value&other=2";
        let result = parse_body(body, Some("application/x-www-form-urlencoded"));
        assert_eq!(
            result.format,
            BodyFormat::Protobuf,
            "Characterization: form-urlencoded body wrongly classified as Protobuf. \
             Will need to be flipped to FormData when task #84 is fixed."
        );
    }

    #[test]
    fn test_parse_body_text_by_content_type() {
        let body = b"plain text response body";
        let result = parse_body(body, Some("text/plain; charset=utf-8"));
        assert_eq!(result.format, BodyFormat::Text);
        assert_eq!(result.parsed.as_str().unwrap(), "plain text response body");
    }

    #[test]
    fn test_parse_body_binary_non_utf8_falls_back_to_base64() {
        // Invalid UTF-8 sequence (0xFF 0xFE) — must hit the Binary branch.
        let body: &[u8] = &[0xFF, 0xFE, 0x00, 0x01];
        let result = parse_body(body, Some("application/octet-stream"));
        assert_eq!(result.format, BodyFormat::Binary);
        // `raw` is base64 of the original bytes
        assert!(result.raw.is_some());
        assert!(result.parsed.is_null(), "Binary should leave `parsed` as null");
    }

    #[test]
    fn test_parse_body_json_without_content_type() {
        // No content-type hint — successful JSON parse should still classify
        // as Json.
        let body = b"{\"x\": 1}";
        let result = parse_body(body, None);
        assert_eq!(result.format, BodyFormat::Json);
        assert_eq!(result.parsed["x"], 1);
    }

    #[test]
    fn test_parse_body_text_fallback_when_nothing_matches() {
        // Plain text that doesn't look like JSON, has no content-type, and
        // doesn't trigger the Protobuf heuristic — should land in Text.
        //
        // NOTE: The Protobuf heuristic in `is_probably_protobuf` is
        // wildly over-eager: it treats ANY run of ASCII bytes (each with
        // bit 7 clear) as a valid Protobuf varint, then also requires
        // `body.len() < 10000`. That means short ASCII text that fails
        // JSON parsing is currently misclassified as Protobuf (see
        // `test_parse_body_short_ascii_text_is_misclassified_as_protobuf`).
        // The single-byte body below intentionally bypasses the
        // heuristic (it requires `body.len() >= 2`) so this test
        // exercises the actual Text fallback path. Tracked as task #84.
        let body: &[u8] = b"h";
        let result = parse_body(body, None);
        assert_eq!(result.format, BodyFormat::Text);
        assert_eq!(result.parsed.as_str().unwrap(), "h");
    }

    #[test]
    fn test_parse_body_short_ascii_text_is_misclassified_as_protobuf() {
        // Characterization of a real bug: `is_probably_protobuf` returns
        // true for short ASCII text that fails JSON parsing, because the
        // heuristic only checks for `byte & 0x80 == 0` (i.e., "could be
        // a varint") and `body.len() < 10000`. A pure ASCII 11-byte
        // string with no `=` is not JSON, not a content-type-hinted
        // protobuf, and not a form-urlencoded payload, but it WILL be
        // classified as Protobuf today.
        //
        // This test pins the current (buggy) behavior so the future fix
        // (task #84) can be detected.
        let body = b"hello world";
        let result = parse_body(body, None);
        assert_eq!(
            result.format,
            BodyFormat::Protobuf,
            "Characterization: short ASCII text wrongly falls into Protobuf. \
             This test will need to be flipped when task #84 is fixed."
        );
    }

    #[test]
    fn test_parse_body_graphql_detected_by_operation_name() {
        // GraphQL detection should fire on any of {query, variables, operationName}
        let body = b"{\"operationName\": \"GetUser\", \"query\": \"...\", \"variables\": {}}";
        let result = parse_body(body, None);
        assert_eq!(result.format, BodyFormat::GraphQL);
    }

    // ------------------------------------------------------------------
    // parse_headers
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_headers_empty_list_returns_empty_object() {
        let result = parse_headers(&[]);
        assert!(result.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_parse_headers_preserves_duplicate_keys_as_last_wins() {
        // serde_json::Map::insert overrides on duplicate key. The function
        // is a thin wrapper, so verify last-wins semantics.
        let headers = vec![
            ("X-Trace".to_string(), "first".to_string()),
            ("X-Trace".to_string(), "second".to_string()),
        ];
        let result = parse_headers(&headers);
        assert_eq!(result["X-Trace"], "second");
    }

    // ------------------------------------------------------------------
    // normalize_http_record — pure function, no DB
    // ------------------------------------------------------------------

    #[test]
    fn test_normalize_http_record_extracts_query_from_path() {
        let rec = normalize_http_record(
            42,
            "2026-06-04 00:00:00",
            "GET",
            "/api/v1/users?id=7&name=alice",
            "{}",
            None,
            Some(200),
            "{}",
            None,
            Some(123),
            None,
        );
        assert_eq!(rec.id, 42);
        assert_eq!(rec.method, "GET");
        assert_eq!(rec.path, "/api/v1/users?id=7&name=alice");
        assert_eq!(rec.query["id"], "7", "Query should be parsed from path");
        assert_eq!(rec.query["name"], "alice");
        assert_eq!(rec.response_status, 200);
        assert_eq!(rec.timing_ms, 123);
        assert_eq!(rec.device_id, None);
    }

    #[test]
    fn test_normalize_http_record_no_query_string_yields_empty_query() {
        let rec = normalize_http_record(
            1, "ts", "GET", "/plain", "{}", None, Some(200), "{}", None, None, None,
        );
        assert!(rec.query.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_normalize_http_record_default_status_and_timing_when_none() {
        // resp_status and duration_ms are None — must default to 0
        let rec = normalize_http_record(
            1, "ts", "GET", "/", "{}", None, None, "{}", None, None, None,
        );
        assert_eq!(rec.response_status, 0, "None resp_status should default to 0");
        assert_eq!(rec.timing_ms, 0, "None duration_ms should default to 0");
    }

    #[test]
    fn test_normalize_http_record_missing_body_yields_null_text() {
        let rec = normalize_http_record(
            1, "ts", "POST", "/", "{}", None, Some(200), "{}", None, None, None,
        );
        // No body — `request_body` should be the null sentinel
        assert!(rec.request_body.is_null());
        assert!(rec.response_body.is_null());
    }

    #[test]
    fn test_normalize_http_record_decodes_request_body() {
        let req_headers = r#"{"Content-Type": "application/json"}"#;
        let req_body = Some(br#"{"hello": "world"}"# as &[u8]);
        let rec = normalize_http_record(
            1, "ts", "POST", "/", req_headers, req_body, Some(200), "{}", None, None, None,
        );
        assert_eq!(rec.request_body["hello"], "world");
    }

    #[test]
    fn test_normalize_http_record_decodes_response_body() {
        let resp_headers = r#"{"content-type": "application/json"}"#;
        let resp_body = Some(br#"{"ok": true}"# as &[u8]);
        let rec = normalize_http_record(
            1, "ts", "GET", "/", "{}", None, Some(200), resp_headers, resp_body, None, None,
        );
        // lowercase content-type header is still detected
        assert_eq!(rec.response_body["ok"], true);
    }

    #[test]
    fn test_normalize_http_record_preserves_device_id() {
        let rec = normalize_http_record(
            1, "ts", "GET", "/", "{}", None, Some(200), "{}", None, Some(42), Some(7),
        );
        assert_eq!(rec.device_id, Some(7));
    }

    #[test]
    fn test_normalize_http_record_invalid_header_json_yields_empty_object() {
        // Malformed req_headers/resp_headers JSON must not panic — should
        // fall back to an empty object via `unwrap_or`.
        let rec = normalize_http_record(
            1, "ts", "GET", "/", "not json", None, Some(200), "also not json", None, None, None,
        );
        assert!(rec.request_headers.as_object().unwrap().is_empty());
        assert!(rec.response_headers.as_object().unwrap().is_empty());
    }

    // ------------------------------------------------------------------
    // DB helpers — get_normalized_traffic_internal
    // ------------------------------------------------------------------

    /// Insert a synthetic http_requests row directly via SQL. Required
    /// columns (scheme, host) get safe defaults; optional columns are
    /// parameterized so each test can pick what it cares about.
    fn insert_http_row(
        conn: &Connection,
        timestamp: &str,
        method: &str,
        path: &str,
        req_body: Option<&[u8]>,
        resp_status: Option<i64>,
        resp_body: Option<&[u8]>,
        duration_ms: Option<i64>,
    ) -> i64 {
        conn.execute(
            "INSERT INTO http_requests
               (timestamp, method, scheme, host, path, req_headers, req_body,
                resp_status, resp_headers, resp_body, duration_ms)
             VALUES (?1, ?2, 'https', 'example.com', ?3, '{}', ?4, ?5, '{}', ?6, ?7)",
            rusqlite::params![timestamp, method, path, req_body, resp_status, resp_body, duration_ms],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn test_get_normalized_traffic_internal_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let records = get_normalized_traffic_internal(&conn, None).unwrap();
        assert!(records.is_empty(), "Empty DB should yield no records");
    }

    #[test]
    fn test_get_normalized_traffic_internal_returns_inserted_records() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        insert_http_row(
            &conn,
            "2026-06-04 00:00:00",
            "GET",
            "/a",
            None,
            Some(200),
            None,
            Some(50),
        );
        insert_http_row(
            &conn,
            "2026-06-04 00:00:01",
            "POST",
            "/b",
            Some(b"{\"x\":1}" as &[u8]),
            Some(201),
            Some(b"{\"ok\":true}" as &[u8]),
            Some(75),
        );

        let records = get_normalized_traffic_internal(&conn, None).unwrap();
        assert_eq!(records.len(), 2);

        // ORDER BY id DESC — second insert is newest
        assert_eq!(records[0].method, "POST");
        assert_eq!(records[0].path, "/b");
        assert_eq!(records[0].response_status, 201);
        assert_eq!(records[0].timing_ms, 75);
        assert_eq!(records[1].method, "GET");
    }

    #[test]
    fn test_get_normalized_traffic_internal_respects_explicit_limit() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        for i in 0..5 {
            insert_http_row(
                &conn,
                "2026-06-04 00:00:00",
                "GET",
                &format!("/p{}", i),
                None,
                Some(200),
                None,
                None,
            );
        }

        let records = get_normalized_traffic_internal(&conn, Some(2)).unwrap();
        assert_eq!(records.len(), 2, "Explicit limit=2 should cap result count");
    }

    #[test]
    fn test_get_normalized_traffic_internal_decodes_persisted_body() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        // Manually insert with a JSON content-type header so the normalize
        // path is exercised end-to-end through the DB.
        conn.execute(
            "INSERT INTO http_requests
               (timestamp, method, scheme, host, path, req_headers, req_body,
                resp_status, resp_headers, resp_body, duration_ms)
             VALUES ('2026-06-04 00:00:00', 'POST', 'https', 'example.com', '/x',
                     '{\"Content-Type\": \"application/json\"}', ?1, 200,
                     '{\"Content-Type\": \"application/json\"}', ?2, 30)",
            rusqlite::params![
                Some(b"{\"a\": 1}" as &[u8]),
                Some(b"{\"b\": 2}" as &[u8]),
            ],
        )
        .unwrap();

        let records = get_normalized_traffic_internal(&conn, None).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].request_body["a"], 1, "Persisted JSON body should round-trip parsed");
        assert_eq!(records[0].response_body["b"], 2);
    }

    // ------------------------------------------------------------------
    // DB helpers — get_traffic_page_internal
    // ------------------------------------------------------------------

    #[test]
    fn test_get_traffic_page_internal_empty_db_has_no_more() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let page = get_traffic_page_internal(&conn, 0, 50).unwrap();
        assert_eq!(page.total, 0, "Empty DB should have total=0");
        assert_eq!(page.records.len(), 0);
        assert!(!page.has_more, "Empty DB must not advertise more pages");
        assert_eq!(page.page, 0);
        assert_eq!(page.page_size, 50);
    }

    #[test]
    fn test_get_traffic_page_internal_paginates_in_id_desc_order() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        // Insert 7 rows so we can paginate at page_size=3 across 3 pages.
        let mut ids = Vec::new();
        for i in 0..7 {
            ids.push(insert_http_row(
                &conn,
                "2026-06-04 00:00:00",
                "GET",
                &format!("/p{}", i),
                None,
                Some(200),
                None,
                None,
            ));
        }

        // Page 0: newest 3 (ids 7, 6, 5) — has_more=true
        let page0 = get_traffic_page_internal(&conn, 0, 3).unwrap();
        assert_eq!(page0.total, 7);
        assert_eq!(page0.records.len(), 3);
        assert_eq!(page0.records[0].id, ids[6]);
        assert_eq!(page0.records[1].id, ids[5]);
        assert_eq!(page0.records[2].id, ids[4]);
        assert!(page0.has_more, "Page 0 of 3-of-7 should have more");

        // Page 1: next 3 (ids 4, 3, 2) — has_more=true
        let page1 = get_traffic_page_internal(&conn, 1, 3).unwrap();
        assert_eq!(page1.records.len(), 3);
        assert_eq!(page1.records[0].id, ids[3]);
        assert!(page1.has_more);

        // Page 2: last 1 (id 1) — has_more=false
        let page2 = get_traffic_page_internal(&conn, 2, 3).unwrap();
        assert_eq!(page2.records.len(), 1);
        assert_eq!(page2.records[0].id, ids[0]);
        assert!(!page2.has_more, "Last page must not advertise more");

        // Page 3: empty
        let page3 = get_traffic_page_internal(&conn, 3, 3).unwrap();
        assert_eq!(page3.records.len(), 0);
        assert!(!page3.has_more);
    }

    #[test]
    fn test_get_traffic_page_internal_has_more_boundary_at_exact_fit() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        // Insert exactly 4 rows; page_size=2 → exactly 2 pages, no third.
        for i in 0..4 {
            insert_http_row(
                &conn,
                "2026-06-04 00:00:00",
                "GET",
                &format!("/p{}", i),
                None,
                Some(200),
                None,
                None,
            );
        }

        let page0 = get_traffic_page_internal(&conn, 0, 2).unwrap();
        assert_eq!(page0.records.len(), 2);
        assert!(page0.has_more, "page 0 of exact-fit 2-of-4 should have more");

        let page1 = get_traffic_page_internal(&conn, 1, 2).unwrap();
        assert_eq!(page1.records.len(), 2);
        // (1+1)*2 = 4 = total — not strictly less, so has_more is false.
        assert!(!page1.has_more, "Boundary page (consumes all rows) must NOT have_more");
    }
}
