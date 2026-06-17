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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use config::SpecConfig;
pub use coverage::{CoverageReport, SpecSource};
pub use error::SpecError;

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
