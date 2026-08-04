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

use std::sync::Arc;
use tauri::State;

use proxybot_core::{
    build_spec, SpecConfig, SpecOutput, SpecRequest, SpecResult, TrafficKind, TrafficRecord,
};

use crate::db::{
    CapturedRequestOrder, CapturedRequestQuery, CapturedRequestRecord, DbState, SessionScope,
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
///
/// `traffic_records` is optional: when omitted (or empty) the
/// command pulls records from the Captured Request persistence Module
/// using the same query as [`get_traffic_records`]. The UI normally
/// passes `None` so we don't pay the cost of round-tripping every
/// record through JSON twice; tests can still inject a synthetic
/// record set by passing `Some(...)`.
#[tauri::command]
pub async fn generate_spec(
    app_state: State<'_, Arc<AppState>>,
    db_state: State<'_, Arc<DbState>>,
    session_id: String,
    traffic_records: Option<Vec<TrafficRecord>>,
) -> Result<SpecResult, String> {
    let config = app_state.specgen_config_snapshot();
    let records = resolve_records(&db_state, &session_id, traffic_records)?;
    let req = SpecRequest {
        session_id: session_id.clone(),
        traffic_records: records,
        inferred: None,
    };
    let result = build_spec(req, &config).await.map_err(|e| e.to_string())?;

    // Persist the full result to disk so export_spec and
    // run_replay_validation can read it back later.
    let dir = app_state.specs_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{session_id}.json"));
    let json = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;

    Ok(result)
}

/// Read a previously-generated spec back as YAML text.
///
/// Returns the OpenAPI YAML concatenated with the AsyncAPI YAML
/// (separated by a single newline). When only one of the two is
/// present in the persisted result, just that one is returned.
///
/// The frontend uses the returned string with the standard
/// `<a download>` browser API to trigger a save dialog — keeping
/// the file-system write entirely in the webview rather than
/// having Rust write to a path the user can't easily pick. The
/// previous shape took `target_path` and silently wrote to the
/// Tauri process's cwd, which gave files an unpredictable home.
#[tauri::command]
pub async fn export_spec(
    state: State<'_, Arc<AppState>>,
    session_id: String,
) -> Result<String, String> {
    let path = state.specs_dir().join(format!("{session_id}.json"));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let result: SpecResult = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;

    let mut out = String::new();
    if let Some(SpecOutput::OpenApi(s)) = result.openapi.as_ref() {
        out.push_str(s);
        if !s.ends_with('\n') {
            out.push('\n');
        }
    }
    if let Some(SpecOutput::AsyncApi(s)) = result.asyncapi.as_ref() {
        // A YAML document separator marks the boundary so YAML
        // parsers see two documents in one file. Without it the
        // OpenAPI doc's last key would absorb the AsyncAPI top
        // keys.
        out.push_str("---\n");
        out.push_str(s);
        if !s.ends_with('\n') {
            out.push('\n');
        }
    }
    Ok(out)
}

/// Re-run replay validation against a previously-generated OpenAPI spec.
///
/// Loads the spec JSON from disk, spins up an ephemeral mock server on
/// the configured port, and replays each HTTP record through it to
/// check that status codes and response bodies still match.
#[tauri::command]
pub async fn run_replay_validation(
    app_state: State<'_, Arc<AppState>>,
    db_state: State<'_, Arc<DbState>>,
    session_id: String,
    traffic_records: Option<Vec<TrafficRecord>>,
) -> Result<ReplayReport, String> {
    let config = app_state.specgen_config_snapshot();
    let path = app_state.specs_dir().join(format!("{session_id}.json"));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let result: SpecResult = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;

    let openapi_yaml = match result.openapi.as_ref() {
        Some(SpecOutput::OpenApi(s)) => s.clone(),
        _ => return Err("no openapi spec".to_string()),
    };
    let port = config.mock_port;
    let records = resolve_records(&db_state, &session_id, traffic_records)?;
    core_run_replay(&openapi_yaml, &records, port)
        .await
        .map_err(|e| e.to_string())
}

/// Pull traffic records to feed into specgen. The UI sends `None`
/// so we read the Captured Request persistence Module; tests
/// inject a synthetic vector via `Some(...)`. An empty `Some(vec![])`
/// is treated the same as `None` because handing the heuristic a
/// truly empty vector errors out with `EmptySession`, and the UI
/// has no way to distinguish "I have nothing" from "I want you to
/// look in the DB" via the JSON wire.
fn resolve_records(
    db_state: &State<'_, Arc<DbState>>,
    session_id: &str,
    provided: Option<Vec<TrafficRecord>>,
) -> Result<Vec<TrafficRecord>, String> {
    if let Some(recs) = provided {
        if !recs.is_empty() {
            return Ok(recs);
        }
    }
    load_traffic_records(db_state, session_id)
}

/// Update the spec-generation configuration at runtime.
///
/// Lets the UI set the DeepSeek API key, tune retry counts, toggle
/// replay validation, and pick a mock port without restarting the app.
#[tauri::command]
pub fn update_specgen_config(
    state: State<'_, Arc<AppState>>,
    config: SpecConfig,
) -> Result<(), String> {
    state.set_specgen_config(config);
    Ok(())
}

/// Return the current spec-generation configuration.
///
/// Mostly useful for the UI to show "is the API key set?" without
/// exposing the actual key value.
#[tauri::command]
pub fn get_specgen_config(state: State<'_, Arc<AppState>>) -> Result<SpecConfig, String> {
    Ok(state.specgen_config_snapshot())
}

/// Mark a UI-selected `session_id` as the *active* session.
///
/// The desktop Capture Event Adapter reads this value and attributes every
/// newly recorded Captured Request to it. Pass `None` to clear attribution.
///
/// The UI calls this from `SpecGenPanel` whenever the user changes
/// the session id field, so further captures land under the new
/// session label without restarting the proxy.
#[tauri::command]
pub fn set_active_session(
    state: State<'_, Arc<AppState>>,
    session_id: Option<String>,
) -> Result<(), String> {
    // An empty string from the UI is treated the same as None so
    // the on-disk session_id column stays NULL for "untagged".
    let normalised = session_id.filter(|s| !s.is_empty());
    state.set_active_session_id(normalised);
    Ok(())
}

/// Return the currently-active session id, or `None` when nothing
/// is selected.
#[tauri::command]
pub fn get_active_session(state: State<'_, Arc<AppState>>) -> Result<Option<String>, String> {
    Ok(state.active_session_id_snapshot())
}

/// Load captured traffic records for a given session. Used by the SpecGenPanel
/// to fetch the records it should hand to [`generate_spec`] / [`run_replay_validation`]
/// without requiring the UI to pipe them through itself.
///
/// `session_id` matches the `session_id` column added by DB migration
/// `v5` (see `src-tauri/src/db.rs`). Records that were never tagged
/// with a session (column NULL) are returned when the caller asks for
/// the empty-string session `""`.
///
/// The query is bounded by a sensible cap (500) so the UI can't
/// accidentally request an unbounded blob from the database.
#[tauri::command]
pub fn get_traffic_records(
    state: State<'_, Arc<DbState>>,
    session_id: String,
) -> Result<Vec<TrafficRecord>, String> {
    load_traffic_records(&state, &session_id)
}

/// Pure DB read used by both the public [`get_traffic_records`]
/// command and the in-Rust fallback inside [`generate_spec`] /
/// [`run_replay_validation`] for when the UI omits the records.
/// Centralising the query keeps the SQL string + row decoder in
/// one place, so a schema tweak only has to change one query.
fn load_traffic_records(
    db_state: &DbState,
    session_id: &str,
) -> Result<Vec<TrafficRecord>, String> {
    let session = if session_id.is_empty() {
        SessionScope::Unassigned
    } else {
        SessionScope::Exact(session_id.to_owned())
    };
    let query = CapturedRequestQuery {
        session,
        order: CapturedRequestOrder::IdAscending,
        limit: Some(500),
        ..Default::default()
    };
    Ok(db_state
        .captured_requests(&query)?
        .iter()
        .map(traffic_record)
        .collect())
}

fn traffic_record(record: &CapturedRequestRecord) -> TrafficRecord {
    let timestamp = record.captured_at().unwrap_or_else(chrono::Utc::now);
    TrafficRecord {
        method: record.method.to_uppercase(),
        path: record.path.clone(),
        host: record.host.clone(),
        request_body: record.request_body.as_deref().and_then(decode_body),
        response_status: record.response_status.unwrap_or(0),
        response_body: record.response_body.as_deref().and_then(decode_body),
        timestamp,
        kind: if record.is_websocket {
            TrafficKind::WebSocket
        } else {
            TrafficKind::Http
        },
    }
}

/// Decode a stored request/response body into a UTF-8 string when
/// possible. Binary bodies come back as `None`; the spec generator
/// handles `None` gracefully (it only inspects JSON-ish payloads).
fn decode_body(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => Some(s.to_string()),
        Err(_) => Some(format!("[binary {} bytes]", bytes.len())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use proxybot_core::TrafficKind;

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

    /// Body decoder: UTF-8 round-trips, empty bytes are None, binary
    /// blobs come back as a size-tagged placeholder.
    #[test]
    fn decode_body_handles_utf8_empty_and_binary() {
        assert_eq!(decode_body(b""), None);
        assert_eq!(decode_body(b"hello"), Some("hello".to_string()));
        let binary = vec![0xff, 0xfe, 0xfd];
        let decoded = decode_body(&binary).unwrap();
        assert!(decoded.contains("3 bytes"), "got {decoded}");
    }
}
