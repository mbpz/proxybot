use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::{parse_captured_timestamp, CapturedRequestOrder, CapturedRequestQuery, DbState};

use super::types::*;

const NODE_LIMIT: usize = 500;
const ERROR_RATE_THRESHOLD: f64 = 0.10;

pub fn build_topology_graph(
    db: &Arc<DbState>,
    filter: &TopologyFilter,
) -> Result<TopologyGraph, String> {
    let (start_ts, end_ts) = resolve_time_window(filter.time_window.as_ref());

    // 1. Query device rows
    let device_rows: Vec<(i64, String, String)> = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut statement = conn
            .prepare("SELECT id, name, last_seen_at FROM devices")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    };

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
            error_count: 0,
            error_rate: 0.0,
            last_seen: parse_timestamp(last_seen),
        })
        .collect();

    // 2. Aggregate canonical Captured Request records. The persistence Module
    // owns SQLite; this analysis Implementation keeps its existing projection.
    let records = db.captured_requests(&CapturedRequestQuery {
        order: CapturedRequestOrder::IdAscending,
        ..Default::default()
    })?;
    let mut grouped = std::collections::BTreeMap::<(i64, String, String), RawAggRow>::new();
    for record in records {
        let timestamp = parse_timestamp(&record.timestamp);
        if timestamp < start_ts || timestamp > end_ts {
            continue;
        }
        if filter
            .host_contains
            .as_ref()
            .is_some_and(|needle| !record.host.contains(needle))
        {
            continue;
        }
        if filter.device_ids.as_ref().is_some_and(|ids| {
            !ids.is_empty()
                && !record
                    .device_id
                    .is_some_and(|id| ids.contains(&id.to_string()))
        }) {
            continue;
        }
        let device_id = record.device_id.unwrap_or(0);
        let app_tag = record.app_tag.unwrap_or_else(|| "unknown".to_owned());
        let key = (device_id, app_tag.clone(), record.host.clone());
        let row = grouped.entry(key).or_insert_with(|| RawAggRow {
            device_id,
            app_tag,
            host: record.host.clone(),
            req_count: 0,
            total_bytes: 0,
            avg_latency: 0.0,
            err_count: 0,
            last_seen: 0,
        });
        let previous_duration = row.avg_latency * row.req_count as f64;
        row.req_count += 1;
        row.total_bytes += record.response_body.as_ref().map_or(0, Vec::len) as i64;
        row.avg_latency =
            (previous_duration + record.duration_ms.unwrap_or(0) as f64) / row.req_count as f64;
        row.err_count += i64::from(record.response_status.is_some_and(|status| status >= 400));
        row.last_seen = row.last_seen.max(timestamp);
    }
    let agg_rows: Vec<_> = grouped.into_values().collect();

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
        let app_entry = app_nodes
            .entry(app_id.clone())
            .or_insert_with(|| TopologyNode {
                id: app_id.clone(),
                kind: NodeKind::App,
                label: app_tag.clone(),
                app_tag: Some(app_tag.clone()),
                device_id: None,
                request_count: 0,
                total_bytes: 0,
                avg_latency_ms: 0.0,
                error_count: 0,
                error_rate: 0.0,
                last_seen: 0,
            });
        app_entry.request_count += row.req_count as u64;
        app_entry.total_bytes += row.total_bytes as u64;
        app_entry.error_count += row.err_count as u64;
        app_entry.last_seen = app_entry.last_seen.max(row.last_seen);

        // Aggregate host node
        let host_entry = host_nodes
            .entry(host_id.clone())
            .or_insert_with(|| TopologyNode {
                id: host_id.clone(),
                kind: NodeKind::Host,
                label: host.clone(),
                app_tag: None,
                device_id: None,
                request_count: 0,
                total_bytes: 0,
                avg_latency_ms: 0.0,
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
    for n in app_nodes
        .values_mut()
        .chain(host_nodes.iter_mut().map(|(_, v)| v))
    {
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
        device_count: all_nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Device)
            .count() as u32,
        app_count: all_nodes.iter().filter(|n| n.kind == NodeKind::App).count() as u32,
        host_count: all_nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Host)
            .count() as u32,
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
    parse_captured_timestamp(ts)
        .map(|timestamp| timestamp.timestamp_millis())
        .unwrap_or(0)
}

/// Escape SQL `LIKE` wildcards (`%` and `_`) plus the escape character itself
/// so user-supplied `host_contains` text matches literally. Pairs with the
/// `ESCAPE '\'` clause added to the `LIKE` expression.
#[cfg(test)]
pub(crate) fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

pub fn get_topology_node_detail(
    db: &Arc<DbState>,
    node_id: &str,
    filter: &TopologyFilter,
) -> Result<NodeDetail, String> {
    let (start_ts, end_ts) = resolve_time_window(filter.time_window.as_ref());

    // Parse node id of the form "kind:key" (e.g. "host:api.weixin.qq.com").
    let (kind_str, key) = node_id.split_once(':').ok_or("invalid node id")?;
    let kind = match kind_str {
        "device" => NodeKind::Device,
        "app" => NodeKind::App,
        "host" => NodeKind::Host,
        _ => return Err(format!("unknown node kind: {}", kind_str)),
    };

    let device_id = if kind == NodeKind::Device {
        Some(
            key.parse::<i64>()
                .map_err(|_| format!("invalid device id: {}", key))?,
        )
    } else {
        None
    };
    let query = CapturedRequestQuery {
        device_id,
        host: (kind == NodeKind::Host).then(|| key.to_owned()),
        app_tag: (kind == NodeKind::App).then(|| key.to_owned()),
        order: CapturedRequestOrder::IdDescending,
        ..Default::default()
    };
    let matching: Vec<_> = db
        .captured_requests(&query)?
        .into_iter()
        .filter(|record| {
            let timestamp = parse_timestamp(&record.timestamp);
            timestamp >= start_ts && timestamp <= end_ts
        })
        .collect();
    let requests: Vec<RecentRequest> = matching
        .iter()
        .take(20)
        .map(|record| RecentRequest {
            id: record.id.to_string(),
            method: record.method.clone(),
            host: record.host.clone(),
            path: record.path.clone(),
            status: record.response_status,
            duration_ms: record.duration_ms.unwrap_or(0).max(0) as u64,
            timestamp: parse_timestamp(&record.timestamp),
        })
        .collect();
    let mut counts = [0_u64; 4];
    for record in &matching {
        match record.response_status.unwrap_or(0) {
            200..=299 => counts[0] += 1,
            300..=399 => counts[1] += 1,
            400..=499 => counts[2] += 1,
            500.. => counts[3] += 1,
            _ => {}
        }
    }

    let status_breakdown = vec![
        StatusCount {
            status_class: "2xx".into(),
            count: counts[0],
        },
        StatusCount {
            status_class: "3xx".into(),
            count: counts[1],
        },
        StatusCount {
            status_class: "4xx".into(),
            count: counts[2],
        },
        StatusCount {
            status_class: "5xx".into(),
            count: counts[3],
        },
    ];

    // The recent_requests Vec above is capped at 20; this COUNT(*) gives
    // the true full-window total for the drawer.
    let full_count = matching.len() as i64;

    // The drawer shows a representative node; request_count is the full-window
    // total (from the COUNT above), not the 20-row recent_requests sample.
    let request_count = full_count as u64;
    let error_count = status_breakdown[2].count + status_breakdown[3].count;
    let error_rate = if request_count > 0 {
        error_count as f64 / request_count as f64
    } else {
        0.0
    };
    let node = TopologyNode {
        id: node_id.to_string(),
        kind: kind.clone(),
        label: key.to_string(),
        app_tag: if kind == NodeKind::App {
            Some(key.to_string())
        } else {
            None
        },
        device_id: if kind == NodeKind::Device {
            Some(key.to_string())
        } else {
            None
        },
        request_count,
        total_bytes: 0,
        avg_latency_ms: 0.0,
        error_count,
        error_rate,
        last_seen: now_unix_ms(),
    };

    Ok(NodeDetail {
        node,
        recent_requests: requests,
        status_breakdown,
    })
}
