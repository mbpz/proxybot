//! Captured Request persistence.
//!
//! This Module owns the SQLite representation of captured HTTP exchanges and
//! WebSocket frames. Callers work with domain-shaped records and queries; table
//! names, column order, JSON encoding, BLOB handling, locking, and bind values
//! remain inside the Implementation.

use super::DbState;
use chrono::{DateTime, NaiveDateTime, Utc};
use proxybot_core::{InterceptedRequest, WsFrame};
use rusqlite::{params_from_iter, types::Value, Connection, Row};

const REQUEST_COLUMNS: &str = "id, timestamp, method, scheme, host, path, \
    req_headers, req_body, resp_status, resp_headers, resp_body, duration_ms, \
    device_id, app_tag, response_size, is_websocket, session_id, client_ip, upstream_ip";

/// A Captured Request as stored by the desktop application.
#[derive(Clone, Debug, PartialEq)]
pub struct CapturedRequestRecord {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub request_headers: Vec<(String, String)>,
    pub request_body: Option<Vec<u8>>,
    pub response_status: Option<u16>,
    pub response_headers: Vec<(String, String)>,
    pub response_body: Option<Vec<u8>>,
    pub duration_ms: Option<i64>,
    pub device_id: Option<i64>,
    pub app_tag: Option<String>,
    pub response_size: Option<i64>,
    pub is_websocket: bool,
    pub session_id: Option<String>,
    pub client_ip: Option<String>,
    pub upstream_ip: Option<String>,
}

impl CapturedRequestRecord {
    /// Convert the persisted record to the shared desktop wire type.
    pub fn as_intercepted(&self) -> InterceptedRequest {
        InterceptedRequest {
            id: self.id.to_string(),
            timestamp: self.timestamp.clone(),
            method: self.method.clone(),
            host: self.host.clone(),
            path: self.path.clone(),
            query_params: self.path.split_once('?').map(|(_, query)| query.to_owned()),
            status: self.response_status,
            latency_ms: self.duration_ms.and_then(|value| u64::try_from(value).ok()),
            scheme: self.scheme.clone(),
            req_headers: self.request_headers.clone(),
            req_body: self
                .request_body
                .as_deref()
                .map(|body| String::from_utf8_lossy(body).into_owned()),
            resp_headers: self.response_headers.clone(),
            resp_body: self
                .response_body
                .as_deref()
                .map(|body| String::from_utf8_lossy(body).into_owned()),
            resp_size: self
                .response_size
                .and_then(|value| usize::try_from(value).ok())
                .or_else(|| self.response_body.as_ref().map(Vec::len)),
            app_name: self.app_tag.clone(),
            app_icon: None,
            device_id: self.device_id,
            device_name: None,
            client_ip: self.client_ip.clone(),
            upstream_ip: self.upstream_ip.clone(),
            is_websocket: self.is_websocket,
            ws_frames: None,
            grpc_decoded: None,
            graphql_op: None,
        }
    }

    /// Parse the persisted timestamp variants emitted by current and legacy
    /// Capture Adapters into one UTC representation.
    pub fn captured_at(&self) -> Option<DateTime<Utc>> {
        parse_captured_timestamp(&self.timestamp)
    }
}

/// Convert all timestamp encodings accepted by Captured Request persistence.
pub fn parse_captured_timestamp(timestamp: &str) -> Option<DateTime<Utc>> {
    if let Ok(datetime) = DateTime::parse_from_rfc3339(timestamp) {
        return Some(datetime.with_timezone(&Utc));
    }
    if let Ok(seconds) = timestamp.parse::<f64>() {
        if seconds.is_finite() {
            let millis = seconds * 1_000.0;
            if millis >= i64::MIN as f64 && millis <= i64::MAX as f64 {
                return DateTime::from_timestamp_millis(millis.round() as i64);
            }
        }
    }
    NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|datetime| datetime.and_utc())
}

/// Input used to persist one Captured Request.
#[derive(Clone, Copy, Debug)]
pub struct NewCapturedRequest<'a> {
    pub timestamp: &'a str,
    pub method: &'a str,
    pub scheme: &'a str,
    pub host: &'a str,
    pub path: &'a str,
    pub request_headers: &'a [(String, String)],
    pub request_body: Option<&'a str>,
    pub response_status: Option<u16>,
    pub response_headers: &'a [(String, String)],
    pub response_body: Option<&'a str>,
    pub response_size: Option<usize>,
    pub duration_ms: Option<u64>,
    pub device_id: Option<i64>,
    pub app_tag: Option<&'a str>,
    pub session_id: Option<&'a str>,
    pub client_ip: Option<&'a str>,
    pub upstream_ip: Option<&'a str>,
}

impl<'a> NewCapturedRequest<'a> {
    pub fn from_intercepted(request: &'a InterceptedRequest) -> Self {
        Self {
            timestamp: &request.timestamp,
            method: &request.method,
            scheme: &request.scheme,
            host: &request.host,
            path: &request.path,
            request_headers: &request.req_headers,
            request_body: request.req_body.as_deref(),
            response_status: request.status,
            response_headers: &request.resp_headers,
            response_body: request.resp_body.as_deref(),
            response_size: request.resp_size,
            duration_ms: request.latency_ms,
            device_id: request.device_id,
            app_tag: request.app_name.as_deref(),
            session_id: None,
            client_ip: request.client_ip.as_deref(),
            upstream_ip: request.upstream_ip.as_deref(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SessionScope {
    #[default]
    Any,
    Unassigned,
    Exact(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CapturedRequestOrder {
    IdAscending,
    #[default]
    IdDescending,
    TimestampAscending,
    TimestampDescending,
}

/// Domain query for Captured Requests.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapturedRequestQuery {
    pub session: SessionScope,
    pub device_id: Option<i64>,
    pub host: Option<String>,
    pub app_tag: Option<String>,
    pub since: Option<String>,
    pub request_body_required: bool,
    pub order: CapturedRequestOrder,
    pub limit: Option<usize>,
    pub offset: usize,
}

/// Input used to persist a WebSocket frame associated with a Captured Request.
#[derive(Clone, Copy, Debug)]
pub struct NewWebSocketFrame<'a> {
    pub request_id: &'a str,
    pub direction: &'a str,
    pub opcode: u8,
    pub payload: &'a str,
    pub payload_binary: Option<&'a [u8]>,
    pub size: usize,
    pub timestamp: &'a str,
}

impl DbState {
    pub fn record_captured_request(&self, request: NewCapturedRequest<'_>) -> Result<i64, String> {
        let connection = self.conn.lock().map_err(|error| error.to_string())?;
        insert_request(&connection, request)
    }

    pub fn captured_request(&self, id: i64) -> Result<Option<CapturedRequestRecord>, String> {
        let connection = self.conn.lock().map_err(|error| error.to_string())?;
        find_request(&connection, id)
    }

    pub fn captured_requests(
        &self,
        query: &CapturedRequestQuery,
    ) -> Result<Vec<CapturedRequestRecord>, String> {
        let connection = self.conn.lock().map_err(|error| error.to_string())?;
        query_requests(&connection, query)
    }

    pub fn count_captured_requests(&self, query: &CapturedRequestQuery) -> Result<i64, String> {
        let connection = self.conn.lock().map_err(|error| error.to_string())?;
        count_requests(&connection, query)
    }

    pub fn clear_captured_requests(&self) -> Result<usize, String> {
        let connection = self.conn.lock().map_err(|error| error.to_string())?;
        connection
            .execute("DELETE FROM http_requests", [])
            .map_err(|error| error.to_string())
    }

    pub fn mark_captured_request_websocket(&self, request_id: &str) -> Result<(), String> {
        let connection = self.conn.lock().map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE http_requests SET is_websocket = 1 WHERE id = ?1",
                [request_id],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn record_websocket_frame(&self, frame: NewWebSocketFrame<'_>) -> Result<i64, String> {
        let connection = self.conn.lock().map_err(|error| error.to_string())?;
        insert_websocket_frame(&connection, frame)
    }

    pub fn websocket_frames(&self, request_id: &str) -> Result<Vec<WsFrame>, String> {
        let connection = self.conn.lock().map_err(|error| error.to_string())?;
        query_websocket_frames(&connection, request_id)
    }
}

fn insert_request(connection: &Connection, request: NewCapturedRequest<'_>) -> Result<i64, String> {
    let request_headers =
        serde_json::to_string(request.request_headers).map_err(|error| error.to_string())?;
    let response_headers =
        serde_json::to_string(request.response_headers).map_err(|error| error.to_string())?;

    connection
        .execute(
            r#"INSERT INTO http_requests
               (timestamp, method, scheme, host, path, req_headers, req_body,
                resp_status, resp_headers, resp_body, duration_ms, device_id, app_tag,
                session_id, response_size, is_websocket, client_ip, upstream_ip)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18)"#,
            rusqlite::params![
                request.timestamp,
                request.method,
                request.scheme,
                request.host,
                request.path,
                request_headers,
                request.request_body.map(str::as_bytes),
                request.response_status,
                response_headers,
                request.response_body.map(str::as_bytes),
                request.duration_ms,
                request.device_id,
                request.app_tag,
                request.session_id,
                request.response_size,
                0_i64,
                request.client_ip,
                request.upstream_ip,
            ],
        )
        .map_err(|error| error.to_string())?;

    Ok(connection.last_insert_rowid())
}

fn find_request(connection: &Connection, id: i64) -> Result<Option<CapturedRequestRecord>, String> {
    let sql = format!("SELECT {REQUEST_COLUMNS} FROM http_requests WHERE id = ?1");
    match connection.query_row(&sql, [id], map_request) {
        Ok(record) => Ok(Some(record)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn query_requests(
    connection: &Connection,
    query: &CapturedRequestQuery,
) -> Result<Vec<CapturedRequestRecord>, String> {
    let (conditions, mut values) = query_conditions(query);
    let mut sql = format!("SELECT {REQUEST_COLUMNS} FROM http_requests");
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(match query.order {
        CapturedRequestOrder::IdAscending => " ORDER BY id ASC",
        CapturedRequestOrder::IdDescending => " ORDER BY id DESC",
        CapturedRequestOrder::TimestampAscending => " ORDER BY timestamp ASC, id ASC",
        CapturedRequestOrder::TimestampDescending => " ORDER BY timestamp DESC, id DESC",
    });
    if let Some(limit) = query.limit {
        sql.push_str(" LIMIT ?");
        values.push(Value::Integer(limit as i64));
        if query.offset > 0 {
            sql.push_str(" OFFSET ?");
            values.push(Value::Integer(query.offset as i64));
        }
    } else if query.offset > 0 {
        sql.push_str(" LIMIT -1 OFFSET ?");
        values.push(Value::Integer(query.offset as i64));
    }

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let records = statement
        .query_map(params_from_iter(values.iter()), map_request)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(records)
}

fn count_requests(connection: &Connection, query: &CapturedRequestQuery) -> Result<i64, String> {
    let (conditions, values) = query_conditions(query);
    let mut sql = String::from("SELECT COUNT(*) FROM http_requests");
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    connection
        .query_row(&sql, params_from_iter(values.iter()), |row| row.get(0))
        .map_err(|error| error.to_string())
}

fn query_conditions(query: &CapturedRequestQuery) -> (Vec<String>, Vec<Value>) {
    let mut conditions = Vec::new();
    let mut values = Vec::new();
    match &query.session {
        SessionScope::Any => {}
        SessionScope::Unassigned => {
            conditions.push("(session_id IS NULL OR session_id = '')".to_owned());
        }
        SessionScope::Exact(session_id) => {
            conditions.push("session_id = ?".to_owned());
            values.push(Value::Text(session_id.clone()));
        }
    }
    if let Some(device_id) = query.device_id {
        conditions.push("device_id = ?".to_owned());
        values.push(Value::Integer(device_id));
    }
    if let Some(host) = &query.host {
        conditions.push("host = ?".to_owned());
        values.push(Value::Text(host.clone()));
    }
    if let Some(app_tag) = &query.app_tag {
        conditions.push("app_tag = ?".to_owned());
        values.push(Value::Text(app_tag.clone()));
    }
    if let Some(since) = &query.since {
        conditions.push("timestamp > ?".to_owned());
        values.push(Value::Text(since.clone()));
    }
    if query.request_body_required {
        conditions.push("req_body IS NOT NULL".to_owned());
    }
    (conditions, values)
}

fn map_request(row: &Row<'_>) -> rusqlite::Result<CapturedRequestRecord> {
    let request_headers_json: String = row.get(6)?;
    let response_headers_json: String = row.get(9)?;
    let is_websocket: i64 = row.get(15)?;
    Ok(CapturedRequestRecord {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        method: row.get(2)?,
        scheme: row.get(3)?,
        host: row.get(4)?,
        path: row.get(5)?,
        request_headers: serde_json::from_str(&request_headers_json).unwrap_or_default(),
        request_body: row.get(7)?,
        response_status: row.get(8)?,
        response_headers: serde_json::from_str(&response_headers_json).unwrap_or_default(),
        response_body: row.get(10)?,
        duration_ms: row.get(11)?,
        device_id: row.get(12)?,
        app_tag: row.get(13)?,
        response_size: row.get(14)?,
        is_websocket: is_websocket != 0,
        session_id: row.get(16)?,
        client_ip: row.get(17)?,
        upstream_ip: row.get(18)?,
    })
}

fn insert_websocket_frame(
    connection: &Connection,
    frame: NewWebSocketFrame<'_>,
) -> Result<i64, String> {
    connection
        .execute(
            r#"INSERT INTO ws_frames
               (request_id, direction, opcode, payload, payload_bin, size, timestamp)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            rusqlite::params![
                frame.request_id,
                frame.direction,
                frame.opcode,
                frame.payload,
                frame.payload_binary,
                frame.size as i64,
                frame.timestamp,
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(connection.last_insert_rowid())
}

fn query_websocket_frames(
    connection: &Connection,
    request_id: &str,
) -> Result<Vec<WsFrame>, String> {
    let mut statement = connection
        .prepare(
            "SELECT direction, opcode, payload, size, timestamp
             FROM ws_frames WHERE request_id = ?1 ORDER BY timestamp ASC",
        )
        .map_err(|error| error.to_string())?;
    let frames = statement
        .query_map([request_id], |row| {
            let opcode: i32 = row.get(1)?;
            let size: i64 = row.get(3)?;
            Ok(WsFrame {
                direction: row.get(0)?,
                opcode: opcode as u8,
                payload: row.get(2)?,
                size: size as usize,
                timestamp: row.get(4)?,
                truncated: (size as usize) > crate::ws_frames::MAX_PAYLOAD_SIZE,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> InterceptedRequest {
        InterceptedRequest {
            timestamp: "2026-08-04 10:00:00".to_owned(),
            method: "POST".to_owned(),
            scheme: "https".to_owned(),
            host: "api.example.com".to_owned(),
            path: "/v1/items?limit=2".to_owned(),
            req_headers: vec![("content-type".to_owned(), "application/json".to_owned())],
            req_body: Some("{\"name\":\"demo\"}".to_owned()),
            status: Some(201),
            resp_headers: vec![("content-type".to_owned(), "application/json".to_owned())],
            resp_body: Some("{\"id\":1}".to_owned()),
            resp_size: Some(42),
            latency_ms: Some(12),
            app_name: Some("Example".to_owned()),
            device_id: Some(7),
            client_ip: Some("10.0.0.2".to_owned()),
            upstream_ip: Some("203.0.113.8".to_owned()),
            ..Default::default()
        }
    }

    #[test]
    fn persists_and_queries_a_domain_shaped_request() {
        let state = DbState::new_in_memory(std::sync::Mutex::new(())).unwrap();
        let request = request();
        let mut input = NewCapturedRequest::from_intercepted(&request);
        input.session_id = Some("session-1");
        let id = state.record_captured_request(input).unwrap();

        let record = state.captured_request(id).unwrap().unwrap();
        assert_eq!(record.host, "api.example.com");
        assert_eq!(record.request_headers, request.req_headers);
        assert_eq!(
            record.request_body.as_deref(),
            request.req_body.as_deref().map(str::as_bytes)
        );
        assert_eq!(record.session_id.as_deref(), Some("session-1"));
        assert_eq!(
            record.response_size,
            request.resp_size.map(|size| size as i64)
        );

        let intercepted = record.as_intercepted();
        assert_eq!(intercepted.query_params.as_deref(), Some("limit=2"));
        assert_eq!(intercepted.app_name.as_deref(), Some("Example"));
        assert_eq!(intercepted.resp_size, Some(42));
        assert_eq!(intercepted.client_ip.as_deref(), Some("10.0.0.2"));
        assert_eq!(intercepted.upstream_ip.as_deref(), Some("203.0.113.8"));
    }

    #[test]
    fn parses_all_persisted_timestamp_encodings() {
        let epoch = parse_captured_timestamp("1704067200.500").unwrap();
        let rfc3339 = parse_captured_timestamp("2024-01-01T00:00:00.500Z").unwrap();
        let sqlite = parse_captured_timestamp("2024-01-01 00:00:00").unwrap();

        assert_eq!(epoch, rfc3339);
        assert_eq!(sqlite.timestamp(), 1_704_067_200);
        assert!(parse_captured_timestamp("not-a-timestamp").is_none());
    }

    #[test]
    fn query_filters_are_parameterized_and_count_ignores_pagination() {
        let state = DbState::new_in_memory(std::sync::Mutex::new(())).unwrap();
        for session in ["session-1", "session-2"] {
            let request = request();
            let mut input = NewCapturedRequest::from_intercepted(&request);
            input.session_id = Some(session);
            state.record_captured_request(input).unwrap();
        }

        let query = CapturedRequestQuery {
            session: SessionScope::Exact("session-1' OR 1=1 --".to_owned()),
            limit: Some(1),
            ..Default::default()
        };
        assert!(state.captured_requests(&query).unwrap().is_empty());
        assert_eq!(state.count_captured_requests(&query).unwrap(), 0);

        let query = CapturedRequestQuery {
            session: SessionScope::Exact("session-1".to_owned()),
            limit: Some(1),
            offset: 1,
            ..Default::default()
        };
        assert!(state.captured_requests(&query).unwrap().is_empty());
        assert_eq!(state.count_captured_requests(&query).unwrap(), 1);
    }

    #[test]
    fn websocket_frames_share_the_captured_request_module() {
        let state = DbState::new_in_memory(std::sync::Mutex::new(())).unwrap();
        let id = state
            .record_captured_request(NewCapturedRequest::from_intercepted(&request()))
            .unwrap();
        state
            .mark_captured_request_websocket(&id.to_string())
            .unwrap();
        state
            .record_websocket_frame(NewWebSocketFrame {
                request_id: &id.to_string(),
                direction: "incoming",
                opcode: 1,
                payload: "hello",
                payload_binary: None,
                size: 5,
                timestamp: "2026-08-04 10:00:01",
            })
            .unwrap();

        assert!(state.captured_request(id).unwrap().unwrap().is_websocket);
        assert_eq!(state.websocket_frames(&id.to_string()).unwrap().len(), 1);
    }
}
