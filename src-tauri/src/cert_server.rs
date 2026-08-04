//! Owned lifecycle for the LAN certificate distribution server.

use crate::cert::{mobileconfig, wizard};
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::State;

struct RunningCertServer {
    shutdown_tx: mpsc::Sender<()>,
    task: JoinHandle<()>,
}

pub struct CertServerState {
    running: Mutex<Option<RunningCertServer>>,
}

impl CertServerState {
    pub fn new() -> Self {
        Self {
            running: Mutex::new(None),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|running| !running.task.is_finished())
    }

    fn start(
        &self,
        address: &str,
        cert_path: &Path,
        local_ip: String,
        proxy_port: u16,
        dns_port: u16,
    ) -> Result<(), String> {
        let mut slot = self.running.lock().unwrap();
        if slot
            .as_ref()
            .is_some_and(|running| !running.task.is_finished())
        {
            return Err("Certificate server is already running".to_owned());
        }
        if let Some(finished) = slot.take() {
            let _ = finished.task.join();
        }

        let server = tiny_http::Server::http(address)
            .map_err(|error| format!("Certificate server bind failed on {address}: {error}"))?;
        let ca_pem = Arc::new(
            std::fs::read_to_string(cert_path)
                .map_err(|error| format!("Failed to read CA certificate: {error}"))?,
        );
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let task = std::thread::spawn(move || {
            loop {
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }
                match server.recv_timeout(Duration::from_millis(100)) {
                    Ok(Some(request)) => {
                        handle_request(request, ca_pem.as_str(), &local_ip, proxy_port, dns_port)
                    }
                    Ok(None) => {}
                    Err(error) => {
                        log::error!("Certificate server receive error: {error}");
                        break;
                    }
                }
            }
            log::info!("Certificate server stopped");
        });
        *slot = Some(RunningCertServer { shutdown_tx, task });
        Ok(())
    }

    pub(crate) fn stop(&self) -> Result<(), String> {
        let Some(running) = self.running.lock().unwrap().take() else {
            return Ok(());
        };
        let _ = running.shutdown_tx.send(());
        running
            .task
            .join()
            .map_err(|_| "Certificate server task panicked during shutdown".to_owned())
    }
}

impl Default for CertServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Start certificate distribution after both the listener bind and CA read
/// succeed. Repeated starts return an error and never create a second listener.
#[tauri::command]
pub fn start_cert_server(
    cert_path: String,
    local_ip: String,
    state: State<'_, Arc<CertServerState>>,
    config: State<'_, Arc<proxybot_core::AppConfig>>,
) -> Result<String, String> {
    let port = config.cert_server_port;
    let server_url = format!("http://{local_ip}:{port}");
    state.start(
        &format!("{local_ip}:{port}"),
        Path::new(&cert_path),
        local_ip,
        config.proxy_port,
        config.dns_port,
    )?;
    log::info!("Certificate server listening on {server_url}");
    Ok(server_url)
}

#[tauri::command]
pub fn stop_cert_server(state: State<'_, Arc<CertServerState>>) -> Result<(), String> {
    state.stop()
}

#[tauri::command]
pub fn is_cert_server_running(state: State<'_, Arc<CertServerState>>) -> bool {
    state.is_running()
}

fn handle_request(
    request: tiny_http::Request,
    ca_pem: &str,
    local_ip: &str,
    proxy_port: u16,
    dns_port: u16,
) {
    let url = request.url();
    let path = url.split('?').next().unwrap_or(url);
    let response = match path {
        "/ios.mobileconfig" => tiny_http::Response::from_string(mobileconfig::build_ios_profile(
            ca_pem, local_ip, proxy_port, dns_port,
        ))
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
        ),
        "/android-setup" => tiny_http::Response::from_string(wizard::build_android_wizard(
            local_ip, proxy_port, dns_port,
        ))
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                .unwrap(),
        ),
        _ => tiny_http::Response::from_data(ca_pem.as_bytes().to_vec())
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
            ),
    };
    if let Err(error) = request.respond(response) {
        log::error!("Certificate server response error: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn cert_file() -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"test-ca").unwrap();
        file
    }

    #[test]
    fn lifecycle_starts_after_bind_rejects_repeat_and_waits_for_stop() {
        let state = CertServerState::new();
        let cert = cert_file();
        state
            .start(
                "127.0.0.1:0",
                cert.path(),
                "127.0.0.1".to_owned(),
                9090,
                5300,
            )
            .unwrap();
        assert!(state.is_running());
        assert!(state
            .start(
                "127.0.0.1:0",
                cert.path(),
                "127.0.0.1".to_owned(),
                9090,
                5300,
            )
            .is_err());
        state.stop().unwrap();
        assert!(!state.is_running());
        state.stop().unwrap();
    }

    #[test]
    fn bind_or_certificate_failure_never_reports_running() {
        let state = CertServerState::new();
        let cert = cert_file();
        let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = occupied.local_addr().unwrap().to_string();
        assert!(state
            .start(&address, cert.path(), "127.0.0.1".to_owned(), 9090, 5300,)
            .is_err());
        assert!(!state.is_running());

        assert!(state
            .start(
                "127.0.0.1:0",
                Path::new("/missing/ca.pem"),
                "127.0.0.1".to_owned(),
                9090,
                5300,
            )
            .is_err());
        assert!(!state.is_running());
    }
}
