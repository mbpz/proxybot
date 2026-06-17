//! Tauri commands for the OpenAPI / AsyncAPI spec generator.
//!
//! Three commands are exposed to the React frontend:
//!
//! - [`generate_spec`] – runs the full pipeline (LLM -> heuristic fallback)
//!   for a session's traffic and persists the result to `~/.proxybot/specs/`.
//! - [`export_spec`] – writes a session's generated OpenAPI / AsyncAPI YAML
//!   out to a user-chosen path on disk.
//! - [`run_replay_validation`] – re-runs the mock-server replay check
//!   against a previously-generated spec.
//!
//! A small bonus command [`update_specgen_config`] lets the UI override
//! the LLM API key, retry count, and replay toggles at runtime without
//! restarting the app.

use tauri::State;

use proxybot_core::{
    build_spec, SpecConfig, SpecOutput, SpecRequest, SpecResult, TrafficRecord,
};

use crate::state::AppState;

use proxybot_core::specgen::replay::run_replay as core_run_replay;
use proxybot_core::ReplayReport;

/// Generate an OpenAPI + AsyncAPI spec for the given session's traffic.
///
/// The session is identified by `session_id` (used as the file name and
/// embedded in the spec's `info.title`). The full result is persisted
/// to `~/.proxybot/specs/<session_id>.json` and also returned to the
/// caller so the UI can display it without a second round-trip.
#[tauri::command]
pub async fn generate_spec(
    state: State<'_, AppState>,
    session_id: String,
    traffic_records: Vec<TrafficRecord>,
) -> Result<SpecResult, String> {
    let config = state.specgen_config_snapshot();
    let req = SpecRequest {
        session_id: session_id.clone(),
        traffic_records,
        inferred: None,
    };
    let result = build_spec(req, &config)
        .await
        .map_err(|e| e.to_string())?;

    // Persist the full result to disk so export_spec and
    // run_replay_validation can read it back later.
    let dir = state.specs_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{session_id}.json"));
    let json = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;

    Ok(result)
}

/// Export a previously-generated spec to a user-supplied path on disk.
///
/// Reads the JSON for the session, pulls out the OpenAPI and AsyncAPI
/// YAML blobs, concatenates them with a single newline, and writes the
/// combined output. If only one of the two is present, just that one
/// is written.
#[tauri::command]
pub async fn export_spec(
    state: State<'_, AppState>,
    session_id: String,
    target_path: String,
) -> Result<(), String> {
    let path = state.specs_dir().join(format!("{session_id}.json"));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let result: SpecResult = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;

    let mut out = String::new();
    if let Some(p) = &result.openapi {
        if let SpecOutput::OpenApi(s) = p {
            out.push_str(s);
            out.push('\n');
        }
    }
    if let Some(p) = &result.asyncapi {
        if let SpecOutput::AsyncApi(s) = p {
            out.push_str(s);
            out.push('\n');
        }
    }
    std::fs::write(&target_path, out).map_err(|e| e.to_string())?;
    Ok(())
}

/// Re-run replay validation against a previously-generated OpenAPI spec.
///
/// Loads the spec JSON from disk, spins up an ephemeral mock server on
/// the configured port, and replays each HTTP record through it to
/// check that status codes and response bodies still match.
#[tauri::command]
pub async fn run_replay_validation(
    state: State<'_, AppState>,
    session_id: String,
    traffic_records: Vec<TrafficRecord>,
) -> Result<ReplayReport, String> {
    let config = state.specgen_config_snapshot();
    let path = state.specs_dir().join(format!("{session_id}.json"));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let result: SpecResult = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;

    let openapi_yaml = match result.openapi.as_ref() {
        Some(SpecOutput::OpenApi(s)) => s.clone(),
        _ => return Err("no openapi spec".to_string()),
    };
    let port = config.mock_port;
    core_run_replay(&openapi_yaml, &traffic_records, port)
        .await
        .map_err(|e| e.to_string())
}

/// Update the spec-generation configuration at runtime.
///
/// Lets the UI set the DeepSeek API key, tune retry counts, toggle
/// replay validation, and pick a mock port without restarting the app.
#[tauri::command]
pub fn update_specgen_config(state: State<'_, AppState>, config: SpecConfig) -> Result<(), String> {
    state.set_specgen_config(config);
    Ok(())
}

/// Return the current spec-generation configuration.
///
/// Mostly useful for the UI to show "is the API key set?" without
/// exposing the actual key value.
#[tauri::command]
pub fn get_specgen_config(
    state: State<'_, AppState>,
) -> Result<SpecConfig, String> {
    Ok(state.specgen_config_snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proxybot_core::TrafficKind;
    use chrono::Utc;

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

    /// Smoke test for the spec-persistence path. We don't go through
    /// the Tauri `State` wrapper (it needs a full Tauri runtime);
    /// instead we exercise the build + persist logic directly using
    /// the public `AppState` API. The LLM call will fail without an
    /// API key, so we use the heuristic-only `build_spec_heuristic`
    /// for the round-trip.
    #[test]
    fn build_heuristic_then_persist_round_trip() {
        use proxybot_core::build_spec_heuristic;

        let req = proxybot_core::SpecRequest {
            session_id: "s".into(),
            traffic_records: vec![rec("GET", "/api/users/1", TrafficKind::Http)],
            inferred: None,
        };
        let result = build_spec_heuristic(&req).expect("heuristic build succeeds");

        // Persist + read back.
        let tmp = std::env::temp_dir().join(format!(
            "proxybot-specgen-rs-test-{}-{}",
            std::process::id(),
            "session-abc"
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("session-abc.json");
        let json = serde_json::to_string_pretty(&result).unwrap();
        std::fs::write(&path, &json).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let back: SpecResult = serde_json::from_slice(&bytes).unwrap();
        assert!(back.openapi.is_some());
    }
}
