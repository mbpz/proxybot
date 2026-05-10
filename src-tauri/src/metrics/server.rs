use crate::metrics::counters::ProxyMetrics;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A lightweight HTTP server that exposes a Prometheus-compatible `/metrics` endpoint.
///
/// Runs a simple TCP accept loop — no heavyweight HTTP framework needed for
/// scraping. Each accepted connection reads one request, writes the full metrics
/// payload, and closes.
pub struct MetricsServer {
    addr: SocketAddr,
}

impl MetricsServer {
    pub fn new(port: u16) -> Self {
        Self {
            addr: SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), port),
        }
    }

    /// Start the Prometheus metrics HTTP server.
    ///
    /// Binds to `localhost:<port>` and serves `GET /metrics` (or any path) with
    /// the current metric values.  Runs forever until the Tokio runtime is dropped
    /// or the process is terminated.
    pub async fn start(&self, metrics: Arc<ProxyMetrics>) -> Result<(), String> {
        let listener = TcpListener::bind(self.addr)
            .await
            .map_err(|e| format!("Metrics server bind failed on {}: {}", self.addr, e))?;

        log::info!(
            "Prometheus metrics server listening on http://{}",
            self.addr
        );

        loop {
            let (mut stream, peer) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    log::error!("Metrics accept error: {}", e);
                    continue;
                }
            };

            log::debug!("Metrics scrape from {}", peer);

            let metrics = Arc::clone(&metrics);

            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).await.unwrap_or(0);

                // Only log non-empty requests at trace level
                if n > 0 {
                    log::trace!(
                        "Metrics request: {}",
                        String::from_utf8_lossy(&buf[..n.min(200)])
                    );
                }

                let response = metrics.render();
                let http_response = format!(
                    "HTTP/1.1 200 OK\r\n\
                     Content-Type: text/plain; version=0.0.4\r\n\
                     Content-Length: {}\r\n\
                     Connection: close\r\n\
                     \r\n\
                     {}",
                    response.len(),
                    response,
                );

                if let Err(e) = stream.write_all(http_response.as_bytes()).await {
                    log::error!("Metrics write error for {}: {}", peer, e);
                }
            });
        }
    }
}
