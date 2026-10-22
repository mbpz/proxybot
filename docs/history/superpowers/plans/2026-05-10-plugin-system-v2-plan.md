# Plugin System v2.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement declarative rule-based routing, priority ordering, async hooks, and hot reload for ProxyBot plugin system.

**Architecture:** RuleEngine performs pattern-matched plugin dispatch. New rule_engine.rs handles rule matching, new executor.rs handles hook execution with timeout. Existing PluginHooks extended with async variants.

**Tech Stack:** Rust (notify crate for file watching, tokio for async), serde_yaml for rule file parsing

---

## File Structure

```
src-tauri/src/plugin/
├── mod.rs              # Add: rule_engine, executor exports
├── plugin_trait.rs     # Modify: add async hook fields
├── registry.rs        # Modify: add rule_cache field
├── loader.rs          # Unchanged
├── sandbox.rs         # Unchanged
├── rule_engine.rs     # NEW: RuleEngine, PluginRule, RulePattern
└── executor.rs        # NEW: HookExecutor, async/sync execution
```

---

## Dependencies

Add to `Cargo.toml`:
```toml
notify = "8"  # already present
serde_yaml = "0.9"  # already present
tokio = { version = "1", features = ["full"] }  # already present
```

---

## Tasks

### Task 1: RulePattern enum and matching logic

**Files:**
- Create: `src/plugin/rule_engine.rs`

- [ ] **Step 1: Write failing test for RulePattern::matches()**

```rust
// src/plugin/rule_engine.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_suffix_match() {
        let pattern = RulePattern::DomainSuffix("weixin.qq.com".into());
        let request = InterceptedRequest {
            method: "GET".into(),
            scheme: "https".into(),
            host: "api.weixin.qq.com".into(),
            path: "/cgi-bin/token".into(),
            headers: vec![],
            body: None,
        };
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_domain_suffix_no_match() {
        let pattern = RulePattern::DomainSuffix("weixin.qq.com".into());
        let request = InterceptedRequest {
            method: "GET".into(),
            scheme: "https".into(),
            host: "api.douyin.com".into(),
            path: "/".into(),
            headers: vec![],
            body: None,
        };
        assert!(!pattern.matches(&request));
    }

    #[test]
    fn test_domain_keyword_match() {
        let pattern = RulePattern::DomainKeyword("weixin".into());
        let request = InterceptedRequest {
            method: "GET".into(),
            scheme: "https".into(),
            host: "api.weixin.qq.com".into(),
            path: "/".into(),
            headers: vec![],
            body: None,
        };
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
        let request = InterceptedRequest {
            method: "POST".into(),
            scheme: "https".into(),
            host: "api.example.com".into(),
            path: "/upload/file.jpg".into(),
            headers: vec![],
            body: None,
        };
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_header_match() {
        let pattern = RulePattern::Header {
            key: "Authorization".into(),
            value: "Bearer *".into(),
        };
        let request = InterceptedRequest {
            method: "GET".into(),
            scheme: "https".into(),
            host: "api.example.com".into(),
            path: "/".into(),
            headers: vec![("Authorization".into(), "Bearer token123".into())],
            body: None,
        };
        assert!(pattern.matches(&request));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib rule_engine::tests::test_domain_suffix_match -v`
Expected: FAIL - module not found

- [ ] **Step 3: Write RulePattern and wildcard matching**

```rust
// src/plugin/rule_engine.rs

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
        method: Option<String>,   // GET, POST, *
        scheme: Option<String>,   // https, *
        host: Option<String>,     // api.example.com, *
        path: Option<String>,     // /v2/*, *
    },
    /// Header match
    Header { key: String, value: String },
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
            // * matches one segment
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
            RulePattern::DomainSuffix(suffix) => request.host.ends_with(suffix),
            RulePattern::DomainKeyword(keyword) => request.host.contains(keyword),
            RulePattern::UrlPattern { method, scheme, host, path } => {
                method.as_ref().map_or(true, |m| m == "*" || m == &request.method) &&
                scheme.as_ref().map_or(true, |s| s == "*" || s == &request.scheme) &&
                host.as_ref().map_or(true, |h| h == "*" || h == &request.host) &&
                path.as_ref().map_or(true, |p| wildcard_match(p, &request.path))
            }
            RulePattern::Header { key, value } => {
                request.headers.iter().any(|(k, v)| k == key && wildcard_match(value, v))
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib rule_engine -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/plugin/rule_engine.rs
git commit -m "feat(plugin): add RulePattern enum with domain/URL/header matching"
```

---

### Task 2: PluginRule and RuleEngine core

**Files:**
- Modify: `src/plugin/rule_engine.rs`

- [ ] **Step 1: Write failing test for PluginRule and RuleEngine**

```rust
#[test]
fn test_plugin_rule_priority_ordering() {
    let rule_a = PluginRule {
        id: 1,
        name: "A".into(),
        pattern: RulePattern::DomainSuffix("qq.com".into()),
        plugin_name: "plugin-a".into(),
        priority: 100,
        enabled: true,
    };
    let rule_b = PluginRule {
        id: 2,
        name: "B".into(),
        pattern: RulePattern::DomainSuffix("weixin.qq.com".into()),
        plugin_name: "plugin-b".into(),
        priority: 50,
        enabled: true,
    };
    let rule_c = PluginRule {
        id: 3,
        name: "C".into(),
        pattern: RulePattern::DomainSuffix("qq.com".into()),
        plugin_name: "plugin-c".into(),
        priority: 100,
        enabled: true,
    };

    let mut rules = vec![rule_a, rule_b, rule_c];
    rules.sort_by_key(|r| r.priority);

    assert_eq!(rules[0].plugin_name, "plugin-b");  // priority 50
    assert_eq!(rules[1].plugin_name, "plugin-a");  // priority 100
    assert_eq!(rules[2].plugin_name, "plugin-c");  // priority 100, order preserved
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib test_plugin_rule_priority_ordering -v`
Expected: FAIL

- [ ] **Step 3: Write PluginRule and RuleEngine skeleton**

```rust
/// A single plugin routing rule
#[derive(Clone, Debug)]
pub struct PluginRule {
    pub id: u64,
    pub name: String,
    pub pattern: RulePattern,
    pub plugin_name: String,
    pub priority: u16,  // lower = higher priority (0 = highest)
    pub enabled: bool,
}

/// Rule engine for pattern-matched plugin dispatch
pub struct RuleEngine {
    rules: std::sync::RwLock<Vec<PluginRule>>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self { rules: std::sync::RwLock::new(Vec::new()) }
    }

    /// Match request against rules, return first matching rule (highest priority)
    pub fn match_request<'a>(&self, request: &'a InterceptedRequest) -> Option<&'a PluginRule> {
        let rules = self.rules.read().unwrap();
        rules.iter()
            .filter(|r| r.enabled && r.pattern.matches(request))
            .min_by_key(|r| r.priority)
    }

    /// Match request against all matching rules sorted by priority
    pub fn match_all<'a>(&self, request: &'a InterceptedRequest) -> Vec<&'a PluginRule> {
        let rules = self.rules.read().unwrap();
        let mut matched: Vec<_> = rules.iter()
            .filter(|r| r.enabled && r.pattern.matches(request))
            .collect();
        matched.sort_by_key(|r| r.priority);
        matched
    }

    /// Add a rule
    pub fn add_rule(&self, rule: PluginRule) {
        let mut rules = self.rules.write().unwrap();
        rules.push(rule);
        rules.sort_by_key(|r| r.priority);
    }

    /// Remove a rule by id
    pub fn remove_rule(&self, id: u64) -> Option<PluginRule> {
        let mut rules = self.rules.write().unwrap();
        if let Some(pos) = rules.iter().position(|r| r.id == id) {
            Some(rules.remove(pos))
        } else {
            None
        }
    }
}

impl Default for RuleEngine {
    fn default() -> Self { Self::new() }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib test_plugin_rule_priority_ordering -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/plugin/rule_engine.rs
git commit -m "feat(plugin): add PluginRule struct and RuleEngine with priority matching"
```

---

### Task 3: RuleEngine YAML loading and hot reload

**Files:**
- Modify: `src/plugin/rule_engine.rs`

- [ ] **Step 1: Write failing test for YAML loading**

```rust
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
    // Priority 50 should come first
    assert_eq!(rules[0].name, "Upload");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib test_rule_engine_from_yaml -v`
Expected: FAIL

- [ ] **Step 3: Write YAML loading and file watcher**

```rust
use serde::{Deserialize, Deserialization};
use std::path::Path;
use notify::{Watcher, RecursiveMode, EventKind};
use std::sync::mpsc::channel;
use std::time::Duration;

/// YAML rule file format
#[derive(Deserialize, Debug)]
struct RuleFile {
    rules: Vec<RuleFileEntry>,
}

#[derive(Deserialize, Debug)]
struct RuleFileEntry {
    name: String,
    pattern: PatternEntry,
    plugin: String,
    priority: u16,
    enabled: bool,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum PatternEntry {
    DomainSuffix { value: String },
    DomainKeyword { value: String },
    UrlPattern {
        method: Option<String>,
        scheme: Option<String>,
        host: Option<String>,
        path: Option<String>,
    },
    Header { key: String, value: String },
}

impl RuleFile {
    fn into_rules(self, start_id: u64) -> Vec<PluginRule> {
        self.rules.into_iter()
            .enumerate()
            .map(|(i, entry)| {
                let pattern = match entry.pattern {
                    PatternEntry::DomainSuffix { value } => RulePattern::DomainSuffix(value),
                    PatternEntry::DomainKeyword { value } => RulePattern::DomainKeyword(value),
                    PatternEntry::UrlPattern { method, scheme, host, path } => {
                        RulePattern::UrlPattern { method, scheme, host, path }
                    }
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

impl RuleEngine {
    /// Create from YAML file (loads and sorts rules by priority)
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let file: RuleFile = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
        let mut rules = file.into_rules(0);
        rules.sort_by_key(|r| r.priority);
        Ok(Self { rules: std::sync::RwLock::new(rules) })
    }

    /// Reload rules from file (for hot reload)
    pub fn reload(&self, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let file: RuleFile = serde_yaml::from_str(&content).map_err(|e| e.to_string())?;
        let new_rules = file.into_rules(0);
        let mut rules = self.rules.write().unwrap();
        *rules = new_rules;
        rules.sort_by_key(|r| r.priority);
        Ok(())
    }

    /// List all rules
    pub fn list_rules(&self) -> Vec<PluginRule> {
        self.rules.read().unwrap().clone()
    }

    /// Watch file for changes (auto-reload on modify)
    pub fn watch(&mut self, path: &Path) -> Result<(), String> {
        let rules = Arc::new(std::sync::RwLock::new(Vec::new()));
        let rules_for_watcher = rules.clone();
        let path_owned = path.to_path_buf();

        let (tx, rx) = channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if res.is_ok() {
                let _ = tx.send(());
            }
        }).map_err(|e| format!("Watcher failed: {}", e))?;

        watcher.watch(path, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;

        // Spawn thread to handle debounced reloads
        std::thread::spawn(move || {
            let mut last_reload = std::time::Instant::now() - Duration::from_secs(1);
            loop {
                if rx.recv_timeout(Duration::from_millis(100)).is_ok() {
                    let now = std::time::Instant::now();
                    if now.duration_since(last_reload) > Duration::from_millis(500) {
                        last_reload = now;
                        if let Err(e) = RuleEngine::reload(&rules_for_watcher, &path_owned) {
                            eprintln!("Rule reload failed: {}", e);
                        }
                    }
                }
            }
        });

        Ok(())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib test_rule_engine_from_yaml -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/plugin/rule_engine.rs
git commit -m "feat(plugin): add YAML loading and file watcher for hot reload"
```

---

### Task 4: HookExecutor with async support

**Files:**
- Create: `src/plugin/executor.rs`

- [ ] **Step 1: Write failing test for HookExecutor**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use crate::plugin::registry::PluginRegistry;

    struct TestPlugin {
        name: String,
        call_count: Arc<AtomicUsize>,
    }
    impl Plugin for TestPlugin {
        fn name(&self) -> &str { &self.name }
        fn hooks(&self) -> PluginHooks {
            let count = self.call_count.clone();
            PluginHooks {
                on_request: Some(Box::new(move |_| { count.fetch_add(1, Ordering::SeqCst); })),
                on_response: None,
                on_connect: None,
                on_error: None,
                on_request_async: None,
                on_response_async: None,
            }
        }
    }

    #[test]
    fn test_executor_single_match() {
        let registry = PluginRegistry::new();
        registry.register(Box::new(TestPlugin {
            name: "test-plugin".into(),
            call_count: Arc::new(AtomicUsize::new(0)),
        }));

        let rule = PluginRule {
            id: 1,
            name: "Test Rule".into(),
            pattern: RulePattern::DomainSuffix("example.com".into()),
            plugin_name: "test-plugin".into(),
            priority: 100,
            enabled: true,
        };

        let mut request = InterceptedRequest {
            method: "GET".into(),
            scheme: "https".into(),
            host: "api.example.com".into(),
            path: "/".into(),
            headers: vec![],
            body: None,
        };

        HookExecutor::execute_request_sync(&registry, &[&rule], &mut request);

        let rules = vec![&rule];
        assert_eq!(rules.len(), 1);  // placeholder
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib executor -v`
Expected: FAIL - module not found

- [ ] **Step 3: Write HookExecutor**

```rust
// src/plugin/executor.rs

use crate::proxy::{InterceptedRequest, InterceptedResponse};
use crate::plugin::{PluginRegistry, PluginHooks};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Default hook timeout (5 seconds)
const HOOK_TIMEOUT: Duration = Duration::from_secs(5);

/// Hook executor for running plugin hooks with rule-based routing
pub struct HookExecutor;

impl HookExecutor {
    /// Execute on_request hooks for the first matching rule (sync)
    pub fn execute_request_sync(
        plugins: &Arc<PluginRegistry>,
        rules: &[&PluginRule],
        request: &mut InterceptedRequest,
    ) {
        // Find first matching rule
        let matched = rules.iter()
            .filter(|r| r.enabled && r.pattern.matches(request))
            .min_by_key(|r| r.priority);

        if let Some(rule) = matched {
            if let Some(plugin) = plugins.get(&rule.plugin_name) {
                if let Some(ref hook) = plugin.hooks().on_request {
                    hook(request);
                }
            }
        }
    }

    /// Execute on_request hooks for all matching rules (sync)
    pub fn execute_request_sync_all(
        plugins: &Arc<PluginRegistry>,
        rules: &[&PluginRule],
        request: &mut InterceptedRequest,
    ) {
        let matched: Vec<_> = rules.iter()
            .filter(|r| r.enabled && r.pattern.matches(request))
            .collect();

        for rule in matched {
            if let Some(plugin) = plugins.get(&rule.plugin_name) {
                if let Some(ref hook) = plugin.hooks().on_request {
                    hook(request);
                }
            }
        }
    }

    /// Execute on_response hooks for all matching rules (sync)
    pub fn execute_response_sync(
        plugins: &Arc<PluginRegistry>,
        rules: &[&PluginRule],
        response: &mut InterceptedResponse,
    ) {
        let matched: Vec<_> = rules.iter()
            .filter(|r| r.enabled)
            .collect();

        for rule in matched {
            if let Some(plugin) = plugins.get(&rule.plugin_name) {
                if let Some(ref hook) = plugin.hooks().on_response {
                    hook(response);
                }
            }
        }
    }

    /// Execute async hooks with timeout
    pub async fn execute_request_async(
        plugins: Arc<PluginRegistry>,
        rules: Vec<PluginRule>,
        mut request: InterceptedRequest,
    ) -> InterceptedRequest {
        let matched: Vec<_> = rules.iter()
            .filter(|r| r.enabled && r.pattern.matches(&request))
            .collect();

        for rule in matched {
            if let Some(plugin) = plugins.get(&rule.plugin_name) {
                if let Some(ref hook) = plugin.hooks().on_request_async {
                    let future = hook(&mut request);
                    let _ = timeout(HOOK_TIMEOUT, future).await;
                }
            }
        }
        request
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib executor -v`
Expected: PASS (may need TestPlugin import fix)

- [ ] **Step 5: Commit**

```bash
git add src/plugin/executor.rs
git commit -m "feat(plugin): add HookExecutor with sync/async hook execution"
```

---

### Task 5: Update mod.rs exports and proxy.rs integration

**Files:**
- Modify: `src/plugin/mod.rs`

- [ ] **Step 1: Write failing test for module exports**

```rust
#[test]
fn test_mod_exports_rule_engine() {
    use crate::plugin::{RuleEngine, RulePattern, PluginRule};
    let engine = RuleEngine::new();
    let rules = engine.list_rules();
    assert_eq!(rules.len(), 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib test_mod_exports_rule_engine -v`
Expected: FAIL

- [ ] **Step 3: Update mod.rs exports**

```rust
// src/plugin/mod.rs

pub mod registry;
pub mod loader;
pub mod plugin_trait;
pub mod sandbox;
pub mod rule_engine;   // NEW
pub mod executor;      // NEW
pub use plugin_trait::{Plugin, PluginHooks, ConnectDecision, InterceptedResponse};
pub use rule_engine::{RuleEngine, RulePattern, PluginRule};
pub use executor::HookExecutor;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib test_mod_exports_rule_engine -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/plugin/mod.rs
git commit -m "feat(plugin): export RuleEngine, RulePattern, PluginRule, HookExecutor"
```

---

### Task 6: Integrate RuleEngine into proxy.rs

**Files:**
- Modify: `src/proxy.rs`

- [ ] **Step 1: Write failing test for rule-based hook dispatch**

```rust
#[test]
fn test_rule_based_dispatch() {
    use crate::plugin::{RuleEngine, RulePattern, PluginRule};

    let engine = RuleEngine::new();

    // Add test rule
    engine.add_rule(PluginRule {
        id: 1,
        name: "Test".into(),
        pattern: RulePattern::DomainSuffix("example.com".into()),
        plugin_name: "test-plugin".into(),
        priority: 100,
        enabled: true,
    });

    let request = InterceptedRequest {
        method: "GET".into(),
        scheme: "https".into(),
        host: "api.example.com".into(),
        path: "/".into(),
        headers: vec![],
        body: None,
    };

    let matched = engine.match_request(&request);
    assert!(matched.is_some());
    assert_eq!(matched.unwrap().plugin_name, "test-plugin");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib test_rule_based_dispatch -v`
Expected: FAIL

- [ ] **Step 3: Update proxy.rs to use RuleEngine**

In `src/proxy.rs`, replace the `call_on_request_hooks` function:

```rust
/// Call on_request hooks using rule-based dispatch
fn call_on_request_hooks(
    plugins: &Arc<PluginRegistry>,
    rule_engine: &Arc<RuleEngine>,
    request: &mut InterceptedRequest,
) {
    let rules: Vec<_> = rule_engine.match_all(request);
    HookExecutor::execute_request_sync_all(plugins, &rules.iter().collect::<Vec<_>>(), request);
}
```

And update all call sites (around line 1404).

Also update `call_on_response_hooks` similarly.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib test_rule_based_dispatch -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/proxy.rs
git commit -m "feat(proxy): integrate RuleEngine for pattern-based hook dispatch"
```

---

### Task 7: Add async hook fields to PluginHooks

**Files:**
- Modify: `src/plugin/plugin_trait.rs`

- [ ] **Step 1: Write failing test for async hooks**

```rust
#[test]
fn test_async_hooks_present() {
    let hooks = PluginHooks::default();
    assert!(hooks.on_request_async.is_none());
    assert!(hooks.on_response_async.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib test_async_hooks_present -v`
Expected: FAIL

- [ ] **Step 3: Add async hook fields**

```rust
// In PluginHooks struct, add after existing fields:

// Async variants
pub on_request_async: Option<Box<dyn Fn(&mut InterceptedRequest) -> BoxFuture<'static, ()> + Send + Sync>>,
pub on_response_async: Option<Box<dyn Fn(&mut InterceptedResponse) -> BoxFuture<'static, ()> + Send + Sync>>,
```

Add type alias at top:
```rust
use std::pin::Pin;
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib test_async_hooks_present -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/plugin/plugin_trait.rs
git commit -m "feat(plugin): add async hook fields to PluginHooks"
```

---

## Verification

```bash
# Run all tests
cargo test --lib

# Build to verify compilation
cargo build --lib

# Verify plugin module compiles
cargo check --lib 2>&1 | grep -i plugin
```

---

## Dependencies

Required Cargo.toml entries (already present):
- `notify = "8"`
- `serde_yaml = "0.9"`
- `tokio = { version = "1", features = ["full"] }`
