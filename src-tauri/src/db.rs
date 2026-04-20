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
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                mac_address  TEXT UNIQUE NOT NULL,
                name         TEXT NOT NULL,
                created_at   TEXT NOT NULL,
                last_seen_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS app_tags (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                app_name    TEXT UNIQUE NOT NULL,
                domain_rule TEXT NOT NULL
            );

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
