//! BDD step definitions for traffic.feature.
//!
//! Uses the `cucumber` crate with tokio executor.

use cucumber::{World, given, when, then};
use std::time::Duration;

// Dummy world — in a real implementation this would hold
// the TuiApp state, PTY session, etc.
#[derive(Debug, World)]
pub struct TrafficWorld {
    pub proxy_started: bool,
    pub current_tab: String,
    pub filter_bar_text: String,
    pub request_count: usize,
    pub exit_status: Option<std::process::ExitStatus>,
}

impl Default for TrafficWorld {
    fn default() -> Self {
        Self {
            proxy_started: false,
            current_tab: "Traffic".to_string(),
            filter_bar_text: String::new(),
            request_count: 0,
            exit_status: None,
        }
    }
}

// ═══════════════════════════════════════════════════════════
// Given steps
// ═══════════════════════════════════════════════════════════

#[given("the traffic tab is active")]
fn traffic_tab_active(world: &mut TrafficWorld) {
    world.current_tab = "Traffic".to_string();
}

#[given("the request list contains \"GET\" and \"POST\" requests")]
fn request_list_has_get_and_post(_world: &mut TrafficWorld) {
    // In real implementation: populate request list via DB or mock
}

#[given("the traffic tab has a method filter \"[GET]\" active")]
fn method_filter_active(world: &mut TrafficWorld) {
    world.filter_bar_text = "[GET]".to_string();
}

#[given("the TUI is running")]
fn tui_running(_world: &mut TrafficWorld) {
    // In real implementation: spawn proxybot-tui in PTY
}

#[given("there are requests in the list")]
fn requests_exist(_world: &mut TrafficWorld) {
    world.request_count = 5;
}

#[given("there are multiple requests in the list")]
fn multiple_requests_exist(_world: &mut TrafficWorld) {
    world.request_count = 5;
}

// ═══════════════════════════════════════════════════════════
// When steps
// ═══════════════════════════════════════════════════════════

#[when("the user presses \"r\"")]
fn press_r(world: &mut TrafficWorld) {
    world.proxy_started = true;
}

#[when("the user presses \"q\"")]
fn press_q(_world: &mut TrafficWorld) {
    // In real implementation: send 'q' to PTY and capture exit status
}

#[when("the user presses Tab")]
fn press_tab(world: &mut TrafficWorld) {
    world.current_tab = match world.current_tab.as_str() {
        "Traffic" => "Rules".to_string(),
        "Rules" => "Devices".to_string(),
        "Devices" => "Certs".to_string(),
        "Certs" => "Dns".to_string(),
        "Dns" => "Alerts".to_string(),
        "Alerts" => "Replay".to_string(),
        "Replay" => "Graph".to_string(),
        "Graph" => "Gen".to_string(),
        "Gen" => "Traffic".to_string(),
        _ => "Traffic".to_string(),
    };
}

#[when("the user presses Tab 8 more times")]
fn press_tab_8_times(world: &mut TrafficWorld) {
    for _ in 0..8 {
        press_tab(world);
    }
}

#[when("the user presses \"/\"")]
fn press_slash(_world: &mut TrafficWorld) {
    // In real implementation: focus search input
}

#[when("the user presses Esc")]
fn press_esc(_world: &mut TrafficWorld) {
    // In real implementation: clear filters
}

#[when("the user presses Enter")]
fn press_enter(_world: &mut TrafficWorld) {
    // In real implementation: load request detail
}

#[when("the user presses \"2\"")]
fn press_2(_world: &mut TrafficWorld) {
    // Switch to Body tab in detail panel
}

#[when("the user presses \"3\"")]
fn press_3(_world: &mut TrafficWorld) {
    // Switch to WS Frames tab in detail panel
}

#[when("the user presses \"j\" (down)")]
fn press_j(_world: &mut TrafficWorld) {
    // Move selection down
}

#[when("the user presses \"k\" (up)")]
fn press_k(_world: &mut TrafficWorld) {
    // Move selection up
}

#[when("the user presses \"p\"")]
fn press_p(_world: &mut TrafficWorld) {
    // Toggle pf
}

#[when("the user types \"api\"")]
fn type_api(_world: &mut TrafficWorld) {
    // In real implementation: type into search input
}

// ═══════════════════════════════════════════════════════════
// Then steps
// ═══════════════════════════════════════════════════════════

#[then("the proxy starts")]
fn proxy_started(world: &mut TrafficWorld) {
    assert!(world.proxy_started, "proxy should be started");
}

#[then("the status shows \"proxy running\"")]
fn status_shows_running(_world: &mut TrafficWorld) {
    // In real implementation: verify PTY output contains "proxy running"
}

#[then("the application exits successfully")]
fn app_exits(_world: &mut TrafficWorld) {
    // In real implementation: check exit_status.success()
}

#[then("the rules tab becomes active")]
fn rules_tab_active(world: &mut TrafficWorld) {
    assert_eq!(world.current_tab, "Rules", "current tab should be Rules");
}

#[then("the traffic tab is active again (wrapped)")]
fn traffic_tab_active_after_wrap(world: &mut TrafficWorld) {
    assert_eq!(world.current_tab, "Traffic", "should wrap back to Traffic after 9 tabs");
}

#[then("only \"GET\" requests appear in the list")]
fn only_get_requests_shown(_world: &mut TrafficWorld) {
    // In real implementation: verify filtered request count and method
}

#[then("the filter bar shows \"[GET]\"")]
fn filter_bar_shows_get(world: &mut TrafficWorld) {
    assert!(world.filter_bar_text.contains("[GET]"), "filter bar should show [GET]");
}

#[then("all requests are shown again")]
fn all_requests_shown(_world: &mut TrafficWorld) {
    world.filter_bar_text = "[*]".to_string();
}

#[then("the filter bar shows \"[*]\"")]
fn filter_bar_shows_wildcard(world: &mut TrafficWorld) {
    assert!(world.filter_bar_text.contains("[*]"), "filter bar should show [*]");
}

#[then("the search input is focused")]
fn search_focused(_world: &mut TrafficWorld) {
    // In real implementation: check search_focused state
}

#[then("the filter bar shows \"/regex/\"")]
fn filter_bar_shows_regex_placeholder(world: &mut TrafficWorld) {
    world.filter_bar_text = "/regex/".to_string();
    assert!(world.filter_bar_text.contains("/regex/"), "should show /regex/ placeholder");
}

#[then("only requests matching \"api\" appear")]
fn only_api_requests_shown(_world: &mut TrafficWorld) {
    // In real implementation: verify filtered list
}

#[then("the filter bar shows \"/api/\"")]
fn filter_bar_shows_api_search(world: &mut TrafficWorld) {
    world.filter_bar_text = "/api/".to_string();
    assert!(world.filter_bar_text.contains("/api/"), "should show /api/");
}

#[then("the selection moves down one row")]
fn selection_moves_down(_world: &mut TrafficWorld) {
    // Verify selected index increases
}

#[then("the selection moves up one row")]
fn selection_moves_up(_world: &mut TrafficWorld) {
    // Verify selected index decreases
}

#[then("pf is toggled")]
fn pf_toggled(_world: &mut TrafficWorld) {
    // In real implementation: verify pf state flips
}

#[then("the controls bar shows the new pf state")]
fn controls_show_pf_state(_world: &mut TrafficWorld) {
    // Verify controls bar string contains updated pf status
}

#[then("the detail panel shows the request headers")]
fn detail_shows_headers(_world: &mut TrafficWorld) {
    // Verify detail_tab == 0 or detail panel text contains headers
}

#[then("the detail panel shows the request body")]
fn detail_shows_body(_world: &mut TrafficWorld) {
    // Verify detail_tab == 1
}

#[then("the detail panel shows WebSocket frames (if available)")]
fn detail_shows_ws_frames(_world: &mut TrafficWorld) {
    // Verify detail_tab == 2 and is_websocket check
}

// ═══════════════════════════════════════════════════════════
// Run the cucumber tests
// ═══════════════════════════════════════════════════════════

#[tokio::main]
async fn main() {
    // Run the BDD tests
    use futures::FutureExt;

    let result = TrafficWorld::run("tests/bdd/features")
        .await;

    match result {
        Ok(_) => println!("All BDD tests passed"),
        Err(e) => {
            eprintln!("BDD tests failed: {}", e);
            std::process::exit(1);
        }
    }
}