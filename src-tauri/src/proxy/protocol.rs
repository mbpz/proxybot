//! HTTP/WS/gRPC/GraphQL protocol parsing helpers and small utilities.

use crate::graphql::GraphQLDecoder;
use crate::protobuf;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// HTTP request/response parsing
// ---------------------------------------------------------------------------

/// Parse HTTP request line, headers, and body from buffered data.
pub(super) fn parse_http_request(
    data: &[u8],
) -> Option<(String, String, String, Vec<(String, String)>, Vec<u8>)> {
    let first_line_end = data.windows(2).position(|w| w == b"\r\n")?;
    let first_line = String::from_utf8_lossy(&data[..first_line_end]);
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();
    let version = parts[2].to_string();

    let mut headers = Vec::new();
    let mut pos = first_line_end + 2;
    let mut body_start = data.len();
    while pos < data.len().saturating_sub(3) {
        let rest = &data[pos..];
        let line_end = rest.windows(2).position(|w| w == b"\r\n")?;
        pos += line_end + 2;
        if line_end == 0 {
            body_start = pos;
            break;
        }
        let line = String::from_utf8_lossy(&data[pos - line_end - 2..pos - 2]);
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }

    let body = if body_start < data.len() {
        data[body_start..].to_vec()
    } else {
        Vec::new()
    };

    Some((method, path, version, headers, body))
}

/// Parse HTTP response status and headers from buffered data.
pub(super) fn parse_http_response(data: &[u8]) -> Option<(u16, Vec<(String, String)>, Vec<u8>)> {
    let first_line_end = data.windows(2).position(|w| w == b"\r\n")?;
    let first_line = String::from_utf8_lossy(&data[..first_line_end]);
    let parts: Vec<&str> = first_line.split(' ').collect();
    if parts.len() < 2 {
        return None;
    }
    let status: u16 = parts[1].parse().ok()?;

    let mut headers = Vec::new();
    let mut pos = first_line_end + 2;
    let mut body_start = data.len();
    while pos < data.len().saturating_sub(3) {
        let rest = &data[pos..];
        let line_end = rest.windows(2).position(|w| w == b"\r\n")?;
        pos += line_end + 2;
        if line_end == 0 {
            body_start = pos;
            break;
        }
        let line = String::from_utf8_lossy(&data[pos - line_end - 2..pos - 2]);
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }

    let body = if body_start < data.len() {
        data[body_start..].to_vec()
    } else {
        Vec::new()
    };

    Some((status, headers, body))
}

// ---------------------------------------------------------------------------
// Query / body / header helpers
// ---------------------------------------------------------------------------

/// Extract query parameters from URL path.
pub(super) fn extract_query_params(path: &str) -> Option<String> {
    path.split_once('?').map(|(_, query)| query.to_string())
}

/// Try to parse body as UTF-8 string, fall back to hex representation.
pub(super) fn body_to_string(body: &[u8]) -> Option<String> {
    String::from_utf8(body.to_vec()).ok()
}

/// Decompress a response body according to its `Content-Encoding`.
///
/// Thin wrapper over [`proxybot_core::body::decompress`] that pulls
/// the encoding token out of the response headers. Captured bodies
/// arrive as raw wire bytes; gzip/deflate/brotli responses must be
/// inflated before `body_to_string`, or the compressed bytes fail
/// UTF-8 and the body is dropped to `None`.
pub(super) fn decompress_body(headers: &[(String, String)], body: &[u8]) -> Vec<u8> {
    let encoding = header_value(headers, "content-encoding").unwrap_or("");
    proxybot_core::body::decompress(encoding, body)
}

pub(super) fn http_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
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
        _ => "OK",
    }
}

pub(super) fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

pub(super) fn set_header(headers: &mut Vec<(String, String)>, name: &str, value: String) {
    if let Some((_, existing)) = headers
        .iter_mut()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
    {
        *existing = value;
    } else {
        headers.push((name.to_string(), value));
    }
}

pub(super) fn infer_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "json" => "application/json; charset=utf-8",
        "html" | "htm" => "text/html; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub(super) fn expand_user_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(path)
    }
}

pub(super) fn parse_host_port(s: &str) -> Option<(&str, u16)> {
    if let Some((host, port_str)) = s.split_once(':') {
        port_str.parse().ok().map(|p| (host, p))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Byte-level helpers used by the request parser
// ---------------------------------------------------------------------------

pub(super) fn trim_bytes(s: &[u8]) -> &[u8] {
    let start = s
        .iter()
        .position(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
        .unwrap_or(s.len());
    let end = s
        .iter()
        .rposition(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n')
        .map(|p| p + 1)
        .unwrap_or(0);
    &s[start..end.max(start)]
}

pub(super) fn find_colon(line: &[u8]) -> Option<(&[u8], &[u8])> {
    for (i, &b) in line.iter().enumerate() {
        if b == b':' {
            return Some((&line[..i], &line[i + 1..]));
        }
    }
    None
}

// ---------------------------------------------------------------------------
// WebSocket frame parsing
// ---------------------------------------------------------------------------

/// Check if an HTTP response is a WebSocket upgrade (101 + Upgrade: websocket).
pub(super) fn is_ws_upgrade_response(status: u16, resp_headers: &[(String, String)]) -> bool {
    if status != 101 {
        return false;
    }
    resp_headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("upgrade") && value.eq_ignore_ascii_case("websocket")
    })
}

/// Check if an HTTP request is a WebSocket upgrade request.
pub(super) fn is_ws_upgrade_request(headers: &[(String, String)]) -> bool {
    headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("upgrade") && value.eq_ignore_ascii_case("websocket")
    })
}

/// Parsed WebSocket frame header.
pub(super) struct WsFrameHeader {
    pub(super) _fin: bool,
    pub(super) opcode: u8,
    pub(super) _masked: bool,
    pub(super) payload_len: usize,
    pub(super) mask_key: Option<[u8; 4]>,
    pub(super) header_size: usize,
}

/// Parse a single WebSocket frame from raw bytes.
/// Returns (frame_header, total_bytes_needed) or None if not enough data.
pub(super) fn parse_ws_frame_header(data: &[u8]) -> Option<(WsFrameHeader, usize)> {
    if data.len() < 2 {
        return None;
    }

    let fin = (data[0] & 0x80) != 0;
    let opcode = data[0] & 0x0F;
    let masked = (data[1] & 0x80) != 0;
    let mut payload_len = (data[1] & 0x7F) as usize;
    let mut offset = 2;

    if payload_len == 126 {
        if data.len() < 4 {
            return None;
        }
        payload_len = u16::from_be_bytes([data[2], data[3]]) as usize;
        offset = 4;
    } else if payload_len == 127 {
        if data.len() < 10 {
            return None;
        }
        payload_len = u64::from_be_bytes(data[2..10].try_into().ok()?) as usize;
        offset = 10;
    }

    let mask_key = if masked {
        if data.len() < offset + 4 {
            return None;
        }
        let key = [data[offset], data[offset + 1], data[offset + 2], data[offset + 3]];
        offset += 4;
        Some(key)
    } else {
        None
    };

    let total = offset + payload_len;
    if data.len() < total {
        return None;
    }

    Some((
        WsFrameHeader {
            _fin: fin,
            opcode,
            _masked: masked,
            payload_len,
            mask_key,
            header_size: offset,
        },
        total,
    ))
}

/// Decode WS frame payload (unmask if needed) and try to convert to UTF-8 text.
pub(super) fn decode_ws_payload(data: &[u8], header: &WsFrameHeader) -> (Vec<u8>, String) {
    let payload_start = header.header_size;
    let payload_end = payload_start + header.payload_len;
    let mut raw = data[payload_start..payload_end].to_vec();

    if let Some(mask) = header.mask_key {
        for (i, byte) in raw.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
    }

    let text = String::from_utf8_lossy(&raw).to_string();
    (raw, text)
}

// ---------------------------------------------------------------------------
// gRPC / GraphQL body decoders
// ---------------------------------------------------------------------------

/// Auto-decode gRPC response body if the content-type indicates gRPC.
/// Returns JSON representation of protobuf fields, or None if not gRPC or decode fails.
pub(super) fn try_decode_grpc_body(
    resp_headers: &[(String, String)],
    resp_body: &[u8],
) -> Option<String> {
    if !protobuf::is_grpc_request(resp_headers) {
        return None;
    }
    if resp_body.is_empty() {
        return None;
    }
    // For standard gRPC, extract protobuf messages from each frame
    if protobuf::is_standard_grpc(resp_headers) {
        let frames = protobuf::decode_grpc_frames(resp_body);
        if frames.is_empty() {
            return None;
        }
        let mut results: Vec<String> = Vec::new();
        for (i, frame) in frames.iter().enumerate() {
            match protobuf::decode_protobuf(&frame.data) {
                Ok(decoded) if decoded != "[]" => {
                    results.push(format!(r#"{{"frame":{},"decoded":{}}}"#, i, decoded));
                }
                _ => {}
            }
        }
        if results.is_empty() {
            // Fall back to raw protobuf decode on whole body
            protobuf::decode_protobuf(resp_body).ok()
        } else {
            Some(format!("[{}]", results.join(",")))
        }
    } else {
        // gRPC-Web: decode body directly as protobuf
        protobuf::decode_protobuf(resp_body).ok()
    }
}

/// Auto-detect and parse GraphQL request bodies.
///
/// Returns a JSON representation of the parsed `GraphQLOperation`, or
/// `None` if the request does not look like GraphQL or parsing fails.
pub(super) fn try_decode_graphql_body(
    req_headers: &[(String, String)],
    req_body: Option<&str>,
) -> Option<String> {
    let body = req_body?;
    if body.is_empty() {
        return None;
    }
    if !GraphQLDecoder::is_graphql_content_type(req_headers) {
        return None;
    }
    if !GraphQLDecoder::is_graphql_body(body) {
        return None;
    }
    let op = GraphQLDecoder::parse_request(body).ok()?;
    serde_json::to_string(&op).ok()
}
