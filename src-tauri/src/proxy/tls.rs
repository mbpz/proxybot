//! TLS client/server configuration, SNI extraction, and original-destination
//! recovery for transparent proxy mode.

use crate::cert::CertManager;
use rustls::client::danger as rustls_danger;
use rustls::{pki_types::ServerName, ClientConfig, DigitallySignedStruct, SignatureScheme};
use std::fs::OpenOptions;
use std::net::{IpAddr, SocketAddr};
use std::os::fd::AsRawFd;

/// Extract SNI (Server Name Indication) from TLS ClientHello data.
/// Returns the hostname if SNI extension is found, None otherwise.
pub(super) fn extract_sni_from_client_hello(data: &[u8]) -> Option<String> {
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
pub(super) fn get_original_dst(
    peer_addr: SocketAddr,
    local_addr: SocketAddr,
) -> Option<SocketAddr> {
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
        af: 2,        // AF_INET
        proto: 6,     // IPPROTO_TCP
        direction: 1, // PF_IN (rdr redirects inbound traffic)
        pad: [0u8; 5],
    };

    pack_ipv4(&peer_addr.ip(), &mut nl.saddr);
    pack_ipv4(&local_addr.ip(), &mut nl.daddr);

    // Issue the ioctl
    let ret = unsafe {
        libc::ioctl(
            fd.as_raw_fd(),
            DIOCNATLOOK as libc::c_ulong,
            &mut nl as *mut _ as *mut libc::c_void,
        )
    };

    if ret != 0 {
        let err = std::io::Error::last_os_error();
        log::warn!(
            "DIOCNATLOOK failed for {}→{}: {}",
            peer_addr,
            local_addr,
            err
        );
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
pub(super) fn get_original_dst_addr(socket: &tokio::net::TcpStream) -> Option<SocketAddr> {
    let peer_addr = socket.peer_addr().ok()?;
    let local_addr = socket.local_addr().ok()?;
    get_original_dst(peer_addr, local_addr)
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
pub(super) fn build_client_config(_cert_manager: &CertManager) -> Result<ClientConfig, String> {
    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerification))
        .with_no_client_auth();

    Ok(config)
}

use std::sync::Arc;
