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
            (
                5,
                "Add session_id column to http_requests for session-scoped queries",
                r#"
                ALTER TABLE http_requests ADD COLUMN session_id TEXT;
                CREATE INDEX IF NOT EXISTS idx_http_requests_session_id ON http_requests(session_id);
                "#,
            ),
            (
                6,
                "Add tls_decryption_rules table for per-host MITM policy",
                r#"
                CREATE TABLE IF NOT EXISTS tls_decryption_rules (
                    id         INTEGER PRIMARY KEY AUTOINCREMENT,
                    pattern    TEXT NOT NULL UNIQUE,
                    action     TEXT NOT NULL CHECK (action IN ('Decrypt', 'Bypass', 'Passthrough')),
                    hit_count  INTEGER NOT NULL DEFAULT 0,
                    sort_order INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_tls_rules_sort ON tls_decryption_rules(sort_order);
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

/// Register a new device or return existing device id - internal non-Tauri version.
///
/// **Note on naming:** The `_only` suffix distinguishes this helper from
/// [`DbState::register_device_internal`], which does the same INSERT OR IGNORE
/// PLUS an `UPDATE devices SET last_seen_at = ?` on top. This helper deliberately
/// does NOT touch `last_seen_at` — call [`update_device_last_seen_internal`]
/// separately when you need that side effect.
pub(crate) fn register_device_only(
    conn: &Connection,
    mac_address: &str,
    name: &str,
) -> Result<DeviceInfo, String> {
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

/// Register a new device or return existing device id.
#[tauri::command]
pub fn register_device(
    state: State<'_, Arc<DbState>>,
    mac_address: String,
    name: String,
) -> Result<DeviceInfo, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    register_device_only(&conn, &mac_address, &name)
}

/// Update device last seen timestamp - internal non-Tauri version.
pub(crate) fn update_device_last_seen_internal(
    conn: &Connection,
    mac_address: &str,
) -> Result<(), String> {
    let now = chrono_lite_timestamp();
    conn.execute(
        "UPDATE devices SET last_seen_at = ?1 WHERE mac_address = ?2",
        rusqlite::params![now, mac_address],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Update device last seen timestamp.
#[tauri::command]
pub fn update_device_last_seen(
    state: State<'_, Arc<DbState>>,
    mac_address: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    update_device_last_seen_internal(&conn, &mac_address)
}

/// Update device byte counters - internal non-Tauri version.
pub(crate) fn update_device_stats_internal(
    conn: &Connection,
    mac_address: &str,
    upload_bytes: i64,
    download_bytes: i64,
) -> Result<(), String> {
    conn.execute(
        "UPDATE devices SET upload_bytes = upload_bytes + ?1, download_bytes = download_bytes + ?2
         WHERE mac_address = ?3",
        rusqlite::params![upload_bytes, download_bytes, mac_address],
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
    update_device_stats_internal(&conn, &mac_address, upload_bytes, download_bytes)
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

/// Get device by MAC address - internal non-Tauri version.
pub(crate) fn get_device_by_mac_internal(
    conn: &Connection,
    mac_address: &str,
) -> Result<Option<DeviceInfo>, String> {
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

/// Get device by MAC address.
#[tauri::command]
pub fn get_device_by_mac(
    state: State<'_, Arc<DbState>>,
    mac_address: String,
) -> Result<Option<DeviceInfo>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    get_device_by_mac_internal(&conn, &mac_address)
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

pub(crate) fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

/// Record an HTTP request to the database (for TUI and non-Tauri usage).
// This is a row-shaped persistence boundary; keeping columns explicit makes
// schema changes reviewable alongside the INSERT below.
#[allow(clippy::too_many_arguments)]
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
    session_id: Option<&str>,
) -> Result<i64, String> {
    let req_headers_json = serde_json::to_string(req_headers).map_err(|e| e.to_string())?;
    let resp_headers_json = serde_json::to_string(resp_headers).map_err(|e| e.to_string())?;
    let req_body_bytes: Option<Vec<u8>> = req_body.map(|s| s.as_bytes().to_vec());
    let resp_body_bytes: Option<Vec<u8>> = resp_body.map(|s| s.as_bytes().to_vec());

    conn.execute(
        r#"INSERT INTO http_requests
           (timestamp, method, scheme, host, path, req_headers, req_body,
            resp_status, resp_headers, resp_body, duration_ms, device_id, app_tag,
            session_id)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"#,
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
            session_id,
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
    for r in rows.flatten() {
        requests.push(r);
    }
    Ok(requests)
}

/// Timestamp formatted as "YYYY-MM-DD HH:MM:SS" for WS frame recording.
pub fn timestamp_now_for_ws() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Record a WebSocket frame to the database.
// This is a row-shaped persistence boundary; arguments mirror table columns.
#[allow(clippy::too_many_arguments)]
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

/// Retrieve all WebSocket frames for a request, ordered by timestamp ascending.
pub fn get_ws_frames(
    conn: &Connection,
    request_id: &str,
) -> Result<Vec<crate::proxy::WsFrame>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT direction, opcode, payload, size, timestamp
             FROM ws_frames WHERE request_id = ?1 ORDER BY timestamp ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([request_id], |row| {
            let opcode: i32 = row.get(1)?;
            let payload: String = row.get(2)?;
            let size: i64 = row.get(3)?;
            let timestamp: String = row.get(4)?;
            let truncated = (size as usize) > crate::ws_frames::MAX_PAYLOAD_SIZE;
            Ok(crate::proxy::WsFrame {
                direction: row.get(0)?,
                opcode: opcode as u8,
                payload,
                size: size as usize,
                timestamp,
                truncated,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
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

/// A persisted per-host TLS decryption rule. `action` is one of
/// `Decrypt` / `Bypass` / `Passthrough` (validated by the table's
/// CHECK constraint and re-parsed into `TlsAction` by the command
/// layer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsRuleRow {
    pub id: i64,
    pub pattern: String,
    pub action: String,
    pub hit_count: i64,
    pub sort_order: i64,
}

/// Load all TLS decryption rules ordered by `sort_order` (first match
/// wins, so the UI controls precedence through this column).
pub fn get_tls_rules(conn: &Connection) -> Result<Vec<TlsRuleRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, pattern, action, hit_count, sort_order
             FROM tls_decryption_rules
             ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(TlsRuleRow {
                id: row.get(0)?,
                pattern: row.get(1)?,
                action: row.get(2)?,
                hit_count: row.get(3)?,
                sort_order: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// Insert a TLS rule. `sort_order` defaults to the current max + 1 so
/// new rules land at the end (lowest precedence) unless the caller
/// reorders them. Returns the new row id.
pub fn add_tls_rule(conn: &Connection, pattern: &str, action: &str) -> Result<i64, String> {
    let next_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM tls_decryption_rules",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO tls_decryption_rules (pattern, action, hit_count, sort_order, created_at)
         VALUES (?1, ?2, 0, ?3, ?4)",
        rusqlite::params![pattern, action, next_order, chrono_lite_timestamp()],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// Delete a TLS rule by id.
pub fn delete_tls_rule(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM tls_decryption_rules WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

impl DbState {
    /// Record AI token usage from the tracker.
    ///
    /// **Schema coupling note**: This INSERT writes 11 columns to `ai_token_usage`.
    /// If the table schema changes, `get_ai_stats()` (which groups by provider+model
    /// with 6 aggregate columns) must be kept in sync. Both are tested together
    /// via `cargo test --lib ai_stats`.
    #[allow(clippy::too_many_arguments)]
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
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
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
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
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
            .query_row(
                "SELECT response_size FROM http_requests LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(size, 1024);
    }

    #[test]
    fn test_deployments_table_upsert_and_get() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        DbState::init_schema(&conn).unwrap();

        // Initial upsert with a git-init timestamp
        upsert_deployment(
            &conn,
            "sess1",
            "proj1",
            "/tmp/proj1",
            Some("2026-06-04T00:00:00Z"),
        )
        .unwrap();
        let rec = get_deployment(&conn, "sess1", "proj1").unwrap().unwrap();
        assert_eq!(rec.session_id, "sess1");
        assert_eq!(rec.project_name, "proj1");
        assert_eq!(rec.bundle_path, "/tmp/proj1");
        assert_eq!(
            rec.last_git_init_at,
            Some("2026-06-04T00:00:00Z".to_string())
        );

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
        upsert_deployment(
            &conn,
            "sess1",
            "proj1",
            "/tmp/proj1_v3",
            Some("2026-06-05T00:00:00Z"),
        )
        .unwrap();
        let rec = get_deployment(&conn, "sess1", "proj1").unwrap().unwrap();
        assert_eq!(rec.bundle_path, "/tmp/proj1_v3");
        assert_eq!(
            rec.last_git_init_at,
            Some("2026-06-05T00:00:00Z".to_string())
        );

        // Missing returns None
        let none = get_deployment(&conn, "sess1", "missing").unwrap();
        assert!(none.is_none());
    }

    // ------------------------------------------------------------------
    // Device CRUD tests
    // ------------------------------------------------------------------

    #[test]
    fn test_register_device_inserts_new_device() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let device = register_device_only(&conn, "aa:bb:cc:dd:ee:ff", "Test Phone").unwrap();
        assert_eq!(device.mac_address, "aa:bb:cc:dd:ee:ff");
        assert_eq!(device.name, "Test Phone");
        assert_eq!(
            device.upload_bytes, 0,
            "New device should have zero upload_bytes"
        );
        assert_eq!(
            device.download_bytes, 0,
            "New device should have zero download_bytes"
        );
        assert!(device.id > 0, "Auto-incremented id should be > 0");
        assert!(
            !device.created_at.is_empty(),
            "created_at should be populated"
        );
        assert!(
            !device.last_seen_at.is_empty(),
            "last_seen_at should be populated"
        );
    }

    #[test]
    fn test_register_device_returns_existing_on_duplicate_mac() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let first = register_device_only(&conn, "11:22:33:44:55:66", "Original Name").unwrap();
        let second = register_device_only(&conn, "11:22:33:44:55:66", "Different Name").unwrap();

        assert_eq!(
            first.id, second.id,
            "Duplicate MAC should return the same row id"
        );
        assert_eq!(
            second.name, "Original Name",
            "INSERT OR IGNORE preserves original name on duplicate MAC"
        );
        assert_eq!(second.mac_address, "11:22:33:44:55:66");
    }

    #[test]
    fn test_get_device_by_mac_returns_inserted_device() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let inserted = register_device_only(&conn, "de:ad:be:ef:00:01", "My Laptop").unwrap();
        let fetched = get_device_by_mac_internal(&conn, "de:ad:be:ef:00:01").unwrap();

        assert!(fetched.is_some(), "Should find the just-inserted device");
        let device = fetched.unwrap();
        assert_eq!(device.id, inserted.id);
        assert_eq!(device.mac_address, "de:ad:be:ef:00:01");
        assert_eq!(device.name, "My Laptop");
    }

    #[test]
    fn test_get_device_by_mac_returns_none_for_missing() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let result = get_device_by_mac_internal(&conn, "ff:ff:ff:ff:ff:ff").unwrap();
        assert!(
            result.is_none(),
            "Missing MAC should return Ok(None), not an error"
        );
    }

    #[test]
    fn test_update_device_last_seen_changes_timestamp() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        // Manually backdate last_seen_at to a known earlier value, then call
        // update_device_last_seen_internal and verify it was updated to "now"
        // (strictly greater than the backdated value).
        conn.execute(
            "INSERT INTO devices (mac_address, name, created_at, last_seen_at)
             VALUES ('aa:bb:cc:dd:ee:01', 'Phone', '2000-01-01 00:00:00', '2000-01-01 00:00:00')",
            [],
        )
        .unwrap();

        update_device_last_seen_internal(&conn, "aa:bb:cc:dd:ee:01").unwrap();

        let device = get_device_by_mac_internal(&conn, "aa:bb:cc:dd:ee:01")
            .unwrap()
            .unwrap();
        assert_ne!(
            device.last_seen_at, "2000-01-01 00:00:00",
            "last_seen_at should be updated away from the backdated value"
        );
        assert!(
            device.created_at == "2000-01-01 00:00:00",
            "created_at must NOT be touched"
        );
    }

    #[test]
    fn test_update_device_stats_accumulates_bytes() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        register_device_only(&conn, "aa:bb:cc:dd:ee:02", "Device").unwrap();

        update_device_stats_internal(&conn, "aa:bb:cc:dd:ee:02", 100, 200).unwrap();
        let device = get_device_by_mac_internal(&conn, "aa:bb:cc:dd:ee:02")
            .unwrap()
            .unwrap();
        assert_eq!(device.upload_bytes, 100);
        assert_eq!(device.download_bytes, 200);

        update_device_stats_internal(&conn, "aa:bb:cc:dd:ee:02", 50, 75).unwrap();
        let device = get_device_by_mac_internal(&conn, "aa:bb:cc:dd:ee:02")
            .unwrap()
            .unwrap();
        assert_eq!(
            device.upload_bytes, 150,
            "Second update should be additive (100 + 50)"
        );
        assert_eq!(
            device.download_bytes, 275,
            "Second update should be additive (200 + 75)"
        );
    }

    #[test]
    fn test_get_devices_internal_returns_all_devices() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        register_device_only(&conn, "aa:bb:cc:dd:ee:10", "One").unwrap();
        register_device_only(&conn, "aa:bb:cc:dd:ee:20", "Two").unwrap();
        register_device_only(&conn, "aa:bb:cc:dd:ee:30", "Three").unwrap();

        let devices = get_devices_internal(&conn).unwrap();
        assert_eq!(devices.len(), 3, "Should return all 3 registered devices");

        let names: Vec<&str> = devices.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"One"));
        assert!(names.contains(&"Two"));
        assert!(names.contains(&"Three"));
    }

    // ------------------------------------------------------------------
    // HTTP request CRUD tests
    // ------------------------------------------------------------------

    #[test]
    fn test_record_http_request_returns_id_and_persists() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let req_headers: Vec<(String, String)> =
            vec![("User-Agent".to_string(), "test".to_string())];
        let resp_headers: Vec<(String, String)> =
            vec![("Content-Type".to_string(), "application/json".to_string())];

        let id = record_http_request(
            &conn,
            "2026-06-04 00:00:00",
            "GET",
            "https",
            "example.com",
            "/",
            &req_headers,
            None,
            Some(200),
            &resp_headers,
            Some("{}"),
            Some(42),
            None,
            None,
            None,
        )
        .unwrap();

        assert!(id > 0, "Auto-increment id should be > 0");

        let recent = get_recent_requests(&conn, 10).unwrap();
        assert_eq!(recent.len(), 1, "Should have 1 request in the database");
        assert_eq!(recent[0].id, id);
        assert_eq!(recent[0].host, "example.com");
        assert_eq!(recent[0].method, "GET");
        assert_eq!(recent[0].status, Some(200));
    }

    #[test]
    fn test_get_recent_requests_orders_by_id_desc() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let empty_headers: Vec<(String, String)> = vec![];
        let id1 = record_http_request(
            &conn,
            "2026-06-04 00:00:00",
            "GET",
            "https",
            "a.com",
            "/",
            &empty_headers,
            None,
            Some(200),
            &empty_headers,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let id2 = record_http_request(
            &conn,
            "2026-06-04 00:00:01",
            "GET",
            "https",
            "b.com",
            "/",
            &empty_headers,
            None,
            Some(200),
            &empty_headers,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let id3 = record_http_request(
            &conn,
            "2026-06-04 00:00:02",
            "GET",
            "https",
            "c.com",
            "/",
            &empty_headers,
            None,
            Some(200),
            &empty_headers,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let recent = get_recent_requests(&conn, 10).unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].id, id3, "Most recently inserted should be first");
        assert_eq!(recent[1].id, id2);
        assert_eq!(recent[2].id, id1);
    }

    #[test]
    fn test_record_http_request_with_optional_fields_none() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let empty_headers: Vec<(String, String)> = vec![];

        // All optional fields as None
        let id = record_http_request(
            &conn,
            "2026-06-04 00:00:00",
            "POST",
            "https",
            "example.com",
            "/api",
            &empty_headers,
            None,
            None,
            &empty_headers,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert!(
            id > 0,
            "Should successfully insert even with all optional fields None"
        );

        let recent = get_recent_requests(&conn, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].status, None, "resp_status should be None");
        assert_eq!(recent[0].duration_ms, None, "duration_ms should be None");
        assert_eq!(recent[0].app_tag, None, "app_tag should be None");
    }

    // ------------------------------------------------------------------
    // WebSocket tests
    // ------------------------------------------------------------------

    #[test]
    fn test_record_ws_frame_persists() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let id = record_ws_frame(
            &conn,
            "req-1",
            // "outgoing" not "send" — ws_frames has a CHECK constraint limiting direction to 'incoming'|'outgoing'
            "outgoing",
            1,
            "hello",
            None,
            5,
            "2026-06-04 00:00:00",
        )
        .unwrap();

        assert!(id > 0, "Auto-increment id should be > 0");

        // Verify the row was actually written
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ws_frames WHERE request_id = ?1",
                ["req-1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "ws_frame row should be persisted");
    }

    #[test]
    fn test_get_ws_frames_returns_in_timestamp_order() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let req_id = "req-frames-1";
        record_ws_frame(
            &conn,
            req_id,
            "outgoing",
            0x01,
            "first",
            None,
            5,
            "2026-06-14 10:00:00",
        )
        .unwrap();
        record_ws_frame(
            &conn,
            req_id,
            "incoming",
            0x01,
            "second",
            None,
            6,
            "2026-06-14 10:00:01",
        )
        .unwrap();

        let frames = get_ws_frames(&conn, req_id).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].payload, "first");
        assert_eq!(frames[0].direction, "outgoing");
        assert_eq!(frames[0].opcode, 0x01);
        assert_eq!(frames[1].payload, "second");
        assert_eq!(frames[1].direction, "incoming");
    }

    #[test]
    fn test_get_ws_frames_empty_for_unknown_request() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let frames = get_ws_frames(&conn, "nonexistent").unwrap();
        assert!(frames.is_empty());
    }

    #[test]
    fn test_get_ws_frames_filters_by_request_id() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        // 2 frames for req-A (different timestamps, opcodes, payloads)
        record_ws_frame(
            &conn,
            "req-A",
            "outgoing",
            0x01,
            "alpha-text",
            None,
            10,
            "2026-06-15 10:00:00",
        )
        .unwrap();
        record_ws_frame(
            &conn,
            "req-A",
            "incoming",
            0x02,
            "alpha-binary",
            None,
            12,
            "2026-06-15 10:00:01",
        )
        .unwrap();

        // 2 frames for req-B
        record_ws_frame(
            &conn,
            "req-B",
            "outgoing",
            0x01,
            "beta-text",
            None,
            9,
            "2026-06-15 10:00:00",
        )
        .unwrap();
        record_ws_frame(
            &conn,
            "req-B",
            "incoming",
            0x09,
            "beta-ping",
            None,
            9,
            "2026-06-15 10:00:01",
        )
        .unwrap();

        // Query req-A: must return exactly 2 frames, all from req-A only
        let frames_a = get_ws_frames(&conn, "req-A").unwrap();
        assert_eq!(frames_a.len(), 2, "req-A should return exactly 2 frames");
        for f in &frames_a {
            // All returned frames must be from req-A, not req-B
            assert!(
                f.payload == "alpha-text" || f.payload == "alpha-binary",
                "req-A query should not return req-B payloads, got {}",
                f.payload
            );
        }
        // Confirm ordered by timestamp asc
        assert_eq!(frames_a[0].payload, "alpha-text");
        assert_eq!(frames_a[1].payload, "alpha-binary");

        // Query req-B: must return exactly 2 frames from req-B
        let frames_b = get_ws_frames(&conn, "req-B").unwrap();
        assert_eq!(frames_b.len(), 2, "req-B should return exactly 2 frames");
        for f in &frames_b {
            assert!(
                f.payload == "beta-text" || f.payload == "beta-ping",
                "req-B query should not return req-A payloads, got {}",
                f.payload
            );
        }
        assert_eq!(frames_b[0].payload, "beta-text");
        assert_eq!(frames_b[1].payload, "beta-ping");
    }

    #[test]
    fn test_ws_frame_truncated_flag() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let req_id = "req-trunc";

        // Frame A: small payload (size = 100) -> truncated = false
        record_ws_frame(
            &conn,
            req_id,
            "outgoing",
            0x01,
            "small",
            None,
            100,
            "2026-06-15 11:00:00",
        )
        .unwrap();

        // Frame B: payload larger than MAX_PAYLOAD_SIZE (64KB + 1) -> truncated = true
        let big_size = crate::ws_frames::MAX_PAYLOAD_SIZE + 1;
        record_ws_frame(
            &conn,
            req_id,
            "incoming",
            0x02,
            "big",
            None,
            big_size,
            "2026-06-15 11:00:01",
        )
        .unwrap();

        let frames = get_ws_frames(&conn, req_id).unwrap();
        assert_eq!(frames.len(), 2, "should retrieve both frames");

        // Frame A: size=100, truncated = false
        assert_eq!(frames[0].size, 100);
        assert!(
            !frames[0].truncated,
            "small frame (size=100) should NOT be truncated"
        );

        // Frame B: size=MAX_PAYLOAD_SIZE+1, truncated = true
        assert_eq!(frames[1].size, big_size);
        assert_eq!(frames[1].size, crate::ws_frames::MAX_PAYLOAD_SIZE + 1);
        assert!(
            frames[1].truncated,
            "frame exceeding MAX_PAYLOAD_SIZE must be truncated"
        );
    }

    #[test]
    fn test_mark_request_websocket_sets_flag() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let empty_headers: Vec<(String, String)> = vec![];
        let id = record_http_request(
            &conn,
            "2026-06-04 00:00:00",
            "GET",
            "wss",
            "ws.example.com",
            "/socket",
            &empty_headers,
            None,
            Some(101),
            &empty_headers,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        // Should not panic, should return Ok
        mark_request_websocket(&conn, &id.to_string()).unwrap();

        // Verify the flag was set
        let is_ws: i64 = conn
            .query_row(
                "SELECT is_websocket FROM http_requests WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            is_ws, 1,
            "is_websocket flag should be set to 1 after mark_request_websocket"
        );
    }

    /// `session_id` is the missing link that ties captured rows to a
    /// `SpecGenPanel` session. This test pins the round-trip:
    /// `record_http_request(..., Some("s1"))` writes "s1" into the
    /// `session_id` column, and a `None` record stays NULL.
    /// Without this, the spec-generation flow would silently keep
    /// returning empty record sets even after the column landed in
    /// migration v5.
    #[test]
    fn test_record_http_request_persists_session_id() {
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        let empty: Vec<(String, String)> = vec![];

        // With Some session id.
        let id_tagged = record_http_request(
            &conn,
            "2026-06-04 00:00:00",
            "GET",
            "https",
            "ex.com",
            "/a",
            &empty,
            None,
            Some(200),
            &empty,
            None,
            None,
            None,
            None,
            Some("session-7"),
        )
        .unwrap();

        // With None session id (current capture path before the user
        // selects anything in `SpecGenPanel`).
        let id_untagged = record_http_request(
            &conn,
            "2026-06-04 00:00:01",
            "GET",
            "https",
            "ex.com",
            "/b",
            &empty,
            None,
            Some(200),
            &empty,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        let tagged: Option<String> = conn
            .query_row(
                "SELECT session_id FROM http_requests WHERE id = ?1",
                [id_tagged],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tagged.as_deref(), Some("session-7"));

        let untagged: Option<String> = conn
            .query_row(
                "SELECT session_id FROM http_requests WHERE id = ?1",
                [id_untagged],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            untagged.is_none(),
            "untagged row should have NULL session_id"
        );
    }

    // ------------------------------------------------------------------
    // Pure helper tests
    // ------------------------------------------------------------------

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000), "2000 is divisible by 400 — leap year");
        assert!(
            !is_leap_year(1900),
            "1900 is divisible by 100 but not 400 — NOT a leap year"
        );
        assert!(
            is_leap_year(2024),
            "2024 is divisible by 4 and not 100 — leap year"
        );
        assert!(
            !is_leap_year(2023),
            "2023 is not divisible by 4 — NOT a leap year"
        );
        assert!(
            is_leap_year(2020),
            "2020 is divisible by 4 and not 100 — leap year"
        );
    }

    #[test]
    fn test_timestamp_now_for_ws_format() {
        let ts = timestamp_now_for_ws();
        // Format is "YYYY-MM-DD HH:MM:SS" — 19 chars
        assert_eq!(ts.len(), 19, "Timestamp should be 19 chars, got '{}'", ts);
        // Verify shape: digits and separators
        let bytes = ts.as_bytes();
        assert_eq!(bytes[4], b'-', "Year separator");
        assert_eq!(bytes[7], b'-', "Month separator");
        assert_eq!(bytes[10], b' ', "Date-time separator");
        assert_eq!(bytes[13], b':', "Hour separator");
        assert_eq!(bytes[16], b':', "Minute separator");
        // Verify all non-separator positions are digits
        for (i, &b) in bytes.iter().enumerate() {
            let is_sep = matches!(i, 4 | 7 | 10 | 13 | 16);
            assert!(
                is_sep || b.is_ascii_digit(),
                "Position {} should be digit or separator, got {:?}",
                i,
                b as char
            );
        }
    }
}
