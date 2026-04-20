//! Built-in DNS server for ProxyBot.
//!
//! Listens on UDP port 5300 (pf redirects 53->5300), parses DNS queries to extract
//! domain names, forwards all queries to configurable upstream DNS (plain UDP or DoH),
//! and relays responses back. Supports local hosts file, blocklist, and routing integration.

use std::collections::VecDeque;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};
use tauri::{AppHandle, Emitter, State};

use crate::db::DbState;
use crate::rules::RulesEngine;

/// DNS server listening port (pf redirects 53 -> 5300).
const DNS_PORT: u16 = 5300;
/// Default upstream DNS (plain UDP fallback).
const DEFAULT_UPSTREAM_DNS: &str = "8.8.8.8:53";
/// Default DoH upstream.
const DEFAULT_DOH_URL: &str = "https://1.1.1.1/dns-query";
/// Maximum DNS entries to store.
const MAX_DNS_ENTRIES: usize = 10000;
/// Upstream query timeout.
const DNS_TIMEOUT_SECS: u64 = 5;

/// DNS upstream protocol type.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DnsUpstreamType {
    PlainUdp,
    Doh,
}

/// DNS upstream configuration.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DnsUpstream {
    pub upstream_type: DnsUpstreamType,
    pub address: String,  // "8.8.8.8:53" for UDP, URL for DoH
}

impl Default for DnsUpstream {
    fn default() -> Self {
        // Default to DoH for secure DNS
        Self {
            upstream_type: DnsUpstreamType::Doh,
            address: DEFAULT_DOH_URL.to_string(),
        }
    }
}

/// A single DNS query entry with app classification and routing action.
#[derive(Clone, serde::Serialize)]
pub struct DnsEntry {
    pub domain: String,
    pub timestamp_ms: u64,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
    pub action: Option<String>,  // Routing action: DIRECT, PROXY, REJECT
    pub resolved_ips: Vec<String>,
}

/// Shared DNS state.
pub struct DnsState {
    pub entries: Arc<Mutex<VecDeque<DnsEntry>>>,
    pub running: Arc<AtomicBool>,
    pub shutdown_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>,
    pub db_state: Option<Arc<DbState>>,
    pub upstream: Arc<Mutex<DnsUpstream>>,
    pub hosts: Arc<Mutex<Vec<HostsEntry>>>,
    pub blocklist: Arc<Mutex<Vec<BlocklistEntry>>>,
    pub rules_engine: Option<Arc<RulesEngine>>,
}

/// A single hosts file entry (domain -> IP mapping).
#[derive(Clone, Debug)]
pub(crate) struct HostsEntry {
    domain: String,
    ip: String,
}

/// A single blocklist entry (domain pattern).
#[derive(Clone, Debug)]
pub(crate) struct BlocklistEntry {
    domain: String,  // Exact match or suffix with leading dot
}

impl DnsState {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(MAX_DNS_ENTRIES))),
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx: Arc::new(Mutex::new(None)),
            db_state: None,
            upstream: Arc::new(Mutex::new(DnsUpstream::default())),
            hosts: Arc::new(Mutex::new(Vec::new())),
            blocklist: Arc::new(Mutex::new(Vec::new())),
            rules_engine: None,
        }
    }

    pub fn with_db(db: Arc<DbState>) -> Self {
        let mut state = Self::new();
        state.db_state = Some(db);
        state
    }

    /// Set the rules engine for routing decisions.
    pub fn with_rules_engine(mut self, engine: Arc<RulesEngine>) -> Self {
        self.rules_engine = Some(engine);
        self
    }

    /// Load hosts file from ~/.proxybot/hosts.
    /// Format: "IPAddress DomainName" (same as /etc/hosts)
    pub fn load_hosts_file(&self) {
        let path = get_proxybot_dir().join("hosts");
        let mut entries = Vec::new();

        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                let line = line.trim();
                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Parse: IP domain
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let ip = parts[0].to_string();
                    let domain = parts[1].to_lowercase();
                    entries.push(HostsEntry { domain, ip });
                }
            }
            log::info!("Loaded {} hosts entries from {:?}", entries.len(), path);
        }

        *self.hosts.lock().unwrap() = entries;
    }

    /// Load blocklist from ~/.proxybot/blocklist.txt.
    /// Format: one domain per line (0.0.0.0 domain.com for hosts-style, or just domain.com)
    pub fn load_blocklist(&self) {
        let path = get_proxybot_dir().join("blocklist.txt");
        let mut entries = Vec::new();

        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                let line = line.trim();
                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Remove hosts-style prefix (0.0.0.0 or 127.0.0.1)
                let domain = if line.starts_with("0.0.0.0 ") {
                    line[8..].trim().to_lowercase()
                } else if line.starts_with("127.0.0.1 ") {
                    line[10..].trim().to_lowercase()
                } else {
                    line.to_lowercase()
                };

                if !domain.is_empty() {
                    entries.push(BlocklistEntry { domain });
                }
            }
            log::info!("Loaded {} blocklist entries from {:?}", entries.len(), path);
        }

        *self.blocklist.lock().unwrap() = entries;
    }

    /// Set the upstream DNS configuration.
    pub fn set_upstream(&self, upstream: DnsUpstream) {
        *self.upstream.lock().unwrap() = upstream;
    }

    /// Get current upstream configuration.
    pub fn get_upstream(&self) -> DnsUpstream {
        self.upstream.lock().unwrap().clone()
    }

    /// Check if a domain is in the blocklist.
    fn is_blocked(&self, domain: &str) -> bool {
        let blocklist = self.blocklist.lock().unwrap();
        let domain_lower = domain.to_lowercase();

        for entry in blocklist.iter() {
            // Exact match or suffix match (leading dot means suffix match)
            if entry.domain.starts_with('.') {
                // Suffix match: .example.com matches www.example.com
                let suffix = &entry.domain[1..];
                if domain_lower == suffix || domain_lower.ends_with(suffix) {
                    return true;
                }
            } else if domain_lower == entry.domain {
                return true;
            }
        }
        false
    }

    /// Check hosts file for a domain.
    /// Returns Some(ip) if found, None otherwise.
    fn check_hosts(&self, domain: &str) -> Option<String> {
        let hosts = self.hosts.lock().unwrap();
        let domain_lower = domain.to_lowercase();

        for entry in hosts.iter() {
            if domain_lower == entry.domain {
                return Some(entry.ip.clone());
            }
        }
        None
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

    /// Get routing action for a resolved domain.
    fn get_routing_action(&self, domain: &str) -> Option<String> {
        if let Some(engine) = &self.rules_engine {
            if let Some(action) = engine.match_host(domain, None) {
                return Some(action.to_string());
            }
        }
        None
    }
}

/// Get the ProxyBot config directory.
fn get_proxybot_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".proxybot")
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

    // Get routing action from rules engine
    let action = state.get_routing_action(&domain);

    let entry = DnsEntry {
        domain: domain.clone(),
        timestamp_ms: timestamp_ms_val,
        app_name: app_name.clone(),
        app_icon: app_icon.clone(),
        action: action.clone(),
        resolved_ips: response_ips.to_vec(),
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
fn parse_response_ips(response: &[u8]) -> Vec<String> {
    let mut ips = Vec::new();

    // DNS header is 12 bytes
    if response.len() < 12 {
        return ips;
    }

    // Skip past the question section first
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

/// Send DNS query via plain UDP.
async fn query_upstream_udp(
    query: &[u8],
    upstream: &str,
) -> Result<Vec<u8>, String> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

    socket
        .send_to(query, upstream)
        .await
        .map_err(|e| format!("Failed to send UDP query: {}", e))?;

    let mut response_buf = vec![0u8; 512];
    let (resp_len, _) = socket
        .recv_from(&mut response_buf)
        .await
        .map_err(|e| format!("Failed to receive UDP response: {}", e))?;

    response_buf.truncate(resp_len);
    Ok(response_buf)
}

/// Simple base64 encoding for DoH (URL-safe variant).
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0F) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3F] as char);
        } else {
            result.push('=');
        }
    }

    // URL-safe base64 variant
    result.replace('+', "-").replace('/', "_").replace('=', "")
}

/// Send DNS query via DoH (DNS over HTTPS) using reqwest.
async fn query_upstream_doh(
    query: &[u8],
    doh_url: &str,
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(DNS_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Encode query as base64url
    let query_b64 = base64_encode(query);

    // Build the DoH request URL with query parameter
    let url = if doh_url.contains('?') {
        format!("{}&dns={}", doh_url, query_b64)
    } else {
        format!("{}?dns={}", doh_url, query_b64)
    };

    let res = client
        .get(&url)
        .header("Accept", "application/dns-message")
        .header("User-Agent", "ProxyBot/1.0")
        .send()
        .await
        .map_err(|e| format!("DoH request failed: {}", e))?;

    let body = res
        .bytes()
        .await
        .map_err(|e| format!("Failed to read DoH response body: {}", e))?;

    Ok(body.to_vec())
}

/// Forward DNS query to upstream based on configuration.
async fn forward_dns_query(
    query: &[u8],
    upstream: &DnsUpstream,
) -> Result<Vec<u8>, String> {
    match upstream.upstream_type {
        DnsUpstreamType::PlainUdp => {
            query_upstream_udp(query, &upstream.address).await
        }
        DnsUpstreamType::Doh => {
            query_upstream_doh(query, &upstream.address).await
        }
    }
}

/// Build a DNS response with a single A record for blocked domains (0.0.0.0).
fn build_blocked_response(query: &[u8]) -> Vec<u8> {
    // Build a minimal DNS response with NXDOMAIN or 0.0.0.0
    // Since we can't easily construct a proper DNS message without trust-dns,
    // we'll construct a simple response manually

    if query.len() < 12 {
        return Vec::new();
    }

    let mut response = Vec::with_capacity(query.len() + 100);

    // Copy ID from query
    response.extend_from_slice(&query[0..2]);

    // Flags: QR=1 (response), AA=1, RA=1, RCODE=0 (No error)
    // byte 2: QR(1) | AA(1) | reserved(1) | RD(1) | RA(1) | reserved(1) | RCODE(3) = 0x84 (or 0x85 for NXDOMAIN)
    // Actually let's use simpler: 0x81 for standard response
    response.push(0x81);  // QR=1, AA=1, RD=1
    response.push(0x80);  // RA=1, RCODE=0

    // QDCOUNT: copy from query (usually 1)
    response.extend_from_slice(&query[4..6]);

    // ANCOUNT: 1 (we're adding one A record)
    response.push(0x00);
    response.push(0x01);

    // NSCOUNT: 0
    response.push(0x00);
    response.push(0x00);

    // ARCOUNT: 0
    response.push(0x00);
    response.push(0x00);

    // Question section: copy from query (after header)
    response.extend_from_slice(&query[12..]);

    // Answer section
    // Name: pointer to question name (0xC0 0x0C)
    response.push(0xC0);
    response.push(0x0C);

    // Type: A (1)
    response.push(0x00);
    response.push(0x01);

    // Class: IN (1)
    response.push(0x00);
    response.push(0x01);

    // TTL: 300 seconds
    response.push(0x00);
    response.push(0x00);
    response.push(0x01);
    response.push(0x2C);

    // RDLENGTH: 4
    response.push(0x00);
    response.push(0x04);

    // RDATA: 0.0.0.0 (blocked IP)
    response.push(0x00);
    response.push(0x00);
    response.push(0x00);
    response.push(0x00);

    response
}

/// Handle a single DNS query: parse domain, check hosts/blocklist, forward upstream, relay response.
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
    let upstream = state.get_upstream();

    // Check hosts file first
    if let Some(hosts_ip) = state.check_hosts(&domain) {
        log::debug!("Domain {} found in hosts file: {}", domain, hosts_ip);
        response_ips = vec![hosts_ip.clone()];

        // Build response with hosts IP
        let response = build_hosts_response(data, &hosts_ip);

        if let Err(e) = socket.send_to(&response, src).await {
            log::error!("Failed to send DNS response to {}: {}", src, e);
        }

        // Record the query with hosts IP
        record_query(state, domain, &response_ips, app_handle);
        return;
    }

    // Check blocklist
    if state.is_blocked(&domain) {
        log::debug!("Domain {} is blocked", domain);

        // Build blocked response (0.0.0.0)
        let response = build_blocked_response(data);

        if let Err(e) = socket.send_to(&response, src).await {
            log::error!("Failed to send DNS response to {}: {}", src, e);
        }

        // Record as blocked (empty response)
        record_query(state, domain, &[], app_handle);
        return;
    }

    // Forward to upstream DNS
    match timeout(
        Duration::from_secs(DNS_TIMEOUT_SECS),
        forward_dns_query(data, &upstream),
    )
    .await
    {
        Ok(Ok(response_data)) => {
            // Extract response IPs from the DNS response
            response_ips = parse_response_ips(&response_data);

            // Send response back to client
            if let Err(e) = socket.send_to(&response_data, src).await {
                log::error!("Failed to send DNS response to {}: {}", src, e);
            }
        }
        Ok(Err(e)) => {
            log::error!("DNS upstream error for {}: {}", domain, e);
            // Try fallback to plain UDP on DoH failure
            if upstream.upstream_type != DnsUpstreamType::PlainUdp {
                log::info!("Trying fallback to plain UDP for {}", domain);
                let fallback = DnsUpstream {
                    upstream_type: DnsUpstreamType::PlainUdp,
                    address: DEFAULT_UPSTREAM_DNS.to_string(),
                };
                match timeout(
                    Duration::from_secs(DNS_TIMEOUT_SECS),
                    forward_dns_query(data, &fallback),
                )
                .await
                {
                    Ok(Ok(response_data)) => {
                        response_ips = parse_response_ips(&response_data);
                        if let Err(e) = socket.send_to(&response_data, src).await {
                            log::error!("Failed to send DNS response to {}: {}", src, e);
                        }
                    }
                    _ => {
                        log::error!("Fallback to plain UDP also failed for {}", domain);
                    }
                }
            }
        }
        Err(_) => {
            log::warn!("DNS upstream response timed out for {}", domain);
        }
    }

    // Record the query with response IPs
    record_query(state, domain, &response_ips, app_handle);
}

/// Build a DNS response with a hosts file IP.
fn build_hosts_response(query: &[u8], ip: &str) -> Vec<u8> {
    // Parse the IP
    let ip_parts: Vec<u8> = ip
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    if ip_parts.len() != 4 {
        // Invalid IP, return empty response
        return Vec::new();
    }

    if query.len() < 12 {
        return Vec::new();
    }

    let mut response = Vec::with_capacity(query.len() + 100);

    // Copy ID from query
    response.extend_from_slice(&query[0..2]);

    // Flags: QR=1 (response), AA=1, RA=1, RCODE=0
    response.push(0x81);  // QR=1, AA=1, RD=1
    response.push(0x80);  // RA=1, RCODE=0

    // QDCOUNT: copy from query (usually 1)
    response.extend_from_slice(&query[4..6]);

    // ANCOUNT: 1
    response.push(0x00);
    response.push(0x01);

    // NSCOUNT: 0
    response.push(0x00);
    response.push(0x00);

    // ARCOUNT: 0
    response.push(0x00);
    response.push(0x00);

    // Question section: copy from query (after header)
    response.extend_from_slice(&query[12..]);

    // Answer section
    // Name: pointer to question name (0xC0 0x0C)
    response.push(0xC0);
    response.push(0x0C);

    // Type: A (1)
    response.push(0x00);
    response.push(0x01);

    // Class: IN (1)
    response.push(0x00);
    response.push(0x01);

    // TTL: 300 seconds
    response.push(0x00);
    response.push(0x00);
    response.push(0x01);
    response.push(0x2C);

    // RDLENGTH: 4
    response.push(0x00);
    response.push(0x04);

    // RDATA: IP address
    response.extend_from_slice(&ip_parts);

    response
}

/// Run the DNS server loop.
async fn run_dns_server(app_handle: AppHandle, state: Arc<DnsState>) -> Result<(), String> {
    let addr = format!("0.0.0.0:{}", DNS_PORT);
    let socket = UdpSocket::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind DNS socket to {}: {}", addr, e))?;

    log::info!("DNS server listening on {}", addr);

    // Load hosts file and blocklist
    state.load_hosts_file();
    state.load_blocklist();

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

/// Get current DNS upstream configuration.
#[tauri::command]
pub fn get_dns_upstream(state: State<'_, Arc<DnsState>>) -> DnsUpstream {
    state.get_upstream()
}

/// Set DNS upstream configuration.
#[tauri::command]
pub fn set_dns_upstream(state: State<'_, Arc<DnsState>>, upstream: DnsUpstream) -> Result<(), String> {
    // Validate upstream
    match upstream.upstream_type {
        DnsUpstreamType::PlainUdp => {
            // Plain UDP address should be host:port
            if !upstream.address.contains(':') {
                return Err("Plain UDP upstream must be in format 'host:port'".to_string());
            }
        }
        DnsUpstreamType::Doh => {
            // DoH URL should start with https://
            if !upstream.address.starts_with("https://") {
                return Err("DoH URL must start with https://".to_string());
            }
        }
    }

    state.set_upstream(upstream);
    log::info!("DNS upstream configuration updated");
    Ok(())
}

/// Reload hosts file and blocklist from disk.
#[tauri::command]
pub fn reload_dns_lists(state: State<'_, Arc<DnsState>>) {
    state.load_hosts_file();
    state.load_blocklist();
    log::info!("DNS hosts and blocklist reloaded");
}
