//! SQLite database module for ProxyBot.
//!
//! Manages the database at ~/.proxybot/proxybot.db with WAL mode enabled.
//! Tables: http_requests, dns_queries, devices, app_tags

use rusqlite::{Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

/// Database state managed by Tauri.
pub struct DbState {
    pub conn: Mutex<Connection>,
}

impl DbState {
    /// Open (or create) the database at ~/.proxybot/proxybot.db
    /// and initialize the schema with WAL mode.
    pub fn new() -> SqlResult<Self> {
        let db_path = Self::db_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;

        // Enable WAL mode for concurrent read/write
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Initialize schema
        Self::init_schema(&conn)?;

        log::info!("Database initialized at {:?}", db_path);
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create an in-memory database for CLI mode (e.g., MCP stdio).
    /// Does not persist data - useful for standalone tools.
    pub fn new_in_memory(_guard: std::sync::Mutex<()>) -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;

        // Initialize schema
        Self::init_schema(&conn)?;

        log::info!("In-memory database initialized for CLI mode");
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Internal method to get device by IP (used by mac_address field).
    pub(crate) fn get_device_by_ip_internal(&self, ip: &str) -> Option<DeviceInfo> {
        let conn = self.conn.lock().ok()?;
        conn.query_row(
            "SELECT id, mac_address, name, created_at, last_seen_at, upload_bytes, download_bytes, rule_override
             FROM devices WHERE mac_address = ?1",
            rusqlite::params![ip],
            |row| {
                Ok(DeviceInfo {
                    id: row.get(0)?,
                    mac_address: row.get(1)?,
                    name: row.get(2)?,
                    created_at: row.get(3)?,
                    last_seen_at: row.get(4)?,
                    upload_bytes: row.get(5)?,
                    download_bytes: row.get(6)?,
                    rule_override: row.get(7)?,
                })
            },
        )
        .ok()
    }

    /// Internal method to register a device.
    pub(crate) fn register_device_internal(
        &self,
        ip: &str,
        name: &str,
    ) -> Result<DeviceInfo, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = chrono_lite_timestamp();

        // Try to insert, on conflict do nothing and select existing
        conn.execute(
            "INSERT OR IGNORE INTO devices (mac_address, name, created_at, last_seen_at, upload_bytes, download_bytes)
             VALUES (?1, ?2, ?3, ?3, 0, 0)",
            rusqlite::params![ip, name, now],
        )
        .map_err(|e| e.to_string())?;

        // Update last_seen and get the device
        conn.execute(
            "UPDATE devices SET last_seen_at = ?1 WHERE mac_address = ?2",
            rusqlite::params![now, ip],
        )
        .map_err(|e| e.to_string())?;

        let device = conn
            .query_row(
                "SELECT id, mac_address, name, created_at, last_seen_at, upload_bytes, download_bytes, rule_override
                 FROM devices WHERE mac_address = ?1",
                rusqlite::params![ip],
                |row| {
                    Ok(DeviceInfo {
                        id: row.get(0)?,
                        mac_address: row.get(1)?,
                        name: row.get(2)?,
                        created_at: row.get(3)?,
                        last_seen_at: row.get(4)?,
                        upload_bytes: row.get(5)?,
                        download_bytes: row.get(6)?,
                        rule_override: row.get(7)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?;

        Ok(device)
    }

    fn db_path() -> PathBuf {
        crate::config::db_path()
    }

    pub(crate) fn init_schema(conn: &Connection) -> SqlResult<()> {
        // Create schema_version table first for migration tracking
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_version (
                version     INTEGER PRIMARY KEY,
                applied_at  TEXT NOT NULL,
                description TEXT NOT NULL
            );
            "#,
        )?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS http_requests (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp   TEXT NOT NULL,
                method      TEXT NOT NULL,
                scheme      TEXT NOT NULL,
                host        TEXT NOT NULL,
                path        TEXT NOT NULL,
                req_headers TEXT NOT NULL DEFAULT '{}',
                req_body    BLOB,
                resp_status INTEGER,
                resp_headers TEXT NOT NULL DEFAULT '{}',
                resp_body   BLOB,
                duration_ms INTEGER,
                device_id   INTEGER,
                app_tag     TEXT,
                FOREIGN KEY (device_id) REFERENCES devices(id)
            );

            CREATE TABLE IF NOT EXISTS dns_queries (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp    TEXT NOT NULL,
                query_name   TEXT NOT NULL,
                query_type   INTEGER NOT NULL,
                response_ips TEXT NOT NULL DEFAULT '[]',
                device_id    INTEGER,
                app_tag      TEXT,
                FOREIGN KEY (device_id) REFERENCES devices(id)
            );

            CREATE TABLE IF NOT EXISTS devices (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                mac_address    TEXT UNIQUE NOT NULL,
                name           TEXT NOT NULL,
                created_at     TEXT NOT NULL,
                last_seen_at   TEXT NOT NULL,
                upload_bytes   INTEGER NOT NULL DEFAULT 0,
                download_bytes INTEGER NOT NULL DEFAULT 0,
                rule_override  TEXT
            );

            CREATE TABLE IF NOT EXISTS app_tags (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                app_name    TEXT UNIQUE NOT NULL,
                domain_rule TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS inferred_apis (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id      TEXT NOT NULL,
                name            TEXT NOT NULL,
                method          TEXT NOT NULL,
                path            TEXT NOT NULL,
                params          TEXT NOT NULL DEFAULT '{}',
                auth_required   INTEGER NOT NULL DEFAULT 0,
                request_ids     TEXT NOT NULL DEFAULT '[]',
                score           REAL,
                created_at      TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_inferred_apis_session ON inferred_apis(session_id);

            CREATE TABLE IF NOT EXISTS inference_evaluations (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id      TEXT NOT NULL,
                valid           INTEGER NOT NULL DEFAULT 0,
                errors          TEXT NOT NULL DEFAULT '[]',
                score           REAL NOT NULL,
                evaluated_at    TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_inference_evaluations_session ON inference_evaluations(session_id);

            CREATE INDEX IF NOT EXISTS idx_http_requests_host ON http_requests(host);
            CREATE INDEX IF NOT EXISTS idx_http_requests_timestamp ON http_requests(timestamp);
            CREATE INDEX IF NOT EXISTS idx_http_requests_device_id ON http_requests(device_id);
            CREATE INDEX IF NOT EXISTS idx_dns_queries_timestamp ON dns_queries(timestamp);
            CREATE INDEX IF NOT EXISTS idx_dns_queries_device_id ON dns_queries(device_id);

            CREATE TABLE IF NOT EXISTS dag_nodes (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                host TEXT NOT NULL,
                device_id INTEGER,
                FOREIGN KEY (device_id) REFERENCES devices(id)
            );

            CREATE TABLE IF NOT EXISTS dag_edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_node_id INTEGER NOT NULL,
                to_node_id INTEGER NOT NULL,
                token_value TEXT NOT NULL,
                FOREIGN KEY (from_node_id) REFERENCES dag_nodes(id),
                FOREIGN KEY (to_node_id) REFERENCES dag_nodes(id)
            );

            CREATE INDEX IF NOT EXISTS idx_dag_edges_from ON dag_edges(from_node_id);
            CREATE INDEX IF NOT EXISTS idx_dag_edges_to ON dag_edges(to_node_id);

            CREATE TABLE IF NOT EXISTS alerts (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id       INTEGER,
                severity        TEXT NOT NULL,
                alert_type      TEXT NOT NULL,
                details         TEXT NOT NULL,
                created_at      TEXT NOT NULL,
                acknowledged    INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (device_id) REFERENCES devices(id)
            );

            CREATE INDEX IF NOT EXISTS idx_alerts_device_id ON alerts(device_id);
            CREATE INDEX IF NOT EXISTS idx_alerts_severity ON alerts(severity);

            CREATE TABLE IF NOT EXISTS vision_analyses (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id      TEXT NOT NULL,
                filename        TEXT NOT NULL,
                components_json TEXT NOT NULL,
                raw_response    TEXT NOT NULL,
                score           REAL NOT NULL DEFAULT 0.0,
                created_at      TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_vision_analyses_session ON vision_analyses(session_id);

            CREATE TABLE IF NOT EXISTS ai_token_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT,
                request_id TEXT,
                prompt_tokens INTEGER DEFAULT 0,
                completion_tokens INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                max_tokens INTEGER,
                context_window INTEGER,
                estimated BOOLEAN DEFAULT 0,
                cost_usd REAL DEFAULT 0.0
            );

            CREATE INDEX IF NOT EXISTS idx_ai_token_usage_provider ON ai_token_usage(provider);
            CREATE INDEX IF NOT EXISTS idx_ai_token_usage_timestamp ON ai_token_usage(timestamp);
            "#,
        )?;

        // Record baseline as version 0 if not already recorded
        let current_version = Self::get_schema_version(conn)?;
        if current_version < 0 {
            conn.execute(
                "INSERT OR IGNORE INTO schema_version (version, applied_at, description) VALUES (0, ?1, 'Baseline schema')",
                rusqlite::params![chrono_lite_timestamp()],
            )?;
        }

        // Run pending migrations
        Self::run_migrations(conn)?;

        Ok(())
    }

    /// Get the current schema version, or -1 if no version recorded.
    fn get_schema_version(conn: &Connection) -> SqlResult<i64> {
        let version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), -1) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(-1);
        Ok(version)
    }

    /// Run pending migrations. Each migration is a (version, description, sql) tuple.
    fn run_migrations(conn: &Connection) -> SqlResult<()> {
        let current = Self::get_schema_version(conn)?;

        let migrations: Vec<(i64, &str, &str)> = vec![
            (
                1,
                "Add response_size column to http_requests",
                "ALTER TABLE http_requests ADD COLUMN response_size INTEGER;",
            ),
            (
                2,
                "Add query_name index for DNS lookups",
                "CREATE INDEX IF NOT EXISTS idx_dns_queries_query_name ON dns_queries(query_name);",
            ),
            (
                3,
                "Add ws_frames table and is_websocket column",
                r#"
                CREATE TABLE IF NOT EXISTS ws_frames (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    request_id  TEXT NOT NULL,
                    direction   TEXT NOT NULL CHECK (direction IN ('incoming', 'outgoing')),
                    opcode      INTEGER NOT NULL,
                    payload     TEXT NOT NULL DEFAULT '',
                    payload_bin BLOB,
                    size        INTEGER NOT NULL DEFAULT 0,
                    timestamp   TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_ws_frames_request_id ON ws_frames(request_id);
                ALTER TABLE http_requests ADD COLUMN is_websocket INTEGER NOT NULL DEFAULT 0;
                "#,
            ),
            (
                4,
                "Add deployments table for Deploy panel persistence",
                r#"
                CREATE TABLE IF NOT EXISTS deployments (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL,
                    project_name TEXT NOT NULL,
                    bundle_path TEXT NOT NULL,
                    last_git_init_at TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    UNIQUE(session_id, project_name)
                );
                CREATE INDEX IF NOT EXISTS idx_deployments_session_project
                    ON deployments(session_id, project_name);
                "#,
            ),
        ];

        for (version, description, sql) in migrations {
            if version > current {
                log::info!("Running migration {}: {}", version, description);
                conn.execute_batch(sql)?;
                conn.execute(
                    "INSERT INTO schema_version (version, applied_at, description) VALUES (?1, ?2, ?3)",
                    rusqlite::params![version, chrono_lite_timestamp(), description],
                )?;
            }
        }

        Ok(())
    }
}

/// Statistics about the database tables.
#[derive(Serialize)]
pub struct DbStats {
    pub http_requests_count: i64,
    pub dns_queries_count: i64,
    pub devices_count: i64,
    pub app_tags_count: i64,
}

/// Device information for UI display.
#[derive(Serialize, Clone)]
pub struct DeviceInfo {
    pub id: i64,
    pub mac_address: String,
    pub name: String,
    pub created_at: String,
    pub last_seen_at: String,
    pub upload_bytes: i64,
    pub download_bytes: i64,
    pub rule_override: Option<String>,
}

/// Get database statistics.
#[tauri::command]
pub fn get_db_stats(state: State<'_, Arc<DbState>>) -> Result<DbStats, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let http_requests_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let dns_queries_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM dns_queries", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let devices_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let app_tags_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM app_tags", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    Ok(DbStats {
        http_requests_count,
        dns_queries_count,
        devices_count,
        app_tags_count,
    })
}

/// Get all registered devices - internal non-Tauri version.
pub fn get_devices_internal(conn: &Connection) -> Result<Vec<DeviceInfo>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, mac_address, name, created_at, last_seen_at, upload_bytes, download_bytes, rule_override
             FROM devices ORDER BY last_seen_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let devices = stmt
        .query_map([], |row| {
            Ok(DeviceInfo {
                id: row.get(0)?,
                mac_address: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
                last_seen_at: row.get(4)?,
                upload_bytes: row.get(5)?,
                download_bytes: row.get(6)?,
                rule_override: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(devices)
}

/// Get all registered devices.
#[tauri::command]
pub fn get_devices(state: State<'_, Arc<DbState>>) -> Result<Vec<DeviceInfo>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    get_devices_internal(&conn)
}

/// Register a new device or return existing device id.
#[tauri::command]
pub fn register_device(
    state: State<'_, Arc<DbState>>,
    mac_address: String,
    name: String,
) -> Result<DeviceInfo, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono_lite_timestamp();

    // Try to insert, on conflict do nothing and select existing
    conn.execute(
        "INSERT OR IGNORE INTO devices (mac_address, name, created_at, last_seen_at, upload_bytes, download_bytes)
         VALUES (?1, ?2, ?3, ?3, 0, 0)",
        rusqlite::params![mac_address, name, now],
    )
    .map_err(|e| e.to_string())?;

    // Get the device (either newly inserted or existing)
    let device = conn
        .query_row(
            "SELECT id, mac_address, name, created_at, last_seen_at, upload_bytes, download_bytes, rule_override
             FROM devices WHERE mac_address = ?1",
            rusqlite::params![mac_address],
            |row| {
                Ok(DeviceInfo {
                    id: row.get(0)?,
                    mac_address: row.get(1)?,
                    name: row.get(2)?,
                    created_at: row.get(3)?,
                    last_seen_at: row.get(4)?,
                    upload_bytes: row.get(5)?,
                    download_bytes: row.get(6)?,
                    rule_override: row.get(7)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(device)
}

/// Update device last seen timestamp.
#[tauri::command]
pub fn update_device_last_seen(
    state: State<'_, Arc<DbState>>,
    mac_address: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono_lite_timestamp();
    conn.execute(
        "UPDATE devices SET last_seen_at = ?1 WHERE mac_address = ?2",
        rusqlite::params![now, mac_address],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Update device byte counters.
#[tauri::command]
pub fn update_device_stats(
    state: State<'_, Arc<DbState>>,
    mac_address: String,
    upload_bytes: i64,
    download_bytes: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE devices SET upload_bytes = upload_bytes + ?1, download_bytes = download_bytes + ?2
         WHERE mac_address = ?3",
        rusqlite::params![upload_bytes, download_bytes, mac_address],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Set device rule override (internal, takes Connection directly).
pub fn set_device_rule_override_internal(
    conn: &Connection,
    mac_address: &str,
    rule_override: Option<String>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE devices SET rule_override = ?1 WHERE mac_address = ?2",
        rusqlite::params![rule_override, mac_address],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Set device rule override.
#[tauri::command]
pub fn set_device_rule_override(
    state: State<'_, Arc<DbState>>,
    mac_address: String,
    rule_override: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE devices SET rule_override = ?1 WHERE mac_address = ?2",
        rusqlite::params![rule_override, mac_address],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Get device by MAC address.
#[tauri::command]
pub fn get_device_by_mac(
    state: State<'_, Arc<DbState>>,
    mac_address: String,
) -> Result<Option<DeviceInfo>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let result = conn.query_row(
        "SELECT id, mac_address, name, created_at, last_seen_at, upload_bytes, download_bytes, rule_override
         FROM devices WHERE mac_address = ?1",
        rusqlite::params![mac_address],
        |row| {
            Ok(DeviceInfo {
                id: row.get(0)?,
                mac_address: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
                last_seen_at: row.get(4)?,
                upload_bytes: row.get(5)?,
                download_bytes: row.get(6)?,
                rule_override: row.get(7)?,
            })
        },
    );

    match result {
        Ok(device) => Ok(Some(device)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Format timestamp for SQLite (YYYY-MM-DD HH:MM:SS).
pub(crate) fn chrono_lite_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    let mut remaining = secs;

    // Years
    let mut year = 1970;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year * 86400 {
            break;
        }
        remaining -= days_in_year * 86400;
        year += 1;
    }

    // Months
    let days_in_months: &[u64] = if is_leap_year(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for days in days_in_months {
        if remaining < days * 86400 {
            break;
        }
        remaining -= days * 86400;
        month += 1;
    }

    // Days, hours, minutes, seconds
    let day = (remaining / 86400) + 1;
    remaining %= 86400;
    let hour = remaining / 3600;
    remaining %= 3600;
    let minute = remaining / 60;
    let second = remaining % 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    )
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Record an HTTP request to the database (for TUI and non-Tauri usage).
pub fn record_http_request(
    conn: &Connection,
    timestamp: &str,
    method: &str,
    scheme: &str,
    host: &str,
    path: &str,
    req_headers: &[(String, String)],
    req_body: Option<&str>,
    resp_status: Option<u16>,
    resp_headers: &[(String, String)],
    resp_body: Option<&str>,
    duration_ms: Option<u64>,
    device_id: Option<i64>,
    app_tag: Option<&str>,
) -> Result<i64, String> {
    let req_headers_json = serde_json::to_string(req_headers).map_err(|e| e.to_string())?;
    let resp_headers_json = serde_json::to_string(resp_headers).map_err(|e| e.to_string())?;
    let req_body_bytes: Option<Vec<u8>> = req_body.map(|s| s.as_bytes().to_vec());
    let resp_body_bytes: Option<Vec<u8>> = resp_body.map(|s| s.as_bytes().to_vec());

    conn.execute(
        r#"INSERT INTO http_requests
           (timestamp, method, scheme, host, path, req_headers, req_body,
            resp_status, resp_headers, resp_body, duration_ms, device_id, app_tag)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"#,
        rusqlite::params![
            timestamp,
            method,
            scheme,
            host,
            path,
            req_headers_json,
            req_body_bytes,
            resp_status,
            resp_headers_json,
            resp_body_bytes,
            duration_ms,
            device_id,
            app_tag,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

/// Get recent HTTP requests for TUI display.
pub fn get_recent_requests(conn: &Connection, limit: i64) -> Result<Vec<RecentRequest>, String> {
    let mut stmt = conn
        .prepare(
            r#"SELECT id, timestamp, method, scheme, host, path,
                      resp_status, duration_ms, app_tag
               FROM http_requests
               ORDER BY id DESC
               LIMIT ?1"#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([limit], |row| {
            Ok(RecentRequest {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                method: row.get(2)?,
                scheme: row.get(3)?,
                host: row.get(4)?,
                path: row.get(5)?,
                status: row.get(6)?,
                duration_ms: row.get(7)?,
                app_tag: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut requests = Vec::new();
    for row in rows {
        if let Ok(r) = row {
            requests.push(r);
        }
    }
    Ok(requests)
}

/// Timestamp formatted as "YYYY-MM-DD HH:MM:SS" for WS frame recording.
pub fn timestamp_now_for_ws() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Record a WebSocket frame to the database.
pub fn record_ws_frame(
    conn: &Connection,
    request_id: &str,
    direction: &str,
    opcode: u8,
    payload: &str,
    payload_bin: Option<&[u8]>,
    size: usize,
    timestamp: &str,
) -> Result<i64, String> {
    conn.execute(
        r#"INSERT INTO ws_frames
           (request_id, direction, opcode, payload, payload_bin, size, timestamp)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
        rusqlite::params![
            request_id,
            direction,
            opcode,
            payload,
            payload_bin,
            size as i64,
            timestamp,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

/// Mark an HTTP request as a WebSocket connection.
pub fn mark_request_websocket(conn: &Connection, request_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE http_requests SET is_websocket = 1 WHERE id = ?1",
        rusqlite::params![request_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Lightweight request struct for TUI list view.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RecentRequest {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub status: Option<u16>,
    pub duration_ms: Option<i64>,
    pub app_tag: Option<String>,
}

/// AI token usage summary for stats queries.
#[derive(Serialize, Debug)]
pub struct AiTokenStats {
    pub provider: String,
    pub model: String,
    pub total_tokens: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cost_usd: f64,
    pub requests: i64,
}

/// Deployment record persisted per (session_id, project_name) for the Deploy panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecord {
    pub id: i64,
    pub session_id: String,
    pub project_name: String,
    pub bundle_path: String,
    pub last_git_init_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Insert or update a deployment record for (session_id, project_name).
///
/// On conflict, `bundle_path` is overwritten with the new value. `last_git_init_at`
/// is treated as a monotonic timestamp: passing `Some(_)` overwrites the previous
/// value, but passing `None` preserves any prior value via COALESCE. This ensures
/// that re-writing the bundle record (e.g. for a new deployment of an existing
/// project) does not erase the original "when was git last initialized" timestamp.
pub fn upsert_deployment(
    conn: &Connection,
    session_id: &str,
    project_name: &str,
    bundle_path: &str,
    last_git_init_at: Option<&str>,
) -> Result<(), String> {
    let now = chrono_lite_timestamp();
    conn.execute(
        r#"
        INSERT INTO deployments
            (session_id, project_name, bundle_path, last_git_init_at, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?5)
        ON CONFLICT(session_id, project_name) DO UPDATE SET
            bundle_path = excluded.bundle_path,
            last_git_init_at = COALESCE(excluded.last_git_init_at, deployments.last_git_init_at),
            updated_at = excluded.updated_at
        "#,
        rusqlite::params![session_id, project_name, bundle_path, last_git_init_at, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Fetch a deployment record for (session_id, project_name), or None if absent.
pub fn get_deployment(
    conn: &Connection,
    session_id: &str,
    project_name: &str,
) -> Result<Option<DeploymentRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, project_name, bundle_path, last_git_init_at, created_at, updated_at
             FROM deployments WHERE session_id = ?1 AND project_name = ?2",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map(rusqlite::params![session_id, project_name], |row| {
            Ok(DeploymentRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                project_name: row.get(2)?,
                bundle_path: row.get(3)?,
                last_git_init_at: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    match rows.next() {
        Some(Ok(rec)) => Ok(Some(rec)),
        Some(Err(e)) => Err(e.to_string()),
        None => Ok(None),
    }
}

impl DbState {
    /// Record AI token usage from the tracker.
    ///
    /// **Schema coupling note**: This INSERT writes 11 columns to `ai_token_usage`.
    /// If the table schema changes, `get_ai_stats()` (which groups by provider+model
    /// with 6 aggregate columns) must be kept in sync. Both are tested together
    /// via `cargo test --lib ai_stats`.
    pub fn record_token_usage(
        &self,
        timestamp: &str,
        provider: &str,
        model: &str,
        request_id: &str,
        prompt_tokens: i64,
        completion_tokens: i64,
        total_tokens: i64,
        context_window: i64,
        estimated: bool,
        cost_usd: f64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO ai_token_usage (timestamp, provider, model, request_id, prompt_tokens, completion_tokens, total_tokens, max_tokens, context_window, estimated, cost_usd) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                timestamp,
                provider,
                model,
                request_id,
                prompt_tokens,
                completion_tokens,
                total_tokens,
                0i64,
                context_window,
                estimated,
                cost_usd,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Get aggregated AI token usage stats grouped by provider and model.
    /// **Schema coupling note**: Reads 7 columns (index 0-6) from `ai_token_usage` via
    /// GROUP BY aggregation. Column count and order must match the INSERT in
    /// `record_token_usage()` above. Tested together via `cargo test --lib ai_stats`.
    pub fn get_ai_stats(&self) -> Result<Vec<AiTokenStats>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT provider, model, SUM(total_tokens) as total, SUM(prompt_tokens) as prompt_total, SUM(completion_tokens) as completion_total, SUM(cost_usd) as cost, COUNT(*) as requests FROM ai_token_usage GROUP BY provider, model",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(AiTokenStats {
                    provider: row.get(0)?,
                    model: row.get(1)?,
                    total_tokens: row.get(2)?,
                    prompt_tokens: row.get(3)?,
                    completion_tokens: row.get(4)?,
                    cost_usd: row.get(5)?,
                    requests: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        DbState::init_schema(&conn).unwrap();

        // Verify tables exist
        conn.execute(
            "INSERT INTO devices (mac_address, name, created_at, last_seen_at)
             VALUES ('AA:BB:CC:DD:EE:FF', 'Test Device', '2024-01-01', '2024-01-01')",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_schema_version_tracking() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        DbState::init_schema(&conn).unwrap();

        // schema_version table should exist and have baseline
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert!(version >= 0, "Schema version should be >= 0 after init");
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();

        // Run init_schema twice — should not error
        DbState::init_schema(&conn).unwrap();
        DbState::init_schema(&conn).unwrap();

        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert!(version >= 0);
    }

    #[test]
    fn test_migration_adds_response_size_column() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        DbState::init_schema(&conn).unwrap();

        // Migration 1 should have added response_size column
        conn.execute(
            "INSERT INTO http_requests (timestamp, method, scheme, host, path, response_size)
             VALUES ('2024-01-01', 'GET', 'https', 'example.com', '/', 1024)",
            [],
        )
        .unwrap();

        let size: i64 = conn
            .query_row("SELECT response_size FROM http_requests LIMIT 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(size, 1024);
    }

    #[test]
    fn test_deployments_table_upsert_and_get() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        DbState::init_schema(&conn).unwrap();

        // Initial upsert with a git-init timestamp
        upsert_deployment(&conn, "sess1", "proj1", "/tmp/proj1", Some("2026-06-04T00:00:00Z")).unwrap();
        let rec = get_deployment(&conn, "sess1", "proj1").unwrap().unwrap();
        assert_eq!(rec.session_id, "sess1");
        assert_eq!(rec.project_name, "proj1");
        assert_eq!(rec.bundle_path, "/tmp/proj1");
        assert_eq!(rec.last_git_init_at, Some("2026-06-04T00:00:00Z".to_string()));

        // Upserting with None preserves the existing last_git_init_at (COALESCE semantics)
        upsert_deployment(&conn, "sess1", "proj1", "/tmp/proj1_v2", None).unwrap();
        let rec = get_deployment(&conn, "sess1", "proj1").unwrap().unwrap();
        assert_eq!(rec.bundle_path, "/tmp/proj1_v2");
        assert_eq!(
            rec.last_git_init_at,
            Some("2026-06-04T00:00:00Z".to_string()),
            "Re-write with None must preserve prior git-init timestamp"
        );

        // Explicitly passing a new timestamp overwrites
        upsert_deployment(&conn, "sess1", "proj1", "/tmp/proj1_v3", Some("2026-06-05T00:00:00Z")).unwrap();
        let rec = get_deployment(&conn, "sess1", "proj1").unwrap().unwrap();
        assert_eq!(rec.bundle_path, "/tmp/proj1_v3");
        assert_eq!(rec.last_git_init_at, Some("2026-06-05T00:00:00Z".to_string()));

        // Missing returns None
        let none = get_deployment(&conn, "sess1", "missing").unwrap();
        assert!(none.is_none());
    }
}
