//! Integration tests for the rules engine.

use proxybot_lib::rules::{Rule, RuleAction, RulePattern, RulesEngine};
use std::sync::Arc;
use tempfile::tempdir;

fn make_rule(pattern: RulePattern, value: &str, action: RuleAction) -> Rule {
    Rule {
        pattern,
        value: value.to_string(),
        action,
        name: String::new(),
        priority: 100,
        enabled: true,
        comment: String::new(),
    }
}

#[test]
fn test_rules_engine_creation() {
    let engine = RulesEngine::new();
    let rules = engine.get_rules();
    drop(rules);
}

#[test]
fn test_match_host_returns_none_for_unknown() {
    let engine = RulesEngine::new();
    let result = engine.match_host("unknown.example.xyz", None);
    assert!(result.is_none());
}

#[test]
fn test_rule_action_display() {
    assert_eq!(RuleAction::Direct.to_string(), "DIRECT");
    assert_eq!(RuleAction::Proxy.to_string(), "PROXY");
    assert_eq!(RuleAction::Reject.to_string(), "REJECT");
    assert_eq!(
        RuleAction::MapRemote("https://other.com".to_string()).to_string(),
        "MAPREMOTE:https://other.com"
    );
    assert_eq!(
        RuleAction::MapLocal("/path/to/file".to_string()).to_string(),
        "MAPLOCAL:/path/to/file"
    );
}

#[test]
fn test_rule_pattern_display() {
    assert_eq!(RulePattern::Domain.to_string(), "DOMAIN");
    assert_eq!(RulePattern::DomainSuffix.to_string(), "DOMAIN-SUFFIX");
    assert_eq!(RulePattern::DomainKeyword.to_string(), "DOMAIN-KEYWORD");
    assert_eq!(RulePattern::IpCidr.to_string(), "IP-CIDR");
    assert_eq!(RulePattern::Geoip.to_string(), "GEOIP");
    assert_eq!(RulePattern::RuleSet.to_string(), "RULE-SET");
}

#[test]
fn test_rule_serialization_roundtrip() {
    let rule = make_rule(RulePattern::DomainSuffix, "qq.com", RuleAction::Proxy);

    let json = serde_json::to_string(&rule).unwrap();
    let deserialized: Rule = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.value, "qq.com");
    assert_eq!(deserialized.action, RuleAction::Proxy);
}

#[test]
fn test_rule_with_all_fields() {
    let rule = Rule {
        pattern: RulePattern::Domain,
        value: "example.com".to_string(),
        action: RuleAction::Reject,
        name: "Block example".to_string(),
        priority: 50,
        enabled: false,
        comment: "Test rule".to_string(),
    };

    assert_eq!(rule.name, "Block example");
    assert_eq!(rule.priority, 50);
    assert!(!rule.enabled);
    assert_eq!(rule.comment, "Test rule");
}

#[test]
fn test_rules_reload_does_not_panic() {
    let engine = Arc::new(RulesEngine::new());
    engine.reload();
    let rules = engine.get_rules();
    drop(rules);
}

#[test]
fn test_reorder_rule_empty_file_returns_error() {
    let dir = tempdir().unwrap();
    let engine = RulesEngine::with_dir(dir.path().to_path_buf());

    assert!(matches!(
        engine.reorder_rules(0, 0, "custom.yaml"),
        Err(proxybot_lib::rules::RulesError::ReorderOutOfBounds { .. })
    ));
}

#[test]
fn test_rule_action_equality() {
    assert_eq!(RuleAction::Direct, RuleAction::Direct);
    assert_ne!(RuleAction::Direct, RuleAction::Proxy);
    assert_ne!(RuleAction::Reject, RuleAction::Proxy);
}

#[test]
fn test_rule_default_values() {
    let rule = make_rule(RulePattern::Domain, "test.com", RuleAction::Direct);
    assert_eq!(rule.priority, 100);
    assert!(rule.enabled);
    assert!(rule.name.is_empty());
    assert!(rule.comment.is_empty());
}
