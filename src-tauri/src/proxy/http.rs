//! Plain HTTP handler: applies request rules, forwards to upstream (or returns
//! a synthetic response), captures WebSocket upgrades, and records the result.

use super::forward::{forward_map_remote, pipe_ws_bidirectional};
use super::hooks::call_on_request_hooks;
use super::hooks::call_on_response_hooks;
use super::protocol::{
    body_to_string, extract_query_params, is_ws_upgrade_request, is_ws_upgrade_response,
    parse_http_response, try_decode_graphql_body, try_decode_grpc_body,
};
use super::requests::{
    build_intercepted_request, build_request_context, emit_and_record, generate_request_id,
    timestamp_now,
};
use super::rules::{apply_request_rule, build_http_response, RuleApplication, RuleResponse};
use super::{DeviceContext, ProxyContext};
use crate::db::record_http_request;
use crate::plugin::InterceptedResponse;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub(super) async fn handle_http(
    ctx: ProxyContext,
    device_ctx: Option<DeviceContext>,
    mut client_stream: TcpStream,
    client_addr: SocketAddr,
    method: &str,
    path: &str,
    host: &str,
    port: u16,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(), String> {
    log::info!("HTTP {} {} from {}", method, path, client_addr);

    let start = std::time::Instant::now();

    // Metrics: count the plain HTTP request
    ctx.metrics
        .http_requests_total
        .fetch_add(1, Ordering::Relaxed);
    ctx.metrics.record_method(method);
    ctx.metrics
        .bytes_received
        .fetch_add(body.len() as u64, Ordering::Relaxed);

    // Build request context and call on_request hooks
    let mut request_ctx = build_request_context(
        method,
        if port == 443 { "https" } else { "http" },
        host,
        path,
        headers,
        body,
        client_addr,
    );
    call_on_request_hooks(&ctx.plugins, &ctx.plugin_rules, &mut request_ctx);
    // Also run Rhai script hooks
    if ctx.scripts.run_all_on_request(&request_ctx) == crate::scripting::engine::ScriptResult::Block
    {
        log::info!("Rhai script blocked HTTP request to {}", host);
        return Ok(());
    }

    // Apply rules - check for MapRemote, Respond, or breakpoint modifications
    let rule_result = apply_request_rule(
        &ctx,
        client_addr,
        if port == 443 { "https" } else { "http" },
        host,
        &request_ctx.method,
        &request_ctx.path,
        &request_ctx.req_headers,
        request_ctx
            .req_body
            .as_deref()
            .unwrap_or_default()
            .as_bytes(),
    )?;

    match rule_result {
        RuleApplication::Continue {
            method: rule_method,
            path: rule_path,
            headers,
            body,
        } => {
            // Proceed with normal request forwarding using rule-returned data
            let target_addr = format!("{}:{}", host, port);
            let mut target_stream = TcpStream::connect(&target_addr).await.map_err(|e| {
                ctx.metrics.connect_errors.fetch_add(1, Ordering::Relaxed);
                format!("Failed to connect to {}: {}", target_addr, e)
            })?;

            let http_version = "HTTP/1.1";
            let mut request = format!("{} {} {}\r\n", rule_method, rule_path, http_version);
            for (name, value) in &headers {
                request.push_str(&format!("{}: {}\r\n", name, value));
            }
            request.push_str("\r\n");

            target_stream
                .write_all(request.as_bytes())
                .await
                .map_err(|e| format!("Write request failed: {}", e))?;
            if !body.is_empty() {
                target_stream
                    .write_all(&body)
                    .await
                    .map_err(|e| format!("Write body failed: {}", e))?;
            }

            let mut response_buf = Vec::new();
            let mut buf = vec![0u8; 16384];
            loop {
                match target_stream.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        response_buf.extend_from_slice(&buf[..n]);
                        if response_buf.len() > 4 && response_buf.ends_with(b"\r\n\r\n") {
                            break;
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            if !response_buf.is_empty() {
                                break;
                            }
                            continue;
                        }
                        break;
                    }
                }
            }

            let mut client_stream = client_stream;
            client_stream
                .write_all(&response_buf)
                .await
                .map_err(|e| format!("Write response failed: {}", e))?;

            let latency = start.elapsed().as_millis() as u64;
            let request_id = generate_request_id();

            // Check if this is a WebSocket upgrade — if so, switch to WS frame capture
            let (status_early, resp_headers_early, _) =
                parse_http_response(&response_buf).unwrap_or((0u16, Vec::new(), Vec::new()));
            if is_ws_upgrade_response(status_early, &resp_headers_early)
                && is_ws_upgrade_request(&headers)
            {
                log::info!("WebSocket upgrade detected for {}{}", host, rule_path);

                // Record the upgrade request to DB and get the ID
                let ws_request_id = {
                    let ts = timestamp_now();
                    // Run the standard classification chain (host → DNS correlation).
                    // The IP-fallback arm cannot run here because target_stream is
                    // not yet established at WS-upgrade time, so pass None for
                    // resolved_ip — the host-string path still benefits from the
                    // 5-minute correlation window.
                    let app_info = crate::proxy::classify::classify_captured_request(
                        host,
                        &client_addr.ip().to_string(),
                        None,
                        &ctx.dns_state,
                    );
                    if let Ok(conn) = ctx.db_state.conn.lock() {
                        match record_http_request(
                            &conn,
                            &ts,
                            &rule_method,
                            if port == 443 { "wss" } else { "ws" },
                            host,
                            &rule_path,
                            &headers,
                            None,
                            Some(101),
                            &resp_headers_early,
                            None,
                            Some(latency),
                            device_ctx.as_ref().map(|d| d.device_id),
                            app_info.as_ref().map(|(n, _)| n.as_str()),
                        ) {
                            Ok(id) => {
                                let _ = crate::db::mark_request_websocket(&conn, &id.to_string());
                                Some(id.to_string())
                            }
                            Err(e) => {
                                log::error!("Failed to record WS upgrade request: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                };
                // conn guard is dropped here — safe to await

                if let Some(req_id) = ws_request_id {
                    let _ = pipe_ws_bidirectional(
                        client_stream,
                        target_stream,
                        req_id,
                        &ctx.db_state,
                        &ctx.network,
                        &ctx.ws_frame_tx,
                    )
                    .await;
                }
                return Ok(());
            }

            let (status, resp_headers, resp_body) =
                parse_http_response(&response_buf).unwrap_or((0u16, Vec::new(), Vec::new()));
            let resp_size = response_buf.len();

            // Metrics: response status and bytes sent
            ctx.metrics.record_status(status);
            ctx.metrics
                .bytes_sent
                .fetch_add(resp_size as u64, Ordering::Relaxed);
            let query_params = extract_query_params(&rule_path);
            let req_body_str = body_to_string(&body);
            let resp_body_str = body_to_string(&resp_body);
            let grpc_decoded = try_decode_grpc_body(&resp_headers, &resp_body);
            let graphql_op = try_decode_graphql_body(&headers, req_body_str.as_deref());

            // Call on_response hooks with the response data
            let mut response_ctx = InterceptedResponse {
                status: Some(status),
                headers: resp_headers.clone(),
                body: resp_body_str.clone(),
            };
            call_on_response_hooks(
                &ctx.plugins,
                &ctx.plugin_rules,
                &mut response_ctx,
                &request_ctx,
            );
            ctx.scripts.run_all_on_response(&response_ctx, &request_ctx);

            // Classify by direct domain match first, then fall back to DNS correlation
            // (host-string, then IP).
            let resolved_ip: Option<String> = target_stream
                .peer_addr()
                .ok()
                .map(|a| a.ip().to_string());
            let app_info = crate::proxy::classify::classify_captured_request(
                host,
                &client_addr.ip().to_string(),
                resolved_ip.as_deref(),
                &ctx.dns_state,
            );
            let (app_name, app_icon) = app_info
                .map(|(n, i)| (Some(n), Some(i)))
                .unwrap_or((None, None));

            let client_ip = client_addr.ip().to_string();
            let (device_id, device_name) = device_ctx
                .map(|d| (Some(d.device_id), Some(d.device_name)))
                .unwrap_or((None, None));

            let req = super::InterceptedRequest {
                id: request_id,
                timestamp: timestamp_now(),
                method: rule_method,
                host: host.to_string(),
                path: rule_path,
                query_params,
                status: Some(status),
                latency_ms: Some(latency),
                scheme: if port == 443 { "https" } else { "http" }.to_string(),
                req_headers: headers,
                req_body: req_body_str,
                resp_headers,
                resp_body: resp_body_str,
                resp_size: Some(resp_size),
                app_name,
                app_icon,
                device_id,
                device_name,
                client_ip: Some(client_ip),
                is_websocket: false,
                ws_frames: None,
                grpc_decoded,
                graphql_op,
            };

            let _ = ctx.event_tx.send(req.clone());

            // Record to database for TUI/persistence
            if let Ok(conn) = ctx.db_state.conn.lock() {
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
                );
            }

            Ok(())
        }
        RuleApplication::Respond {
            status,
            headers: resp_headers,
            body: resp_body,
        } => {
            // Direct response without connecting to upstream
            let response = build_http_response(&RuleResponse {
                status,
                headers: resp_headers,
                body: resp_body,
            });
            client_stream
                .write_all(&response)
                .await
                .map_err(|e| format!("Write rule response failed: {}", e))?;
            Ok(())
        }
        RuleApplication::MapRemote {
            target,
            method,
            path,
            headers,
            body,
        } => {
            // Forward to remote target
            let response_buf =
                forward_map_remote(&ctx.cert_manager, &target, &method, &path, &headers, &body)
                    .await?;

            client_stream
                .write_all(&response_buf)
                .await
                .map_err(|e| format!("Write MapRemote response failed: {}", e))?;

            // Record the request
            let latency = start.elapsed().as_millis() as u64;
            let (_status, _resp_headers, _resp_body) =
                parse_http_response(&response_buf).unwrap_or((0u16, Vec::new(), Vec::new()));

            // Classify by direct domain match first, then fall back to DNS correlation
            // (host-string, then IP). MapRemote does not use a direct upstream
            // TCP connection, so no resolved IP is available.
            let resolved_ip: Option<String> = None;
            let app_info = crate::proxy::classify::classify_captured_request(
                host,
                &client_addr.ip().to_string(),
                resolved_ip.as_deref(),
                &ctx.dns_state,
            );

            let req = build_intercepted_request(
                method,
                if port == 443 { "https" } else { "http" }.to_string(),
                host.to_string(),
                path,
                headers,
                &body,
                &response_buf,
                latency,
                client_addr,
                device_ctx,
                app_info,
            );
            emit_and_record(&ctx, req);

            Ok(())
        }
    }
}
