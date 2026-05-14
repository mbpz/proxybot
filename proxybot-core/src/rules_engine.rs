//! Rules engine for domain-based traffic routing — no Tauri/GUI dependencies.
//!
//! Handles:
//! - Domain/domain-suffix/domain-keyword/IP-CIDR pattern matching
//! - Priority-based rule evaluation (lower priority = higher precedence)
//! - Integration with app classification for combined routing decisions
//!
//! # Integration
//!
//! This module contains pure matching logic. The Tauri layer
//! (`src-tauri/src/rules.rs`) wraps it with YAML file loading,
//! hot-reload file watching, and `#[tauri::command]` annotations.

use crate::app_classifier;
use crate::types::{AppRule, Rule, RuleAction, RulePattern};
use std::collections::HashMap;
use std::net::IpAddr;

/// Result of matching a host against the rule set.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleMatch {
    /// Matched a specific app rule.
    App(String),
    /// No matching rule found.
    None,
}

/// Rules engine for routing decisions and classification.
pub struct RulesEngine {
    /// App classification rules: app_name → domains
    app_rules: HashMap<String, Vec<String>>,
    /// User-defined routing rules, sorted by priority.
    routing_rules: Vec<Rule>,
}

impl RulesEngine {
    /// Create a new RulesEngine with default app classification rules.
    pub fn new() -> Self {
        let default_rules = app_classifier::get_default_rules();
        let mut app_rules = HashMap::new();
        for rule in &default_rules {
            app_rules.insert(rule.name.clone(), rule.domains.clone());
        }

        Self {
            app_rules,
            routing_rules: Vec::new(),
        }
    }

    /// Create a new RulesEngine with custom app rules.
    pub fn with_app_rules(app_rules_list: Vec<AppRule>) -> Self {
        let mut app_rules = HashMap::new();
        for rule in &app_rules_list {
            app_rules.insert(rule.name.clone(), rule.domains.clone());
        }
        Self {
            app_rules,
            routing_rules: Vec::new(),
        }
    }

    /// Set the routing rules. Rules are sorted by priority (ascending).
    pub fn set_rules(&mut self, rules: Vec<Rule>) {
        self.routing_rules = rules;
        self.routing_rules.sort_by_key(|r| r.priority);
    }

    /// Add a single routing rule.
    pub fn add_rule(&mut self, rule: Rule) {
        self.routing_rules.push(rule);
        self.routing_rules.sort_by_key(|r| r.priority);
    }

    /// Add an app classification rule.
    pub fn add_app_rule(&mut self, app: &str, domains: Vec<String>) {
        self.app_rules
            .entry(app.to_string())
            .or_insert_with(Vec::new)
            .extend(domains);
    }

    /// Check if a domain matches any known app rules.
    pub fn classify_app(&self, host: &str) -> RuleMatch {
        for (app, domains) in &self.app_rules {
            for domain in domains {
                if host_matches_domain(host, domain) {
                    return RuleMatch::App(app.clone());
                }
            }
        }
        RuleMatch::None
    }

    /// Match a host against routing rules.
    ///
    /// Returns the first matching rule's action (highest priority first).
    /// `client_ip` is optional and used for IP-CIDR rules.
    pub fn match_host(&self, host: &str, client_ip: Option<IpAddr>) -> Option<RuleAction> {
        for rule in &self.routing_rules {
            if match_rule(rule, host, client_ip) {
                return Some(rule.action.clone());
            }
        }
        None
    }

    /// Get all app classification rules.
    pub fn get_app_rules(&self) -> HashMap<String, Vec<String>> {
        self.app_rules.clone()
    }

    /// Get all routing rules.
    pub fn get_rules(&self) -> &[Rule] {
        &self.routing_rules
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a host matches a domain pattern.
///
/// Supports:
/// - Exact match: `host == domain`
/// - Subdomain match: `api.example.com` matches `example.com`
/// - Wildcard suffix: `*.example.com` matches `api.example.com`
/// - Case-insensitive comparison
///
/// # False-positive protection
///
/// `domain = "qq.com"` does NOT match `"qq.com.evil.com"`.
/// Only the exact domain or direct subdomains match.
pub fn host_matches_domain(host: &str, domain: &str) -> bool {
    let host = host.to_lowercase();
    let domain = domain.to_lowercase();

    if domain.starts_with("*.") {
        let suffix = &domain[2..];
        if host == suffix {
            return true;
        }
        if host.ends_with(&format!(".{}", suffix)) {
            return true;
        }
        return false;
    }

    // Exact match
    if host == domain {
        return true;
    }

    // Subdomain match: "api.example.com" matches "example.com"
    if host.ends_with(&format!(".{}", domain)) {
        return true;
    }

    false
}

/// Match a host against a routing rule.
///
/// Evaluates the rule's pattern and value against the host.
/// Returns false if the rule is disabled.
pub fn match_rule(rule: &Rule, host: &str, client_ip: Option<IpAddr>) -> bool {
    if !rule.enabled {
        return false;
    }

    let host_lower = host.to_lowercase();

    match rule.pattern {
        RulePattern::Domain => host_lower == rule.value.to_lowercase(),
        RulePattern::DomainSuffix => {
            let suffix = rule.value.to_lowercase();
            // Match: host is exactly suffix OR host ends with ".suffix"
            // but NOT "suffix.evil.com"
            if host_lower == suffix {
                return true;
            }
            if host_lower.ends_with(&format!(".{}", suffix)) {
                return true;
            }
            false
        }
        RulePattern::DomainKeyword => host_lower.contains(&rule.value.to_lowercase()),
        RulePattern::IpCidr => {
            if let Some(ip) = client_ip {
                if let Ok(network) = rule.value.parse::<ipnetwork::IpNetwork>() {
                    return network.contains(ip);
                }
            }
            false
        }
        RulePattern::Geoip => {
            // Placeholder — GeoIP requires external database
            false
        }
        RulePattern::RuleSet => {
            // Placeholder — RuleSet requires external file loading
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── host_matches_domain tests ──────────────────────────────────────

    #[test]
    fn test_exact_match() {
        assert!(host_matches_domain("example.com", "example.com"));
        assert!(!host_matches_domain("example.com", "other.com"));
    }

    #[test]
    fn test_wildcard_match() {
        assert!(host_matches_domain("api.weixin.qq.com", "*.weixin.qq.com"));
        assert!(host_matches_domain("weixin.qq.com", "*.weixin.qq.com"));
        assert!(!host_matches_domain("evil.weixin.qq.com.evil.com", "*.weixin.qq.com"));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(host_matches_domain("EXAMPLE.COM", "example.com"));
        assert!(host_matches_domain("Api.Weixin.QQ.Com", "*.weixin.qq.com"));
    }

    #[test]
    fn test_no_false_positive() {
        assert!(!host_matches_domain("qq.com.evil.com", "qq.com"));
        assert!(!host_matches_domain("qq.com.evil.com", "*.qq.com"));
    }

    // ─── match_rule tests ───────────────────────────────────────────────

    #[test]
    fn test_rule_domain_exact() {
        let rule = Rule {
            pattern: RulePattern::Domain,
            value: "example.com".to_string(),
            action: RuleAction::Direct,
            name: "test".to_string(),
            priority: 100,
            enabled: true,
            comment: "".to_string(),
        };
        assert!(match_rule(&rule, "example.com", None));
        assert!(match_rule(&rule, "EXAMPLE.COM", None));
        assert!(!match_rule(&rule, "sub.example.com", None));
    }

    #[test]
    fn test_rule_domain_suffix() {
        let rule = Rule {
            pattern: RulePattern::DomainSuffix,
            value: "example.com".to_string(),
            action: RuleAction::Proxy,
            name: "test".to_string(),
            priority: 100,
            enabled: true,
            comment: "".to_string(),
        };
        assert!(match_rule(&rule, "example.com", None));
        assert!(match_rule(&rule, "sub.example.com", None));
        assert!(!match_rule(&rule, "example.com.evil.com", None));
    }

    #[test]
    fn test_rule_domain_keyword() {
        let rule = Rule {
            pattern: RulePattern::DomainKeyword,
            value: "wechat".to_string(),
            action: RuleAction::Direct,
            name: "test".to_string(),
            priority: 100,
            enabled: true,
            comment: "".to_string(),
        };
        assert!(match_rule(&rule, "api.wechat.com", None));
        assert!(match_rule(&rule, "wechat-api.example.com", None));
        assert!(!match_rule(&rule, "chat.example.com", None));
    }

    #[test]
    fn test_rule_ip_cidr() {
        let rule = Rule {
            pattern: RulePattern::IpCidr,
            value: "10.0.0.0/8".to_string(),
            action: RuleAction::Reject,
            name: "test".to_string(),
            priority: 100,
            enabled: true,
            comment: "".to_string(),
        };
        let ip: IpAddr = "10.1.2.3".parse().unwrap();
        assert!(match_rule(&rule, "host.example.com", Some(ip)));
        let ip2: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(!match_rule(&rule, "host.example.com", Some(ip2)));
    }

    #[test]
    fn test_disabled_rule() {
        let rule = Rule {
            pattern: RulePattern::Domain,
            value: "example.com".to_string(),
            action: RuleAction::Reject,
            name: "disabled".to_string(),
            priority: 1,
            enabled: false,
            comment: "".to_string(),
        };
        assert!(!match_rule(&rule, "example.com", None));
    }

    // ─── RulesEngine tests ──────────────────────────────────────────────

    #[test]
    fn test_engine_classify_wechat() {
        let engine = RulesEngine::new();
        assert_eq!(
            engine.classify_app("web.weixin.qq.com"),
            RuleMatch::App("WeChat".to_string())
        );
    }

    #[test]
    fn test_engine_classify_unknown() {
        let engine = RulesEngine::new();
        assert_eq!(engine.classify_app("example.com"), RuleMatch::None);
    }

    #[test]
    fn test_engine_match_host() {
        let mut engine = RulesEngine::new();
        engine.set_rules(vec![Rule {
            pattern: RulePattern::DomainSuffix,
            value: "api.example.com".to_string(),
            action: RuleAction::MapRemote("https://mock.local".to_string()),
            name: "mock".to_string(),
            priority: 50,
            enabled: true,
            comment: "".to_string(),
        }]);

        assert!(engine.match_host("api.example.com", None).is_some());
        assert!(engine.match_host("unrelated.com", None).is_none());
    }

    #[test]
    fn test_engine_priority_order() {
        let mut engine = RulesEngine::new();
        engine.set_rules(vec![
            Rule {
                pattern: RulePattern::Domain,
                value: "example.com".to_string(),
                action: RuleAction::Proxy,
                name: "lower-priority".to_string(),
                priority: 100,
                enabled: true,
                comment: "".to_string(),
            },
            Rule {
                pattern: RulePattern::Domain,
                value: "example.com".to_string(),
                action: RuleAction::Reject,
                name: "higher-priority".to_string(),
                priority: 1,
                enabled: true,
                comment: "".to_string(),
            },
        ]);

        // Higher priority (lower number) should match first
        assert_eq!(
            engine.match_host("example.com", None),
            Some(RuleAction::Reject)
        );
    }

    #[test]
    fn test_engine_add_app_rule() {
        let mut engine = RulesEngine::new();
        engine.add_app_rule("CustomApp", vec!["custom.app".to_string()]);
        assert_eq!(
            engine.classify_app("api.custom.app"),
            RuleMatch::App("CustomApp".to_string())
        );
    }
}
