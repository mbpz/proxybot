//! SQLite database module for ProxyBot.
//!
//! Manages the database at ~/.proxybot/proxybot.db with WAL mode enabled.
//! Tables: http_requests, dns_queries, devices, app_tags

use rusqlite::{Connection, Result as SqlResult};
use serde::Serialize;
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
    pub(crate) fn register_device_internal(&self, ip: &str, name: &str) -> Result<DeviceInfo, String> {
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
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".proxybot").join("proxybot.db")
    }

    pub(crate) fn init_schema(conn: &Connection) -> SqlResult<()> {
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

            CREATE INDEX IF NOT EXISTS idx_http_requests_host ON http_requests(host);
            CREATE INDEX IF NOT EXISTS idx_http_requests_timestamp ON http_requests(timestamp);
            CREATE INDEX IF NOT EXISTS idx_http_requests_device_id ON http_requests(device_id);
            CREATE INDEX IF NOT EXISTS idx_dns_queries_timestamp ON dns_queries(timestamp);
            CREATE INDEX IF NOT EXISTS idx_dns_queries_device_id ON dns_queries(device_id);
            "#,
        )?;
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

/// Get all registered devices.
#[tauri::command]
pub fn get_devices(state: State<'_, Arc<DbState>>) -> Result<Vec<DeviceInfo>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
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
pub fn update_device_last_seen(state: State<'_, Arc<DbState>>, mac_address: String) -> Result<(), String> {
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
fn chrono_lite_timestamp() -> String {
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
}
