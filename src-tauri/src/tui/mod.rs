//! TUI module for ProxyBot - Terminal UI for the HTTPS MITM proxy.
//!
//! Provides a multi-tab terminal interface for monitoring and controlling
//! the proxy, viewing traffic, managing rules, devices, certificates, and DNS.

pub mod input;
pub mod render;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use regex::Regex;

// Import subsystem types from lib (same crate via proxybot_lib alias)
use crate::cert::CertManager;
use crate::db::{DbState, RecentRequest};
use crate::dns::DnsState;
use crate::proxy::InterceptedRequest;
use crate::proxy::ProxyState;
use crate::proxy::BreakpointDecision;
use crate::rules::RulesEngine;
use crate::anomaly::AnomalyDetector;
use crate::tun::TunState;
use crate::replay::ReplayState;

/// Filter configuration for traffic list.
#[derive(Default)]
pub struct TrafficFilters {
    pub method: Option<String>,        // GET, POST, PUT, DELETE, etc.
    pub host_pattern: Option<String>,  // substring match
    pub status_class: Option<String>,  // "2xx", "3xx", "4xx", "5xx"
    pub app_tag: Option<String>,       // app name filter
}

/// Which filter field is being edited in filter input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Method,
    Host,
    Status,
    AppTag,
}

/// Traffic tab state.
#[derive(Default)]
pub struct TrafficState {
    pub requests: Vec<RecentRequest>,
    pub selected: usize,
    pub last_id: i64,
    // Filters
    pub filters: TrafficFilters,
    // Regex search across host + path
    pub search_regex: Option<Regex>,
    pub search_input: String,
    pub search_focused: bool,
    // Filter input mode (m/f/o/a key → enter mode → type → Enter to confirm)
    pub filter_mode: Option<FilterMode>,
    pub filter_input: String,
    // Detail panel
    pub detail_request: Option<InterceptedRequest>,
    pub detail_loading: bool,
    // Scroll offset for detail panel
    pub detail_scroll: Option<u64>,
    // Detail sub-tabs: Headers / Body / WS Frames
    pub detail_tab: usize,
    // Loading animation frame counter
    pub loading_frame: usize,
    // pf/DNS status
    pub pf_enabled: bool,
    pub dns_running: bool,
    // Breakpoint state
    pub breakpoint: BreakpointState,
}

/// Breakpoint type.
#[derive(Clone, PartialEq, Eq)]
pub enum BreakpointType {
    Request,
    Response,
}

/// Breakpoint mode in the TUI.
#[derive(Clone, PartialEq, Eq, Default)]
pub enum BreakpointMode {
    #[default]
    None,
    RequestPaused,
    ResponsePaused,
}

/// Breakpoint 编辑模式
#[derive(Clone, PartialEq, Eq)]
pub enum BreakpointEditMode {
    None,           // 非编辑模式（正常查看）
    Editing(usize), // 编辑模式（usize = 选中字段索引）
}

/// 可编辑的字段类型
#[derive(Clone, PartialEq, Eq)]
pub enum BreakpointField {
    Method,   // 索引 0
    Url,      // 索引 1
    Headers,  // 索引 2
    Body,     // 索引 3
}

/// Breakpoint state for managing paused requests.
pub struct BreakpointState {
    pub mode: BreakpointMode,
    pub edit_mode: BreakpointEditMode,              // 新增
    pub selected_field: BreakpointField,           // 新增
    pub editing_header_index: Option<usize>,       // 新增：正在编辑的 header 行索引
    pub header_input: String,                      // 新增
    pub body_input: String,                         // 新增
    pub url_input: String,                          // 新增
    pub method_input: String,                       // 新增
    // 现有字段
    pub queue: Vec<crate::proxy::InterceptedRequest>,
    pub current_edit: Option<crate::proxy::InterceptedRequest>,
    pub current_index: usize,
}

impl Default for BreakpointState {
    fn default() -> Self {
        Self {
            mode: BreakpointMode::default(),
            edit_mode: BreakpointEditMode::None,
            selected_field: BreakpointField::Method,
            editing_header_index: None,
            header_input: String::new(),
            body_input: String::new(),
            url_input: String::new(),
            method_input: String::new(),
            queue: Vec::new(),
            current_edit: None,
            current_index: 0,
        }
    }
}

impl TrafficState {
    pub fn add_request(&mut self, req: &RecentRequest) {
        self.requests.insert(0, req.clone());
        if self.requests.len() > 1000 {
            self.requests.pop();
        }
    }

    /// Returns filtered+searched requests.
    pub fn filtered_requests(&self) -> Vec<&RecentRequest> {
        let mut results: Vec<&RecentRequest> = self.requests
            .iter()
            .filter(|req| {
                // Method filter
                if let Some(ref m) = self.filters.method {
                    if &req.method != m {
                        return false;
                    }
                }
                // Host filter (substring)
                if let Some(ref h) = self.filters.host_pattern {
                    if !req.host.to_lowercase().contains(&h.to_lowercase()) {
                        return false;
                    }
                }
                // Status class filter
                if let Some(ref sc) = self.filters.status_class {
                    let sc_str = sc.as_str();
                    let status = match req.status {
                        Some(s) => s,
                        None => {
                            if sc_str == "pending" {
                                return true;
                            }
                            return false;
                        }
                    };
                    let matches = match sc_str {
                        "2xx" => (200..=299).contains(&status),
                        "3xx" => (300..=399).contains(&status),
                        "4xx" => (400..=499).contains(&status),
                        "5xx" => (500..=599).contains(&status),
                        _ => false,
                    };
                    if !matches {
                        return false;
                    }
                }
                // App tag filter
                if let Some(ref a) = self.filters.app_tag {
                    if req.app_tag.as_deref() != Some(a.as_str()) {
                        return false;
                    }
                }
                // Regex search
                if let Some(ref re) = self.search_regex {
                    let target = format!("{} {}", req.host, req.path);
                    if !re.is_match(&target) {
                        return false;
                    }
                }
                true
            })
            .collect::<Vec<_>>();
        // Sort newest-first (highest id first)
        results.sort_by_key(|r| std::cmp::Reverse(r.id));
        results
    }

    /// Clear all filters and search.
    pub fn clear_filters(&mut self) {
        self.filters = TrafficFilters::default();
        self.search_regex = None;
        self.search_input.clear();
        self.search_focused = false;
        self.detail_request = None;
    }
}

/// Tab enumeration for the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Traffic,
    Rules,
    Devices,
    Certs,
    Dns,
    Alerts,
    Replay,
    Graph,
    Gen,
}

impl Tab {
    /// All tabs in display order.
    pub const ALL: [Tab; 9] = [
        Tab::Traffic,
        Tab::Rules,
        Tab::Devices,
        Tab::Certs,
        Tab::Dns,
        Tab::Alerts,
        Tab::Replay,
        Tab::Graph,
        Tab::Gen,
    ];

    /// Label for display in the tab bar.
    pub fn label(&self) -> &'static str {
        match self {
            Tab::Traffic => "Traffic",
            Tab::Rules => "Rules",
            Tab::Devices => "Devices",
            Tab::Certs => "Certs",
            Tab::Dns => "DNS",
            Tab::Alerts => "Alerts",
            Tab::Replay => "Replay",
            Tab::Graph => "Graph",
            Tab::Gen => "Gen",
        }
    }

    /// Next tab (wraps around).
    pub fn next(&self) -> Tab {
        match self {
            Tab::Traffic => Tab::Rules,
            Tab::Rules => Tab::Devices,
            Tab::Devices => Tab::Certs,
            Tab::Certs => Tab::Dns,
            Tab::Dns => Tab::Alerts,
            Tab::Alerts => Tab::Replay,
            Tab::Replay => Tab::Graph,
            Tab::Graph => Tab::Gen,
            Tab::Gen => Tab::Traffic,
        }
    }

    /// Previous tab (wraps around).
    pub fn prev(&self) -> Tab {
        match self {
            Tab::Traffic => Tab::Gen,
            Tab::Rules => Tab::Traffic,
            Tab::Devices => Tab::Rules,
            Tab::Certs => Tab::Devices,
            Tab::Dns => Tab::Certs,
            Tab::Alerts => Tab::Dns,
            Tab::Replay => Tab::Alerts,
            Tab::Graph => Tab::Replay,
            Tab::Gen => Tab::Graph,
        }
    }
}

/// Devices tab state.
#[derive(Default)]
pub struct DevicesState {
    pub selected: usize,
    pub selected_override: Option<usize>,
    /// Whether we're editing the rule override for selected device
    pub editing_override: bool,
    /// Rule override input buffer
    pub override_input: String,
}

/// Rules tab state.
#[derive(Default)]
pub struct RulesState {
    pub selected: usize,
    /// Modal open for add/edit
    pub modal_open: bool,
    /// "add" or "edit"
    pub modal_mode: String,
    /// Buffer for editing fields: (name, pattern, action)
    pub edit_buffer: (String, String, String),
    /// Hot-reload watcher active
    pub watcher_active: bool,
}

/// Certs tab state.
#[derive(Default)]
pub struct CertsState {
    pub selected: usize,
    /// Regenerate button feedback
    pub regenerate_status: Option<String>,
    /// Last exported path
    pub export_path: Option<String>,
}

/// DNS tab state.
#[derive(Default)]
pub struct DnsTabState {
    pub selected: usize,
    /// Number of hosts entries
    pub hosts_count: usize,
    /// Number of blocklist entries
    pub blocklist_count: usize,
}

/// Alerts tab state.
#[derive(Default)]
pub struct AlertsState {
    pub selected: usize,
    pub alerts_list: Vec<crate::anomaly::Alert>,
    pub baseline_info: Option<crate::anomaly::TrafficBaseline>,
}

/// Replay tab state.
#[derive(Default)]
pub struct ReplayState2 {
    pub selected: usize,
    pub targets_list: Vec<crate::replay::ReplayTarget>,
    pub diff_output: Option<String>,
    pub har_export_status: Option<String>,
}

/// Graph tab view type.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum GraphViewType {
    #[default]
    Dag,
    AuthStateMachine,
}

/// Graph tab state.
#[derive(Default)]
pub struct GraphState {
    pub selected: usize,
    /// Current view type: DAG or Auth state machine
    pub view_type: GraphViewType,
    /// Lines for the DAG ASCII art
    pub dag_lines: Vec<String>,
    /// Lines for the auth state machine ASCII art
    pub state_machine_lines: Vec<String>,
    /// Auth state machine states found
    pub auth_states: Vec<String>,
    /// Auth state machine transitions
    pub auth_transitions: Vec<(String, String)>,
}

/// Generation mode for the Gen tab.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum GenMode {
    #[default]
    Mock,
    Frontend,
    Docker,
}

/// Gen (Scaffold/Generate) tab state.
#[derive(Default)]
pub struct GenState {
    pub selected: usize,
    /// Current generation mode
    pub gen_mode: GenMode,
    /// Progress/output lines from generation
    pub progress_output: Vec<String>,
    /// Last output path from generation
    pub output_path: Option<String>,
    /// Whether generation is in progress
    pub is_generating: bool,
}

/// Main TUI application state.
pub struct TuiApp {
    // Subsystems (shared Arc types from lib.rs)
    pub db_state: Arc<DbState>,
    pub cert_manager: Arc<CertManager>,
    pub rules_engine: Arc<RulesEngine>,
    pub dns_state: Arc<DnsState>,
    pub proxy_state: Arc<ProxyState>,
    pub anomaly_detector: Arc<AnomalyDetector>,
    pub tun_state: Arc<TunState>,
    pub replay_state: Arc<ReplayState>,

    // Proxy runtime
    pub proxy_running: AtomicBool,
    pub shutdown_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    /// Sender for breakpoint decisions (proceed/drop) back to proxy
    pub breakpoint_decision_tx: Mutex<Option<tokio::sync::oneshot::Sender<BreakpointDecision>>>,
    /// Available update version (if newer than current)
    pub update_available: Arc<std::sync::Mutex<Option<String>>>,

    // UI state
    pub current_tab: Tab,
    pub traffic: TrafficState,
    pub devices: DevicesState,
    pub rules: RulesState,
    pub certs: CertsState,
    pub dns: DnsTabState,
    pub alerts: AlertsState,
    pub replay: ReplayState2,
    pub graph: GraphState,
    pub gen: GenState,
}

impl TuiApp {
    /// Create a new TuiApp with all subsystem handles.
    pub fn new(
        db_state: Arc<DbState>,
        cert_manager: Arc<CertManager>,
        rules_engine: Arc<RulesEngine>,
        dns_state: Arc<DnsState>,
        proxy_state: Arc<ProxyState>,
        anomaly_detector: Arc<AnomalyDetector>,
        tun_state: Arc<TunState>,
        replay_state: Arc<ReplayState>,
    ) -> Self {
        Self {
            db_state,
            cert_manager,
            rules_engine,
            dns_state,
            proxy_state,
            anomaly_detector,
            tun_state,
            replay_state,
            proxy_running: AtomicBool::new(false),
            shutdown_tx: Mutex::new(None),
            breakpoint_decision_tx: Mutex::new(None),
            update_available: Arc::new(std::sync::Mutex::new(None)),
            current_tab: Tab::Traffic,
            traffic: TrafficState::default(),
            devices: DevicesState::default(),
            rules: RulesState::default(),
            certs: CertsState::default(),
            dns: DnsTabState::default(),
            alerts: AlertsState::default(),
            replay: ReplayState2::default(),
            graph: GraphState::default(),
            gen: GenState::default(),
        }
    }

    /// Switch to next tab.
    pub fn next_tab(&mut self) {
        self.current_tab = self.current_tab.next();
    }

    /// Switch to previous tab.
    pub fn prev_tab(&mut self) {
        self.current_tab = self.current_tab.prev();
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RecentRequest;

    fn make_req(id: i64, method: &str, host: &str, path: &str, status: Option<u16>) -> RecentRequest {
        RecentRequest { id, timestamp: id.to_string(), method: method.into(), scheme: "https".into(),
            host: host.into(), path: path.into(), status, duration_ms: Some(100), app_tag: None }
    }

    #[test]
    fn test_tab_navigation_next() {
        assert_eq!(Tab::Traffic.next(), Tab::Rules);
        assert_eq!(Tab::Gen.next(), Tab::Traffic);
    }

    #[test]
    fn test_tab_navigation_prev() {
        assert_eq!(Tab::Gen.prev(), Tab::Graph);
        assert_eq!(Tab::Traffic.prev(), Tab::Gen);
    }

    #[test]
    fn test_tab_labels() {
        assert_eq!(Tab::Traffic.label(), "Traffic");
        assert_eq!(Tab::Gen.label(), "Gen");
    }

    #[test]
    fn test_filter_by_method() {
        let mut s = TrafficState::default();
        s.requests.push(make_req(1, "GET", "a.com", "/", Some(200)));
        s.requests.push(make_req(2, "POST", "b.com", "/", Some(200)));
        s.filters.method = Some("GET".into());
        assert_eq!(s.filtered_requests().len(), 1);
    }

    #[test]
    fn test_filter_by_host() {
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
    fn test_filter_combined() {
        let mut s = TrafficState::default();
        s.requests.push(make_req(1, "GET", "api.example.com", "/", Some(200)));
        s.requests.push(make_req(2, "POST", "api.example.com", "/", Some(200)));
        s.requests.push(make_req(3, "GET", "cdn.example.com", "/", Some(200)));
        s.filters.method = Some("GET".into());
        s.filters.host_pattern = Some("api".into());
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
        for i in 0..1005 { s.add_request(&make_req(i, "GET", "a.com", "/", Some(200))); }
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
    fn test_regex_search() {
        let mut s = TrafficState::default();
        s.requests.push(make_req(1, "GET", "api.example.com", "/v1/users", Some(200)));
        s.requests.push(make_req(2, "GET", "cdn.example.com", "/static/app.js", Some(200)));
        s.search_regex = Some(regex::Regex::new("users").unwrap());
        assert_eq!(s.filtered_requests().len(), 1);
    }

    #[test]
    fn test_state_structs_constructible() {
        let _t = TrafficState::default();
        let _r = RulesState::default();
        let _d = DevicesState::default();
        let _c = CertsState::default();
        let _dns = DnsTabState::default();
        let _a = AlertsState::default();
        let _rp = ReplayState2::default();
        let _g = GraphState::default();
        let _gen = GenState::default();
    }

    #[test]
    fn test_breakpoint_mode_default() {
        let state = BreakpointState::default();
        assert!(matches!(state.mode, BreakpointMode::None));
        assert!(state.queue.is_empty());
        assert!(state.current_edit.is_none());
    }

    #[test]
    fn test_breakpoint_go_clears_current() {
        let mut state = BreakpointState::default();
        let req = crate::proxy::InterceptedRequest {
            id: "1".to_string(),
            timestamp: "1".to_string(),
            method: "GET".to_string(),
            host: "example.com".to_string(),
            path: "/".to_string(),
            scheme: "https".to_string(),
            ..Default::default()
        };
        state.queue.push(req.clone());
        state.current_edit = Some(req);
        state.mode = BreakpointMode::RequestPaused;

        // Simulate GO: remove first item
        if !state.queue.is_empty() {
            state.queue.remove(0);
        }
        state.current_edit = state.queue.first().cloned();
        if state.current_edit.is_none() {
            state.mode = BreakpointMode::None;
        }

        assert!(state.queue.is_empty());
        assert!(state.current_edit.is_none());
        assert!(matches!(state.mode, BreakpointMode::None));
    }

    #[test]
    fn test_breakpoint_cancel_clears_all() {
        let mut state = BreakpointState::default();
        for i in 0..3 {
            let req = crate::proxy::InterceptedRequest {
                id: i.to_string(),
                timestamp: i.to_string(),
                method: "GET".to_string(),
                host: "example.com".to_string(),
                path: "/".to_string(),
                scheme: "https".to_string(),
                ..Default::default()
            };
            state.queue.push(req);
        }
        state.current_edit = state.queue.first().cloned();
        state.mode = BreakpointMode::RequestPaused;

        // Simulate Cancel
        state.queue.clear();
        state.current_edit = None;
        state.mode = BreakpointMode::None;

        assert!(state.queue.is_empty());
        assert!(state.current_edit.is_none());
        assert!(matches!(state.mode, BreakpointMode::None));
    }

    #[test]
    fn test_breakpoint_multiple_in_queue() {
        let mut state = BreakpointState::default();
        for i in 0..3 {
            let req = crate::proxy::InterceptedRequest {
                id: i.to_string(),
                timestamp: i.to_string(),
                method: "GET".to_string(),
                host: format!("host{}.com", i),
                path: "/".to_string(),
                scheme: "https".to_string(),
                ..Default::default()
            };
            state.queue.push(req);
        }
        state.mode = BreakpointMode::RequestPaused;

        assert_eq!(state.queue.len(), 3);

        // Go through each item
        state.queue.remove(0);
        assert_eq!(state.queue.len(), 2);
        state.queue.remove(0);
        assert_eq!(state.queue.len(), 1);
        state.queue.remove(0);
        assert_eq!(state.queue.len(), 0);
    }
}
