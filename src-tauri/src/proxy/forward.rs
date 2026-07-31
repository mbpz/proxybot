//! Forwarding helpers: build upstream requests, connect to remotes, pipe
//! raw TCP and WebSocket frames bidirectionally.

use super::rules::RemoteTarget;
use super::tls::build_client_config;
use crate::cert::CertManager;
use crate::network::NetworkConditionEngine;
use rustls::pki_types::ServerName;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

/// Build the raw HTTP/1.1 request bytes sent to an upstream server.
pub(super) fn build_upstream_request(
    method: &str,
    path: &str,
    host: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Vec<u8> {
    let mut request_headers: Vec<(String, String)> = headers
        .iter()
        .filter(|(name, _)| {
            !name.eq_ignore_ascii_case("host")
                && !name.eq_ignore_ascii_case("connection")
                && !name.eq_ignore_ascii_case("proxy-connection")
                && !name.eq_ignore_ascii_case("content-length")
        })
        .cloned()
        .collect();
    request_headers.insert(0, ("Host".to_string(), host.to_string()));
    request_headers.push(("Connection".to_string(), "close".to_string()));
    if !body.is_empty() {
        request_headers.push(("Content-Length".to_string(), body.len().to_string()));
    }

    let mut request = format!("{} {} HTTP/1.1\r\n", method, path).into_bytes();
    for (name, value) in request_headers {
        request.extend_from_slice(format!("{}: {}\r\n", name, value).as_bytes());
    }
    request.extend_from_slice(b"\r\n");
    request.extend_from_slice(body);
    request
}

pub(super) async fn read_full_response(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut response = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(15),
        stream.read_to_end(&mut response),
    )
    .await
    .map_err(|_| "Timed out reading upstream response".to_string())?
    .map_err(|e| format!("Read upstream response failed: {}", e))?;
    Ok(response)
}

/// Forward a request to a MapRemote target and return the raw response bytes.
pub(super) async fn forward_map_remote(
    cert_manager: &CertManager,
    target: &RemoteTarget,
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<Vec<u8>, String> {
    use super::rules::combine_remote_path;

    let addr = format!("{}:{}", target.host, target.port);
    let remote_path = combine_remote_path(&target.path_prefix, path);
    let request = build_upstream_request(method, &remote_path, &target.host, headers, body);

    let mut tcp = TcpStream::connect(&addr)
        .await
        .map_err(|e| format!("Failed to connect MAPREMOTE target {}: {}", addr, e))?;

    if target.scheme == "https" {
        let config = build_client_config(cert_manager)?;
        let server_name_static: &'static str = Box::leak(target.host.clone().into_boxed_str());
        let server_name = ServerName::try_from(server_name_static)
            .map_err(|e| format!("Invalid MAPREMOTE server name {}: {}", target.host, e))?;
        let connector = TlsConnector::from(Arc::new(config));
        let mut tls = connector
            .connect(server_name, tcp)
            .await
            .map_err(|e| format!("TLS connect to MAPREMOTE target {} failed: {}", addr, e))?;
        tls.write_all(&request)
            .await
            .map_err(|e| format!("Write MAPREMOTE request failed: {}", e))?;
        let mut response = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            tls.read_to_end(&mut response),
        )
        .await
        .map_err(|_| "Timed out reading MAPREMOTE HTTPS response".to_string())?
        .map_err(|e| format!("Read MAPREMOTE HTTPS response failed: {}", e))?;
        Ok(response)
    } else {
        tcp.write_all(&request)
            .await
            .map_err(|e| format!("Write MAPREMOTE request failed: {}", e))?;
        read_full_response(&mut tcp).await
    }
}

/// Pipe data between client and upstream using plain TCP (for tunnel mode).
pub(super) async fn pipe_tcp_bidirectional(
    mut client_stream: TcpStream,
    mut upstream_stream: TcpStream,
    network: &NetworkConditionEngine,
) -> Result<(), String> {
    let mut client_buf = vec![0u8; 16384];
    let mut upstream_buf = vec![0u8; 16384];

    loop {
        tokio::select! {
            n = client_stream.read(&mut client_buf) => {
                let n = n.map_err(|e| format!("Read from client failed: {}", e))?;
                if n == 0 {
                    let _ = upstream_stream.shutdown().await;
                    return Ok(());
                }
                // Apply network conditions before writing upstream
                let effect = network.apply(n);
                if effect.drop {
                    continue; // drop this chunk
                }
                if effect.delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(effect.delay_ms)).await;
                }
                upstream_stream.write_all(&client_buf[..n]).await
                    .map_err(|e| format!("Write to upstream failed: {}", e))?;
            }
            n = upstream_stream.read(&mut upstream_buf) => {
                let n = n.map_err(|e| format!("Read from upstream failed: {}", e))?;
                if n == 0 {
                    return Ok(());
                }
                // Apply network conditions before writing to client
                let effect = network.apply(n);
                if effect.drop {
                    continue; // drop this chunk
                }
                if effect.delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(effect.delay_ms)).await;
                }
                client_stream.write_all(&upstream_buf[..n]).await
                    .map_err(|e| format!("Write to client failed: {}", e))?;
            }
        }
    }
}

/// Run bidirectional WebSocket frame forwarding between client and upstream,
/// capturing frames and recording them to the database.
#[allow(clippy::too_many_arguments)]
pub(super) async fn pipe_ws_bidirectional(
    mut client_stream: TcpStream,
    mut upstream_stream: TcpStream,
    request_id: String,
    db_state: &Arc<crate::db::DbState>,
    network: &NetworkConditionEngine,
    ws_frame_tx: &tokio::sync::broadcast::Sender<(String, crate::proxy::WsFrame)>,
) -> Result<(), String> {
    use super::protocol::{decode_ws_payload, parse_ws_frame_header};
    use crate::db::{record_ws_frame, timestamp_now_for_ws};

    let mut client_buf = vec![0u8; 65536];
    let mut upstream_buf = vec![0u8; 65536];
    let mut client_remainder: Vec<u8> = Vec::new();
    let mut upstream_remainder: Vec<u8> = Vec::new();

    loop {
        tokio::select! {
            n = client_stream.read(&mut client_buf) => {
                let n = n.map_err(|e| format!("WS read from client failed: {}", e))?;
                if n == 0 { return Ok(()); }

                let effect = network.apply(n);
                if effect.drop { continue; }
                if effect.delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(effect.delay_ms)).await;
                }

                // Forward raw bytes to upstream
                upstream_stream.write_all(&client_buf[..n]).await
                    .map_err(|e| format!("WS write to upstream failed: {}", e))?;

                // Parse frames from accumulated buffer
                client_remainder.extend_from_slice(&client_buf[..n]);
                while let Some((header, total)) = parse_ws_frame_header(&client_remainder) {
                    let (_, text) = decode_ws_payload(&client_remainder, &header);
                    let ts = timestamp_now_for_ws();
                    if let Ok(conn) = db_state.conn.lock() {
                        let _ = record_ws_frame(&conn, &request_id, "outgoing", header.opcode, &text, None, header.payload_len, &ts);
                    }
                    let truncated = header.payload_len > crate::ws_frames::MAX_PAYLOAD_SIZE;
                    let frame = crate::proxy::WsFrame {
                        direction: "outgoing".to_string(),
                        timestamp: ts.clone(),
                        payload: text,
                        size: header.payload_len,
                        opcode: header.opcode,
                        truncated,
                    };
                    let _ = ws_frame_tx.send((request_id.clone(), frame));
                    client_remainder.drain(..total);
                }
            }
            n = upstream_stream.read(&mut upstream_buf) => {
                let n = n.map_err(|e| format!("WS read from upstream failed: {}", e))?;
                if n == 0 { return Ok(()); }

                // Forward raw bytes to client
                client_stream.write_all(&upstream_buf[..n]).await
                    .map_err(|e| format!("WS write to client failed: {}", e))?;

                // Parse frames from accumulated buffer
                upstream_remainder.extend_from_slice(&upstream_buf[..n]);
                while let Some((header, total)) = parse_ws_frame_header(&upstream_remainder) {
                    let (_, text) = decode_ws_payload(&upstream_remainder, &header);
                    let ts = timestamp_now_for_ws();
                    if let Ok(conn) = db_state.conn.lock() {
                        let _ = record_ws_frame(&conn, &request_id, "incoming", header.opcode, &text, None, header.payload_len, &ts);
                    }
                    let truncated = header.payload_len > crate::ws_frames::MAX_PAYLOAD_SIZE;
                    let frame = crate::proxy::WsFrame {
                        direction: "incoming".to_string(),
                        timestamp: ts.clone(),
                        payload: text,
                        size: header.payload_len,
                        opcode: header.opcode,
                        truncated,
                    };
                    let _ = ws_frame_tx.send((request_id.clone(), frame));
                    upstream_remainder.drain(..total);
                }
            }
        }
    }
}
