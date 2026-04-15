//! Built-in DNS server for ProxyBot.
//!
//! Listens on UDP port 5300 (pf redirects 53->5300), parses DNS queries to extract
//! domain names, forwards all queries to 8.8.8.8:53, and relays responses back.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};
use tauri::{AppHandle, Emitter, State};

/// DNS server listening port (pf redirects 53 -> 5300).
const DNS_PORT: u16 = 5300;
/// Upstream DNS server.
const UPSTREAM_DNS: &str = "8.8.8.8:53";
/// Maximum DNS entries to store.
const MAX_DNS_ENTRIES: usize = 10000;
/// Upstream query timeout.
const DNS_TIMEOUT_SECS: u64 = 3;

/// A single DNS query entry.
#[derive(Clone, serde::Serialize)]
pub struct DnsEntry {
    pub domain: String,
    pub timestamp_ms: u64,
}

/// Shared DNS state.
pub struct DnsState {
    pub entries: Arc<Mutex<VecDeque<DnsEntry>>>,
    pub running: Arc<AtomicBool>,
    pub shutdown_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>,
}

impl DnsState {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_DNS_ENTRIES))),
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Record a DNS query entry and emit a Tauri event.
    fn record_query(&self, domain: String, app_handle: &AppHandle) {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let entry = DnsEntry {
            domain: domain.clone(),
            timestamp_ms,
        };

        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= MAX_DNS_ENTRIES {
            entries.pop_front();
        }
        entries.push_back(entry);

        // Emit event to frontend
        let _ = app_handle.emit("dns-query", &DnsEntry { domain, timestamp_ms });
    }
}

/// Parse a domain name from DNS question section (RFC 1035 QNAME format).
///
/// QNAME is a sequence of length-prefixed labels, terminated by a null byte.
/// Each label: 1 byte length + label bytes (max 63 per label).
///
/// Returns the domain name as a string (e.g., "example.com") or None if invalid.
pub fn parse_dns_query(buf: &[u8]) -> Option<String> {
    // DNS header is 12 bytes. Question section starts after header.
    if buf.len() < 12 {
        return None;
    }

    let mut pos = 12;
    let mut labels = Vec::new();

    loop {
        if pos >= buf.len() {
            return None;
        }

        let label_len = buf[pos];

        // End of domain name
        if label_len == 0 {
            break;
        }

        // Compression pointer (top 2 bits set) - not supported in queries
        if label_len & 0xC0 != 0 {
            return None;
        }

        // Label too long (max 63)
        if label_len > 63 {
            return None;
        }

        pos += 1;

        if pos + label_len as usize > buf.len() {
            return None;
        }

        let label = &buf[pos..pos + label_len as usize];

        // Check for valid label characters (printable ASCII)
        if !label.iter().all(|&b| b >= 0x21 && b <= 0x7E) {
            return None;
        }

        labels.push(String::from_utf8_lossy(label).to_string());
        pos += label_len as usize;
    }

    if labels.is_empty() {
        return None;
    }

    Some(labels.join("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_domain() {
        // DNS header (12 bytes) + QNAME for "example.com":
        // [7] 'example' [3] 'com' [0]
        let mut buf = vec![0u8; 12];
        buf.extend_from_slice(&[7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0]);
        assert_eq!(parse_dns_query(&buf), Some("example.com".to_string()));
    }

    #[test]
    fn test_subdomain() {
        // DNS header (12 bytes) + QNAME for "www.example.com":
        // [3]www[7]example[3]com[0]
        let mut buf = vec![0u8; 12];
        buf.extend_from_slice(&[3, b'w', b'w', b'w', 7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0]);
        assert_eq!(parse_dns_query(&buf), Some("www.example.com".to_string()));
    }

    #[test]
    fn test_empty_query() {
        assert_eq!(parse_dns_query(&[]), None);
    }

    #[test]
    fn test_truncated_query() {
        // DNS header present but QNAME truncated: label length 5 but only 2 bytes available
        let buf = [0u8; 12];
        let mut full_buf = buf.to_vec();
        full_buf.extend_from_slice(&[5, b'a', b'b']);
        assert_eq!(parse_dns_query(&full_buf), None);
    }
}

/// Handle a single DNS query: parse domain, record it, forward to upstream, relay response.
async fn handle_dns_query(
    buf: &[u8],
    len: usize,
    src: SocketAddr,
    socket: &Arc<UdpSocket>,
    app_handle: &AppHandle,
    state: &DnsState,
) {
    let data = &buf[..len];

    // Parse domain name from question section
    let domain = parse_dns_query(data).unwrap_or_else(|| "unknown".to_string());

    // Record the query
    state.record_query(domain.clone(), app_handle);

    log::debug!("DNS query from {} for domain: {}", src, domain);

    // Forward to upstream DNS (8.8.8.8:53)
    match timeout(
        Duration::from_secs(DNS_TIMEOUT_SECS),
        socket.send_to(data, UPSTREAM_DNS),
    )
    .await
    {
        Ok(Ok(_)) => {
            // Read response from upstream
            let mut response_buf = vec![0u8; 512];
            match timeout(
                Duration::from_secs(DNS_TIMEOUT_SECS),
                socket.recv_from(&mut response_buf),
            )
            .await
            {
                Ok(Ok((resp_len, _))) => {
                    // Send response back to client
                    if let Err(e) = socket
                        .send_to(&response_buf[..resp_len], src)
                        .await
                    {
                        log::error!("Failed to send DNS response to {}: {}", src, e);
                    }
                }
                Ok(Err(e)) => {
                    log::error!("Failed to receive DNS response: {}", e);
                }
                Err(_) => {
                    log::warn!("DNS upstream response timed out for {}", domain);
                }
            }
        }
        Ok(Err(e)) => {
            log::error!("Failed to forward DNS query to {}: {}", UPSTREAM_DNS, e);
        }
        Err(_) => {
            log::warn!("DNS query to {} timed out", UPSTREAM_DNS);
        }
    }
}

/// Run the DNS server loop.
async fn run_dns_server(app_handle: AppHandle, state: Arc<DnsState>) -> Result<(), String> {
    let addr = format!("0.0.0.0:{}", DNS_PORT);
    let socket = UdpSocket::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind DNS socket to {}: {}", addr, e))?;

    log::info!("DNS server listening on {}", addr);

    // Wrap socket in Arc for use in spawned tasks
    let socket = Arc::new(socket);

    // Set up shutdown receiver for interrupting recv_from
    let shutdown_tx = state
        .shutdown_tx
        .lock()
        .unwrap()
        .take()
        .expect("shutdown_tx must be set before starting DNS server");
    let mut shutdown_rx = shutdown_tx.subscribe();

    let mut buf = vec![0u8; 512];

    while state.running.load(Ordering::SeqCst) {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                break;
            }
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((len, src)) => {
                        let app_handle = app_handle.clone();
                        let state = state.clone();
                        let socket = Arc::clone(&socket);
                        let buf_copy = buf.clone();

                        // Spawn task to handle this query (avoid blocking the loop)
                        tokio::spawn(async move {
                            handle_dns_query(&buf_copy, len, src, &socket, &app_handle, &state).await;
                        });
                    }
                    Err(e) => {
                        if state.running.load(Ordering::SeqCst) {
                            log::error!("DNS recv error: {}", e);
                        }
                    }
                }
            }
        }
    }

    log::info!("DNS server stopped");
    Ok(())
}

/// Start the DNS server and return the shared state.
pub fn start_dns_server(app_handle: AppHandle, state: Arc<DnsState>) {
    if state.running.swap(true, Ordering::SeqCst) {
        log::warn!("DNS server already running");
        return;
    }

    // Create shutdown channel for interrupting recv_from
    let (shutdown_tx, _shutdown_rx) = broadcast::channel(1);
    *state.shutdown_tx.lock().unwrap() = Some(shutdown_tx);

    let app_handle_clone = app_handle.clone();
    let state_clone = state.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_dns_server(app_handle_clone, state_clone).await {
            log::error!("DNS server error: {}", e);
        }
    });

    log::info!("DNS server started");
}

/// Stop the DNS server.
pub fn stop_dns_server(state: &Arc<DnsState>) {
    // Wake up the blocking recv_from call via shutdown channel
    if let Some(tx) = state.shutdown_tx.lock().unwrap().as_ref() {
        let _ = tx.send(());
    }
    state.running.store(false, Ordering::SeqCst);
    log::info!("DNS server stop signal sent");
}

/// Get the current DNS log entries.
#[tauri::command]
pub fn get_dns_log(state: State<'_, Arc<DnsState>>) -> Vec<DnsEntry> {
    let entries = state.entries.lock().unwrap();
    entries.iter().rev().take(50).cloned().collect()
}
