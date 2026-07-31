//! Tauri Adapter for the file-backed Routing Rules Module.

pub use proxybot_core::{BreakpointTarget, Rule, RuleAction, RulePattern, RulesEngine, RulesError};
use std::sync::Arc;
use tauri::State;

/// Get rules from one Rule File.
#[tauri::command]
pub fn get_rules(
    engine: State<'_, Arc<RulesEngine>>,
    filename: String,
) -> Result<Vec<Rule>, String> {
    engine
        .get_rules_for_file(&filename)
        .map_err(|error| error.to_string())
}

/// Add a Routing Rule or replace the exact rule being edited.
#[tauri::command]
pub fn save_rule(
    engine: State<'_, Arc<RulesEngine>>,
    rule: Rule,
    filename: String,
    original_rule: Option<Rule>,
) -> Result<(), String> {
    engine
        .save_rule(rule, original_rule.as_ref(), &filename)
        .map_err(|error| error.to_string())
}

/// Delete one exact Routing Rule from a Rule File.
#[tauri::command]
pub fn delete_rule(
    engine: State<'_, Arc<RulesEngine>>,
    rule: Rule,
    filename: String,
) -> Result<(), String> {
    engine
        .delete_rule(&rule, &filename)
        .map_err(|error| error.to_string())
}

/// Reorder Routing Rules within one Rule File.
#[tauri::command]
pub fn reorder_rules(
    engine: State<'_, Arc<RulesEngine>>,
    from_index: usize,
    to_index: usize,
    filename: String,
) -> Result<(), String> {
    engine
        .reorder_rules(from_index, to_index, &filename)
        .map_err(|error| error.to_string())
}

/// List available Rule Files.
#[tauri::command]
pub fn list_rule_files(engine: State<'_, Arc<RulesEngine>>) -> Result<Vec<String>, String> {
    engine.list_rule_files().map_err(|error| error.to_string())
}

/// Match a host against the loaded Routing Rules.
#[tauri::command]
pub fn match_host(
    engine: State<'_, Arc<RulesEngine>>,
    host: String,
    ip: Option<String>,
) -> Option<RuleAction> {
    let client_ip = ip.and_then(|value| value.parse().ok());
    engine.match_host(&host, client_ip)
}
