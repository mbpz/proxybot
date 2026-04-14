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

const PROXY_PORT: u16 = 8080;

static PROXY_RUNNING: AtomicBool = AtomicBool::new(false);

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
}

struct ProxyContext {
    app_handle: AppHandle,
    #[allow(dead_code)]
    cert_manager: Arc<CertManager>,
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

/// Handle HTTPS CONNECT tunnel with TLS termination on both sides.
async fn handle_https_connect(
    ctx: ProxyContext,
    mut client_stream: TcpStream,
    client_addr: SocketAddr,
    target_host: String,
    target_port: u16,
) {
    let target_addr = format!("{}:{}", target_host, target_port);
    log::info!("HTTPS CONNECT tunnel to {} from {}", target_addr, client_addr);

    let start = std::time::Instant::now();

    // Send HTTP 200 Connection Established to browser
    if let Err(e) = client_stream
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await
    {
        log::error!("Failed to send 200 response to browser {}: {}", client_addr, e);
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

    // Accept TLS from browser
    let client_tls_stream = match tls_acceptor.accept(client_stream).await {
        Ok(stream) => stream,
        Err(e) => {
            log::error!("TLS accept failed for browser {}: {}", client_addr, e);
            return;
        }
    };

    log::debug!("TLS handshake completed with browser {}", client_addr);

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
    let upstream_tls_stream = match connector.connect(server_name, upstream_tcp).await {
        Ok(stream) => stream,
        Err(e) => {
            log::error!("TLS connect to upstream {} failed: {}", upstream_addr, e);
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

    let app_info = app_rules::classify_host(&target_host);
    let (app_name, app_icon) = app_info
        .map(|(n, i)| (Some(n.to_string()), Some(i.to_string())))
        .unwrap_or((None, None));

    let req = InterceptedRequest {
        id: request_id,
        timestamp: timestamp_now(),
        method,
        host: target_host.clone(),
        path,
        status,
        latency_ms: Some(latency),
        scheme: "https".to_string(),
        app_name,
        app_icon,
    };

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

    let app_info = app_rules::classify_host(host);
    let (app_name, app_icon) = app_info
        .map(|(n, i)| (Some(n.to_string()), Some(i.to_string())))
        .unwrap_or((None, None));

    let req = InterceptedRequest {
        id: request_id,
        timestamp: timestamp_now(),
        method: method.to_string(),
        host: host.to_string(),
        path: path.to_string(),
        status,
        latency_ms: Some(latency),
        scheme: if port == 443 { "https" } else { "http" }.to_string(),
        app_name,
        app_icon,
    };

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
