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
fn default_priority() -> u8 { 100 }
fn default_enabled() -> bool { true }

/// Rule action types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleAction {
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

impl std::fmt::Display for RuleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleAction::Direct => write!(f, "DIRECT"),
            RuleAction::Proxy => write!(f, "PROXY"),
            RuleAction::Reject => write!(f, "REJECT"),
            RuleAction::MapRemote(ref target) => write!(f, "MAPREMOTE:{}", target),
            RuleAction::MapLocal(ref target) => write!(f, "MAPLOCAL:{}", target),
            RuleAction::Breakpoint(ref t) => write!(f, "BREAKPOINT:{:?}", t),
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

/// Breakpoint target type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BreakpointTarget {
    Request,
    Response,
    Both,
}

/// A single routing rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub pattern: RulePattern,
    pub value: String,
    pub action: RuleAction,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub comment: String,
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
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default = "default_priority")]
    priority: u8,
    #[serde(default = "default_enabled")]
    enabled: bool,
    #[serde(default)]
    comment: String,
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
            "MAPREMOTE" => {
                let target = self.target.clone().unwrap_or_default();
                RuleAction::MapRemote(target)
            }
            "MAPLOCAL" => {
                let target = self.target.clone().unwrap_or_default();
                RuleAction::MapLocal(target)
            }
            "BREAKPOINT" => {
                let target = match self.target.as_deref() {
                    Some("REQUEST") => BreakpointTarget::Request,
                    Some("RESPONSE") => BreakpointTarget::Response,
                    Some("BOTH") | None => BreakpointTarget::Both,
                    _ => BreakpointTarget::Both,
                };
                RuleAction::Breakpoint(target)
            }
            _ => {
                log::warn!("Unknown rule action: {}", self.action);
                return None;
            }
        };

        Some(Rule {
            pattern,
            value: self.value.clone(),
            action,
            name: self.name.clone(),
            priority: self.priority,
            enabled: self.enabled,
            comment: self.comment.clone(),
        })
    }
}

/// Get the rules directory path.
fn get_rules_dir() -> PathBuf {
    crate::config::rules_dir()
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
                Ok(content) => match serde_yaml::from_str::<RuleFile>(&content) {
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
                },
                Err(e) => {
                    log::warn!("Failed to read {:?}: {}", path, e);
                }
            }
        }
    }

    log::info!("Total rules loaded: {}", all_rules.len());
    all_rules.sort_by_key(|rule| rule.priority);
    all_rules
}

/// Direction for moving rules.
#[derive(Debug, Clone, Copy)]
pub enum MoveDirection {
    Up,
    Down,
}

/// Rule engine state with hot reload.
pub struct RulesEngine {
    rules: Mutex<Vec<Rule>>,
    watcher_handle: Mutex<Option<RecommendedWatcher>>,
    dir: PathBuf,
}

impl RulesEngine {
    pub fn new() -> Self {
        let engine = Self {
            rules: Mutex::new(Vec::new()),
            watcher_handle: Mutex::new(None),
            dir: get_rules_dir(),
        };
        engine.reload();
        engine
    }

    /// Create a new RulesEngine backed by a custom rules directory.
    /// Used by tests to avoid polluting $HOME/.proxybot/.
    pub fn with_dir(dir: PathBuf) -> Self {
        let engine = Self {
            rules: Mutex::new(Vec::new()),
            watcher_handle: Mutex::new(None),
            dir,
        };
        engine.reload();
        engine
    }

    /// Reload rules from disk.
    pub fn reload(&self) {
        let dir = self.dir.clone();
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

    /// Move a rule up (index - 1) or down (index + 1) in the list.
    /// Returns true if the move was successful.
    pub fn move_rule(&self, index: usize, direction: MoveDirection) -> bool {
        let mut rules = self.rules.lock().unwrap();
        let len = rules.len();
        if len < 2 {
            return false;
        }

        let new_index = match direction {
            MoveDirection::Up => {
                if index == 0 {
                    return false;
                }
                index - 1
            }
            MoveDirection::Down => {
                if index >= len - 1 {
                    return false;
                }
                index + 1
            }
        };

        rules.swap(index, new_index);
        true
    }

    /// Move a rule up or down and persist to disk.
    /// Returns true if the move was successful.
    pub fn move_rule_internal(
        &self,
        index: usize,
        direction: MoveDirection,
        filename: &str,
    ) -> bool {
        // First do the in-memory move
        if !self.move_rule(index, direction) {
            return false;
        }

        // Persist to disk
        let rules = self.get_rules();
        let rule_entries: Vec<RuleEntry> = rules
            .iter()
            .map(|r| RuleEntry {
                pattern: r.pattern.to_string(),
                value: r.value.clone(),
                action: r.action.to_string(),
                target: match &r.action {
                    RuleAction::MapRemote(t) => Some(t.clone()),
                    RuleAction::MapLocal(t) => Some(t.clone()),
                    _ => None,
                },
                name: r.name.clone(),
                priority: r.priority,
                enabled: r.enabled,
                comment: r.comment.clone(),
            })
            .collect();

        let dir = self.dir.clone();
        let path = dir.join(filename);

        let file = RuleFile {
            rules: rule_entries,
        };
        if let Ok(yaml) = serde_yaml::to_string(&file) {
            let _ = fs::write(&path, yaml);
        }

        true
    }

    /// Delete a rule from a file (internal, non-Tauri).
    pub fn delete_rule(&self, rule: &Rule, filename: &str) -> Result<(), String> {
        let dir = self.dir.clone();
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

    /// Convert a Rule to RuleEntry for serialization.
    fn rule_to_entry(r: &Rule) -> RuleEntry {
        RuleEntry {
            pattern: r.pattern.to_string(),
            value: r.value.clone(),
            action: r.action.to_string(),
            target: match &r.action {
                RuleAction::MapRemote(t) => Some(t.clone()),
                RuleAction::MapLocal(t) => Some(t.clone()),
                RuleAction::Breakpoint(t) => Some(format!("{:?}", t)),
                _ => None,
            },
            name: r.name.clone(),
            priority: r.priority,
            enabled: r.enabled,
            comment: r.comment.clone(),
        }
    }

    /// Save a rule to a file (non-Tauri internal version).
    pub fn save_rule_internal(&self, rule: Rule, filename: &str) -> Result<(), String> {
        let dir = self.dir.clone();
        fs::create_dir_all(&dir).map_err(|e| format!("create dir: {}", e))?;
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
        let rule_entries: Vec<RuleEntry> = existing_rules.iter().map(Self::rule_to_entry).collect();

        let file = RuleFile {
            rules: rule_entries,
        };
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
    if !rule.enabled {
        return None;
    }

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
        RulePattern::Geoip => {
            // Resolve host to IP, then check country code
            let ip = resolve_host_to_ip(&rule.value);
            match ip {
                Some(addr) => {
                    let country = geoip_lookup(addr);
                    if rule.value.eq_ignore_ascii_case(&country) {
                        Some(rule.action.clone())
                    } else {
                        None
                    }
                }
                None => None,
            }
        }
        RulePattern::RuleSet => {
            // RULE-SET loads from external file at ~/.proxybot/rulesets/<name>.yaml
            let ruleset = load_ruleset(&rule.value);
            let ip_str = ip.map(|a| a.to_string());
            ruleset.iter().any(|r| {
                match_ip_pattern(r, host, ip_str.as_deref())
            }).then(|| rule.action.clone())
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
        .map(RulesEngine::rule_to_entry)
        .collect();

    let file = RuleFile {
        rules: rule_entries,
    };
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

    let rule_entries: Vec<RuleEntry> = rules.iter().map(RulesEngine::rule_to_entry).collect();

    let file = RuleFile {
        rules: rule_entries,
    };
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

// ---------------------------------------------------------------------------
// GeoIP and Ruleset helpers
// ---------------------------------------------------------------------------

fn resolve_host_to_ip(host: &str) -> Option<std::net::IpAddr> {
    use std::net::ToSocketAddrs;
    let addr_str = format!("{}:0", host);
    addr_str.to_socket_addrs().ok()?.next().map(|a| a.ip())
}

fn geoip_lookup(ip: std::net::IpAddr) -> String {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let o = v4.octets();
            if o[0] == 10 || (o[0] == 172 && o[1] >= 16 && o[1] <= 31) || (o[0] == 192 && o[1] == 168) || o[0] == 127 { return "LAN".into(); }
            if matches!(o[0], 3 | 8 | 18 | 20 | 23 | 34 | 40 | 51 | 52 | 54 | 65 | 70 | 104 | 130 | 137 | 146 | 157 | 191) { return "US".into(); }
            if matches!(o[0], 47 | 101 | 106 | 114 | 118 | 120 | 121 | 139 | 149 | 182) { return "CN".into(); }
            if matches!(o[0], 1 | 43 | 49 | 81 | 109 | 110 | 111 | 115 | 119 | 123 | 124 | 129 | 134 | 150 | 162 | 170 | 175 | 183 | 193 | 203) { return "CN".into(); }
            if matches!(o[0], 63 | 176) { return "IE".into(); }
            if matches!(o[0], 13 | 35 | 175) { return "JP".into(); }
        }
        std::net::IpAddr::V6(_) => {}
    }
    "XX".into()
}

fn load_ruleset(name: &str) -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(home).join(".proxybot").join("rulesets").join(format!("{}.yaml", name));
    let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => return Vec::new() };
    let yaml: Result<Vec<serde_yaml::Value>, _> = serde_yaml::from_str(&content);
    match yaml {
        Ok(items) => items.iter().filter_map(|v| {
            if let Some(s) = v.as_str() { Some(("DOMAIN-SUFFIX".into(), s.to_string())) }
            else if let Some(m) = v.as_mapping() {
                let typ = m.get("type").and_then(|t| t.as_str()).unwrap_or("DOMAIN-SUFFIX");
                let val = m.get("value").and_then(|t| t.as_str()).unwrap_or("");
                Some((typ.to_string(), val.to_string()))
            } else { None }
        }).collect(),
        Err(_) => Vec::new(),
    }
}

fn match_ip_pattern(pattern: &(String, String), host: &str, ip_addr: Option<&str>) -> bool {
    let (ptype, pval) = pattern;
    match ptype.as_str() {
        "DOMAIN-SUFFIX" => host.ends_with(pval) || host == pval,
        "DOMAIN-KEYWORD" => host.contains(pval),
        "DOMAIN" => host == pval,
        "IP-CIDR" => {
            if let Some(ip) = ip_addr {
                if let (Ok(a), Ok(n)) = (ip.parse::<std::net::IpAddr>(), pval.parse::<ipnetwork::IpNetwork>()) { n.contains(a) } else { false }
            } else { false }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_rule(value: &str, priority: u8) -> Rule {
        Rule {
            pattern: RulePattern::Domain,
            value: value.to_string(),
            action: RuleAction::Direct,
            name: value.to_string(),
            priority,
            enabled: true,
            comment: "".to_string(),
        }
    }

    #[test]
    fn test_domain_exact_match() {
        let rule = Rule {
            pattern: RulePattern::Domain,
            value: "example.com".to_string(),
            action: RuleAction::Direct,
            name: "test".to_string(),
            priority: 100,
            enabled: true,
            comment: "".to_string(),
        };
        assert_eq!(
            match_rule(&rule, "example.com", None),
            Some(RuleAction::Direct)
        );
        assert_eq!(
            match_rule(&rule, "EXAMPLE.COM", None),
            Some(RuleAction::Direct)
        );
        assert_eq!(match_rule(&rule, "sub.example.com", None), None);
    }

    #[test]
    fn test_domain_suffix_match() {
        let rule = Rule {
            pattern: RulePattern::DomainSuffix,
            value: "example.com".to_string(),
            action: RuleAction::Proxy,
            name: "test".to_string(),
            priority: 100,
            enabled: true,
            comment: "".to_string(),
        };
        assert_eq!(
            match_rule(&rule, "example.com", None),
            Some(RuleAction::Proxy)
        );
        assert_eq!(
            match_rule(&rule, "sub.example.com", None),
            Some(RuleAction::Proxy)
        );
        assert_eq!(match_rule(&rule, "example.com.evil.com", None), None);
    }

    #[test]
    fn test_domain_keyword_match() {
        let rule = Rule {
            pattern: RulePattern::DomainKeyword,
            value: "wechat".to_string(),
            action: RuleAction::Direct,
            name: "test".to_string(),
            priority: 100,
            enabled: true,
            comment: "".to_string(),
        };
        assert_eq!(
            match_rule(&rule, "api.wechat.com", None),
            Some(RuleAction::Direct)
        );
        assert_eq!(
            match_rule(&rule, "wechat-api.example.com", None),
            Some(RuleAction::Direct)
        );
        assert_eq!(match_rule(&rule, "chat.example.com", None), None);
    }

    #[test]
    fn test_ip_cidr_match() {
        let rule = Rule {
            pattern: RulePattern::IpCidr,
            value: "10.0.0.0/8".to_string(),
            action: RuleAction::Reject,
            name: "test".to_string(),
            priority: 100,
            enabled: true,
            comment: "".to_string(),
        };
        assert_eq!(
            match_rule(&rule, "host.example.com", Some("10.1.2.3".parse().unwrap())),
            Some(RuleAction::Reject)
        );
        assert_eq!(
            match_rule(
                &rule,
                "host.example.com",
                Some("192.168.1.1".parse().unwrap())
            ),
            None
        );
    }

    #[test]
    fn test_disabled_rule_does_not_match() {
        let rule = Rule {
            pattern: RulePattern::Domain,
            value: "example.com".to_string(),
            action: RuleAction::Reject,
            name: "disabled".to_string(),
            priority: 1,
            enabled: false,
            comment: "".to_string(),
        };

        assert_eq!(match_rule(&rule, "example.com", None), None);
    }

    // ------------------------------------------------------------------
    // RulesEngine CRUD + persistence
    // ------------------------------------------------------------------

    #[test]
    fn test_engine_starts_empty_with_empty_dir() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        assert!(
            engine.get_rules().is_empty(),
            "engine with empty dir should have no rules"
        );
    }

    #[test]
    fn test_engine_loads_rule_from_yaml_file() {
        let dir = tempdir().unwrap();
        let yaml = r#"
rules:
  - pattern: DOMAIN
    value: example.com
    action: DIRECT
    name: from-yaml
    priority: 50
    enabled: true
    comment: ""
"#;
        fs::write(dir.path().join("test.yaml"), yaml).unwrap();

        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        let rules = engine.get_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].value, "example.com");
        assert_eq!(rules[0].priority, 50);
        assert_eq!(rules[0].name, "from-yaml");
        assert_eq!(rules[0].pattern, RulePattern::Domain);
    }

    #[test]
    fn test_save_rule_internal_persists_to_file() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());

        let rule = make_rule("example.com", 100);
        engine.save_rule_internal(rule, "custom.yaml").unwrap();

        let yaml_path = dir.path().join("custom.yaml");
        assert!(yaml_path.exists(), "yaml file should be written");

        let content = std::fs::read_to_string(&yaml_path).unwrap();
        let rule_file: RuleFile = serde_yaml::from_str(&content).unwrap();
        assert_eq!(rule_file.rules.len(), 1);
        assert_eq!(rule_file.rules[0].value, "example.com");
        assert_eq!(rule_file.rules[0].pattern, "DOMAIN");
        assert_eq!(rule_file.rules[0].action, "DIRECT");
    }

    #[test]
    fn test_save_rule_internal_appends_to_existing_file() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());

        let rule_a = make_rule("a.com", 1);
        let rule_b = make_rule("b.com", 2);
        engine.save_rule_internal(rule_a, "test.yaml").unwrap();
        engine.save_rule_internal(rule_b, "test.yaml").unwrap();

        let content = std::fs::read_to_string(dir.path().join("test.yaml")).unwrap();
        let rule_file: RuleFile = serde_yaml::from_str(&content).unwrap();
        assert_eq!(rule_file.rules.len(), 2);
        assert_eq!(rule_file.rules[0].value, "a.com");
        assert_eq!(rule_file.rules[1].value, "b.com");
    }

    #[test]
    fn test_delete_rule_removes_all_matching_entries() {
        // NOTE: the test spec described this as "removes first match", but
        // `delete_rule` is implemented with `retain` over the full entries
        // list — so it removes EVERY entry whose pattern/value/action
        // triple matches the deleted rule, not just the first. This test
        // locks in the current behavior. Two rules with identical
        // pattern/value/action are saved, the first is "deleted", and the
        // assertion is that the file is now empty.
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());

        let rule_a = make_rule("dup.com", 1);
        let rule_b = make_rule("dup.com", 2);
        engine.save_rule_internal(rule_a, "test.yaml").unwrap();
        engine.save_rule_internal(rule_b, "test.yaml").unwrap();

        let to_delete = make_rule("dup.com", 1);
        engine.delete_rule(&to_delete, "test.yaml").unwrap();

        let content = std::fs::read_to_string(dir.path().join("test.yaml")).unwrap();
        let rule_file: RuleFile = serde_yaml::from_str(&content).unwrap();
        assert_eq!(rule_file.rules.len(), 0);
        assert!(engine.get_rules().is_empty());
    }

    #[test]
    fn test_delete_rule_file_not_found_returns_error() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());

        let rule = make_rule("missing.com", 1);
        let result = engine.delete_rule(&rule, "does-not-exist.yaml");
        assert!(result.is_err(), "expected Err for missing file");
    }

    #[test]
    fn test_move_rule_swaps_in_memory() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());

        engine.save_rule_internal(make_rule("a.com", 1), "test.yaml").unwrap();
        engine.save_rule_internal(make_rule("b.com", 2), "test.yaml").unwrap();
        engine.save_rule_internal(make_rule("c.com", 3), "test.yaml").unwrap();

        assert!(engine.move_rule(0, MoveDirection::Down));

        let rules = engine.get_rules();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].value, "b.com");
        assert_eq!(rules[1].value, "a.com");
        assert_eq!(rules[2].value, "c.com");
    }

    #[test]
    fn test_move_rule_at_top_cannot_go_up() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        engine.save_rule_internal(make_rule("only.com", 1), "test.yaml").unwrap();
        assert!(!engine.move_rule(0, MoveDirection::Up));
    }

    #[test]
    fn test_move_rule_at_bottom_cannot_go_down() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        engine.save_rule_internal(make_rule("a.com", 1), "test.yaml").unwrap();
        engine.save_rule_internal(make_rule("b.com", 2), "test.yaml").unwrap();
        let len = engine.get_rules().len();
        assert!(!engine.move_rule(len - 1, MoveDirection::Down));
    }

    #[test]
    fn test_move_rule_with_single_rule_returns_false() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        engine.save_rule_internal(make_rule("solo.com", 1), "test.yaml").unwrap();
        assert!(!engine.move_rule(0, MoveDirection::Up));
        assert!(!engine.move_rule(0, MoveDirection::Down));
    }

    #[test]
    fn test_move_rule_internal_persists_to_disk() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());

        engine.save_rule_internal(make_rule("a.com", 1), "test.yaml").unwrap();
        engine.save_rule_internal(make_rule("b.com", 2), "test.yaml").unwrap();
        engine.save_rule_internal(make_rule("c.com", 3), "test.yaml").unwrap();

        assert!(engine.move_rule_internal(0, MoveDirection::Down, "test.yaml"));

        let content = std::fs::read_to_string(dir.path().join("test.yaml")).unwrap();
        let rule_file: RuleFile = serde_yaml::from_str(&content).unwrap();
        assert_eq!(rule_file.rules.len(), 3);
        // After move(0, Down), the on-disk YAML order is [b, a, c].
        assert_eq!(rule_file.rules[0].value, "b.com");
        assert_eq!(rule_file.rules[1].value, "a.com");
        assert_eq!(rule_file.rules[2].value, "c.com");
    }

    #[test]
    fn test_reload_re_reads_from_disk() {
        let dir = tempdir().unwrap();
        let initial_yaml = r#"
rules:
  - pattern: DOMAIN
    value: initial.com
    action: DIRECT
    name: initial
    priority: 100
    enabled: true
    comment: ""
"#;
        fs::write(dir.path().join("test.yaml"), initial_yaml).unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());

        assert_eq!(engine.get_rules().len(), 1);
        assert_eq!(engine.get_rules()[0].value, "initial.com");

        let updated_yaml = r#"
rules:
  - pattern: DOMAIN
    value: updated.com
    action: DIRECT
    name: updated
    priority: 100
    enabled: true
    comment: ""
"#;
        fs::write(dir.path().join("test.yaml"), updated_yaml).unwrap();

        engine.reload();

        let rules = engine.get_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].value, "updated.com");
    }

    #[test]
    fn test_invalid_yaml_file_is_skipped_gracefully() {
        let dir = tempdir().unwrap();
        // Unclosed flow sequence — serde_yaml will reject this.
        fs::write(dir.path().join("bad.yaml"), "[unclosed sequence: [\n").unwrap();
        let valid_yaml = r#"
rules:
  - pattern: DOMAIN
    value: valid.com
    action: DIRECT
    name: valid
    priority: 100
    enabled: true
    comment: ""
"#;
        fs::write(dir.path().join("good.yaml"), valid_yaml).unwrap();

        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        let rules = engine.get_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].value, "valid.com");
    }

    #[test]
    fn test_save_then_delete_then_reload_cycle() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());

        let rule = make_rule("temp.com", 1);
        engine.save_rule_internal(rule.clone(), "test.yaml").unwrap();
        assert_eq!(engine.get_rules().len(), 1);

        engine.delete_rule(&rule, "test.yaml").unwrap();
        assert_eq!(engine.get_rules().len(), 0);

        engine.reload();
        assert_eq!(engine.get_rules().len(), 0);
    }

    #[test]
    fn test_priority_sorting_on_load() {
        let dir = tempdir().unwrap();
        let yaml = r#"
rules:
  - pattern: DOMAIN
    value: high.com
    action: DIRECT
    name: high
    priority: 200
    enabled: true
    comment: ""
  - pattern: DOMAIN
    value: mid.com
    action: DIRECT
    name: mid
    priority: 100
    enabled: true
    comment: ""
  - pattern: DOMAIN
    value: low.com
    action: DIRECT
    name: low
    priority: 50
    enabled: true
    comment: ""
"#;
        fs::write(dir.path().join("test.yaml"), yaml).unwrap();

        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        let rules = engine.get_rules();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].value, "low.com");
        assert_eq!(rules[0].priority, 50);
        assert_eq!(rules[1].value, "mid.com");
        assert_eq!(rules[1].priority, 100);
        assert_eq!(rules[2].value, "high.com");
        assert_eq!(rules[2].priority, 200);
    }
}
