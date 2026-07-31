//! Integration tests for database operations.
//! Tests the DbState schema and query functions.

use proxybot_lib::db::DbState;
use std::sync::{Arc, Mutex};

fn make_db() -> Arc<DbState> {
    let guard = Mutex::new(());
    Arc::new(DbState::new_in_memory(guard).expect("Failed to create in-memory DB"))
}

fn insert_device(conn: &rusqlite::Connection, mac: &str, name: &str) -> i64 {
    conn.execute(
        "INSERT INTO devices (mac_address, name, created_at, last_seen_at, upload_bytes, download_bytes)
         VALUES (?1, ?2, datetime('now'), datetime('now'), 0, 0)",
        rusqlite::params![mac, name],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[test]
fn test_in_memory_db_creation() {
    let db = make_db();
    let conn = db.conn.lock().unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_schema_tables_exist() {
    let db = make_db();
    let conn = db.conn.lock().unwrap();

    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(tables.contains(&"http_requests".to_string()));
    assert!(tables.contains(&"dns_queries".to_string()));
    assert!(tables.contains(&"devices".to_string()));
    assert!(tables.contains(&"app_tags".to_string()));
    assert!(tables.contains(&"schema_version".to_string()));
}

#[test]
fn test_insert_and_query_device() {
    let db = make_db();
    let conn = db.conn.lock().unwrap();

    insert_device(&conn, "AA:BB:CC:DD:EE:FF", "Test Phone");

    let device = conn
        .query_row(
            "SELECT id, mac_address, name, upload_bytes, download_bytes FROM devices WHERE mac_address = ?1",
            rusqlite::params!["AA:BB:CC:DD:EE:FF"],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .unwrap();

    assert!(device.0 > 0);
    assert_eq!(device.1, "AA:BB:CC:DD:EE:FF");
    assert_eq!(device.2, "Test Phone");
    assert_eq!(device.3, 0);
    assert_eq!(device.4, 0);
}

#[test]
fn test_insert_http_request() {
    let db = make_db();
    let conn = db.conn.lock().unwrap();

    let device_id = insert_device(&conn, "192.168.1.50", "Phone");

    conn.execute(
        "INSERT INTO http_requests (timestamp, method, scheme, host, path, req_headers, resp_status, resp_headers, duration_ms, device_id, app_tag)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            "2024-01-01T00:00:00Z",
            "GET",
            "https",
            "api.example.com",
            "/users",
            "{}",
            200,
            "{\"content-type\": \"application/json\"}",
            42,
            device_id,
            "WeChat"
        ],
    )
    .unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);

    // Query back
    let (host, method, status): (String, String, i64) = conn
        .query_row(
            "SELECT host, method, resp_status FROM http_requests WHERE device_id = ?1",
            rusqlite::params![device_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();

    assert_eq!(host, "api.example.com");
    assert_eq!(method, "GET");
    assert_eq!(status, 200);
}

#[test]
fn test_insert_dns_query() {
    let db = make_db();
    let conn = db.conn.lock().unwrap();

    conn.execute(
        "INSERT INTO dns_queries (timestamp, query_name, query_type, response_ips, app_tag)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            "2024-01-01T00:00:00Z",
            "api.weixin.qq.com",
            1, // A record
            "[\"101.89.47.100\"]",
            "WeChat"
        ],
    )
    .unwrap();

    let (domain, app_tag): (String, Option<String>) = conn
        .query_row(
            "SELECT query_name, app_tag FROM dns_queries LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(domain, "api.weixin.qq.com");
    assert_eq!(app_tag, Some("WeChat".to_string()));
}

#[test]
fn test_get_recent_requests_empty() {
    let db = make_db();
    let conn = db.conn.lock().unwrap();
    let requests = proxybot_lib::db::get_recent_requests(&conn, 10).unwrap();
    assert!(requests.is_empty());
}

#[test]
fn test_get_recent_requests_with_data() {
    let db = make_db();
    let conn = db.conn.lock().unwrap();

    let device_id = insert_device(&conn, "192.168.1.50", "Phone");

    conn.execute(
        "INSERT INTO http_requests (timestamp, method, scheme, host, path, req_headers, resp_status, resp_headers, duration_ms, device_id, app_tag)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            "2024-01-01T00:00:00Z",
            "POST",
            "https",
            "api.douyin.com",
            "/feed",
            "{}",
            200,
            "{}",
            156,
            device_id,
            "Douyin"
        ],
    )
    .unwrap();

    let requests = proxybot_lib::db::get_recent_requests(&conn, 10).unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].host, "api.douyin.com");
    assert_eq!(requests[0].method, "POST");
}

#[test]
fn test_multiple_devices() {
    let db = make_db();
    let conn = db.conn.lock().unwrap();

    insert_device(&conn, "AA:BB:CC:DD:EE:01", "Phone 1");
    insert_device(&conn, "AA:BB:CC:DD:EE:02", "Phone 2");
    insert_device(&conn, "AA:BB:CC:DD:EE:03", "Laptop");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 3);
}

#[test]
fn test_db_stats_queries() {
    let db = make_db();
    let conn = db.conn.lock().unwrap();

    let request_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM http_requests", [], |row| row.get(0))
        .unwrap();
    let dns_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM dns_queries", [], |row| row.get(0))
        .unwrap();

    assert_eq!(request_count, 0);
    assert_eq!(dns_count, 0);
}

#[test]
fn test_journal_mode_set() {
    let db = make_db();
    let conn = db.conn.lock().unwrap();
    // In-memory DBs use "memory" journal mode; file DBs use "wal"
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert!(
        journal_mode == "wal" || journal_mode == "memory",
        "Expected wal or memory, got: {}",
        journal_mode
    );
}
