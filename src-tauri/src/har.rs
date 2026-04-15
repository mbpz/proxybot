//! HAR (HTTP Archive) export functionality.
//! Converts intercepted requests to HAR 1.2 format for use in Chrome DevTools / Charles Proxy / Fiddler.

use serde::Serialize;

use crate::proxy::InterceptedRequest;

// ---------------------------------------------------------------------------
// HAR data structures
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct HarLog {
    #[serde(rename = "log")]
    log: HarLogInner,
}

#[derive(Serialize)]
struct HarLogInner {
    version: &'static str,
    creator: HarCreator,
    entries: Vec<HarEntry>,
}

#[derive(Serialize)]
struct HarCreator {
    name: String,
    version: String,
}

#[derive(Serialize)]
pub struct HarEntry {
    started_date_time: String,  // ISO 8601
    time: i64,                 // total latency in ms
    request: HarRequest,
    response: HarResponse,
    #[serde(rename = "timings")]
    timings_obj: HarTimings,
}

#[derive(Serialize)]
struct HarRequest {
    method: String,
    url: String,
    http_version: String,
    headers: Vec<HarHeader>,
    query_string: Vec<HarQueryParam>,
    cookies: Vec<()>,
    headers_size: i64,
    body_size: i64,
}

#[derive(Serialize)]
struct HarResponse {
    status: u16,
    status_text: String,
    http_version: String,
    headers: Vec<HarHeader>,
    content: HarContent,
    redirect_url: String,
    headers_size: i64,
    body_size: i64,
}

#[derive(Serialize)]
struct HarHeader {
    name: String,
    value: String,
}

#[derive(Serialize)]
struct HarQueryParam {
    name: String,
    value: String,
}

#[derive(Serialize)]
struct HarContent {
    size: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(Serialize)]
struct HarTimings {
    send: i64,
    wait: i64,
    receive: i64,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a complete HAR log from a list of intercepted requests.
pub fn build_har(requests: Vec<InterceptedRequest>) -> HarLog {
    let entries: Vec<HarEntry> = requests
        .iter()
        .map(intercepted_req_to_har_entry)
        .collect();

    HarLog {
        log: HarLogInner {
            version: "1.2",
            creator: HarCreator {
                name: "ProxyBot".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            entries,
        },
    }
}

/// Convert a single InterceptedRequest into a HAR entry.
pub fn intercepted_req_to_har_entry(req: &InterceptedRequest) -> HarEntry {
    let url = format!("https://{}{}", req.host, req.path);

    let request_headers = req
        .request_headers
        .as_deref()
        .unwrap_or("");
    let response_headers = req
        .response_headers
        .as_deref()
        .unwrap_or("");

    let har_request = HarRequest {
        method: req.method.clone(),
        url,
        http_version: "HTTP/1.1".to_string(),
        headers: parse_headers(request_headers),
        query_string: Vec::new(), // InterceptedRequest doesn't have query string parsed
        cookies: Vec::new(),
        headers_size: -1,
        body_size: 0,
    };

    let status = req.status.unwrap_or(0);
    let status_text = http_status_text(status);

    let response_body = req.response_body.as_deref().unwrap_or("");
    let body_size = response_body.len() as i64;

    let mime_type = extract_content_type(response_headers);

    let har_response = HarResponse {
        status,
        status_text,
        http_version: "HTTP/1.1".to_string(),
        headers: parse_headers(response_headers),
        content: HarContent {
            size: body_size,
            mime_type,
            text: if body_size > 0 {
                Some(response_body.to_string())
            } else {
                None
            },
        },
        redirect_url: String::new(),
        headers_size: -1,
        body_size,
    };

    let latency = req.latency_ms.unwrap_or(0) as i64;
    let timings = HarTimings {
        send: -1,
        wait: latency,
        receive: -1,
    };

    HarEntry {
        started_date_time: timestamp_to_iso8601(&req.timestamp),
        time: latency,
        request: har_request,
        response: har_response,
        timings_obj: timings,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Parse headers from "Name: value\r\n..." string format into a Vec<HarHeader>.
fn parse_headers(headers_str: &str) -> Vec<HarHeader> {
    headers_str
        .split("\r\n")
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            // Split on ": " (colon + space) to separate name and value
            line.split_once(": ")
                .map(|(name, value)| HarHeader {
                    name: name.to_string(),
                    value: value.to_string(),
                })
        })
        .collect()
}

/// Convert a ProxyBot timestamp string ("seconds.milliseconds") to ISO 8601.
/// Returns e.g. "2026-04-15T10:23:45.123Z".
fn timestamp_to_iso8601(ts: &str) -> String {
    // ts format: "1704067200.123"
    let (secs_str, ms_str) = match ts.split_once('.') {
        Some((s, ms)) => (s, ms),
        None => (ts, "0"),
    };

    let secs: i64 = secs_str.parse().unwrap_or(0);
    let ms: u32 = ms_str.parse().unwrap_or(0);

    // Format as ISO 8601 UTC string
    to_iso8601(secs, ms)
}

/// Format timestamp components as ISO 8601 string in UTC.
fn to_iso8601(secs: i64, ms: u32) -> String {
    // Use C library for portable time formatting (no external crate needed)
    use std::ffi::CStr;

    // Convert to time_t
    let time_t_val = secs as libc::time_t;

    // Get UTC time struct
    let mut utc: libc::tm = unsafe { std::mem::zeroed() };
    let _ = unsafe { libc::gmtime_r(&time_t_val, &mut utc) };

    // Format: "YYYY-MM-DDTHH:MM:SS.mmmZ" (snprintf adds null terminator)
    let mut buf = [0u8; 64];
    let len = unsafe {
        libc::snprintf(
            buf.as_mut_ptr() as *mut libc::c_char,
            buf.len() as libc::size_t,
            b"%04d-%02d-%02dT%02d:%02d:%02d.%03uZ\0".as_ptr() as *const libc::c_char,
            utc.tm_year + 1900,
            utc.tm_mon + 1,
            utc.tm_mday,
            utc.tm_hour,
            utc.tm_min,
            utc.tm_sec,
            ms,
        )
    };

    if len < 0 || len as usize >= buf.len() {
        // Fallback: return timestamp as string
        return format!("{}.{:03}Z", secs, ms);
    }

    // Convert C string to Rust String (CStr, not CString)
    unsafe {
        let c_str = CStr::from_ptr(buf.as_ptr() as *const libc::c_char);
        c_str.to_string_lossy().into_owned()
    }
}

/// Return a human-readable status text for an HTTP status code.
fn http_status_text(status: u16) -> String {
    match status {
        200 => "OK".to_string(),
        201 => "Created".to_string(),
        204 => "No Content".to_string(),
        301 => "Moved Permanently".to_string(),
        302 => "Found".to_string(),
        304 => "Not Modified".to_string(),
        400 => "Bad Request".to_string(),
        401 => "Unauthorized".to_string(),
        403 => "Forbidden".to_string(),
        404 => "Not Found".to_string(),
        405 => "Method Not Allowed".to_string(),
        500 => "Internal Server Error".to_string(),
        502 => "Bad Gateway".to_string(),
        503 => "Service Unavailable".to_string(),
        _ => format!("Status {}", status),
    }
}

/// Extract Content-Type from response headers string.
fn extract_content_type(headers_str: &str) -> Option<String> {
    headers_str
        .split("\r\n")
        .filter_map(|line| {
            let line = line.trim().to_lowercase();
            if line.starts_with("content-type:") {
                Some(line.trim_start_matches("content-type:").trim().to_string())
            } else {
                None
            }
        })
        .next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headers() {
        let input = "Host: httpbin.org\r\nContent-Type: application/json\r\nAccept: */*";
        let headers = parse_headers(input);
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].name, "Host");
        assert_eq!(headers[0].value, "httpbin.org");
        assert_eq!(headers[2].name, "Accept");
        assert_eq!(headers[2].value, "*/*");
    }

    #[test]
    fn test_parse_headers_empty() {
        let headers = parse_headers("");
        assert!(headers.is_empty());
    }

    #[test]
    fn test_http_status_text() {
        assert_eq!(http_status_text(200), "OK");
        assert_eq!(http_status_text(404), "Not Found");
        assert_eq!(http_status_text(500), "Internal Server Error");
        assert_eq!(http_status_text(999), "Status 999");
    }

    #[test]
    fn test_extract_content_type() {
        assert_eq!(
            extract_content_type("Content-Type: application/json\r\nContent-Length: 123"),
            Some("application/json".to_string())
        );
        assert_eq!(extract_content_type("Content-Length: 123"), None);
    }
}
