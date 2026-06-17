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

/// Run a full replay validation cycle against the generated spec.
pub async fn run_replay(
    openapi_yaml: &str,
    records: &[crate::specgen::TrafficRecord],
    port: Option<u16>,
) -> Result<ReplayReport, SpecError> {
    use crate::specgen::extract::template_path;

    let started_at = chrono::Utc::now();

    // Bind ephemeral port if not provided.
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port.unwrap_or(0)))
        .await
        .map_err(|e| SpecError::ReplayFailed(e.to_string()))?;
    let mock_port = listener
        .local_addr()
        .map(|a| a.port())
        .map_err(|e| SpecError::ReplayFailed(e.to_string()))?;

    let mut routes = HashMap::new();
    // Seed mock routes from the first example per (method, template).
    let doc: serde_yaml::Value = serde_yaml::from_str(openapi_yaml)
        .map_err(|e| SpecError::ReplayFailed(e.to_string()))?;
    if let Some(paths) = doc.get("paths").and_then(|p| p.as_mapping()) {
        for (tpl_key, item) in paths {
            let tpl = tpl_key.as_str().unwrap_or("").to_string();
            for (m, _op) in item.as_mapping().into_iter().flatten() {
                let method = m.as_str().unwrap_or("").to_uppercase();
                let key = format!("{} {}", method, tpl.trim_start_matches('/'));
                routes.insert(key, serde_json::json!({}));
            }
        }
    }

    let state = MockState {
        routes: Arc::new(Mutex::new(routes)),
    };
    let app = build_mock_router(state.clone());
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });

    // Replay each record and compare.
    let client = reqwest::Client::new();
    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut error = 0usize;
    let mut failures = Vec::new();

    for r in records
        .iter()
        .filter(|r| r.kind == crate::specgen::TrafficKind::Http)
    {
        let tpl = template_path(&r.path)
            .template
            .trim_start_matches('/')
            .to_string();
        let url = format!("http://127.0.0.1:{}/{}", mock_port, tpl);
        let method = reqwest::Method::from_bytes(r.method.as_bytes())
            .unwrap_or(reqwest::Method::GET);
        let resp = match client
            .request(method, &url)
            .body(r.request_body.clone().unwrap_or_default())
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => {
                error += 1;
                continue;
            }
        };
        let actual_status = resp.status().as_u16();
        let body_bytes = resp.bytes().await.unwrap_or_default();
        let body_diff = body_diff_summary(&r.response_body, &body_bytes);
        let status_ok = actual_status == r.response_status;
        let body_ok = body_diff.is_none();
        if status_ok && body_ok {
            pass += 1;
        } else {
            fail += 1;
            failures.push(ReplayFailure {
                path: r.path.clone(),
                method: r.method.clone(),
                expected_status: r.response_status,
                actual_status,
                body_diff_summary: body_diff,
            });
        }
    }

    server_handle.abort();
    let total = pass + fail + error;
    let pass_rate = if total == 0 {
        0.0
    } else {
        pass as f32 / total as f32
    };
    Ok(ReplayReport {
        total,
        pass,
        fail,
        error,
        pass_rate,
        failures,
        started_at,
        finished_at: chrono::Utc::now(),
        mock_port,
    })
}

fn body_diff_summary(expected: &Option<String>, actual: &[u8]) -> Option<String> {
    let exp = expected.as_deref().unwrap_or("");
    if let (Ok(e), Ok(a)) = (
        serde_json::from_str::<Value>(exp),
        serde_json::from_slice::<Value>(actual),
    ) {
        if shallow_eq(&e, &a) {
            return None;
        }
        return Some("body json differs".into());
    }
    if exp.as_bytes() == actual {
        None
    } else {
        Some("body bytes differ".into())
    }
}

fn shallow_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).map(|bv| shallow_eq(v, bv)).unwrap_or(false))
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(xi, yi)| shallow_eq(xi, yi))
        }
        _ => a == b,
    }
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

#[cfg(test)]
mod replay_tests {
    use super::*;
    use crate::specgen::{TrafficKind, TrafficRecord};
    use chrono::Utc;

    fn rec(method: &str, path: &str) -> TrafficRecord {
        TrafficRecord {
            method: method.into(),
            path: path.into(),
            host: "x".into(),
            request_body: Some("{}".into()),
            response_status: 200,
            response_body: Some(r#"{"ok":true}"#.into()),
            timestamp: Utc::now(),
            kind: TrafficKind::Http,
        }
    }

    #[tokio::test]
    async fn run_replay_returns_report() {
        let openapi = r#"
openapi: 3.1.0
info: { title: t, version: 1.0.0, description: d }
servers: [{ url: "http://x" }]
paths:
  /echo:
    get:
      operationId: getEcho
      summary: echo
      tags: [auto]
      responses: {}
"#;
        let records = vec![rec("GET", "/echo")];
        let report = run_replay(openapi, &records, Some(0)).await.unwrap();
        assert_eq!(report.total, 1);
        assert!(report.pass + report.fail + report.error == 1);
    }
}
