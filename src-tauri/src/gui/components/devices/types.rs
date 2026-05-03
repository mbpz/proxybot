use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub mac: String,
    pub last_seen: i64,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub app: Option<String>,
    pub rule_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStats {
    pub total_devices: usize,
    pub devices: Vec<Device>,
}