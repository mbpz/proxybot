//! Local HTTP server for serving the CA certificate to mobile devices on the LAN.
//!
//! Path-based routing:
//!   /ios.mobileconfig  — dynamic .mobileconfig profile
//!   /android-setup     — self-contained HTML setup wizard
//!   /* (anything else) — CA certificate download (unchanged behavior)

use crate::cert::{mobileconfig, wizard};
use crate::config;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

/// Returns true if the CertServer is currently listening.
pub fn is_running() -> bool {
    SERVER_RUNNING.load(Ordering::SeqCst)
}

/// Starts a tiny_http server that serves the CA certificate at /ca.crt.
/// Returns the LAN IP and port so mobile devices can download via browser.
#[tauri::command]
pub fn start_cert_server(cert_path: String, local_ip: String) -> String {
    let port = config::cert_server_port();
    let server_url = format!("http://{}:{}", local_ip, port);

    if SERVER_RUNNING.load(Ordering::SeqCst) {
        return server_url;
    }

    // Bind synchronously on the calling thread BEFORE marking the server
    // as running. This avoids a race where a concurrent caller observes
    // is_running()==true and builds a QR for a port that isn't bound yet
    // (or has just failed to bind).
    let server = match tiny_http::Server::http(format!("{}:{}", local_ip, port)) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to start cert server: {}", e);
            // Do NOT set SERVER_RUNNING — is_running() must reflect the
            // bind state so callers can surface the failure.
            return server_url;
        }
    };

    // Read the CA PEM once at bind time. The cert doesn't change while
    // the server is running, so caching it avoids a disk read on every
    // request to /ios.mobileconfig, /android-setup, or the catch-all
    // CA download route.
    let ca_pem = match std::fs::read_to_string(&cert_path) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::error!("Failed to read CA cert: {}", e);
            // Bind succeeded but we can't serve cert-dependent routes.
            // Leave SERVER_RUNNING=false so generate_device_qr fails
            // gracefully instead of pointing phones at a broken server.
            return server_url;
        }
    };

    // Bind succeeded AND CA is readable; mark the server as running.
    SERVER_RUNNING.store(true, Ordering::SeqCst);
    log::info!("Cert server listening on {}", server_url);

    std::thread::spawn(move || {
        let proxy_port = config::proxy_port();
        let dns_port = config::dns_port();
        let ca_pem_arc = ca_pem.clone();
        let ca_pem = ca_pem_arc.as_str();

        for request in server.incoming_requests() {
            // Extract path, stripping any query string
            let url = request.url();
            let path = url.split('?').next().unwrap_or(url);

            match path {
                "/ios.mobileconfig" => {
                    let body =
                        mobileconfig::build_ios_profile(ca_pem, &local_ip, proxy_port, dns_port);
                    let response = tiny_http::Response::from_string(body)
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"application/x-apple-aspen-config; charset=utf-8"[..],
                            )
                            .unwrap(),
                        )
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Disposition"[..],
                                &b"attachment; filename=\"proxybot-ios.mobileconfig\""[..],
                            )
                            .unwrap(),
                        );
                    if let Err(e) = request.respond(response) {
                        log::error!("Cert server respond error: {}", e);
                    }
                }
                "/android-setup" => {
                    let body = wizard::build_android_wizard(&local_ip, proxy_port, dns_port);
                    let response = tiny_http::Response::from_string(body).with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"text/html; charset=utf-8"[..],
                        )
                        .unwrap(),
                    );
                    if let Err(e) = request.respond(response) {
                        log::error!("Cert server respond error: {}", e);
                    }
                }
                _ => {
                    // Default: serve the CA certificate (unchanged behavior)
                    // Bytes are served from the cached PEM string.
                    let response = tiny_http::Response::from_data(ca_pem.as_bytes().to_vec())
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"application/x-x509-ca-cert"[..],
                            )
                            .unwrap(),
                        )
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Disposition"[..],
                                &b"attachment; filename=\"ProxyBot_CA.crt\""[..],
                            )
                            .unwrap(),
                        );
                    if let Err(e) = request.respond(response) {
                        log::error!("Cert server respond error: {}", e);
                    }
                }
            }
        }
    });

    server_url
}
