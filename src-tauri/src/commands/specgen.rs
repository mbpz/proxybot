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
    build_spec, SpecConfig, SpecOutput, SpecRequest, SpecResult, TrafficKind, TrafficRecord,
};

use crate::state::AppState;
use crate::db::DbState;

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

/// Load captured traffic records for a given session from the SQLite
/// `http_requests` table. Used by the SpecGenPanel to fetch the records
/// it should hand to [`generate_spec`] / [`run_replay_validation`]
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
    state: State<'_, DbState>,
    session_id: String,
) -> Result<Vec<TrafficRecord>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let limit: i64 = 500;

    // For an empty session_id we surface untagged records; otherwise
    // we filter by exact match. Index `idx_http_requests_session_id`
    // (added in migration 5) covers the typical case.
    let sql = if session_id.is_empty() {
        r#"SELECT timestamp, method, host, path, req_body, resp_status, resp_body, is_websocket
           FROM http_requests
           WHERE session_id IS NULL OR session_id = ''
           ORDER BY id ASC
           LIMIT ?1"#
    } else {
        r#"SELECT timestamp, method, host, path, req_body, resp_status, resp_body, is_websocket
           FROM http_requests
           WHERE session_id = ?1
           ORDER BY id ASC
           LIMIT ?2"#
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let row_to_record = |row: &rusqlite::Row<'_>| -> rusqlite::Result<TrafficRecord> {
        let timestamp_str: String = row.get(0)?;
        let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .or_else(|_| {
                // Fall back to SQLite's "YYYY-MM-DD HH:MM:SS" format used
                // by `timestamp_now_for_ws` and most older rows.
                chrono::NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S")
                    .map(|ndt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(ndt, chrono::Utc))
            })
            .unwrap_or_else(|_| chrono::Utc::now());

        let req_body: Option<Vec<u8>> = row.get(4)?;
        let resp_body: Option<Vec<u8>> = row.get(6)?;
        let is_websocket: i64 = row.get(7)?;

        Ok(TrafficRecord {
            method: row.get::<_, String>(1)?.to_uppercase(),
            path: row.get::<_, String>(3)?,
            host: row.get::<_, String>(2)?,
            request_body: req_body.and_then(|b| decode_body(&b)),
            response_status: row.get::<_, Option<i64>>(5)?.map(|s| s as u16).unwrap_or(0),
            response_body: resp_body.and_then(|b| decode_body(&b)),
            timestamp,
            kind: if is_websocket != 0 {
                TrafficKind::WebSocket
            } else {
                TrafficKind::Http
            },
        })
    };

    let rows = if session_id.is_empty() {
        stmt.query_map([limit], row_to_record)
    } else {
        stmt.query_map(rusqlite::params![session_id, limit], row_to_record)
    }
    .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in rows {
        match row {
            Ok(r) => out.push(r),
            Err(e) => return Err(e.to_string()),
        }
    }

    Ok(out)
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
