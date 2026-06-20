//! Tauri commands for per-host TLS decryption rules.
//!
//! The DB table `tls_decryption_rules` is the source of truth. Each
//! mutating command writes the table and then rebuilds the cached
//! [`TlsRuleSet`] in [`AppState`] so the proxy hot path
//! (`proxy/https.rs`) picks up the change without a restart.
//!
//! Rule precedence is first-match-wins, ordered by the table's
//! `sort_order` column. New rules append to the end (lowest
//! precedence); the design doc notes the UI will eventually expose
//! reordering, but adding a specific `Decrypt` rule and a broad
//! `Bypass` rule in that order already covers the common case.

use std::sync::Arc;

use tauri::State;

use proxybot_core::{TlsAction, TlsRule, TlsRuleSet};

use crate::db::{self, DbState, TlsRuleRow};
use crate::state::AppState;

/// Parse the stored action string into the core enum. The DB CHECK
/// constraint guarantees only valid values land, but we still map
/// defensively (unknown → Decrypt, the safe MITM default).
fn parse_action(s: &str) -> TlsAction {
    match s {
        "Bypass" => TlsAction::Bypass,
        "Passthrough" => TlsAction::Passthrough,
        _ => TlsAction::Decrypt,
    }
}

/// Serialize a core action back to its stored string. Kept in lockstep
/// with `parse_action` and the table's CHECK constraint.
fn action_to_str(a: TlsAction) -> &'static str {
    match a {
        TlsAction::Decrypt => "Decrypt",
        TlsAction::Bypass => "Bypass",
        TlsAction::Passthrough => "Passthrough",
    }
}

/// Build a [`TlsRuleSet`] from the persisted rows, preserving order.
fn ruleset_from_rows(rows: &[TlsRuleRow]) -> TlsRuleSet {
    let rules = rows
        .iter()
        .map(|r| TlsRule {
            pattern: r.pattern.clone(),
            action: parse_action(&r.action),
        })
        .collect();
    TlsRuleSet::new(rules)
}

/// Load all TLS rules from the DB and refresh the cached rule set.
/// Called at startup and after every mutation. Returns the rows so
/// the UI can render them with hit counts.
pub fn reload_tls_rules(
    db_state: &Arc<DbState>,
    app_state: &Arc<AppState>,
) -> Result<Vec<TlsRuleRow>, String> {
    let rows = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        db::get_tls_rules(&conn)?
    };
    app_state.set_tls_rules(ruleset_from_rows(&rows));
    Ok(rows)
}

/// List the configured TLS decryption rules with their hit counts.
#[tauri::command]
pub fn get_tls_rules(
    db_state: State<'_, Arc<DbState>>,
) -> Result<Vec<TlsRuleRow>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    db::get_tls_rules(&conn)
}

/// Add a TLS decryption rule and refresh the proxy's cached policy.
///
/// `action` must be one of `Decrypt` / `Bypass` / `Passthrough`;
/// anything else is rejected before touching the DB so the caller
/// gets a clear error rather than a CHECK-constraint failure.
#[tauri::command]
pub fn add_tls_rule(
    db_state: State<'_, Arc<DbState>>,
    app_state: State<'_, Arc<AppState>>,
    pattern: String,
    action: String,
) -> Result<Vec<TlsRuleRow>, String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Err("pattern must not be empty".into());
    }
    // Normalise + validate the action by round-tripping through the
    // enum, so a typo from the UI fails loudly here.
    let normalised = action_to_str(parse_action(&action));
    if normalised != action {
        return Err(format!(
            "invalid action '{action}', expected Decrypt | Bypass | Passthrough"
        ));
    }
    {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        db::add_tls_rule(&conn, pattern, normalised)?;
    }
    reload_tls_rules(db_state.inner(), app_state.inner())
}

/// Delete a TLS rule by id and refresh the cached policy.
#[tauri::command]
pub fn delete_tls_rule(
    db_state: State<'_, Arc<DbState>>,
    app_state: State<'_, Arc<AppState>>,
    id: i64,
) -> Result<Vec<TlsRuleRow>, String> {
    {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        db::delete_tls_rule(&conn, id)?;
    }
    reload_tls_rules(db_state.inner(), app_state.inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_round_trips() {
        for a in [TlsAction::Decrypt, TlsAction::Bypass, TlsAction::Passthrough] {
            assert_eq!(parse_action(action_to_str(a)), a);
        }
    }

    #[test]
    fn unknown_action_defaults_to_decrypt() {
        assert_eq!(parse_action("garbage"), TlsAction::Decrypt);
    }

    #[test]
    fn ruleset_from_rows_preserves_order_and_actions() {
        let rows = vec![
            TlsRuleRow {
                id: 1,
                pattern: "api.example.com".into(),
                action: "Decrypt".into(),
                hit_count: 0,
                sort_order: 0,
            },
            TlsRuleRow {
                id: 2,
                pattern: "*.example.com".into(),
                action: "Bypass".into(),
                hit_count: 5,
                sort_order: 1,
            },
        ];
        let rs = ruleset_from_rows(&rows);
        // First-match-wins: exact decrypt beats the wildcard bypass.
        assert_eq!(rs.decide("api.example.com"), TlsAction::Decrypt);
        assert_eq!(rs.decide("cdn.example.com"), TlsAction::Bypass);
    }
}
