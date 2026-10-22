# Plugin System v2.0 Design Specification

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enhance ProxyBot plugin system with declarative rule-based routing, priority ordering, hot reload, and async hook support.

**Architecture:** RuleEngine performs pattern-matched plugin dispatch. Plugins register declarative rules (domain/URL patterns) with priority. File watcher + CLI manage hot reload.

**Tech Stack:** Rust (notify crate for file watching, tokio for async hooks), YAML for rule files

---

## 1. Overview

ProxyBot v1.0 has a basic Plugin trait with 4 hooks (on_request, on_response, on_connect, on_error). This design enhances it with:

1. **Declarative rule routing** - plugins declare URL patterns that trigger them (whistle-style: `pattern pluginName`)
2. **Per-rule priority** - each rule has its own priority; more specific patterns win first
3. **Hot reload** - file watching + CLI commands for dynamic rule/plugin management
4. **Async hook support** - hooks can be async for non-blocking execution

---

## 2. Data Structures

### 2.1 PluginRule

```rust
#[derive(Clone, Debug)]
pub struct PluginRule {
    pub id: u64,
    pub name: String,
    pub pattern: RulePattern,
    pub plugin_name: String,
    pub priority: u16,  // lower = higher priority (0 = highest)
    pub enabled: bool,
}

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
```

### 2.2 RuleEngine

```rust
pub struct RuleEngine {
    rules: RwLock<Vec<PluginRule>>,
    file_watcher: Option<notify::Watcher>,
    rule_file_path: PathBuf,
}
```

### 2.3 Enhanced PluginHooks (async support)

```rust
#[derive(Default)]
pub struct PluginHooks {
    pub on_request: Option<Box<dyn Fn(&mut InterceptedRequest) + Send + Sync>>,
    pub on_response: Option<Box<dyn Fn(&mut InterceptedResponse) + Send + Sync>>,
    pub on_connect: Option<Box<dyn Fn(&str) -> ConnectDecision + Send + Sync>>,
    pub on_error: Option<Box<dyn Fn(&AppError) + Send + Sync>>,
    // NEW: async variants
    pub on_request_async: Option<Box<dyn Fn(&mut InterceptedRequest) -> BoxFuture<'static, ()> + Send + Sync>>,
    pub on_response_async: Option<Box<dyn Fn(&mut InterceptedResponse) -> BoxFuture<'static, ()> + Send + Sync>>,
}
```

---

## 3. Rule File Format

**Location:** `~/.proxybot/rules/plugins.yaml`

```yaml
rules:
  - name: WeChat Traffic Classifier
    pattern:
      type: DomainSuffix
      value: "*.weixin.qq.com"
    plugin: wechat-classifier
    priority: 100
    enabled: true

  - name: Douyin API Monitor
    pattern:
      type: DomainKeyword
      value: "douyin"
    plugin: douyin-monitor
    priority: 100
    enabled: true

  - name: Upload Interceptor
    pattern:
      type: UrlPattern
      method: POST
      path: "*/upload/*"
    plugin: upload-inspector
    priority: 50
    enabled: true

  - name: Auth Header Check
    pattern:
      type: Header
      key: "Authorization"
      value: "Bearer *"
    plugin: auth-validator
    priority: 75
    enabled: true
```

---

## 4. RuleEngine API

### 4.1 Core Methods

```rust
impl RuleEngine {
    /// Create from YAML file (loads and sorts rules by priority)
    pub fn from_file(path: &Path) -> Result<Self, String> { ... }

    /// Reload rules from file (for hot reload)
    pub fn reload(&self) -> Result<(), String> { ... }

    /// Watch file for changes (auto-reload on modify)
    pub fn watch(&mut self, path: &Path) -> Result<(), String> { ... }

    /// Match request against rules, return first matching rule
    pub fn match_request<'a>(&self, request: &'a InterceptedRequest) -> Option<&'a PluginRule> { ... }

    /// Match request against all matching rules (for multi-plugin dispatch)
    pub fn match_all<'a>(&self, request: &'a InterceptedRequest) -> Vec<&'a PluginRule> { ... }

    /// Add a rule dynamically
    pub fn add_rule(&self, rule: PluginRule) { ... }

    /// Remove a rule by id
    pub fn remove_rule(&self, id: u64) { ... }

    /// Enable/disable a rule
    pub fn set_rule_enabled(&self, id: u64, enabled: bool) { ... }
}
```

### 4.2 Pattern Matching Logic

```rust
impl RulePattern {
    pub fn matches(&self, request: &InterceptedRequest) -> bool {
        match self {
            DomainSuffix(suffix) => request.host.ends_with(suffix),
            DomainKeyword(keyword) => request.host.contains(keyword),
            UrlPattern { method, scheme, host, path } => {
                method.map_or(true, |m| m == "*" || m == request.method) &&
                scheme.map_or(true, |s| s == "*" || s == request.scheme) &&
                host.map_or(true, |h| h == "*" || h == request.host) &&
                path.map_or(true, |p| wildcard_match(&p, &request.path))
            }
            Header { key, value } => {
                request.headers.iter().any(|(k, v)| k == key && wildcard_match(value, v))
            }
        }
    }
}
```

### 4.3 Priority Sorting

Rules sorted by `priority` ascending (lower = higher priority). For same priority, order of definition in file is preserved. On `match_all`, returns all matching rules sorted by priority.

---

## 5. Hook Executor

### 5.1 Sync Hook Execution

```rust
fn execute_request_hooks_sync(
    plugins: &Arc<PluginRegistry>,
    rules: &[PluginRule],
    request: &mut InterceptedRequest,
) {
    // Single-match mode: first rule wins
    if let Some(rule) = RuleEngine::match_request(rules, request) {
        if let Some(plugin) = plugins.get(&rule.plugin_name) {
            if let Some(ref hook) = plugin.hooks().on_request {
                hook(request);
            }
        }
    }
}
```

### 5.2 Async Hook Execution

```rust
async fn execute_request_hooks_async(
    plugins: &Arc<PluginRegistry>,
    rules: &[PluginRule],
    request: &mut InterceptedRequest,
) {
    let matched = RuleEngine::match_all(rules, request);
    for rule in matched {
        if let Some(plugin) = plugins.get(&rule.plugin_name) {
            if let Some(ref hook) = plugin.hooks().on_request_async {
                hook(request).await;
            }
        }
    }
}
```

### 5.3 Timeout Handling

```rust
const HOOK_TIMEOUT: Duration = Duration::from_secs(5);

async fn execute_with_timeout<F>(future: F) -> Option<F::Output>
where
    F: Future,
{
    tokio::time::timeout(HOOK_TIMEOUT, future).await.ok()
}
```

---

## 6. Hot Reload

### 6.1 File Watcher

```rust
impl RuleEngine {
    pub fn watch(&mut self, path: &Path) -> Result<(), String> {
        let engine = self.engine.clone();
        let rule_path = self.rule_file_path.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(Event { kind: EventKind::Modify(_), .. }) = res {
                if let Err(e) = engine.borrow_mut().reload() {
                    eprintln!("Rule reload failed: {}", e);
                }
            }
        }).map_err(|e| format!("Watcher failed: {}", e))?;

        watcher.watch(path, RecursiveMode::NonRecursive).map_err(|e| e.to_string())?;
        self.file_watcher = Some(watcher);
        Ok(())
    }
}
```

### 6.2 CLI Commands

```bash
# Plugin management
proxybot plugin load <path>          # Load plugin from .so/.wasm
proxybot plugin unload <name>        # Unload plugin
proxybot plugin reload [name]        # Reload rules or specific plugin
proxybot plugin list                  # List loaded plugins + active rules

# Rule management
proxybot rule add <yaml>             # Add rule from YAML
proxybot rule remove <id>            # Remove rule by ID
proxybot rule enable <id>            # Enable rule
proxybot rule disable <id>           # Disable rule
proxybot rule list                   # List all rules with priorities
```

---

## 7. Error Handling

| Error | Behavior |
|-------|----------|
| Plugin panic | Caught by hook executor, logged with plugin name, request continues |
| Hook timeout | After 5s, log warning, cancel task, request continues |
| Rule file invalid | Load fails, use cached rules, show error in TUI status bar |
| Plugin not found | Skip rule, log warning once per plugin |
| Watcher fails | Log error, continue without auto-reload |

---

## 8. File Structure

```
src-tauri/src/
├── plugin/
│   ├── mod.rs              # Module exports
│   ├── plugin_trait.rs     # Plugin trait, PluginHooks (unchanged)
│   ├── registry.rs        # PluginRegistry (enhanced with rule_cache)
│   ├── loader.rs          # PluginLoader (enhanced for .so loading)
│   ├── sandbox.rs         # WasmSandbox (unchanged)
│   ├── rule_engine.rs     # NEW: RuleEngine, PluginRule, RulePattern
│   └── executor.rs        # NEW: HookExecutor, async/sync execution
```

---

## 9. Migration

- Keep existing `Plugin` trait compatible
- Add `on_request_async`/`on_response_async` as optional fields
- Old sync hooks take precedence if both are defined
- Existing plugin code continues to work unchanged

---

## 10. Test Plan

1. **Unit tests** for `RulePattern::matches()` - all pattern types
2. **Unit tests** for priority sorting
3. **Integration test** - load rules, verify first-match behavior
4. **File watcher test** - modify YAML, verify reload
5. **Hook executor test** - verify timeout and error handling
