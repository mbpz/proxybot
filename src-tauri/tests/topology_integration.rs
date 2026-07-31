//! Integration tests for the topology public API.
//!
//! These tests live in the `tests/` directory (not `src/topology/tests.rs`) and
//! exercise the topology module through the public `proxybot_lib` crate
//! surface — i.e. exactly how the Tauri commands call it. The Tauri commands
//! themselves are thin wrappers over `topology::builder::*`, so testing the
//! underlying functions with an in-memory `DbState` is the right integration
//! boundary: it verifies the data path end-to-end without needing a full Tauri
//! app harness.

use std::sync::{Arc, Mutex};

use proxybot_lib::{db::DbState, topology};

/// Build a `DbState` backed by an in-memory SQLite connection.
///
/// `DbState::new_in_memory` runs `init_schema` (with all migrations) on the
/// connection it creates, so no extra schema setup is required here. This is
/// the same pattern used by `src/topology/tests.rs`.
fn make_in_memory_db() -> Arc<DbState> {
    Arc::new(DbState::new_in_memory(Mutex::new(())).unwrap())
}

fn seed_device(db: &DbState, mac: &str, name: &str) -> i64 {
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO devices (mac_address, name, created_at, last_seen_at) \
         VALUES (?1, ?2, '2026-01-01 00:00:00', '2026-01-01 00:00:00')",
        rusqlite::params![mac, name],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn seed_request(
    db: &DbState,
    device_id: i64,
    host: &str,
    app_tag: &str,
    status: u16,
    dur_ms: i64,
    ts: &str,
) {
    let conn = db.conn.lock().unwrap();
    conn.execute(
        "INSERT INTO http_requests (timestamp, method, scheme, host, path, resp_status, duration_ms, device_id, app_tag) \
         VALUES (?1, 'GET', 'https', ?2, '/', ?3, ?4, ?5, ?6)",
        rusqlite::params![ts, host, status, dur_ms, device_id, app_tag],
    )
    .unwrap();
}

#[test]
fn topology_graph_contains_seeded_request() {
    let db = make_in_memory_db();
    let dev = seed_device(&db, "mac-integration", "TestPhone");
    seed_request(
        &db,
        dev,
        "test.com",
        "wechat",
        200,
        100,
        "2026-01-01 00:00:00",
    );

    let graph =
        topology::builder::build_topology_graph(&db, &topology::TopologyFilter::default()).unwrap();

    assert!(
        graph.meta.total_requests > 0,
        "expected total_requests > 0 after seeding one request"
    );
    assert!(!graph.nodes.is_empty(), "expected at least one node");
    assert!(!graph.edges.is_empty(), "expected at least one edge");
}

#[test]
fn topology_filter_narrows_results() {
    let db = make_in_memory_db();
    let dev = seed_device(&db, "mac-integration", "TestPhone");
    // Two hosts across two apps, to give the filter something to narrow down.
    seed_request(
        &db,
        dev,
        "keep.test.com",
        "wechat",
        200,
        50,
        "2026-01-01 00:00:00",
    );
    seed_request(
        &db,
        dev,
        "drop.other.com",
        "douyin",
        200,
        50,
        "2026-01-01 00:00:01",
    );

    let full =
        topology::builder::build_topology_graph(&db, &topology::TopologyFilter::default()).unwrap();
    let filtered = topology::builder::build_topology_graph(
        &db,
        &topology::TopologyFilter {
            host_contains: Some("keep".to_string()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        filtered.nodes.len() <= full.nodes.len(),
        "filtered graph must not be larger than the unfiltered graph"
    );
    // Sanity: the unfiltered graph contains both hosts, the filtered one
    // contains at most the keep.test.com host. Use a more specific check than
    // "filtered is smaller" so a regression in the filter is caught loudly.
    let keep_hosts = filtered
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, topology::NodeKind::Host))
        .count();
    assert!(
        keep_hosts <= 1,
        "host_contains:\"keep\" should match at most one host, got {}",
        keep_hosts
    );
}
