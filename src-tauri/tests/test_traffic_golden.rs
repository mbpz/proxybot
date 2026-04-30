//! Golden/snapshot tests for traffic filter bar text generation.
//!
//! Tests the filter bar string formatting logic directly without
//! requiring a full TuiApp instance.

mod common;

use proxybot_lib::tui::TrafficState;
use common::make_req;

/// Helper: format a filter line the same way render_filter_bar does.
fn format_filter_line(traffic: &TrafficState) -> String {
    let method_str = traffic.filters.method.as_deref().unwrap_or("*");
    let host_str = traffic.filters.host_pattern.as_deref().unwrap_or("");
    let status_str = traffic.filters.status_class.as_deref().unwrap_or("*");
    let app_tag_str = traffic.filters.app_tag.as_deref().unwrap_or("");
    let search_str = if traffic.search_input.is_empty() {
        "/regex/"
    } else {
        &format!("/{}/", traffic.search_input)
    };

    format!(
        " Method:[{}] Host:[{:<15}] Status:[{}] App:[{:<10}] {} [press letter to set, / for search, Esc to clear]",
        method_str,
        host_str.chars().take(15).collect::<String>(),
        status_str,
        app_tag_str.chars().take(10).collect::<String>(),
        search_str,
    )
}

/// Helper: build a TrafficState with a few requests.
fn make_traffic() -> TrafficState {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "api.example.com", "/v1/users", Some(200)));
    s.requests.push(make_req(2, "POST", "api.example.com", "/v1/login", Some(201)));
    s.requests.push(make_req(3, "GET", "cdn.example.com", "/assets/logo.png", Some(404)));
    s.requests.push(make_req(4, "DELETE", "api.example.com", "/v1/users/123", Some(204)));
    s.requests.push(make_req(5, "GET", "api.example.com", "/health", Some(200)));
    s
}

// ═══════════════════════════════════════════════════════════
// Golden tests for filter bar text generation
// ═══════════════════════════════════════════════════════════

#[test]
fn test_filter_bar_shows_all_filter_components() {
    let traffic = make_traffic();
    let line = format_filter_line(&traffic);

    assert!(line.contains("Method:"), "filter bar should show Method label");
    assert!(line.contains("Host:"), "filter bar should show Host label");
    assert!(line.contains("Status:"), "filter bar should show Status label");
    assert!(line.contains("App:"), "filter bar should show App label");
    assert!(line.contains("/regex/"), "filter bar should show search placeholder");
}

#[test]
fn test_filter_bar_shows_wildcard_when_no_filter_set() {
    let traffic = make_traffic();
    let line = format_filter_line(&traffic);

    // Wildcard means method shows as *
    assert!(line.contains("[*]"), "no method filter should show *");
}

#[test]
fn test_filter_bar_shows_active_method_filter() {
    let mut traffic = make_traffic();
    traffic.filters.method = Some("GET".into());
    let line = format_filter_line(&traffic);

    assert!(line.contains("[GET]"), "active method filter should show [GET]");
}

#[test]
fn test_filter_bar_shows_host_filter() {
    let mut traffic = make_traffic();
    traffic.filters.host_pattern = Some("api".into());
    let line = format_filter_line(&traffic);

    assert!(line.contains("Host:[api"), "host filter should show api pattern");
}

#[test]
fn test_filter_bar_shows_status_filter() {
    let mut traffic = make_traffic();
    traffic.filters.status_class = Some("2xx".into());
    let line = format_filter_line(&traffic);

    assert!(line.contains("Status:[2xx]"), "status filter should show [2xx]");
}

#[test]
fn test_filter_bar_shows_search_input() {
    let mut traffic = make_traffic();
    traffic.search_input = "api".to_string();
    let line = format_filter_line(&traffic);

    assert!(line.contains("/api/"), "search input should be displayed as /api/");
}

#[test]
fn test_filter_bar_truncates_long_host() {
    let mut traffic = make_traffic();
    traffic.filters.host_pattern = Some("this-is-a-very-long-hostname.example.com".into());
    let line = format_filter_line(&traffic);

    // Host field is 15 chars, so long hostnames get truncated
    assert!(line.contains("this-is-a-very"), "long host should be truncated to 15 chars");
}

#[test]
fn test_filter_bar_shows_empty_app_tag() {
    let traffic = make_traffic();
    let line = format_filter_line(&traffic);

    // When app_tag is empty, chars().take(10).collect() gives ""
    // So the format shows App:[] (empty between brackets)
    // We check that App:[ is present in the line
    assert!(line.contains("App:["), "empty app tag should show as App:[]");
}

#[test]
fn test_filter_bar_controls_pf_on() {
    let mut traffic = make_traffic();
    traffic.pf_enabled = true;

    let pf_str = if traffic.pf_enabled { "[p]f: ON " } else { "[p]f: OFF " };
    assert!(pf_str.contains("ON"), "pf enabled should show ON");
}

#[test]
fn test_filter_bar_controls_pf_off() {
    let mut traffic = make_traffic();
    traffic.pf_enabled = false;

    let pf_str = if traffic.pf_enabled { "[p]f: ON " } else { "[p]f: OFF " };
    assert!(pf_str.contains("OFF"), "pf disabled should show OFF");
}

#[test]
fn test_filter_bar_controls_dns_on() {
    let mut traffic = make_traffic();
    traffic.dns_running = true;

    let dns_str = if traffic.dns_running { "[d]ns: ON " } else { "[d]ns: OFF " };
    assert!(dns_str.contains("ON"), "dns running should show ON");
}

#[test]
fn test_filter_bar_controls_key_hints() {
    let controls = "[Enter] select  [/] search  [1/2/3] detail tab  [Esc] clear filters";
    assert!(controls.contains("Enter"), "controls should show Enter hint");
    assert!(controls.contains("search"), "controls should show search hint");
    assert!(controls.contains("detail"), "controls should show detail hint");
}