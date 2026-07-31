//! Serialization types for workspace files.
//!
//! The .proxybot format is a gzip-compressed tar archive containing:
//! - workspace.json — metadata
//! - requests.db — SQLite database (copy)
//! - rules.yaml — exported rules

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use proxybot_core::RuleAction;

/// Legacy workspace metadata about a rule. This is deliberately named as an
/// archive Adapter rather than pretending to be a complete Routing Rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRule {
    pub id: String,
    #[serde(with = "workspace_rule_action")]
    pub action: RuleAction,
    pub pattern: String,
    pub enabled: bool,
}

impl WorkspaceRule {
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
    pub rules: Vec<WorkspaceRule>,
    pub devices: Vec<Device>,
    pub created_at: DateTime<Utc>,
}

impl Workspace {
    pub fn new(
        name: String,
        db_path: PathBuf,
        rules: Vec<WorkspaceRule>,
        devices: Vec<Device>,
    ) -> Self {
        Self {
            name,
            db_path,
            rules,
            devices,
            created_at: Utc::now(),
        }
    }
}

/// Preserve the historical workspace wire shape while using the canonical
/// Rule Action domain type in memory. Canonical tagged JSON is also accepted
/// on import so archives can migrate forward without a flag day.
mod workspace_rule_action {
    use proxybot_core::{BreakpointTarget, RuleAction};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "UPPERCASE")]
    enum LegacyAction {
        Direct,
        Proxy,
        Reject,
        #[serde(rename = "MAPREMOTE")]
        MapRemote(String),
        #[serde(rename = "MAPLOCAL")]
        MapLocal(String),
        #[serde(rename = "BREAKPOINT")]
        Breakpoint(BreakpointTarget),
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum CompatibleAction {
        Legacy(LegacyAction),
        Canonical(RuleAction),
    }

    pub fn serialize<S>(action: &RuleAction, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let legacy = match action {
            RuleAction::Direct => LegacyAction::Direct,
            RuleAction::Proxy => LegacyAction::Proxy,
            RuleAction::Reject => LegacyAction::Reject,
            RuleAction::MapRemote(target) => LegacyAction::MapRemote(target.clone()),
            RuleAction::MapLocal(target) => LegacyAction::MapLocal(target.clone()),
            RuleAction::Breakpoint(target) => LegacyAction::Breakpoint(target.clone()),
        };
        legacy.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<RuleAction, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match CompatibleAction::deserialize(deserializer)? {
            CompatibleAction::Legacy(action) => match action {
                LegacyAction::Direct => RuleAction::Direct,
                LegacyAction::Proxy => RuleAction::Proxy,
                LegacyAction::Reject => RuleAction::Reject,
                LegacyAction::MapRemote(target) => RuleAction::MapRemote(target),
                LegacyAction::MapLocal(target) => RuleAction::MapLocal(target),
                LegacyAction::Breakpoint(target) => RuleAction::Breakpoint(target),
            },
            CompatibleAction::Canonical(action) => action,
        })
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
                WorkspaceRule::new(
                    "rule-1".to_string(),
                    RuleAction::Direct,
                    "*.example.com".to_string(),
                    true,
                ),
                WorkspaceRule::new(
                    "rule-2".to_string(),
                    RuleAction::Proxy,
                    "api.example.com".to_string(),
                    false,
                ),
            ],
            vec![Device {
                id: 1,
                name: "iPhone".to_string(),
                mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
            }],
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

    #[test]
    fn workspace_rule_preserves_legacy_actions_and_accepts_canonical_actions() {
        let legacy = WorkspaceRule::new(
            "legacy".to_owned(),
            RuleAction::MapRemote("http://localhost:8080".to_owned()),
            "*.example.com".to_owned(),
            true,
        );
        let value = serde_json::to_value(&legacy).unwrap();
        assert_eq!(
            value["action"],
            serde_json::json!({ "MAPREMOTE": "http://localhost:8080" })
        );

        let canonical: WorkspaceRule = serde_json::from_value(serde_json::json!({
            "id": "canonical",
            "action": { "type": "BREAKPOINT", "target": "BOTH" },
            "pattern": "debug.example.com",
            "enabled": true
        }))
        .unwrap();
        assert_eq!(
            canonical.action,
            RuleAction::Breakpoint(proxybot_core::BreakpointTarget::Both)
        );
    }
}
