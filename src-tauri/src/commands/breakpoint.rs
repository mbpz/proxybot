//! Tauri commands for interactive request interception (breakpoints).
//!
//! The proxy rules engine emits `BreakpointRequest`s into an mpsc channel
//! when a rule matches. The bridge task (spawned by `start_proxy` in
//! `listener.rs`) reads that channel, stashes the oneshot sender in
//! `AppState::pending_breakpoints`, and emits a Tauri event so the UI
//! wakes up. The UI polls via [`get_pending_breakpoints`] and resolves
//! each via [`resolve_breakpoint`].

use std::sync::Arc;
use tauri::State;

use crate::proxy::{BreakpointDecision, InterceptedRequest};
use crate::state::{AppState, BreakpointSnapshot};

/// Return all currently-pending breakpoints as serialisable snapshots.
#[tauri::command]
pub fn get_pending_breakpoints(state: State<'_, Arc<AppState>>) -> Vec<BreakpointSnapshot> {
    state.list_breakpoints()
}

/// Resolve a breakpoint: forward, drop, or modify-and-forward.
///
/// `id` is the `bp-<uuid>` returned by the `breakpoint:new` event
/// payload. `decision` is `"proceed"`, `"drop"`, or `"modify"`.
///
/// **Edit-and-forward payload**: when `decision` is `"modify"`, the
/// caller should also send a sparse `InterceptedRequest` JSON with
/// only the fields that changed. Currently the modify path only
/// supports overriding `method`, `path`, `req_headers`, and
/// `req_body`; all other fields on the sent request are ignored.
///
/// Returns the remaining breakpoint count so the UI knows whether
/// the panel can close.
///
/// `mutated` is only meaningful for `decision == "modify"`. The
/// frontend may send the minimum request fields needed by its editor;
/// the proxy forwards the editable request fields from that snapshot.
#[tauri::command]
pub fn resolve_breakpoint(
    state: State<'_, Arc<AppState>>,
    id: String,
    decision: String,
    mutated: Option<InterceptedRequest>,
) -> Result<usize, String> {
    let decision_val = match decision.as_str() {
        "proceed" => BreakpointDecision::Proceed,
        "drop" => BreakpointDecision::Drop,
        "modify" => BreakpointDecision::Modify(Box::new(
            mutated.ok_or_else(|| "mutated request required when decision='modify'".to_string())?,
        )),
        other => {
            return Err(format!(
                "invalid decision '{other}', expected proceed|drop|modify"
            ))
        }
    };

    let tx = state
        .take_breakpoint_sender(&id)
        .ok_or_else(|| format!("breakpoint '{id}' not found or already resolved"))?;

    tx.send(decision_val)
        .map_err(|_| "breakpoint receiver dropped — request already proceeding".to_string())?;

    let remaining = state
        .pending_breakpoints
        .lock()
        .map_err(|e| e.to_string())?
        .len();
    Ok(remaining)
}

/// Cancel all pending breakpoints (used on proxy stop).
#[tauri::command]
pub fn cancel_all_breakpoints(state: State<'_, Arc<AppState>>) {
    state.cancel_all_breakpoints();
}
