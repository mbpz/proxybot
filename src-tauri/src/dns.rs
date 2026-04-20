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

use crate::db::DbState;

/// DNS server listening port (pf redirects 53 -> 5300).
const DNS_PORT: u16 = 5300;
/// Upstream DNS server.
const UPSTREAM_DNS: &str = "8.8.8.8:53";
/// Maximum DNS entries to store.
const MAX_DNS_ENTRIES: usize = 10000;
/// Upstream query timeout.
const DNS_TIMEOUT_SECS: u64 = 3;

/// A single DNS query entry with app classification.
#[derive(Clone, serde::Serialize)]
pub struct DnsEntry {
    pub domain: String,
    pub timestamp_ms: u64,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
}

/// Shared DNS state.
pub struct DnsState {
    pub entries: Arc<Mutex<VecDeque<DnsEntry>>>,
    pub running: Arc<AtomicBool>,
    pub shutdown_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>,
    pub db_state: Option<Arc<DbState>>,
}

impl DnsState {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_DNS_ENTRIES))),
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
            db_state: None,
        }
    }

    pub fn with_db(db: Arc<DbState>) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_DNS_ENTRIES))),
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
            db_state: Some(db),
        }
    }

    /// Find the most recent DNS query matching the given host within a time window.
    /// Returns app_name and app_icon if found within the window.
    pub fn correlate_app(&self, host: &str, request_timestamp_ms: u64) -> Option<(String, String)> {
        // 5 second correlation window
        let window_ms = 5000u64;

        let entries = self.entries.lock().unwrap();

        // Find DNS queries within the window that match the host
        for entry in entries.iter().rev() {
            if request_timestamp_ms < entry.timestamp_ms {
                continue;
            }
            if request_timestamp_ms - entry.timestamp_ms > window_ms {
                break;
            }

            // Check if host matches the DNS query domain
            let domain = &entry.domain;
            if host == domain || host.ends_with(&format!(".{}", domain)) {
                if let (Some(name), Some(icon)) = (&entry.app_name, &entry.app_icon) {
                    return Some((name.clone(), icon.clone()));
                }
            }
        }

        None
    }
}

/// Get current timestamp in milliseconds since UNIX epoch.
fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Record a DNS query entry and emit a Tauri event.
fn record_query(
    state: &DnsState,
    domain: String,
    response_ips: &[String],
    app_handle: &AppHandle,
) {
    let timestamp_ms_val = timestamp_ms();

    // Classify the domain for app tagging
    let app_info = crate::app_rules::classify_host(&domain);
    let (app_name, app_icon) = app_info
        .map(|(n, i)| (Some(n), Some(i)))
        .unwrap_or((None, None));

    let entry = DnsEntry {
        domain: domain.clone(),
        timestamp_ms: timestamp_ms_val,
        app_name: app_name.clone(),
        app_icon: app_icon.clone(),
    };

    let mut entries = state.entries.lock().unwrap();
    if entries.len() >= MAX_DNS_ENTRIES {
        entries.pop_front();
    }
    entries.push_back(entry.clone());

    // Emit event to frontend
    let _ = app_handle.emit("dns-query", &entry);

    // Log to database if db_state is available
    if let Some(db) = &state.db_state {
        if let Ok(conn) = db.conn.lock() {
            let timestamp_str = chrono_lite_timestamp();
            let query_type = 1; // A record
            let response_ips_json = serde_json::to_string(response_ips).unwrap_or_else(|_| "[]".to_string());
            let app_tag = app_name.unwrap_or_else(|| "unknown".to_string());

            let _ = conn.execute(
                "INSERT INTO dns_queries (timestamp, query_name, query_type, response_ips, device_id, app_tag)
                 VALUES (?1, ?2, ?3, ?4, NULL, ?5)",
                rusqlite::params![timestamp_str, domain, query_type, response_ips_json, app_tag],
            );
        }
    }
}

/// Get a lightweight timestamp string for SQLite (YYYY-MM-DD HH:MM:SS).
fn chrono_lite_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Calculate year/month/day from epoch
    let mut year = 1970;
    let mut remaining_days = now as i64 / 86400;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year as i64 {
            break;
        }
        remaining_days -= days_in_year as i64;
        year += 1;
    }

    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days_in_month in days_in_months.iter() {
        if remaining_days < *days_in_month as i64 {
            break;
        }
        remaining_days -= *days_in_month as i64;
        month += 1;
    }

    let day = remaining_days + 1;
    let seconds_in_day = now as i64 % 86400;
    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;
    let seconds = seconds_in_day % 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year,
        month,
        day,
        hours,
        minutes,
        seconds
    )
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
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

/// Parse response IPs from a DNS response packet.
///
/// The DNS response format after the question section is:
/// - 2 bytes: query ID
/// - 2 bytes: flags
/// - 2 bytes: QDCOUNT (questions)
/// - 2 bytes: ANCOUNT (answer RRs)
/// ... then answer RRs contain A records with IP addresses
///
/// This extracts IPv4 addresses from A records in the response.
fn parse_response_ips(response: &[u8]) -> Vec<String> {
    let mut ips = Vec::new();

    // DNS header is 12 bytes
    if response.len() < 12 {
        return ips;
    }

    // Skip past the question section first
    // Start after header
    let mut pos = 12;

    // Skip QNAME in question section
    loop {
        if pos >= response.len() {
            return ips;
        }
        let label_len = response[pos];
        if label_len == 0 {
            pos += 1;
            break;
        }
        if label_len & 0xC0 != 0 {
            // Compression pointer - skip 2 bytes
            pos += 2;
            break;
        }
        pos += 1 + label_len as usize;
    }

    // Skip QTYPE (2 bytes) and QCLASS (2 bytes)
    pos += 4;
    if pos > response.len() {
        return ips;
    }

    // Now parse answer RRs
    // Each RR: name (compressed), type (2), class (2), TTL (4), rdlength (2), rdata
    while pos < response.len() - 12 {
        // Check for compression pointer at start of name
        if response[pos] & 0xC0 == 0xC0 {
            pos += 2;
        } else {
            // Skip the name
            loop {
                if pos >= response.len() {
                    return ips;
                }
                let label_len = response[pos];
                if label_len == 0 {
                    pos += 1;
                    break;
                }
                if label_len > 63 {
                    return ips;
                }
                pos += 1 + label_len as usize;
            }
        }

        // Need at least 10 more bytes for type, class, TTL, rdlength
        if pos + 10 > response.len() {
            break;
        }

        let rr_type = u16::from_be_bytes([response[pos], response[pos + 1]]);
        pos += 2; // type
        pos += 2; // class
        pos += 4; // TTL
        let rdlength = u16::from_be_bytes([response[pos], response[pos + 1]]);
        pos += 2;

        // A record: 4 bytes IP
        if rr_type == 1 && rdlength == 4 && pos + 4 <= response.len() {
            let ip = format!(
                "{}.{}.{}.{}",
                response[pos],
                response[pos + 1],
                response[pos + 2],
                response[pos + 3]
            );
            ips.push(ip);
        }

        pos += rdlength as usize;
    }

    ips
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

    log::debug!("DNS query from {} for domain: {}", src, domain);

    let mut response_ips: Vec<String> = Vec::new();

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
                    // Extract response IPs from the DNS response
                    response_ips = parse_response_ips(&response_buf[..resp_len]);

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

    // Record the query with response IPs
    record_query(state, domain, &response_ips, app_handle);
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
