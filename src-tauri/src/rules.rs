//! YAML rule engine with hot reload.
//!
//! Rules are loaded from ~/.proxybot/rules/*.yaml
//! Supports: DOMAIN, DOMAIN-SUFFIX, IP-CIDR, GEOIP, RULE-SET
//! Actions: DIRECT, PROXY, REJECT
//! File watcher triggers hot-reload within 2 seconds of file save.

use ipnetwork::IpNetwork;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::State;
use tokio::sync::mpsc;

/// Rule action types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RuleAction {
    Direct,
    Proxy,
    Reject,
}

impl std::fmt::Display for RuleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleAction::Direct => write!(f, "DIRECT"),
            RuleAction::Proxy => write!(f, "PROXY"),
            RuleAction::Reject => write!(f, "REJECT"),
        }
    }
}

/// Rule pattern types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RulePattern {
    Domain,
    DomainSuffix,
    #[serde(rename = "DOMAIN-KEYWORD")]
    DomainKeyword,
    IpCidr,
    Geoip,
    RuleSet,
}

impl std::fmt::Display for RulePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RulePattern::Domain => write!(f, "DOMAIN"),
            RulePattern::DomainSuffix => write!(f, "DOMAIN-SUFFIX"),
            RulePattern::DomainKeyword => write!(f, "DOMAIN-KEYWORD"),
            RulePattern::IpCidr => write!(f, "IP-CIDR"),
            RulePattern::Geoip => write!(f, "GEOIP"),
            RulePattern::RuleSet => write!(f, "RULE-SET"),
        }
    }
}

/// A single routing rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub pattern: RulePattern,
    pub value: String,
    pub action: RuleAction,
}

/// Raw YAML structure for a single rule file.
#[derive(Debug, Deserialize, Serialize)]
struct RuleFile {
    rules: Vec<RuleEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RuleEntry {
    pattern: String,
    value: String,
    action: String,
}

impl RuleEntry {
    fn to_rule(&self) -> Option<Rule> {
        let pattern = match self.pattern.to_uppercase().as_str() {
            "DOMAIN" => RulePattern::Domain,
            "DOMAIN-SUFFIX" => RulePattern::DomainSuffix,
            "DOMAIN-KEYWORD" => RulePattern::DomainKeyword,
            "IP-CIDR" => RulePattern::IpCidr,
            "GEOIP" => RulePattern::Geoip,
            "RULE-SET" => RulePattern::RuleSet,
            _ => {
                log::warn!("Unknown rule pattern: {}", self.pattern);
                return None;
            }
        };

        let action = match self.action.to_uppercase().as_str() {
            "DIRECT" => RuleAction::Direct,
            "PROXY" => RuleAction::Proxy,
            "REJECT" => RuleAction::Reject,
            _ => {
                log::warn!("Unknown rule action: {}", self.action);
                return None;
            }
        };

        Some(Rule {
            pattern,
            value: self.value.clone(),
            action,
        })
    }
}

/// Get the rules directory path.
fn get_rules_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".proxybot").join("rules")
}

/// Ensure the rules directory exists.
fn ensure_rules_dir() -> std::io::Result<PathBuf> {
    let dir = get_rules_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Load all rules from YAML files in the rules directory.
fn load_rules_from_dir(dir: &PathBuf) -> Vec<Rule> {
    let mut all_rules = Vec::new();

    if !dir.exists() {
        log::info!("Rules directory does not exist: {:?}", dir);
        return all_rules;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            log::error!("Failed to read rules directory: {}", e);
            return all_rules;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_yaml::from_str::<RuleFile>(&content) {
                        Ok(rule_file) => {
                            for entry in rule_file.rules {
                                if let Some(rule) = entry.to_rule() {
                                    all_rules.push(rule);
                                }
                            }
                            log::info!("Loaded rules from {:?}", path);
                        }
                        Err(e) => {
                            log::warn!("Failed to parse {:?}: {}", path, e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read {:?}: {}", path, e);
                }
            }
        }
    }

    log::info!("Total rules loaded: {}", all_rules.len());
    all_rules
}

/// Rule engine state with hot reload.
pub struct RulesEngine {
    rules: Mutex<Vec<Rule>>,
    watcher_handle: Mutex<Option<RecommendedWatcher>>,
}

impl RulesEngine {
    pub fn new() -> Self {
        let engine = Self {
            rules: Mutex::new(Vec::new()),
            watcher_handle: Mutex::new(None),
        };
        engine.reload();
        engine
    }

    /// Reload rules from disk.
    pub fn reload(&self) {
        let dir = get_rules_dir();
        let rules = load_rules_from_dir(&dir);
        *self.rules.lock().unwrap() = rules;
        log::info!("Rules reloaded");
    }

    /// Start file watching for hot reload.
    pub fn start_watcher(self: Arc<Self>) {
        let (tx, mut rx) = mpsc::channel(100);
        let rules_dir = get_rules_dir();

        // Ensure directory exists first
        if let Err(e) = ensure_rules_dir() {
            log::error!("Failed to create rules directory: {}", e);
            return;
        }

        let tx_clone = tx.clone();
        let mut watcher = match RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                        let _ = tx_clone.blocking_send(event);
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        ) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to create file watcher: {}", e);
                return;
            }
        };

        if let Err(e) = watcher.watch(&rules_dir, RecursiveMode::NonRecursive) {
            log::error!("Failed to watch rules directory: {}", e);
            return;
        }

        log::info!("File watcher started for {:?}", rules_dir);

        *self.watcher_handle.lock().unwrap() = Some(watcher);

        // Spawn a task to handle file events
        let rules_engine = Arc::new(self);
        tokio::spawn(async move {
            let reload_delay = Duration::from_millis(500);
            let mut pending_reload = false;

            loop {
                tokio::select! {
                    Some(_event) = rx.recv() => {
                        pending_reload = true;
                    }
                    _ = tokio::time::sleep(reload_delay) => {
                        if pending_reload {
                            rules_engine.reload();
                            pending_reload = false;
                        }
                    }
                }
            }
        });
    }

    /// Match a host (and optionally IP) against the rules.
    /// Returns the matched action, or None if no rule matches.
    pub fn match_host(&self, host: &str, ip: Option<IpAddr>) -> Option<RuleAction> {
        let rules = self.rules.lock().unwrap();
        for rule in rules.iter() {
            if let Some(action) = self::match_rule(rule, host, ip) {
                return Some(action);
            }
        }
        None
    }

    /// Get all rules.
    pub fn get_rules(&self) -> Vec<Rule> {
        self.rules.lock().unwrap().clone()
    }

    /// Delete a rule from a file (internal, non-Tauri).
    pub fn delete_rule(&self, rule: &Rule, filename: &str) -> Result<(), String> {
        let dir = get_rules_dir();
        let path = dir.join(filename);

        if !path.exists() {
            return Err("File not found".to_string());
        }

        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut rule_file: RuleFile = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;

        // Remove the rule (first match)
        rule_file.rules.retain(|entry| {
            !(entry.pattern == rule.pattern.to_string()
                && entry.value == rule.value
                && entry.action == rule.action.to_string())
        });

        let yaml = serde_yaml::to_string(&rule_file).map_err(|e| e.to_string())?;
        fs::write(&path, yaml).map_err(|e| e.to_string())?;

        self.reload();
        Ok(())
    }

    /// Save a rule to a file (non-Tauri internal version).
    pub fn save_rule_internal(&self, rule: Rule, filename: &str) -> Result<(), String> {
        ensure_rules_dir().map_err(|e| e.to_string())?;

        let dir = get_rules_dir();
        let path = dir.join(filename);

        // Load existing rules from that file if it exists
        let mut existing_rules: Vec<Rule> = Vec::new();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(rule_file) = serde_yaml::from_str::<RuleFile>(&content) {
                    for entry in rule_file.rules {
                        if let Some(r) = entry.to_rule() {
                            existing_rules.push(r);
                        }
                    }
                }
            }
        }

        // Add the new rule
        existing_rules.push(rule);

        // Serialize and save
        let rule_entries: Vec<RuleEntry> = existing_rules
            .iter()
            .map(|r| RuleEntry {
                pattern: r.pattern.to_string(),
                value: r.value.clone(),
                action: r.action.to_string(),
            })
            .collect();

        let file = RuleFile { rules: rule_entries };
        let yaml = serde_yaml::to_string(&file).map_err(|e| e.to_string())?;

        fs::write(&path, yaml).map_err(|e| e.to_string())?;

        // Reload rules
        self.reload();

        Ok(())
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Match a single rule against host/IP.
fn match_rule(rule: &Rule, host: &str, ip: Option<IpAddr>) -> Option<RuleAction> {
    match rule.pattern {
        RulePattern::Domain => {
            if host.eq_ignore_ascii_case(&rule.value) {
                Some(rule.action.clone())
            } else {
                None
            }
        }
        RulePattern::DomainSuffix => {
            if host.eq_ignore_ascii_case(&rule.value)
                || host.ends_with(&format!(".{}", &rule.value))
            {
                Some(rule.action.clone())
            } else {
                None
            }
        }
        RulePattern::DomainKeyword => {
            if host.to_lowercase().contains(&rule.value.to_lowercase()) {
                Some(rule.action.clone())
            } else {
                None
            }
        }
        RulePattern::IpCidr => {
            if let Some(client_ip) = ip {
                if let Ok(network) = rule.value.parse::<IpNetwork>() {
                    if network.contains(client_ip) {
                        return Some(rule.action.clone());
                    }
                }
            }
            None
        }
        RulePattern::Geoip | RulePattern::RuleSet => {
            // GeoIP and RULE-SET require external data sources
            // For now, log and skip
            log::debug!("GeoIP/RULE-SET not yet implemented: {} {}", rule.pattern, rule.value);
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Get all loaded rules.
#[tauri::command]
pub fn get_rules(engine: State<'_, Arc<RulesEngine>>) -> Vec<Rule> {
    engine.get_rules()
}

/// Save a rule to a YAML file.
#[tauri::command]
pub fn save_rule(
    engine: State<'_, Arc<RulesEngine>>,
    rule: Rule,
    filename: String,
) -> Result<(), String> {
    ensure_rules_dir().map_err(|e| e.to_string())?;

    let dir = get_rules_dir();
    let path = dir.join(&filename);

    // Load existing rules from that file if it exists
    let mut existing_rules: Vec<Rule> = Vec::new();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(rule_file) = serde_yaml::from_str::<RuleFile>(&content) {
                for entry in rule_file.rules {
                    if let Some(r) = entry.to_rule() {
                        existing_rules.push(r);
                    }
                }
            }
        }
    }

    // Add the new rule
    existing_rules.push(rule);

    // Serialize and save
    let rule_entries: Vec<RuleEntry> = existing_rules
        .iter()
        .map(|r| RuleEntry {
            pattern: r.pattern.to_string(),
            value: r.value.clone(),
            action: r.action.to_string(),
        })
        .collect();

    let file = RuleFile { rules: rule_entries };
    let yaml = serde_yaml::to_string(&file).map_err(|e| e.to_string())?;

    fs::write(&path, yaml).map_err(|e| e.to_string())?;

    // Reload rules
    engine.reload();

    Ok(())
}

/// Delete a rule from a file.
#[tauri::command]
pub fn delete_rule(
    engine: State<'_, Arc<RulesEngine>>,
    rule: Rule,
    filename: String,
) -> Result<(), String> {
    let dir = get_rules_dir();
    let path = dir.join(&filename);

    if !path.exists() {
        return Err("File not found".to_string());
    }

    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut rule_file: RuleFile = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;

    // Remove the rule (first match)
    rule_file.rules.retain(|entry| {
        !(entry.pattern == rule.pattern.to_string()
            && entry.value == rule.value
            && entry.action == rule.action.to_string())
    });

    let yaml = serde_yaml::to_string(&rule_file).map_err(|e| e.to_string())?;
    fs::write(&path, yaml).map_err(|e| e.to_string())?;

    engine.reload();

    Ok(())
}

/// Reorder rules within a file.
#[tauri::command]
pub fn reorder_rules(
    engine: State<'_, Arc<RulesEngine>>,
    rules: Vec<Rule>,
    filename: String,
) -> Result<(), String> {
    ensure_rules_dir().map_err(|e| e.to_string())?;

    let path = get_rules_dir().join(&filename);

    let rule_entries: Vec<RuleEntry> = rules
        .iter()
        .map(|r| RuleEntry {
            pattern: r.pattern.to_string(),
            value: r.value.clone(),
            action: r.action.to_string(),
        })
        .collect();

    let file = RuleFile { rules: rule_entries };
    let yaml = serde_yaml::to_string(&file).map_err(|e| e.to_string())?;

    fs::write(&path, yaml).map_err(|e| e.to_string())?;

    engine.reload();

    Ok(())
}

/// List available rule files.
#[tauri::command]
pub fn list_rule_files() -> Vec<String> {
    let dir = get_rules_dir();
    if !dir.exists() {
        return Vec::new();
    }

    fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s == "yaml")
                        .unwrap_or(false)
                })
                .filter_map(|e| e.file_name().to_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Match a host against rules (for testing/debug).
#[tauri::command]
pub fn match_host(
    engine: State<'_, Arc<RulesEngine>>,
    host: String,
    ip: Option<String>,
) -> Option<RuleAction> {
    let ip_addr = ip.and_then(|s| s.parse().ok());
    engine.match_host(&host, ip_addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_exact_match() {
        let rule = Rule {
            pattern: RulePattern::Domain,
            value: "example.com".to_string(),
            action: RuleAction::Direct,
        };
        assert_eq!(match_rule(&rule, "example.com", None), Some(RuleAction::Direct));
        assert_eq!(match_rule(&rule, "EXAMPLE.COM", None), Some(RuleAction::Direct));
        assert_eq!(match_rule(&rule, "sub.example.com", None), None);
    }

    #[test]
    fn test_domain_suffix_match() {
        let rule = Rule {
            pattern: RulePattern::DomainSuffix,
            value: "example.com".to_string(),
            action: RuleAction::Proxy,
        };
        assert_eq!(match_rule(&rule, "example.com", None), Some(RuleAction::Proxy));
        assert_eq!(match_rule(&rule, "sub.example.com", None), Some(RuleAction::Proxy));
        assert_eq!(match_rule(&rule, "example.com.evil.com", None), None);
    }

    #[test]
    fn test_domain_keyword_match() {
        let rule = Rule {
            pattern: RulePattern::DomainKeyword,
            value: "wechat".to_string(),
            action: RuleAction::Direct,
        };
        assert_eq!(match_rule(&rule, "api.wechat.com", None), Some(RuleAction::Direct));
        assert_eq!(match_rule(&rule, "wechat-api.example.com", None), Some(RuleAction::Direct));
        assert_eq!(match_rule(&rule, "chat.example.com", None), None);
    }

    #[test]
    fn test_ip_cidr_match() {
        let rule = Rule {
            pattern: RulePattern::IpCidr,
            value: "10.0.0.0/8".to_string(),
            action: RuleAction::Reject,
        };
        assert_eq!(
            match_rule(&rule, "host.example.com", Some("10.1.2.3".parse().unwrap())),
            Some(RuleAction::Reject)
        );
        assert_eq!(
            match_rule(&rule, "host.example.com", Some("192.168.1.1".parse().unwrap())),
            None
        );
    }
}
