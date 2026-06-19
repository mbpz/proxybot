//! Fixture-driven integration tests for the specgen heuristic pipeline.
//!
//! Owns the SM-4 acceptance check the design doc calls out (≥ 80%
//! coverage on real-shaped traffic) so it lives next to the
//! `proxybot-core` library rather than the Tauri-side `e2e/`
//! Playwright suite. Playwright still smoke-tests the React UI, but
//! the *correctness* of the generated spec — paths clustered, params
//! templated, AsyncAPI channels surfaced — is pinned here where it
//! can run on every `cargo test`.
//!
//! The fixtures under `tests/fixtures/specgen/` are intentionally
//! small (10 HTTP records + 3 WS/SSE frames) so the test runs in
//! < 100 ms. The shapes still cover the templating heuristic's
//! interesting cases:
//!
//! - numeric path segment (`/api/v3/contacts/42` → `{contactsId}`)
//! - UUID path segment (`/api/v3/sessions/<uuid>` → `{sessionsId}`)
//! - same template, multiple methods (`GET` and `DELETE` on
//!   `/api/v3/contacts/{id}`)
//! - same path, multiple records (`/api/v3/feed` × 2 → one path item)
//! - request bodies present and absent
//!
//! A regression here usually means `extract::cluster_paths` or
//! `extract::template_path` lost a case — the assertions are
//! deliberately specific so the failure points at the rule, not
//! "some YAML field is wrong".

use proxybot_core::{
    build_spec_heuristic, SpecOutput, SpecRequest, SpecSource, TrafficKind, TrafficRecord,
};

/// Load a JSON array fixture into `Vec<TrafficRecord>`.
fn load_fixture(name: &str) -> Vec<TrafficRecord> {
    let path = format!("tests/fixtures/specgen/{name}");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("read fixture {path}: {e}"));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("parse fixture {path}: {e}"))
}

#[test]
fn wechat_session_clusters_into_expected_paths() {
    // Arrange — 10 HTTP records, 5 distinct templates after
    // path-templating: profile, contacts/{id}, messages,
    // sessions/{id}, feed.
    let records = load_fixture("wechat-session.json");
    assert_eq!(records.len(), 10, "fixture shape sanity check");

    let req = SpecRequest {
        session_id: "wechat".into(),
        traffic_records: records,
        inferred: None,
    };

    // Act
    let result = build_spec_heuristic(&req).expect("heuristic build");

    // Assert: source label + presence of OpenAPI/AsyncAPI blobs.
    assert_eq!(result.source, SpecSource::Heuristic);
    let SpecOutput::OpenApi(yaml) = result
        .openapi
        .as_ref()
        .expect("openapi present for HTTP records")
    else {
        panic!("openapi variant must be OpenApi");
    };
    assert!(result.asyncapi.is_none(), "no WS records → no asyncapi");

    // Assert: all 5 templates surface, with the param-name heuristic
    // applied to numeric/UUID segments. If any of these is missing
    // the path-clustering rule regressed.
    for expected in [
        "/api/v3/user/profile",        // plain path
        "/api/v3/contacts/{contactsId}", // numeric → {prevSegment}Id
        "/api/v3/messages",
        "/api/v3/sessions/{sessionsId}", // UUID → {prevSegment}Id
        "/api/v3/feed",
    ] {
        assert!(
            yaml.contains(expected),
            "expected template `{expected}` in OpenAPI yaml, got:\n{yaml}"
        );
    }

    // Assert: same template, distinct methods both render. The
    // heuristic's operationId scheme is camelCase
    // (`<method><PascalSegments>`), produced by the
    // `heuristic_operation_id` helper so the output also passes
    // the LLM-validation schema's `^[a-z][a-zA-Z0-9]+$` regex —
    // see `heuristic_output_satisfies_llm_validation_schema` in
    // proxybot-core/src/specgen/mod.rs. Pin the actual shape so
    // any further tightening (e.g. dropping `Api` from the
    // prefix) breaks loudly and on purpose.
    assert!(yaml.contains("getApiV3ContactsContactsId"));
    assert!(yaml.contains("deleteApiV3ContactsContactsId"));
    assert!(yaml.contains("getApiV3UserProfile"));
    assert!(yaml.contains("postApiV3UserProfile"));

    // Assert: SM-4 coverage gate. Every concrete path must map back
    // to one of the emitted templates; the design doc's ≥ 80% bar is
    // a floor, but the heuristic should hit 100% on this fixture.
    assert!(
        result.coverage.coverage_rate >= 0.8,
        "coverage_rate {} below SM-4 floor (0.8)",
        result.coverage.coverage_rate
    );
    assert_eq!(result.coverage.total_requests, 10);
    assert!(
        result.coverage.uncovered_paths.is_empty(),
        "every concrete path should match a template; uncovered={:?}",
        result.coverage.uncovered_paths
    );
}

#[test]
fn ws_session_produces_asyncapi_channels() {
    // Arrange — mixed WS upgrade + SSE event.
    let records = load_fixture("ws-chat-session.json");
    assert_eq!(records.len(), 3);
    assert!(records.iter().any(|r| r.kind == TrafficKind::WebSocket));
    assert!(records.iter().any(|r| r.kind == TrafficKind::Sse));

    let req = SpecRequest {
        session_id: "ws".into(),
        traffic_records: records,
        inferred: None,
    };

    // Act
    let result = build_spec_heuristic(&req).expect("heuristic build");

    // Assert: AsyncAPI surfaced both channels. The HTTP path is
    // empty here because every record is WS or SSE, so the
    // OpenAPI blob is the empty-paths skeleton — that's expected.
    let SpecOutput::AsyncApi(yaml) = result
        .asyncapi
        .as_ref()
        .expect("asyncapi present for WS/SSE records")
    else {
        panic!("asyncapi variant must be AsyncApi");
    };
    assert!(yaml.contains("/ws/chat"), "ws channel missing:\n{yaml}");
    assert!(yaml.contains("/sse/feed"), "sse channel missing:\n{yaml}");
    // The SSE record should land under the `subscribe` shape; the
    // heuristic groups WebSocket and SSE the same way (one-way
    // server-to-client) so we just check the subscribe key is
    // there for at least one of them.
    assert!(yaml.contains("subscribe:"));
}

#[test]
fn mixed_session_yields_both_openapi_and_asyncapi() {
    // Arrange — splice the two fixtures so the orchestrator sees a
    // realistic mixed session: HTTP API + WS chat. This is the
    // shape `SpecGenPanel` will hand it in production.
    let mut records = load_fixture("wechat-session.json");
    records.extend(load_fixture("ws-chat-session.json"));

    let req = SpecRequest {
        session_id: "mixed".into(),
        traffic_records: records,
        inferred: None,
    };

    // Act
    let result = build_spec_heuristic(&req).expect("heuristic build");

    // Assert: both blobs come out, source is Heuristic (no LLM), and
    // coverage stays at the SM-4 floor. The mixed fixture is the
    // closest thing we have to a "real session" before the LLM
    // path is exercised end-to-end with a mock DeepSeek.
    assert!(result.openapi.is_some());
    assert!(result.asyncapi.is_some());
    assert_eq!(result.source, SpecSource::Heuristic);
    assert!(
        result.coverage.coverage_rate >= 0.8,
        "mixed-session coverage {} below SM-4 floor",
        result.coverage.coverage_rate
    );
}
