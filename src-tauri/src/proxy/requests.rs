//! InterceptedRequest construction, persistence emission, and device lookup.

use super::protocol::{
    body_to_string, decompress_body, extract_query_params, parse_http_response,
    try_decode_graphql_body, try_decode_grpc_body,
};
use super::{DeviceContext, InterceptedRequest, ProxyContext};
use crate::db::{record_http_request, DbState};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn timestamp_now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| format!("{}.{:03}", dur.as_secs(), dur.subsec_millis()))
        .unwrap_or_else(|_| "0.000".to_string())
}

pub(super) fn generate_request_id() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| format!("req-{}", dur.as_nanos()))
        .unwrap_or_else(|_| format!("req-{}", std::time::Instant::now().elapsed().as_nanos()))
}

pub(super) fn emit_and_record(ctx: &ProxyContext, req: InterceptedRequest) {
    let _ = ctx.event_tx.send(req.clone());

    if let Ok(conn) = ctx.db_state.conn.lock() {
        let session_id = ctx.active_session_id.lock().ok().and_then(|g| g.clone());
        let _ = record_http_request(
            &conn,
            &req.timestamp,
            &req.method,
            &req.scheme,
            &req.host,
            &req.path,
            &req.req_headers,
            req.req_body.as_deref(),
            req.status,
            &req.resp_headers,
            req.resp_body.as_deref(),
            req.latency_ms,
            req.device_id,
            req.app_name.as_deref(),
            session_id.as_deref(),
        );
    }
}

/// Build an InterceptedRequest from a captured request and buffered response.
// This constructor mirrors the complete captured request record.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_intercepted_request(
    method: String,
    scheme: String,
    host: String,
    path: String,
    headers: Vec<(String, String)>,
    body: &[u8],
    response_buf: &[u8],
    latency: u64,
    client_addr: SocketAddr,
    device_ctx: Option<DeviceContext>,
    app_info: Option<(String, String)>,
) -> InterceptedRequest {
    let (status, resp_headers, resp_body) =
        parse_http_response(response_buf).unwrap_or((0u16, Vec::new(), Vec::new()));
    // Decompress gzip/deflate/brotli before stringifying — otherwise a
    // compressed JSON body is non-UTF-8 and gets dropped to None.
    // gRPC decoding still runs on the raw bytes (it has its own framing).
    let grpc_decoded = try_decode_grpc_body(&resp_headers, &resp_body);
    let resp_body = decompress_body(&resp_headers, &resp_body);
    let graphql_op = try_decode_graphql_body(&headers, body_to_string(body).as_deref());
    let (app_name, app_icon) = app_info
        .map(|(n, i)| (Some(n), Some(i)))
        .unwrap_or((None, None));
    let (device_id, device_name) = device_ctx
        .map(|d| (Some(d.device_id), Some(d.device_name)))
        .unwrap_or((None, None));

    InterceptedRequest {
        id: generate_request_id(),
        timestamp: timestamp_now(),
        method,
        host,
        path: path.clone(),
        query_params: extract_query_params(&path),
        status: Some(status),
        latency_ms: Some(latency),
        scheme,
        req_headers: headers,
        req_body: body_to_string(body),
        resp_headers,
        resp_body: body_to_string(&resp_body),
        resp_size: Some(response_buf.len()),
        app_name,
        app_icon,
        device_id,
        device_name,
        client_ip: Some(client_addr.ip().to_string()),
        is_websocket: false,
        ws_frames: None,
        grpc_decoded,
        graphql_op,
    }
}

/// Build a request-side InterceptedRequest that has no response yet.
/// Used as input to rule matching and as the basis for hook execution.
pub(super) fn build_request_context(
    method: &str,
    scheme: &str,
    host: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
    client_addr: SocketAddr,
) -> InterceptedRequest {
    InterceptedRequest {
        id: generate_request_id(),
        timestamp: timestamp_now(),
        method: method.to_string(),
        scheme: scheme.to_string(),
        host: host.to_string(),
        path: path.to_string(),
        query_params: extract_query_params(path),
        status: None,
        latency_ms: None,
        req_headers: headers.to_vec(),
        req_body: body_to_string(body),
        resp_headers: Vec::new(),
        resp_body: None,
        resp_size: None,
        app_name: None,
        app_icon: None,
        device_id: None,
        device_name: None,
        client_ip: Some(client_addr.ip().to_string()),
        is_websocket: false,
        ws_frames: None,
        grpc_decoded: None,
        graphql_op: None,
    }
}

/// Get or create a device for the given IP address.
/// Uses IP as the identifier since MAC is not available from TCP connections.
/// Returns DeviceContext with device info.
pub(super) async fn get_or_create_device(
    db_state: &Arc<DbState>,
    ip_address: &str,
) -> Option<DeviceContext> {
    // Try to get existing device first
    let device = db_state.get_device_by_ip_internal(ip_address);
    if let Some(d) = device {
        return Some(DeviceContext {
            device_id: d.id,
            device_name: d.name,
            ip_address: ip_address.to_string(),
        });
    }

    // Create new device with IP as name/identifier
    let name = format!("Device-{}", ip_address);
    match db_state.register_device_internal(ip_address, &name) {
        Ok(d) => Some(DeviceContext {
            device_id: d.id,
            device_name: d.name,
            ip_address: ip_address.to_string(),
        }),
        Err(e) => {
            log::warn!("Failed to register device {}: {}", ip_address, e);
            None
        }
    }
}
