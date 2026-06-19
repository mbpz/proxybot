//! OpenAPI/AsyncAPI spec generation from captured traffic.
//!
//! See `docs/superpowers/specs/2026-06-16-openapi-asyncapi-generation-design.md`
//! for the design.

pub mod config;
pub mod coverage;
pub mod error;
pub mod extract;
pub mod llm;
pub mod render;
pub mod replay;
pub mod validate;

use crate::specgen::llm::DeepSeekClient;
use crate::specgen::validate::validate_paths_object;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use config::SpecConfig;
pub use coverage::{CoverageReport, SpecSource};
pub use error::SpecError;
pub use render::{
    AsyncApiChannel, AsyncApiDoc, AsyncApiExample, AsyncApiMessage, OpenApiDoc, OpenApiInfo,
    OpenApiMediaType, OpenApiOperation, OpenApiParameter, OpenApiPathItem, OpenApiResponse,
    OpenApiSchema, OpenApiServer,
};
pub use replay::{ReplayFailure, ReplayReport};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficRecord {
    pub method: String,
    pub path: String,
    pub host: String,
    pub request_body: Option<String>,
    pub response_status: u16,
    pub response_body: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub kind: TrafficKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrafficKind {
    Http,
    WebSocket,
    Sse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredSemantics {
    pub interfaces: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecRequest {
    pub session_id: String,
    pub traffic_records: Vec<TrafficRecord>,
    pub inferred: Option<InferredSemantics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecOutput {
    OpenApi(String),
    AsyncApi(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecResult {
    pub openapi: Option<SpecOutput>,
    pub asyncapi: Option<SpecOutput>,
    pub coverage: CoverageReport,
    pub replay: Option<crate::specgen::replay::ReplayReport>,
    pub generated_at: DateTime<Utc>,
    pub source: SpecSource,
    /// Human-readable reason the LLM path was skipped or failed
    /// when the result fell back to (or merged with) the heuristic.
    /// `None` means the LLM round succeeded cleanly. The UI shows
    /// this as a yellow banner above the path list so users
    /// understand why the source badge says `Heuristic` instead of
    /// `Llm`. Always `None` for the pure-heuristic entrypoint
    /// (`build_spec_heuristic`) — that path has no LLM to degrade
    /// from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degradation_reason: Option<String>,
}

/// Heuristic-only build: produces a spec from `traffic_records` with no LLM call.
/// Used for the fallback path and for unit testing the full pipeline.
pub fn build_spec_heuristic(req: &SpecRequest) -> Result<SpecResult, SpecError> {
    if req.traffic_records.is_empty() {
        return Err(SpecError::EmptySession);
    }

    let http: Vec<&TrafficRecord> = req
        .traffic_records
        .iter()
        .filter(|r| r.kind == TrafficKind::Http)
        .collect();
    let ws: Vec<&TrafficRecord> = req
        .traffic_records
        .iter()
        .filter(|r| r.kind == TrafficKind::WebSocket || r.kind == TrafficKind::Sse)
        .collect();

    // --- OpenAPI ---
    let mut paths: BTreeMap<String, render::OpenApiPathItem> = BTreeMap::new();
    let rec_pairs: Vec<(String, String)> = http
        .iter()
        .map(|r| (r.method.clone(), r.path.clone()))
        .collect();
    let clustered = extract::cluster_paths(&rec_pairs);
    for (tpl, methods) in &clustered {
        let mut item = render::OpenApiPathItem::default();
        for (m, _example_path) in methods {
            let op = render::OpenApiOperation {
                operation_id: format!(
                    "{}{}",
                    m.to_lowercase(),
                    tpl.replace(['/', '{', '}'], "_").trim_matches('_')
                ),
                summary: format!("{m} {tpl}"),
                tags: vec!["auto".into()],
                parameters: vec![],
                responses: BTreeMap::new(),
            };
            match m.as_str() {
                "GET" => item.get = Some(op),
                "POST" => item.post = Some(op),
                "PUT" => item.put = Some(op),
                "DELETE" => item.delete = Some(op),
                "PATCH" => item.patch = Some(op),
                _ => {}
            }
        }
        paths.insert(tpl.clone(), item);
    }
    let openapi_templates: Vec<String> = paths.keys().cloned().collect();
    let openapi_yaml = render::render_openapi(
        &format!("ProxyBot spec for {}", req.session_id),
        "https://api.example.com",
        paths,
        SpecSource::Heuristic,
    );

    // --- AsyncAPI ---
    let mut channels: BTreeMap<String, render::AsyncApiChannel> = BTreeMap::new();
    for r in &ws {
        channels.insert(
            r.path.clone(),
            render::AsyncApiChannel {
                description: format!("{} channel", r.kind_str()),
                subscribe: Some(render::AsyncApiMessage {
                    payload: render::OpenApiSchema {
                        schema_type: Some("object".into()),
                        properties: None,
                        example: r
                            .response_body
                            .as_deref()
                            .and_then(|b| serde_json::from_str(b).ok()),
                    },
                    examples: None,
                }),
                publish: None,
            },
        );
    }
    let asyncapi_yaml = if !channels.is_empty() {
        Some(render::render_asyncapi("WS", "wss://api.example.com", channels))
    } else {
        None
    };

    // `all_paths` feeds the coverage check; it has to span every
    // record (HTTP + WS + SSE), not just HTTP. If we only passed
    // `http` here a mixed session of 10 HTTP + 3 WS records would
    // report coverage_rate = 10/13 ≈ 0.77 even when the WS frames
    // were channeled correctly — the AsyncAPI match column would
    // see no concrete WS paths to count. Pull every kind in.
    let all_paths: Vec<String> = req
        .traffic_records
        .iter()
        .map(|r| r.path.clone())
        .collect();
    let asyncapi_channels: Vec<String> = ws.iter().map(|r| r.path.clone()).collect();
    let coverage = CoverageReport::compute(
        req.traffic_records.len(),
        &openapi_templates,
        &asyncapi_channels,
        &all_paths,
    );

    Ok(SpecResult {
        openapi: Some(SpecOutput::OpenApi(openapi_yaml)),
        asyncapi: asyncapi_yaml.map(SpecOutput::AsyncApi),
        coverage,
        replay: None,
        generated_at: Utc::now(),
        source: SpecSource::Heuristic,
        // Pure-heuristic path: there's no LLM round to degrade
        // from, so leave the reason empty. `build_spec` overrides
        // this when it falls back here.
        degradation_reason: None,
    })
}

/// End-to-end orchestrator. Calls DeepSeek for OpenAPI in sequence,
/// validates responses, renders YAML, then optionally runs replay.
pub async fn build_spec(req: SpecRequest, config: &SpecConfig) -> Result<SpecResult, SpecError> {
    if req.traffic_records.is_empty() {
        return Err(SpecError::EmptySession);
    }

    // Try LLM path; fall back to heuristic. We track the reason
    // for any degradation so the UI can surface it as a banner.
    let api_key = match config
        .deepseek_api_key
        .clone()
        .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
    {
        Some(k) => k,
        None => {
            // No key configured — go straight to heuristic with a
            // friendly explanation rather than erroring out. This
            // matches the design doc's §8 "downgrade gracefully"
            // contract.
            let mut result = build_spec_heuristic(&req)?;
            result.degradation_reason =
                Some("LLM 不可用：未设置 DEEPSEEK_API_KEY，已用启发式生成".into());
            return finalise_with_replay(result, &req, config).await;
        }
    };

    let client = DeepSeekClient::new(api_key);
    let schema = serde_json::json!({
        "type": "object",
        "required": ["paths"],
        "properties": {
            "paths": { "type": "object", "additionalProperties": true }
        }
    });

    let user_payload = build_user_payload(&req);
    let llm_attempt = client
        .call_with_schema(SYSTEM_PROMPT, &user_payload, &schema, config.max_retry)
        .await;

    // (yaml, source, optional degradation_reason)
    let (openapi_yaml, source, degradation): (String, SpecSource, Option<String>) = match llm_attempt {
        Ok(v) => match validate_paths_object(&v) {
            Ok(()) => {
                // LLM succeeded. Also run heuristic, then merge: LLM paths win,
                // heuristic fills in any (method, template) the LLM missed.
                let llm_paths_map = v.get("paths").cloned().unwrap_or(serde_json::json!({}));
                let heuristic = build_spec_heuristic(&req)?;
                let merged_map = merge_paths(&llm_paths_map, &heuristic);
                let label = if merged_map.used_heuristic {
                    SpecSource::Hybrid
                } else {
                    SpecSource::Llm
                };
                let rendered = render_paths_as_openapi(
                    &merged_map.paths,
                    &req.session_id,
                    label,
                );
                // Hybrid is "good enough" — no banner. Pure Llm
                // also clean. We only flag a reason on Heuristic
                // fallback below.
                (rendered, label, None)
            }
            Err(e) => {
                let r = build_spec_heuristic(&req)?;
                let reason = format!(
                    "LLM 输出不符合 schema，已用启发式生成（覆盖度可能较低）: {}",
                    truncate_reason(&e.to_string())
                );
                (extract_openapi_yaml(&r), r.source, Some(reason))
            }
        },
        Err(e) => {
            let r = build_spec_heuristic(&req)?;
            let reason = format!(
                "LLM 调用失败，已用启发式生成: {}",
                truncate_reason(&e.to_string())
            );
            (extract_openapi_yaml(&r), r.source, Some(reason))
        }
    };

    // AsyncAPI: try LLM, fall back to heuristic on failure or no frames.
    // Only attempt LLM if the OpenAPI source indicates LLM was reachable
    // (Llm or Hybrid); on Heuristic fallback the LLM round already failed
    // so a doomed AsyncAPI call would just add latency.
    let mut result = build_spec_heuristic(&req)?;
    result.source = source;
    result.openapi = Some(SpecOutput::OpenApi(openapi_yaml));
    result.degradation_reason = degradation;

    if matches!(source, SpecSource::Llm | SpecSource::Hybrid) {
        if let Some(asyncapi_yaml) =
            build_asyncapi_with_llm(&req, &client, config.max_retry).await
        {
            result.asyncapi = Some(SpecOutput::AsyncApi(asyncapi_yaml));
        }
    }

    finalise_with_replay(result, &req, config).await
}

/// Truncate noisy error messages so the UI banner stays one line.
fn truncate_reason(s: &str) -> String {
    const MAX: usize = 200;
    if s.len() <= MAX {
        s.to_string()
    } else {
        format!("{}…", &s[..MAX])
    }
}

/// Run replay validation if enabled and stash the report on the
/// result. Shared by every `build_spec` exit path so we don't
/// duplicate the toggle check.
async fn finalise_with_replay(
    mut result: SpecResult,
    req: &SpecRequest,
    config: &SpecConfig,
) -> Result<SpecResult, SpecError> {
    if config.enable_replay_validation {
        if let Some(SpecOutput::OpenApi(ref yaml)) = result.openapi {
            let replay = replay::run_replay(yaml, &req.traffic_records, config.mock_port).await?;
            result.replay = Some(replay);
        }
    }
    Ok(result)
}

/// Result of merging LLM-emitted paths with heuristic paths.
struct MergedPaths {
    /// JSON value shaped like `{ "<path>": { <path item> }, ... }`, suitable
    /// for `render_paths_as_openapi`.
    paths: serde_json::Value,
    /// True if any path was filled in from the heuristic because the LLM
    /// didn't include it. Used to decide between `SpecSource::Llm` and
    /// `SpecSource::Hybrid`.
    used_heuristic: bool,
}

/// Merge LLM paths with heuristic paths. LLM paths take precedence. Heuristic
/// fills in any path template the LLM missed.
fn merge_paths(
    llm_paths: &serde_json::Value,
    heuristic: &SpecResult,
) -> MergedPaths {
    use std::collections::BTreeMap;
    // Convert the LLM's "paths" object into typed path items.
    let mut typed: BTreeMap<String, render::OpenApiPathItem> = BTreeMap::new();
    if let Some(obj) = llm_paths.as_object() {
        for (k, v) in obj {
            let item: render::OpenApiPathItem =
                serde_json::from_value(v.clone()).unwrap_or_default();
            typed.insert(k.clone(), item);
        }
    }
    // Pull heuristic paths from the heuristic SpecResult.
    let heuristic_paths = match heuristic.openapi.as_ref() {
        Some(SpecOutput::OpenApi(yaml)) => {
            // Re-parse the heuristic YAML to recover the typed path items.
            let doc: serde_json::Value =
                serde_json::to_value(serde_yaml::from_str::<serde_yaml::Value>(yaml).ok())
                    .unwrap_or(serde_json::json!({}));
            doc.get("paths").cloned().unwrap_or(serde_json::json!({}))
        }
        _ => serde_json::json!({}),
    };

    let mut used_heuristic = false;
    if let Some(obj) = heuristic_paths.as_object() {
        for (k, v) in obj {
            if !typed.contains_key(k) {
                let item: render::OpenApiPathItem =
                    serde_json::from_value(v.clone()).unwrap_or_default();
                typed.insert(k.clone(), item);
                used_heuristic = true;
            }
        }
    }

    // Re-emit as a JSON object shaped like the LLM's paths input.
    let mut out = serde_json::Map::new();
    for (k, v) in &typed {
        out.insert(
            k.clone(),
            serde_json::to_value(v).unwrap_or(serde_json::json!({})),
        );
    }
    MergedPaths {
        paths: serde_json::Value::Object(out),
        used_heuristic,
    }
}

const SYSTEM_PROMPT: &str = "你是 API 规范生成助手。根据用户提供的流量记录和（可选的）已推断的接口语义，输出符合 JSON Schema 的 OpenAPI 3.1 路径对象。\n\n如果用户提供了 `inferred` 字段，优先采用其中的接口名、operationId 和 tags，不要自己重新命名。\n\n规则：\n- 路径必须用 {param} 模板化（如 /api/user/123 → /api/user/{id}）\n- 不臆造字段，只在流量中实际出现的字段才写\n- 每个接口给 operationId (camelCase)、summary、tags\n- 至少 1 个 example（从流量 body 取）\n- 中文 summary";

const ASYNCAPI_SYSTEM_PROMPT: &str = "你是 AsyncAPI 规范生成助手。根据用户提供的 WebSocket / SSE 流量，输出符合 JSON Schema 的 AsyncAPI 2.6 channels 对象。\n\n规则：\n- 路径用 {param} 模板化\n- 不臆造字段，只在流量中实际出现的字段才写\n- 每个 channel 给 description（中文）+ subscribe.message.payload\n- 至少 1 个 example（从流量帧 body 取）";

fn asyncapi_user_payload(req: &SpecRequest) -> String {
    let mut payload = serde_json::Map::new();
    payload.insert("session_id".into(), serde_json::json!(req.session_id));
    let frames: Vec<serde_json::Value> = req
        .traffic_records
        .iter()
        .filter(|r| r.kind == TrafficKind::WebSocket || r.kind == TrafficKind::Sse)
        .take(50)
        .map(|r| {
            serde_json::json!({
                "kind": r.kind_str(),
                "path": r.path,
                "host": r.host,
                "body": r.response_body,
            })
        })
        .collect();
    payload.insert("frames".into(), serde_json::json!(frames));
    serde_json::to_string(&payload).unwrap_or_default()
}

fn asyncapi_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["channels"],
        "properties": {
            "channels": { "type": "object", "additionalProperties": true }
        }
    })
}

/// Optional LLM call for AsyncAPI. Returns rendered YAML if successful,
/// `None` to signal "fall back to heuristic" (the orchestrator will then
/// keep the heuristic AsyncAPI it already built). Returning `None` also
/// covers no-frames sessions — the LLM call is short-circuited so we
/// don't waste a network round-trip on HTTP-only traffic.
async fn build_asyncapi_with_llm(
    req: &SpecRequest,
    client: &DeepSeekClient,
    max_retry: u32,
) -> Option<String> {
    let frames_count = req
        .traffic_records
        .iter()
        .filter(|r| r.kind == TrafficKind::WebSocket || r.kind == TrafficKind::Sse)
        .count();
    if frames_count == 0 {
        return None;
    }
    let payload = asyncapi_user_payload(req);
    let schema = asyncapi_schema();
    let v = client
        .call_with_schema(ASYNCAPI_SYSTEM_PROMPT, &payload, &schema, max_retry)
        .await
        .ok()?;
    let channels_value = v.get("channels")?.clone();
    Some(render_channels_as_asyncapi(&channels_value, &req.session_id))
}

fn render_channels_as_asyncapi(channels_map: &serde_json::Value, session_id: &str) -> String {
    use std::collections::BTreeMap;
    let mut typed: BTreeMap<String, render::AsyncApiChannel> = BTreeMap::new();
    if let Some(obj) = channels_map.as_object() {
        for (k, v) in obj {
            let item: render::AsyncApiChannel = serde_json::from_value(v.clone())
                .unwrap_or_else(|_| render::AsyncApiChannel {
                    description: format!("LLM channel for {k}"),
                    subscribe: None,
                    publish: None,
                });
            typed.insert(k.clone(), item);
        }
    }
    render::render_asyncapi(
        &format!("ProxyBot AsyncAPI for {session_id}"),
        "wss://api.example.com",
        typed,
    )
}

fn build_user_payload(req: &SpecRequest) -> String {
    let mut payload = serde_json::Map::new();
    payload.insert("session_id".into(), serde_json::json!(req.session_id));
    if let Some(inferred) = &req.inferred {
        payload.insert("inferred".into(), serde_json::json!(inferred.interfaces));
    }
    let simplified: Vec<serde_json::Value> = req
        .traffic_records
        .iter()
        .take(50)
        .map(|r| {
            serde_json::json!({
                "method": r.method,
                "path": r.path,
                "host": r.host,
                "status": r.response_status,
            })
        })
        .collect();
    payload.insert("traffic".into(), serde_json::json!(simplified));
    serde_json::to_string(&payload).unwrap_or_default()
}

fn render_paths_as_openapi(
    paths_map: &serde_json::Value,
    session_id: &str,
    source: SpecSource,
) -> String {
    use std::collections::BTreeMap;
    let mut typed: BTreeMap<String, render::OpenApiPathItem> = BTreeMap::new();
    if let Some(obj) = paths_map.as_object() {
        for (k, v) in obj {
            let item: render::OpenApiPathItem = serde_json::from_value(v.clone()).unwrap_or_default();
            typed.insert(k.clone(), item);
        }
    }
    render::render_openapi(
        &format!("ProxyBot spec for {session_id}"),
        "https://api.example.com",
        typed,
        source,
    )
}

fn extract_openapi_yaml(r: &SpecResult) -> String {
    match r.openapi.as_ref() {
        Some(SpecOutput::OpenApi(s)) => s.clone(),
        _ => String::new(),
    }
}

impl TrafficRecord {
    pub fn kind_str(&self) -> &'static str {
        match self.kind {
            TrafficKind::Http => "HTTP",
            TrafficKind::WebSocket => "WebSocket",
            TrafficKind::Sse => "SSE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(method: &str, path: &str, kind: TrafficKind) -> TrafficRecord {
        TrafficRecord {
            method: method.into(),
            path: path.into(),
            host: "api.example.com".into(),
            request_body: None,
            response_status: 200,
            response_body: Some(r#"{"ok":true}"#.into()),
            timestamp: Utc::now(),
            kind,
        }
    }

    #[test]
    fn empty_session_errors() {
        let req = SpecRequest {
            session_id: "s".into(),
            traffic_records: vec![],
            inferred: None,
        };
        assert!(build_spec_heuristic(&req).is_err());
    }

    #[test]
    fn http_only_produces_openapi_no_asyncapi() {
        let req = SpecRequest {
            session_id: "s".into(),
            traffic_records: vec![
                rec("GET", "/api/users/1", TrafficKind::Http),
                rec("GET", "/api/users/2", TrafficKind::Http),
            ],
            inferred: None,
        };
        let r = build_spec_heuristic(&req).unwrap();
        assert!(r.openapi.is_some());
        assert!(r.asyncapi.is_none());
        assert_eq!(r.source, SpecSource::Heuristic);
    }

    #[test]
    fn ws_records_produce_asyncapi() {
        let req = SpecRequest {
            session_id: "s".into(),
            traffic_records: vec![rec("GET", "/ws/chat", TrafficKind::WebSocket)],
            inferred: None,
        };
        let r = build_spec_heuristic(&req).unwrap();
        assert!(r.asyncapi.is_some());
    }

    #[test]
    fn user_payload_includes_inferred_semantics() {
        let inferred = InferredSemantics {
            interfaces: vec![serde_json::json!({
                "name": "userProfile",
                "method": "GET",
                "path": "/api/v3/user/profile",
            })],
        };
        let req = SpecRequest {
            session_id: "session-xyz".into(),
            traffic_records: vec![rec("GET", "/api/v3/user/profile", TrafficKind::Http)],
            inferred: Some(inferred),
        };

        let payload = build_user_payload(&req);
        assert!(
            payload.contains("\"inferred\""),
            "payload missing inferred key: {payload}"
        );
        assert!(
            payload.contains("userProfile"),
            "payload missing inferred interface name: {payload}"
        );
        assert!(
            payload.contains("session-xyz"),
            "payload missing session_id: {payload}"
        );

        // Without inferred, the key should be absent.
        let req_none = SpecRequest {
            inferred: None,
            ..req
        };
        let payload_none = build_user_payload(&req_none);
        assert!(
            !payload_none.contains("\"inferred\""),
            "payload should not include inferred key when None: {payload_none}"
        );
    }

    #[tokio::test]
    async fn build_asyncapi_with_llm_renders_channels_on_success() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{
                    "message": { "content": r#"{"channels":{"/ws/chat":{"description":"chat","subscribe":{"payload":{"type":"object"}}}}}"# }
                }]
            })))
            .mount(&server)
            .await;

        let client = DeepSeekClient {
            api_key: "sk-test".into(),
            endpoint: format!("{}/v1/chat/completions", server.uri()),
            http: reqwest::Client::new(),
        };
        let req = SpecRequest {
            session_id: "s".into(),
            traffic_records: vec![rec("GET", "/ws/chat", TrafficKind::WebSocket)],
            inferred: None,
        };
        let yaml = build_asyncapi_with_llm(&req, &client, 0)
            .await
            .expect("returns Some");
        assert!(yaml.contains("/ws/chat"));
        assert!(yaml.contains("asyncapi: 2.6.0"));
    }

    #[tokio::test]
    async fn build_asyncapi_with_llm_returns_none_for_no_frames() {
        // HTTP-only session: short-circuits before any network call.
        let client = DeepSeekClient::new("sk-test".into());
        let req = SpecRequest {
            session_id: "s".into(),
            traffic_records: vec![rec("GET", "/api/users/1", TrafficKind::Http)],
            inferred: None,
        };
        assert!(build_asyncapi_with_llm(&req, &client, 0).await.is_none());
    }
}
