//! Protocol detection from initial TCP stream bytes.
//!
//! Inspects the first few bytes of a connection to identify the protocol.
//! TLS ClientHello parsing extracts SNI from the server_name extension.

use crate::transport::types::DetectedProtocol;

/// Buffer size for protocol detection (enough for TLS ClientHello + SNI).
const DETECT_BUF_SIZE: usize = 4096;

/// Detect protocol from initial stream bytes.
///
/// Returns the detected protocol. For TLS connections, SNI is extracted
/// from the ClientHello server_name extension if present.
pub fn detect_protocol(data: &[u8]) -> DetectedProtocol {
    if data.len() < 2 {
        return DetectedProtocol::Unknown;
    }

    // ─── TLS Handshake ──────────────────────────────────────────────────
    // TLS record: [content_type: u8(22)][version: u16][length: u16][handshake...]
    if data[0] == 0x16 {
        if let Some(sni) = extract_tls_sni(data) {
            return DetectedProtocol::Tls { sni: Some(sni) };
        }
        return DetectedProtocol::Tls { sni: None };
    }

    // ─── SSH ────────────────────────────────────────────────────────────
    // SSH banner: "SSH-2.0-..."
    if data.len() >= 4 && &data[0..4] == b"SSH-" {
        return DetectedProtocol::Ssh;
    }

    // ─── SMTP ───────────────────────────────────────────────────────────
    // SMTP greeting: "220 ..."
    if data.len() >= 4 && &data[0..4] == b"220 " {
        return DetectedProtocol::Smtp;
    }

    // ─── IMAP ───────────────────────────────────────────────────────────
    // IMAP greeting: "* OK ..."
    if data.len() >= 4 && &data[0..4] == b"* OK" {
        return DetectedProtocol::Imap;
    }

    // ─── HTTP ───────────────────────────────────────────────────────────
    // HTTP request line: "GET / HTTP/1.1", "POST /api HTTP/1.1", etc.
    if let Some(proto) = detect_http(data) {
        return proto;
    }

    DetectedProtocol::Unknown
}

/// Detect HTTP request from raw bytes.
fn detect_http(data: &[u8]) -> Option<DetectedProtocol> {
    let methods: &[&[u8]] = &[b"GET ", b"POST ", b"PUT ", b"DELETE ", b"HEAD ",
                    b"PATCH ", b"OPTIONS ", b"CONNECT "];

    for method_bytes in methods.iter() {
        if data.len() >= method_bytes.len() && &data[0..method_bytes.len()] == *method_bytes {
            let method = std::str::from_utf8(&method_bytes[0..method_bytes.len()-1]).unwrap_or("UNKNOWN");
            // Try to find the path (between method and HTTP version)
            let rest = &data[method_bytes.len()..];
            if let Some(path_end) = rest.iter().position(|&b| b == b' ') {
                let path = std::str::from_utf8(&rest[0..path_end]).unwrap_or("/");
                return Some(DetectedProtocol::Http {
                    method: method.to_string(),
                    path: path.to_string(),
                });
            }
            return Some(DetectedProtocol::Http {
                method: method.to_string(),
                path: "/".to_string(),
            });
        }
    }
    None
}

/// Extract SNI from TLS ClientHello message.
///
/// TLS 1.2/1.3 ClientHello structure:
/// ```
/// [content_type: 0x16] [version: u16] [length: u16]
///   [handshake_type: 0x01] [handshake_length: u24]
///     [client_version: u16] [random: 32]
///     [session_id_length: u8] [session_id: ...]
///     [cipher_suites_length: u16] [cipher_suites: ...]
///     [compression_length: u8] [compression: ...]
///     [extensions_length: u16]
///       [extension_type: u16] [extension_length: u16] [extension_data: ...]
///         ...
///         [0x0000 server_name]
///           [server_name_list_length: u16]
///             [name_type: 0x00] [name_length: u16] [name: ...]
/// ```
fn extract_tls_sni(data: &[u8]) -> Option<String> {
    // Minimum: content_type(1) + version(2) + length(2) + handshake_type(1) +
    // handshake_length(3) + client_version(2) + random(32) + session_id_length(1)
    let min_len = 1 + 2 + 2 + 1 + 3 + 2 + 32 + 1;
    if data.len() < min_len {
        return None;
    }

    // Verify content_type = 0x16 (Handshake)
    if data[0] != 0x16 {
        return None;
    }

    // Verify handshake_type = 0x01 (ClientHello)
    // data[5] is where handshake_type starts (after 5-byte record header)
    if data.len() <= 5 || data[5] != 0x01 {
        return None;
    }

    // Skip to session_id to find extensions
    // record_header(5) + handshake_type(1) + handshake_length(3) + client_version(2) + random(32)
    let session_id_offset = 5 + 1 + 3 + 2 + 32;
    if data.len() <= session_id_offset {
        return None;
    }

    let session_id_len = data[session_id_offset] as usize;
    let cipher_offset = session_id_offset + 1 + session_id_len;
    if data.len() <= cipher_offset + 2 {
        return None;
    }

    let cipher_len = u16::from_be_bytes([data[cipher_offset], data[cipher_offset + 1]]) as usize;
    let comp_offset = cipher_offset + 2 + cipher_len;
    if data.len() <= comp_offset {
        return None;
    }

    let comp_len = data[comp_offset] as usize;
    let ext_offset = comp_offset + 1 + comp_len;
    if data.len() <= ext_offset + 2 {
        return None;
    }

    let ext_len = u16::from_be_bytes([data[ext_offset], data[ext_offset + 1]]) as usize;
    let mut pos = ext_offset + 2;
    let end = pos + ext_len;

    while pos + 4 <= end && pos + 4 <= data.len() {
        let ext_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
        let ext_data_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        // server_name extension (0x0000)
        if ext_type == 0x0000 {
            if pos + 5 > data.len() {
                return None;
            }
            // server_name_list_length: u16
            let list_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2;
            if pos + 3 > data.len() {
                return None;
            }
            // name_type: u8 (0x00 = host_name)
            if data[pos] != 0x00 {
                return None;
            }
            let name_len = u16::from_be_bytes([data[pos + 1], data[pos + 2]]) as usize;
            pos += 3;
            if pos + name_len > data.len() {
                return None;
            }
            let sni = std::str::from_utf8(&data[pos..pos + name_len]).ok()?;
            return Some(sni.to_string());
        }

        pos += ext_data_len;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_http_get() {
        let data = b"GET /api/v1/users HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let proto = detect_protocol(data);
        match proto {
            DetectedProtocol::Http { method, path } => {
                assert_eq!(method, "GET");
                assert_eq!(path, "/api/v1/users");
            }
            _ => panic!("Expected HTTP, got {:?}", proto),
        }
    }

    #[test]
    fn test_detect_http_post() {
        let data = b"POST /login HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let proto = detect_protocol(data);
        match proto {
            DetectedProtocol::Http { method, path } => {
                assert_eq!(method, "POST");
                assert_eq!(path, "/login");
            }
            _ => panic!("Expected HTTP, got {:?}", proto),
        }
    }

    #[test]
    fn test_detect_ssh() {
        let data = b"SSH-2.0-OpenSSH_8.9\r\n";
        assert_eq!(detect_protocol(data), DetectedProtocol::Ssh);
    }

    #[test]
    fn test_detect_smtp() {
        let data = b"220 smtp.example.com ESMTP\r\n";
        assert_eq!(detect_protocol(data), DetectedProtocol::Smtp);
    }

    #[test]
    fn test_detect_imap() {
        let data = b"* OK IMAP4rev1 server ready\r\n";
        assert_eq!(detect_protocol(data), DetectedProtocol::Imap);
    }

    #[test]
    fn test_detect_tls_without_sni() {
        // Minimal TLS record header + handshake (not enough for SNI)
        let mut data = vec![0x16, 0x03, 0x01]; // content_type, version
        data.extend_from_slice(&[0x00, 0x05]); // length = 5
        data.extend_from_slice(&[0x01]); // handshake_type = ClientHello
        data.extend_from_slice(&[0x00, 0x00, 0x01]); // handshake_length
        data.push(0x00); // 1 byte of handshake data
        let proto = detect_protocol(&data);
        assert!(matches!(proto, DetectedProtocol::Tls { sni: None }));
    }

    #[test]
    fn test_detect_unknown() {
        let data = b"\x00\x00\x00\x00\x00\x00";
        assert_eq!(detect_protocol(data), DetectedProtocol::Unknown);
    }

    #[test]
    fn test_tls_sni_extraction() {
        // This test validates the TLS ClientHello parser with a synthetic
        // ClientHello containing a server_name extension.
        // Build a minimal but valid ClientHello:
        // - TLS record: content_type(0x16) version(0x0303) length
        // - Handshake: type(0x01) length
        // - ClientHello: version(0x0303) random(32 bytes of zero)
        // - SessionID: length(0x00)
        // - CipherSuites: length(0x0002), suite(0x1301=TLS_AES_128_GCM_SHA256)
        // - Compression: length(0x01), method(0x00)
        // - Extensions: server_name
        //   - ext_type(0x0000) ext_length
        //   - server_name_list_length
        //   - name_type(0x00) name_length name

        let sni = "example.com";
        let sni_bytes = sni.as_bytes();
        let sni_len = sni_bytes.len();

        // server_name extension
        // name_type(1) + name_length(2) + name(sni_len)
        let name_data_len = 1 + 2 + sni_len;
        // server_name_list_length(2) + name_data
        let ext_data_len = 2 + name_data_len;
        // ext_type(2) + ext_length(2) + ext_data
        let ext_total_len: u16 = (2 + 2 + ext_data_len) as u16;

        // Extensions: just the server_name extension
        let extensions_len: u16 = ext_total_len;

        // ClientHello body:
        // client_version(2) + random(32) + session_id_length(1) +
        // cipher_suites_length(2) + cipher_suites(2) +
        // compression_length(1) + compression(1) +
        // extensions_length(2) + extensions(ext_total_len)
        let client_hello_body_len: usize = 2 + 32 + 1 + 2 + 2 + 1 + 1 + 2 + ext_total_len as usize;

        // Handshake: type(1) + length(3) = 4 bytes + body
        let handshake_len: u32 = client_hello_body_len as u32;
        // handshake_length is 3 bytes (u24)
        let handshake_length_bytes = [
            ((handshake_len >> 16) & 0xFF) as u8,
            ((handshake_len >> 8) & 0xFF) as u8,
            (handshake_len & 0xFF) as u8,
        ];

        // TLS record: content_type(1) + version(2) + length(2) = 5 bytes
        let record_len: u16 = (4 + client_hello_body_len) as u16; // handshake header + body

        let mut data = Vec::new();
        // Record header
        data.push(0x16); // content_type = Handshake
        data.extend_from_slice(&[0x03, 0x03]); // TLS 1.2 version
        data.extend_from_slice(&record_len.to_be_bytes());
        // Handshake
        data.push(0x01); // ClientHello
        data.extend_from_slice(&handshake_length_bytes);
        // ClientHello body
        data.extend_from_slice(&[0x03, 0x03]); // client_version
        data.extend_from_slice(&[0u8; 32]); // random
        data.push(0x00); // session_id_length = 0
        // Cipher suites
        data.extend_from_slice(&[0x00, 0x02]); // length = 2
        data.extend_from_slice(&[0x13, 0x01]); // TLS_AES_128_GCM_SHA256
        // Compression
        data.push(0x01); // length = 1
        data.push(0x00); // null compression
        // Extensions
        data.extend_from_slice(&extensions_len.to_be_bytes());
        // server_name extension
        data.extend_from_slice(&[0x00, 0x00]); // ext_type = server_name
        data.extend_from_slice(&(ext_data_len as u16).to_be_bytes()); // ext_length
        // server_name_list
        data.extend_from_slice(&(name_data_len as u16).to_be_bytes()); // list_length
        data.push(0x00); // name_type = host_name
        data.extend_from_slice(&(sni_len as u16).to_be_bytes()); // name_length
        data.extend_from_slice(sni_bytes); // name

        let sni_result = extract_tls_sni(&data);
        assert_eq!(sni_result, Some("example.com".to_string()));
    }

    #[test]
    fn test_tls_sni_extraction_empty() {
        let sni_result = extract_tls_sni(b"\x16\x03\x03");
        assert!(sni_result.is_none());
    }
}
