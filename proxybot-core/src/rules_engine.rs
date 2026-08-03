//! File-backed routing rules engine with hot reload.
//!
//! This Module owns the complete Routing Rule lifecycle: Rule File loading,
//! priority ordering, matching, file-scoped mutations, and filesystem reloads.
//! Desktop callers add only a transport Adapter; they do not reimplement rule
//! behavior.

use crate::types::{Rule, RuleAction, RuleEntry, RuleFile, RulePattern};
use notify::{Config, PollWatcher, RecursiveMode, Watcher};
use std::fs;
use std::io::Write;
use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use thiserror::Error;

/// The hostname/IP matching primitive shared by Routing Rules and Network
/// Condition rules. Higher-level patterns such as GEOIP and RULE-SET remain in
/// their owning Module.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum HostPattern {
    #[serde(rename = "DOMAIN")]
    Domain,
    #[serde(rename = "DOMAIN-SUFFIX")]
    DomainSuffix,
    #[serde(rename = "DOMAIN-KEYWORD")]
    DomainKeyword,
    #[serde(rename = "IP-CIDR")]
    IpCidr,
}

/// Failures exposed by the Rules Engine Interface.
#[derive(Debug, Error)]
pub enum RulesError {
    #[error("Rule filename must be a .yaml basename")]
    InvalidFilename,
    #[error("Rule file not found: {0}")]
    FileNotFound(String),
    #[error("Rule not found")]
    RuleNotFound,
    #[error("The rule being edited no longer exists")]
    EditConflict,
    #[error("Rule reorder indices ({from_index}, {to_index}) are out of bounds for {len} rules")]
    ReorderOutOfBounds {
        from_index: usize,
        to_index: usize,
        len: usize,
    },
    #[error("failed to {operation} {}: {source}", path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {}: {source}", path.display())]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("invalid rule entry in {}", path.display())]
    InvalidEntry { path: PathBuf },
    #[error("failed to watch Rule Files: {0}")]
    Watch(#[from] notify::Error),
}

/// Routing Rules Engine state shared by desktop, DNS, and proxy callers.
pub struct RulesEngine {
    rules: Mutex<Vec<Rule>>,
    mutations: Mutex<()>,
    watcher: Mutex<Option<PollWatcher>>,
    dir: PathBuf,
}

impl RulesEngine {
    /// Open the configured Rule File directory and load every valid file.
    pub fn new() -> Self {
        Self::with_dir(crate::config::rules_dir())
    }

    /// Open a specific Rule File directory.
    pub fn with_dir(dir: PathBuf) -> Self {
        let engine = Self {
            rules: Mutex::new(Vec::new()),
            mutations: Mutex::new(()),
            watcher: Mutex::new(None),
            dir,
        };
        engine.reload();
        engine
    }

    /// Replace the in-memory rules, ordered by Rule Priority.
    pub fn set_rules(&self, mut rules: Vec<Rule>) {
        rules.sort_by_key(|rule| rule.priority);
        *self.rules.lock().unwrap() = rules;
    }

    /// Add one in-memory rule and preserve evaluation order.
    pub fn add_rule(&self, rule: Rule) {
        let mut rules = self.rules.lock().unwrap();
        rules.push(rule);
        rules.sort_by_key(|rule| rule.priority);
    }

    /// Reload all valid Rule Files. Invalid files are logged and skipped so a
    /// single draft cannot take down the proxy.
    pub fn reload(&self) {
        self.set_rules(load_rules_from_dir(&self.dir));
        log::info!("Rules reloaded from {}", self.dir.display());
    }

    /// Watch the Rule File directory and reload after a 500ms quiet period.
    pub fn start_watcher(self: &Arc<Self>) -> Result<(), RulesError> {
        fs::create_dir_all(&self.dir)
            .map_err(|source| io_error("create rules directory", &self.dir, source))?;

        let (tx, rx) = mpsc::channel();
        let mut watcher = PollWatcher::new(
            move |result: Result<notify::Event, notify::Error>| {
                if result.is_ok() {
                    let _ = tx.send(());
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(250)),
        )?;
        watcher.watch(&self.dir, RecursiveMode::NonRecursive)?;
        *self.watcher.lock().unwrap() = Some(watcher);

        let engine = Arc::downgrade(self);
        std::thread::spawn(move || {
            while rx.recv().is_ok() {
                loop {
                    match rx.recv_timeout(Duration::from_millis(500)) {
                        Ok(()) => continue,
                        Err(mpsc::RecvTimeoutError::Timeout) => break,
                        Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    }
                }

                let Some(engine) = engine.upgrade() else {
                    return;
                };
                engine.reload();
            }
        });
        Ok(())
    }

    /// Match a host (and optionally a client IP) in Rule Priority order.
    pub fn match_host(&self, host: &str, client_ip: Option<IpAddr>) -> Option<RuleAction> {
        let rules = self.rules.lock().unwrap();
        rules
            .iter()
            .find(|rule| {
                if rule.pattern == RulePattern::RuleSet {
                    rule.enabled && self.rule_set_matches(&rule.value, host, client_ip)
                } else {
                    match_rule(rule, host, client_ip)
                }
            })
            .map(|rule| rule.action.clone())
    }

    /// Snapshot every loaded Routing Rule in evaluation order.
    pub fn get_rules(&self) -> Vec<Rule> {
        self.rules.lock().unwrap().clone()
    }

    /// Get one Rule File in evaluation order.
    pub fn get_rules_for_file(&self, filename: &str) -> Result<Vec<Rule>, RulesError> {
        let mut rules = self.read_rule_file(filename)?;
        rules.sort_by_key(|rule| rule.priority);
        Ok(rules)
    }

    /// List Rule Files deterministically.
    pub fn list_rule_files(&self) -> Result<Vec<String>, RulesError> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }

        let mut filenames = fs::read_dir(&self.dir)
            .map_err(|source| io_error("read rules directory", &self.dir, source))?
            .flatten()
            .filter(|entry| {
                entry.path().extension().and_then(|value| value.to_str()) == Some("yaml")
            })
            .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
            .collect::<Vec<_>>();
        filenames.sort();
        Ok(filenames)
    }

    /// Add a Routing Rule, or replace the exact rule being edited.
    pub fn save_rule(
        &self,
        rule: Rule,
        original_rule: Option<&Rule>,
        filename: &str,
    ) -> Result<(), RulesError> {
        let _mutation = self.mutations.lock().unwrap();
        let mut rules = self.read_rule_file(filename)?;
        if let Some(original) = original_rule {
            let index = rules
                .iter()
                .position(|candidate| candidate == original)
                .ok_or(RulesError::EditConflict)?;
            rules[index] = rule;
        } else {
            rules.push(rule);
        }
        self.write_rule_file(filename, &rules)
    }

    /// Delete one exact Routing Rule from a Rule File.
    pub fn delete_rule(&self, rule: &Rule, filename: &str) -> Result<(), RulesError> {
        let _mutation = self.mutations.lock().unwrap();
        let path = self.rule_path(filename)?;
        if !path.exists() {
            return Err(RulesError::FileNotFound(filename.to_owned()));
        }

        let mut rules = self.read_rule_file(filename)?;
        let index = rules
            .iter()
            .position(|candidate| candidate == rule)
            .ok_or(RulesError::RuleNotFound)?;
        rules.remove(index);
        self.write_rule_file(filename, &rules)
    }

    /// Move a Routing Rule within one Rule File and preserve the resulting
    /// evaluation order after reload.
    pub fn reorder_rules(
        &self,
        from_index: usize,
        to_index: usize,
        filename: &str,
    ) -> Result<(), RulesError> {
        let _mutation = self.mutations.lock().unwrap();
        let mut rules = self.get_rules_for_file(filename)?;
        if from_index >= rules.len() || to_index >= rules.len() {
            return Err(RulesError::ReorderOutOfBounds {
                from_index,
                to_index,
                len: rules.len(),
            });
        }
        if from_index == to_index {
            return Ok(());
        }

        let from_priority = rules[from_index].priority;
        let to_priority = rules[to_index].priority;
        rules[from_index].priority = to_priority;
        rules[to_index].priority = from_priority;
        rules.swap(from_index, to_index);
        self.write_rule_file(filename, &rules)
    }

    /// Compatibility helper for callers adding a rule without an edit token.
    pub fn save_rule_internal(&self, rule: Rule, filename: &str) -> Result<(), RulesError> {
        self.save_rule(rule, None, filename)
    }

    fn rule_path(&self, filename: &str) -> Result<PathBuf, RulesError> {
        let filename_path = Path::new(filename);
        let is_basename =
            filename_path.file_name().and_then(|value| value.to_str()) == Some(filename);
        let is_yaml = filename_path.extension().and_then(|value| value.to_str()) == Some("yaml");
        if !is_basename || !is_yaml {
            return Err(RulesError::InvalidFilename);
        }
        Ok(self.dir.join(filename_path))
    }

    fn read_rule_file(&self, filename: &str) -> Result<Vec<Rule>, RulesError> {
        let path = self.rule_path(filename)?;
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path)
            .map_err(|source| io_error("read Rule File", &path, source))?;
        let rule_file: RuleFile =
            serde_yaml::from_str(&content).map_err(|source| RulesError::Parse {
                path: path.clone(),
                source,
            })?;
        rule_file
            .rules
            .into_iter()
            .map(|entry| {
                entry
                    .to_rule()
                    .ok_or_else(|| RulesError::InvalidEntry { path: path.clone() })
            })
            .collect()
    }

    fn write_rule_file(&self, filename: &str, rules: &[Rule]) -> Result<(), RulesError> {
        let path = self.rule_path(filename)?;
        fs::create_dir_all(&self.dir)
            .map_err(|source| io_error("create rules directory", &self.dir, source))?;
        let file = RuleFile {
            rules: rules.iter().map(RuleEntry::from).collect(),
        };
        let yaml = serde_yaml::to_string(&file).map_err(|source| RulesError::Parse {
            path: path.clone(),
            source,
        })?;
        let mut temporary = tempfile::NamedTempFile::new_in(&self.dir)
            .map_err(|source| io_error("create temporary Rule File", &path, source))?;
        temporary
            .write_all(yaml.as_bytes())
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|source| io_error("write temporary Rule File", &path, source))?;
        temporary
            .persist(&path)
            .map_err(|error| io_error("replace Rule File", &path, error.error))?;
        self.reload();
        Ok(())
    }

    fn rule_set_matches(&self, name: &str, host: &str, client_ip: Option<IpAddr>) -> bool {
        let name_path = Path::new(name);
        if name_path.file_name().and_then(|value| value.to_str()) != Some(name) {
            return false;
        }
        let rulesets_dir = self.dir.parent().unwrap_or(&self.dir).join("rulesets");
        let path = rulesets_dir.join(format!("{name}.yaml"));
        let Ok(content) = fs::read_to_string(path) else {
            return false;
        };
        let Ok(items) = serde_yaml::from_str::<Vec<serde_yaml::Value>>(&content) else {
            return false;
        };
        items.iter().any(|value| {
            let (pattern, value) = if let Some(value) = value.as_str() {
                (RulePattern::DomainSuffix, value)
            } else if let Some(mapping) = value.as_mapping() {
                let pattern = mapping
                    .get("type")
                    .and_then(serde_yaml::Value::as_str)
                    .and_then(parse_pattern);
                let value = mapping.get("value").and_then(serde_yaml::Value::as_str);
                let (Some(pattern), Some(value)) = (pattern, value) else {
                    return false;
                };
                (pattern, value)
            } else {
                return false;
            };
            match_pattern(&pattern, value, host, client_ip)
        })
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Match a complete Routing Rule. `RULE-SET` requires a file-backed engine and
/// is therefore evaluated by [`RulesEngine::match_host`].
pub fn match_rule(rule: &Rule, host: &str, client_ip: Option<IpAddr>) -> bool {
    rule.enabled && match_pattern(&rule.pattern, &rule.value, host, client_ip)
}

/// Match one Rule Pattern without constructing a Routing Rule.
pub fn match_pattern(
    pattern: &RulePattern,
    value: &str,
    host: &str,
    client_ip: Option<IpAddr>,
) -> bool {
    match pattern {
        RulePattern::Domain => match_host_pattern(&HostPattern::Domain, value, host, client_ip),
        RulePattern::DomainSuffix => {
            match_host_pattern(&HostPattern::DomainSuffix, value, host, client_ip)
        }
        RulePattern::DomainKeyword => {
            match_host_pattern(&HostPattern::DomainKeyword, value, host, client_ip)
        }
        RulePattern::IpCidr => match_host_pattern(&HostPattern::IpCidr, value, host, client_ip),
        RulePattern::Geoip => geoip_match(host, value),
        RulePattern::RuleSet => false,
    }
}

/// Match the shared hostname/IP pattern primitive.
pub fn match_host_pattern(
    pattern: &HostPattern,
    value: &str,
    host: &str,
    client_ip: Option<IpAddr>,
) -> bool {
    let host = host.to_ascii_lowercase();
    let value = value.to_ascii_lowercase();
    match pattern {
        HostPattern::Domain => host == value,
        HostPattern::DomainSuffix => host == value || host.ends_with(&format!(".{value}")),
        HostPattern::DomainKeyword => host.contains(&value),
        HostPattern::IpCidr => client_ip
            .zip(value.parse::<ipnetwork::IpNetwork>().ok())
            .is_some_and(|(ip, network)| network.contains(ip)),
    }
}

/// Check an exact domain, subdomain, or `*.` suffix case-insensitively.
pub fn host_matches_domain(host: &str, domain: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let domain = domain.to_ascii_lowercase();
    let suffix = domain.strip_prefix("*.").unwrap_or(&domain);
    host == suffix || host.ends_with(&format!(".{suffix}"))
}

fn load_rules_from_dir(dir: &Path) -> Vec<Rule> {
    if !dir.exists() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(dir) else {
        log::error!("Failed to read rules directory: {}", dir.display());
        return Vec::new();
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    paths.sort();

    let mut rules = Vec::new();
    for path in paths {
        let Some(rule_file) = fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_yaml::from_str::<RuleFile>(&content).ok())
        else {
            log::warn!("Skipping invalid Rule File: {}", path.display());
            continue;
        };
        for entry in rule_file.rules {
            if let Some(rule) = entry.to_rule() {
                rules.push(rule);
            } else {
                log::warn!("Skipping invalid rule entry in {}", path.display());
            }
        }
    }
    rules.sort_by_key(|rule| rule.priority);
    rules
}

fn parse_pattern(value: &str) -> Option<RulePattern> {
    match value.to_ascii_uppercase().as_str() {
        "DOMAIN" => Some(RulePattern::Domain),
        "DOMAIN-SUFFIX" => Some(RulePattern::DomainSuffix),
        "DOMAIN-KEYWORD" => Some(RulePattern::DomainKeyword),
        "IP-CIDR" => Some(RulePattern::IpCidr),
        "GEOIP" => Some(RulePattern::Geoip),
        _ => None,
    }
}

fn geoip_match(host: &str, country: &str) -> bool {
    let address = format!("{host}:0")
        .to_socket_addrs()
        .ok()
        .and_then(|mut addresses| addresses.next())
        .map(|address| address.ip());
    address
        .map(geoip_country)
        .is_some_and(|detected| detected.eq_ignore_ascii_case(country))
}

fn geoip_country(ip: IpAddr) -> &'static str {
    let IpAddr::V4(address) = ip else {
        return "XX";
    };
    let octets = address.octets();
    if octets[0] == 10
        || (octets[0] == 172 && (16..=31).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 168)
        || octets[0] == 127
    {
        "LAN"
    } else if matches!(
        octets[0],
        3 | 8 | 18 | 20 | 23 | 34 | 40 | 51 | 52 | 54 | 65 | 70 | 104 | 130 | 137 | 146 | 157 | 191
    ) {
        "US"
    } else if matches!(
        octets[0],
        1 | 43
            | 47
            | 49
            | 81
            | 101
            | 106
            | 109
            | 110
            | 111
            | 114
            | 115
            | 118
            | 119
            | 120
            | 121
            | 123
            | 124
            | 129
            | 134
            | 139
            | 149
            | 150
            | 162
            | 170
            | 175
            | 182
            | 183
            | 193
            | 203
    ) {
        "CN"
    } else if matches!(octets[0], 63 | 176) {
        "IE"
    } else if matches!(octets[0], 13 | 35) {
        "JP"
    } else {
        "XX"
    }
}

fn io_error(operation: &'static str, path: &Path, source: std::io::Error) -> RulesError {
    RulesError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_rule(value: &str, priority: u8) -> Rule {
        Rule {
            pattern: RulePattern::Domain,
            value: value.to_owned(),
            action: RuleAction::Direct,
            name: value.to_owned(),
            priority,
            enabled: true,
            comment: String::new(),
        }
    }

    #[test]
    fn pattern_matching_is_case_insensitive_and_suffix_safe() {
        assert!(host_matches_domain("API.Example.com", "*.example.com"));
        assert!(!host_matches_domain("example.com.evil.com", "example.com"));
        assert!(match_pattern(
            &RulePattern::DomainKeyword,
            "WeChat",
            "api.wechat.com",
            None
        ));
    }

    #[test]
    fn disabled_and_ip_rules_match_consistently() {
        let mut rule = make_rule("10.0.0.0/8", 1);
        rule.pattern = RulePattern::IpCidr;
        assert!(match_rule(&rule, "host", Some("10.1.2.3".parse().unwrap())));
        rule.enabled = false;
        assert!(!match_rule(
            &rule,
            "host",
            Some("10.1.2.3".parse().unwrap())
        ));
    }

    #[test]
    fn priority_controls_the_first_action() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        let mut low = make_rule("example.com", 100);
        low.action = RuleAction::Proxy;
        let mut high = make_rule("example.com", 1);
        high.action = RuleAction::Reject;
        engine.set_rules(vec![low, high]);

        assert_eq!(
            engine.match_host("example.com", None),
            Some(RuleAction::Reject)
        );
    }

    #[test]
    fn file_scoped_crud_replaces_and_deletes_exact_rules() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        let original = make_rule("before.com", 1);
        let sibling = make_rule("before.com", 2);
        engine
            .save_rule_internal(original.clone(), "custom.yaml")
            .unwrap();
        engine
            .save_rule_internal(sibling.clone(), "custom.yaml")
            .unwrap();

        let mut updated = original.clone();
        updated.value = "after.com".to_owned();
        engine
            .save_rule(updated.clone(), Some(&original), "custom.yaml")
            .unwrap();
        assert_eq!(
            engine.get_rules_for_file("custom.yaml").unwrap(),
            [updated.clone(), sibling.clone()]
        );

        engine.delete_rule(&updated, "custom.yaml").unwrap();
        assert_eq!(engine.get_rules_for_file("custom.yaml").unwrap(), [sibling]);
    }

    #[test]
    fn concurrent_file_mutations_do_not_lose_rules() {
        let dir = tempdir().unwrap();
        let engine = Arc::new(RulesEngine::with_dir(dir.path().to_path_buf()));
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let handles = ["a.com", "b.com"].map(|value| {
            let engine = Arc::clone(&engine);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                engine
                    .save_rule_internal(make_rule(value, 1), "shared.yaml")
                    .unwrap();
            })
        });
        barrier.wait();
        for handle in handles {
            handle.join().unwrap();
        }

        let mut values = engine
            .get_rules_for_file("shared.yaml")
            .unwrap()
            .into_iter()
            .map(|rule| rule.value)
            .collect::<Vec<_>>();
        values.sort();
        assert_eq!(values, ["a.com", "b.com"]);
    }

    #[test]
    fn rule_files_are_isolated_sorted_and_reorderable() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        engine
            .save_rule_internal(make_rule("a.com", 1), "a.yaml")
            .unwrap();
        engine
            .save_rule_internal(make_rule("b.com", 2), "a.yaml")
            .unwrap();
        engine
            .save_rule_internal(make_rule("other.com", 1), "b.yaml")
            .unwrap();

        assert_eq!(engine.list_rule_files().unwrap(), ["a.yaml", "b.yaml"]);
        engine.reorder_rules(0, 1, "a.yaml").unwrap();
        let rules = engine.get_rules_for_file("a.yaml").unwrap();
        assert_eq!(
            rules
                .iter()
                .map(|rule| rule.value.as_str())
                .collect::<Vec<_>>(),
            ["b.com", "a.com"]
        );
        assert_eq!(
            engine.get_rules_for_file("b.yaml").unwrap()[0].value,
            "other.com"
        );
    }

    #[test]
    fn invalid_filename_and_stale_edits_have_typed_errors() {
        let dir = tempdir().unwrap();
        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        assert!(matches!(
            engine.save_rule_internal(make_rule("escape.com", 1), "../escape.yaml"),
            Err(RulesError::InvalidFilename)
        ));
        assert!(matches!(
            engine.save_rule(
                make_rule("new.com", 1),
                Some(&make_rule("missing.com", 1)),
                "custom.yaml"
            ),
            Err(RulesError::EditConflict)
        ));
    }

    #[test]
    fn invalid_yaml_is_skipped_without_hiding_valid_rule_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("bad.yaml"), "[unclosed: [").unwrap();
        fs::write(
            dir.path().join("good.yaml"),
            "rules:\n  - pattern: DOMAIN\n    value: valid.com\n    action: DIRECT\n",
        )
        .unwrap();

        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        assert_eq!(engine.get_rules()[0].value, "valid.com");
    }

    #[test]
    fn equal_priorities_are_deterministic_across_rule_files() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("z.yaml"),
            "rules:\n  - pattern: DOMAIN\n    value: z.com\n    action: DIRECT\n    priority: 1\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("a.yaml"),
            "rules:\n  - pattern: DOMAIN\n    value: a.com\n    action: DIRECT\n    priority: 1\n",
        )
        .unwrap();

        let engine = RulesEngine::with_dir(dir.path().to_path_buf());
        assert_eq!(
            engine
                .get_rules()
                .iter()
                .map(|rule| rule.value.as_str())
                .collect::<Vec<_>>(),
            ["a.com", "z.com"]
        );
    }

    #[test]
    fn watcher_reloads_external_rule_file_changes() {
        let dir = tempdir().unwrap();
        let engine = Arc::new(RulesEngine::with_dir(dir.path().to_path_buf()));
        engine.start_watcher().unwrap();
        fs::write(
            dir.path().join("external.yaml"),
            "rules:\n  - pattern: DOMAIN\n    value: external.com\n    action: REJECT\n",
        )
        .unwrap();

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline
            && engine.match_host("external.com", None).is_none()
        {
            std::thread::sleep(Duration::from_millis(50));
        }

        assert_eq!(
            engine.match_host("external.com", None),
            Some(RuleAction::Reject)
        );
    }

    #[test]
    fn rule_set_patterns_resolve_beside_the_rules_directory() {
        let root = tempdir().unwrap();
        let rules_dir = root.path().join("rules");
        let rulesets_dir = root.path().join("rulesets");
        fs::create_dir_all(&rules_dir).unwrap();
        fs::create_dir_all(&rulesets_dir).unwrap();
        fs::write(
            rulesets_dir.join("social.yaml"),
            "- example.com\n- type: DOMAIN-KEYWORD\n  value: wechat\n",
        )
        .unwrap();
        let engine = RulesEngine::with_dir(rules_dir);
        let mut rule = make_rule("social", 1);
        rule.pattern = RulePattern::RuleSet;
        rule.action = RuleAction::Reject;
        engine.set_rules(vec![rule]);

        assert_eq!(
            engine.match_host("api.example.com", None),
            Some(RuleAction::Reject)
        );
        assert_eq!(
            engine.match_host("api.wechat.com", None),
            Some(RuleAction::Reject)
        );
    }
}
