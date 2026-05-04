use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub upstream: UpstreamType,
    pub upstream_host: String,
    pub blocklist_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpstreamType {
    PlainUdp,
    DoH,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsQuery {
    pub name: String,
    pub timestamp: i64,
    pub latency_ms: u64,
    pub blocked: bool,
    pub response: Option<String>,
}