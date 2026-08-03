//! Desktop Adapter for custom Application Attribution rules.
//!
//! The store is constructed by the composition root with an explicit path;
//! there is no process-global cache or test-time environment mutation.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use proxybot_core::{AppSignature, CustomAppRule};
use tauri::State;

pub struct CustomAppRuleStore {
    path: PathBuf,
    rules: Mutex<Vec<CustomAppRule>>,
}

impl CustomAppRuleStore {
    pub fn from_path(path: PathBuf) -> Self {
        let rules = proxybot_core::load_custom_app_rules_from(&path);
        Self {
            path,
            rules: Mutex::new(rules),
        }
    }

    fn snapshot(&self) -> Result<Vec<CustomAppRule>, String> {
        self.rules
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))
            .map(|rules| rules.clone())
    }

    fn upsert(&self, rule: CustomAppRule) -> Result<Vec<CustomAppRule>, String> {
        let mut rules = self
            .rules
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        if let Some(existing) = rules.iter_mut().find(|entry| entry.app_id == rule.app_id) {
            *existing = rule;
        } else {
            rules.push(rule);
        }
        persist(&self.path, &rules)?;
        Ok(rules.clone())
    }

    fn remove(&self, app_id: &str) -> Result<Vec<CustomAppRule>, String> {
        let mut rules = self
            .rules
            .lock()
            .map_err(|error| format!("lock poisoned: {error}"))?;
        rules.retain(|entry| entry.app_id != app_id);
        persist(&self.path, &rules)?;
        Ok(rules.clone())
    }
}

fn persist(path: &Path, rules: &[CustomAppRule]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("mkdir {parent:?}: {error}"))?;
    }
    let content =
        serde_json::to_string_pretty(rules).map_err(|error| format!("serialize: {error}"))?;
    std::fs::write(path, content).map_err(|error| format!("write {path:?}: {error}"))
}

/// Return the built-in signatures followed by user-defined rules.
#[tauri::command]
pub fn get_app_signatures(
    store: State<'_, Arc<CustomAppRuleStore>>,
) -> Result<Vec<AppSignature>, String> {
    let mut signatures = proxybot_core::get_default_signatures();
    for rule in store.snapshot()? {
        let sni_patterns = rule
            .conditions
            .iter()
            .filter_map(|condition| match condition {
                proxybot_core::RuleCondition::Sni { pattern } => Some(pattern.clone()),
                _ => None,
            })
            .collect();
        signatures.push(AppSignature {
            app_id: rule.app_id,
            app_name: rule.app_name,
            icon: rule.icon,
            sni_patterns,
            fingerprints: Vec::new(),
        });
    }
    Ok(signatures)
}

#[tauri::command]
pub fn add_custom_rule(
    rule: CustomAppRule,
    dns: State<'_, Arc<crate::dns::DnsState>>,
    store: State<'_, Arc<CustomAppRuleStore>>,
) -> Result<(), String> {
    let rules = store.upsert(rule)?;
    dns.replace_custom_attribution_rules(rules);
    Ok(())
}

#[tauri::command]
pub fn remove_custom_rule(
    app_id: String,
    dns: State<'_, Arc<crate::dns::DnsState>>,
    store: State<'_, Arc<CustomAppRuleStore>>,
) -> Result<(), String> {
    let rules = store.remove(&app_id)?;
    dns.replace_custom_attribution_rules(rules);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proxybot_core::RuleCondition;

    fn rule(id: &str, name: &str) -> CustomAppRule {
        CustomAppRule {
            app_id: id.into(),
            app_name: name.into(),
            icon: "I".into(),
            conditions: vec![RuleCondition::Sni {
                pattern: "*.internal.corp".into(),
            }],
            confidence: 0.7,
        }
    }

    #[test]
    fn store_persists_upsert_and_idempotent_remove() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("app_signatures.json");
        let store = CustomAppRuleStore::from_path(path.clone());

        store.upsert(rule("internal", "v1")).unwrap();
        store.upsert(rule("internal", "v2")).unwrap();
        assert_eq!(store.snapshot().unwrap()[0].app_name, "v2");
        assert_eq!(proxybot_core::load_custom_app_rules_from(&path).len(), 1);

        store.remove("internal").unwrap();
        store.remove("internal").unwrap();
        assert!(proxybot_core::load_custom_app_rules_from(&path).is_empty());
    }

    #[test]
    fn custom_rules_project_to_signatures() {
        let temp = tempfile::tempdir().unwrap();
        let store = CustomAppRuleStore::from_path(temp.path().join("rules.json"));
        store.upsert(rule("internal", "Internal")).unwrap();
        let custom = store.snapshot().unwrap().remove(0);
        assert_eq!(custom.app_id, "internal");
        assert!(matches!(custom.conditions[0], RuleCondition::Sni { .. }));
    }
}
