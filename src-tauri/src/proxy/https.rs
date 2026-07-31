//! HTTPS CONNECT handler — performs TLS termination on both the client
//! (browser) and upstream (origin server) sides, then pipes the decrypted
//! traffic bidirectionally while recording the captured request/response.

use super::forward::pipe_tcp_bidirectional;
use super::hooks::{call_on_connect_hooks, call_on_request_hooks, call_on_response_hooks};
use super::protocol::{
    body_to_string, extract_query_params, parse_http_request, parse_http_response,
    try_decode_graphql_body, try_decode_grpc_body,
};
use super::requests::{build_request_context, generate_request_id, timestamp_now};
use super::{DeviceContext, InterceptedRequest, ProxyContext};
use crate::db::record_http_request;
use rustls::pki_types::ServerName;
use rustls::ServerConfig;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsConnector};

/// Handle HTTPS CONNECT tunnel with TLS termination on both sides.
pub(super) async fn handle_https_connect(
    ctx: ProxyContext,
    device_ctx: Option<DeviceContext>,
    mut client_stream: TcpStream,
    client_addr: SocketAddr,
    target_host: String,
    target_port: u16,
) {
    let target_addr = format!("{}:{}", target_host, target_port);
    log::info!(
        "HTTPS CONNECT tunnel to {} from {}",
        target_addr,
        client_addr
    );

    let start = std::time::Instant::now();

    // Metrics: count the HTTPS request
    ctx.metrics
        .https_requests_total
        .fetch_add(1, Ordering::Relaxed);

    // Call on_connect hooks - block or redirect if needed
    if let Some(crate::plugin::ConnectDecision::Block) =
        call_on_connect_hooks(&ctx.plugins, &target_host)
    {
        log::info!("Connection to {} blocked by plugin", target_host);
        let _ = client_stream
            .write_all(b"HTTP/1.1 403 Forbidden\r\n\r\n")
            .await;
        return;
    }

    // Send HTTP 200 Connection Established to browser
    if let Err(e) = client_stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await
    {
        log::error!(
            "Failed to send 200 response to browser {}: {}",
            client_addr,
            e
        );
        ctx.metrics.connect_errors.fetch_add(1, Ordering::Relaxed);
        return;
    }

    // Per-host TLS policy: a Bypass/Passthrough host is tunnelled raw
    // instead of MITM'd. This is the escape hatch for cert-pinned apps
    // (which reject our leaf cert and crash) and noisy telemetry hosts.
    // Default (no matching rule) is Decrypt, preserving prior behaviour.
    let tls_action = ctx
        .tls_rules
        .read()
        .map(|rs| rs.decide(&target_host))
        .unwrap_or(proxybot_core::TlsAction::Decrypt);

    if !tls_action.is_decrypt() {
        log::info!(
            "TLS rule: {:?} for {} — tunnelling without decryption",
            tls_action,
            target_host
        );
        // Record CONNECT metadata for Bypass (so the host still shows
        // in the capture as a passthrough); Passthrough records nothing.
        if tls_action.should_log() {
            if let Ok(conn) = ctx.db_state.conn.lock() {
                let session_id = ctx.active_session_id.lock().ok().and_then(|g| g.clone());
                let _ = record_http_request(
                    &conn,
                    &timestamp_now(),
                    "CONNECT",
                    "https",
                    &target_host,
                    "/",
                    &[],
                    None,
                    Some(200),
                    &[],
                    None,
                    None,
                    device_ctx.as_ref().map(|d| d.device_id),
                    None,
                    session_id.as_deref(),
                );
            }
        }
        let upstream = match TcpStream::connect(&target_addr).await {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to connect to upstream {}: {}", target_addr, e);
                return;
            }
        };
        if let Err(e) = pipe_tcp_bidirectional(client_stream, upstream, &ctx.network).await {
            log::error!("Bypass tunnel pipe failed: {}", e);
        }
        return;
    }

    // Generate certificate for the target host signed by our CA
    let (cert_pem, key_pem) = match ctx.cert_manager.generate_host_cert(&target_host) {
        Ok(cert) => cert,
        Err(e) => {
            log::error!("Failed to generate certificate for {}: {}", target_host, e);
            // Fall back to raw TCP tunnel
            let upstream = match TcpStream::connect(&target_addr).await {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to connect to upstream {}: {}", target_addr, e);
                    return;
                }
            };
            let client = client_stream;
            if let Err(e) = pipe_tcp_bidirectional(client, upstream, &ctx.network).await {
                log::error!("Tunnel pipe failed: {}", e);
            }
            return;
        }
    };

    // Build server TLS config for accepting browser connection
    let certs = match rustls_pemfile::certs(&mut std::io::Cursor::new(cert_pem.as_bytes()))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(certs) => certs,
        Err(e) => {
            log::error!("Failed to parse certificate PEM: {}", e);
            return;
        }
    };

    let keys = match rustls_pemfile::private_key(&mut std::io::Cursor::new(key_pem.as_bytes())) {
        Ok(Some(key)) => key,
        Ok(None) => {
            log::error!("No private key found in PEM");
            return;
        }
        Err(e) => {
            log::error!("Failed to parse private key PEM: {}", e);
            return;
        }
    };

    let server_config = match ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, keys)
    {
        Ok(config) => config,
        Err(e) => {
            log::error!("Failed to build server config: {}", e);
            return;
        }
    };

    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

    // Accept TLS from browser
    let client_tls_stream = match tls_acceptor.accept(client_stream).await {
        Ok(stream) => stream,
        Err(e) => {
            log::error!("TLS accept failed for browser {}: {}", client_addr, e);
            ctx.metrics.tls_errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
    };

    log::debug!("TLS handshake completed with browser {}", client_addr);

    // Build client TLS config for connecting to upstream
    let client_config = match super::tls::build_client_config(&ctx.cert_manager) {
        Ok(config) => config,
        Err(e) => {
            log::error!("Failed to build client TLS config: {}", e);
            return;
        }
    };

    // Connect to upstream server with TLS
    let upstream_addr = format!("{}:{}", target_host, target_port);
    let upstream_tcp = match TcpStream::connect(&upstream_addr).await {
        Ok(stream) => stream,
        Err(e) => {
            log::error!("Failed to connect to upstream {}: {}", upstream_addr, e);
            ctx.metrics.connect_errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
    };

    // Capture the upstream's resolved IP before the TCP stream is moved
    // into the TLS connector, so the DNS-correlation helper can use it
    // for IP-literal classification.
    let resolved_ip: Option<String> = upstream_tcp.peer_addr().ok().map(|a| a.ip().to_string());

    // Use SNI to tell the upstream server which host we're connecting to
    // Box::leak to get 'static lifetime for rustls ServerName requirement
    let target_host_static: &'static str = Box::leak(target_host.clone().into_boxed_str());
    let server_name = match ServerName::try_from(target_host_static) {
        Ok(name) => name,
        Err(e) => {
            log::error!("Invalid server name {}: {}", target_host_static, e);
            return;
        }
    };

    let connector = TlsConnector::from(Arc::new(client_config));
    let upstream_tls_stream = match connector.connect(server_name, upstream_tcp).await {
        Ok(stream) => stream,
        Err(e) => {
            log::error!("TLS connect to upstream {} failed: {}", upstream_addr, e);
            ctx.metrics.tls_errors.fetch_add(1, Ordering::Relaxed);
            return;
        }
    };

    log::debug!("TLS handshake completed with upstream {}", upstream_addr);

    // Pipe data bidirectionally between the two TLS streams
    let (mut client_read, mut client_write) = tokio::io::split(client_tls_stream);
    let (mut upstream_read, mut upstream_write) = tokio::io::split(upstream_tls_stream);

    let mut client_buf = vec![0u8; 16384];
    let mut upstream_buf = vec![0u8; 16384];
    let mut request_data = Vec::new();
    let mut response_data = Vec::new();

    loop {
        tokio::select! {
            n = client_read.read(&mut client_buf) => {
                let n = match n {
                    Ok(n) => n,
                    Err(e) => {
                        log::error!("Read from client TLS failed: {}", e);
                        break;
                    }
                };
                if n == 0 {
                    break;
                }
                request_data.extend_from_slice(&client_buf[..n]);
                // Apply network conditions before writing upstream
                let effect = ctx.network.apply(n);
                if effect.drop {
                    continue; // drop this chunk
                }
                if effect.delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(effect.delay_ms)).await;
                }
                if let Err(e) = upstream_write.write_all(&client_buf[..n]).await {
                    log::error!("Write to upstream failed: {}", e);
                    break;
                }
            }
            n = upstream_read.read(&mut upstream_buf) => {
                let n = match n {
                    Ok(n) => n,
                    Err(e) => {
                        log::error!("Read from upstream TLS failed: {}", e);
                        break;
                    }
                };
                if n == 0 {
                    break;
                }
                response_data.extend_from_slice(&upstream_buf[..n]);
                // Apply network conditions before writing to client
                let effect = ctx.network.apply(n);
                if effect.drop {
                    continue; // drop this chunk
                }
                if effect.delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(effect.delay_ms)).await;
                }
                if let Err(e) = client_write.write_all(&upstream_buf[..n]).await {
                    log::error!("Write to client failed: {}", e);
                    break;
                }
            }
        }
    }

    // Log the intercepted request
    let latency = start.elapsed().as_millis() as u64;

    // Metrics: bytes transferred
    ctx.metrics
        .bytes_received
        .fetch_add(request_data.len() as u64, Ordering::Relaxed);
    ctx.metrics
        .bytes_sent
        .fetch_add(response_data.len() as u64, Ordering::Relaxed);

    // Parse request and response for logging
    let (method, path, _, req_headers, req_body) = parse_http_request(&request_data)
        .unwrap_or_else(|| {
            (
                "CONNECT".to_string(),
                "/".to_string(),
                "1.1".to_string(),
                Vec::new(),
                Vec::new(),
            )
        });

    let (status, resp_headers, resp_body) =
        parse_http_response(&response_data).unwrap_or((0u16, Vec::new(), Vec::new()));

    // Metrics: method and status
    ctx.metrics.record_method(&method);
    ctx.metrics.record_status(status);

    let request_id = generate_request_id();
    let query_params = extract_query_params(&path);
    let resp_size = response_data.len();
    let req_body_str = body_to_string(&req_body);
    let resp_body_str = body_to_string(&resp_body);
    let grpc_decoded = try_decode_grpc_body(&resp_headers, &resp_body);
    let graphql_op = try_decode_graphql_body(&req_headers, req_body_str.as_deref());

    // Build request context and run plugin hooks for HTTPS interception
    let mut request_ctx = build_request_context(
        &method,
        "https",
        &target_host,
        &path,
        &req_headers,
        &req_body,
        client_addr,
    );
    call_on_request_hooks(&ctx.plugins, &ctx.plugin_rules, &mut request_ctx);
    // Also run Rhai script hooks — can block or rewrite the body.
    match ctx.scripts.run_all_on_request(&request_ctx) {
        crate::scripting::engine::ScriptResult::Block => {
            log::info!("Rhai script blocked request to {}", target_host);
            return;
        }
        crate::scripting::engine::ScriptResult::RewriteBody(new_body) => {
            log::info!(
                "Rhai script rewrote request body for {} {}",
                request_ctx.method,
                request_ctx.host
            );
            request_ctx.req_body = Some(new_body);
        }
        crate::scripting::engine::ScriptResult::Continue => {}
    }

    let mut response_ctx = crate::plugin::InterceptedResponse {
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
    if let crate::scripting::engine::ScriptResult::RewriteBody(new_body) =
        ctx.scripts.run_all_on_response(&response_ctx, &request_ctx)
    {
        response_ctx.body = Some(new_body);
    }

    // Classify by direct domain match first, then fall back to DNS correlation
    // (host-string, then IP).
    let app_info = crate::proxy::classify::classify_captured_request(
        &target_host,
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

    let req = InterceptedRequest {
        id: request_id,
        timestamp: timestamp_now(),
        method,
        host: target_host.clone(),
        path,
        query_params,
        status: Some(status),
        latency_ms: Some(latency),
        scheme: "https".to_string(),
        req_headers,
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

    log::info!(
        "HTTPS CONNECT tunnel completed: {} -> {} ({}ms, status: {:?})",
        client_addr,
        target_addr,
        latency,
        status
    );
}
