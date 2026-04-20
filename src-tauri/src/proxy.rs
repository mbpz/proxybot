use crate::app_rules;
use crate::cert::{CaMetadata, CertManager};
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
    pub query_params: Option<String>,
    pub status: Option<u16>,
    pub latency_ms: Option<u64>,
    pub scheme: String,
    pub req_headers: Vec<(String, String)>,
    pub req_body: Option<String>,
    pub resp_headers: Vec<(String, String)>,
    pub resp_body: Option<String>,
    pub resp_size: Option<usize>,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
    pub is_websocket: bool,
    pub ws_frames: Option<Vec<WsFrame>>,
}

#[derive(Clone, serde::Serialize)]
pub struct WsFrame {
    pub direction: String,
    pub timestamp: String,
    pub payload: String,
    pub size: usize,
}

struct ProxyContext {
    app_handle: AppHandle,
    #[allow(dead_code)]
    cert_manager: Arc<CertManager>,
    dns_state: Arc<DnsState>,
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

/// Extract SNI (Server Name Indication) from TLS ClientHello data.
/// Returns the hostname if SNI extension is found, None otherwise.
fn extract_sni_from_client_hello(data: &[u8]) -> Option<String> {
    // TLS record header: content_type (1) + version (2) + length (2)
    // ClientHello starts after the record header
    if data.len() < 5 {
        return None;
    }

    // Verify this is a TLS handshake (content_type = 0x16)
    if data[0] != 0x16 {
        return None;
    }

    let mut pos = 5; // Skip TLS record header

    // ClientHello format:
    // - handshake_type (1) = 0x01 for ClientHello
    // - length (3)
    // - version (2)
    // - random (32)
    // - session_id_length (1)
    // - cipher_suites_length (2)
    // - compression_methods_length (1)
    // - extensions_length (2)
    // - extensions...

    if pos + 4 > data.len() {
        return None;
    }

    // Verify handshake type is ClientHello (0x01)
    if data[pos] != 0x01 {
        return None;
    }
    pos += 4; // skip handshake type (1) + length (3)

    // Skip client version (2) + random (32) = 34 bytes
    if pos + 34 > data.len() {
        return None;
    }
    pos += 34;

    // Skip session_id_length (1) + session_id
    if pos + 1 > data.len() {
        return None;
    }
    let session_id_len = data[pos] as usize;
    pos += 1 + session_id_len;

    // Skip cipher_suites_length (2) + cipher_suites
    if pos + 2 > data.len() {
        return None;
    }
    let cipher_suites_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2 + cipher_suites_len;

    // Skip compression_methods_length (1) + compression_methods
    if pos + 1 > data.len() {
        return None;
    }
    let compression_len = data[pos] as usize;
    pos += 1 + compression_len;

    // Skip extensions_length (2)
    if pos + 2 > data.len() {
        return None;
    }
    let extensions_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;

    // Now parse extensions
    let extensions_end = (pos + extensions_len).min(data.len());
    while pos + 4 < extensions_end {
        let ext_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let ext_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        // SNI extension type is 0x0000
        if ext_type == 0x0000 {
            // SNI format: list of (type, length, value) where type=0 means hostname
            if pos + 2 > extensions_end {
                return None;
            }
            let sni_list_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2;

            if pos + sni_list_len > extensions_end {
                return None;
            }

            // Parse hostname from SNI list
            let sni_end = pos + sni_list_len;
            while pos + 3 < sni_end {
                let name_type = data[pos];
                let name_len = u16::from_be_bytes([data[pos + 1], data[pos + 2]]) as usize;
                pos += 3;

                if pos + name_len > sni_end {
                    return None;
                }

                if name_type == 0 {
                    // hostname (DNS)
                    let hostname = String::from_utf8_lossy(&data[pos..pos + name_len]).to_string();
                    return Some(hostname);
                }

                pos += name_len;
            }

            return None;
        }

        // Skip this extension
        if pos + ext_len > extensions_end {
            break;
        }
        pos += ext_len;
    }

    None
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
    handle_https_connect(ctx, client_stream, client_addr, effective_host, target_port).await;
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

/// Parse HTTP request line, headers, and body from buffered data.
fn parse_http_request(data: &[u8]) -> Option<(String, String, String, Vec<(String, String)>, Vec<u8>)> {
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
fn parse_http_response(data: &[u8]) -> Option<(u16, Vec<(String, String)>, Vec<u8>)> {
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

/// Extract query parameters from URL path.
fn extract_query_params(path: &str) -> Option<String> {
    path.split_once('?').map(|(_, query)| query.to_string())
}

/// Try to parse body as UTF-8 string, fall back to hex representation.
fn body_to_string(body: &[u8]) -> Option<String> {
    String::from_utf8(body.to_vec()).ok()
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
    let (method, path, _, req_headers, req_body) = parse_http_request(&request_data)
        .unwrap_or_else(|| ("CONNECT".to_string(), "/".to_string(), "1.1".to_string(), Vec::new(), Vec::new()));

    let (status, resp_headers, resp_body) = parse_http_response(&response_data)
        .unwrap_or((0u16, Vec::new(), Vec::new()));

    let request_id = generate_request_id();
    let query_params = extract_query_params(&path);
    let resp_size = response_data.len();
    let req_body_str = body_to_string(&req_body);
    let resp_body_str = body_to_string(&resp_body);

    // Classify by direct domain match first, then fall back to DNS correlation
    let app_info = app_rules::classify_host(&target_host)
        .or_else(|| {
            let request_ts_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            ctx.dns_state.correlate_app(&target_host, request_ts_ms)
        });
    let (app_name, app_icon) = app_info
        .map(|(n, i)| (Some(n), Some(i)))
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
        is_websocket: false,
        ws_frames: None,
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

    let (status, resp_headers, resp_body) = parse_http_response(&response_buf)
        .unwrap_or((0u16, Vec::new(), Vec::new()));
    let resp_size = response_buf.len();
    let query_params = extract_query_params(path);
    let req_body_str = body_to_string(body);
    let resp_body_str = body_to_string(&resp_body);

    // Classify by direct domain match first, then fall back to DNS correlation
    let app_info = app_rules::classify_host(host)
        .or_else(|| {
            let request_ts_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            ctx.dns_state.correlate_app(host, request_ts_ms)
        });
    let (app_name, app_icon) = app_info
        .map(|(n, i)| (Some(n), Some(i)))
        .unwrap_or((None, None));

    let req = InterceptedRequest {
        id: request_id,
        timestamp: timestamp_now(),
        method: method.to_string(),
        host: host.to_string(),
        path: path.to_string(),
        query_params,
        status: Some(status),
        latency_ms: Some(latency),
        scheme: if port == 443 { "https" } else { "http" }.to_string(),
        req_headers: headers.to_vec(),
        req_body: req_body_str,
        resp_headers,
        resp_body: resp_body_str,
        resp_size: Some(resp_size),
        app_name,
        app_icon,
        is_websocket: false,
        ws_frames: None,
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
            handle_transparent_https(ctx, client_stream, client_addr, original_dst, data.to_vec()).await;
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

async fn run_proxy(app_handle: AppHandle, cert_manager: Arc<CertManager>, dns_state: Arc<DnsState>, mut shutdown_rx: tokio::sync::oneshot::Receiver<()>) -> Result<(), String> {
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
                            dns_state: dns_state.clone(),
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
    dns_state: State<'_, Arc<DnsState>>,
) -> Result<String, String> {
    // Prevent starting proxy multiple times
    if PROXY_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("Proxy is already running".to_string());
    }

    let cm = cert_manager.inner().clone();
    let ds = dns_state.inner().clone();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    tauri::async_runtime::spawn(async move {
        // Keep shutdown_tx alive by dropping it at the end
        let _shutdown_tx = shutdown_tx;
        if let Err(e) = run_proxy(app_handle, cm, ds, shutdown_rx).await {
            log::error!("Proxy error: {}", e);
        }
        PROXY_RUNNING.store(false, Ordering::SeqCst);
    });

    Ok(format!("Proxy starting on port {}", PROXY_PORT))
}

#[tauri::command]
pub fn get_ca_cert_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".proxybot").join("ca.pem").to_string_lossy().to_string()
}

#[tauri::command]
pub fn get_ca_cert_pem(cert_manager: State<Arc<CertManager>>) -> String {
    cert_manager.get_ca_cert_pem()
}

#[tauri::command]
pub fn regenerate_ca(cert_manager: State<Arc<CertManager>>) -> Result<(), String> {
    cert_manager.regenerate_ca()
}

#[tauri::command]
pub fn get_ca_metadata(cert_manager: State<Arc<CertManager>>) -> Option<CaMetadata> {
    cert_manager.get_ca_metadata()
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
pub fn is_pf_enabled() -> bool {
    crate::pf::is_pf_enabled()
}
