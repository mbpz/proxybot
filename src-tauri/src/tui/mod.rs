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
    pub app_tag: Option<String>,      // app name filter
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
    // Detail panel
    pub detail_request: Option<InterceptedRequest>,
    pub detail_loading: bool,
    // Scroll offset for detail panel
    pub detail_scroll: Option<u64>,
    // pf/DNS status
    pub pf_enabled: bool,
    pub dns_running: bool,
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
        self.requests
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
                        "pending" => status >= 100 && status < 600,
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
            .collect()
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

/// Graph tab state.
#[derive(Default)]
pub struct GraphState {
    pub selected: usize,
}

/// Gen (Scaffold/Generate) tab state.
#[derive(Default)]
pub struct GenState {
    pub selected: usize,
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
