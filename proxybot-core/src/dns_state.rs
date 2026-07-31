//! DNS state management — query tracking and correlation.
//!
//! Tracks DNS queries and responses for app identification
//! via DNS resolution pattern correlation.
//!
//! # Integration
//!
//! This module provides the data structures and query tracking.
//! The actual DNS server loop lives in `src-tauri/src/dns.rs`
//! because it depends on Tauri's async runtime and event system.

use crate::types::{DnsEntry, DnsUpstream, HostsEntry};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// A single DNS query record (internal).
#[derive(Debug, Clone)]
pub struct DnsQuery {
    pub domain: String,
    pub resolved_ip: Option<String>,
    pub timestamp: u64,
}

impl DnsQuery {
    pub fn new(domain: String) -> Self {
        Self {
            domain,
            resolved_ip: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn set_resolved(&mut self, ip: String) {
        self.resolved_ip = Some(ip);
    }
}

/// DNS state manager for tracking queries.
pub struct DnsState {
    /// Ring buffer of DNS log entries for display.
    pub entries: Mutex<VecDeque<DnsEntry>>,
    /// Whether the DNS server is running.
    pub running: AtomicBool,
    /// Map of client IP → DNS queries.
    queries_by_ip: Mutex<HashMap<String, Vec<DnsQuery>>>,
    /// Map of domain → IP addresses resolved.
    domains_by_ip: Mutex<HashMap<String, Vec<String>>>,
    /// DNS upstream configuration.
    pub upstream: Mutex<DnsUpstream>,
    /// Hosts file entries.
    pub hosts: Mutex<Vec<HostsEntry>>,
    /// Blocklist entries.
    pub blocklist: Mutex<Vec<String>>,
    /// Whether blocklist is enabled.
    pub blocklist_enabled: AtomicBool,
    /// Max entries in the ring buffer.
    max_entries: usize,
}

impl DnsState {
    /// Create a new DnsState with default capacity.
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(VecDeque::with_capacity(10000)),
            running: AtomicBool::new(false),
            queries_by_ip: Mutex::new(HashMap::new()),
            domains_by_ip: Mutex::new(HashMap::new()),
            upstream: Mutex::new(DnsUpstream::default()),
            hosts: Mutex::new(Vec::new()),
            blocklist: Mutex::new(Vec::new()),
            blocklist_enabled: AtomicBool::new(true),
            max_entries: 10000,
        }
    }

    /// Get current upstream configuration.
    pub fn get_upstream(&self) -> DnsUpstream {
        self.upstream.lock().unwrap().clone()
    }

    /// Set upstream configuration.
    pub fn set_upstream(&self, upstream: DnsUpstream) {
        *self.upstream.lock().unwrap() = upstream;
    }

    /// Record a DNS query from a client IP.
    pub fn record_query(&self, client_ip: &str, domain: String) {
        let query = DnsQuery::new(domain);
        let mut queries = self.queries_by_ip.lock().unwrap();
        queries
            .entry(client_ip.to_string())
            .or_default()
            .push(query);
    }

    /// Record a DNS response (IP resolved for domain).
    pub fn record_response(&self, domain: &str, ip: &str) {
        let mut domains = self.domains_by_ip.lock().unwrap();
        let entry = domains.entry(domain.to_string()).or_default();
        if !entry.contains(&ip.to_string()) {
            entry.push(ip.to_string());
        }

        let mut queries = self.queries_by_ip.lock().unwrap();
        for queries_list in queries.values_mut() {
            for query in queries_list.iter_mut() {
                if query.domain == domain && query.resolved_ip.is_none() {
                    query.set_resolved(ip.to_string());
                }
            }
        }
    }

    /// Get all domains queried from a specific IP.
    pub fn get_domains_for_ip(&self, ip: &str) -> Vec<String> {
        let queries = self.queries_by_ip.lock().unwrap();
        queries
            .get(ip)
            .map(|list| list.iter().map(|q| q.domain.clone()).collect())
            .unwrap_or_default()
    }

    /// Get all IPs a domain resolved to.
    pub fn get_ips_for_domain(&self, domain: &str) -> Vec<String> {
        let domains = self.domains_by_ip.lock().unwrap();
        domains.get(domain).cloned().unwrap_or_default()
    }

    /// Push a display entry into the ring buffer.
    pub fn push_entry(&self, entry: DnsEntry) {
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= self.max_entries {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    /// Get recent entries.
    pub fn get_entries(&self, limit: usize) -> Vec<DnsEntry> {
        let entries = self.entries.lock().unwrap();
        entries.iter().rev().take(limit).cloned().collect()
    }

    /// Clear all state.
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
        self.queries_by_ip.lock().unwrap().clear();
        self.domains_by_ip.lock().unwrap().clear();
    }

    /// Check if a domain is in the blocklist.
    pub fn is_blocked(&self, domain: &str) -> bool {
        if !self.blocklist_enabled.load(Ordering::SeqCst) {
            return false;
        }
        let blocklist = self.blocklist.lock().unwrap();
        blocklist
            .iter()
            .any(|entry| domain == entry.as_str() || domain.ends_with(&format!(".{}", entry)))
    }

    /// Check hosts file for a domain.
    pub fn lookup_hosts(&self, domain: &str) -> Option<String> {
        let hosts = self.hosts.lock().unwrap();
        hosts
            .iter()
            .find(|h| h.domain == domain)
            .map(|h| h.ip.clone())
    }
}

impl Default for DnsState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DnsUpstreamType;

    #[test]
    fn test_dns_state_new() {
        let state = DnsState::new();
        assert!(state.get_domains_for_ip("192.168.1.100").is_empty());
    }

    #[test]
    fn test_record_query_and_response() {
        let state = DnsState::new();
        state.record_query("192.168.1.100", "example.com".to_string());
        state.record_response("example.com", "93.184.216.34");
        assert_eq!(
            state.get_ips_for_domain("example.com"),
            vec!["93.184.216.34"]
        );
    }

    #[test]
    fn test_get_domains_for_ip() {
        let state = DnsState::new();
        state.record_query("192.168.1.100", "example.com".to_string());
        state.record_query("192.168.1.100", "test.com".to_string());
        let domains = state.get_domains_for_ip("192.168.1.100");
        assert_eq!(domains.len(), 2);
    }

    #[test]
    fn test_entry_ring_buffer() {
        let state = DnsState::new();
        state.push_entry(DnsEntry {
            domain: "example.com".to_string(),
            timestamp_ms: 1000,
            app_name: Some("Test".to_string()),
            app_icon: Some("T".to_string()),
            action: Some("DIRECT".to_string()),
            resolved_ips: vec!["1.2.3.4".to_string()],
        });
        let entries = state.get_entries(10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].domain, "example.com");
    }

    #[test]
    fn test_blocklist() {
        let state = DnsState::new();
        state.blocklist.lock().unwrap().push("evil.com".to_string());
        assert!(state.is_blocked("evil.com"));
        assert!(state.is_blocked("sub.evil.com"));
        assert!(!state.is_blocked("good.com"));
    }

    #[test]
    fn test_blocklist_disabled() {
        let state = DnsState::new();
        state.blocklist.lock().unwrap().push("evil.com".to_string());
        state
            .blocklist_enabled
            .store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(!state.is_blocked("evil.com"));
    }

    #[test]
    fn test_hosts_lookup() {
        let state = DnsState::new();
        state.hosts.lock().unwrap().push(HostsEntry {
            domain: "local.dev".to_string(),
            ip: "127.0.0.1".to_string(),
        });
        assert_eq!(
            state.lookup_hosts("local.dev"),
            Some("127.0.0.1".to_string())
        );
        assert_eq!(state.lookup_hosts("unknown.dev"), None);
    }

    #[test]
    fn test_upstream_config() {
        let state = DnsState::new();
        let upstream = state.get_upstream();
        assert_eq!(upstream.upstream_type, DnsUpstreamType::Doh);

        state.set_upstream(DnsUpstream {
            upstream_type: DnsUpstreamType::PlainUdp,
            address: "8.8.8.8:53".to_string(),
        });
        assert_eq!(
            state.get_upstream().upstream_type,
            DnsUpstreamType::PlainUdp
        );
    }
}
