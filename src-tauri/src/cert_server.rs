//! Local HTTP server for serving the CA certificate to mobile devices on the LAN.
//!
//! Path-based routing:
//!   /ios.mobileconfig  — dynamic .mobileconfig profile
//!   /android-setup     — self-contained HTML setup wizard
//!   /* (anything else) — CA certificate download (unchanged behavior)

use crate::cert::{mobileconfig, wizard};
use crate::config;
use std::sync::atomic::{AtomicBool, Ordering};

const CERT_SERVER_PORT: u16 = 19876;
static SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

/// Returns true if the CertServer is currently listening.
pub fn is_running() -> bool {
    SERVER_RUNNING.load(Ordering::SeqCst)
}

/// Starts a tiny_http server that serves the CA certificate at /ca.crt.
/// Returns the LAN IP and port so mobile devices can download via browser.
#[tauri::command]
pub fn start_cert_server(cert_path: String, local_ip: String) -> String {
    if SERVER_RUNNING.swap(true, Ordering::SeqCst) {
        return format!("http://{}:{}", local_ip, CERT_SERVER_PORT);
    }

    let server_url = format!("http://{}:{}", local_ip, CERT_SERVER_PORT);
    let cert_path_clone = cert_path;
    let server_url_clone = server_url.clone();

    std::thread::spawn(move || {
        let server = match tiny_http::Server::http(format!("{}:{}", local_ip, CERT_SERVER_PORT)) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to start cert server: {}", e);
                SERVER_RUNNING.store(false, Ordering::SeqCst);
                return;
            }
        };
        log::info!("Cert server listening on {}", server_url_clone);

        let proxy_port = config::proxy_port();
        let dns_port = config::dns_port();

        for request in server.incoming_requests() {
            // Extract path, stripping any query string
            let url = request.url();
            let path = url.split('?').next().unwrap_or(url);

            match path {
                "/ios.mobileconfig" => {
                    match std::fs::read_to_string(&cert_path_clone) {
                        Ok(ca_pem) => {
                            let body = mobileconfig::build_ios_profile(
                                &ca_pem,
                                &local_ip,
                                proxy_port,
                                dns_port,
                            );
                            let response = tiny_http::Response::from_string(body)
                                .with_header(
                                    tiny_http::Header::from_bytes(
                                        &b"Content-Type"[..],
                                        &b"application/x-apple-aspen-config; charset=utf-8"[..],
                                    )
                                    .unwrap(),
                                );
                            if let Err(e) = request.respond(response) {
                                log::error!("Cert server respond error: {}", e);
                            }
                        }
                        Err(e) => {
                            log::error!("Cert server failed to read CA for mobileconfig: {}", e);
                            let response = tiny_http::Response::from_string(
                                "<h1>500</h1><p>CA certificate not found.</p>",
                            )
                            .with_status_code(500)
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..],
                                    &b"text/html; charset=utf-8"[..],
                                )
                                .unwrap(),
                            );
                            let _ = request.respond(response);
                        }
                    }
                }
                "/android-setup" => {
                    match std::fs::read_to_string(&cert_path_clone) {
                        Ok(ca_pem) => {
                            let body = wizard::build_android_wizard(
                                &ca_pem,
                                &local_ip,
                                proxy_port,
                                dns_port,
                            );
                            let response = tiny_http::Response::from_string(body)
                                .with_header(
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
                        Err(e) => {
                            log::error!("Cert server failed to read CA for android wizard: {}", e);
                            let response = tiny_http::Response::from_string(
                                "<h1>500</h1><p>CA certificate not found.</p>",
                            )
                            .with_status_code(500)
                            .with_header(
                                tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..],
                                    &b"text/html; charset=utf-8"[..],
                                )
                                .unwrap(),
                            );
                            let _ = request.respond(response);
                        }
                    }
                }
                _ => {
                    // Default: serve the CA certificate (unchanged behavior)
                    match std::fs::read(&cert_path_clone) {
                        Ok(data) => {
                            let response = tiny_http::Response::from_data(data)
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
                        Err(e) => {
                            log::error!("Cert server failed to read cert: {}", e);
                            let response =
                                tiny_http::Response::from_string("Certificate not found")
                                    .with_status_code(404);
                            let _ = request.respond(response);
                        }
                    }
                }
            }
        }
    });

    server_url
}
