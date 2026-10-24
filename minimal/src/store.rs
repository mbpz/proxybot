//! SQLite persistence: one `requests` table, plus helpers for the web API.

use anyhow::{Context, Result};
use base64::Engine as _;
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub const MAX_ROWS: i64 = 5_000;
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Captured {
    pub ts_ms: i64,
    pub client_ip: Option<String>,
    pub scheme: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub app: Option<String>,
    pub app_source: Option<String>,
    pub req_headers: Vec<(String, String)>,
    pub req_body: Option<Vec<u8>>,
    pub req_truncated: bool,
    pub resp_headers: Vec<(String, String)>,
    pub resp_body: Option<Vec<u8>>,
    pub resp_truncated: bool,
    pub content_type: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BodyView {
    pub encoding: String,
    pub text: String,
    pub size: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestSummary {
    pub id: i64,
    pub ts: i64,
    pub client_ip: Option<String>,
    pub scheme: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub app: Option<String>,
    pub app_source: Option<String>,
    pub content_type: Option<String>,
    pub resp_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestDetail {
    #[serde(flatten)]
    pub summary: RequestSummary,
    pub req_headers: Vec<(String, String)>,
    pub req_body: Option<BodyView>,
    pub resp_headers: Vec<(String, String)>,
    pub resp_body: Option<BodyView>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarExport {
    pub log: HarLog,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarLog {
    pub version: String,
    pub creator: HarCreator,
    pub entries: Vec<HarEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarEntry {
    #[serde(rename = "startedDateTime")]
    pub started_date_time: String,
    pub time: u64,
    pub request: HarRequest,
    pub response: HarResponse,

    #[serde(rename = "cache")]
    pub cache: serde_json::Value,

    #[serde(rename = "timings")]
    pub timings: HarTimings,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarRequest {
    pub method: String,
    pub url: String,

    #[serde(rename = "httpVersion")]
    pub http_version: String,

    pub headers: Vec<HarHeader>,

    #[serde(rename = "queryString")]
    pub query_string: Vec<HarHeader>,

    pub cookies: Vec<serde_json::Value>,

    #[serde(rename = "headersSize")]
    pub headers_size: i64,

    #[serde(rename = "bodySize")]
    pub body_size: i64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_data: Option<HarPostData>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarResponse {
    pub status: i64,

    #[serde(rename = "statusText")]
    pub status_text: String,

    #[serde(rename = "httpVersion")]
    pub http_version: String,

    pub headers: Vec<HarHeader>,
    pub cookies: Vec<serde_json::Value>,
    pub content: HarContent,

    #[serde(rename = "redirectURL")]
    pub redirect_url: String,

    #[serde(rename = "headersSize")]
    pub headers_size: i64,

    #[serde(rename = "bodySize")]
    pub body_size: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarContent {
    pub size: i64,

    #[serde(rename = "mimeType")]
    pub mime_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarPostData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarTimings {
    pub send: i64,
    pub wait: u64,
    pub receive: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListFilter {
    pub q: Option<String>,
    pub app: Option<String>,
    pub method: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path).context("open SQLite history database")?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            CREATE TABLE IF NOT EXISTS requests (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                ts            INTEGER NOT NULL,
                client_ip     TEXT,
                scheme        TEXT NOT NULL,
                method        TEXT NOT NULL,
                host          TEXT NOT NULL,
                path          TEXT NOT NULL,
                status        INTEGER,
                duration_ms   INTEGER,
                app           TEXT,
                app_source    TEXT,
                req_headers   TEXT NOT NULL,
                req_body      BLOB,
                req_truncated INTEGER NOT NULL DEFAULT 0,
                resp_headers  TEXT NOT NULL,
                resp_body     BLOB,
                resp_truncated INTEGER NOT NULL DEFAULT 0,
                content_type  TEXT,
                error         TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_requests_ts ON requests(ts DESC);
            CREATE INDEX IF NOT EXISTS idx_requests_host ON requests(host);
            CREATE INDEX IF NOT EXISTS idx_requests_app ON requests(app);
            CREATE TABLE IF NOT EXISTS ws_frames (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                ts          INTEGER NOT NULL,
                host        TEXT NOT NULL,
                client_ip   TEXT,
                direction   TEXT NOT NULL,
                opcode      INTEGER NOT NULL,
                size        INTEGER NOT NULL,
                encoding    TEXT NOT NULL,
                payload     TEXT,
                truncated   INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_ws_host_ts ON ws_frames(host, ts DESC);
            CREATE TABLE IF NOT EXISTS tls_attempts (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                ts          INTEGER NOT NULL,
                host        TEXT NOT NULL,
                client_ip   TEXT,
                passthrough INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_tls_host_ts ON tls_attempts(host, ts DESC);
            "#,
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert(&self, captured: &Captured) -> Result<RequestSummary> {
        let req_headers = serde_json::to_string(&captured.req_headers).unwrap_or_else(|_| "[]".into());
        let resp_headers =
            serde_json::to_string(&captured.resp_headers).unwrap_or_else(|_| "[]".into());
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute(
            r#"INSERT INTO requests
               (ts, client_ip, scheme, method, host, path, status, duration_ms, app, app_source,
                req_headers, req_body, req_truncated, resp_headers, resp_body, resp_truncated,
                content_type, error)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)"#,
            params![
                captured.ts_ms,
                captured.client_ip,
                captured.scheme,
                captured.method,
                captured.host,
                captured.path,
                captured.status.map(i64::from),
                captured.duration_ms.map(|value| value as i64),
                captured.app,
                captured.app_source,
                req_headers,
                captured.req_body.as_deref(),
                i64::from(captured.req_truncated),
                resp_headers,
                captured.resp_body.as_deref(),
                i64::from(captured.resp_truncated),
                captured.content_type,
                captured.error,
            ],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "DELETE FROM requests WHERE id NOT IN (SELECT id FROM requests ORDER BY id DESC LIMIT ?1)",
            params![MAX_ROWS],
        )?;
        Ok(RequestSummary {
            id,
            ts: captured.ts_ms,
            client_ip: captured.client_ip.clone(),
            scheme: captured.scheme.clone(),
            method: captured.method.clone(),
            host: captured.host.clone(),
            path: captured.path.clone(),
            status: captured.status,
            duration_ms: captured.duration_ms,
            app: captured.app.clone(),
            app_source: captured.app_source.clone(),
            content_type: captured.content_type.clone(),
            resp_size: captured.resp_body.as_ref().map(Vec::len),
        })
    }

    pub fn list(&self, filter: &ListFilter) -> Result<Vec<RequestSummary>> {
        let (sql, params) = self.list_query(filter, true);
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params.iter()), map_summary)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn detail(&self, id: i64) -> Result<Option<RequestDetail>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let row = conn
            .query_row(
                r#"SELECT id, ts, client_ip, scheme, method, host, path, status, duration_ms,
                          app, app_source, content_type, length(resp_body),
                          req_headers, req_body, req_truncated, resp_headers, resp_body,
                          resp_truncated, error
                   FROM requests WHERE id = ?1"#,
                params![id],
                |row| {
                    let summary = RequestSummary {
                        id: row.get(0)?,
                        ts: row.get(1)?,
                        client_ip: row.get(2)?,
                        scheme: row.get(3)?,
                        method: row.get(4)?,
                        host: row.get(5)?,
                        path: row.get(6)?,
                        status: row.get::<_, Option<i64>>(7)?.map(|v| v as u16),
                        duration_ms: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                        app: row.get(9)?,
                        app_source: row.get(10)?,
                        content_type: row.get(11)?,
                        resp_size: row.get::<_, Option<i64>>(12)?.map(|v| v as usize),
                    };
                    let req_headers: String = row.get(13)?;
                    let req_body: Option<Vec<u8>> = row.get(14)?;
                    let req_truncated: i64 = row.get(15)?;
                    let resp_headers: String = row.get(16)?;
                    let resp_body: Option<Vec<u8>> = row.get(17)?;
                    let resp_truncated: i64 = row.get(18)?;
                    let error: Option<String> = row.get(19)?;
                    Ok(RequestDetail {
                        summary,
                        req_headers: serde_json::from_str(&req_headers).unwrap_or_default(),
                        req_body: req_body.map(|bytes| body_view(&bytes, req_truncated != 0)),
                        resp_headers: serde_json::from_str(&resp_headers).unwrap_or_default(),
                        resp_body: resp_body.map(|bytes| body_view(&bytes, resp_truncated != 0)),
                        error,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    pub fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute("DELETE FROM requests", [])?;
        Ok(())
    }

    pub fn har(&self, filter: &ListFilter) -> Result<HarExport> {
        let (sql, params) = self.list_query(filter, true);
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(&sql)?;
        let ids = stmt
            .query_map(params_from_iter(params.iter()), |row| row.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);
        drop(conn);

        let mut entries = Vec::new();
        for id in ids {
            if let Some(detail) = self.detail(id)? {
                entries.push(entry_from_detail(&detail));
            }
        }
        Ok(HarExport {
            log: HarLog {
                version: "1.2".into(),
                creator: HarCreator {
                    name: "ProxyBot".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                },
                entries,
            },
        })
    }

    fn list_query(&self, filter: &ListFilter, include_id: bool) -> (String, Vec<Value>) {
        let mut where_clauses = vec!["1 = 1".to_string()];
        let mut params: Vec<Value> = Vec::new();

        if let Some(q) = filter.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
            where_clauses.push("(host LIKE ? OR path LIKE ? OR app LIKE ?)".into());
            let pattern = format!("%{q}%");
            params.push(Value::Text(pattern.clone()));
            params.push(Value::Text(pattern.clone()));
            params.push(Value::Text(pattern));
        }
        if let Some(app) = filter.app.as_deref().filter(|a| !a.is_empty() && *a != "all") {
            where_clauses.push("app = ?".into());
            params.push(Value::Text(app.to_string()));
        }
        if let Some(method) = filter
            .method
            .as_deref()
            .filter(|m| !m.is_empty() && *m != "all")
        {
            where_clauses.push("method = ?".into());
            params.push(Value::Text(method.to_ascii_uppercase()));
        }
        if let Some(status) = filter
            .status
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "all")
        {
            if let Ok(class) = status.parse::<i64>() {
                where_clauses.push("status IS NOT NULL AND status / 100 = ?".into());
                params.push(Value::Integer(class));
            }
        }

        let limit = filter.limit.unwrap_or(200).clamp(1, 1000) as i64;
        params.push(Value::Integer(limit));

        let columns = if include_id {
            "id, ts, client_ip, scheme, method, host, path, status, duration_ms, app, app_source, content_type, length(resp_body)"
        } else {
            "*"
        };
        let sql = format!(
            "SELECT {columns} FROM requests WHERE {} ORDER BY ts DESC, id DESC LIMIT ?",
            where_clauses.join(" AND ")
        );
        (sql, params)
    }
}

fn map_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<RequestSummary> {
    Ok(RequestSummary {
        id: row.get(0)?,
        ts: row.get(1)?,
        client_ip: row.get(2)?,
        scheme: row.get(3)?,
        method: row.get(4)?,
        host: row.get(5)?,
        path: row.get(6)?,
        status: row.get::<_, Option<i64>>(7)?.map(|v| v as u16),
        duration_ms: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
        app: row.get(9)?,
        app_source: row.get(10)?,
        content_type: row.get(11)?,
        resp_size: row.get::<_, Option<i64>>(12)?.map(|v| v as usize),
    })
}

fn body_view(bytes: &[u8], truncated: bool) -> BodyView {
    match std::str::from_utf8(bytes) {
        Ok(text) if is_mostly_text(text) => BodyView {
            encoding: "utf8".into(),
            text: text.to_string(),
            size: bytes.len(),
            truncated,
        },
        _ => BodyView {
            encoding: "base64".into(),
            text: base64::engine::general_purpose::STANDARD.encode(bytes),
            size: bytes.len(),
            truncated,
        },
    }
}

fn is_mostly_text(text: &str) -> bool {
    let control = text
        .chars()
        .filter(|c| c.is_control() && *c != '\n' && *c != '\r' && *c != '\t')
        .count();
    control * 20 <= text.chars().count().max(1)
}

fn entry_from_detail(detail: &RequestDetail) -> HarEntry {
    let summary = &detail.summary;
    let url = format!(
        "{}://{}{}",
        summary.scheme,
        summary.host,
        if summary.path.is_empty() {
            "/"
        } else {
            summary.path.as_str()
        }
    );
    let started = DateTime::<Utc>::from_timestamp_millis(summary.ts)
        .unwrap_or_else(Utc::now);
    let mime_type = summary
        .content_type
        .clone()
        .unwrap_or_else(|| "application/octet-stream".into());

    HarEntry {
        started_date_time: started.to_rfc3339_opts(SecondsFormat::Millis, true),
        time: summary.duration_ms.unwrap_or(0),
        request: HarRequest {
            method: summary.method.clone(),
            url,
            http_version: "HTTP/1.1".into(),
            headers: detail
                .req_headers
                .iter()
                .map(|(name, value)| HarHeader {
                    name: name.clone(),
                    value: value.clone(),
                })
                .collect(),
            query_string: Vec::new(),
            cookies: Vec::new(),
            headers_size: -1,
            body_size: detail.req_body.as_ref().map(|b| b.size as i64).unwrap_or(0),
            post_data: detail.req_body.as_ref().map(|body| HarPostData {
                mime_type: mime_type.clone(),
                text: body.text.clone(),
            }),
        },
        response: HarResponse {
            status: summary.status.unwrap_or(0) as i64,
            status_text: String::new(),
            http_version: "HTTP/1.1".into(),
            headers: detail
                .resp_headers
                .iter()
                .map(|(name, value)| HarHeader {
                    name: name.clone(),
                    value: value.clone(),
                })
                .collect(),
            cookies: Vec::new(),
            content: HarContent {
                size: detail
                    .resp_body
                    .as_ref()
                    .map(|body| body.size as i64)
                    .unwrap_or(0),
                mime_type,
                text: detail.resp_body.as_ref().map(|body| body.text.clone()),
                encoding: detail
                    .resp_body
                    .as_ref()
                    .filter(|body| body.encoding == "base64")
                    .map(|_| "base64".to_string()),
            },
            redirect_url: String::new(),
            headers_size: -1,
            body_size: detail
                .resp_body
                .as_ref()
                .map(|body| body.size as i64)
                .unwrap_or(0),
        },
        cache: serde_json::json!({}),
        timings: HarTimings {
            send: 0,
            wait: summary.duration_ms.unwrap_or(0),
            receive: 0,
        },
    }
}


#[derive(Debug, Clone)]
pub struct WsFrameInput {
    pub ts_ms: i64,
    pub host: String,
    pub client_ip: Option<String>,
    pub direction: String,
    pub opcode: u8,
    pub size: usize,
    pub encoding: String,
    pub payload: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WsFrameRecord {
    pub id: i64,
    pub ts: i64,
    pub host: String,
    pub client_ip: Option<String>,
    pub direction: String,
    pub opcode: u8,
    pub size: usize,
    pub encoding: String,
    pub payload: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WsHost {
    pub host: String,
    pub frames: i64,
    pub last_ts: i64,
}

impl Store {
    pub fn insert_ws_frame(&self, frame: &WsFrameInput) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT INTO ws_frames (ts, host, client_ip, direction, opcode, size, encoding, payload, truncated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                frame.ts_ms,
                frame.host,
                frame.client_ip,
                frame.direction,
                i64::from(frame.opcode),
                frame.size as i64,
                frame.encoding,
                frame.payload,
                i64::from(frame.truncated),
            ],
        )?;
        conn.execute(
            "DELETE FROM ws_frames WHERE id NOT IN (SELECT id FROM ws_frames ORDER BY id DESC LIMIT 20000)",
            [],
        )?;
        Ok(())
    }

    pub fn ws_hosts(&self) -> Result<Vec<WsHost>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT host, COUNT(*) AS frames, MAX(ts) AS last_ts FROM ws_frames GROUP BY host ORDER BY last_ts DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(WsHost {
                host: row.get(0)?,
                frames: row.get(1)?,
                last_ts: row.get(2)?,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn ws_frames(&self, host: &str, limit: usize) -> Result<Vec<WsFrameRecord>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, ts, host, client_ip, direction, opcode, size, encoding, payload, truncated
             FROM ws_frames WHERE host = ?1 ORDER BY ts DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![host, limit as i64], |row| {
            Ok(WsFrameRecord {
                id: row.get(0)?,
                ts: row.get(1)?,
                host: row.get(2)?,
                client_ip: row.get(3)?,
                direction: row.get(4)?,
                opcode: row.get::<_, i64>(5)? as u8,
                size: row.get::<_, i64>(6)? as usize,
                encoding: row.get(7)?,
                payload: row.get(8)?,
                truncated: row.get::<_, i64>(9)? != 0,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }

    pub fn update_response_body(&self, id: i64, bytes: &[u8], truncated: bool) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute(
            "UPDATE requests SET resp_body = ?1, resp_truncated = ?2 WHERE id = ?3",
            params![bytes, i64::from(truncated), id],
        )?;
        Ok(())
    }
}


#[derive(Debug, Clone, serde::Serialize)]
pub struct TlsHost {
    pub host: String,
    pub attempts: i64,
    pub last_ts: i64,
    pub passthrough: bool,
}

impl Store {
    pub fn insert_tls_attempt(&self, ts: i64, host: &str, client_ip: Option<&str>, passthrough: bool) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT INTO tls_attempts (ts, host, client_ip, passthrough) VALUES (?1, ?2, ?3, ?4)",
            params![ts, host, client_ip, i64::from(passthrough)],
        )?;
        conn.execute(
            "DELETE FROM tls_attempts WHERE id NOT IN (SELECT id FROM tls_attempts ORDER BY id DESC LIMIT 20000)",
            [],
        )?;
        Ok(())
    }

    pub fn tls_hosts(&self) -> Result<Vec<TlsHost>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT host, COUNT(*) AS attempts, MAX(ts) AS last_ts,
                    MAX(passthrough) AS passthrough
             FROM tls_attempts GROUP BY host ORDER BY last_ts DESC LIMIT 500",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TlsHost {
                host: row.get(0)?,
                attempts: row.get(1)?,
                last_ts: row.get(2)?,
                passthrough: row.get::<_, i64>(3)? != 0,
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
    }
}
