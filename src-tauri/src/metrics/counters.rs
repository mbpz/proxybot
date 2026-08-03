use std::sync::atomic::{AtomicU64, Ordering};

/// Prometheus-compatible metrics counters.
///
/// All counters use `AtomicU64` with `Relaxed` ordering — metrics are
/// best-effort and do not participate in any synchronization protocol.
pub struct ProxyMetrics {
    // Connection counters
    pub connections_total: AtomicU64,
    pub connections_active: AtomicU64,
    pub connections_closed: AtomicU64,

    // Request counters
    pub http_requests_total: AtomicU64,
    pub https_requests_total: AtomicU64,
    pub requests_by_method: [AtomicU64; 8], // GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS, OTHER

    // Response counters
    pub responses_2xx: AtomicU64,
    pub responses_3xx: AtomicU64,
    pub responses_4xx: AtomicU64,
    pub responses_5xx: AtomicU64,

    // Bytes transferred
    pub bytes_received: AtomicU64,
    pub bytes_sent: AtomicU64,

    // Error counters
    pub errors_total: AtomicU64,
    pub tls_errors: AtomicU64,
    pub connect_errors: AtomicU64,

    // Plugin hooks
    pub plugin_hooks_total: AtomicU64,
    pub plugin_hooks_errors: AtomicU64,
}

impl ProxyMetrics {
    pub fn new() -> Self {
        Self {
            connections_total: AtomicU64::new(0),
            connections_active: AtomicU64::new(0),
            connections_closed: AtomicU64::new(0),
            http_requests_total: AtomicU64::new(0),
            https_requests_total: AtomicU64::new(0),
            requests_by_method: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            responses_2xx: AtomicU64::new(0),
            responses_3xx: AtomicU64::new(0),
            responses_4xx: AtomicU64::new(0),
            responses_5xx: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            tls_errors: AtomicU64::new(0),
            connect_errors: AtomicU64::new(0),
            plugin_hooks_total: AtomicU64::new(0),
            plugin_hooks_errors: AtomicU64::new(0),
        }
    }

    /// Record a request by HTTP method.
    pub fn record_method(&self, method: &str) {
        let idx = match method {
            "GET" => 0,
            "POST" => 1,
            "PUT" => 2,
            "DELETE" => 3,
            "PATCH" => 4,
            "HEAD" => 5,
            "OPTIONS" => 6,
            _ => 7,
        };
        self.requests_by_method[idx].fetch_add(1, Ordering::Relaxed);
    }

    /// Record a response status code (bucketed by 2xx/3xx/4xx/5xx).
    pub fn record_status(&self, status: u16) {
        match status / 100 {
            2 => {
                self.responses_2xx.fetch_add(1, Ordering::Relaxed);
            }
            3 => {
                self.responses_3xx.fetch_add(1, Ordering::Relaxed);
            }
            4 => {
                self.responses_4xx.fetch_add(1, Ordering::Relaxed);
            }
            5 => {
                self.responses_5xx.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        };
    }

    /// Generate Prometheus text-format output (OpenMetrics-compatible).
    pub fn render(&self) -> String {
        let method_names = [
            "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "OTHER",
        ];
        let mut out = String::with_capacity(2048);

        // Connections
        out.push_str("# HELP proxybot_connections_total Total connections accepted\n");
        out.push_str("# TYPE proxybot_connections_total counter\n");
        out.push_str(&format!(
            "proxybot_connections_total {}\n",
            self.connections_total.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP proxybot_connections_active Currently active connections\n");
        out.push_str("# TYPE proxybot_connections_active gauge\n");
        out.push_str(&format!(
            "proxybot_connections_active {}\n",
            self.connections_active.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP proxybot_connections_closed Total closed connections\n");
        out.push_str("# TYPE proxybot_connections_closed counter\n");
        out.push_str(&format!(
            "proxybot_connections_closed {}\n",
            self.connections_closed.load(Ordering::Relaxed)
        ));

        // Requests
        out.push_str("# HELP proxybot_http_requests_total Total plain HTTP requests\n");
        out.push_str("# TYPE proxybot_http_requests_total counter\n");
        out.push_str(&format!(
            "proxybot_http_requests_total {}\n",
            self.http_requests_total.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP proxybot_https_requests_total Total HTTPS requests\n");
        out.push_str("# TYPE proxybot_https_requests_total counter\n");
        out.push_str(&format!(
            "proxybot_https_requests_total {}\n",
            self.https_requests_total.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP proxybot_requests_by_method Requests by HTTP method\n");
        out.push_str("# TYPE proxybot_requests_by_method counter\n");
        for (i, name) in method_names.iter().enumerate() {
            let count = self.requests_by_method[i].load(Ordering::Relaxed);
            if count > 0 {
                out.push_str(&format!(
                    "proxybot_requests_by_method{{method=\"{}\"}} {}\n",
                    name, count
                ));
            }
        }

        // Responses
        out.push_str("# HELP proxybot_responses_total Responses by status class\n");
        out.push_str("# TYPE proxybot_responses_total counter\n");
        out.push_str(&format!(
            "proxybot_responses_total{{status_class=\"2xx\"}} {}\n",
            self.responses_2xx.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proxybot_responses_total{{status_class=\"3xx\"}} {}\n",
            self.responses_3xx.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proxybot_responses_total{{status_class=\"4xx\"}} {}\n",
            self.responses_4xx.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proxybot_responses_total{{status_class=\"5xx\"}} {}\n",
            self.responses_5xx.load(Ordering::Relaxed)
        ));

        // Bytes
        out.push_str("# HELP proxybot_bytes_received Total bytes received from clients\n");
        out.push_str("# TYPE proxybot_bytes_received counter\n");
        out.push_str(&format!(
            "proxybot_bytes_received {}\n",
            self.bytes_received.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP proxybot_bytes_sent Total bytes sent to clients\n");
        out.push_str("# TYPE proxybot_bytes_sent counter\n");
        out.push_str(&format!(
            "proxybot_bytes_sent {}\n",
            self.bytes_sent.load(Ordering::Relaxed)
        ));

        // Errors
        out.push_str("# HELP proxybot_errors_total Total errors\n");
        out.push_str("# TYPE proxybot_errors_total counter\n");
        out.push_str(&format!(
            "proxybot_errors_total {}\n",
            self.errors_total.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proxybot_tls_errors_total {}\n",
            self.tls_errors.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proxybot_connect_errors_total {}\n",
            self.connect_errors.load(Ordering::Relaxed)
        ));

        // Plugin hooks
        out.push_str("# HELP proxybot_plugin_hooks_total Plugin hook executions\n");
        out.push_str("# TYPE proxybot_plugin_hooks_total counter\n");
        out.push_str(&format!(
            "proxybot_plugin_hooks_total {}\n",
            self.plugin_hooks_total.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "proxybot_plugin_hooks_errors_total {}\n",
            self.plugin_hooks_errors.load(Ordering::Relaxed)
        ));

        out
    }
}

impl Default for ProxyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_method() {
        let m = ProxyMetrics::new();
        m.record_method("GET");
        m.record_method("POST");
        m.record_method("GET");
        m.record_method("UNKNOWN");
        assert_eq!(m.requests_by_method[0].load(Ordering::Relaxed), 2); // GET
        assert_eq!(m.requests_by_method[1].load(Ordering::Relaxed), 1); // POST
        assert_eq!(m.requests_by_method[7].load(Ordering::Relaxed), 1); // OTHER
    }

    #[test]
    fn test_record_status() {
        let m = ProxyMetrics::new();
        m.record_status(200);
        m.record_status(201);
        m.record_status(301);
        m.record_status(404);
        m.record_status(500);
        m.record_status(502);
        assert_eq!(m.responses_2xx.load(Ordering::Relaxed), 2);
        assert_eq!(m.responses_3xx.load(Ordering::Relaxed), 1);
        assert_eq!(m.responses_4xx.load(Ordering::Relaxed), 1);
        assert_eq!(m.responses_5xx.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_record_status_ignores_invalid() {
        let m = ProxyMetrics::new();
        m.record_status(99); // invalid - should be ignored
        m.record_status(600); // invalid - should be ignored
        assert_eq!(m.responses_2xx.load(Ordering::Relaxed), 0);
        assert_eq!(m.responses_5xx.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_render_contains_keys() {
        let m = ProxyMetrics::new();
        m.connections_total.fetch_add(42, Ordering::Relaxed);
        m.connections_active.fetch_add(3, Ordering::Relaxed);
        m.http_requests_total.fetch_add(10, Ordering::Relaxed);
        m.https_requests_total.fetch_add(5, Ordering::Relaxed);
        m.record_method("GET");
        m.record_status(200);
        m.bytes_received.fetch_add(1024, Ordering::Relaxed);
        m.bytes_sent.fetch_add(512, Ordering::Relaxed);
        m.errors_total.fetch_add(1, Ordering::Relaxed);
        m.plugin_hooks_total.fetch_add(7, Ordering::Relaxed);

        let out = m.render();
        assert!(out.contains("proxybot_connections_total 42"));
        assert!(out.contains("proxybot_connections_active 3"));
        assert!(out.contains("proxybot_http_requests_total 10"));
        assert!(out.contains("proxybot_https_requests_total 5"));
        assert!(out.contains("proxybot_requests_by_method{method=\"GET\"} 1"));
        assert!(out.contains("proxybot_responses_total{status_class=\"2xx\"} 1"));
        assert!(out.contains("proxybot_bytes_received 1024"));
        assert!(out.contains("proxybot_bytes_sent 512"));
        assert!(out.contains("proxybot_errors_total 1"));
        assert!(out.contains("proxybot_plugin_hooks_total 7"));
    }

    #[test]
    fn test_default_creates_new_instance() {
        let m1 = ProxyMetrics::default();
        let m2 = ProxyMetrics::new();
        // Both should render identically for zero values
        assert_eq!(m1.render(), m2.render());
    }

    #[test]
    fn test_connection_counters() {
        let m = ProxyMetrics::new();
        m.connections_total.fetch_add(1, Ordering::Relaxed);
        m.connections_active.fetch_add(1, Ordering::Relaxed);
        m.connections_closed.fetch_add(1, Ordering::Relaxed);
        m.connections_active.fetch_sub(1, Ordering::Relaxed);

        assert_eq!(m.connections_total.load(Ordering::Relaxed), 1);
        assert_eq!(m.connections_active.load(Ordering::Relaxed), 0);
        assert_eq!(m.connections_closed.load(Ordering::Relaxed), 1);
    }
}
