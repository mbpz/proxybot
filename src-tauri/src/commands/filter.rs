//! Tauri commands for the Filter DSL.

use crate::filter::preset::{self, FilterPreset};
use crate::filter::query::{CompiledTrafficQuery, TrafficQuery};
use std::sync::Arc;
use tauri::State;

proxybot_core::desktop_contract_type! {
    /// Result of validating a Filter DSL expression.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct ParseResult {
        pub ok: bool,
        pub error: Option<String>,
    }
}

/// Parse a DSL expression. Always returns a `ParseResult` rather than
/// `Result<_, String>` so the frontend can handle invalid input
/// gracefully without try/catch on every keystroke.
#[tauri::command]
pub fn parse_filter(expr: String) -> ParseResult {
    match CompiledTrafficQuery::compile(&TrafficQuery {
        expression: expr,
        ..Default::default()
    }) {
        Ok(_) => ParseResult {
            ok: true,
            error: None,
        },
        Err(e) => ParseResult {
            ok: false,
            error: Some(e),
        },
    }
}

/// Return all saved filter presets.
#[tauri::command]
pub fn list_filter_presets(
    config: State<'_, Arc<proxybot_core::AppConfig>>,
) -> Result<Vec<FilterPreset>, String> {
    preset::list(&config.filter_presets_path)
}

/// Save (insert or replace-by-id) a preset.
#[tauri::command]
pub fn save_filter_preset(
    preset: FilterPreset,
    config: State<'_, Arc<proxybot_core::AppConfig>>,
) -> Result<(), String> {
    preset::save(&config.filter_presets_path, preset)
}

/// Delete the preset with the given id. Returns Err if not found.
#[tauri::command]
pub fn delete_filter_preset(
    id: String,
    config: State<'_, Arc<proxybot_core::AppConfig>>,
) -> Result<(), String> {
    preset::delete(&config.filter_presets_path, &id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_uses_the_same_compiler_as_traffic_queries() {
        assert!(parse_filter("status:>=400".to_owned()).ok);
        assert!(parse_filter("".to_owned()).ok);
        assert!(!parse_filter("path:~[unterminated".to_owned()).ok);
        assert!(!parse_filter("status:>=many".to_owned()).ok);
    }
}
