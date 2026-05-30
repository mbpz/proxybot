//! HTTP/3 and QUIC transport prototype.
//!
//! HTTP/3 runs on QUIC (Quick UDP Internet Connections) instead of TCP.
//! QUIC uses UDP with built-in TLS 1.3 encryption — no separate TLS handshake.
//!
//! ## Architecture
//!
//! For MITM interception of HTTP/3 traffic:
//!
//! 1. DNS hijack: Redirect the QUIC-capable domain's DNS response to our IP
//! 2. UDP redirect: pf / WFP redirects UDP 443 → local QUIC proxy port
//! 3. QUIC handshake: Accept QUIC connection, present our CA-signed cert
//! 4. HTTP/3 stream: Decode HTTP/3 frames from QUIC streams
//! 5. Forward: Open new QUIC connection to origin, relay modified requests
//!
//! ## Dependencies
//!
//! The `quinn` crate (optional, behind `http3` feature) provides QUIC
//! transport. Enable with:
//! ```toml
//! [features]
//! http3 = ["quinn"]
//! ```
//!
//! ## Current Status
//!
//! Prototype stage — no open-source proxy currently supports HTTP/3 MITM.
//! Quinn provides the QUIC transport layer; HTTP/3 frame parsing would
//! require an additional crate like `h3` or manual frame decoding.
//!
//! ## Key Challenges
//!
//! - **Alt-Svc**: Servers advertise HTTP/3 via Alt-Svc headers in HTTP/2
//!   responses. The proxy must strip these to prevent client upgrades.
//! - **0-RTT**: QUIC supports 0-RTT resumption — cached connections skip
//!   the handshake, bypassing MITM. Proxy must disable 0-RTT.
//! - **Connection Migration**: QUIC connections survive IP changes via
//!   connection IDs. MITM state must track connection IDs.
//!
//! See `docs/sdd/http3-quic-research.md` for the full research document.

/// Placeholder for QUIC connection interception.
pub struct QuicInterceptor;

impl QuicInterceptor {
    pub fn new() -> Self { Self }

    /// Check if a UDP packet on port 443 looks like a QUIC initial packet.
    /// QUIC initial packets have a specific format: long header with version.
    pub fn is_quic_initial(data: &[u8]) -> bool {
        if data.len() < 6 {
            return false;
        }
        // QUIC long header: top bit set, followed by version (4 bytes)
        let header_byte = data[0];
        let is_long_header = (header_byte & 0x80) != 0;
        if !is_long_header {
            return false;
        }
        // Check if the version field is a known QUIC version
        let version = u32::from_be_bytes([data[1], data[2], data[3], data[4]]);
        // QUIC v1 = 0x00000001, draft versions use 0xff000000 + draft number
        version == 1 || (version & 0xff00_0000) == 0xff00_0000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quic_initial_detection() {
        // QUIC v1 initial packet: long header + version 1
        let packet = [0xc0u8, 0x00, 0x00, 0x00, 0x01, 0x00];
        assert!(QuicInterceptor::is_quic_initial(&packet));
    }

    #[test]
    fn test_short_header_not_quic_initial() {
        // Short header (top bit 0)
        let packet = [0x40u8, 0x00, 0x00, 0x00, 0x01, 0x00];
        assert!(!QuicInterceptor::is_quic_initial(&packet));
    }

    #[test]
    fn test_empty_packet() {
        assert!(!QuicInterceptor::is_quic_initial(&[]));
        assert!(!QuicInterceptor::is_quic_initial(&[0xc0]));
    }
}
