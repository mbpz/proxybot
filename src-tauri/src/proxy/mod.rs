//! Proxy module root.
//!
//! Decomposed from the original `proxy.rs` into focused sub-modules.
//! All public items are re-exported here so external callers can still
//! reference them as `crate::proxy::<name>`.

// Sub-modules
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
    Modify(InterceptedRequest),
    Drop,
}

// ---------------------------------------------------------------------------
// Proxy runtime state
// ---------------------------------------------------------------------------

pub(super) static PROXY_RUNNING: AtomicBool = AtomicBool::new(false);

pub(super) static SHUTDOWN_TX: LazyLock<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>> =
    LazyLock::new(|| std::sync::Mutex::new(None));

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
}

// ---------------------------------------------------------------------------
// Internal shared types (used by sibling sub-modules)
// ---------------------------------------------------------------------------

pub(super) struct ProxyContext {
    pub(super) event_tx: broadcast::Sender<InterceptedRequest>,
    pub(super) breakpoint_tx: tokio::sync::mpsc::Sender<BreakpointRequest>,
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
use crate::rules::RulesEngine;
use crate::scripting::ScriptEngine;
pub use crate::rules::BreakpointTarget;
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
}
