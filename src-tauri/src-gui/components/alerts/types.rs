use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Sev1,  // Critical
    Sev2,  // Warning
    Sev3,  // Info
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub source: String,
    pub description: String,
    pub severity: Severity,
    pub timestamp: i64,
    pub acknowledged: bool,
}