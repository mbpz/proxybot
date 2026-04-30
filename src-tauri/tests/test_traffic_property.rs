//! Property-based tests for traffic filter logic.
//!
//! Uses proptest to test filter invariants across a wide range of inputs.

mod common;

use common::make_req;
use proxybot_lib::tui::TrafficState;
use regex;
use proptest::prelude::*;

// ═══════════════════════════════════════════════════════════
// Helper: build a TrafficState from generated components
// ═══════════════════════════════════════════════════════════

fn make_traffic_with(
    requests: Vec<(i64, &'static str, &'static str, Option<u16>)>,
    method_filter: Option<String>,
    host_filter: Option<String>,
    status_filter: Option<String>,
    app_tag_filter: Option<String>,
    search_input: String,
) -> TrafficState {
    let mut s = TrafficState::default();
    for (id, method, host, status) in requests {
        s.requests.push(make_req(id, method, host, "/", status));
    }
    s.filters.method = method_filter;
    s.filters.host_pattern = host_filter;
    s.filters.status_class = status_filter;
    s.filters.app_tag = app_tag_filter;
    s.search_input = search_input;
    s
}

// ═══════════════════════════════════════════════════════════
// Property-based test helpers (without prop_compose)
// ═══════════════════════════════════════════════════════════

/// Setting a filter can only decrease (not increase) the result count.
fn filter_decreases_count(s: &TrafficState) -> bool {
    let unfiltered = s.requests.len();
    let filtered = s.filtered_requests().len();
    filtered <= unfiltered
}

// ═══════════════════════════════════════════════════════════
// Property tests using proptest!
// ═══════════════════════════════════════════════════════════

#[test]
fn property_filter_count_never_exceeds_input() {
    // Test with a variety of request counts
    proptest!(|(req_count in 0..100u8)| {
        let requests: Vec<_> = (0..req_count as i64).map(|i| (i, "GET", "example.com", Some(200))).collect();
        let s = make_traffic_with(requests, None, None, None, None, String::new());
        // filtered should never exceed total requests
        prop_assert!(s.filtered_requests().len() <= s.requests.len());
    });
}

#[test]
fn property_filtered_always_newest_first() {
    proptest!(|(req_count in 1..50u8)| {
        let requests: Vec<_> = (0..req_count as i64).map(|i| (i, "GET", "example.com", Some(200))).collect();
        let s = make_traffic_with(requests, None, None, None, None, String::new());
        let filtered = s.filtered_requests();
        if filtered.len() > 1 {
            // Check newest-first: each id should be greater than the next
            let mut ok = true;
            for i in 0..filtered.len() - 1 {
                if filtered[i].id <= filtered[i + 1].id {
                    ok = false;
                    break;
                }
            }
            prop_assert!(ok, "filtered should be sorted newest-first");
        }
    });
}

#[test]
fn property_no_filters_returns_all() {
    proptest!(|(req_count in 0..30u8)| {
        let requests: Vec<_> = (0..req_count as i64).map(|i| (i, "GET", "example.com", Some(200))).collect();
        let s = make_traffic_with(requests, None, None, None, None, String::new());
        prop_assert_eq!(s.filtered_requests().len(), s.requests.len());
    });
}

#[test]
fn property_filter_count_decreases() {
    proptest!(|(req_count in 0..50u8, has_method_filter in 0..2u8)| {
        let method_filter = if has_method_filter == 1 { Some("GET".to_string()) } else { None };
        let requests: Vec<_> = (0..req_count as i64).map(|i| (i, "GET", "example.com", Some(200))).collect();
        let s = make_traffic_with(requests, method_filter, None, None, None, String::new());
        prop_assert!(filter_decreases_count(&s));
    });
}

// ═══════════════════════════════════════════════════════════
// Additional unit-style property tests
// ═══════════════════════════════════════════════════════════

#[test]
fn property_method_filter_exact_match() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "a.com", "/", Some(200)));
    s.requests.push(make_req(2, "POST", "a.com", "/", Some(200)));
    s.requests.push(make_req(3, "GET", "a.com", "/", Some(200)));

    s.filters.method = Some("GET".into());
    let filtered = s.filtered_requests();
    assert_eq!(filtered.len(), 2);
    for req in filtered {
        assert_eq!(req.method, "GET");
    }
}

#[test]
fn property_host_filter_substring_match() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "api.example.com", "/", Some(200)));
    s.requests.push(make_req(2, "GET", "cdn.example.com", "/", Some(200)));
    s.requests.push(make_req(3, "GET", "static.example.com", "/", Some(200)));

    s.filters.host_pattern = Some("api".into());
    let filtered = s.filtered_requests();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].host, "api.example.com");
}

#[test]
fn property_status_filter_2xx() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "a.com", "/", Some(200)));
    s.requests.push(make_req(2, "GET", "a.com", "/", Some(404)));
    s.requests.push(make_req(3, "GET", "a.com", "/", Some(500)));

    s.filters.status_class = Some("2xx".into());
    let filtered = s.filtered_requests();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].status, Some(200));
}

#[test]
fn property_empty_requests_returns_empty() {
    let s = TrafficState::default();
    assert_eq!(s.filtered_requests().len(), 0);
}

#[test]
fn property_large_request_count_handled() {
    let mut s = TrafficState::default();
    for i in 0..1000i64 {
        s.requests.push(make_req(i, "GET", "a.com", "/", Some(200)));
    }
    let filtered = s.filtered_requests();
    assert_eq!(filtered.len(), 1000);
    assert_eq!(filtered[0].id, 999); // newest first
}

#[test]
fn property_status_filter_handles_missing_status() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "a.com", "/", Some(200)));
    s.requests.push(make_req(2, "GET", "a.com", "/", None)); // pending

    s.filters.status_class = Some("2xx".into());
    let filtered = s.filtered_requests();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].status, Some(200));
}

#[test]
fn property_multiple_filters_combine() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "api.example.com", "/", Some(200)));
    s.requests.push(make_req(2, "GET", "api.example.com", "/", Some(404)));
    s.requests.push(make_req(3, "POST", "api.example.com", "/", Some(200)));
    s.requests.push(make_req(4, "GET", "cdn.example.com", "/", Some(200)));

    s.filters.method = Some("GET".into());
    s.filters.status_class = Some("2xx".into());
    let filtered = s.filtered_requests();
    // Only GET + 2xx
    assert_eq!(filtered.len(), 2);
    for req in filtered {
        assert_eq!(req.method, "GET");
        assert_eq!(req.status, Some(200));
    }
}

#[test]
fn property_search_filters_by_host_and_path() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "api.example.com", "/v1/users", Some(200)));
    s.requests.push(make_req(2, "GET", "api.example.com", "/v1/login", Some(200)));
    s.requests.push(make_req(3, "GET", "other.com", "/v1/users", Some(200)));

    // Set search_input AND search_regex (as the UI would)
    s.search_input = "users".to_string();
    s.search_regex = Some(regex::Regex::new("users").unwrap());
    let filtered = s.filtered_requests();
    // Should match requests where "users" appears in host or path
    assert_eq!(filtered.len(), 2);
}