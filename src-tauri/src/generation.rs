//! Shared inference-session input and generated-artifact conventions.
//!
//! A single SQLite read transaction builds [`InferenceSessionSnapshot`]. Spec,
//! Mock, Scaffold, Vision and Deployment Adapters consume that immutable value
//! instead of reloading and remapping session rows independently.

use crate::db::{
    captured_requests_with, CapturedRequestOrder, CapturedRequestQuery, CapturedRequestRecord,
    DbState, SessionScope,
};
use chrono::Utc;
use proxybot_core::specgen::InferredSemantics;
use proxybot_core::{TrafficKind, TrafficRecord};
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// One inferred API persisted for an inference session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InferredApi {
    pub id: i64,
    pub session_id: String,
    pub name: String,
    pub method: String,
    pub path: String,
    pub params: String,
    pub auth_required: bool,
    pub request_ids: String,
    pub score: Option<f64>,
    pub created_at: String,
}

/// Immutable input shared by every generated-artifact transformation.
#[derive(Debug, Clone, PartialEq)]
pub struct InferenceSessionSnapshot {
    pub session_id: String,
    pub inferred_apis: Vec<InferredApi>,
    pub captured_requests: Vec<CapturedRequestRecord>,
}

impl InferenceSessionSnapshot {
    pub fn require_inferred_apis(&self) -> Result<&[InferredApi], String> {
        if self.inferred_apis.is_empty() {
            Err("No inferred APIs found for this session. Run API inference first.".to_owned())
        } else {
            Ok(&self.inferred_apis)
        }
    }

    /// Canonical Spec projection of the same persisted Captured Requests.
    pub fn traffic_records(&self) -> Vec<TrafficRecord> {
        self.captured_requests
            .iter()
            .map(|record| {
                let timestamp = record.captured_at().unwrap_or_else(Utc::now);
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
            })
            .collect()
    }

    /// Canonical Spec projection of stored inference semantics.
    pub fn inferred_semantics(&self) -> Option<InferredSemantics> {
        if self.inferred_apis.is_empty() {
            return None;
        }
        Some(InferredSemantics {
            interfaces: self
                .inferred_apis
                .iter()
                .map(|api| {
                    serde_json::json!({
                        "id": api.id,
                        "name": api.name,
                        "method": api.method,
                        "path": api.path,
                        "params": api.params,
                        "auth_required": api.auth_required,
                        "request_ids": api.request_ids,
                        "score": api.score,
                    })
                })
                .collect(),
        })
    }
}

impl DbState {
    /// Load one inference session under a single SQLite read transaction.
    pub fn inference_session(&self, session_id: &str) -> Result<InferenceSessionSnapshot, String> {
        let mut conn = self.conn.lock().map_err(|error| error.to_string())?;
        let transaction = conn.transaction().map_err(|error| error.to_string())?;
        let snapshot = load_inference_session(&transaction, session_id)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(snapshot)
    }

    /// Query persisted inference results through their authoritative row mapper.
    pub fn inferred_apis(&self, session_id: Option<&str>) -> Result<Vec<InferredApi>, String> {
        let conn = self.conn.lock().map_err(|error| error.to_string())?;
        load_inferred_apis(&conn, session_id)
    }
}

pub(crate) fn load_inference_session(
    conn: &Connection,
    session_id: &str,
) -> Result<InferenceSessionSnapshot, String> {
    let session = if session_id.is_empty() {
        SessionScope::Unassigned
    } else {
        SessionScope::Exact(session_id.to_owned())
    };
    let captured_requests = captured_requests_with(
        conn,
        &CapturedRequestQuery {
            session,
            order: CapturedRequestOrder::IdAscending,
            ..Default::default()
        },
    )?;
    let inferred_apis = load_inferred_apis(conn, Some(session_id))?;
    Ok(InferenceSessionSnapshot {
        session_id: session_id.to_owned(),
        inferred_apis,
        captured_requests,
    })
}

pub(crate) fn load_inferred_apis(
    conn: &Connection,
    session_id: Option<&str>,
) -> Result<Vec<InferredApi>, String> {
    let (sql, parameter) = match session_id {
        Some(session_id) => (
            "SELECT id, session_id, name, method, path, params, auth_required, request_ids, score, created_at
             FROM inferred_apis WHERE session_id = ?1 ORDER BY id",
            Some(session_id),
        ),
        None => (
            "SELECT id, session_id, name, method, path, params, auth_required, request_ids, score, created_at
             FROM inferred_apis ORDER BY id",
            None,
        ),
    };
    let mut statement = conn.prepare(sql).map_err(|error| error.to_string())?;
    let rows = match parameter {
        Some(session_id) => statement.query_map(params![session_id], map_inferred_api),
        None => statement.query_map([], map_inferred_api),
    }
    .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn map_inferred_api(row: &Row<'_>) -> rusqlite::Result<InferredApi> {
    Ok(InferredApi {
        id: row.get(0)?,
        session_id: row.get(1)?,
        name: row.get(2)?,
        method: row.get(3)?,
        path: row.get(4)?,
        params: row.get(5)?,
        auth_required: row.get::<_, i32>(6)? != 0,
        request_ids: row.get(7)?,
        score: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub(crate) fn decode_body(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    match std::str::from_utf8(bytes) {
        Ok(value) => Some(value.to_owned()),
        Err(_) => Some(format!("[binary {} bytes]", bytes.len())),
    }
}

/// Validated location for a generated artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactTarget {
    path: PathBuf,
}

impl ArtifactTarget {
    /// Resolve a session-scoped file without allowing a session id to escape its root.
    pub fn session_file(root: &Path, session_id: &str, extension: &str) -> Result<Self, String> {
        validate_artifact_name("session id", session_id)?;
        validate_extension(extension)?;
        Ok(Self {
            path: root.join(format!("{session_id}.{extension}")),
        })
    }

    /// Resolve a project directory. An explicit path remains user-controlled;
    /// the default is always `<configured root>/<validated project name>`.
    pub fn project_directory(
        root: &Path,
        project_name: &str,
        explicit: Option<&str>,
    ) -> Result<Self, String> {
        validate_artifact_name("project name", project_name)?;
        let path = match explicit {
            Some(path) if path.trim().is_empty() => {
                return Err("output directory must not be empty".to_owned())
            }
            Some(path) => PathBuf::from(path),
            None => root.join(project_name),
        };
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn display(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }

    /// Resolve an artifact-owned relative file without allowing generated
    /// project metadata to escape the validated project directory.
    pub fn child_file(&self, relative: &str) -> Result<PathBuf, String> {
        let relative = Path::new(relative);
        if relative.as_os_str().is_empty()
            || relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err("artifact file path must stay inside its project directory".to_owned());
        }
        Ok(self.path.join(relative))
    }

    pub fn prepare_directory(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.path).map_err(|error| {
            format!(
                "Failed to create artifact directory {}: {error}",
                self.path.display()
            )
        })
    }

    pub fn write_file(&self, contents: impl AsRef<[u8]>) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| format!("Artifact path has no parent: {}", self.path.display()))?;
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Failed to create artifact directory {}: {error}",
                parent.display()
            )
        })?;
        std::fs::write(&self.path, contents)
            .map_err(|error| format!("Failed to write artifact {}: {error}", self.path.display()))
    }
}

pub fn validate_artifact_name(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return Err(format!("{label} contains an invalid path component"));
    }
    Ok(())
}

fn validate_extension(extension: &str) -> Result<(), String> {
    if extension.is_empty()
        || extension.contains('.')
        || extension.contains('/')
        || extension.contains('\\')
    {
        return Err("artifact extension is invalid".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NewCapturedRequest;
    use crate::infer::{store_inference_result_internal, ApiInterface, InferenceResult};
    use tempfile::tempdir;

    fn fixed_session_fixture() -> DbState {
        let db = DbState::new_in_memory(std::sync::Mutex::new(())).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            store_inference_result_internal(
                &conn,
                "fixed-session",
                &InferenceResult {
                    interfaces: vec![ApiInterface {
                        name: "ListUsers".to_owned(),
                        method: "GET".to_owned(),
                        path: "/users".to_owned(),
                        params: "none".to_owned(),
                        auth_required: true,
                    }],
                    modules: Vec::new(),
                    valid: true,
                    errors: Vec::new(),
                    score: 0.9,
                },
            )
            .unwrap();
        }
        db.record_captured_request(NewCapturedRequest {
            timestamp: "2026-08-04 12:00:00",
            method: "get",
            scheme: "https",
            host: "api.example.com",
            path: "/users",
            request_headers: &[],
            request_body: None,
            response_status: Some(200),
            response_headers: &[],
            response_body: Some(r#"{"users":[]}"#),
            duration_ms: Some(12),
            device_id: None,
            app_tag: None,
            response_size: None,
            session_id: Some("fixed-session"),
            client_ip: None,
            upstream_ip: None,
        })
        .unwrap();
        db
    }

    #[test]
    fn fixed_session_snapshot_supplies_every_generation_input_consistently() {
        let snapshot = fixed_session_fixture()
            .inference_session("fixed-session")
            .unwrap();
        assert_eq!(snapshot.session_id, "fixed-session");
        assert_eq!(snapshot.require_inferred_apis().unwrap().len(), 1);
        assert_eq!(snapshot.captured_requests.len(), 1);
        assert_eq!(
            snapshot.inferred_apis[0].path,
            snapshot.captured_requests[0].path
        );

        let traffic = snapshot.traffic_records();
        assert_eq!(traffic.len(), snapshot.captured_requests.len());
        assert_eq!(traffic[0].method, "GET");
        assert_eq!(traffic[0].path, snapshot.inferred_apis[0].path);
        let inferred = snapshot.inferred_semantics().unwrap();
        assert_eq!(inferred.interfaces[0]["name"], "ListUsers");
    }

    #[test]
    fn artifact_targets_share_validation_and_default_path_rules() {
        let root = Path::new("/configured/artifacts");
        let project = ArtifactTarget::project_directory(root, "sample-app", None).unwrap();
        assert_eq!(
            project.path(),
            Path::new("/configured/artifacts/sample-app")
        );
        let file = ArtifactTarget::session_file(root, "session-7", "json").unwrap();
        assert_eq!(
            file.path(),
            Path::new("/configured/artifacts/session-7.json")
        );

        for invalid in ["", ".", "..", "../escape", "nested/name"] {
            assert!(ArtifactTarget::project_directory(root, invalid, None).is_err());
            assert!(ArtifactTarget::session_file(root, invalid, "json").is_err());
        }
    }

    #[test]
    fn artifact_target_prepares_and_writes_outputs() {
        let directory = tempdir().unwrap();
        let file = ArtifactTarget::session_file(directory.path(), "session", "json").unwrap();
        file.write_file(b"{}\n").unwrap();
        assert_eq!(std::fs::read(file.path()).unwrap(), b"{}\n");

        let project = ArtifactTarget::project_directory(directory.path(), "project", None).unwrap();
        project.prepare_directory().unwrap();
        assert!(project.path().is_dir());
        assert_eq!(
            project.child_file("src/App.tsx").unwrap(),
            project.path().join("src/App.tsx")
        );
        assert!(project.child_file("../outside.txt").is_err());
        assert!(project.child_file("/outside.txt").is_err());
    }
}
