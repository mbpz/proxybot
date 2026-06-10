use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyGraph {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    pub meta: TopologyMeta,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyNode {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    pub app_tag: Option<String>,
    pub device_id: Option<String>,
    pub request_count: u64,
    pub total_bytes: u64,
    pub avg_latency_ms: f64,
    pub error_count: u64,
    pub error_rate: f64,
    pub last_seen: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub request_count: u64,
    pub total_bytes: u64,
    pub avg_latency_ms: f64,
    pub error_rate: f64,
    pub is_anomalous: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TopologyMeta {
    pub total_requests: u64,
    pub total_bytes: u64,
    pub device_count: u32,
    pub app_count: u32,
    pub host_count: u32,
    pub time_range: (i64, i64),
    pub built_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Device,
    App,
    Host,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TopologyFilter {
    pub device_ids: Option<Vec<String>>,
    pub app_tags: Option<Vec<String>>,
    pub host_contains: Option<String>,
    pub time_window: Option<TimeWindow>,
    pub sync_global: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TimeWindow {
    Last5Min,
    Last1Hour,
    Session,
    Custom { start: i64, end: i64 },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDetail {
    pub node: TopologyNode,
    pub recent_requests: Vec<RecentRequest>,
    pub status_breakdown: Vec<StatusCount>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecentRequest {
    pub id: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: Option<u16>,
    pub duration_ms: u64,
    pub timestamp: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StatusCount {
    pub status_class: String,
    pub count: u64,
}
