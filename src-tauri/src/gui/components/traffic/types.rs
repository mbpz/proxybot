use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub app: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficStats {
    pub total_requests: u64,
    pub bytes_up: u64,
    pub bytes_down: u64,
}