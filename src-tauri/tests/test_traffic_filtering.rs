//! Integration tests for traffic filtering logic.

mod common;

use proxybot_lib::tui::TrafficState;
use common::make_req;

#[test]
fn test_filter_by_method() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "a.com", "/", Some(200)));
    s.requests.push(make_req(2, "POST", "b.com", "/", Some(200)));
    s.filters.method = Some("GET".into());
    assert_eq!(s.filtered_requests().len(), 1);
}

#[test]
fn test_filter_by_host_substring() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "api.example.com", "/", Some(200)));
    s.requests.push(make_req(2, "GET", "cdn.example.com", "/", Some(200)));
    s.filters.host_pattern = Some("api".into());
    assert_eq!(s.filtered_requests().len(), 1);
}

#[test]
fn test_filter_by_status_2xx() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "a.com", "/", Some(200)));
    s.requests.push(make_req(2, "GET", "a.com", "/", Some(404)));
    s.filters.status_class = Some("2xx".into());
    assert_eq!(s.filtered_requests().len(), 1);
}

#[test]
fn test_filter_by_status_4xx() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "a.com", "/", Some(200)));
    s.requests.push(make_req(2, "GET", "a.com", "/", Some(401)));
    s.requests.push(make_req(3, "GET", "a.com", "/", Some(404)));
    s.filters.status_class = Some("4xx".into());
    assert_eq!(s.filtered_requests().len(), 2);
}

#[test]
fn test_filter_combined_method_and_host() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "api.example.com", "/", Some(200)));
    s.requests.push(make_req(2, "POST", "api.example.com", "/", Some(200)));
    s.requests.push(make_req(3, "GET", "cdn.example.com", "/", Some(200)));
    s.filters.method = Some("GET".into());
    s.filters.host_pattern = Some("api".into());
    assert_eq!(s.filtered_requests().len(), 1);
}

#[test]
fn test_regex_search_matches_host() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "api.example.com", "/v1/users", Some(200)));
    s.requests.push(make_req(2, "GET", "cdn.example.com", "/static/app.js", Some(200)));
    s.search_regex = Some(regex::Regex::new("users").unwrap());
    assert_eq!(s.filtered_requests().len(), 1);
}

#[test]
fn test_regex_search_matches_path() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "api.example.com", "/v1/users", Some(200)));
    s.requests.push(make_req(2, "GET", "cdn.example.com", "/static/app.js", Some(200)));
    s.search_regex = Some(regex::Regex::new("static").unwrap());
    assert_eq!(s.filtered_requests().len(), 1);
}

#[test]
fn test_add_request_newest_first() {
    let mut s = TrafficState::default();
    s.add_request(&make_req(1, "GET", "a.com", "/", Some(200)));
    s.add_request(&make_req(2, "GET", "b.com", "/", Some(200)));
    assert_eq!(s.requests[0].id, 2);
    assert_eq!(s.requests[1].id, 1);
}

#[test]
fn test_add_request_limit() {
    let mut s = TrafficState::default();
    for i in 0..1005 {
        s.add_request(&make_req(i, "GET", "a.com", "/", Some(200)));
    }
    assert_eq!(s.requests.len(), 1000);
}

#[test]
fn test_clear_filters() {
    let mut s = TrafficState::default();
    s.filters.method = Some("GET".into());
    s.search_input = "test".into();
    s.clear_filters();
    assert!(s.filters.method.is_none());
    assert!(s.search_input.is_empty());
}

#[test]
fn test_filter_by_pending_status() {
    let mut s = TrafficState::default();
    s.requests.push(make_req(1, "GET", "a.com", "/", None)); // pending
    s.requests.push(make_req(2, "GET", "a.com", "/", Some(200)));
    s.filters.status_class = Some("pending".into());
    assert_eq!(s.filtered_requests().len(), 1);
}
