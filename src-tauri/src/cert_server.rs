//! Local HTTP server for serving the CA certificate to mobile devices on the LAN.

use std::sync::atomic::{AtomicBool, Ordering};

const CERT_SERVER_PORT: u16 = 19876;
static SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

/// Starts a tiny_http server that serves the CA certificate at /ca.crt.
/// Returns the LAN IP and port so mobile devices can download via browser.
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

        for request in server.incoming_requests() {
            let path = request.url().trim_start_matches('/');
            let file_path = &cert_path_clone;

            match std::fs::read(file_path) {
                Ok(data) => {
                    let response = tiny_http::Response::from_data(data)
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"application/x-x509-ca-cert"[..],
                            ).unwrap(),
                        )
                        .with_header(
                            tiny_http::Header::from_bytes(
                                &b"Content-Disposition"[..],
                                &b"attachment; filename=\"ProxyBot_CA.crt\""[..],
                            ).unwrap(),
                        );
                    if let Err(e) = request.respond(response) {
                        log::error!("Cert server respond error: {}", e);
                    }
                }
                Err(e) => {
                    log::error!("Cert server failed to read cert: {}", e);
                    let response = tiny_http::Response::from_string("Certificate not found")
                        .with_status_code(404);
                    let _ = request.respond(response);
                }
            }
        }
    });

    server_url
}
