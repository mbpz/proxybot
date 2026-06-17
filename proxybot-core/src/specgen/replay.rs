//! Replay validation against generated OpenAPI spec.

use crate::specgen::error::SpecError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayReport {
    pub total: usize,
    pub pass: usize,
    pub fail: usize,
    pub error: usize,
    pub pass_rate: f32,
    pub failures: Vec<ReplayFailure>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: chrono::DateTime<chrono::Utc>,
    pub mock_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayFailure {
    pub path: String,
    pub method: String,
    pub expected_status: u16,
    pub actual_status: u16,
    pub body_diff_summary: Option<String>,
}

/// Run a full replay validation cycle. Implementation lives in Task 12-13.
pub async fn run_replay(
    _openapi_yaml: &str,
    _records: &[crate::specgen::TrafficRecord],
    _port: Option<u16>,
) -> Result<ReplayReport, SpecError> {
    Err(SpecError::ReplayFailed("not yet implemented".into()))
}
