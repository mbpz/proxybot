//! Per-connection request handling entry point.
//!
//! `handle_client` is the dispatcher: it peeks the first byte to detect TLS,
//! recovers the original destination via DIOCNATLOOK for transparent mode,
//! parses the first line to decide between CONNECT (HTTPS) and plain HTTP,
//! then hands off to the specialized handler in `https` or `http`.

use super::http::handle_http;
use super::https::handle_https_connect;
use super::protocol::{find_colon, parse_host_port, trim_bytes};
use super::requests::get_or_create_device;
use super::tls::{extract_sni_from_client_hello, get_original_dst_addr};
use super::ProxyContext;
use crate::config::proxy_port;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

pub(super) async fn handle_client(
    ctx: ProxyContext,
    client_stream: TcpStream,
    client_addr: std::net::SocketAddr,
) {
    // Get or create device for this client IP
    let client_ip = client_addr.ip().to_string();
    let device_ctx = get_or_create_device(&ctx.db_state, &client_ip).await;

    // Peek at the first byte to detect TLS without consuming it.
    // TcpStream::peek() in tokio reads without advancing the cursor,
    // so the TLS acceptor will still see the full ClientHello starting at byte 0.
    let mut peek_buf = [0u8; 1];
    let peek_n = match client_stream.peek(&mut peek_buf).await {
        Ok(n) => n,
        Err(e) => {
            log::error!("Peek from client {} failed: {}", client_addr, e);
            return;
        }
    };

    let is_tls = peek_n > 0 && peek_buf[0] == 0x16;

    // Now read all buffered data (this consumes the stream).
    let mut client_stream = client_stream;
    let mut buf = vec![0u8; 32768];
    let n = match client_stream.read(&mut buf).await {
        Ok(n) => n,
        Err(e) => {
            log::error!("Read from client {} failed: {}", client_addr, e);
            return;
        }
    };

    if n == 0 {
        return;
    }

    let data = &buf[..n];

    if is_tls {
        // This is a TLS ClientHello for transparent HTTPS.
        // Use DIOCNATLOOK to determine the real destination.
        if let Some(original_dst) = get_original_dst_addr(&client_stream) {
            log::info!(
                "Transparent HTTPS connection from {} (original dst: {})",
                client_addr,
                original_dst
            );
            handle_transparent_https(
                ctx,
                device_ctx.clone(),
                client_stream,
                client_addr,
                original_dst,
                data.to_vec(),
            )
            .await;
            return;
        } else {
            log::warn!(
                "Could not get original destination for TLS connection from {}",
                client_addr
            );
        }
    }

    let first_line = match data.split(|&b| b == b'\n').next() {
        Some(line) => String::from_utf8_lossy(trim_bytes(line)).to_string(),
        None => return,
    };

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 3 {
        log::warn!("Invalid request from {}: {:?}", client_addr, first_line);
        return;
    }

    let method = parts[0];
    let path = parts[1];
    let _version = parts[2];

    if method == "CONNECT" {
        if let Some((host, port)) = parse_host_port(path) {
            handle_https_connect(
                ctx,
                device_ctx.clone(),
                client_stream,
                client_addr,
                host.to_string(),
                port,
            )
            .await;
        }
        return;
    }

    let mut headers = Vec::new();
    let mut body_start = 0;

    for i in 0..data.len().saturating_sub(3) {
        if data[i] == b'\r' && data[i + 1] == b'\n' && data[i + 2] == b'\r' && data[i + 3] == b'\n'
        {
            body_start = i + 4;
            break;
        }
    }

    let header_section = &data[..body_start];
    for line in header_section.split(|&b| b == b'\n') {
        let line_trimmed = trim_bytes(line);
        if line_trimmed.is_empty() || line_trimmed == b"\r" {
            continue;
        }
        if let Some((name_bytes, value_bytes)) = find_colon(line_trimmed) {
            let name = String::from_utf8_lossy(trim_bytes(name_bytes)).to_string();
            let value = String::from_utf8_lossy(trim_bytes(value_bytes)).to_string();
            headers.push((name, value));
        }
    }

    let body = &data[body_start..];

    // For transparent HTTP, try to use SO_ORIGINAL_DST as fallback.
    // This handles cases where the Host header might be missing or incorrect.
    let (host, port) = if let Some(original_dst) = get_original_dst_addr(&client_stream) {
        if !original_dst.ip().is_loopback() && original_dst.port() != proxy_port() {
            // Use the original destination from pf redirection.
            (original_dst.ip().to_string(), original_dst.port())
        } else {
            // Fall back to Host header parsing.
            parse_host_from_headers(&headers)
        }
    } else {
        // No SO_ORIGINAL_DST available, use Host header.
        parse_host_from_headers(&headers)
    };

    let path_to_use = if path.starts_with("http://") {
        path.split("//")
            .nth(2)
            .map(|s| {
                s.split_once('/')
                    .map(|(_, p)| format!("/{}", p))
                    .unwrap_or_else(|| "/".to_string())
            })
            .unwrap_or_else(|| "/".to_string())
    } else {
        path.to_string()
    };

    if let Err(e) = handle_http(
        ctx,
        device_ctx,
        client_stream,
        client_addr,
        method,
        &path_to_use,
        &host,
        port,
        &headers,
        body,
    )
    .await
    {
        log::error!("HTTP handle failed for {}: {}", client_addr, e);
    }
}

/// Handle a transparent HTTPS request where the original destination
/// is recovered via DIOCNATLOOK. This performs MITM to intercept
/// the encrypted traffic.
async fn handle_transparent_https(
    ctx: ProxyContext,
    device_ctx: Option<super::DeviceContext>,
    client_stream: TcpStream,
    client_addr: std::net::SocketAddr,
    original_dst: std::net::SocketAddr,
    tls_data: Vec<u8>,
) {
    let target_host = original_dst.ip().to_string();
    let target_port = original_dst.port();

    log::info!(
        "Transparent HTTPS from {} (original dst: {}:{})",
        client_addr,
        target_host,
        target_port
    );

    // Try to extract SNI from TLS ClientHello for better app classification
    let sni_host = extract_sni_from_client_hello(&tls_data);

    // Use SNI if available, otherwise fall back to original destination
    let effective_host = sni_host.clone().unwrap_or_else(|| target_host.clone());

    log::debug!(
        "Transparent HTTPS effective host: {} (SNI: {:?}, original: {})",
        effective_host,
        sni_host,
        target_host
    );

    // Use the existing HTTPS CONNECT handler with the SNI-based host if available.
    handle_https_connect(
        ctx,
        device_ctx,
        client_stream,
        client_addr,
        effective_host,
        target_port,
    )
    .await;
}

/// Parse the `Host` header into (host, port). Falls back to ("localhost", 80)
/// when the header is missing or malformed.
fn parse_host_from_headers(headers: &[(String, String)]) -> (String, u16) {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("host"))
        .map(|(_, v)| {
            let v = v.trim();
            if let Some((h, p)) = parse_host_port(v) {
                (h.to_string(), p)
            } else {
                (v.to_string(), 80)
            }
        })
        .unwrap_or_else(|| ("localhost".to_string(), 80))
}
