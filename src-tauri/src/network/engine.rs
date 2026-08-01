use crate::network::builtin_presets;
use crate::network::profile::NetworkProfile;
pub use proxybot_core::HostPattern as NetworkHostPattern;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

pub struct ConditionEffect {
    pub delay_ms: u64,
    pub drop: bool,
}

/// Per-host condition rule — matches a host pattern and applies a named profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConditionRule {
    pub id: u64,
    pub pattern: NetworkHostPattern,
    pub value: String,
    pub profile: String,
    pub enabled: bool,
}

/// New condition-rule input from the API. `id` is assigned by the engine.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewConditionRule {
    pub pattern: NetworkHostPattern,
    pub value: String,
    pub profile: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

pub struct NetworkConditionEngine {
    profiles: RwLock<HashMap<String, NetworkProfile>>,
    active_profile: RwLock<Option<NetworkProfile>>,
    rules: RwLock<Vec<ConditionRule>>,
    next_rule_id: AtomicU64,
}

impl NetworkConditionEngine {
    pub fn new() -> Self {
        let profiles: HashMap<String, NetworkProfile> = builtin_presets()
            .into_iter()
            .map(|p| (p.name.clone(), p))
            .collect();
        Self {
            profiles: RwLock::new(profiles),
            active_profile: RwLock::new(None),
            rules: RwLock::new(Vec::new()),
            next_rule_id: AtomicU64::new(1),
        }
    }

    pub fn set_active(&self, name: &str) -> Result<(), String> {
        let profiles = self.profiles.read().unwrap();
        let profile = profiles
            .get(name)
            .ok_or_else(|| format!("Profile not found: {}", name))?;
        *self.active_profile.write().unwrap() = Some(profile.clone());
        Ok(())
    }

    pub fn disable(&self) {
        *self.active_profile.write().unwrap() = None;
    }

    pub fn get_active(&self) -> Option<NetworkProfile> {
        self.active_profile.read().unwrap().clone()
    }

    pub fn list_profiles(&self) -> Vec<NetworkProfile> {
        self.profiles.read().unwrap().values().cloned().collect()
    }

    pub fn add_profile(&self, profile: NetworkProfile) {
        self.profiles
            .write()
            .unwrap()
            .insert(profile.name.clone(), profile);
    }

    /// Add a new condition rule. Returns the assigned id.
    pub fn add_rule(&self, new_rule: NewConditionRule) -> u64 {
        let id = self.next_rule_id.fetch_add(1, Ordering::SeqCst);
        let rule = ConditionRule {
            id,
            pattern: new_rule.pattern,
            value: new_rule.value,
            profile: new_rule.profile,
            enabled: new_rule.enabled,
        };
        self.rules.write().unwrap().push(rule);
        id
    }

    /// Remove a rule by id. Returns true if the rule existed.
    pub fn remove_rule(&self, id: u64) -> bool {
        let mut rules = self.rules.write().unwrap();
        let before = rules.len();
        rules.retain(|r| r.id != id);
        rules.len() != before
    }

    /// Snapshot of all condition rules (enabled and disabled).
    pub fn list_rules(&self) -> Vec<ConditionRule> {
        self.rules.read().unwrap().clone()
    }

    /// Find the first enabled rule matching the given host, return its profile name.
    pub fn match_profile_for_host(&self, host: &str) -> Option<String> {
        let rules = self.rules.read().unwrap();
        for rule in rules.iter().filter(|r| r.enabled) {
            if proxybot_core::match_host_pattern(
                &rule.pattern,
                &rule.value,
                host,
                host.parse().ok(),
            ) {
                return Some(rule.profile.clone());
            }
        }
        None
    }

    /// Compute condition effect for a read of N bytes
    pub fn apply(&self, read_size: usize) -> ConditionEffect {
        let profile = match self.active_profile.read().unwrap().as_ref() {
            Some(p) => p.clone(),
            None => {
                return ConditionEffect {
                    delay_ms: 0,
                    drop: false,
                }
            }
        };

        Self::effect(&profile, read_size)
    }

    /// Compute the effect for a host-specific rule, falling back to the active profile.
    pub fn apply_for_host(&self, host: &str, read_size: usize) -> ConditionEffect {
        let profile = self
            .match_profile_for_host(host)
            .and_then(|name| self.profiles.read().unwrap().get(&name).cloned())
            .or_else(|| self.active_profile.read().unwrap().clone());
        match profile {
            Some(profile) => Self::effect(&profile, read_size),
            None => ConditionEffect {
                delay_ms: 0,
                drop: false,
            },
        }
    }

    fn effect(profile: &NetworkProfile, read_size: usize) -> ConditionEffect {
        // Packet loss: randomly drop
        let drop = if profile.packet_loss_pct > 0 {
            (rand::random::<u8>() % 100) < profile.packet_loss_pct
        } else {
            false
        };

        // Latency
        let latency_delay = profile.latency_ms;

        // Bandwidth cap: minimum time to transfer N bytes at capped rate
        let bandwidth_delay = if profile.bandwidth_kbps > 0 {
            // bits / (kbps * 1000) * 1000 = ms
            (read_size as u64 * 8 * 1000) / (profile.bandwidth_kbps * 1000)
        } else {
            0
        };

        let delay_ms = latency_delay.max(bandwidth_delay);

        ConditionEffect { delay_ms, drop }
    }
}

impl Default for NetworkConditionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presets_exist() {
        let engine = NetworkConditionEngine::new();
        let profiles = engine.list_profiles();
        assert!(profiles.iter().any(|p| p.name == "3G"));
        assert!(profiles.iter().any(|p| p.name == "4G"));
        assert!(profiles.iter().any(|p| p.name == "2G"));
    }

    #[test]
    fn test_set_active() {
        let engine = NetworkConditionEngine::new();
        engine.set_active("3G").unwrap();
        let active = engine.get_active().unwrap();
        assert_eq!(active.name, "3G");
        assert_eq!(active.latency_ms, 300);
    }

    #[test]
    fn test_disable() {
        let engine = NetworkConditionEngine::new();
        engine.set_active("3G").unwrap();
        engine.disable();
        assert!(engine.get_active().is_none());
    }

    #[test]
    fn test_apply_no_profile() {
        let engine = NetworkConditionEngine::new();
        let effect = engine.apply(1024);
        assert_eq!(effect.delay_ms, 0);
        assert!(!effect.drop);
    }

    #[test]
    fn test_apply_latency() {
        let engine = NetworkConditionEngine::new();
        let mut profile = engine.profiles.read().unwrap().get("3G").unwrap().clone();
        profile.packet_loss_pct = 0;
        profile.bandwidth_kbps = 0;
        *engine.active_profile.write().unwrap() = Some(profile);

        let effect = engine.apply(1024);
        assert_eq!(effect.delay_ms, 300);
        assert!(!effect.drop);
    }

    #[test]
    fn host_rule_applies_without_changing_global_profile() {
        let engine = NetworkConditionEngine::new();
        engine.add_rule(NewConditionRule {
            pattern: NetworkHostPattern::Domain,
            value: "api.example.com".to_owned(),
            profile: "3G".to_owned(),
            enabled: true,
        });

        assert!(engine.get_active().is_none());
        assert_eq!(engine.apply_for_host("api.example.com", 1024).delay_ms, 300);
        assert_eq!(engine.apply_for_host("other.example.com", 1024).delay_ms, 0);
    }

    #[test]
    fn test_apply_bandwidth_cap() {
        let engine = NetworkConditionEngine::new();
        // 8 kbps = 1 KB/s, so 1024 bytes should take ~1000ms
        let profile = NetworkProfile {
            name: "test".into(),
            latency_ms: 0,
            bandwidth_kbps: 8,
            packet_loss_pct: 0,
        };
        *engine.active_profile.write().unwrap() = Some(profile);

        let effect = engine.apply(1024);
        // 1024 bytes * 8 bits / 8000 bps = 1.024s ≈ 1024ms
        assert!(effect.delay_ms >= 900);
    }

    #[test]
    fn test_add_rule_increments_id() {
        let engine = NetworkConditionEngine::new();
        let id1 = engine.add_rule(NewConditionRule {
            pattern: NetworkHostPattern::DomainSuffix,
            value: "example.com".into(),
            profile: "3G".into(),
            enabled: true,
        });
        let id2 = engine.add_rule(NewConditionRule {
            pattern: NetworkHostPattern::DomainSuffix,
            value: "test.com".into(),
            profile: "4G".into(),
            enabled: true,
        });
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn test_list_rules_returns_added() {
        let engine = NetworkConditionEngine::new();
        let id = engine.add_rule(NewConditionRule {
            pattern: NetworkHostPattern::DomainKeyword,
            value: "cdn".into(),
            profile: "2G".into(),
            enabled: true,
        });
        let rules = engine.list_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, id);
        assert_eq!(rules[0].profile, "2G");
    }

    #[test]
    fn test_remove_rule_deletes_by_id() {
        let engine = NetworkConditionEngine::new();
        let id = engine.add_rule(NewConditionRule {
            pattern: NetworkHostPattern::Domain,
            value: "exact.example".into(),
            profile: "Edge".into(),
            enabled: true,
        });
        assert!(engine.remove_rule(id));
        assert_eq!(engine.list_rules().len(), 0);
        // Removing a non-existent id returns false
        assert!(!engine.remove_rule(9999));
    }

    #[test]
    fn test_match_profile_for_host() {
        let engine = NetworkConditionEngine::new();
        engine.add_rule(NewConditionRule {
            pattern: NetworkHostPattern::DomainSuffix,
            value: "example.com".into(),
            profile: "3G".into(),
            enabled: true,
        });
        engine.add_rule(NewConditionRule {
            pattern: NetworkHostPattern::DomainKeyword,
            value: "video".into(),
            profile: "Edge".into(),
            enabled: false, // disabled — should be ignored
        });

        assert_eq!(
            engine.match_profile_for_host("api.example.com"),
            Some("3G".to_string())
        );
        assert_eq!(engine.match_profile_for_host("unknown.test"), None);
        // disabled rule is skipped even though it would match
        assert_eq!(engine.match_profile_for_host("video.cdn.com"), None);
    }

    #[test]
    fn test_rule_pattern_serialization() {
        let r = NewConditionRule {
            pattern: NetworkHostPattern::DomainKeyword,
            value: "x".into(),
            profile: "WiFi".into(),
            enabled: true,
        };
        let json = serde_json::to_string(&r).unwrap();
        // DomainKeyword has rename = "DOMAIN-KEYWORD"
        assert!(json.contains("DOMAIN-KEYWORD"));
        let back: NewConditionRule = serde_json::from_str(&json).unwrap();
        assert_eq!(back.profile, "WiFi");
    }
}
