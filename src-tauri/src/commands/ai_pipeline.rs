//! Tauri commands for the AI Two-Phase Analysis Pipeline
//!
//! Exposes the NoiseFilter, ApiAnalyzer, and AiPipeline as Tauri commands
//! for the Gen tab and MCP server integration.

use crate::ai_pipeline::{AiPipeline, NoiseFilter, PipelineResult};
use serde::Serialize;

/// Run the complete two-phase AI analysis pipeline
#[tauri::command]
pub fn run_ai_pipeline(
    requests: Vec<crate::proxy::InterceptedRequest>,
    session_id: String,
) -> Result<serde_json::Value, String> {
    let pipeline = AiPipeline::new();
    let result = pipeline.run(requests, &session_id);
    serde_json::to_value(result).map_err(|e| e.to_string())
}

/// Run only Phase 1 (noise filter) and return the filter result
#[tauri::command]
pub fn run_noise_filter(
    requests: Vec<crate::proxy::InterceptedRequest>,
) -> Result<serde_json::Value, String> {
    let filter = NoiseFilter::new();
    let result = filter.filter(requests);
    serde_json::to_value(result).map_err(|e| e.to_string())
}

/// Get a summary of noise categories from requests
#[tauri::command]
pub fn get_noise_summary(
    requests: Vec<crate::proxy::InterceptedRequest>,
) -> Result<serde_json::Value, String> {
    let filter = NoiseFilter::new();
    let result = filter.filter(requests);
    serde_json::to_value(result.summary).map_err(|e| e.to_string())
}

/// Estimate cost for processing requests through the pipeline
#[tauri::command]
pub fn estimate_pipeline_cost_cmd(
    requests: Vec<crate::proxy::InterceptedRequest>,
    provider: String,
    model: String,
) -> Result<f64, String> {
    let avg_tokens = 500; // Rough estimate
    Ok(crate::ai_pipeline::estimate_pipeline_cost(
        requests.len(),
        avg_tokens,
        &provider,
        &model,
    ))
}

#[derive(Serialize)]
struct NoiseReport {
    total: usize,
    noise_count: usize,
    candidate_count: usize,
    categories: std::collections::HashMap<String, usize>,
}

#[tauri::command]
pub fn get_noise_report(
    requests: Vec<crate::proxy::InterceptedRequest>,
) -> Result<serde_json::Value, String> {
    let filter = NoiseFilter::new();
    let result = filter.filter(requests);
    let report = NoiseReport {
        total: result.candidates.len() + result.noise.len(),
        noise_count: result.noise.len(),
        candidate_count: result.candidates.len(),
        categories: result.summary,
    };
    serde_json::to_value(report).map_err(|e| e.to_string())
}