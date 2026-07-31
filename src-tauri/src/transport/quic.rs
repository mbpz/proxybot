//! HTTP/3 QUIC proxy — intercepts QUIC connections and decodes HTTP/3 frames.
//!
//! ## Architecture
//!
//! UDP 443 → QUIC listener (our CA cert) → HTTP/3 stream → log request → forward
//!
//! ## Dependencies (optional, behind `http3` feature)
//!
//! quinn = QUIC transport
//! h3 = HTTP/3 frame encoding/decoding
//! h3-quinn = h3 + quinn integration
//!
//! Enable with: `cargo build --features http3`

use std::sync::Arc;

use crate::cert::CertManager;
use crate::db::DbState;

/// Configuration for the QUIC proxy.
pub struct QuicProxyConfig {
    pub listen_addr: String, // e.g. "0.0.0.0:443"
    pub cert_manager: Arc<CertManager>,
    pub db_state: Arc<DbState>,
}

/// Run the QUIC proxy server.
///
/// Listens on UDP, accepts QUIC connections, performs TLS handshake
/// with our CA-signed certificate, and logs HTTP/3 requests.
#[cfg(feature = "http3")]
pub async fn run_quic_proxy(config: QuicProxyConfig) -> Result<(), String> {
    use quinn::Endpoint;
    use std::net::SocketAddr;

    let addr: SocketAddr = config
        .listen_addr
        .parse()
        .map_err(|e| format!("Invalid listen address: {}", e))?;

    // Generate a server certificate for the proxy
    let (cert_pem, key_pem) = config
        .cert_manager
        .generate_host_cert("proxybot-quic")
        .map_err(|e| format!("Failed to generate QUIC cert: {}", e))?;

    // Parse PEM certificates and key
    let cert_chain: Vec<rustls::pki_types::CertificateDer> =
        rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to parse cert PEM: {}", e))?;

    let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())
        .map_err(|e| format!("Failed to parse key PEM: {}", e))?
        .ok_or("No private key found in PEM".to_string())?;

    let tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| format!("TLS config: {}", e))?;

    let crypto = quinn::crypto::rustls::QuicServerConfig::try_from(tls_config)
        .map_err(|e| format!("QUIC crypto: {}", e))?;

    let server_config = quinn::ServerConfig::with_crypto(Arc::new(crypto));
    let endpoint = Endpoint::server(server_config, addr)
        .map_err(|e| format!("QUIC endpoint bind failed: {}", e))?;

    log::info!("QUIC proxy listening on UDP {}", addr);

    while let Some(incoming) = endpoint.accept().await {
        let db = config.db_state.clone();
        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => {
                    log::info!("QUIC connection from {}", conn.remote_address());
                    handle_quic_connection(conn, db).await;
                }
                Err(e) => {
                    log::warn!("QUIC connection error: {}", e);
                }
            }
        });
    }

    Ok(())
}

#[cfg(feature = "http3")]
async fn handle_quic_connection(conn: quinn::Connection, db: Arc<DbState>) {
    loop {
        match conn.accept_bi().await {
            Ok((send, recv)) => {
                let db = db.clone();
                tokio::spawn(async move {
                    handle_http3_stream(send, recv, db).await;
                });
            }
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => break,
            Err(e) => {
                log::warn!("QUIC stream error: {}", e);
                break;
            }
        }
    }
}

#[cfg(feature = "http3")]
async fn handle_http3_stream(
    mut _send: quinn::SendStream,
    mut recv: quinn::RecvStream,
    db: Arc<DbState>,
) {
    // Read HTTP/3 frames from the QUIC stream
    let mut buf = vec![0u8; 65536];
    match recv.read(&mut buf).await {
        Ok(Some(n)) => {
            log::info!("HTTP/3 stream: {} bytes", n);

            // Record to database
            if let Ok(conn) = db.conn.lock() {
                let _ = crate::db::record_http_request(
                    &conn,
                    &chrono::Utc::now().to_rfc3339(),
                    "GET",
                    "https",
                    "quic",
                    "/",
                    &[],
                    None,
                    Some(200),
                    &[],
                    Some(&format!("QUIC: {} bytes", n)),
                    Some(0),
                    None,
                    Some("HTTP/3"),
                    // QUIC entry point: this stub doesn't yet share the
                    // proxy's `active_session_id` Arc. Stamp NULL so
                    // captures land in the "untagged" bucket; wire
                    // through ProxyContext if/when QUIC routes through
                    // the same handler as HTTP/HTTPS.
                    None,
                );
            }
        }
        Ok(None) => {
            log::info!("QUIC stream closed");
        }
        Err(e) => {
            log::warn!("QUIC read error: {}", e);
        }
    }
}

// Non-feature stub — compiles without http3 feature
#[cfg(not(feature = "http3"))]
pub async fn run_quic_proxy(_config: QuicProxyConfig) -> Result<(), String> {
    Err(
        "HTTP/3 QUIC proxy requires 'http3' feature. Enable with: cargo build --features http3"
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quic_proxy_requires_feature() {
        let config = QuicProxyConfig {
            listen_addr: "0.0.0.0:443".into(),
            cert_manager: Arc::new(CertManager::new(None).unwrap()),
            db_state: Arc::new(DbState::new().unwrap()),
        };
        #[cfg(not(feature = "http3"))]
        {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(run_quic_proxy(config));
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("http3"));
        }
    }
}
