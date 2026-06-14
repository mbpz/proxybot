//! Tauri commands for the Filter DSL.

use serde::{Deserialize, Serialize};

use crate::filter::dsl;
use crate::filter::evaluator::Evaluator;
use crate::filter::preset::{self, FilterPreset};
use crate::proxy::InterceptedRequest;

/// Result of parsing a DSL expression. Returned as JSON so the
/// frontend can display `error` inline without throwing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Parse a DSL expression. Always returns a `ParseResult` rather than
/// `Result<_, String>` so the frontend can handle invalid input
/// gracefully without try/catch on every keystroke.
#[tauri::command]
pub fn parse_filter(expr: String) -> ParseResult {
    match dsl::parse(&expr) {
        Ok(_) => ParseResult { ok: true, error: None },
        Err(e) => ParseResult { ok: false, error: Some(e) },
    }
}

/// Parse + evaluate a DSL expression against a single request.
/// Returns `false` on parse error so the frontend can still display
/// the row.
#[tauri::command]
pub fn evaluate_filter(expr: String, request: InterceptedRequest) -> bool {
    match dsl::parse(&expr) {
        Ok(parsed) => Evaluator::evaluate(&parsed, &request),
        Err(_) => false,
    }
}

/// Return all saved filter presets.
#[tauri::command]
pub fn list_filter_presets() -> Result<Vec<FilterPreset>, String> {
    preset::list()
}

/// Save (insert or replace-by-id) a preset.
#[tauri::command]
pub fn save_filter_preset(preset: FilterPreset) -> Result<(), String> {
    preset::save(preset)
}

/// Delete the preset with the given id. Returns Err if not found.
#[tauri::command]
pub fn delete_filter_preset(id: String) -> Result<(), String> {
    preset::delete(&id)
}