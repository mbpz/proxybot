//! DNS state management for tracking and correlating DNS queries
//! Used for app identification based on DNS resolution patterns

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// A single DNS query record
#[derive(Debug, Clone)]
pub struct DnsQuery {
    /// Domain name queried
    pub domain: String,
    /// IP address resolved to
    pub resolved_ip: Option<String>,
    /// Timestamp of query
    pub timestamp: u64,
}

impl DnsQuery {
    /// Create a new DNS query
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

    /// Set the resolved IP address
    pub fn set_resolved(&mut self, ip: String) {
        self.resolved_ip = Some(ip);
    }
}

/// DNS state manager for tracking queries
pub struct DnsState {
    /// Map of IP address -> list of DNS queries
    /// Tracks which domains were resolved from each IP
    queries_by_ip: HashMap<String, Vec<DnsQuery>>,
    /// Map of domain -> IP addresses it resolved to
    domains_by_ip: HashMap<String, Vec<String>>,
}

impl DnsState {
    /// Create a new DnsState
    pub fn new() -> Self {
        Self {
            queries_by_ip: HashMap::new(),
            domains_by_ip: HashMap::new(),
        }
    }

    /// Record a DNS query from a client IP
    pub fn record_query(&mut self, client_ip: &str, domain: String) -> DnsQuery {
        let query = DnsQuery::new(domain.clone());
        let entry = self.queries_by_ip.entry(client_ip.to_string()).or_insert_with(Vec::new);
        entry.push(query.clone());
        query
    }

    /// Record a DNS response (IP resolved for domain)
    pub fn record_response(&mut self, domain: &str, ip: &str) {
        // Update domain -> IP mapping
        let entry = self.domains_by_ip.entry(domain.to_string()).or_insert_with(Vec::new);
        if !entry.contains(&ip.to_string()) {
            entry.push(ip.to_string());
        }
        // Update all pending queries for this domain
        for (_, queries) in &mut self.queries_by_ip {
            for query in queries.iter_mut() {
                if query.domain == domain && query.resolved_ip.is_none() {
                    query.set_resolved(ip.to_string());
                }
            }
        }
    }

    /// Get all domains queried from a specific IP
    pub fn get_domains_for_ip(&self, ip: &str) -> Vec<&str> {
        self.queries_by_ip
            .get(ip)
            .map(|queries| queries.iter().map(|q| q.domain.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get all IPs a domain resolved to
    pub fn get_ips_for_domain(&self, domain: &str) -> Vec<&str> {
        self.domains_by_ip
            .get(domain)
            .map(|ips| ips.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Clear all DNS state
    pub fn clear(&mut self) {
        self.queries_by_ip.clear();
        self.domains_by_ip.clear();
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

    #[test]
    fn test_dns_state_new() {
        let state = DnsState::new();
        assert!(state.get_domains_for_ip("192.168.1.100").is_empty());
    }

    #[test]
    fn test_record_query() {
        let mut state = DnsState::new();
        let query = state.record_query("192.168.1.100", "example.com".to_string());
        assert_eq!(query.domain, "example.com");
        assert!(query.resolved_ip.is_none());
    }

    #[test]
    fn test_record_response() {
        let mut state = DnsState::new();
        state.record_query("192.168.1.100", "example.com".to_string());
        state.record_response("example.com", "93.184.216.34");
        assert_eq!(state.get_ips_for_domain("example.com"), vec!["93.184.216.34"]);
    }

    #[test]
    fn test_get_domains_for_ip() {
        let mut state = DnsState::new();
        state.record_query("192.168.1.100", "example.com".to_string());
        state.record_query("192.168.1.100", "test.com".to_string());
        let domains = state.get_domains_for_ip("192.168.1.100");
        assert_eq!(domains.len(), 2);
    }
}