use crate::metrics::counters::ProxyMetrics;
use crate::proxy::InterceptedRequest;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::watch;

const MAX_REQUESTS: usize = 1000;

/// Lightweight HTTP server for mobile dashboard access.
///
/// Binds to 0.0.0.0:<port> with token-based auth.
/// Phones on the same LAN access via http://<LAN_IP>:<port>?token=<TOKEN>
pub struct DashboardServer {
    port: u16,
    token: String,
    pub(crate) requests: Arc<Mutex<Vec<InterceptedRequest>>>,
    running: Arc<AtomicBool>,
    shutdown_tx: Arc<Mutex<Option<watch::Sender<()>>>>,
    metrics: Arc<ProxyMetrics>,
}

impl DashboardServer {
    pub fn new(port: u16, metrics: Arc<ProxyMetrics>) -> Self {
        let token = generate_token();
        Self {
            port,
            token,
            requests: Arc::new(Mutex::new(Vec::with_capacity(MAX_REQUESTS))),
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
            metrics,
        }
    }

    pub fn push_request(&self, req: InterceptedRequest) {
        let mut guard = self.requests.lock().unwrap();
        if guard.len() >= MAX_REQUESTS {
            guard.remove(0);
        }
        guard.push(req);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub async fn start(&self) -> Result<(), String> {
        if self.running.load(Ordering::Relaxed) {
            return Err("Dashboard already running".into());
        }

        let addr = SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), self.port);
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| format!("Dashboard bind failed on {}: {}", addr, e))?;

        self.running.store(true, Ordering::Relaxed);
        let (shutdown_tx, mut shutdown_rx) = watch::channel(());
        *self.shutdown_tx.lock().unwrap() = Some(shutdown_tx);

        let requests = Arc::clone(&self.requests);
        let running = Arc::clone(&self.running);
        let token = self.token.clone();
        let metrics = Arc::clone(&self.metrics);

        log::info!("Dashboard server listening on http://0.0.0.0:{}", self.port);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        let (mut stream, peer) = match result {
                            Ok(conn) => conn,
                            Err(e) => {
                                log::error!("Dashboard accept error: {}", e);
                                continue;
                            }
                        };

                        let requests = Arc::clone(&requests);
                        let token = token.clone();
                        let metrics = Arc::clone(&metrics);

                        tokio::spawn(async move {
                            let mut buf = [0u8; 8192];
                            let n = stream.read(&mut buf).await.unwrap_or(0);
                            if n == 0 { return; }

                            let request_str = String::from_utf8_lossy(&buf[..n]);
                            let first_line = request_str.lines().next().unwrap_or("");
                            let parts: Vec<&str> = first_line.split_whitespace().collect();

                            let (method, path) = if parts.len() >= 2 {
                                (parts[0], parts[1])
                            } else {
                                return;
                            };

                            // Handle CORS preflight
                            if method == "OPTIONS" {
                                let response = "HTTP/1.1 204 No Content\r\n\
                                    Access-Control-Allow-Origin: *\r\n\
                                    Access-Control-Allow-Methods: GET, OPTIONS\r\n\
                                    Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
                                    Access-Control-Max-Age: 86400\r\n\
                                    Connection: close\r\n\r\n";
                                let _ = stream.write_all(response.as_bytes()).await;
                                return;
                            }

                            // Extract path without query string for routing
                            let (clean_path, query) = match path.split_once('?') {
                                Some((p, q)) => (p, q),
                                None => (path, ""),
                            };

                            // Parse token from query string
                            let req_token = query.split('&')
                                .find(|p| p.starts_with("token="))
                                .and_then(|p| p.strip_prefix("token="))
                                .unwrap_or("");

                            // Auth check — reject if token missing/wrong (except for root page which shows instructions)
                            if clean_path != "/" && req_token != token {
                                let json = serde_json::json!({"error": "Unauthorized", "message": "Add ?token=<TOKEN> to the URL"});
                                let body = json.to_string();
                                let response = format!(
                                    "HTTP/1.1 401 Unauthorized\r\n\
                                     Content-Type: application/json\r\n\
                                     Content-Length: {}\r\n\
                                     Access-Control-Allow-Origin: *\r\n\
                                     Connection: close\r\n\r\n{}",
                                    body.len(), body
                                );
                                let _ = stream.write_all(response.as_bytes()).await;
                                return;
                            }

                            let (status, content_type, body) = match (method, clean_path) {
                                ("GET", "/") => {
                                    let html = include_str!("dashboard.html");
                                    ("200 OK", "text/html; charset=utf-8", html.to_string())
                                }
                                ("GET", "/api/requests") => {
                                    let guard = requests.lock().unwrap();
                                    let recent: Vec<&InterceptedRequest> = guard.iter().rev().take(100).collect();
                                    match serde_json::to_string(&recent) {
                                        Ok(json) => ("200 OK", "application/json", json),
                                        Err(e) => ("500 Internal Server Error", "application/json",
                                            format!(r#"{{"error":"{}"}}"#, e)),
                                    }
                                }
                                ("GET", "/api/stats") => {
                                    let m = &*metrics;
                                    let total = m.http_requests_total.load(Ordering::Relaxed)
                                        + m.https_requests_total.load(Ordering::Relaxed);
                                    let active = m.connections_active.load(Ordering::Relaxed);
                                    let bytes = m.bytes_received.load(Ordering::Relaxed)
                                        + m.bytes_sent.load(Ordering::Relaxed);
                                    let json = serde_json::json!({
                                        "total_requests": total,
                                        "active_connections": active,
                                        "bytes_total": bytes,
                                    });
                                    ("200 OK", "application/json", json.to_string())
                                }
                                ("GET", "/api/connections") => {
                                    let count = metrics.connections_active.load(Ordering::Relaxed);
                                    let json = serde_json::json!({ "active_connections": count });
                                    ("200 OK", "application/json", json.to_string())
                                }
                                _ => {
                                    let json = serde_json::json!({ "error": "Not Found", "path": clean_path });
                                    ("404 Not Found", "application/json", json.to_string())
                                }
                            };

                            let response = format!(
                                "HTTP/1.1 {}\r\n\
                                 Content-Type: {}\r\n\
                                 Content-Length: {}\r\n\
                                 Access-Control-Allow-Origin: *\r\n\
                                 Access-Control-Allow-Methods: GET, OPTIONS\r\n\
                                 Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
                                 Connection: close\r\n\r\n{}",
                                status, content_type, body.len(), body,
                            );

                            if let Err(e) = stream.write_all(response.as_bytes()).await {
                                log::error!("Dashboard write error for {}: {}", peer, e);
                            }
                        });
                    }
                    _ = shutdown_rx.changed() => {
                        log::info!("Dashboard server shutting down");
                        break;
                    }
                }
            }
            running.store(false, Ordering::Relaxed);
        });

        Ok(())
    }

    pub fn stop(&self) {
        if let Some(tx) = self.shutdown_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
        // Don't set running=false here — the spawned task does it on exit.
        // This avoids the race where stop() returns before the port is released.
    }
}

/// Generate a cryptographically random 128-bit token for dashboard auth.
fn generate_token() -> String {
    use rand::{rngs::OsRng, RngCore};
    use std::fmt::Write;

    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);

    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(token, "{byte:02x}").expect("writing to a String cannot fail");
    }
    token
}
