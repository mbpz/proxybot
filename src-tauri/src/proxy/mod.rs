//! Proxy module root.
//!
//! Decomposed from the original `proxy.rs` into focused sub-modules.
//! All public items are re-exported here so external callers can still
//! reference them as `crate::proxy::<name>`.

// Sub-modules
mod classify;
mod commands;
mod forward;
mod handler;
mod hooks;
mod http;
mod https;
mod listener;
mod protocol;
mod requests;
mod rules;
mod tls;

// Re-exports: public API surface (must be preserved for backward compatibility).
// Use wildcard re-exports so that the `#[tauri::command]`-generated hidden
// items (e.g. `__cmd__start_proxy`) flow through too — `tauri::generate_handler`
// looks them up at the proxy module level, not at the sub-module.
pub use commands::*;
pub use listener::*;

// ---------------------------------------------------------------------------
// Breakpoint channel types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct BreakpointRequest {
    pub request: InterceptedRequest,
    pub target: BreakpointTarget,
    pub decision_tx: tokio::sync::oneshot::Sender<BreakpointDecision>,
}

#[derive(Clone, Debug)]
pub enum BreakpointDecision {
    Proceed,
    Modify(Box<InterceptedRequest>),
    Drop,
}

// ---------------------------------------------------------------------------
// Proxy runtime state
// ---------------------------------------------------------------------------

pub(super) static PROXY_RUNNING: AtomicBool = AtomicBool::new(false);

pub(super) static SHUTDOWN_TX: LazyLock<
    std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
> = LazyLock::new(|| std::sync::Mutex::new(None));

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct InterceptedRequest {
    pub id: String,
    pub timestamp: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub query_params: Option<String>,
    pub status: Option<u16>,
    pub latency_ms: Option<u64>,
    pub scheme: String,
    pub req_headers: Vec<(String, String)>,
    pub req_body: Option<String>,
    pub resp_headers: Vec<(String, String)>,
    pub resp_body: Option<String>,
    pub resp_size: Option<usize>,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
    pub device_id: Option<i64>,
    pub device_name: Option<String>,
    pub client_ip: Option<String>,
    pub is_websocket: bool,
    pub ws_frames: Option<Vec<WsFrame>>,
    pub grpc_decoded: Option<String>,
    pub graphql_op: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WsFrame {
    pub direction: String,
    pub timestamp: String,
    pub payload: String,
    pub size: usize,
    pub opcode: u8,
    pub truncated: bool,
}

/// Wrapper emitted on the `ws-frame:new` Tauri event channel.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WsFrameEvent {
    pub request_id: String,
    pub frame: WsFrame,
}

/// Map a WebSocket frame opcode to a human-readable name.
pub fn get_opcode_name(opcode: u8) -> &'static str {
    match opcode {
        0x01 => "Text",
        0x02 => "Binary",
        0x08 => "Close",
        0x09 => "Ping",
        0x0A => "Pong",
        _ => "Unknown",
    }
}

// ---------------------------------------------------------------------------
// Internal shared types (used by sibling sub-modules)
// ---------------------------------------------------------------------------

pub(super) struct ProxyContext {
    pub(super) event_tx: broadcast::Sender<InterceptedRequest>,
    pub(super) breakpoint_tx: tokio::sync::mpsc::Sender<BreakpointRequest>,
    pub(super) ws_frame_tx: broadcast::Sender<(String, WsFrame)>,
    #[allow(dead_code)]
    pub(super) cert_manager: Arc<CertManager>,
    pub(super) dns_state: Arc<DnsState>,
    pub(super) db_state: Arc<DbState>,
    pub(super) rules_engine: Arc<RulesEngine>,
    pub(super) plugins: Arc<PluginRegistry>,
    pub(super) plugin_rules: Arc<RuleEngine>,
    pub(super) network: Arc<NetworkConditionEngine>,
    pub(super) scripts: Arc<ScriptEngine>,
    pub(super) metrics: Arc<ProxyMetrics>,
    /// Cloned `Arc<Mutex<Option<String>>>` from `AppState`. The
    /// capture-side `record_http_request` calls in `proxy/{http,
    /// https}.rs` lock this and stamp every newly-recorded
    /// `http_requests` row with the current value (NULL when
    /// nothing is selected). The TUI startup path
    /// (`start_proxy_core`) creates a fresh empty `Arc` since
    /// there's no UI to set it.
    pub(super) active_session_id: Arc<std::sync::Mutex<Option<String>>>,
    /// Cloned `Arc<RwLock<TlsRuleSet>>` from `AppState`. The HTTPS
    /// handler consults this before generating a leaf cert: a
    /// `Bypass`/`Passthrough` host is tunnelled raw instead of
    /// MITM'd. The TUI path starts with an empty set (decrypt all).
    pub(super) tls_rules: Arc<std::sync::RwLock<proxybot_core::TlsRuleSet>>,
}

/// Device context for tracking which device made a request.
#[derive(Clone)]
pub(super) struct DeviceContext {
    pub(super) device_id: i64,
    pub(super) device_name: String,
    #[allow(dead_code)]
    pub ip_address: String,
}

// ---------------------------------------------------------------------------
// Tauri-managed state types
// ---------------------------------------------------------------------------

/// Shared proxy state — stores network config set by get_network_info.
pub struct ProxyState {
    pub interface: std::sync::Mutex<Option<String>>,
    pub local_ip: std::sync::Mutex<Option<String>>,
}

impl Default for ProxyState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            interface: std::sync::Mutex::new(None),
            local_ip: std::sync::Mutex::new(None),
        }
    }
}

pub struct KeepRunningState {
    pub keep_running: std::sync::Mutex<bool>,
}

impl Default for KeepRunningState {
    fn default() -> Self {
        Self::new()
    }
}

impl KeepRunningState {
    pub fn new() -> Self {
        Self {
            keep_running: std::sync::Mutex::new(false),
        }
    }
}

// ---------------------------------------------------------------------------
// Imports needed by this module's type definitions
// ---------------------------------------------------------------------------

use crate::cert::CertManager;
use crate::db::DbState;
use crate::dns::DnsState;
use crate::metrics::ProxyMetrics;
use crate::network::NetworkConditionEngine;
use crate::plugin::registry::PluginRegistry;
use crate::plugin::RuleEngine;
pub use crate::rules::BreakpointTarget;
use crate::rules::RulesEngine;
use crate::scripting::ScriptEngine;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::broadcast;

#[cfg(test)]
mod tests_inner {
    use super::*;

    #[test]
    fn test_keep_running_state() {
        let state = KeepRunningState::new();
        assert!(!*state.keep_running.lock().unwrap());
        *state.keep_running.lock().unwrap() = true;
        assert!(*state.keep_running.lock().unwrap());
    }

    #[test]
    fn test_get_opcode_name() {
        assert_eq!(get_opcode_name(0x01), "Text");
        assert_eq!(get_opcode_name(0x02), "Binary");
        assert_eq!(get_opcode_name(0x08), "Close");
        assert_eq!(get_opcode_name(0x09), "Ping");
        assert_eq!(get_opcode_name(0x0A), "Pong");
        assert_eq!(get_opcode_name(0x00), "Unknown");
    }
}
