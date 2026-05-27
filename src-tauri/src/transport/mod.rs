//! Transport-layer proxy — TCP/UDP connection handling beyond HTTP/HTTPS.
//!
//! Extends ProxyBot's capture surface from HTTP/HTTPS on :80/:443 to
//! arbitrary TCP connections. Non-HTTP protocols get connection-level
//! logging with metadata (bytes, duration, SNI for TLS, protocol guess).
//!
//! # Architecture
//!
//! ```text
//! pf redirects all TCP → transport proxy (port 8089) → detect protocol
//!     ├── TLS (any port) → extract SNI, log, pass-through
//!     ├── HTTP (any port) → forward to MITM proxy (8088)
//!     └── unknown       → log metadata, pass-through
//! ```

pub mod protocol;
pub mod tcp_proxy;
pub mod types;

pub use protocol::detect_protocol;
pub use tcp_proxy::TransportProxy;
pub use types::{ConnectionMeta, DetectedProtocol, TransportConfig, TransportEvent};
