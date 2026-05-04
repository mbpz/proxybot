use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenType {
    MockApi,
    Scaffold,
    Docker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenRequest {
    pub gen_type: GenType,
    pub options: serde_json::Value,
}