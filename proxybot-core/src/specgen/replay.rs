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

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct MockState {
    pub routes: Arc<Mutex<HashMap<String, Value>>>,
}

pub fn build_mock_router(state: MockState) -> Router {
    Router::new()
        .route("/", get(echo))
        .route("/*path", get(echo_path).post(echo_path).put(echo_path).delete(echo_path))
        .with_state(state)
}

async fn echo() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}

async fn echo_path(
    State(state): State<MockState>,
    Path(path): Path<String>,
    method: axum::http::Method,
) -> impl IntoResponse {
    let key = format!("{} /{}", method.as_str(), path);
    let routes = state.routes.lock().await;
    if let Some(v) = routes.get(&key) {
        return (StatusCode::OK, Json(v.clone())).into_response();
    }
    (StatusCode::OK, Json(serde_json::json!({ "echoed_path": path }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_returns_200_for_unknown_route() {
        let app = build_mock_router(MockState::default());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let url = format!("http://{}/x/y", addr);
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);
    }
}
