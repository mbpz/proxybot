//! Shared types for transport-layer proxy.

use serde::Serialize;

/// Detected protocol from initial connection bytes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum DetectedProtocol {
    /// TLS handshake detected (any port); contains SNI if extracted
    Tls { sni: Option<String> },
    /// HTTP request detected (method + path visible)
    Http { method: String, path: String },
    /// SSH handshake detected
    Ssh,
    /// SMTP greeting detected
    Smtp,
    /// IMAP greeting detected
    Imap,
    /// Unknown protocol
    Unknown,
}

impl DetectedProtocol {
    pub fn name(&self) -> &str {
        match self {
            DetectedProtocol::Tls { .. } => "TLS",
            DetectedProtocol::Http { .. } => "HTTP",
            DetectedProtocol::Ssh => "SSH",
            DetectedProtocol::Smtp => "SMTP",
            DetectedProtocol::Imap => "IMAP",
            DetectedProtocol::Unknown => "Unknown",
        }
    }
}

/// Metadata about a generic TCP connection.
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionMeta {
    pub id: String,
    pub timestamp_ms: u64,
    pub src_addr: String,
    pub dst_addr: String,
    pub dst_port: u16,
    pub protocol: DetectedProtocol,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub duration_ms: u64,
    pub sni: Option<String>,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
}

/// Event emitted to the traffic pipeline.
#[derive(Debug, Clone, Serialize)]
pub struct TransportEvent {
    pub event_type: TransportEventType,
    pub connection: ConnectionMeta,
}

#[derive(Debug, Clone, Serialize)]
pub enum TransportEventType {
    ConnectionOpened,
    ConnectionClosed,
    DataTransferred { bytes: u64 },
}

/// Configuration for the transport proxy.
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Port the transport proxy listens on (pf redirects to here)
    pub listen_port: u16,
    /// Port the MITM HTTP proxy listens on (HTTP connections forwarded here)
    pub http_proxy_port: u16,
    /// SNI extraction buffer size (initial bytes to read)
    pub sni_buffer_size: usize,
    /// Connection timeout for protocol detection
    pub detect_timeout_ms: u64,
    /// Max concurrent connections
    pub max_connections: usize,
    /// Whether to log unknown protocols (noisy)
    pub log_unknown: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            listen_port: 8089,
            http_proxy_port: 8088,
            sni_buffer_size: 4096,
            detect_timeout_ms: 5000,
            max_connections: 1024,
            log_unknown: false, // Default: only log known protocols
        }
    }
}
