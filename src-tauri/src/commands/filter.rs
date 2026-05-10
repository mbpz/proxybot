use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPreset {
    pub id: String,
    pub name: String,
    pub expr: String,
}

#[tauri::command]
pub fn parse_filter(expr: &str) -> Result<String, String> {
    crate::filter::dsl::parse(expr)?;
    Ok("valid".to_string())
}

#[tauri::command]
pub fn evaluate_filter(expr: &str, request_json: &str) -> Result<bool, String> {
    let expr = crate::filter::dsl::parse(expr)?;
    let request: crate::filter::evaluator::InterceptedRequest =
        serde_json::from_str(request_json).map_err(|e| e.to_string())?;
    Ok(crate::filter::evaluator::Evaluator::evaluate(
        &expr, &request,
    ))
}

#[tauri::command]
pub fn save_filter_preset(preset: FilterPreset) -> Result<(), String> {
    // Save to config file
    Ok(())
}

#[tauri::command]
pub fn get_filter_presets() -> Result<Vec<FilterPreset>, String> {
    Ok(vec![])
}
