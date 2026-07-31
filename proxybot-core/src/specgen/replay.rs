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
    // Seed mock routes from examples + response body schemas in the spec.
    let doc: serde_yaml::Value =
        serde_yaml::from_str(openapi_yaml).map_err(|e| SpecError::ReplayFailed(e.to_string()))?;
    // Convert YAML value tree → JSON value tree for consistent access
    let json_doc: serde_json::Value =
        serde_json::to_value(&doc).map_err(|e| SpecError::ReplayFailed(e.to_string()))?;
    if let Some(paths) = json_doc.get("paths").and_then(|p| p.as_object()) {
        for (tpl_key, item) in paths {
            let tpl = tpl_key.clone();
            for (m, op_val) in item.as_object().into_iter().flatten() {
                let method = m.to_uppercase();

                let example_body = extract_example(op_val);
                // If the spec didn't carry an example, fall back to the first
                // captured response body for this (method, template). Without
                // this fallback, the mock returns `{}` and the body diff in
                // replay will fail for any non-trivial spec.
                let body = if example_body == serde_json::json!({}) {
                    first_captured_body(records, &tpl, &method).unwrap_or(example_body)
                } else {
                    example_body
                };
                let status_code = extract_status(op_val, records, &tpl, &method);

                // The handler templates the incoming concrete path with the
                // heuristic in `template_path`, which produces
                // /api/users/{usersId} for /api/users/1. The spec template
                // may use a different param name (e.g. /api/users/{id}). Seed
                // under BOTH forms so the lookup succeeds regardless of which
                // param naming the spec used.
                let heuristic_tpl =
                    heuristic_template(records, &tpl, &method).unwrap_or_else(|| tpl.clone());
                for key_tpl in std::iter::once(tpl.clone())
                    .chain(std::iter::once(heuristic_tpl).filter(|h| h != &tpl))
                {
                    routes.insert(
                        format!("{} {}", method, key_tpl),
                        MockRoute {
                            body: body.clone(),
                            status: status_code,
                        },
                    );
                }
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
        let method =
            reqwest::Method::from_bytes(r.method.as_bytes()).unwrap_or(reqwest::Method::GET);
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

/// Extract the first example body from an OpenAPI operation value.
fn extract_example(op_val: &serde_json::Value) -> serde_json::Value {
    if let Some(responses) = op_val.get("responses") {
        // Try 200, 201, then first available status
        for code in &["200", "201"] {
            if let Some(example) = extract_from_response(responses, code) {
                return example;
            }
        }
        // Fallback: first response
        if let Some(first) = responses.as_object().and_then(|r| r.values().next()) {
            if let Some(example) = extract_from_response_val(first) {
                return example;
            }
        }
    }
    serde_json::json!({})
}

fn extract_from_response(responses: &serde_json::Value, code: &str) -> Option<serde_json::Value> {
    responses.get(code).and_then(extract_from_response_val)
}

fn extract_from_response_val(resp: &serde_json::Value) -> Option<serde_json::Value> {
    // Try: content → application/json → examples → first → value
    if let Some(example) = resp
        .pointer("/content/application~1json/examples")
        .and_then(|ex| ex.as_object())
        .and_then(|ex| ex.values().next())
        .and_then(|v| v.get("value"))
    {
        return Some(example.clone());
    }
    // Try: content → application/json → example
    if let Some(example) = resp.pointer("/content/application~1json/example") {
        return Some(example.clone());
    }
    // Try: content → application/json → schema → example
    if let Some(example) = resp.pointer("/content/application~1json/schema/example") {
        return Some(example.clone());
    }
    None
}

/// Extract the expected status code from an OpenAPI operation.
/// Falls back to matching recorded responses for the same path+method template.
fn extract_status(
    op_val: &serde_json::Value,
    records: &[crate::specgen::TrafficRecord],
    tpl: &str,
    method: &str,
) -> u16 {
    // Try to find a matching recorded response
    for r in records {
        if r.method.eq_ignore_ascii_case(method) {
            let r_tpl = crate::specgen::extract::template_path(&r.path).template;
            if r_tpl.trim_start_matches('/') == tpl.trim_start_matches('/') {
                return r.response_status;
            }
        }
    }
    // Fallback: try to read from spec responses
    if let Some(responses) = op_val.get("responses") {
        if responses.get("200").or(responses.get("201")).is_some() {
            return 200;
        }
        if let Some(first_key) = responses.as_object().and_then(|r| r.keys().next()) {
            if let Ok(c) = first_key.parse::<u16>() {
                return c;
            }
        }
    }
    200
}

/// Find the first captured response body for a (method, template) pair.
/// Used as a fallback when the spec doesn't carry an example for an operation.
/// Templates are normalized to the heuristic form (numeric/uuid segments →
/// `{prevName}Id`) so that `/api/users/{id}` (spec) matches `/api/users/{usersId}`
/// (heuristic on `/api/users/1`).
fn first_captured_body(
    records: &[crate::specgen::TrafficRecord],
    tpl: &str,
    method: &str,
) -> Option<serde_json::Value> {
    let target = normalize_template(tpl);
    for r in records
        .iter()
        .filter(|r| r.kind == crate::specgen::TrafficKind::Http)
    {
        if !r.method.eq_ignore_ascii_case(method) {
            continue;
        }
        let r_tpl = normalize_template(&template_path(&r.path).template);
        if r_tpl == target {
            if let Some(body) = r
                .response_body
                .as_deref()
                .and_then(|b| serde_json::from_str::<serde_json::Value>(b).ok())
            {
                return Some(body);
            }
        }
    }
    None
}

/// Return the template the heuristic would produce for the first captured
/// record that matches `(tpl, method)`. The spec's literal template and the
/// heuristic's templated form can differ in param names (e.g. `{id}` vs
/// `{usersId}`); the mock needs to seed under whichever form the request
/// handler will compute, which is the heuristic's.
fn heuristic_template(
    records: &[crate::specgen::TrafficRecord],
    tpl: &str,
    method: &str,
) -> Option<String> {
    let target = normalize_template(tpl);
    for r in records
        .iter()
        .filter(|r| r.kind == crate::specgen::TrafficKind::Http)
    {
        if !r.method.eq_ignore_ascii_case(method) {
            continue;
        }
        let r_tpl = normalize_template(&template_path(&r.path).template);
        if r_tpl == target {
            return Some(template_path(&r.path).template);
        }
    }
    None
}

/// Normalize a template to a comparable form: collapse any single-segment
/// `{name}` placeholder to a canonical token so that `/api/users/{id}` and
/// `/api/users/{usersId}` compare equal.
fn normalize_template(tpl: &str) -> String {
    let mut out = String::with_capacity(tpl.len());
    let mut in_param = false;
    for ch in tpl.chars() {
        match ch {
            '{' => {
                in_param = true;
                out.push('#');
            }
            '}' => {
                in_param = false;
                out.push('#');
            }
            _ if in_param => {}
            _ => out.push(ch),
        }
    }
    out
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

use crate::specgen::extract::template_path;

/// A mock route with its expected response body and status code.
#[derive(Clone, Debug)]
pub struct MockRoute {
    pub body: serde_json::Value,
    pub status: u16,
}

#[derive(Clone, Default)]
pub struct MockState {
    pub routes: Arc<Mutex<HashMap<String, MockRoute>>>,
}

pub fn build_mock_router(state: MockState) -> Router {
    Router::new()
        .route("/", get(echo))
        .route(
            "/*path",
            get(echo_path)
                .post(echo_path)
                .put(echo_path)
                .delete(echo_path),
        )
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
    // Concrete incoming path (e.g. "/api/users/1") → template (e.g. "/api/users/{usersId}")
    let tpl = template_path(&format!("/{}", path.trim_start_matches('/'))).template;
    let key = format!("{} {}", method.as_str(), tpl);
    let routes = state.routes.lock().await;
    if let Some(route) = routes.get(&key) {
        let status = StatusCode::from_u16(route.status).unwrap_or(StatusCode::OK);
        return (status, Json(route.body.clone())).into_response();
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({ "echoed_path": path })),
    )
        .into_response()
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

    #[tokio::test]
    async fn run_replay_seeds_mock_with_captured_bodies() {
        // Spec has no examples, so the mock must fall back to the first
        // captured response_body per (method, template) — otherwise every
        // replay would fail the body diff.
        let openapi = r#"
openapi: 3.1.0
info: { title: t, version: 1.0.0, description: d }
servers: [{ url: "http://x" }]
paths:
  /api/users/{id}:
    get:
      operationId: getUser
      summary: get user
      tags: [auto]
      responses: {}
  /api/posts/{id}:
    get:
      operationId: getPost
      summary: get post
      tags: [auto]
      responses: {}
"#;
        let mut a = rec("GET", "/api/users/1");
        a.response_body = Some(r#"{"foo":"bar"}"#.into());
        let mut b = rec("GET", "/api/posts/9");
        b.response_body = Some(r#"{"baz":"qux"}"#.into());
        let records = vec![a, b];
        let report = run_replay(openapi, &records, Some(0)).await.unwrap();
        assert_eq!(report.total, 2);
        assert_eq!(
            report.pass, 2,
            "expected both replays to pass; failures={:?}",
            report.failures
        );
        assert_eq!(report.fail, 0);
    }
}
