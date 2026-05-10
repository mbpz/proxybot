//! VPN tunnel server — listens for iOS device VPN connections.
//!
//! Implements the Mac side of the Atlantis-style TCP bridge:
//! - iOS NEPacketTunnelProvider captures raw IP packets via VPN API
//! - Packets are forwarded over a single TCP connection to this server
//! - The server receives packets and feeds them into the proxy pipeline
//!
//! Protocol (binary framing over TCP):
//!   [length: u32 BE][protocol: u8][src_ip: 4B][src_port: u16 BE]
//!   [dst_ip: 4B][dst_port: u16 BE][payload: N bytes]

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

/// A single packet received from the iOS VPN tunnel.
#[derive(Debug, Clone)]
pub struct TunnelPacket {
    /// The raw IP packet payload (transport-layer data).
    pub data: Vec<u8>,
    /// IP protocol number: 6 = TCP, 17 = UDP.
    pub protocol: u8,
    /// Source IPv4 address (network byte order from the captured packet).
    pub source_ip: [u8; 4],
    /// Destination IPv4 address.
    pub dest_ip: [u8; 4],
    /// Source port (from TCP/UDP header inside the captured packet).
    pub source_port: u16,
    /// Destination port.
    pub dest_port: u16,
}

/// Current status of the VPN tunnel server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelStatus {
    /// Server has not been started.
    Stopped,
    /// Server is binding to the listen address.
    Starting,
    /// Server is running and accepting connections.
    Running { addr: SocketAddr, connections: usize },
    /// Server encountered a fatal error.
    Error(String),
}

/// VPN tunnel server — listens on a TCP port for iOS PacketTunnelProvider connections.
///
/// Each iOS device opens one long-lived TCP connection.  Packets are framed
/// and sent bidirectionally over that connection.
pub struct TunnelServer {
    bind_addr: SocketAddr,
    running: Arc<AtomicBool>,
    status_tx: watch::Sender<TunnelStatus>,
}

impl TunnelServer {
    /// Create a new tunnel server that will listen on the given port.
    pub fn new(port: u16) -> Self {
        Self {
            bind_addr: SocketAddr::new(
                std::net::Ipv4Addr::UNSPECIFIED.into(),
                port,
            ),
            running: Arc::new(AtomicBool::new(false)),
            status_tx: watch::channel(TunnelStatus::Stopped).0,
        }
    }

    /// Create a new tunnel server bound to a specific address.
    pub fn with_addr(addr: SocketAddr) -> Self {
        Self {
            bind_addr: addr,
            running: Arc::new(AtomicBool::new(false)),
            status_tx: watch::channel(TunnelStatus::Stopped).0,
        }
    }

    /// Returns a receiver that watches the server status.
    pub fn status_rx(&self) -> watch::Receiver<TunnelStatus> {
        self.status_tx.subscribe()
    }

    /// Returns the current status immediately.
    pub fn current_status(&self) -> TunnelStatus {
        self.status_tx.borrow().clone()
    }

    /// Returns whether the server is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the VPN tunnel listener.
    ///
    /// Accepts connections from iOS PacketTunnelProvider instances.
    /// Each connection is handled in a spawned task.
    pub async fn start(&self) -> Result<(), String> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err("VPN tunnel server is already running".to_string());
        }

        let _ = self.status_tx.send(TunnelStatus::Starting);

        let listener = TcpListener::bind(self.bind_addr)
            .await
            .map_err(|e| format!("VPN tunnel bind failed: {}", e))?;

        let addr = listener.local_addr().map_err(|e| format!("Failed to get local addr: {}", e))?;
        log::info!("VPN tunnel server listening on {}", addr);

        let running = self.running.clone();
        let status_tx = self.status_tx.clone();

        tokio::spawn(async move {
            let mut connections: usize = 0;

            let _ = status_tx.send(TunnelStatus::Running {
                addr,
                connections,
            });

            loop {
                if !running.load(Ordering::SeqCst) {
                    log::info!("VPN tunnel server shutting down");
                    let _ = status_tx.send(TunnelStatus::Stopped);
                    break;
                }

                match listener.accept().await {
                    Ok((stream, peer)) => {
                        connections += 1;
                        log::info!(
                            "VPN tunnel connection from iOS device: {} (total: {})",
                            peer,
                            connections
                        );

                        let _ = status_tx.send(TunnelStatus::Running {
                            addr,
                            connections,
                        });

                        let _running = running.clone();
                        let _status_tx = status_tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_tunnel(stream).await {
                                log::error!("VPN tunnel error (peer {}): {}", peer, e);
                            }
                            log::info!("VPN tunnel disconnected: {}", peer);

                            // Decrement connection count on the next status update
                            // (best-effort — the status_tx will be refreshed on the next accept)
                        });
                    }
                    Err(e) => {
                        log::error!("VPN tunnel accept failed: {}", e);
                        let _ = status_tx.send(TunnelStatus::Error(e.to_string()));
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Signal the server to stop accepting new connections.
    pub fn stop(&self) {
        log::info!("Stopping VPN tunnel server...");
        self.running.store(false, Ordering::SeqCst);
        let _ = self.status_tx.send(TunnelStatus::Stopped);
    }

    /// Handle a single tunnel connection from one iOS device.
    ///
    /// Frame format (big-endian):
    ///   [length: u32][protocol: u8][src_ip: 4B][src_port: u16]
    ///   [dst_ip: 4B][dst_port: u16][payload: N bytes]
    ///
    /// length includes the 13-byte header (1 + 4 + 2 + 4 + 2) plus the payload.
    async fn handle_tunnel(mut stream: TcpStream) -> Result<(), String> {
        let mut len_buf = [0u8; 4];
        let mut buf = vec![0u8; 65536];

        loop {
            // --- Read frame length (4 bytes, big-endian) ---
            if let Err(e) = stream.read_exact(&mut len_buf).await {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    log::debug!("VPN tunnel: client closed connection");
                } else {
                    log::error!("VPN tunnel: read length failed: {}", e);
                }
                return Ok(());
            }

            let payload_len = u32::from_be_bytes(len_buf) as usize;

            // Guard against empty frames and unreasonably large lengths
            if payload_len == 0 || payload_len > 65535 {
                log::warn!("VPN tunnel: invalid frame length {}", payload_len);
                return Ok(());
            }

            // --- Read the rest of the frame ---
            let n = stream
                .read(&mut buf[..payload_len])
                .await
                .map_err(|e| format!("Read failed: {}", e))?;

            // Minimum frame size is 13 bytes (header without payload)
            if n < 13 {
                log::debug!("VPN tunnel: frame too short ({} bytes), skipping", n);
                continue;
            }

            let protocol = buf[0];
            let src_ip = [buf[1], buf[2], buf[3], buf[4]];
            let src_port = u16::from_be_bytes([buf[5], buf[6]]);
            let dst_ip = [buf[7], buf[8], buf[9], buf[10]];
            let dst_port = u16::from_be_bytes([buf[11], buf[12]]);
            let payload = buf[13..n].to_vec();

            let packet = TunnelPacket {
                data: payload,
                protocol,
                source_ip: src_ip,
                dest_ip: dst_ip,
                source_port: src_port,
                dest_port: dst_port,
            };

            log::debug!(
                "VPN packet: proto={} {}.{}.{}.{}:{} -> {}.{}.{}.{}:{} len={}",
                packet.protocol,
                packet.source_ip[0],
                packet.source_ip[1],
                packet.source_ip[2],
                packet.source_ip[3],
                packet.source_port,
                packet.dest_ip[0],
                packet.dest_ip[1],
                packet.dest_ip[2],
                packet.dest_ip[3],
                packet.dest_port,
                packet.data.len()
            );

            // TODO: Forward the packet into the proxy pipeline
            // For now the packet is logged; integration with proxy::forward_tunnel_packet
            // will be added when the full proxy pipeline supports tunnel-sourced packets.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;

    /// Build a framed packet bytes matching the wire format.
    fn make_frame(
        protocol: u8,
        src_ip: [u8; 4],
        src_port: u16,
        dst_ip: [u8; 4],
        dst_port: u16,
        payload: &[u8],
    ) -> Vec<u8> {
        let header_len: u32 = 13 + payload.len() as u32;
        let mut frame = Vec::with_capacity(4 + header_len as usize);

        frame.extend_from_slice(&header_len.to_be_bytes()); // length
        frame.push(protocol);
        frame.extend_from_slice(&src_ip);
        frame.extend_from_slice(&src_port.to_be_bytes());
        frame.extend_from_slice(&dst_ip);
        frame.extend_from_slice(&dst_port.to_be_bytes());
        frame.extend_from_slice(payload);

        frame
    }

    #[tokio::test]
    async fn test_tunnel_server_new_and_status() {
        let server = TunnelServer::with_addr("127.0.0.1:19999".parse().unwrap());
        assert!(!server.is_running());
        assert_eq!(server.current_status(), TunnelStatus::Stopped);

        // Start returns Ok and spawns the accept loop
        let result = server.start().await;
        assert!(result.is_ok());
        assert!(server.is_running());

        // Give the background task a moment
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Stop
        server.stop();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_double_start_is_error() {
        let server = TunnelServer::with_addr("127.0.0.1:19998".parse().unwrap());
        server.start().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let result = server.start().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already running"));

        server.stop();
    }

    #[tokio::test]
    async fn test_tunnel_frame_decode() {
        // Start a test server
        let server = TunnelServer::with_addr("127.0.0.1:19997".parse().unwrap());
        server.start().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Connect a client and send a valid frame
        let mut stream = TcpStream::connect("127.0.0.1:19997").await.unwrap();

        let frame = make_frame(
            6, // TCP
            [10, 0, 0, 1],
            443,
            [192, 168, 1, 100],
            8080,
            b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n",
        );

        stream.write_all(&frame).await.unwrap();
        stream.flush().await.unwrap();

        // Let the server process the frame
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Close the client — server should log the disconnect without error
        drop(stream);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        server.stop();
    }

    #[tokio::test]
    async fn test_empty_frame_handled() {
        let server = TunnelServer::with_addr("127.0.0.1:19996".parse().unwrap());
        server.start().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut stream = TcpStream::connect("127.0.0.1:19996").await.unwrap();

        // Send zero-length frame — server should log warning and return
        stream.write_all(&0u32.to_be_bytes()).await.unwrap();
        stream.flush().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        server.stop();
    }

    #[tokio::test]
    async fn test_frame_too_short_skipped() {
        let server = TunnelServer::with_addr("127.0.0.1:19995".parse().unwrap());
        server.start().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut stream = TcpStream::connect("127.0.0.1:19995").await.unwrap();

        // Frame claims 14 bytes but only 10 bytes of header (too short for full header)
        let mut frame = vec![];
        frame.extend_from_slice(&14u32.to_be_bytes()); // 1 byte payload
        frame.extend_from_slice(&[6, 10, 0, 0, 1, 0, 1, 187, 192, 168]); // only 10 bytes
        stream.write_all(&frame).await.unwrap();
        stream.flush().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        server.stop();
    }
}
