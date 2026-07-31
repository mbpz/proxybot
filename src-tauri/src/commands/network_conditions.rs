//! Tauri commands for Network Conditions (latency / bandwidth / loss).
//!
//! The engine is exposed as a `tauri::State` so the frontend can switch
//! between built-in presets (`2G`, `3G`, `4G`, `WiFi`, `Edge`) or disable
//! conditions entirely. Per-host rules are added/removed via the same
//! engine; matching against live traffic is currently a future extension
//! (host extraction from CONNECT/SNI is not yet wired in).

use std::sync::Arc;
use tauri::State;

use crate::network::NetworkConditionEngine;
use crate::network::NetworkProfile;
use crate::network::NewConditionRule;

/// Wrapper newtype so the engine can live in `tauri::State` even though
/// the engine itself is not `Send`/`Sync` aware (it is, but Tauri's
/// `manage` requires a concrete type to disambiguate from other Arcs).
pub struct NetworkConditionsState(pub Arc<NetworkConditionEngine>);

/// List all available network profiles (built-in + custom).
#[tauri::command]
pub fn get_network_profiles(
    state: State<'_, NetworkConditionsState>,
) -> Result<Vec<NetworkProfile>, String> {
    Ok(state.0.list_profiles())
}

/// Set the active profile by name. Pass `None` (or the empty string) to
/// disable network conditions.
#[tauri::command]
pub fn set_active_profile(
    state: State<'_, NetworkConditionsState>,
    name: Option<String>,
) -> Result<(), String> {
    match name {
        None => {
            state.0.disable();
            Ok(())
        }
        Some(ref n) if n.is_empty() => {
            state.0.disable();
            Ok(())
        }
        Some(n) => state.0.set_active(&n),
    }
}

/// Get the currently active profile, or `None` if conditions are disabled.
#[tauri::command]
pub fn get_active_profile(
    state: State<'_, NetworkConditionsState>,
) -> Result<Option<NetworkProfile>, String> {
    Ok(state.0.get_active())
}

/// Add a per-host condition rule. Returns the assigned rule id.
#[tauri::command]
pub fn add_condition_rule(
    state: State<'_, NetworkConditionsState>,
    rule: NewConditionRule,
) -> Result<u64, String> {
    // Validate the profile name exists so the frontend gets fast feedback.
    if state
        .0
        .list_profiles()
        .iter()
        .all(|p| p.name != rule.profile)
    {
        return Err(format!("Unknown profile: {}", rule.profile));
    }
    Ok(state.0.add_rule(rule))
}

/// Remove a condition rule by id. Returns `true` if the rule existed.
#[tauri::command]
pub fn remove_condition_rule(
    state: State<'_, NetworkConditionsState>,
    id: u64,
) -> Result<bool, String> {
    Ok(state.0.remove_rule(id))
}

/// List all condition rules (enabled + disabled).
#[tauri::command]
pub fn list_condition_rules(
    state: State<'_, NetworkConditionsState>,
) -> Result<Vec<crate::network::ConditionRule>, String> {
    Ok(state.0.list_rules())
}
