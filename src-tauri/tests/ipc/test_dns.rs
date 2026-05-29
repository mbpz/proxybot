//! Integration tests for DNS state and operations.

use proxybot_lib::dns::{DnsState, DnsUpstream, DnsUpstreamType};
use proxybot_lib::rules::RulesEngine;
use std::sync::Arc;

#[test]
fn test_dns_state_creation() {
    let dns = DnsState::new();
    let upstream = dns.get_upstream();
    // Default is DoH to Cloudflare
    assert!(upstream.address.contains("1.1.1.1"));
}

#[test]
fn test_dns_state_with_rules_engine() {
    let engine = Arc::new(RulesEngine::new());
    let dns = DnsState::new().with_rules_engine(engine);
    let upstream = dns.get_upstream();
    assert!(!upstream.address.is_empty());
}

#[test]
fn test_set_upstream_plain_udp() {
    let dns = DnsState::new();
    dns.set_upstream(DnsUpstream {
        upstream_type: DnsUpstreamType::PlainUdp,
        address: "8.8.8.8:53".to_string(),
    });
    let upstream = dns.get_upstream();
    assert_eq!(upstream.upstream_type, DnsUpstreamType::PlainUdp);
    assert_eq!(upstream.address, "8.8.8.8:53");
}

#[test]
fn test_set_upstream_doh() {
    let dns = DnsState::new();
    dns.set_upstream(DnsUpstream {
        upstream_type: DnsUpstreamType::Doh,
        address: "https://dns.google/dns-query".to_string(),
    });
    let upstream = dns.get_upstream();
    assert_eq!(upstream.upstream_type, DnsUpstreamType::Doh);
    assert!(upstream.address.contains("dns.google"));
}

#[test]
fn test_dns_upstream_default_is_doh() {
    let upstream = DnsUpstream::default();
    assert_eq!(upstream.upstream_type, DnsUpstreamType::Doh);
    assert!(upstream.address.contains("1.1.1.1"));
}

#[test]
fn test_correlate_app_no_entries() {
    let dns = DnsState::new();
    let result = dns.correlate_app("api.weixin.qq.com", 1000);
    // No DNS entries, so no correlation
    assert!(result.is_none());
}

#[test]
fn test_load_hosts_file_no_panic() {
    let dns = DnsState::new();
    // Should not panic even if hosts file doesn't exist
    dns.load_hosts_file();
}

#[test]
fn test_load_blocklist_no_panic() {
    let dns = DnsState::new();
    // Should not panic even if blocklist file doesn't exist
    dns.load_blocklist();
}

#[test]
fn test_dns_upstream_serialization() {
    let upstream = DnsUpstream {
        upstream_type: DnsUpstreamType::PlainUdp,
        address: "1.1.1.1:53".to_string(),
    };
    let json = serde_json::to_string(&upstream).unwrap();
    assert!(json.contains("plainudp"));
    assert!(json.contains("1.1.1.1:53"));
}
