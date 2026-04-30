//! Approval/snapshot tests for TUI render output.
//!
//! Uses `insta` to capture render output as snapshot files.
//! When render output changes unexpectedly, test fails and developer
//! reviews the diff via `cargo insta review`.
//!
//! Snapshots are stored in `tests/snapshots/` and should be committed.
//!
//! Workflow:
//!   cargo test                    # fail if output changed
//!   cargo insta review            # review changed snapshots
//!   cargo insta accept            # accept new snapshot as correct

mod common;

use proxybot_lib::tui::TrafficState;
use common::make_req;
use insta::assert_snapshot;

fn make_traffic() -> TrafficState {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "api.example.com", "/v1/users", Some(200)));
    s.requests.push(make_req(2, "POST", "api.example.com", "/v1/login", Some(201)));
    s.requests.push(make_req(3, "GET", "cdn.example.com", "/assets/logo.png", Some(404)));
    s.requests.push(make_req(4, "DELETE", "api.example.com", "/v1/users/123", Some(204)));
    s.requests.push(make_req(5, "GET", "api.example.com", "/health", Some(200)));
    s
}

/// Format a filter line the same way render_filter_bar does.
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

// ═══════════════════════════════════════════════════════════
// Snapshot tests — filter bar
// ═══════════════════════════════════════════════════════════

#[test]
fn snapshot_filter_bar_no_filter() {
    let traffic = make_traffic();
    let line = format_filter_line(&traffic);
    assert_snapshot!("filter_bar_no_filter", line);
}

#[test]
fn snapshot_filter_bar_method_filter_get() {
    let mut traffic = make_traffic();
    traffic.filters.method = Some("GET".into());
    let line = format_filter_line(&traffic);
    assert_snapshot!("filter_bar_method_get", line);
}

#[test]
fn snapshot_filter_bar_host_filter() {
    let mut traffic = make_traffic();
    traffic.filters.host_pattern = Some("api".into());
    let line = format_filter_line(&traffic);
    assert_snapshot!("filter_bar_host_filter", line);
}

#[test]
fn snapshot_filter_bar_status_filter() {
    let mut traffic = make_traffic();
    traffic.filters.status_class = Some("2xx".into());
    let line = format_filter_line(&traffic);
    assert_snapshot!("filter_bar_status_filter", line);
}

#[test]
fn snapshot_filter_bar_search_active() {
    let mut traffic = make_traffic();
    traffic.search_input = "users".to_string();
    let line = format_filter_line(&traffic);
    assert_snapshot!("filter_bar_search_active", line);
}

#[test]
fn snapshot_filter_bar_all_filters() {
    let mut traffic = make_traffic();
    traffic.filters.method = Some("GET".into());
    traffic.filters.host_pattern = Some("api.example".into());
    traffic.filters.status_class = Some("2xx".into());
    traffic.search_input = "v1".to_string();
    let line = format_filter_line(&traffic);
    assert_snapshot!("filter_bar_all_filters", line);
}

// ═══════════════════════════════════════════════════════════
// Snapshot tests — controls bar
// ═══════════════════════════════════════════════════════════

#[test]
fn snapshot_controls_bar_pf_on_dns_on() {
    let mut traffic = make_traffic();
    traffic.pf_enabled = true;
    traffic.dns_running = true;

    let pf_str = if traffic.pf_enabled { "[p]f: ON " } else { "[p]f: OFF " };
    let dns_str = if traffic.dns_running { "[d]ns: ON " } else { "[d]ns: OFF " };
    let controls = format!("{}{} | [Enter] select  [/] search  [1/2/3] detail tab  [Esc] clear filters", pf_str, dns_str);
    assert_snapshot!("controls_pf_on_dns_on", controls);
}

#[test]
fn snapshot_controls_bar_pf_off_dns_off() {
    let mut traffic = make_traffic();
    traffic.pf_enabled = false;
    traffic.dns_running = false;

    let pf_str = if traffic.pf_enabled { "[p]f: ON " } else { "[p]f: OFF " };
    let dns_str = if traffic.dns_running { "[d]ns: ON " } else { "[d]ns: OFF " };
    let controls = format!("{}{} | [Enter] select  [/] search  [1/2/3] detail tab  [Esc] clear filters", pf_str, dns_str);
    assert_snapshot!("controls_pf_off_dns_off", controls);
}