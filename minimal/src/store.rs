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
    pub source_type: String,
    pub peer_ip: Option<String>,
    pub device_id: Option<String>,
    pub connection_id: Option<String>,
    pub identity_source: String,
    pub confidence: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestSummary {
    pub id: i64,
    pub ts: i64,
    pub client_ip: Option<String>,
    pub source_type: String,
    pub peer_ip: Option<String>,
    pub device_id: Option<String>,
    pub connection_id: Option<String>,
    pub identity_source: String,
    pub confidence: String,
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
    pub client_ip: Option<String>,
    pub source_type: Option<String>,
    pub unknown: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceSummary {
    pub client_ip: Option<String>,
    pub source_type: String,
    pub identity_source: String,
    pub confidence: String,
    pub device_id: Option<String>,
    pub count: i64,
    pub last_ts: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceGroups {
    pub sources: Vec<SourceSummary>,
    pub unknown_count: i64,
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
                source_type   TEXT NOT NULL DEFAULT 'local_proxy',
                peer_ip       TEXT,
                device_id     TEXT,
                connection_id TEXT,
                identity_source TEXT NOT NULL DEFAULT 'unknown',
                confidence    TEXT NOT NULL DEFAULT 'unknown',
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
            CREATE INDEX IF NOT EXISTS idx_requests_client_ip ON requests(client_ip);
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
        // Additive migration for newer provenance fields.
        let existing: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(requests)")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
            rows.filter_map(|row| row.ok()).collect()
        };
        for column in [
            "source_type TEXT NOT NULL DEFAULT 'local_proxy'",
            "peer_ip TEXT",
            "device_id TEXT",
            "connection_id TEXT",
            "identity_source TEXT NOT NULL DEFAULT 'unknown'",
            "confidence TEXT NOT NULL DEFAULT 'unknown'",
        ] {
            let name = column.split_whitespace().next().unwrap_or("");
            if !existing.iter().any(|existing| existing == name) {
                conn.execute(&format!("ALTER TABLE requests ADD COLUMN {column}"), [])?;
            }
        }
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_requests_source ON requests(source_type);
             CREATE INDEX IF NOT EXISTS idx_requests_identity ON requests(source_type, client_ip, identity_source, confidence);",
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
               (ts, client_ip, source_type, peer_ip, device_id, connection_id,
                identity_source, confidence,
                scheme, method, host, path, status, duration_ms, app, app_source,
                req_headers, req_body, req_truncated, resp_headers, resp_body, resp_truncated,
                content_type, error)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)"#,
            params![
                captured.ts_ms,
                captured.client_ip,
                captured.source_type,
                captured.peer_ip,
                captured.device_id,
                captured.connection_id,
                captured.identity_source,
                captured.confidence,
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
            source_type: captured.source_type.clone(),
            peer_ip: captured.peer_ip.clone(),
            device_id: captured.device_id.clone(),
            connection_id: captured.connection_id.clone(),
            identity_source: captured.identity_source.clone(),
            confidence: captured.confidence.clone(),
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

    pub fn sources(&self) -> Result<SourceGroups> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(
            r#"SELECT client_ip, source_type, identity_source, confidence,
                      COUNT(*), MAX(ts), NULL
               FROM requests
               WHERE client_ip IS NOT NULL
               GROUP BY client_ip, source_type, identity_source, confidence
               ORDER BY MAX(ts) DESC"#,
        )?;
        let sources = stmt
            .query_map([], |row| {
                Ok(SourceSummary {
                    client_ip: row.get(0)?,
                    source_type: row.get(1)?,
                    identity_source: row.get(2)?,
                    confidence: row.get(3)?,
                    device_id: row.get(6)?,
                    count: row.get(4)?,
                    last_ts: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let unknown_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM requests WHERE source_type = 'gateway' AND client_ip IS NULL AND identity_source = 'unknown' AND confidence = 'unknown'",
            [],
            |row| row.get(0),
        )?;
        Ok(SourceGroups { sources, unknown_count })
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
                r#"SELECT id, ts, client_ip, source_type, peer_ip, device_id, connection_id,
                          identity_source, confidence,
                          scheme, method, host, path, status, duration_ms,
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
                        source_type: row.get(3)?,
                        peer_ip: row.get(4)?,
                        device_id: row.get(5)?,
                        connection_id: row.get(6)?,
                        identity_source: row.get(7)?,
                        confidence: row.get(8)?,
                        scheme: row.get(9)?,
                        method: row.get(10)?,
                        host: row.get(11)?,
                        path: row.get(12)?,
                        status: row.get::<_, Option<i64>>(13)?.map(|v| v as u16),
                        duration_ms: row.get::<_, Option<i64>>(14)?.map(|v| v as u64),
                        app: row.get(15)?,
                        app_source: row.get(16)?,
                        content_type: row.get(17)?,
                        resp_size: row.get::<_, Option<i64>>(18)?.map(|v| v as usize),
                    };
                    let req_headers: String = row.get(19)?;
                    let req_body: Option<Vec<u8>> = row.get(20)?;
                    let req_truncated: i64 = row.get(21)?;
                    let resp_headers: String = row.get(22)?;
                    let resp_body: Option<Vec<u8>> = row.get(23)?;
                    let resp_truncated: i64 = row.get(24)?;
                    let error: Option<String> = row.get(25)?;
                    let req_headers: Vec<(String, String)> =
                        serde_json::from_str(&req_headers).unwrap_or_default();
                    let resp_headers: Vec<(String, String)> =
                        serde_json::from_str(&resp_headers).unwrap_or_default();
                    let mut req_body = req_body.map(|bytes| body_view(&bytes, req_truncated != 0));
                    let mut resp_body =
                        resp_body.map(|bytes| body_view(&bytes, resp_truncated != 0));
                    if let Some(body) = req_body.as_mut() {
                        body.note = body_note(true, &summary.path, &req_headers, &resp_headers, body);
                    }
                    if let Some(body) = resp_body.as_mut() {
                        body.note = body_note(false, &summary.path, &req_headers, &resp_headers, body);
                    }
                    Ok(RequestDetail {
                        summary,
                        req_headers,
                        req_body,
                        resp_headers,
                        resp_body,
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
        if let Some(client_ip) = filter
            .client_ip
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            where_clauses.push("client_ip = ?".into());
            params.push(Value::Text(client_ip.to_string()));
        }
        if let Some(source_type) = filter
            .source_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            where_clauses.push("source_type = ?".into());
            params.push(Value::Text(source_type.to_string()));
        }
        if filter.unknown == Some(true) {
            where_clauses.push(
                "source_type = 'gateway' AND client_ip IS NULL AND identity_source = 'unknown' AND confidence = 'unknown'"
                    .into(),
            );
        }

        let limit = filter.limit.unwrap_or(200).clamp(1, 1000) as i64;
        let offset = filter.offset.unwrap_or(0) as i64;
        params.push(Value::Integer(limit));
        params.push(Value::Integer(offset));

        let columns = if include_id {
            "id, ts, client_ip, source_type, peer_ip, device_id, connection_id, identity_source, confidence, scheme, method, host, path, status, duration_ms, app, app_source, content_type, length(resp_body)"
        } else {
            "*"
        };
        let sql = format!(
            "SELECT {columns} FROM requests WHERE {} ORDER BY ts DESC, id DESC LIMIT ? OFFSET ?",
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
        source_type: row.get(3)?,
        peer_ip: row.get(4)?,
        device_id: row.get(5)?,
        connection_id: row.get(6)?,
        identity_source: row.get(7)?,
        confidence: row.get(8)?,
        scheme: row.get(9)?,
        method: row.get(10)?,
        host: row.get(11)?,
        path: row.get(12)?,
        status: row.get::<_, Option<i64>>(13)?.map(|v| v as u16),
        duration_ms: row.get::<_, Option<i64>>(14)?.map(|v| v as u64),
        app: row.get(15)?,
        app_source: row.get(16)?,
        content_type: row.get(17)?,
        resp_size: row.get::<_, Option<i64>>(18)?.map(|v| v as usize),
    })
}

fn body_view(bytes: &[u8], truncated: bool) -> BodyView {
    match std::str::from_utf8(bytes) {
        Ok(text) if is_mostly_text(text) => BodyView {
            encoding: "utf8".into(),
            text: text.to_string(),
            size: bytes.len(),
            truncated,
            note: None,
            preview_hex: None,
        },
        _ => BodyView {
            encoding: "base64".into(),
            text: base64::engine::general_purpose::STANDARD.encode(bytes),
            size: bytes.len(),
            truncated,
            note: None,
            preview_hex: Some(hex_preview(bytes, 128)),
        },
    }
}

fn hex_preview(bytes: &[u8], cap: usize) -> String {
    let mut out = String::new();
    for (index, byte) in bytes.iter().take(cap).enumerate() {
        if index % 16 == 0 {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("{index:08x}  "));
        }
        out.push_str(&format!("{byte:02x} "));
    }
    if bytes.len() > cap {
        out.push_str("…");
    }
    out
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn body_note(
    is_request: bool,
    path: &str,
    req_headers: &[(String, String)],
    resp_headers: &[(String, String)],
    body: &BodyView,
) -> Option<String> {
    let upgrade = header_value(req_headers, "upgrade").unwrap_or_default().to_ascii_lowercase();
    if path.to_ascii_lowercase().contains("/mmtls") || upgrade.contains("mmtls") {
        return Some(
            "WeChat MMTLS: application-layer encrypted frame. TLS interception cannot decrypt it \
             (no HTTP body). Use this only as binary/protocol evidence."
                .into(),
        );
    }
    if header_value(req_headers, "x-tt-").is_some()
        || req_headers.iter().any(|(key, _)| key.to_ascii_lowercase().starts_with("x-tt-"))
    {
        return Some("ByteDance/TTNet binary frame (application-layer protocol; may be encrypted).".into());
    }
    if let Some(decoded_from) = header_value(
        if is_request { req_headers } else { resp_headers },
        "x-proxybot-decoded-from",
    ) {
        return Some(format!(
            "Body decoded from {} before storage; content above is plaintext.",
            decoded_from
        ));
    }
    let encoding = header_value(if is_request { req_headers } else { resp_headers }, "content-encoding")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !encoding.is_empty() && encoding != "identity" {
        return Some(format!(
            "Body is still {encoding}-encoded (decode failed or unsupported).",
        ));
    }
    if body.encoding == "base64" {
        return Some("Binary body (application/octet-stream or unknown format).".into());
    }
    None
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

    pub fn update_response_body(
        &self,
        id: i64,
        bytes: &[u8],
        truncated: bool,
        headers: &[(String, String)],
    ) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let headers = serde_json::to_string(headers).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "UPDATE requests SET resp_body = ?1, resp_truncated = ?2, resp_headers = ?3 WHERE id = ?4",
            params![bytes, i64::from(truncated), headers, id],
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

#[cfg(test)]
mod tests {
    use super::*;

    fn captured(client: Option<&str>, source: &str, identity: &str, confidence: &str, host: &str) -> Captured {
        Captured {
            ts_ms: 1000,
            client_ip: client.map(str::to_string),
            source_type: source.into(),
            peer_ip: Some("127.0.0.1".into()),
            device_id: None,
            connection_id: Some("conn".into()),
            identity_source: identity.into(),
            confidence: confidence.into(),
            scheme: "https".into(),
            method: "GET".into(),
            host: host.into(),
            path: "/".into(),
            status: Some(200),
            duration_ms: Some(1),
            app: None,
            app_source: None,
            req_headers: vec![],
            req_body: None,
            req_truncated: false,
            resp_headers: vec![],
            resp_body: None,
            resp_truncated: false,
            content_type: None,
            error: None,
        }
    }

    fn temp_store(name: &str) -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("proxybot-store-{name}-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).unwrap(), path)
    }

    #[test]
    fn filters_by_source_and_unknown_identity() {
        let (store, path) = temp_store("filters");
        store.insert(&captured(Some("192.168.3.190"), "gateway", "tun_source_ip", "inferred", "a.test")).unwrap();
        store.insert(&captured(Some("192.168.3.191"), "gateway", "tun_source_ip", "inferred", "b.test")).unwrap();
        store.insert(&captured(None, "gateway", "unknown", "unknown", "c.test")).unwrap();
        store.insert(&captured(Some("192.168.3.188"), "local_proxy", "unknown", "unknown", "d.test")).unwrap();

        let by_ip = store
            .list(&ListFilter { client_ip: Some("192.168.3.190".into()), ..Default::default() })
            .unwrap();
        assert_eq!(by_ip.len(), 1);
        assert_eq!(by_ip[0].host, "a.test");

        let unknown = store
            .list(&ListFilter { unknown: Some(true), ..Default::default() })
            .unwrap();
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].host, "c.test");

        let groups = store.sources().unwrap();
        assert_eq!(groups.unknown_count, 1);
        assert!(groups.sources.iter().any(|s| s.client_ip.as_deref() == Some("192.168.3.190")));
        assert!(groups.sources.iter().any(|s| s.client_ip.as_deref() == Some("192.168.3.191")));
        assert!(groups.sources.iter().any(|s| s.client_ip.as_deref() == Some("192.168.3.188")));

        drop(store);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }
}
