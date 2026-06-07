use super::builder::*;
use super::types::*;
use crate::db::DbState;
use std::sync::{Arc, Mutex};

/// Build a `DbState` backed by an in-memory SQLite connection.
///
/// `DbState::new_in_memory` runs `init_schema` (with all migrations) on the
/// connection it creates, so no extra schema setup is required here.
fn make_in_memory_db() -> Arc<DbState> {
    Arc::new(DbState::new_in_memory(Mutex::new(())).unwrap())
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
