use crate::app_rules;
use crate::cert::CertManager;
use crate::dns;
use crate::dns::DnsState;
use crate::network::NetworkInfo;
use std::net::{IpAddr, SocketAddr};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use rustls::client::danger as rustls_danger;
use rustls::{
    ServerConfig,
    ClientConfig,
    pki_types::ServerName,
    SignatureScheme,
    DigitallySignedStruct,
};
use libc;
use std::fs::OpenOptions;
use sha1::Digest;
use base64::Engine;
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use dashmap::DashMap;
use std::sync::LazyLock;

const PROXY_PORT: u16 = 8080;

static PROXY_RUNNING: AtomicBool = AtomicBool::new(false);

/// Global store for intercepted requests, keyed by request ID.
static REQUEST_STORE: LazyLock<DashMap<String, InterceptedRequest>, fn() -> DashMap<String, InterceptedRequest>> =
    LazyLock::new(|| DashMap::new());

/// Maximum response body size to store (10KB).
const MAX_BODY_SIZE: usize = 10 * 1024;

#[derive(Clone, serde::Serialize)]
pub struct InterceptedRequest {
    pub id: String,
    pub timestamp: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: Option<u16>,
    pub latency_ms: Option<u64>,
    pub scheme: String,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
    pub request_headers: Option<String>,
    pub response_headers: Option<String>,
    pub response_body: Option<String>,
    pub request_body: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct WssMessage {
    pub id: String,
    pub timestamp: String,
    pub host: String,
    pub direction: String,
    pub size: usize,
    pub content: String,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
}

struct ProxyContext {
    app_handle: AppHandle,
    #[allow(dead_code)]
    cert_manager: Arc<CertManager>,
}

/// Check if an HTTP request is a WebSocket upgrade request.
/// Returns (Sec-WebSocket-Key, Sec-WebSocket-Protocol) if it is.
fn is_websocket_upgrade(request_data: &[u8]) -> Option<(String, Option<String>)> {
    // Look for "Upgrade: websocket" and "Connection: Upgrade" headers
    let data_str = String::from_utf8_lossy(request_data);

    let has_upgrade = data_str.lines()
        .any(|line| line.eq_ignore_ascii_case("Upgrade: websocket"));

    let has_connection = data_str.lines()
        .any(|line| line.eq_ignore_ascii_case("Connection: Upgrade"));

    if has_upgrade && has_connection {
        let mut ws_key = None;
        let mut ws_protocol = None;
        for line in data_str.lines() {
            if line.starts_with("Sec-WebSocket-Key:") {
                let key = line.trim_start_matches("Sec-WebSocket-Key:").trim();
                ws_key = Some(key.to_string());
            }
            if line.starts_with("Sec-WebSocket-Protocol:") {
                let proto = line.trim_start_matches("Sec-WebSocket-Protocol:").trim();
                ws_protocol = Some(proto.to_string());
            }
        }
        if let Some(key) = ws_key {
            return Some((key, ws_protocol));
        }
    }
    None
}

/// Compute the Sec-WebSocket-Accept key from the client's key.
/// RFC 6455: base64(SHA1(key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))
fn compute_ws_accept_key(client_key: &str) -> String {
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined = format!("{}{}", client_key, GUID);
    let mut hasher = sha1::Sha1::new();
    hasher.update(combined.as_bytes());
    let result = hasher.finalize();
    base64_encode(&result)
}

fn base64_encode(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn timestamp_now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| format!("{}.{:03}", dur.as_secs(), dur.subsec_millis()))
        .unwrap_or_else(|_| "0.000".to_string())
}

fn generate_request_id() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| format!("req-{}", dur.as_nanos()))
        .unwrap_or_else(|_| format!("req-{}", std::time::Instant::now().elapsed().as_nanos()))
}

/// Format headers as "Header-Name: value\r\n..." string.
fn format_headers(headers: &[(String, String)]) -> String {
    headers
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join("\r\n")
}

/// Decode body bytes to UTF-8 string, fallback to binary marker if invalid.
fn decode_body(body: &[u8]) -> String {
    let body = if body.len() > MAX_BODY_SIZE {
        &body[..MAX_BODY_SIZE]
    } else {
        body
    };
    match String::from_utf8(body.to_vec()) {
        Ok(s) => s,
        Err(_) => format!("[Binary {} bytes]", body.len()),
    }
}

/// Parse HTTP response headers from raw response data.
fn parse_response_headers(data: &[u8]) -> Option<String> {
    let header_end = data.windows(4).position(|w| w == b"\r\n\r\n")?;
    let header_slice = &data[..header_end];
    let mut headers = Vec::new();
    for line in header_slice.split(|&b| b == b'\n') {
        let line_trimmed = if line.ends_with(b"\r") {
            &line[..line.len() - 1]
        } else {
            line
        };
        if line_trimmed.is_empty() {
            continue;
        }
        if let Some(colon_pos) = line_trimmed.iter().position(|&b| b == b':') {
            let name_bytes = &line_trimmed[..colon_pos];
            let value_bytes = &line_trimmed[colon_pos + 1..];
            let name = String::from_utf8_lossy(name_bytes).trim().to_string();
            let value = String::from_utf8_lossy(value_bytes).trim().to_string();
            headers.push((name, value));
        }
    }
    Some(format_headers(&headers))
}

/// Extract response body from raw HTTP response data.
fn extract_response_body(data: &[u8]) -> Option<Vec<u8>> {
    let header_end = data.windows(4).position(|w| w == b"\r\n\r\n")?;
    let body_start = header_end + 4;
    if body_start >= data.len() {
        return Some(Vec::new());
    }
    Some(data[body_start..].to_vec())
}

/// Store an intercepted request in the global store.
fn store_request(req: InterceptedRequest) {
    REQUEST_STORE.insert(req.id.clone(), req);
}

fn parse_host_port(s: &str) -> Option<(&str, u16)> {
    if let Some((host, port_str)) = s.split_once(':') {
        port_str.parse().ok().map(|p| (host, p))
    } else {
        None
    }
}

/// macOS pf NAT lookup via DIOCNATLOOK ioctl.
///
/// Recovers the original destination address/port from a pf rdr redirect.
/// This is the correct method for macOS - SO_ORIGINAL_DST does not exist on macOS.
fn get_original_dst(peer_addr: SocketAddr, local_addr: SocketAddr) -> Option<SocketAddr> {
    // Open /dev/pf for DIOCNATLOOK ioctl (O_RDWR required for _IOWR ioctls)
    let fd = match OpenOptions::new().read(true).write(true).open("/dev/pf") {
        Ok(f) => f,
        Err(e) => {
            log::warn!("Failed to open /dev/pf: {}", e);
            return None;
        }
    };

    // Build the pfioc_natlook structure
    // struct pfioc_natlook {
    //     struct pf_addr saddr;  // source IP (the original client)
    //     struct pf_addr daddr;  // destination IP as seen by proxy (127.0.0.1)
    //     struct pf_addr rsaddr; // out: original source
    //     struct pf_addr rdaddr; // out: original destination (what we want)
    //     u_int16_t sport;       // source port
    //     u_int16_t dport;       // destination port as seen by proxy
    //     u_int16_t rsport;
    //     u_int16_t rdport;      // out: original destination port (what we want)
    //     sa_family_t af;        // AF_INET = 2
    //     u_int8_t proto;        // IPPROTO_TCP = 6
    //     u_int8_t direction;    // PF_OUT = 2
    // };
    #[repr(C)]
    struct PfiocNatlook {
        saddr: [u8; 16],
        daddr: [u8; 16],
        rsaddr: [u8; 16],
        rdaddr: [u8; 16],
        sport: u16,
        dport: u16,
        rsport: u16,
        rdport: u16,
        af: u8,
        proto: u8,
        direction: u8,
        pad: [u8; 5],
    }

    // DIOCNATLOOK = _IOWR('D', 23, struct pfioc_natlook) = 0xC0544417
    const DIOCNATLOOK: libc::c_ulong = 0xC0544417;

    // Helper to pack an IPv4 address into the pf_addr array (16 bytes, network order)
    fn pack_ipv4(addr: &IpAddr, arr: &mut [u8; 16]) {
        arr.fill(0);
        if let IpAddr::V4(ipv4) = addr {
            arr[..4].copy_from_slice(&ipv4.octets());
        }
    }

    let mut nl = PfiocNatlook {
        saddr: [0u8; 16],
        daddr: [0u8; 16],
        rsaddr: [0u8; 16],
        rdaddr: [0u8; 16],
        sport: peer_addr.port().to_be(),
        dport: local_addr.port().to_be(),
        rsport: 0,
        rdport: 0,
        af: 2, // AF_INET
        proto: 6, // IPPROTO_TCP
        direction: 1, // PF_IN (rdr redirects inbound traffic)
        pad: [0u8; 5],
    };

    pack_ipv4(&peer_addr.ip(), &mut nl.saddr);
    pack_ipv4(&local_addr.ip(), &mut nl.daddr);

    // Issue the ioctl
    let ret = unsafe {
        libc::ioctl(fd.as_raw_fd(), DIOCNATLOOK as libc::c_ulong, &mut nl as *mut _ as *mut libc::c_void)
    };

    if ret != 0 {
        let err = std::io::Error::last_os_error();
        log::warn!("DIOCNATLOOK failed for {}→{}: {}", peer_addr, local_addr, err);
        return None;
    }

    // Extract original destination from rdaddr:rdport
    let ip = std::net::Ipv4Addr::new(nl.rdaddr[0], nl.rdaddr[1], nl.rdaddr[2], nl.rdaddr[3]);
    let port = u16::from_be(nl.rdport);
    Some(SocketAddr::new(IpAddr::V4(ip), port))
}

/// Get the original destination address of a socket using DIOCNATLOOK on macOS.
/// This is used for transparent proxy mode to determine where the browser was
/// trying to connect before pf redirected it.
fn get_original_dst_addr(socket: &tokio::net::TcpStream) -> Option<SocketAddr> {
    let peer_addr = socket.peer_addr().ok()?;
    let local_addr = socket.local_addr().ok()?;
    get_original_dst(peer_addr, local_addr)
}

/// Handle a transparent HTTPS request where the original destination
/// is recovered via DIOCNATLOOK. This performs MITM to intercept
/// the encrypted traffic.
async fn handle_transparent_https(
    ctx: ProxyContext,
    client_stream: TcpStream,
    client_addr: SocketAddr,
    original_dst: SocketAddr,
) {
    let target_host = original_dst.ip().to_string();
    let target_port = original_dst.port();

    log::info!(
        "Transparent HTTPS from {} (original dst: {}:{})",
        client_addr,
        target_host,
        target_port
    );

    // Use the existing HTTPS CONNECT handler but with the original destination.
    handle_https_connect(ctx, client_stream, client_addr, target_host, target_port).await;
}

/// A ServerCertVerifier that accepts all certificates.
/// Used for MITM proxy upstream connections where we need to inspect decrypted traffic.
#[derive(Debug)]
struct NoVerification;

impl rustls_danger::ServerCertVerifier for NoVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls_danger::ServerCertVerified, rustls::Error> {
        Ok(rustls_danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<rustls_danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls_danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<rustls_danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls_danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}

/// Build a rustls ClientConfig for connecting to upstream servers.
/// Uses dangerous certificate verification to accept all certs (MITM proxy mode).
fn build_client_config(_cert_manager: &CertManager) -> Result<ClientConfig, String> {
    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerification))
        .with_no_client_auth();

    Ok(config)
}

/// Parse HTTP request line and headers from buffered data.
fn parse_http_request(data: &[u8]) -> Option<(String, String, String, Vec<(String, String)>)> {
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
    while pos < data.len() {
        let rest = &data[pos..];
        let line_end = rest.windows(2).position(|w| w == b"\r\n")?;
        pos += line_end + 2;
        if line_end == 0 {
            break;
        }
        let line = String::from_utf8_lossy(&data[pos - line_end - 2..pos - 2]);
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }

    Some((method, path, version, headers))
}

/// Find status code from HTTP response.
fn parse_response_status(data: &[u8]) -> Option<u16> {
    let first_line = data.split(|&b| b == b'\r').next()?;
    let parts: Vec<&[u8]> = first_line.split(|&b| b == b' ').collect();
    if parts.len() >= 2 {
        String::from_utf8_lossy(parts[1]).parse().ok()
    } else {
        None
    }
}

/// Pipe data between client and upstream using plain TCP (for tunnel mode).
async fn pipe_tcp_bidirectional(
    mut client_stream: TcpStream,
    mut upstream_stream: TcpStream,
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
                upstream_stream.write_all(&client_buf[..n]).await
                    .map_err(|e| format!("Write to upstream failed: {}", e))?;
            }
            n = upstream_stream.read(&mut upstream_buf) => {
                let n = n.map_err(|e| format!("Read from upstream failed: {}", e))?;
                if n == 0 {
                    return Ok(());
                }
                client_stream.write_all(&upstream_buf[..n]).await
                    .map_err(|e| format!("Write to client failed: {}", e))?;
            }
        }
    }
}

/// Relay WebSocket frames bidirectionally between browser and server.
/// Emits intercepted-wss events for each Text/Binary frame.
async fn handle_websocket_relay(
    ctx: ProxyContext,
    client_stream: tokio_rustls::server::TlsStream<TcpStream>,
    upstream_stream: tokio_rustls::client::TlsStream<TcpStream>,
    target_host: String,
) {
    let app_info = app_rules::classify_host(&target_host);
    let (app_name, app_icon) = app_info
        .map(|(n, i)| (Some(n.to_string()), Some(i.to_string())))
        .unwrap_or((None, None));

    let target_host_clone = target_host.clone();
    let app_name_clone = app_name.clone();
    let app_icon_clone = app_icon.clone();
    let app_handle_clone = ctx.app_handle.clone();
    let target_host_for_log = target_host.clone();

    // Create WebSocket streams
    use tokio_tungstenite::tungstenite::protocol::Role;
    let browser_ws = WebSocketStream::from_raw_socket(client_stream, Role::Server, None).await;
    let server_ws = WebSocketStream::from_raw_socket(upstream_stream, Role::Client, None).await;

    let request_id_prefix = generate_request_id();
    let request_id_prefix_clone = request_id_prefix.clone();

    // Channels for relaying frames between browser and server
    // browser_to_server: browser_ws sends TO server_ws
    // server_to_browser: server_ws sends TO browser_ws
    let (browser_to_server_tx, mut browser_to_server_rx) = tokio::sync::mpsc::channel::<Message>(100);
    let (server_to_browser_tx, mut server_to_browser_rx) = tokio::sync::mpsc::channel::<Message>(100);

    // Spawn browser -> server relay task
    let browser_ws_handle = tokio::spawn(async move {
        let mut ws = browser_ws;
        let mut msg_id = 0;

        loop {
            tokio::select! {
                // Read from browser WebSocket
                msg_result = ws.next() => {
                    match msg_result {
                        Some(Ok(msg)) => {
                            let forward_msg = match &msg {
                                Message::Text(_) | Message::Binary(_) => true,
                                Message::Close(_) => {
                                    // Forward close to server via channel, then close locally
                                    let _ = browser_to_server_tx.send(Message::Close(None)).await;
                                    if let Err(e) = ws.send(Message::Close(None)).await {
                                        log::error!("Failed to send Close to browser: {}", e);
                                    }
                                    break;
                                }
                                Message::Ping(data) => {
                                    // Respond to ping directly to browser
                                    if let Err(e) = ws.send(Message::Pong(data.clone())).await {
                                        log::error!("Failed to send Pong to browser: {}", e);
                                    }
                                    false
                                }
                                Message::Pong(_) | Message::Frame(_) => false,
                            };

                            if forward_msg {
                                // Emit intercepted event
                                let content = match &msg {
                                    Message::Text(s) => s.to_string(),
                                    Message::Binary(b) => format!("[Binary {} bytes]", b.len()),
                                    _ => String::new(),
                                };

                                let size = msg.len();
                                let wss_msg = WssMessage {
                                    id: format!("{}-ws-{}", request_id_prefix, msg_id),
                                    timestamp: timestamp_now(),
                                    host: target_host_clone.clone(),
                                    direction: "up".to_string(),
                                    size,
                                    content,
                                    app_name: app_name_clone.clone(),
                                    app_icon: app_icon_clone.clone(),
                                };
                                let _ = app_handle_clone.emit("intercepted-wss", &wss_msg);

                                // Forward to server via channel
                                if browser_to_server_tx.send(msg).await.is_err() {
                                    break;
                                }
                                msg_id += 1;
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("WebSocket read error from browser: {}", e);
                            break;
                        }
                        None => break,
                    }
                }
                // Receive from server -> browser channel and forward to browser WS
                msg = server_to_browser_rx.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Err(e) = ws.send(msg).await {
                                log::error!("WebSocket send error to browser: {}", e);
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    });

    // Spawn server -> browser relay task
    let server_ws_handle = tokio::spawn(async move {
        let mut ws = server_ws;
        let mut msg_id = 0;
        let request_id = request_id_prefix_clone;
        let target = target_host.clone();
        let app_name = app_name.clone();
        let app_icon = app_icon.clone();
        let app_handle = ctx.app_handle.clone();

        loop {
            tokio::select! {
                // Read from server WebSocket
                msg_result = ws.next() => {
                    match msg_result {
                        Some(Ok(msg)) => {
                            let forward_msg = match &msg {
                                Message::Text(_) | Message::Binary(_) => true,
                                Message::Close(_) => {
                                    // Forward close to browser via channel, then close locally
                                    let _ = server_to_browser_tx.send(Message::Close(None)).await;
                                    if let Err(e) = ws.send(Message::Close(None)).await {
                                        log::error!("Failed to send Close to server: {}", e);
                                    }
                                    break;
                                }
                                Message::Ping(data) => {
                                    // Respond to ping directly to server
                                    if let Err(e) = ws.send(Message::Pong(data.clone())).await {
                                        log::error!("Failed to send Pong to server: {}", e);
                                    }
                                    false
                                }
                                Message::Pong(_) | Message::Frame(_) => false,
                            };

                            if forward_msg {
                                // Emit intercepted event
                                let content = match &msg {
                                    Message::Text(s) => s.to_string(),
                                    Message::Binary(b) => format!("[Binary {} bytes]", b.len()),
                                    _ => String::new(),
                                };

                                let size = msg.len();
                                let wss_msg = WssMessage {
                                    id: format!("{}-ws-{}", request_id, msg_id),
                                    timestamp: timestamp_now(),
                                    host: target.clone(),
                                    direction: "down".to_string(),
                                    size,
                                    content,
                                    app_name: app_name.clone(),
                                    app_icon: app_icon.clone(),
                                };
                                let _ = app_handle.emit("intercepted-wss", &wss_msg);

                                // Forward to browser via channel
                                if server_to_browser_tx.send(msg).await.is_err() {
                                    break;
                                }
                                msg_id += 1;
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("WebSocket read error from server: {}", e);
                            break;
                        }
                        None => break,
                    }
                }
                // Receive from browser -> server channel and forward to server WS
                msg = browser_to_server_rx.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Err(e) = ws.send(msg).await {
                                log::error!("WebSocket send error to server: {}", e);
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    });

    // Wait for both relay tasks to complete
    let _ = tokio::join!(browser_ws_handle, server_ws_handle);

    log::info!("WebSocket relay completed for {}", target_host_for_log);
}

async fn handle_https_connect(
    ctx: ProxyContext,
    client_stream: TcpStream,
    client_addr: SocketAddr,
    target_host: String,
    target_port: u16,
) {
    let target_addr = format!("{}:{}", target_host, target_port);
    log::info!("HTTPS CONNECT tunnel to {} from {}", target_addr, client_addr);

    let start = std::time::Instant::now();

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
            if let Err(e) = pipe_tcp_bidirectional(client, upstream).await {
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

    // Accept TLS from browser (browser sends TLS ClientHello after CONNECT for explicit proxy)
    let mut client_tls_stream = match tls_acceptor.accept(client_stream).await {
        Ok(stream) => stream,
        Err(e) => {
            log::error!("TLS accept failed for browser {}: {}", client_addr, e);
            return;
        }
    };

    log::debug!("TLS handshake completed with browser {}", client_addr);

    // Send HTTP 200 Connection Established over TLS to browser
    // This tells the browser the tunnel is ready
    if let Err(e) = client_tls_stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await
    {
        log::error!("Failed to send 200 response to browser {}: {}", client_addr, e);
        return;
    }

    // Build client TLS config for connecting to upstream
    let client_config = match build_client_config(&ctx.cert_manager) {
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
            return;
        }
    };

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
    let mut upstream_tls_stream = match connector.connect(server_name, upstream_tcp).await {
        Ok(stream) => stream,
        Err(e) => {
            log::error!("TLS connect to upstream {} failed: {}", upstream_addr, e);
            return;
        }
    };

    log::debug!("TLS handshake completed with upstream {}", upstream_addr);

    // Read the HTTP request from browser to check for WebSocket upgrade
    let mut http_buf = vec![0u8; 16384];
    let http_n = match client_tls_stream.read(&mut http_buf).await {
        Ok(n) => n,
        Err(e) => {
            log::error!("Read HTTP request from browser TLS failed: {}", e);
            return;
        }
    };

    if http_n == 0 {
        log::warn!("Browser closed connection before sending HTTP request");
        return;
    }

    let http_data = http_buf[..http_n].to_vec();

    // Check if this is a WebSocket upgrade request
    if let Some((ws_key, ws_protocol)) = is_websocket_upgrade(&http_data) {
        log::info!("WebSocket upgrade detected for {} from {}", target_addr, client_addr);

        // Forward the HTTP upgrade request to upstream server
        if let Err(e) = upstream_tls_stream.write_all(&http_data).await {
            log::error!("Failed to forward upgrade request to upstream {}: {}", upstream_addr, e);
            return;
        }

        // Read the HTTP response from upstream (including 101 or error)
        let mut upstream_buf = vec![0u8; 16384];
        let upstream_n = match upstream_tls_stream.read(&mut upstream_buf).await {
            Ok(n) => n,
            Err(e) => {
                log::error!("Failed to read upgrade response from upstream {}: {}", upstream_addr, e);
                return;
            }
        };

        if upstream_n == 0 {
            log::error!("Upstream closed connection during upgrade for {}", upstream_addr);
            return;
        }

        let upstream_response = upstream_buf[..upstream_n].to_vec();

        // Check if upstream returned 101 Switching Protocols
        let response_str = String::from_utf8_lossy(&upstream_response);
        let is_101 = response_str.starts_with("HTTP/1.1 101") || response_str.starts_with("HTTP/1.0 101");

        if !is_101 {
            // Upstream did not accept WebSocket upgrade - fall back to blind relay
            log::warn!("Upstream {} did not accept WebSocket upgrade, falling back to relay", upstream_addr);

            // Forward the non-101 response to browser
            if let Err(e) = client_tls_stream.write_all(&upstream_response).await {
                log::error!("Failed to forward non-101 to browser: {}", e);
                return;
            }

            // Do blind relay - pipe data bidirectionally
            let mut client_tls_stream = client_tls_stream;
            let mut upstream_tls_stream = upstream_tls_stream;

            let (mut client_read, mut client_write) = tokio::io::split(&mut client_tls_stream);
            let (mut upstream_read, mut upstream_write) = tokio::io::split(&mut upstream_tls_stream);

            let mut client_buf = vec![0u8; 16384];
            let mut upstream_buf = vec![0u8; 16384];

            // First send the browser's HTTP request to upstream (already read in http_data)
            if let Err(e) = upstream_write.write_all(&http_data).await {
                log::error!("Write to upstream failed: {}", e);
                return;
            }

            // We've already read upstream's response into upstream_response, send it first
            if !upstream_response.is_empty() {
                if let Err(e) = client_write.write_all(&upstream_response).await {
                    log::error!("Write to client failed: {}", e);
                    return;
                }
            }

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
                            let _ = upstream_write.shutdown().await;
                            break;
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
                        if let Err(e) = client_write.write_all(&upstream_buf[..n]).await {
                            log::error!("Write to client failed: {}", e);
                            break;
                        }
                    }
                }
            }

            log::info!("Blind relay completed for {}", target_addr);
            return;
        }

        // Extract Sec-WebSocket-Protocol from upstream response if present
        let upstream_protocol = response_str.lines()
            .find(|line| line.starts_with("Sec-WebSocket-Protocol:"))
            .map(|line| line.trim_start_matches("Sec-WebSocket-Protocol:").trim().to_string());

        // Use upstream's protocol if present, otherwise use client's protocol
        let final_protocol = upstream_protocol.or(ws_protocol);

        // Compute Sec-WebSocket-Accept from client's key
        let accept_key = compute_ws_accept_key(&ws_key);

        // Build 101 response to send to browser
        let mut upgrade_response = format!(
            "HTTP/1.1 101 Switching Protocols\r\n\
            Upgrade: websocket\r\n\
            Connection: Upgrade\r\n\
            Sec-WebSocket-Accept: {}\r\n",
            accept_key
        );

        // Include Sec-WebSocket-Protocol if negotiated
        if let Some(ref proto) = final_protocol {
            upgrade_response.push_str(&format!("Sec-WebSocket-Protocol: {}\r\n", proto));
        }

        upgrade_response.push_str("\r\n");

        // Send 101 Switching Protocols response to browser
        if let Err(e) = client_tls_stream.write_all(upgrade_response.as_bytes()).await {
            log::error!("Failed to send 101 response to browser {}: {}", client_addr, e);
            return;
        }

        // Handle WebSocket relay
        handle_websocket_relay(ctx, client_tls_stream, upstream_tls_stream, target_host.clone()).await;
        return;
    }

    // Not a WebSocket upgrade - do blind relay (regular HTTPS)
    // Forward the HTTP data to upstream
    let mut client_tls_stream = client_tls_stream;
    let mut upstream_tls_stream = upstream_tls_stream;

    // Pipe data bidirectionally between the two TLS streams
    let (mut client_read, mut client_write) = tokio::io::split(&mut client_tls_stream);
    let (mut upstream_read, mut upstream_write) = tokio::io::split(&mut upstream_tls_stream);

    let mut client_buf = vec![0u8; 16384];
    let mut upstream_buf = vec![0u8; 16384];
    let mut request_data = http_data.clone();
    let mut response_data = Vec::new();

    // First send the already-read HTTP data to upstream
    if let Err(e) = upstream_write.write_all(&http_data).await {
        log::error!("Write to upstream failed: {}", e);
        return;
    }

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
                    let _ = upstream_write.shutdown().await;
                    break;
                }
                request_data.extend_from_slice(&client_buf[..n]);
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
                if let Err(e) = client_write.write_all(&upstream_buf[..n]).await {
                    log::error!("Write to client failed: {}", e);
                    break;
                }
            }
        }
    }

    // Log the intercepted request
    let latency = start.elapsed().as_millis() as u64;

    // Parse request and response for logging
    let (method, path) = parse_http_request(&request_data)
        .map(|(m, p, _, _)| (m, p))
        .unwrap_or_else(|| ("CONNECT".to_string(), "/".to_string()));

    let status = parse_response_status(&response_data);
    let request_id = generate_request_id();
    let response_headers = parse_response_headers(&response_data);
    let response_body = extract_response_body(&response_data).map(|b| decode_body(&b));

    // Parse request headers from request_data
    let request_headers = parse_http_request(&request_data)
        .map(|(_, _, _, headers)| format_headers(&headers));

    let app_info = app_rules::classify_host(&target_host);
    let (app_name, app_icon) = app_info
        .map(|(n, i)| (Some(n.to_string()), Some(i.to_string())))
        .unwrap_or((None, None));

    let req = InterceptedRequest {
        id: request_id.clone(),
        timestamp: timestamp_now(),
        method,
        host: target_host.clone(),
        path,
        status,
        latency_ms: Some(latency),
        scheme: "https".to_string(),
        app_name,
        app_icon,
        request_headers,
        response_headers,
        response_body,
        request_body: None,
    };

    store_request(req.clone());
    let _ = ctx.app_handle.emit("intercepted-request", &req);

    log::info!(
        "HTTPS CONNECT tunnel completed: {} -> {} ({}ms, status: {:?})",
        client_addr,
        target_addr,
        latency,
        status
    );
}

async fn handle_http(
    ctx: ProxyContext,
    client_stream: TcpStream,
    client_addr: SocketAddr,
    method: &str,
    path: &str,
    host: &str,
    port: u16,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(), String> {
    let target_addr = format!("{}:{}", host, port);
    log::info!("HTTP {} {} from {}", method, path, client_addr);

    let start = std::time::Instant::now();

    let mut target_stream = TcpStream::connect(&target_addr).await
        .map_err(|e| format!("Failed to connect to {}: {}", target_addr, e))?;

    let http_version = "HTTP/1.1";
    let mut request = format!("{} {} {}\r\n", method, path, http_version);
    for (name, value) in headers {
        request.push_str(&format!("{}: {}\r\n", name, value));
    }
    request.push_str("\r\n");

    target_stream.write_all(request.as_bytes()).await.map_err(|e| format!("Write request failed: {}", e))?;
    if !body.is_empty() {
        target_stream.write_all(body).await.map_err(|e| format!("Write body failed: {}", e))?;
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
    client_stream.write_all(&response_buf).await.map_err(|e| format!("Write response failed: {}", e))?;

    let latency = start.elapsed().as_millis() as u64;
    let request_id = generate_request_id();

    let status = parse_response_status(&response_buf);
    let response_headers = parse_response_headers(&response_buf);
    let response_body = extract_response_body(&response_buf).map(|b| decode_body(&b));

    let app_info = app_rules::classify_host(host);
    let (app_name, app_icon) = app_info
        .map(|(n, i)| (Some(n.to_string()), Some(i.to_string())))
        .unwrap_or((None, None));

    let req = InterceptedRequest {
        id: request_id.clone(),
        timestamp: timestamp_now(),
        method: method.to_string(),
        host: host.to_string(),
        path: path.to_string(),
        status,
        latency_ms: Some(latency),
        scheme: if port == 443 { "https" } else { "http" }.to_string(),
        app_name,
        app_icon,
        request_headers: Some(format_headers(headers)),
        response_headers,
        response_body,
        request_body: if body.is_empty() { None } else { Some(decode_body(body)) },
    };

    store_request(req.clone());
    let _ = ctx.app_handle.emit("intercepted-request", &req);
    Ok(())
}

fn trim_bytes(s: &[u8]) -> &[u8] {
    let start = s.iter().position(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n').unwrap_or(s.len());
    let end = s.iter().rposition(|&b| b != b' ' && b != b'\t' && b != b'\r' && b != b'\n').map(|p| p + 1).unwrap_or(0);
    &s[start..end.max(start)]
}

fn find_colon(line: &[u8]) -> Option<(&[u8], &[u8])> {
    for (i, &b) in line.iter().enumerate() {
        if b == b':' {
            return Some((&line[..i], &line[i+1..]));
        }
    }
    None
}

async fn handle_client(ctx: ProxyContext, client_stream: TcpStream, client_addr: SocketAddr) {
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
            handle_transparent_https(ctx, client_stream, client_addr, original_dst).await;
            return;
        } else {
            log::warn!("Could not get original destination for TLS connection from {}", client_addr);
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
            handle_https_connect(ctx, client_stream, client_addr, host.to_string(), port).await;
        }
        return;
    }

    let mut headers = Vec::new();
    let mut body_start = 0;

    for i in 0..data.len().saturating_sub(3) {
        if data[i] == b'\r' && data[i+1] == b'\n' && data[i+2] == b'\r' && data[i+3] == b'\n' {
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
        if !original_dst.ip().is_loopback() && original_dst.port() != PROXY_PORT {
            // Use the original destination from pf redirection.
            (original_dst.ip().to_string(), original_dst.port())
        } else {
            // Fall back to Host header parsing.
            headers
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case("host"))
                .and_then(|(_, v)| {
                    let v = v.trim();
                    if let Some((h, p)) = parse_host_port(v) {
                        Some((h.to_string(), p))
                    } else {
                        Some((v.to_string(), 80))
                    }
                })
                .unwrap_or_else(|| ("localhost".to_string(), 80))
        }
    } else {
        // No SO_ORIGINAL_DST available, use Host header.
        headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("host"))
            .and_then(|(_, v)| {
                let v = v.trim();
                if let Some((h, p)) = parse_host_port(v) {
                    Some((h.to_string(), p))
                } else {
                    Some((v.to_string(), 80))
                }
            })
            .unwrap_or_else(|| ("localhost".to_string(), 80))
    };

    let path_to_use = if path.starts_with("http://") {
        path.split("//")
            .nth(2)
            .map(|s| s.split_once('/').map(|(_, p)| format!("/{}", p)).unwrap_or_else(|| "/".to_string()))
            .unwrap_or_else(|| "/".to_string())
    } else {
        path.to_string()
    };

    if let Err(e) = handle_http(
        ctx,
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

async fn run_proxy(app_handle: AppHandle, cert_manager: Arc<CertManager>, mut shutdown_rx: tokio::sync::oneshot::Receiver<()>) -> Result<(), String> {
    let addr = format!("0.0.0.0:{}", PROXY_PORT);
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    log::info!("Proxy listening on {}", addr);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, client_addr)) => {
                        let ctx = ProxyContext {
                            app_handle: app_handle.clone(),
                            cert_manager: cert_manager.clone(),
                        };
                        tokio::spawn(handle_client(ctx, stream, client_addr));
                    }
                    Err(e) => {
                        log::error!("Accept failed: {}", e);
                    }
                }
            }
            _ = &mut shutdown_rx => {
                log::info!("Proxy shutdown signal received");
                break;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn start_proxy(
    app_handle: AppHandle,
    cert_manager: State<'_, Arc<CertManager>>,
) -> Result<String, String> {
    // Prevent starting proxy multiple times
    if PROXY_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("Proxy is already running".to_string());
    }

    let cm = cert_manager.inner().clone();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    tauri::async_runtime::spawn(async move {
        // Keep shutdown_tx alive by dropping it at the end
        let _shutdown_tx = shutdown_tx;
        if let Err(e) = run_proxy(app_handle, cm, shutdown_rx).await {
            log::error!("Proxy error: {}", e);
        }
        PROXY_RUNNING.store(false, Ordering::SeqCst);
    });

    Ok(format!("Proxy starting on port {}", PROXY_PORT))
}

#[tauri::command]
pub fn get_ca_cert_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".proxybot").join("ca.crt").to_string_lossy().to_string()
}

#[tauri::command]
pub fn get_ca_cert_pem(cert_manager: State<Arc<CertManager>>) -> String {
    cert_manager.get_ca_cert_pem()
}

/// Shared proxy state — stores network config set by get_network_info.
pub struct ProxyState {
    pub interface: std::sync::Mutex<Option<String>>,
    pub local_ip: std::sync::Mutex<Option<String>>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            interface: std::sync::Mutex::new(None),
            local_ip: std::sync::Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn get_network_info(state: State<'_, Arc<ProxyState>>) -> Result<NetworkInfo, String> {
    let info = crate::network::get_network_info()?;
    *state.interface.lock().unwrap() = Some(info.interface.clone());
    *state.local_ip.lock().unwrap() = Some(info.lan_ip.clone());
    Ok(info)
}

#[tauri::command]
pub fn setup_pf(
    app_handle: AppHandle,
    dns_state: State<'_, Arc<DnsState>>,
    proxy_state: State<'_, Arc<ProxyState>>,
) -> Result<String, String> {
    let interface = proxy_state.interface.lock().unwrap().clone()
        .ok_or_else(|| "Network info not set. Call get_network_info first.")?;
    let local_ip = proxy_state.local_ip.lock().unwrap().clone()
        .ok_or_else(|| "Network info not set. Call get_network_info first.")?;
    let result = crate::pf::setup_pf(interface, local_ip);
    if result.is_ok() {
        // Start DNS server after pf setup succeeds
        dns::start_dns_server(app_handle, dns_state.inner().clone());
    }
    result
}

#[tauri::command]
pub fn teardown_pf(dns_state: State<'_, Arc<DnsState>>) -> Result<(), String> {
    // Stop DNS server first
    dns::stop_dns_server(dns_state.inner());
    // Then tear down pf
    crate::pf::teardown_pf()
}

#[tauri::command]
pub fn get_request_detail(id: String) -> Option<InterceptedRequest> {
    REQUEST_STORE.get(&id).map(|entry| entry.value().clone())
}
