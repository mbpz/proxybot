//! Serialization types for workspace files.
//!
//! The .proxybot format is a gzip-compressed tar archive containing:
//! - workspace.json — metadata
//! - requests.db — SQLite database (copy)
//! - rules.yaml — exported rules

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Rule action types for serialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum RuleAction {
    Direct,
    Proxy,
    Reject,
    #[serde(rename = "MAPREMOTE")]
    MapRemote(String),
    #[serde(rename = "MAPLOCAL")]
    MapLocal(String),
}

/// A serialized rule with ID for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub action: RuleAction,
    pub pattern: String,
    pub enabled: bool,
}

impl Rule {
    pub fn new(id: String, action: RuleAction, pattern: String, enabled: bool) -> Self {
        Self {
            id,
            action,
            pattern,
            enabled,
        }
    }
}

/// A registered device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: i64,
    pub name: String,
    pub mac_address: String,
}

/// Workspace metadata for .proxybot files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub db_path: PathBuf,
    pub rules: Vec<Rule>,
    pub devices: Vec<Device>,
    pub created_at: DateTime<Utc>,
}

impl Workspace {
    pub fn new(name: String, db_path: PathBuf, rules: Vec<Rule>, devices: Vec<Device>) -> Self {
        Self {
            name,
            db_path,
            rules,
            devices,
            created_at: Utc::now(),
        }
    }
}

/// Metadata about an exported workspace file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub rule_count: usize,
    pub device_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_serialization() {
        let workspace = Workspace::new(
            "test-workspace".to_string(),
            PathBuf::from("/tmp/test.db"),
            vec![
                Rule::new(
                    "rule-1".to_string(),
                    RuleAction::Direct,
                    "*.example.com".to_string(),
                    true,
                ),
                Rule::new(
                    "rule-2".to_string(),
                    RuleAction::Proxy,
                    "api.example.com".to_string(),
                    false,
                ),
            ],
            vec![
                Device {
                    id: 1,
                    name: "iPhone".to_string(),
                    mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
                },
            ],
        );

        let json = serde_json::to_string_pretty(&workspace).unwrap();
        let parsed: Workspace = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "test-workspace");
        assert_eq!(parsed.rules.len(), 2);
        assert_eq!(parsed.devices.len(), 1);
        assert_eq!(parsed.rules[0].action, RuleAction::Direct);
    }

    #[test]
    fn test_rule_action_serialization() {
        let actions = vec![
            RuleAction::Direct,
            RuleAction::Proxy,
            RuleAction::Reject,
            RuleAction::MapRemote("http://localhost:8080".to_string()),
            RuleAction::MapLocal("/tmp/local".to_string()),
        ];

        for action in actions {
            let json = serde_json::to_string(&action).unwrap();
            let parsed: RuleAction = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, action);
        }
    }
}