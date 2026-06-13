//! Frida session types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHandle {
    pub session_id: String,
    pub device_id: String,
    pub pid: u32,
    pub process_name: String,
    pub attached_at: u64,
}
