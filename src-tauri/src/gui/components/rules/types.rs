use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub filter: String,
    pub action: RuleAction,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RuleAction {
    Direct,
    Proxy,
    Reject,
    MapRemote { host: String, port: u16 },
    MapLocal { path: String },
}