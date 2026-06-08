use super::builder::*;
use super::types::*;
use crate::db::DbState;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Build a `DbState` backed by an in-memory SQLite connection.
///
/// `DbState::new_in_memory` runs `init_schema` (with all migrations) on the
/// connection it creates, so no extra schema setup is required here.
fn make_in_memory_db() -> Arc<DbState> {
    Arc::new(DbState::new_in_memory(Mutex::new(())).unwrap())
}

fn seed_request(
    conn: &Connection,
    device_id: i64,
    host: &str,
    app_tag: &str,
    status: u16,
    dur_ms: i64,
    ts: &str,
) {
    conn.execute(
        "INSERT INTO http_requests (timestamp, method, scheme, host, path, resp_status, duration_ms, device_id, app_tag) VALUES (?1, 'GET', 'https', ?2, '/', ?3, ?4, ?5, ?6)",
        rusqlite::params![ts, host, status, dur_ms, device_id, app_tag],
    )
    .unwrap();
}

fn seed_device(conn: &Connection, name: &str) -> i64 {
    conn.execute(
        "INSERT INTO devices (mac_address, name, created_at, last_seen_at) VALUES (?1, ?2, '2026-01-01 00:00:00', '2026-01-01 00:00:00')",
        rusqlite::params![format!("mac-{}", name), name],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[test]
fn test_aggregate_empty_db() {
    let db = make_in_memory_db();
    let filter = TopologyFilter::default();
    let graph = build_topology_graph(&db, &filter).unwrap();
    assert_eq!(graph.nodes.len(), 0);
    assert_eq!(graph.edges.len(), 0);
    assert_eq!(graph.meta.total_requests, 0);
}

#[test]
fn test_aggregate_single_device_single_host() {
    let db = make_in_memory_db();
    let conn = db.conn.lock().unwrap();
    let dev = seed_device(&conn, "iPhone");
    seed_request(
        &conn,
        dev,
        "api.weixin.qq.com",
        "wechat",
        200,
        100,
        "2026-01-01 00:00:00",
    );
    seed_request(
        &conn,
        dev,
        "api.weixin.qq.com",
        "wechat",
        200,
        200,
        "2026-01-01 00:00:01",
    );
    drop(conn);

    let graph = build_topology_graph(&db, &TopologyFilter::default()).unwrap();
    assert_eq!(
        graph.nodes.iter().filter(|n| n.kind == NodeKind::Device).count(),
        1
    );
    assert_eq!(
        graph.nodes.iter().filter(|n| n.kind == NodeKind::Host).count(),
        1
    );
    assert_eq!(
        graph.nodes.iter().filter(|n| n.kind == NodeKind::App).count(),
        1
    );
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].request_count, 2);
    assert_eq!(graph.meta.total_requests, 2);
}

#[test]
fn test_aggregate_metrics_accuracy() {
    let db = make_in_memory_db();
    let conn = db.conn.lock().unwrap();
    let dev = seed_device(&conn, "Phone");
    seed_request(
        &conn,
        dev,
        "host.com",
        "wechat",
        200,
        100,
        "2026-01-01 00:00:00",
    );
    seed_request(
        &conn,
        dev,
        "host.com",
        "wechat",
        500,
        300,
        "2026-01-01 00:00:01",
    );
    drop(conn);

    let graph = build_topology_graph(&db, &TopologyFilter::default()).unwrap();
    let host = graph
        .nodes
        .iter()
        .find(|n| n.kind == NodeKind::Host)
        .unwrap();
    assert_eq!(host.request_count, 2);
    assert_eq!(host.error_count, 1);
    assert!((host.error_rate - 0.5).abs() < 0.001);
}

#[test]
fn test_aggregate_filter_by_app_tag() {
    let db = make_in_memory_db();
    let conn = db.conn.lock().unwrap();
    let dev = seed_device(&conn, "Phone");
    seed_request(
        &conn,
        dev,
        "wx.qq.com",
        "wechat",
        200,
        50,
        "2026-01-01 00:00:00",
    );
    seed_request(
        &conn,
        dev,
        "dy.qq.com",
        "douyin",
        200,
        50,
        "2026-01-01 00:00:01",
    );
    drop(conn);

    let filter = TopologyFilter {
        app_tags: Some(vec!["wechat".to_string()]),
        ..Default::default()
    };
    let graph = build_topology_graph(&db, &filter).unwrap();
    let app_count = graph.nodes.iter().filter(|n| n.kind == NodeKind::App).count();
    assert_eq!(app_count, 1);
}

#[test]
fn test_aggregate_filter_by_host_contains() {
    let db = make_in_memory_db();
    let conn = db.conn.lock().unwrap();
    let dev = seed_device(&conn, "Phone");
    seed_request(
        &conn,
        dev,
        "api.weixin.qq.com",
        "wechat",
        200,
        50,
        "2026-01-01 00:00:00",
    );
    seed_request(
        &conn,
        dev,
        "api.douyin.com",
        "douyin",
        200,
        50,
        "2026-01-01 00:00:01",
    );
    drop(conn);

    let filter = TopologyFilter {
        host_contains: Some("weixin".to_string()),
        ..Default::default()
    };
    let graph = build_topology_graph(&db, &filter).unwrap();
    let host_count = graph.nodes.iter().filter(|n| n.kind == NodeKind::Host).count();
    assert_eq!(host_count, 1);
}

#[test]
fn test_anomalous_edge_detection() {
    let db = make_in_memory_db();
    let conn = db.conn.lock().unwrap();
    let dev = seed_device(&conn, "Phone");
    // 9 OK + 2 errors = 18% error rate > 10% threshold
    for _ in 0..9 {
        seed_request(
            &conn,
            dev,
            "flaky.com",
            "wechat",
            200,
            50,
            "2026-01-01 00:00:00",
        );
    }
    seed_request(
        &conn,
        dev,
        "flaky.com",
        "wechat",
        500,
        50,
        "2026-01-01 00:00:01",
    );
    seed_request(
        &conn,
        dev,
        "flaky.com",
        "wechat",
        500,
        50,
        "2026-01-01 00:00:02",
    );
    drop(conn);

    let graph = build_topology_graph(&db, &TopologyFilter::default()).unwrap();
    let edge = &graph.edges[0];
    assert!(edge.is_anomalous);
    assert!(edge.error_rate > 0.10);
}

#[test]
fn test_node_limit_500() {
    let db = make_in_memory_db();
    let conn = db.conn.lock().unwrap();
    let dev = seed_device(&conn, "Phone");
    // Insert 600 distinct host rows
    for i in 0..600 {
        seed_request(
            &conn,
            dev,
            &format!("host{}.com", i),
            "wechat",
            200,
            50,
            "2026-01-01 00:00:00",
        );
    }
    drop(conn);

    let graph = build_topology_graph(&db, &TopologyFilter::default()).unwrap();
    assert!(graph.nodes.len() <= 500);
}
