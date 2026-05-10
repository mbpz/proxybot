use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkProfile {
    pub name: String,
    pub latency_ms: u64,
    pub bandwidth_kbps: u64,  // 0 = unlimited
    pub packet_loss_pct: u8,
}
