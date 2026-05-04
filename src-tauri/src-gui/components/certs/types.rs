use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertInfo {
    pub common_name: String,
    pub expires_at: i64,
    pub issued_at: i64,
    pub serial_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertStats {
    pub total_certs: usize,
    pub certs: Vec<CertInfo>,
}