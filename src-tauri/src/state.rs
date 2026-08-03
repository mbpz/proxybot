//! Global application state shared with Tauri commands.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use crate::proxy::{BreakpointDecision, BreakpointTarget, InterceptedRequest};
use proxybot_core::{SpecConfig, TlsRuleSet};
use serde::Serialize;
use std::collections::HashMap;
use tokio::sync::oneshot;

/// Wrapper newtype so `AppState` can live in `tauri::State` even though
/// the inner config is a plain data type. Following the pattern used by
/// `NetworkConditionsState` in `commands/network_conditions.rs`.
pub struct AppState {
    specs_dir: PathBuf,
    /// User-tunable spec generation knobs (API key, retry counts,
    /// replay toggles, mock port). Wrapped in `Mutex` so commands can
    /// update it at runtime via `update_specgen_config`.
    pub specgen_config: Arc<Mutex<SpecConfig>>,
    /// The session id the UI is currently focused on. The proxy
    /// capture pipeline tags every newly-recorded `http_requests`
    /// row with this value so `get_traffic_records(session_id)` in
    /// `commands/specgen.rs` can later filter by it. `None` means
    /// "no active session" — captured rows have NULL `session_id`
    /// and are returned by `get_traffic_records("")`.
    ///
    /// Held as an `Arc` so the desktop Capture Event Adapter can retain the
    /// same value without going through the Tauri State map per request.
    pub active_session_id: Arc<Mutex<Option<String>>>,
    /// Per-host TLS decryption policy. The core MITM Runtime reads this shared
    /// rule set before deciding whether to decrypt a connection. The DB is the source
    /// of truth; commands that mutate the rules rebuild this cache
    /// so changes take effect without a proxy restart.
    ///
    /// `RwLock` because the read path (every HTTPS CONNECT) is hot
    /// and the write path (user edits a rule) is rare.
    pub tls_rules: Arc<RwLock<TlsRuleSet>>,
    /// Breakpoints awaiting a user decision. The bridge task
    /// (`start_proxy` in `listener.rs`) reads from `bp_rx`, stashes
    /// the oneshot sender here keyed by a generated id, and emits a
    /// Tauri event. The UI fetches the request snapshots via
    /// `get_pending_breakpoints` and resolves each via
    /// `resolve_breakpoint(id, decision)`. The oneshot stays in the
    /// map until resolved, so the proxy worker that's awaiting it
    /// keeps the connection alive.
    pub pending_breakpoints: Arc<Mutex<HashMap<String, PendingBreakpoint>>>,
}

/// One paused request waiting on a user decision. Held in
/// [`AppState::pending_breakpoints`] until the UI calls
/// `resolve_breakpoint(id, ...)` and we send through `decision_tx`.
pub struct PendingBreakpoint {
    pub id: String,
    pub target: BreakpointTarget,
    pub request: InterceptedRequest,
    /// `None` once the decision has been delivered. The bridge task
    /// holds the sender as `Some(...)` so resolve can take it out.
    pub decision_tx: Option<oneshot::Sender<BreakpointDecision>>,
}

/// Wire-format snapshot of a pending breakpoint for the UI. Mirrors
/// `PendingBreakpoint` minus the non-serialisable oneshot.
#[derive(Debug, Clone, Serialize)]
pub struct BreakpointSnapshot {
    pub id: String,
    pub target: BreakpointTarget,
    pub request: InterceptedRequest,
}

impl AppState {
    /// Default-initialised state. The specgen config starts at
    /// `SpecConfig::default()`; callers can mutate the LLM key and
    /// other knobs at runtime via the `update_specgen_config` command.
    pub fn new() -> Self {
        Self::with_specs_dir(PathBuf::from(".proxybot/specs"))
    }

    pub fn with_specs_dir(specs_dir: PathBuf) -> Self {
        Self {
            specs_dir,
            specgen_config: Arc::new(Mutex::new(SpecConfig::default())),
            active_session_id: Arc::new(Mutex::new(None)),
            tls_rules: Arc::new(RwLock::new(TlsRuleSet::default())),
            pending_breakpoints: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Replace the cached TLS rule set wholesale. Called after any
    /// DB mutation so the proxy hot path sees the new policy without
    /// a restart.
    pub fn set_tls_rules(&self, rules: TlsRuleSet) {
        let mut guard = self.tls_rules.write().expect("tls_rules rwlock poisoned");
        *guard = rules;
    }

    /// Snapshot the current active session id. Returns `None` when
    /// no session is selected. Used by the proxy capture pipeline
    /// (via the desktop Capture Event Adapter) and by the
    /// `get_active_session` Tauri command.
    pub fn active_session_id_snapshot(&self) -> Option<String> {
        self.active_session_id
            .lock()
            .expect("active_session_id mutex poisoned")
            .clone()
    }

    /// Replace the active session id. `None` clears it; subsequent
    /// captures will be tagged with NULL.
    pub fn set_active_session_id(&self, id: Option<String>) {
        let mut guard = self
            .active_session_id
            .lock()
            .expect("active_session_id mutex poisoned");
        *guard = id;
    }

    /// Snapshot the current specgen config. Used by commands that
    /// only need read access (build_spec, run_replay_validation).
    pub fn specgen_config_snapshot(&self) -> SpecConfig {
        self.specgen_config
            .lock()
            .expect("specgen_config mutex poisoned")
            .clone()
    }

    /// Replace the specgen config wholesale.
    pub fn set_specgen_config(&self, new_cfg: SpecConfig) {
        let mut guard = self
            .specgen_config
            .lock()
            .expect("specgen_config mutex poisoned");
        *guard = new_cfg;
    }

    /// Directory where generated spec JSON files are persisted. Each
    /// session gets its own file: `<specs_dir>/<session_id>.json`.
    pub fn specs_dir(&self) -> PathBuf {
        self.specs_dir.clone()
    }
}

impl AppState {
    /// Snapshot all pending breakpoints for the UI. The oneshots
    /// stay in the map; only the request data is returned.
    pub fn list_breakpoints(&self) -> Vec<BreakpointSnapshot> {
        let guard = self
            .pending_breakpoints
            .lock()
            .expect("breakpoints mutex poisoned");
        guard
            .values()
            .map(|p| BreakpointSnapshot {
                id: p.id.clone(),
                target: p.target.clone(),
                request: p.request.clone(),
            })
            .collect()
    }

    /// Stash a breakpoint (called by the bridge task). Generates an
    /// id and returns it so the bridge can include it in the
    /// `breakpoint:new` Tauri event payload.
    pub fn insert_breakpoint(
        &self,
        target: BreakpointTarget,
        request: InterceptedRequest,
        decision_tx: oneshot::Sender<BreakpointDecision>,
    ) -> String {
        let id = format!("bp-{}", uuid::Uuid::new_v4().simple());
        let mut guard = self
            .pending_breakpoints
            .lock()
            .expect("breakpoints mutex poisoned");
        guard.insert(
            id.clone(),
            PendingBreakpoint {
                id: id.clone(),
                target,
                request,
                decision_tx: Some(decision_tx),
            },
        );
        id
    }

    /// Remove a breakpoint and take its decision sender. The caller
    /// (`resolve_breakpoint` command) then sends the user's decision.
    /// Returns None if the id is unknown or already resolved.
    pub fn take_breakpoint_sender(&self, id: &str) -> Option<oneshot::Sender<BreakpointDecision>> {
        let mut guard = self
            .pending_breakpoints
            .lock()
            .expect("breakpoints mutex poisoned");
        let mut entry = guard.remove(id)?;
        entry.decision_tx.take()
    }

    /// Cancel all pending breakpoints by resolving each to `Proceed`.
    /// Called on proxy shutdown so we don't strand the suspended
    /// connections.
    pub fn cancel_all_breakpoints(&self) {
        let drained: Vec<_> = {
            let mut guard = self
                .pending_breakpoints
                .lock()
                .expect("breakpoints mutex poisoned");
            guard.drain().collect()
        };
        for (_id, mut bp) in drained {
            if let Some(tx) = bp.decision_tx.take() {
                let _ = tx.send(BreakpointDecision::Proceed);
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_specgen_config_matches_core_default() {
        let s = AppState::new();
        let cfg = s.specgen_config_snapshot();
        assert_eq!(cfg.max_traffic_records, 50);
        assert_eq!(cfg.max_retry, 2);
        assert!(cfg.enable_replay_validation);
        assert!(cfg.deepseek_api_key.is_none());
        assert!(cfg.mock_port.is_none());
    }

    #[test]
    fn set_specgen_config_updates_snapshot() {
        let s = AppState::new();
        let new_cfg = SpecConfig {
            deepseek_api_key: Some("sk-test".into()),
            max_traffic_records: 100,
            max_retry: 5,
            enable_replay_validation: false,
            mock_port: Some(19999),
        };
        s.set_specgen_config(new_cfg.clone());
        let snap = s.specgen_config_snapshot();
        assert_eq!(snap.deepseek_api_key.as_deref(), Some("sk-test"));
        assert_eq!(snap.max_traffic_records, 100);
        assert_eq!(snap.mock_port, Some(19999));
    }

    #[test]
    fn specs_dir_ends_with_specs() {
        let s = AppState::new();
        let p = s.specs_dir();
        assert!(p.ends_with("specs"));
    }

    #[test]
    fn active_session_id_round_trips() {
        let s = AppState::new();
        // Defaults to None.
        assert!(s.active_session_id_snapshot().is_none());

        // Setting Some and reading it back returns the value.
        s.set_active_session_id(Some("session-7".into()));
        assert_eq!(s.active_session_id_snapshot().as_deref(), Some("session-7"));

        // Setting None clears it.
        s.set_active_session_id(None);
        assert!(s.active_session_id_snapshot().is_none());
    }

    #[test]
    fn active_session_id_arc_is_shared() {
        // The Arc<Mutex<...>> is the contract used by the Capture Event Adapter;
        // confirm a clone of the Arc sees mutations through the
        // owner.
        let s = AppState::new();
        let shared = s.active_session_id.clone();
        s.set_active_session_id(Some("abc".into()));
        let observed = shared.lock().unwrap().clone();
        assert_eq!(observed.as_deref(), Some("abc"));
    }
}
