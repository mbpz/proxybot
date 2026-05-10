//! Rules engine for request routing and classification
//! Handles domain-based routing rules (WeChat, Douyin, Alipay, etc.)

use anyhow::Result;
use std::collections::HashMap;

/// Rule match result
#[derive(Debug, Clone, PartialEq)]
pub enum RuleMatch {
    /// Matched a specific app rule
    App(String),
    /// No matching rule found
    None,
}

/// Rules engine for routing decisions
pub struct RulesEngine {
    rules: HashMap<String, Vec<String>>,
}

impl RulesEngine {
    /// Create a new RulesEngine with default rules
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        // WeChat domains
        rules.insert(
            "wechat".to_string(),
            vec![
                "*.weixin.qq.com".to_string(),
                "*.wechat.com".to_string(),
                "*.qq.com".to_string(),
            ],
        );
        // Douyin/TikTok domains
        rules.insert(
            "douyin".to_string(),
            vec![
                "*.douyin.com".to_string(),
                "*.tiktokv.com".to_string(),
                "*.tiktok.com".to_string(),
            ],
        );
        // Alipay domains
        rules.insert(
            "alipay".to_string(),
            vec![
                "*.alipay.com".to_string(),
                "*.alipayusercontent.com".to_string(),
            ],
        );
        Self { rules }
    }

    /// Check if a domain matches any known rules
    pub fn match_domain(&self, domain: &str) -> RuleMatch {
        for (app, patterns) in &self.rules {
            for pattern in patterns {
                if self::domain_matches_pattern(domain, pattern) {
                    return RuleMatch::App(app.clone());
                }
            }
        }
        RuleMatch::None
    }

    /// Add a new rule for an app
    pub fn add_rule(&mut self, app: &str, pattern: &str) -> Result<()> {
        let entry = self.rules.entry(app.to_string()).or_insert_with(Vec::new);
        entry.push(pattern.to_string());
        Ok(())
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a domain matches a wildcard pattern
fn domain_matches_pattern(domain: &str, pattern: &str) -> bool {
    if pattern.starts_with("*.") {
        let suffix = &pattern[2..];
        domain.ends_with(suffix) || domain == suffix
    } else {
        domain == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rules_engine_new() {
        let engine = RulesEngine::new();
        assert_eq!(engine.rules.len(), 3);
    }

    #[test]
    fn test_match_wechat_domain() {
        let engine = RulesEngine::new();
        assert_eq!(
            engine.match_domain("web.weixin.qq.com"),
            RuleMatch::App("wechat".to_string())
        );
    }

    #[test]
    fn test_match_douyin_domain() {
        let engine = RulesEngine::new();
        assert_eq!(
            engine.match_domain("api.tiktokv.com"),
            RuleMatch::App("douyin".to_string())
        );
    }

    #[test]
    fn test_match_alipay_domain() {
        let engine = RulesEngine::new();
        assert_eq!(
            engine.match_domain("open.alipay.com"),
            RuleMatch::App("alipay".to_string())
        );
    }

    #[test]
    fn test_match_unknown_domain() {
        let engine = RulesEngine::new();
        assert_eq!(engine.match_domain("example.com"), RuleMatch::None);
    }

    #[test]
    fn test_domain_matches_pattern() {
        assert!(domain_matches_pattern("web.weixin.qq.com", "*.weixin.qq.com"));
        assert!(domain_matches_pattern("api.douyin.com", "*.douyin.com"));
        assert!(!domain_matches_pattern("evil.com", "*.weixin.qq.com"));
    }
}