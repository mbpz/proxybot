//! Shared classification helpers for proxy request handling.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::app_rules;
use crate::dns::DnsState;

/// Run the standard proxy classification chain:
/// `app_rules::classify_host` (SNI/Host match) -> `dns_state.classify_connection` (DNS correlation).
/// Returns the app tag if any of the steps succeed.
pub fn classify_captured_request(
    host: &str,
    client_ip: &str,
    resolved_ip: Option<&str>,
    dns_state: &DnsState,
) -> Option<(String, String)> {
    let request_ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    app_rules::classify_host(host)
        .or_else(|| dns_state.classify_connection(host, client_ip, resolved_ip, request_ts_ms))
}
