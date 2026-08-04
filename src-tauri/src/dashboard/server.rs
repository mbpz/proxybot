use crate::metrics::counters::ProxyMetrics;
use crate::proxy::InterceptedRequest;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::{JoinHandle, JoinSet};

const MAX_REQUESTS: usize = 1000;

struct RunningDashboard {
    shutdown_tx: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

/// Owned lifecycle for the mobile dashboard listener and its connection tasks.
pub struct DashboardServer {
    configured_port: u16,
    bound_port: Arc<AtomicU16>,
    token: String,
    pub(crate) requests: Arc<Mutex<Vec<InterceptedRequest>>>,
    running: Arc<AtomicBool>,
    lifecycle: tokio::sync::Mutex<Option<RunningDashboard>>,
    metrics: Arc<ProxyMetrics>,
}

impl DashboardServer {
    pub fn new(port: u16, metrics: Arc<ProxyMetrics>) -> Self {
        Self {
            configured_port: port,
            bound_port: Arc::new(AtomicU16::new(port)),
            token: generate_token(),
            requests: Arc::new(Mutex::new(Vec::with_capacity(MAX_REQUESTS))),
            running: Arc::new(AtomicBool::new(false)),
            lifecycle: tokio::sync::Mutex::new(None),
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
        self.running.load(Ordering::Acquire)
    }

    pub fn port(&self) -> u16 {
        self.bound_port.load(Ordering::Acquire)
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub async fn start(&self) -> Result<(), String> {
        let mut lifecycle = self.lifecycle.lock().await;
        if lifecycle
            .as_ref()
            .is_some_and(|running| !running.task.is_finished())
        {
            return Err("Dashboard already running".to_owned());
        }
        if let Some(finished) = lifecycle.take() {
            let _ = finished.task.await;
        }

        let addr = SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), self.configured_port);
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|error| format!("Dashboard bind failed on {addr}: {error}"))?;
        let bound_port = listener
            .local_addr()
            .map_err(|error| format!("Dashboard local address failed: {error}"))?
            .port();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let requests = Arc::clone(&self.requests);
        let token = self.token.clone();
        let metrics = Arc::clone(&self.metrics);
        let running = Arc::clone(&self.running);
        let task = tokio::spawn(async move {
            serve(listener, shutdown_rx, requests, token, metrics).await;
            running.store(false, Ordering::Release);
        });

        self.bound_port.store(bound_port, Ordering::Release);
        self.running.store(true, Ordering::Release);
        *lifecycle = Some(RunningDashboard { shutdown_tx, task });
        log::info!("Dashboard server listening on port {bound_port}");
        Ok(())
    }

    /// Idempotent completion barrier: returns only after the listener and all
    /// spawned connection tasks have stopped.
    pub async fn stop(&self) -> Result<(), String> {
        let Some(running) = self.lifecycle.lock().await.take() else {
            self.running.store(false, Ordering::Release);
            return Ok(());
        };
        let _ = running.shutdown_tx.send(());
        running
            .task
            .await
            .map_err(|error| format!("Dashboard task failed during shutdown: {error}"))?;
        self.running.store(false, Ordering::Release);
        self.bound_port
            .store(self.configured_port, Ordering::Release);
        Ok(())
    }
}

async fn serve(
    listener: TcpListener,
    mut shutdown_rx: oneshot::Receiver<()>,
    requests: Arc<Mutex<Vec<InterceptedRequest>>>,
    token: String,
    metrics: Arc<ProxyMetrics>,
) {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, peer)) => {
                        connections.spawn(handle_connection(
                            stream,
                            peer,
                            Arc::clone(&requests),
                            token.clone(),
                            Arc::clone(&metrics),
                        ));
                    }
                    Err(error) => {
                        log::error!("Dashboard accept error: {error}");
                        break;
                    }
                }
            }
            completed = connections.join_next(), if !connections.is_empty() => {
                if let Some(Err(error)) = completed {
                    log::error!("Dashboard connection task failed: {error}");
                }
            }
        }
    }
    connections.abort_all();
    while connections.join_next().await.is_some() {}
    log::info!("Dashboard server stopped");
}

async fn handle_connection(
    mut stream: TcpStream,
    peer: SocketAddr,
    requests: Arc<Mutex<Vec<InterceptedRequest>>>,
    token: String,
    metrics: Arc<ProxyMetrics>,
) {
    let mut buf = [0_u8; 8192];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    if n == 0 {
        return;
    }
    let request = String::from_utf8_lossy(&buf[..n]);
    let parts: Vec<_> = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    if parts.len() < 2 {
        return;
    }
    let method = parts[0];
    let path = parts[1];
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

    let (clean_path, query) = path.split_once('?').unwrap_or((path, ""));
    let request_token = query
        .split('&')
        .find_map(|part| part.strip_prefix("token="))
        .unwrap_or_default();
    if clean_path != "/" && request_token != token {
        let body = serde_json::json!({
            "error": "Unauthorized",
            "message": "Add ?token=<TOKEN> to the URL"
        })
        .to_string();
        let response = http_response("401 Unauthorized", "application/json", &body);
        let _ = stream.write_all(response.as_bytes()).await;
        return;
    }

    let (status, content_type, body) = match (method, clean_path) {
        ("GET", "/") => (
            "200 OK",
            "text/html; charset=utf-8",
            include_str!("dashboard.html").to_owned(),
        ),
        ("GET", "/api/requests") => {
            let recent = requests
                .lock()
                .unwrap()
                .iter()
                .rev()
                .take(100)
                .cloned()
                .collect::<Vec<_>>();
            match serde_json::to_string(&recent) {
                Ok(json) => ("200 OK", "application/json", json),
                Err(error) => (
                    "500 Internal Server Error",
                    "application/json",
                    serde_json::json!({"error": error.to_string()}).to_string(),
                ),
            }
        }
        ("GET", "/api/stats") => {
            let total = metrics.http_requests_total.load(Ordering::Relaxed)
                + metrics.https_requests_total.load(Ordering::Relaxed);
            let active = metrics.connections_active.load(Ordering::Relaxed);
            let bytes = metrics.bytes_received.load(Ordering::Relaxed)
                + metrics.bytes_sent.load(Ordering::Relaxed);
            (
                "200 OK",
                "application/json",
                serde_json::json!({
                    "total_requests": total,
                    "active_connections": active,
                    "bytes_total": bytes,
                })
                .to_string(),
            )
        }
        ("GET", "/api/connections") => (
            "200 OK",
            "application/json",
            serde_json::json!({
                "active_connections": metrics.connections_active.load(Ordering::Relaxed)
            })
            .to_string(),
        ),
        _ => (
            "404 Not Found",
            "application/json",
            serde_json::json!({"error": "Not Found", "path": clean_path}).to_string(),
        ),
    };
    let response = http_response(status, content_type, &body);
    if let Err(error) = stream.write_all(response.as_bytes()).await {
        log::error!("Dashboard write error for {peer}: {error}");
    }
}

fn http_response(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type, Authorization\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    )
}

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

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    #[tokio::test]
    async fn start_repeat_and_stop_are_completion_safe() {
        let dashboard = DashboardServer::new(0, Arc::new(ProxyMetrics::new()));
        dashboard.start().await.unwrap();
        assert!(dashboard.is_running());
        assert_ne!(dashboard.port(), 0);
        assert!(dashboard.start().await.is_err());
        dashboard.stop().await.unwrap();
        assert!(!dashboard.is_running());
        dashboard.stop().await.unwrap();

        dashboard.start().await.unwrap();
        dashboard.stop().await.unwrap();
    }

    #[tokio::test]
    async fn bind_failure_never_reports_running() {
        let occupied = TcpListener::bind("0.0.0.0:0").await.unwrap();
        let port = occupied.local_addr().unwrap().port();
        let dashboard = DashboardServer::new(port, Arc::new(ProxyMetrics::new()));
        assert!(dashboard.start().await.is_err());
        assert!(!dashboard.is_running());
    }
}
