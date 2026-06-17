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

    let all_paths: Vec<String> = http.iter().map(|r| r.path.clone()).collect();
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
    })
}

/// End-to-end orchestrator. Calls DeepSeek for OpenAPI in sequence,
/// validates responses, renders YAML, then optionally runs replay.
pub async fn build_spec(req: SpecRequest, config: &SpecConfig) -> Result<SpecResult, SpecError> {
    if req.traffic_records.is_empty() {
        return Err(SpecError::EmptySession);
    }

    // Try LLM path; fall back to heuristic.
    let api_key = config
        .deepseek_api_key
        .clone()
        .or_else(|| std::env::var("DEEPSEEK_API_KEY").ok())
        .ok_or_else(|| SpecError::LlmUnavailable("DEEPSEEK_API_KEY not set".into()))?;

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

    let (openapi_yaml, source) = match llm_attempt {
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
                (rendered, label)
            }
            Err(_) => {
                let r = build_spec_heuristic(&req)?;
                (extract_openapi_yaml(&r), r.source)
            }
        },
        Err(_) => {
            let r = build_spec_heuristic(&req)?;
            (extract_openapi_yaml(&r), r.source)
        }
    };

    // AsyncAPI is always heuristic for now (LLM call is future work).
    let mut result = build_spec_heuristic(&req)?;
    result.source = source;
    result.openapi = Some(SpecOutput::OpenApi(openapi_yaml));

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

const SYSTEM_PROMPT: &str = "你是 API 规范生成助手。根据用户提供的流量记录，输出符合 JSON Schema 的 OpenAPI 3.1 路径对象。\n规则：\n- 路径必须用 {param} 模板化（如 /api/user/123 → /api/user/{id}）\n- 不臆造字段，只在流量中实际出现的字段才写\n- 每个接口给 operationId (camelCase)、summary、tags\n- 至少 1 个 example（从流量 body 取）\n- 中文 summary";

fn build_user_payload(req: &SpecRequest) -> String {
    let mut payload = serde_json::json!({
        "traffic": req.traffic_records.iter().take(50).map(|r| {
            serde_json::json!({
                "method": r.method,
                "path": r.path,
                "host": r.host,
                "status": r.response_status,
            })
        }).collect::<Vec<_>>(),
    });

    // Include inferred semantics when available
    if let Some(ref inferred) = req.inferred {
        payload["inferred"] = serde_json::json!(inferred.interfaces);
    }

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
}
