use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::DbState;

use super::types::*;

const NODE_LIMIT: usize = 500;
const ERROR_RATE_THRESHOLD: f64 = 0.10;

pub fn build_topology_graph(
    db: &Arc<DbState>,
    filter: &TopologyFilter,
) -> Result<TopologyGraph, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (start_ts, end_ts) = resolve_time_window(filter.time_window.as_ref());

    // 1. Query device rows
    let mut device_stmt = conn
        .prepare("SELECT id, name, last_seen_at FROM devices")
        .map_err(|e| e.to_string())?;
    let device_rows: Vec<(i64, String, String)> = device_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(device_stmt);

    // Apply device filter
    let device_filter = filter.device_ids.as_ref();
    let mut device_nodes: Vec<TopologyNode> = device_rows
        .iter()
        .filter(|(id, _, _)| match device_filter {
            Some(ids) => ids.contains(&id.to_string()),
            None => true,
        })
        .map(|(id, name, last_seen)| TopologyNode {
            id: format!("device:{}", id),
            kind: NodeKind::Device,
            label: name.clone(),
            app_tag: None,
            device_id: Some(id.to_string()),
            request_count: 0,
            total_bytes: 0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            error_count: 0,
            error_rate: 0.0,
            last_seen: parse_timestamp(last_seen),
        })
        .collect();

    // 2. Aggregate request rows
    let start_ts_str = format_ts_for_sql(start_ts);
    let end_ts_str = format_ts_for_sql(end_ts);
    let mut sql = String::from(
        "SELECT device_id, COALESCE(app_tag, 'unknown') AS app_tag, host,
                COUNT(*) AS req_count, COALESCE(SUM(COALESCE(LENGTH(resp_body),0)),0) AS total_bytes,
                AVG(COALESCE(duration_ms,0)) AS avg_lat,
                SUM(CASE WHEN resp_status >= 400 THEN 1 ELSE 0 END) AS err_count,
                MAX(timestamp) AS max_ts
         FROM http_requests
         WHERE timestamp >= ?1 AND timestamp <= ?2",
    );
    let mut params_vec: Vec<String> = vec![start_ts_str, end_ts_str];
    let mut next_idx: usize = 3;
    if let Some(needle) = &filter.host_contains {
        sql.push_str(&format!(" AND host LIKE ?{}", next_idx));
        params_vec.push(format!("%{}%", needle));
        next_idx += 1;
    }
    if let Some(ids) = &filter.device_ids {
        if !ids.is_empty() {
            let placeholders: Vec<String> = (0..ids.len())
                .map(|i| format!("?{}", next_idx + i))
                .collect();
            sql.push_str(&format!(" AND device_id IN ({})", placeholders.join(",")));
            params_vec.extend(ids.iter().cloned());
        }
    }
    sql.push_str(" GROUP BY device_id, app_tag, host");

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let agg_rows: Vec<RawAggRow> = stmt
        .query_map(rusqlite::params_from_iter(params_vec.iter()), map_agg_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    // 3. Build aggregated (device, app_tag, host) nodes + edges
    let mut app_nodes: std::collections::BTreeMap<String, TopologyNode> =
        std::collections::BTreeMap::new();
    let mut host_nodes: std::collections::BTreeMap<String, TopologyNode> =
        std::collections::BTreeMap::new();
    let mut edges: Vec<TopologyEdge> = Vec::new();
    let mut total_requests: u64 = 0;
    let mut total_bytes: u64 = 0;

    for row in &agg_rows {
        let app_tag = row.app_tag.clone();
        if filter
            .app_tags
            .as_ref()
            .map(|tags| !tags.contains(&app_tag))
            .unwrap_or(false)
        {
            continue;
        }

        total_requests += row.req_count as u64;
        total_bytes += row.total_bytes as u64;

        let host = row.host.clone();
        let app_id = format!("app:{}", app_tag);
        let host_id = format!("host:{}", host);

        // Aggregate app node
        let app_entry = app_nodes.entry(app_id.clone()).or_insert_with(|| TopologyNode {
            id: app_id.clone(),
            kind: NodeKind::App,
            label: app_tag.clone(),
            app_tag: Some(app_tag.clone()),
            device_id: None,
            request_count: 0,
            total_bytes: 0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            error_count: 0,
            error_rate: 0.0,
            last_seen: 0,
        });
        app_entry.request_count += row.req_count as u64;
        app_entry.total_bytes += row.total_bytes as u64;
        app_entry.error_count += row.err_count as u64;
        app_entry.last_seen = app_entry.last_seen.max(row.last_seen);

        // Aggregate host node
        let host_entry = host_nodes.entry(host_id.clone()).or_insert_with(|| TopologyNode {
            id: host_id.clone(),
            kind: NodeKind::Host,
            label: host.clone(),
            app_tag: None,
            device_id: None,
            request_count: 0,
            total_bytes: 0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            error_count: 0,
            error_rate: 0.0,
            last_seen: 0,
        });
        host_entry.request_count += row.req_count as u64;
        host_entry.total_bytes += row.total_bytes as u64;
        host_entry.error_count += row.err_count as u64;
        host_entry.last_seen = host_entry.last_seen.max(row.last_seen);

        // Build edge: device -> host
        let device_id_str = format!("device:{}", row.device_id);
        let error_rate = if row.req_count > 0 {
            row.err_count as f64 / row.req_count as f64
        } else {
            0.0
        };
        edges.push(TopologyEdge {
            id: format!("{}->{}", device_id_str, host_id),
            from: device_id_str,
            to: host_id,
            request_count: row.req_count as u64,
            total_bytes: row.total_bytes as u64,
            avg_latency_ms: row.avg_latency,
            error_rate,
            is_anomalous: error_rate > ERROR_RATE_THRESHOLD,
        });
    }

    // Compute per-node error_rate averages
    for n in app_nodes.values_mut().chain(host_nodes.iter_mut().map(|(_, v)| v)) {
        if n.request_count > 0 {
            n.error_rate = n.error_count as f64 / n.request_count as f64;
        }
    }

    // Update device node last_seen
    for dev_node in device_nodes.iter_mut() {
        if let Some(dev_id_str) = &dev_node.device_id {
            if let Ok(dev_id) = dev_id_str.parse::<i64>() {
                let max_seen = agg_rows
                    .iter()
                    .filter(|r| r.device_id == dev_id)
                    .map(|r| r.last_seen)
                    .max()
                    .unwrap_or(0);
                dev_node.last_seen = dev_node.last_seen.max(max_seen);
            }
        }
    }

    // Assemble final node list with limit
    let mut all_nodes: Vec<TopologyNode> = Vec::new();
    all_nodes.append(&mut device_nodes);
    all_nodes.extend(app_nodes.into_values());
    all_nodes.extend(host_nodes.into_values());
    all_nodes.truncate(NODE_LIMIT);

    let meta = TopologyMeta {
        total_requests,
        total_bytes,
        device_count: all_nodes.iter().filter(|n| n.kind == NodeKind::Device).count() as u32,
        app_count: all_nodes.iter().filter(|n| n.kind == NodeKind::App).count() as u32,
        host_count: all_nodes.iter().filter(|n| n.kind == NodeKind::Host).count() as u32,
        time_range: (start_ts, end_ts),
        built_at: now_unix_ms(),
    };

    Ok(TopologyGraph {
        nodes: all_nodes,
        edges,
        meta,
    })
}

struct RawAggRow {
    device_id: i64,
    app_tag: String,
    host: String,
    req_count: i64,
    total_bytes: i64,
    avg_latency: f64,
    err_count: i64,
    last_seen: i64,
}

fn map_agg_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawAggRow> {
    let max_ts: String = row.get(7)?;
    Ok(RawAggRow {
        device_id: row.get(0)?,
        app_tag: row.get(1)?,
        host: row.get(2)?,
        req_count: row.get(3)?,
        total_bytes: row.get(4)?,
        avg_latency: row.get(5)?,
        err_count: row.get(6)?,
        last_seen: parse_timestamp(&max_ts),
    })
}

fn resolve_time_window(window: Option<&TimeWindow>) -> (i64, i64) {
    let now = now_unix_ms();
    match window {
        Some(TimeWindow::Last5Min) => (now - 5 * 60 * 1000, now),
        Some(TimeWindow::Last1Hour) => (now - 60 * 60 * 1000, now),
        Some(TimeWindow::Session) => (0, now),
        Some(TimeWindow::Custom { start, end }) => (*start, *end),
        None => (0, now),
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn parse_timestamp(ts: &str) -> i64 {
    if let Ok(f) = ts.parse::<f64>() {
        return f as i64;
    }
    chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.and_utc().timestamp_millis())
        .unwrap_or(0)
}

/// Format a unix-millis timestamp as the `"YYYY-MM-DD HH:MM:SS"` literal
/// that `http_requests.timestamp` is stored in (see db.rs schema).
/// Empty string for invalid inputs means the SQL `>=` / `<=` will be
/// skipped by SQLite's `TEXT` collation only if a literal empty is
/// compared — but since we only call this on values produced by
/// `resolve_time_window` (which are i64 unix-millis), the function will
/// always succeed.
fn format_ts_for_sql(ts: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ts)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}
