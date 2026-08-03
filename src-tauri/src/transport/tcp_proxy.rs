//! Generic TCP proxy for transport-layer traffic capture.
//!
//! Accepts connections redirected by pf, detects the protocol,
//! extracts metadata (SNI, protocol type), logs to the event channel,
//! and either passes through or forwards to the HTTP MITM proxy.
//!
//! # Protocol routing
//!
//! - TLS on any port → extract SNI, log connection, pass-through
//! - HTTP on any port → forward to MITM proxy (8088) for full decryption
//! - SSH/SMTP/IMAP → log connection metadata, pass-through
//! - Unknown → pass-through silently (or log if configured)

use crate::transport::protocol::detect_protocol;
use crate::transport::types::{
    ConnectionMeta, DetectedProtocol, TransportConfig, TransportEvent, TransportEventType,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

/// Transport proxy that handles arbitrary TCP connections.
pub struct TransportProxy {
    config: TransportConfig,
    running: Arc<AtomicBool>,
    event_tx: broadcast::Sender<TransportEvent>,
}

impl TransportProxy {
    /// Create a new transport proxy.
    pub fn new(config: TransportConfig, event_tx: broadcast::Sender<TransportEvent>) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            event_tx,
        }
    }

    /// Start the transport proxy listener.
    pub async fn start(&self) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("Transport proxy already running".to_string());
        }

        let addr = format!("0.0.0.0:{}", self.config.listen_port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("Failed to bind transport proxy: {}", e))?;

        log::info!("Transport proxy listening on {}", addr);

        let running = self.running.clone();
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                match listener.accept().await {
                    Ok((stream, src_addr)) => {
                        let config = config.clone();
                        let event_tx = event_tx.clone();
                        tokio::spawn(async move {
                            handle_connection(stream, src_addr, &config, &event_tx).await;
                        });
                    }
                    Err(e) => {
                        if running.load(Ordering::SeqCst) {
                            log::error!("Transport proxy accept error: {}", e);
                        }
                    }
                }
            }
            log::info!("Transport proxy stopped");
        });

        Ok(())
    }

    /// Stop the transport proxy.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if the transport proxy is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Handle a single TCP connection.
async fn handle_connection(
    mut stream: TcpStream,
    src_addr: std::net::SocketAddr,
    config: &TransportConfig,
    event_tx: &broadcast::Sender<TransportEvent>,
) {
    let peer_addr = stream.peer_addr().unwrap_or(src_addr);
    let start = Instant::now();
    let mut buf = vec![0u8; config.sni_buffer_size];
    let mut bytes_received: u64 = 0;

    // Read initial bytes for protocol detection
    let protocol = match tokio::time::timeout(
        std::time::Duration::from_millis(config.detect_timeout_ms),
        stream.read(&mut buf),
    )
    .await
    {
        Ok(Ok(n)) if n > 0 => {
            bytes_received = n as u64;
            Some(detect_protocol(&buf[..n]))
        }
        _ => None,
    };

    let proto = protocol
        .as_ref()
        .cloned()
        .unwrap_or(DetectedProtocol::Unknown);
    let sni = match &proto {
        DetectedProtocol::Tls { sni } => sni.clone(),
        _ => None,
    };

    // Build connection metadata
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let meta = ConnectionMeta {
        id: format!("conn-{}-{}", now_ms, peer_addr.port()),
        timestamp_ms: now_ms,
        src_addr: peer_addr.to_string(),
        dst_addr: format!("transport:{}", peer_addr.port()),
        dst_port: peer_addr.port(),
        protocol: proto.clone(),
        bytes_sent: 0,
        bytes_received,
        duration_ms: 0,
        sni: sni.clone(),
        app_name: None,
        app_icon: None,
    };

    // Classify by SNI if available
    let meta = if let Some(ref host) = sni {
        if let Some(attribution) = proxybot_core::ApplicationClassifier::from_config_files()
            .classify_request("", Some(host), None)
        {
            ConnectionMeta {
                app_name: Some(attribution.app_name),
                app_icon: attribution.app_icon,
                ..meta
            }
        } else {
            meta
        }
    } else {
        meta
    };

    // Emit connection opened event
    let _ = event_tx.send(TransportEvent {
        event_type: TransportEventType::ConnectionOpened,
        connection: meta.clone(),
    });

    // Route based on protocol
    match proto {
        DetectedProtocol::Http { .. } => {
            // Forward to MITM proxy
            forward_to_http_proxy(&buf, bytes_received, &mut stream, config).await;
        }
        _ => {
            // Pass-through: just forward data bidirectionally
            pass_through(&buf, bytes_received, &mut stream, &peer_addr).await;
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    // Emit connection closed event
    let _ = event_tx.send(TransportEvent {
        event_type: TransportEventType::ConnectionClosed,
        connection: ConnectionMeta {
            duration_ms,
            ..meta
        },
    });
}

/// Forward initial data + remaining stream to the HTTP MITM proxy.
async fn forward_to_http_proxy(
    initial_data: &[u8],
    initial_len: u64,
    client_stream: &mut TcpStream,
    config: &TransportConfig,
) {
    let proxy_addr = format!("127.0.0.1:{}", config.http_proxy_port);
    match TcpStream::connect(&proxy_addr).await {
        Ok(mut proxy_stream) => {
            // Send initial data to proxy
            if let Err(e) = proxy_stream
                .write_all(&initial_data[..initial_len as usize])
                .await
            {
                log::debug!("Failed to forward initial data to HTTP proxy: {}", e);
                return;
            }
            // Bidirectional copy
            let (mut client_read, mut client_write) = client_stream.split();
            let (mut proxy_read, mut proxy_write) = proxy_stream.split();
            let _ = tokio::join!(
                tokio::io::copy(&mut client_read, &mut proxy_write),
                tokio::io::copy(&mut proxy_read, &mut client_write),
            );
        }
        Err(e) => {
            log::debug!("Failed to connect to HTTP proxy: {}", e);
        }
    }
}

/// Pass-through: bidirectionally forward traffic without inspection.
async fn pass_through(
    initial_data: &[u8],
    initial_len: u64,
    client_stream: &mut TcpStream,
    _peer_addr: &std::net::SocketAddr,
) {
    // For pass-through, we need to connect to the original destination.
    // Since pf redirects the connection, the original destination is lost.
    // Use SO_ORIGINAL_DST on macOS to recover the original destination address.
    match get_original_dst(client_stream) {
        Some(orig_addr) => {
            match TcpStream::connect(orig_addr).await {
                Ok(mut upstream) => {
                    // Send the initial data that we already read
                    if let Err(e) = upstream
                        .write_all(&initial_data[..initial_len as usize])
                        .await
                    {
                        log::debug!("Failed to forward initial data: {}", e);
                        return;
                    }
                    let (mut client_read, mut client_write) = client_stream.split();
                    let (mut upstream_read, mut upstream_write) = upstream.split();
                    let _ = tokio::join!(
                        tokio::io::copy(&mut client_read, &mut upstream_write),
                        tokio::io::copy(&mut upstream_read, &mut client_write),
                    );
                }
                Err(e) => {
                    log::debug!("Failed to connect to original destination: {}", e);
                }
            }
        }
        None => {
            log::debug!("Cannot pass-through: original destination unknown");
        }
    }
}

/// Get the original destination address of a pf-redirected TCP connection.
///
/// On macOS, pf rdr rules preserve the original destination in the socket
/// state. We can recover it via getsockopt SO_ORIGINAL_DST.
#[cfg(target_os = "macos")]
fn get_original_dst(stream: &TcpStream) -> Option<std::net::SocketAddr> {
    use std::os::fd::AsRawFd;

    let fd = stream.as_raw_fd();
    // macOS uses a special sockaddr structure for original destination
    #[repr(C)]
    struct SockAddrStorage {
        len: u8,
        family: u8,
        data: [u8; 30],
    }

    let mut storage = SockAddrStorage {
        len: 32,
        family: 0,
        data: [0; 30],
    };

    // SO_ORIGINAL_DST is an internal macOS option
    // The option value is documented in xnu source
    let so_original_dst: libc::c_int = 0x1018; // SO_ORIGINAL_DST on macOS

    unsafe {
        let mut len: libc::socklen_t = std::mem::size_of::<SockAddrStorage>() as libc::socklen_t;
        let ret = libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            so_original_dst,
            &mut storage as *mut _ as *mut libc::c_void,
            &mut len,
        );
        if ret != 0 {
            return None;
        }
    }

    // Parse the sockaddr_in from the storage
    if storage.family == 2 {
        // AF_INET
        let port = u16::from_be_bytes([storage.data[0], storage.data[1]]);
        let ip = std::net::Ipv4Addr::new(
            storage.data[2],
            storage.data[3],
            storage.data[4],
            storage.data[5],
        );
        Some(std::net::SocketAddr::V4(std::net::SocketAddrV4::new(
            ip, port,
        )))
    } else if storage.family == 30 {
        // AF_INET6
        let port = u16::from_be_bytes([storage.data[0], storage.data[1]]);
        let mut octets = [0u8; 16];
        octets.copy_from_slice(&storage.data[2..18]);
        let ip = std::net::Ipv6Addr::from(octets);
        Some(std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
            ip, port, 0, 0,
        )))
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
fn get_original_dst(_stream: &TcpStream) -> Option<std::net::SocketAddr> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_proxy_lifecycle() {
        let config = TransportConfig::default();
        let (tx, _rx) = broadcast::channel(16);
        let proxy = TransportProxy::new(config, tx);
        assert!(!proxy.is_running());
    }

    #[test]
    fn test_protocol_detection_integration() {
        // Test that protocol detection works with real data
        let http_data = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let proto = detect_protocol(http_data);
        assert!(matches!(proto, DetectedProtocol::Http { .. }));

        let tls_data = [0x16, 0x03, 0x01, 0x00, 0x05, 0x01, 0x00, 0x00, 0x01, 0x00];
        let proto = detect_protocol(&tls_data);
        assert!(matches!(proto, DetectedProtocol::Tls { .. }));

        let ssh_data = b"SSH-2.0-test\r\n";
        let proto = detect_protocol(ssh_data);
        assert_eq!(proto, DetectedProtocol::Ssh);
    }
}
