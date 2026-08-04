//! Proxy module root.
//!
//! Decomposed from the original `proxy.rs` into focused sub-modules.
//! All public items are re-exported here so external callers can still
//! reference them as `crate::proxy::<name>`.

// Sub-modules
mod capture_decode;
mod classify;
mod commands;
mod listener;
mod requests;
mod runtime_adapter;
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

pub use proxybot_core::BreakpointDecision;

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

pub use proxybot_core::{InterceptedRequest, WsFrame};

proxybot_core::desktop_contract_type! {
    /// Wrapper emitted on the `ws-frame:new` Tauri event channel.
    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct WsFrameEvent {
        pub request_id: String,
        pub frame: WsFrame,
    }
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

pub use crate::rules::BreakpointTarget;

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
