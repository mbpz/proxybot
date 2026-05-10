use crate::metrics::counters::METRICS;
use crate::proxy::InterceptedRequest;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Maximum number of requests to keep in the ring buffer.
const MAX_REQUESTS: usize = 1000;

/// A lightweight embedded HTTP server that serves:
/// - `GET /` — the live dashboard HTML page
/// - `GET /api/requests` — JSON array of recent intercepted requests (last 100)
/// - `GET /api/stats` — JSON summary: total_requests, active_connections, bytes_total
///
/// Uses the same hand-rolled HTTP pattern as `MetricsServer` — no heavyweight
/// framework dependency.
pub struct DashboardServer {
    addr: SocketAddr,
    /// Ring buffer of recent intercepted requests, shared with request-pushing caller.
    requests: Arc<RwLock<Vec<InterceptedRequest>>>,
}

impl DashboardServer {
    pub fn new(port: u16) -> Self {
        Self {
            addr: SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), port),
            requests: Arc::new(RwLock::new(Vec::with_capacity(MAX_REQUESTS))),
        }
    }

    /// Push a newly intercepted request into the ring buffer.
    ///
    /// Thread-safe — can be called from any task. Drops the oldest request when
    /// the buffer exceeds `MAX_REQUESTS`.
    pub fn push_request(&self, req: InterceptedRequest) {
        let mut guard = self.requests.write().unwrap();
        if guard.len() >= MAX_REQUESTS {
            guard.remove(0);
        }
        guard.push(req);
    }

    /// Return a clone of the shared request buffer for use in async contexts.
    pub fn requests_handle(&self) -> Arc<RwLock<Vec<InterceptedRequest>>> {
        Arc::clone(&self.requests)
    }

    /// Start the dashboard HTTP server.
    ///
    /// Binds to `localhost:<port>` and serves dashboard endpoints. Runs
    /// forever until the Tokio runtime is dropped or the process is terminated.
    pub async fn start(&self) -> Result<(), String> {
        let listener = TcpListener::bind(self.addr)
            .await
            .map_err(|e| format!("Dashboard server bind failed on {}: {}", self.addr, e))?;

        log::info!(
            "Dashboard server listening on http://{}",
            self.addr
        );

        let requests = Arc::clone(&self.requests);

        loop {
            let (mut stream, peer) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    log::error!("Dashboard accept error: {}", e);
                    continue;
                }
            };

            log::debug!("Dashboard request from {}", peer);

            let requests = Arc::clone(&requests);

            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }

                let request_str = String::from_utf8_lossy(&buf[..n]);
                let first_line = request_str.lines().next().unwrap_or("");
                let parts: Vec<&str> = first_line.split_whitespace().collect();

                let (method, path) = if parts.len() >= 2 {
                    (parts[0], parts[1])
                } else {
                    return;
                };

                log::trace!("Dashboard: {} {}", method, path);

                let (status, content_type, body) = match (method, path) {
                    ("GET", "/") => {
                        let html = include_str!("dashboard.html");
                        (
                            "200 OK",
                            "text/html; charset=utf-8",
                            html.to_string(),
                        )
                    }
                    ("GET", "/api/requests") => {
                        let guard = requests.read().unwrap();
                        // Return last 100 requests, newest first
                        let recent: Vec<&InterceptedRequest> =
                            guard.iter().rev().take(100).collect();
                        match serde_json::to_string(&recent) {
                            Ok(json) => ("200 OK", "application/json", json),
                            Err(e) => (
                                "500 Internal Server Error",
                                "application/json",
                                format!(r#"{{"error":"{}"}}"#, e),
                            ),
                        }
                    }
                    ("GET", "/api/stats") => {
                        let metrics = &*METRICS;
                        let total_requests = metrics.http_requests_total.load(Ordering::Relaxed)
                            + metrics.https_requests_total.load(Ordering::Relaxed);
                        let active_connections =
                            metrics.connections_active.load(Ordering::Relaxed);
                        let bytes_total = metrics.bytes_received.load(Ordering::Relaxed)
                            + metrics.bytes_sent.load(Ordering::Relaxed);
                        let json = serde_json::json!({
                            "total_requests": total_requests,
                            "active_connections": active_connections,
                            "bytes_total": bytes_total,
                        });
                        ("200 OK", "application/json", json.to_string())
                    }
                    ("GET", "/api/connections") => {
                        let count = METRICS.connections_active.load(Ordering::Relaxed);
                        let json = serde_json::json!({
                            "active_connections": count,
                        });
                        ("200 OK", "application/json", json.to_string())
                    }
                    _ => {
                        let json = serde_json::json!({
                            "error": "Not Found",
                            "path": path,
                        });
                        ("404 Not Found", "application/json", json.to_string())
                    }
                };

                let http_response = format!(
                    "HTTP/1.1 {}\r\n\
                     Content-Type: {}\r\n\
                     Content-Length: {}\r\n\
                     Access-Control-Allow-Origin: *\r\n\
                     Access-Control-Allow-Methods: GET, OPTIONS\r\n\
                     Access-Control-Allow-Headers: Content-Type\r\n\
                     Connection: close\r\n\
                     \r\n\
                     {}",
                    status,
                    content_type,
                    body.len(),
                    body,
                );

                if let Err(e) = stream.write_all(http_response.as_bytes()).await {
                    log::error!("Dashboard write error for {}: {}", peer, e);
                }
            });
        }
    }
}
