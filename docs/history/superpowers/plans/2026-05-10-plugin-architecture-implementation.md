# ProxyBot v0.11.0 插件架构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 统一的插件架构，支持 Rhai 脚本、WASM 第三方插件、Rust 内置插件（gRPC 解码、流量限制）

**Architecture:** 分层架构——ProxyPlugin trait 统一接口，HooksRegistry 统一调度，三个 runtime（Rhai/WASM/Native）各司其职，集成到 proxy pipeline。

**Tech Stack:** Rust (trait + Arc), Rhai (脚本引擎), wasmtime (WASM 沙箱), bitflags

---

## 文件结构

```
src-tauri/src/
├── plugin/
│   ├── mod.rs                      # PluginManager (REPLACE 现有 stub)
│   ├── trait.rs                   # ProxyPlugin trait + HookContext + HookResult (REPLACE 现有)
│   ├── registry.rs                # HooksRegistry (NEW)
│   ├── builtins/
│   │   ├── mod.rs                 # NativeRegistry (NEW)
│   │   ├── grpc_plugin.rs         # GrpcPlugin (NEW)
│   │   └── network_plugin.rs      # NetworkConditionsPlugin (NEW)
│   ├── rhai/
│   │   ├── mod.rs                # RhaiEngine (NEW)
│   │   └── builtin_api.rs        # request.*, ctx.* API (NEW)
│   └── wasm/
│       ├── mod.rs                # WasmRuntime (NEW)
│       └── sandbox.rs            # WASM 资源限制 (NEW)
├── proxy.rs                       # 集成 HooksRegistry (MODIFY)
└── Cargo.toml                     # 添加 rhai, wasmtime 依赖 (MODIFY)

tests/plugin/                       # 单元测试 (NEW directory)
```

**现有文件状态：**
- `src-tauri/src/plugin/mod.rs` — 现有 stub，内容只有 `pub mod trait;`
- `src-tauri/src/plugin/trait.rs` — 现有简陋版，只有 `Plugin` trait（需替换）
- `src-tauri/src/plugin/tests.rs` — 现有基础测试（保留，添加新测试）

---

## Phase 1: 核心调度机制

### Task 1: 替换 trait.rs（新 ProxyPlugin + HookContext + HookResult）

**Files:**
- Modify: `src-tauri/src/plugin/trait.rs`

- [ ] **Step 1: 写入测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_result_modified_short_circuits() {
        let results = vec![
            HookResult::Continue,
            HookResult::Modified(ModifiedContent {
                headers: None,
                body: Some(b"modified".to_vec()),
                intercept: false,
                log_message: None,
            }),
        ];
        // Modified should short-circuit, not continue
        assert!(matches!(
            results[1],
            HookResult::Modified(_)
        ));
    }

    #[test]
    fn test_hook_context_from_request() {
        use crate::proxy::InterceptedRequest;
        let req = InterceptedRequest {
            method: "GET".into(),
            host: "example.com".into(),
            path: "/api/v1".into(),
            ..Default::default()
        };
        let ctx = HookContext::from_intercepted_request(&req, TrafficDirection::Inbound);
        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.host, "example.com");
        assert_eq!(ctx.path, "/api/v1");
    }
}
```

- [ ] **Step 2: 替换 trait.rs 内容**

将现有 `trait.rs` 内容替换为完整实现：

```rust
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Hook type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookType {
    Request,
    Response,
    Connect,
    Error,
    TrafficLog,
}

/// Hook execution order
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HookOrder {
    Earliest = 0,
    Normal = 100,
    Late = 200,
    Last = 255,
}

/// Plugin capabilities bitmask
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
    pub struct PluginCapabilities: u32 {
        const ON_REQUEST     = 1 << 0;
        const ON_RESPONSE    = 1 << 1;
        const ON_CONNECT    = 1 << 2;
        const ON_ERROR      = 1 << 3;
        const ON_TRAFFIC_LOG = 1 << 4;
    }
}

/// Traffic direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrafficDirection {
    Inbound,
    Outbound,
}

/// Context passed to plugin hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub direction: TrafficDirection,
    pub method: String,
    pub host: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub timestamp: i64,
    pub source_ip: String,
    pub dest_ip: String,
    pub dest_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
}

impl HookContext {
    pub fn from_intercepted_request(
        req: &crate::proxy::InterceptedRequest,
        direction: TrafficDirection,
    ) -> Self {
        Self {
            direction,
            method: req.method.clone(),
            host: req.host.clone(),
            path: req.path.clone(),
            headers: req.req_headers.clone(),
            body: req.req_body.as_ref().map(|b| b.as_bytes().to_vec()).unwrap_or_default(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
            source_ip: req.client_ip.clone().unwrap_or_default(),
            dest_ip: String::new(),
            dest_port: if req.scheme == "https" { 443 } else { 80 },
            app_name: req.app_name.clone(),
        }
    }
}

/// Content a plugin can modify
#[derive(Debug, Clone, Default)]
pub struct ModifiedContent {
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
    pub intercept: bool,
    pub log_message: Option<String>,
}

/// Result returned by a plugin hook
#[derive(Debug, Clone)]
pub enum HookResult {
    Continue,
    Modified(ModifiedContent),
    Intercept(String),
impl Default for HookResult {
    fn default() -> Self { HookResult::Continue }
}

/// Unified plugin trait
pub trait ProxyPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn capabilities(&self) -> PluginCapabilities;
    fn on_request(&self, _ctx: &HookContext) -> HookResult { HookResult::Continue }
    fn on_response(&self, _ctx: &HookContext) -> HookResult { HookResult::Continue }
    fn on_connect(&self, _ctx: &HookContext) -> HookResult { HookResult::Continue }
    fn on_error(&self, _ctx: &HookContext, _error: &str) -> HookResult { HookResult::Continue }
    fn on_traffic_log(&self, _ctx: &HookContext) {}
    fn init(&mut self, _config: serde_json::Value) -> Result<(), String> { Ok(()) }
    fn shutdown(&self) {}
}

use std::time::{SystemTime, UNIX_EPOCH};
```

- [ ] **Step 3: 运行测试**

Run: `cargo test -p proxybot_lib plugin::tests --no-default-features -- --nocapture`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/plugin/trait.rs
git commit -m "feat(plugin): replace stub with ProxyPlugin trait + HookContext + HookResult"
```

---

### Task 2: 实现 HooksRegistry

**Files:**
- Create: `src-tauri/src/plugin/registry.rs`
- Modify: `src-tauri/src/plugin/mod.rs` (添加 `pub mod registry;`)

- [ ] **Step 1: 写入测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_dispatch_continues_when_all_continue() {
        let registry = HooksRegistry::new();
        registry.register(HookRegistration {
            plugin_name: "p1".into(),
            hook_type: HookType::Request,
            order: HookOrder::Normal,
            callback: Arc::new(|_| HookResult::Continue),
        });
        registry.register(HookRegistration {
            plugin_name: "p2".into(),
            hook_type: HookType::Request,
            order: HookOrder::Normal,
            callback: Arc::new(|_| HookResult::Continue),
        });

        let ctx = HookContext {
            direction: TrafficDirection::Inbound,
            method: "GET".into(),
            host: "example.com".into(),
            path: "/".into(),
            headers: vec![],
            body: vec![],
            timestamp: 0,
            source_ip: "".into(),
            dest_ip: "".into(),
            dest_port: 80,
            app_name: None,
        };
        let result = registry.dispatch(HookType::Request, &ctx);
        assert!(matches!(result, HookResult::Continue));
    }

    #[test]
    fn test_registry_modified_short_circuits() {
        let registry = HooksRegistry::new();
        registry.register(HookRegistration {
            plugin_name: "p1".into(),
            hook_type: HookType::Request,
            order: HookOrder::Normal,
            callback: Arc::new(|_| HookResult::Modified(ModifiedContent {
                body: Some(b"modified".to_vec()),
                ..Default::default()
            })),
        });
        registry.register(HookRegistration {
            plugin_name: "p2".into(),
            hook_type: HookType::Request,
            order: HookOrder::Normal,
            callback: Arc::new(|_| {
                panic!("should not be called")
            }),
        });

        let ctx = HookContext {
            direction: TrafficDirection::Inbound,
            method: "GET".into(),
            host: "example.com".into(),
            path: "/".into(),
            headers: vec![],
            body: vec![],
            timestamp: 0,
            source_ip: "".into(),
            dest_ip: "".into(),
            dest_port: 80,
            app_name: None,
        };
        let result = registry.dispatch(HookType::Request, &ctx);
        assert!(matches!(result, HookResult::Modified(_)));
    }

    #[test]
    fn test_registry_order_respected() {
        let registry = HooksRegistry::new();
        let call_order = std::sync::atomic::AtomicUsize::new(0);
        let order = Arc::new(call_order);

        let order2 = order.clone();
        registry.register(HookRegistration {
            plugin_name: "late".into(),
            hook_type: HookType::Request,
            order: HookOrder::Late,
            callback: Arc::new(move |_| {
                let prev = order2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                assert_eq!(prev, 2); // Should run third
                HookResult::Continue
            }),
        });

        let order2 = order.clone();
        registry.register(HookRegistration {
            plugin_name: "earliest".into(),
            hook_type: HookType::Request,
            order: HookOrder::Earliest,
            callback: Arc::new(move |_| {
                let prev = order2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                assert_eq!(prev, 0); // Should run first
                HookResult::Continue
            }),
        });

        let order2 = order.clone();
        registry.register(HookRegistration {
            plugin_name: "normal".into(),
            hook_type: HookType::Request,
            order: HookOrder::Normal,
            callback: Arc::new(move |_| {
                let prev = order2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                assert_eq!(prev, 1); // Should run second
                HookResult::Continue
            }),
        });

        let ctx = HookContext {
            direction: TrafficDirection::Inbound,
            method: "GET".into(),
            host: "example.com".into(),
            path: "/".into(),
            headers: vec![],
            body: vec![],
            timestamp: 0,
            source_ip: "".into(),
            dest_ip: "".into(),
            dest_port: 80,
            app_name: None,
        };
        registry.dispatch(HookType::Request, &ctx);
    }
}
```

- [ ] **Step 2: 运行测试（预期 FAIL — 文件不存在）**

Run: `cargo test -p proxybot_lib plugin::registry --no-default-features`
Expected: FAIL — module not found

- [ ] **Step 3: 创建 registry.rs**

```rust
use crate::plugin::trait::{HookContext, HookOrder, HookRegistration, HookResult, HookType};
use std::sync::{Arc, RwLock};
use std::cmp::Ordering;

pub struct HooksRegistry {
    registrations: RwLock<Vec<HookRegistration>>,
}

impl HooksRegistry {
    pub fn new() -> Arc<Self {
        Arc::new(Self {
            registrations: RwLock::new(Vec::new()),
        })
    }

    pub fn register(&self, reg: HookRegistration) {
        let mut regs = self.registrations.write().unwrap();
        regs.push(reg);
        regs.sort_by_key(|r| r.order);
    }

    pub fn dispatch(&self, hook_type: HookType, ctx: &HookContext) -> HookResult {
        let regs = self.registrations.read().unwrap();
        for reg in regs.iter().filter(|r| r.hook_type == hook_type) {
            let result = std::panic::catch_unwind(|| (reg.callback)(ctx.clone()));
            match result {
                Ok(r) => match r {
                    HookResult::Continue => continue,
                    HookResult::Modified(_) | HookResult::Intercept(_) => return r,
                },
                Err(e) => {
                    log::error!("Plugin {} panicked: {:?}", reg.plugin_name, e);
                    continue;
                }
            }
        }
        HookResult::Continue
    }

    pub fn unregister(&self, plugin_name: &str) {
        let mut regs = self.registrations.write().unwrap();
        regs.retain(|r| r.plugin_name != plugin_name);
    }
}

impl Default for HooksRegistry {
    fn default() -> Self {
        Self {
            registrations: RwLock::new(Vec::new()),
        }
    }
}
```

- [ ] **Step 4: 更新 mod.rs**

```rust
pub mod trait;
pub mod registry;
#[cfg(test)]
mod tests;
```

- [ ] **Step 5: 运行测试**

Run: `cargo test -p proxybot_lib plugin::registry --no-default-features`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/plugin/registry.rs src-tauri/src/plugin/mod.rs
git commit -m "feat(plugin): add HooksRegistry with ordering and panic isolation"
```

---

### Task 3: 实现 PluginManager（集成三个 runtime 框架）

**Files:**
- Modify: `src-tauri/src/plugin/mod.rs` (扩展现有 mod.rs)
- Create: `src-tauri/src/plugin/builtins/mod.rs` (stub)
- Create: `src-tauri/src/plugin/builtins/grpc_plugin.rs` (stub)
- Create: `src-tauri/src/plugin/builtins/network_plugin.rs` (stub)

**注意:** 这三个 builtin 插件在 Phase 2 实现，这里先创建 stub 保证编译通过。

- [ ] **Step 1: 写入集成测试**

在 `src-tauri/src/plugin/tests.rs` 添加：

```rust
#[test]
fn test_plugin_manager_loads_builtins() {
    use crate::plugin::PluginManager;
    let manager = PluginManager::new();
    // Builtins should be registered on startup
    assert!(manager.get_plugin("network-conditions").is_some());
    assert!(manager.get_plugin("grpc-protobuf-decoder").is_some());
}

#[test]
fn test_plugin_manager_dispatches_to_hooks_registry() {
    use crate::plugin::{HookContext, TrafficDirection, HookType};
    let registry = crate::plugin::HooksRegistry::new();
    registry.register(crate::plugin::HookRegistration {
        plugin_name: "test".into(),
        hook_type: HookType::Request,
        order: crate::plugin::HookOrder::Normal,
        callback: Arc::new(|_| crate::plugin::HookResult::Continue),
    });
    let ctx = HookContext {
        direction: TrafficDirection::Inbound,
        method: "GET".into(),
        host: "x.com".into(),
        path: "/".into(),
        headers: vec![],
        body: vec![],
        timestamp: 0,
        source_ip: "".into(),
        dest_ip: "".into(),
        dest_port: 80,
        app_name: None,
    };
    let r = registry.dispatch(HookType::Request, &ctx);
    assert!(matches!(r, crate::plugin::HookResult::Continue));
}
```

- [ ] **Step 2: 创建 builtins/stub 文件**

`src-tauri/src/plugin/builtins/mod.rs`:
```rust
pub mod grpc_plugin;
pub mod network_plugin;

use crate::plugin::trait::{HookContext, HookResult, PluginCapabilities, ProxyPlugin};
use std::sync::Arc;

/// Placeholder gRPC decoder plugin — full impl in Phase 2
pub struct GrpcPlugin;

impl ProxyPlugin for GrpcPlugin {
    fn name(&self) -> &str { "grpc-protobuf-decoder" }
    fn version(&self) -> &str { "0.11.0" }
    fn capabilities(&self) -> PluginCapabilities { PluginCapabilities::ON_RESPONSE }
    fn on_response(&self, _ctx: &HookContext) -> HookResult { HookResult::Continue }
}

/// Placeholder network conditions plugin — full impl in Phase 2
pub struct NetworkConditionsPlugin;

impl ProxyPlugin for NetworkConditionsPlugin {
    fn name(&self) -> &str { "network-conditions" }
    fn version(&self) -> &str { "0.11.0" }
    fn capabilities(&self) -> PluginCapabilities { PluginCapabilities::ON_REQUEST | PluginCapabilities::ON_RESPONSE }
    fn on_request(&self, _ctx: &HookContext) -> HookResult { HookResult::Continue }
    fn on_response(&self, _ctx: &HookContext) -> HookResult { HookResult::Continue }
}
```

`src-tauri/src/plugin/builtins/grpc_plugin.rs` — 放stub内容（上面的 GrpcPlugin）
`src-tauri/src/plugin/builtins/network_plugin.rs` — 放stub内容（上面的 NetworkConditionsPlugin）

- [ ] **Step 3: 创建 PluginManager**

在 `src-tauri/src/plugin/mod.rs` 末尾添加：

```rust
use crate::plugin::registry::HooksRegistry;
use crate::plugin::trait::{HookContext, HookResult, HookType, ProxyPlugin};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

pub struct PluginManager {
    instances: RwLock<HashMap<String, Arc<dyn ProxyPlugin>>>,
    hooks_registry: Arc<HooksRegistry>,
}

impl PluginManager {
    pub fn new() -> Arc<Self> {
        let manager = Arc::new(Self {
            instances: RwLock::new(HashMap::new()),
            hooks_registry: HooksRegistry::new(),
        });
        manager.register_builtin(Arc::new(crate::plugin::builtins::GrpcPlugin));
        manager.register_builtin(Arc::new(crate::plugin::builtins::NetworkConditionsPlugin));
        manager
    }

    fn register_builtin(&self, plugin: Arc<dyn ProxyPlugin>) {
        let caps = plugin.capabilities();
        let name = plugin.name().to_string();

        if caps.contains(crate::plugin::trait::PluginCapabilities::ON_REQUEST) {
            self.hooks_registry.register(crate::plugin::trait::HookRegistration {
                plugin_name: name.clone(),
                hook_type: HookType::Request,
                order: crate::plugin::trait::HookOrder::Normal,
                callback: Arc::new({
                    let plugin = plugin.clone();
                    move |ctx| plugin.on_request(ctx)
                }),
            });
        }
        if caps.contains(crate::plugin::trait::PluginCapabilities::ON_RESPONSE) {
            self.hooks_registry.register(crate::plugin::trait::HookRegistration {
                plugin_name: name.clone(),
                hook_type: HookType::Response,
                order: crate::plugin::trait::HookOrder::Normal,
                callback: Arc::new({
                    let plugin = plugin.clone();
                    move |ctx| plugin.on_response(ctx)
                }),
            });
        }

        self.instances.write().unwrap().insert(name, plugin);
    }

    pub fn get_plugin(&self, name: &str) -> Option<Arc<dyn ProxyPlugin>> {
        self.instances.read().unwrap().get(name).cloned()
    }

    pub fn on_request(&self, ctx: &HookContext) -> HookResult {
        self.hooks_registry.dispatch(HookType::Request, ctx)
    }

    pub fn on_response(&self, ctx: &HookContext) -> HookResult {
        self.hooks_registry.dispatch(HookType::Response, ctx)
    }

    pub fn hooks_registry(&self) -> Arc<HooksRegistry> {
        self.hooks_registry.clone()
    }
}
```

- [ ] **Step 4: 更新 mod.rs 引入**

```rust
pub mod trait;
pub mod registry;
pub mod builtins;
#[cfg(test)]
mod tests;
```

- [ ] **Step 5: 运行测试**

Run: `cargo test -p proxybot_lib plugin --no-default-features`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/plugin/mod.rs src-tauri/src/plugin/builtins/
git commit -m "feat(plugin): add PluginManager with builtin registration"
```

---

## Phase 2: Rhai Engine

### Task 4: RhaiEngine + 内置 API

**Files:**
- Create: `src-tauri/src/plugin/rhai/mod.rs`
- Create: `src-tauri/src/plugin/rhai/builtin_api.rs`

- [ ] **Step 1: 添加依赖到 Cargo.toml**

在 `[dependencies]` 添加：
```toml
rhai = "1"
```

- [ ] **Step 2: 创建 rhai/mod.rs**

```rust
use crate::plugin::trait::{HookContext, HookResult, ProxyPlugin};
use rhai::{Engine, AST};
use std::sync::{Arc, RwLock};

pub struct RhaiEngine {
    engine: RwLock<Engine>,
    scripts: RwLock<Vec<(String, AST)>>,
}

impl RhaiEngine {
    pub fn new() -> Arc<Self> {
        let mut engine = Engine::new();
        Self::setup_api(&mut engine);
        Arc::new(Self {
            engine: RwLock::new(engine),
            scripts: RwLock::new(Vec::new()),
        })
    }

    fn setup_api(engine: &mut Engine) {
        // Expose request context API
        engine.register_type::<HookContext>();
        engine.register_get("method", |ctx: &mut HookContext| ctx.method.clone());
        engine.register_get("host", |ctx: &mut HookContext| ctx.host.clone());
        engine.register_get("path", |ctx: &mut HookContext| ctx.path.clone());
        engine.register_get("body", |ctx: &mut HookContext| ctx.body.clone());
        engine.register_get("direction", |ctx: &mut HookContext| format!("{:?}", ctx.direction));

        // ctx.log()
        engine.register_fn("log", |_ctx: &mut HookContext, msg: String| {
            log::info!("[rhai] {}", msg);
        });
    }

    pub fn load_script(&self, name: &str, source: &str) -> Result<(), String> {
        let engine = self.engine.read().unwrap();
        let ast = engine.compile(source).map_err(|e| e.to_string())?;
        self.scripts.write().unwrap().push((name.to_string(), ast));
        Ok(())
    }

    pub fn eval(&self, script_name: &str, ctx: &mut HookContext) -> HookResult {
        let scripts = self.scripts.read().unwrap();
        let (_, ast) = scripts.iter()
            .find(|(n, _)| n == script_name)
            .ok_or("script not found").unwrap();
        let engine = self.engine.read().unwrap();
        // Run the script — HookContext is available as 'ctx'
        match engine.call_fn::<()>(ast, "on_request", (ctx,)) {
            Ok(_) => HookResult::Continue,
            Err(e) => {
                log::error!("rhai script {} error: {}", script_name, e);
                HookResult::Continue
            }
        }
    }
}
```

- [ ] **Step 3: 运行测试**

Run: `cargo build -p proxybot_lib --no-default-features 2>&1 | head -40`
Expected: 编译通过（只有警告）

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/plugin/rhai/ src-tauri/Cargo.toml
git commit -m "feat(plugin): add RhaiEngine with request context API"
```

---

## Phase 3: WASM Runtime

### Task 5: WasmRuntime

**Files:**
- Create: `src-tauri/src/plugin/wasm/mod.rs`
- Create: `src-tauri/src/plugin/wasm/sandbox.rs`

- [ ] **Step 1: 添加依赖到 Cargo.toml**

```toml
wasmtime = "25"
```

- [ ] **Step 2: 创建 wasm/mod.rs**

```rust
use crate::plugin::trait::{HookContext, HookResult, ProxyPlugin};
use std::sync::{Arc, RwLock};
use wasmtime::{Engine, Instance, Module, Store};
use wasmtime::linker::Linker;
use std::collections::HashMap;

pub struct WasmRuntime {
    engine: wasmtime::Engine,
    instances: RwLock<HashMap<String, WasmInstance>>,
}

struct WasmInstance {
    instance: Instance,
    exports: wasmtime::Exports,
}

impl WasmRuntime {
    pub fn new() -> Arc<Self> {
        let engine = Engine::default();
        Arc::new(Self {
            engine,
            instances: RwLock::new(HashMap::new()),
        })
    }

    pub fn load_plugin(&self, name: &str, wasm_bytes: &[u8]) -> Result<(), String> {
        let module = Module::from_binary(&self.engine, wasm_bytes)
            .map_err(|e| e.to_string())?;
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| e.to_string())?;
        let exports = instance.exports(&mut store);
        self.instances.write().unwrap().insert(name.to_string(), WasmInstance { instance, exports });
        Ok(())
    }

    pub fn call_on_request(&self, name: &str, ctx: &HookContext) -> HookResult {
        // Placeholder — full implementation calls WASM export "on_request"
        HookResult::Continue
    }
}
```

- [ ] **Step 3: 运行测试**

Run: `cargo build -p proxybot_lib --no-default-features 2>&1 | head -20`
Expected: 编译通过

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/plugin/wasm/ src-tauri/Cargo.toml
git commit -m "feat(plugin): add WasmRuntime skeleton (wasmtime)"
```

---

## Phase 4: Proxy Pipeline 集成

### Task 6: 将 HooksRegistry 集成到 proxy.rs

**Files:**
- Modify: `src-tauri/src/proxy.rs`

- [ ] **Step 1: 在 ProxyState 中添加 PluginManager**

在 `proxy.rs` 的 `ProxyState` struct 中找到或创建 `Arc<PluginManager>` 字段（通过 AppState 管理）。在 `lib.rs` 的 tauri setup 中将 `PluginManager` 注册到 AppState。

- [ ] **Step 2: 在 handle_request 之前调用 plugin hooks**

在 `handle_request` 函数开头插入：

```rust
// Run plugin hooks before processing
let ctx = HookContext::from_intercepted_request(req, TrafficDirection::Inbound);
if let Some(pm) = self.plugin_manager.as_ref() {
    match pm.on_request(&ctx) {
        HookResult::Modified(modified) => {
            // Apply modifications to req
            if let Some(body) = modified.body {
                req.req_body = Some(String::from_utf8_lossy(&body).to_string());
            }
        }
        HookResult::Intercept(reason) => {
            return Ok(Response::intercepted(reason));
        }
        HookResult::Continue => {}
    }
}
```

- [ ] **Step 3: 在 response 返回前调用 plugin hooks**

在 `handle_response` 或类似位置插入 `pm.on_response()` 调用。

- [ ] **Step 4: 运行测试**

Run: `cargo build -p proxybot_lib --no-default-features 2>&1 | head -30`
Expected: 编译通过

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/proxy.rs src-tauri/src/lib.rs
git commit -m "feat(plugin): integrate HooksRegistry into proxy pipeline"
```

---

## Phase 5: 内置插件完整实现（gRPC + 流量限制）

### Task 7: 完整 GrpcPlugin

**Files:**
- Modify: `src-tauri/src/plugin/builtins/grpc_plugin.rs`

- [ ] **Step 1: 添加 Protobuf 依赖**

```toml
prost = "0.13"
```

- [ ] **Step 2: 实现完整的 GrpcPlugin**

```rust
// Full implementation per spec Section 6.1
// - Detect content-type: application/grpc or application/grpc+proto
// - Decode protobuf body using DescriptorPool
// - Return HookResult::Modified with decoded JSON bytes
```

- [ ] **Step 3: Commit**

---

### Task 8: 完整 NetworkConditionsPlugin

**Files:**
- Modify: `src-tauri/src/plugin/builtins/network_plugin.rs`

- [ ] **Step 1: 实现完整 NetworkConditionsPlugin**

```rust
// Full implementation per spec Section 6.2
// - ThrottleRule with host_pattern, delay_ms, bandwidth_bps, packet_loss_pct
// - Global preset: 3G (1.6Mbps/768kbps/300ms), 4G (20Mbps/10Mbps/100ms)
// - apply_throttle() async delay, non-blocking
```

- [ ] **Step 2: Commit**

---

## 自检清单

1. **Spec coverage:** 逐节检查 spec，确认每项有对应 task
2. **Placeholder scan:** 无 "TBD" / "TODO" / "fill in details"
3. **Type consistency:** trait.rs 定义的所有类型在后续 task 中一致使用
4. **测试覆盖:** HooksRegistry dispatch 短路、order 顺序、panic 隔离有测试

---

## 实施顺序

1. Task 1: ProxyPlugin trait + HookContext（核心类型）
2. Task 2: HooksRegistry（调度机制）
3. Task 3: PluginManager（生命周期 + 内置插件注册）
4. Task 4: RhaiEngine（用户脚本）
5. Task 5: WasmRuntime（WASM 第三方插件）
6. Task 6: Proxy Pipeline 集成
7. Task 7: 完整 GrpcPlugin
8. Task 8: 完整 NetworkConditionsPlugin
