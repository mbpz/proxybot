//! Tauri commands for the App Fingerprint feature.
//!
//! Exposes the v0.9.0 app signature library (default + custom rules)
//! to the React frontend. Custom rules are persisted to
//! `~/.proxybot/app_signatures.json` so they survive restarts.

use std::path::PathBuf;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use proxybot_core::{AppSignature, CustomAppRule};
use tauri::State;

/// Path to the on-disk custom rules file. Created lazily on first write.
fn custom_rules_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".proxybot")
        .join("app_signatures.json")
}

/// In-process cache of custom rules so repeated reads from the UI
/// don't hit the disk.
static CUSTOM_RULES: Lazy<Mutex<Vec<CustomAppRule>>> = Lazy::new(|| Mutex::new(load_from_disk()));

fn load_from_disk() -> Vec<CustomAppRule> {
    proxybot_core::load_custom_app_rules_from(&custom_rules_path())
}

fn persist(rules: &[CustomAppRule]) -> Result<(), String> {
    let path = custom_rules_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {parent:?}: {e}"))?;
    }
    let content = serde_json::to_string_pretty(rules).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&path, content).map_err(|e| format!("write {path:?}: {e}"))?;
    Ok(())
}

/// Return every app signature — the built-in defaults followed by
/// any user-defined custom rules.
#[tauri::command]
pub fn get_app_signatures() -> Result<Vec<AppSignature>, String> {
    let mut out = proxybot_core::get_default_signatures();
    let custom = CUSTOM_RULES
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?
        .clone();
    for rule in custom {
        let sni_patterns = extract_sni_patterns(&rule);
        out.push(AppSignature {
            app_id: rule.app_id,
            app_name: rule.app_name,
            icon: rule.icon,
            sni_patterns,
            fingerprints: Vec::new(),
        });
    }
    Ok(out)
}

fn extract_sni_patterns(rule: &CustomAppRule) -> Vec<String> {
    rule.conditions
        .iter()
        .filter_map(|c| match c {
            proxybot_core::RuleCondition::Sni { pattern } => Some(pattern.clone()),
            _ => None,
        })
        .collect()
}

/// Persist a new custom rule. If a rule with the same `app_id` already
/// exists it is overwritten (idempotent upsert).
#[tauri::command]
pub fn add_custom_rule(
    rule: CustomAppRule,
    dns: State<'_, std::sync::Arc<crate::dns::DnsState>>,
) -> Result<(), String> {
    let rules = upsert_custom_rule(rule)?;
    dns.replace_custom_attribution_rules(rules);
    Ok(())
}

fn upsert_custom_rule(rule: CustomAppRule) -> Result<Vec<CustomAppRule>, String> {
    let mut guard = CUSTOM_RULES
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    if let Some(existing) = guard.iter_mut().find(|r| r.app_id == rule.app_id) {
        *existing = rule;
    } else {
        guard.push(rule);
    }
    persist(&guard)?;
    Ok(guard.clone())
}

/// Remove a custom rule by `app_id`. Returns Ok even if the id was
/// not present (idempotent delete).
#[tauri::command]
pub fn remove_custom_rule(
    app_id: String,
    dns: State<'_, std::sync::Arc<crate::dns::DnsState>>,
) -> Result<(), String> {
    let rules = remove_custom_rule_from_store(&app_id)?;
    dns.replace_custom_attribution_rules(rules);
    Ok(())
}

fn remove_custom_rule_from_store(app_id: &str) -> Result<Vec<CustomAppRule>, String> {
    let mut guard = CUSTOM_RULES
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    guard.retain(|r| r.app_id != app_id);
    persist(&guard)?;
    Ok(guard.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proxybot_core::RuleCondition;
    use std::env;
    use std::sync::Mutex;

    /// Serialise the tests so the shared `CUSTOM_RULES` static doesn't
    /// race between cases.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Each test points CUSTOM_RULES at a unique temp file so they
    /// don't fight over the user's real `~/.proxybot/app_signatures.json`.
    fn with_temp_path<F: FnOnce()>(f: F) {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        env::set_var("HOME", tmp.path());
        // The lazy static is process-wide; clear it so it doesn't
        // leak rules from earlier tests or from the real user dir.
        {
            let mut g = CUSTOM_RULES.lock().unwrap();
            *g = Vec::new();
        }
        f();
    }

    #[test]
    fn add_and_remove_persists() {
        with_temp_path(|| {
            let rule = CustomAppRule {
                app_id: "myapp".into(),
                app_name: "My App".into(),
                icon: "M".into(),
                conditions: vec![RuleCondition::Sni {
                    pattern: "*.mycompany.com".into(),
                }],
                confidence: 0.8,
            };
            upsert_custom_rule(rule.clone()).expect("add");
            assert!(custom_rules_path().exists(), "should write file");

            // Re-read from disk to confirm persistence
            let reread = load_from_disk();
            assert_eq!(reread.len(), 1);
            assert_eq!(reread[0].app_id, "myapp");

            // Idempotent remove
            remove_custom_rule_from_store("myapp").expect("remove");
            remove_custom_rule_from_store("myapp").expect("remove again");
            assert!(load_from_disk().is_empty());
        });
    }

    #[test]
    fn add_overwrites_existing_app_id() {
        with_temp_path(|| {
            let v1 = CustomAppRule {
                app_id: "dup".into(),
                app_name: "v1".into(),
                icon: "1".into(),
                conditions: vec![],
                confidence: 0.5,
            };
            let v2 = CustomAppRule {
                app_id: "dup".into(),
                app_name: "v2".into(),
                icon: "2".into(),
                conditions: vec![],
                confidence: 0.6,
            };
            upsert_custom_rule(v1).unwrap();
            upsert_custom_rule(v2).unwrap();
            let rules = load_from_disk();
            assert_eq!(rules.len(), 1);
            assert_eq!(rules[0].app_name, "v2");
        });
    }

    #[test]
    fn get_app_signatures_merges_default_and_custom() {
        with_temp_path(|| {
            upsert_custom_rule(CustomAppRule {
                app_id: "internal".into(),
                app_name: "Internal".into(),
                icon: "I".into(),
                conditions: vec![RuleCondition::Sni {
                    pattern: "*.internal.corp".into(),
                }],
                confidence: 0.7,
            })
            .unwrap();

            let all = get_app_signatures().expect("signatures");
            // Built-in defaults + our custom one
            assert!(all.iter().any(|s| s.app_id == "internal"));
            assert!(all.iter().any(|s| s.app_id == "tiktok"));
            assert!(all.iter().any(|s| s.app_id == "wechat"));
        });
    }
}
