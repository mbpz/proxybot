use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayTarget {
    pub id: String,
    pub name: String,
    pub url: String,
    pub status: ReplayStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplayStatus {
    Idle,
    Running,
    Completed,
    Error,
}