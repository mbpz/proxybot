use std::path::Path;
use std::sync::Arc;

use notify::{RecursiveMode, Watcher};
use serde::Deserialize;

use crate::proxy::InterceptedRequest;

/// Rule pattern types for declarative plugin routing
#[derive(Clone, Debug)]
pub enum RulePattern {
    /// Domain suffix match: *.weixin.qq.com
    DomainSuffix(String),
    /// Domain keyword match: contains "weixin"
    DomainKeyword(String),
    /// Full URL pattern match
    UrlPattern {
        method: Option<String>,
        scheme: Option<String>,
        host: Option<String>,
        path: Option<String>,
    },
    /// Header match
    Header { key: String, value: String },
}

/// Glob matching for header values — * matches any sequence
fn glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(pos) = pattern.find('*') {
        let prefix = &pattern[..pos];
        let suffix = &pattern[pos + 1..];
        return value.starts_with(prefix) && value.ends_with(suffix);
    }
    pattern == value
}

/// Wildcard matching helper - supports * as single segment wildcard
fn wildcard_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let value_parts: Vec<&str> = value.split('/').collect();

    let mut pi = 0;
    let mut vi = 0;

    while pi < pattern_parts.len() && vi < value_parts.len() {
        let p = pattern_parts[pi];
        let v = value_parts[vi];
        if p == "*" {
            pi += 1;
            vi += 1;
        } else if p != v {
            return false;
        } else {
            pi += 1;
            vi += 1;
        }
    }
    pi == pattern_parts.len() && vi == value_parts.len()
}

impl RulePattern {
    /// Match this pattern against an intercepted request
    pub fn matches(&self, request: &InterceptedRequest) -> bool {
        match self {
            // Fix 2: strip "*. " prefix so DomainSuffix works with wildcard notation
            RulePattern::DomainSuffix(suffix) => {
                let stripped = suffix.strip_prefix("*.").unwrap_or(suffix);
                request.host.ends_with(stripped)
            }
            RulePattern::DomainKeyword(keyword) => request.host.contains(keyword),
            RulePattern::UrlPattern {
                method,
                scheme,
                host,
                path,
            } => {
                method
                    .as_ref()
                    .is_none_or(|m| m == "*" || m == &request.method)
                    && scheme
                        .as_ref()
                        .is_none_or(|s| s == "*" || s == &request.scheme)
                    && host.as_ref().is_none_or(|h| h == "*" || h == &request.host)
                    && path
                        .as_ref()
                        .is_none_or(|p| wildcard_match(p, &request.path))
            }
            // Fix 3: case-insensitive header key matching per RFC 7230
            RulePattern::Header { key, value } => request
                .req_headers
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case(key) && glob_match(value, v)),
        }
    }
}

// ── YAML deserialization types ──

#[derive(Deserialize, Debug)]
struct RuleFile {
    rules: Vec<RuleFileEntry>,
}

#[derive(Deserialize, Debug)]
struct RuleFileEntry {
    name: String,
    pattern: PatternEntry,
    plugin: String,
    #[serde(default = "default_priority")]
    priority: u16,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_priority() -> u16 {
    100
}
fn default_enabled() -> bool {
    true
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum PatternEntry {
    DomainSuffix {
        value: String,
    },
    DomainKeyword {
        value: String,
    },
    UrlPattern {
        #[serde(default)]
        method: Option<String>,
        #[serde(default)]
        scheme: Option<String>,
        #[serde(default)]
        host: Option<String>,
        #[serde(default)]
        path: Option<String>,
    },
    Header {
        key: String,
        value: String,
    },
}

impl RuleFile {
    fn into_rules(self, start_id: u64) -> Vec<PluginRule> {
        self.rules
            .into_iter()
            .enumerate()
            .map(|(i, entry)| {
                let pattern = match entry.pattern {
                    PatternEntry::DomainSuffix { value } => RulePattern::DomainSuffix(value),
                    PatternEntry::DomainKeyword { value } => RulePattern::DomainKeyword(value),
                    PatternEntry::UrlPattern {
                        method,
                        scheme,
                        host,
                        path,
                    } => RulePattern::UrlPattern {
                        method,
                        scheme,
                        host,
                        path,
                    },
                    PatternEntry::Header { key, value } => RulePattern::Header { key, value },
                };
                PluginRule {
                    id: start_id + i as u64,
                    name: entry.name,
                    pattern,
                    plugin_name: entry.plugin,
                    priority: entry.priority,
                    enabled: entry.enabled,
                }
            })
            .collect()
    }
}

// ── RuleEngine ──
#[derive(Clone, Debug)]
pub struct PluginRule {
    pub id: u64,
    pub name: String,
    pub pattern: RulePattern,
    pub plugin_name: String,
    pub priority: u16,
    pub enabled: bool,
}

/// Rule engine for pattern-matched plugin dispatch
pub struct RuleEngine {
    rules: std::sync::RwLock<Vec<PluginRule>>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            rules: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Match request against rules, return first matching rule (highest priority)
    pub fn match_request(&self, request: &InterceptedRequest) -> Option<PluginRule> {
        let rules = self.rules.read().unwrap();
        rules
            .iter()
            .filter(|r| r.enabled && r.pattern.matches(request))
            .min_by_key(|r| r.priority)
            .cloned()
    }

    /// Match request against all matching rules sorted by priority
    pub fn match_all(&self, request: &InterceptedRequest) -> Vec<PluginRule> {
        let rules = self.rules.read().unwrap();
        let mut matched: Vec<_> = rules
            .iter()
            .filter(|r| r.enabled && r.pattern.matches(request))
            .cloned()
            .collect();
        matched.sort_by_key(|r| r.priority);
        matched
    }

    /// Add a rule (insertion-sorted by priority, stable for equal priorities)
    pub fn add_rule(&self, rule: PluginRule) {
        let mut rules = self.rules.write().unwrap();
        let pos = rules
            .iter()
            .position(|r| r.priority > rule.priority)
            .unwrap_or(rules.len());
        rules.insert(pos, rule);
    }

    /// Remove a rule by id
    pub fn remove_rule(&self, id: u64) -> Option<PluginRule> {
        let mut rules = self.rules.write().unwrap();
        rules
            .iter()
            .position(|r| r.id == id)
            .map(|pos| rules.remove(pos))
    }

    /// List all rules sorted by priority
    pub fn list_rules(&self) -> Vec<PluginRule> {
        self.rules.read().unwrap().clone()
    }

    /// Create from YAML file
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let file: RuleFile = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
        let mut rules = file.into_rules(0);
        rules.sort_by_key(|r| r.priority);
        Ok(Self {
            rules: std::sync::RwLock::new(rules),
        })
    }

    /// Reload rules from file
    pub fn reload(&self, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let file: RuleFile = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
        let mut new_rules = file.into_rules(0);
        new_rules.sort_by_key(|r| r.priority);
        let mut rules = self.rules.write().unwrap();
        *rules = new_rules;
        Ok(())
    }

    /// Watch file for changes (auto-reload on modify).
    /// Takes `Arc<Self>` so the watcher thread holds a reference.
    pub fn watch(engine: &Arc<RuleEngine>, path: &Path) -> Result<(), String> {
        let engine_clone = Arc::clone(engine);
        let path_owned = path.to_path_buf();

        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if res.is_ok() {
                    let _ = tx.send(());
                }
            })
            .map_err(|e| format!("Watcher failed: {}", e))?;

        watcher
            .watch(&path_owned, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;

        // Fix 1: move watcher into the spawned thread so it stays alive
        std::thread::spawn(move || {
            let _watcher = watcher; // keep alive for thread lifetime
            let mut last_reload = std::time::Instant::now() - std::time::Duration::from_secs(1);
            loop {
                match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(()) => {
                        let now = std::time::Instant::now();
                        if now.duration_since(last_reload) > std::time::Duration::from_millis(500) {
                            last_reload = now;
                            if let Err(e) = engine_clone.reload(&path_owned) {
                                eprintln!("Rule reload failed: {}", e);
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Ok(())
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(
        host: &str,
        method: &str,
        scheme: &str,
        path: &str,
        headers: Vec<(String, String)>,
    ) -> InterceptedRequest {
        InterceptedRequest {
            host: host.into(),
            method: method.into(),
            scheme: scheme.into(),
            path: path.into(),
            req_headers: headers,
            ..Default::default()
        }
    }

    #[test]
    fn test_domain_suffix_match() {
        let pattern = RulePattern::DomainSuffix("weixin.qq.com".into());
        let request = make_request(
            "api.weixin.qq.com",
            "GET",
            "https",
            "/cgi-bin/token",
            vec![],
        );
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_domain_suffix_no_match() {
        let pattern = RulePattern::DomainSuffix("weixin.qq.com".into());
        let request = make_request("api.douyin.com", "GET", "https", "/", vec![]);
        assert!(!pattern.matches(&request));
    }

    // Fix 2 test: DomainSuffix with "*. " wildcard prefix
    #[test]
    fn test_domain_suffix_with_wildcard_prefix() {
        let pattern = RulePattern::DomainSuffix("*.weixin.qq.com".into());
        let request = make_request("api.weixin.qq.com", "GET", "https", "/", vec![]);
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_domain_keyword_match() {
        let pattern = RulePattern::DomainKeyword("weixin".into());
        let request = make_request("api.weixin.qq.com", "GET", "https", "/", vec![]);
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_url_pattern_match() {
        let pattern = RulePattern::UrlPattern {
            method: Some("POST".into()),
            scheme: None,
            host: Some("api.example.com".into()),
            path: Some("/upload/*".into()),
        };
        let request = make_request(
            "api.example.com",
            "POST",
            "https",
            "/upload/file.jpg",
            vec![],
        );
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_header_match() {
        let pattern = RulePattern::Header {
            key: "Authorization".into(),
            value: "Bearer *".into(),
        };
        let request = make_request(
            "api.example.com",
            "GET",
            "https",
            "/",
            vec![("Authorization".into(), "Bearer token123".into())],
        );
        assert!(pattern.matches(&request));
    }

    // Fix 3 test: header keys must be case-insensitive per RFC 7230
    #[test]
    fn test_header_case_insensitive() {
        let pattern = RulePattern::Header {
            key: "content-type".into(),
            value: "application/json".into(),
        };
        let request = make_request(
            "api.example.com",
            "GET",
            "https",
            "/",
            vec![("Content-Type".into(), "application/json".into())],
        );
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_wildcard_match_segment() {
        assert!(wildcard_match("/upload/*", "/upload/file.jpg"));
        assert!(!wildcard_match("/upload/*", "/upload/deep/file.jpg"));
        assert!(wildcard_match("*", "anything"));
        assert!(wildcard_match("/a/*/c", "/a/b/c"));
    }

    // --- Task 2: PluginRule + RuleEngine ---

    #[test]
    fn test_plugin_rule_priority_ordering() {
        let engine = RuleEngine::new();
        engine.add_rule(PluginRule {
            id: 1,
            name: "A".into(),
            pattern: RulePattern::DomainSuffix("qq.com".into()),
            plugin_name: "plugin-a".into(),
            priority: 100,
            enabled: true,
        });
        engine.add_rule(PluginRule {
            id: 2,
            name: "B".into(),
            pattern: RulePattern::DomainSuffix("weixin.qq.com".into()),
            plugin_name: "plugin-b".into(),
            priority: 50,
            enabled: true,
        });
        engine.add_rule(PluginRule {
            id: 3,
            name: "C".into(),
            pattern: RulePattern::DomainSuffix("qq.com".into()),
            plugin_name: "plugin-c".into(),
            priority: 100,
            enabled: true,
        });

        let rules = engine.list_rules();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].plugin_name, "plugin-b"); // priority 50
        assert_eq!(rules[1].plugin_name, "plugin-a"); // priority 100, inserted first
        assert_eq!(rules[2].plugin_name, "plugin-c"); // priority 100, inserted later
    }

    #[test]
    fn test_rule_engine_match_first() {
        let engine = RuleEngine::new();
        engine.add_rule(PluginRule {
            id: 1,
            name: "A".into(),
            pattern: RulePattern::DomainSuffix("qq.com".into()),
            plugin_name: "plugin-a".into(),
            priority: 100,
            enabled: true,
        });
        engine.add_rule(PluginRule {
            id: 2,
            name: "B".into(),
            pattern: RulePattern::DomainSuffix("weixin.qq.com".into()),
            plugin_name: "plugin-b".into(),
            priority: 50,
            enabled: true,
        });

        let request = make_request("api.weixin.qq.com", "GET", "https", "/", vec![]);
        let matched = engine.match_request(&request);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().plugin_name, "plugin-b"); // higher priority
    }

    #[test]
    fn test_rule_engine_disabled_rule_skipped() {
        let engine = RuleEngine::new();
        engine.add_rule(PluginRule {
            id: 1,
            name: "A".into(),
            pattern: RulePattern::DomainSuffix("qq.com".into()),
            plugin_name: "plugin-a".into(),
            priority: 50,
            enabled: false,
        });
        engine.add_rule(PluginRule {
            id: 2,
            name: "B".into(),
            pattern: RulePattern::DomainSuffix("qq.com".into()),
            plugin_name: "plugin-b".into(),
            priority: 100,
            enabled: true,
        });

        let request = make_request("api.qq.com", "GET", "https", "/", vec![]);
        let matched = engine.match_request(&request);
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().plugin_name, "plugin-b");
    }

    #[test]
    fn test_rule_engine_remove_rule() {
        let engine = RuleEngine::new();
        engine.add_rule(PluginRule {
            id: 1,
            name: "A".into(),
            pattern: RulePattern::DomainSuffix("qq.com".into()),
            plugin_name: "plugin-a".into(),
            priority: 100,
            enabled: true,
        });
        assert_eq!(engine.list_rules().len(), 1);
        let removed = engine.remove_rule(1);
        assert!(removed.is_some());
        assert_eq!(engine.list_rules().len(), 0);
        assert!(engine.remove_rule(99).is_none());
    }

    // --- Task 3: YAML loading ---

    #[test]
    fn test_rule_engine_from_yaml() {
        let yaml = r#"
rules:
  - name: WeChat
    pattern:
      type: DomainSuffix
      value: "*.weixin.qq.com"
    plugin: wechat-plugin
    priority: 100
    enabled: true
  - name: Upload
    pattern:
      type: UrlPattern
      method: POST
      path: "*/upload/*"
    plugin: upload-plugin
    priority: 50
    enabled: true
"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let rule_file = temp_dir.path().join("rules.yaml");
        std::fs::write(&rule_file, yaml).unwrap();

        let engine = RuleEngine::from_file(&rule_file).unwrap();
        let rules = engine.list_rules();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name, "Upload"); // priority 50 first
        assert_eq!(rules[1].name, "WeChat");
    }

    #[test]
    fn test_rule_engine_from_yaml_invalid() {
        let temp_dir = tempfile::tempdir().unwrap();
        let rule_file = temp_dir.path().join("bad.yaml");
        std::fs::write(&rule_file, "not: valid: yaml: [").unwrap();
        assert!(RuleEngine::from_file(&rule_file).is_err());
    }

    #[test]
    fn test_rule_engine_reload() {
        let yaml_a = r#"
rules:
  - name: RuleA
    pattern:
      type: DomainSuffix
      value: "example.com"
    plugin: plugin-a
"#;
        let yaml_b = r#"
rules:
  - name: RuleB
    pattern:
      type: DomainKeyword
      value: "test"
    plugin: plugin-b
"#;
        let temp_dir = tempfile::tempdir().unwrap();
        let rule_file = temp_dir.path().join("rules.yaml");
        std::fs::write(&rule_file, yaml_a).unwrap();

        let engine = RuleEngine::from_file(&rule_file).unwrap();
        assert_eq!(engine.list_rules()[0].name, "RuleA");

        std::fs::write(&rule_file, yaml_b).unwrap();
        engine.reload(&rule_file).unwrap();
        assert_eq!(engine.list_rules()[0].name, "RuleB");
    }

    #[test]
    fn test_rule_engine_match_all() {
        let engine = RuleEngine::new();
        engine.add_rule(PluginRule {
            id: 1,
            name: "A".into(),
            pattern: RulePattern::DomainSuffix("qq.com".into()),
            plugin_name: "plugin-a".into(),
            priority: 100,
            enabled: true,
        });
        engine.add_rule(PluginRule {
            id: 2,
            name: "B".into(),
            pattern: RulePattern::DomainSuffix("qq.com".into()),
            plugin_name: "plugin-b".into(),
            priority: 50,
            enabled: true,
        });

        let request = make_request("api.qq.com", "GET", "https", "/", vec![]);
        let matched = engine.match_all(&request);
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].plugin_name, "plugin-b"); // priority 50 first
        assert_eq!(matched[1].plugin_name, "plugin-a");
    }
}
