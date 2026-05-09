# ProxyBot v0.11.0 集成插件架构设计

## Status: Draft

## 1. Context & Goals

ProxyBot v0.11.0 需要一个统一的插件架构，同时支撑：
1. **用户自定义扩展** — Rhai 脚本 + WASM 第三方插件
2. **内置功能模块化** — gRPC/Protobuf 解码、流量限制（Network Conditions）

四个功能共享同一套插件基础设施：
- 插件系统（Rust trait 统一接口）
- 脚本引擎（Rhai）
- 协议解析（gRPC/Protobuf）
- 流量控制（throttle/latency/packet loss）

---

## 2. 总体架构

```
ProxyBot Core
    │
    ├── HooksRegistry (统一的 hooks 调用点)
    │       HookType: Request / Response / Connect / Error / TrafficLog
    │       HookOrder: Earliest → Normal → Late → Last
    │
    └── PluginManager (统一插件生命周期管理)
            │
            ├── RhaiEngine
            │       ├── Scripts: ~/.proxybot/scripts/*.rhai
            │       └── Builtin scripts: gRPC 解码、流量限制（预加载）
            │
            ├── WasmRuntime (wasmtime)
            │       └── Plugins: ~/.proxybot/plugins/*.wasm (第三方)
            │
            └── NativeRegistry
                    └── Builtins: Rust 实现的官方插件

    每个 plugin 实现 ProxyPlugin trait，通过 HooksRegistry 统一调度。
```

**设计原则：**
- `ProxyPlugin` trait 是所有插件的统一接口，无论来源
- 三个 runtime 各自独立，PluginManager 统一调度
- 官方内置功能（gRPC 解析、限速）通过 NativeRegistry 以 Rust trait 实现注入
- 用户插件通过 Rhai 或 WASM 接入

---

## 3. 核心接口（ProxyPlugin Trait）

### 3.1 基础类型

```rust
/// 插件能力位掩码（插件可以选择性注册 hooks）
bitflags::bitflags! {
    pub struct PluginCapabilities: u32 {
        const ON_REQUEST      = 1 << 0;
        const ON_RESPONSE     = 1 << 1;
        const ON_CONNECT     = 1 << 2;
        const ON_ERROR       = 1 << 3;
        const ON_TRAFFIC_LOG  = 1 << 4;
    }
}

/// 流量方向
pub enum TrafficDirection {
    Inbound,
    Outbound,
}

/// 拦截上下文（传递给插件的请求/响应信息）
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
}

/// 插件可以修改的内容
pub struct ModifiedContent {
    pub headers: Option<Vec<(String, String)>>,
    pub body: Option<Vec<u8>>,
    pub intercept: bool,
    pub log_message: Option<String>,
}

/// 插件的 hook 返回结果
pub enum HookResult {
    Continue,
    Modified(ModifiedContent),
    Intercept(String),
}

/// 统一插件 trait
pub trait ProxyPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn capabilities(&self) -> PluginCapabilities;
    fn on_request(&self, ctx: &HookContext) -> HookResult { HookResult::Continue }
    fn on_response(&self, ctx: &HookContext) -> HookResult { HookResult::Continue }
    fn on_connect(&self, ctx: &HookContext) -> HookResult { HookResult::Continue }
    fn on_error(&self, ctx: &HookContext, error: &str) -> HookResult { HookResult::Continue }
    fn on_traffic_log(&self, ctx: &HookContext) {}
    fn init(&mut self, config: Value) -> Result<(), String>;
    fn shutdown(&self) {}
}
```

**插件按需注册** — 不需要实现全部 hooks，按 capabilities 选择性实现。

---

## 4. HooksRegistry（统一调度）

```rust
pub enum HookOrder {
    Earliest,   // 日志插件等
    Normal,     // 默认
    Late,       // 注入类插件
    Last,       // 唯一最终处理
}

pub struct HookRegistration {
    pub plugin_name: String,
    pub hook_type: HookType,
    pub order: HookOrder,
    pub callback: Arc<dyn Fn(HookContext) -> HookResult + Send + Sync>,
}

pub struct HooksRegistry {
    registrations: RwLock<Vec<HookRegistration>>,
}

impl HooksRegistry {
    pub fn register(&self, reg: HookRegistration);

    /// 按 hook type + order 排序执行
    /// Modified / Intercept 短路，后续插件不再执行
    /// Continue 继续下一个插件
    pub fn dispatch(&self, hook_type: HookType, ctx: &HookContext) -> HookResult;
}
```

---

## 5. 三个 Runtime 集成

### 5.1 PluginManager

```rust
pub enum PluginInstance {
    Rhai(RhaiPlugin),
    Wasm(WasmPlugin),
    Native(NamedPlugin),
}

pub struct PluginManager {
    instances: RwLock<Vec<PluginInstance>>,
    hooks_registry: Arc<HooksRegistry>,
    rhai_engine: Arc<RhaiEngine>,
    wasm_runtime: Arc<WasmRuntime>,
}

impl PluginManager {
    /// 启动时加载所有插件
    pub fn load_all(&self) -> Result<(), String> {
        // 1. 扫描 ~/.proxybot/scripts/*.rhai → 编译注册
        // 2. 扫描 ~/.proxybot/plugins/*.wasm → 实例化注册
        // 3. 注册内置 Rust 插件
    }

    pub fn reload_plugin(&self, name: &str) -> Result<(), String>;
    pub fn unload_plugin(&self, name: &str);

    /// proxy pipeline 中调用
    pub fn on_request(&self, ctx: &HookContext) -> HookResult {
        self.hooks_registry.dispatch(HookType::Request, ctx)
    }
}
```

### 5.2 Rhai Engine

- 用户脚本：`~/.proxybot/scripts/*.rhai`
- 内置脚本：gRPC 解码、流量限制（预加载）
- 禁用危险 API：FS、网络
- API: `request.*`, `ctx.log()`, `ctx.set_body()`, `ctx.intercept()`, `ctx.get_state()`

### 5.3 WASM Runtime

- 第三方插件：`~/.proxybot/plugins/*.wasm`
- wasmtime 驱动，独立内存空间
- 资源限制：内存上限、CPU 时间
- 插件 panic 不影响主程序（catch_unwind）

### 5.4 NativeRegistry

- 内置插件用 Rust 实现 `ProxyPlugin` trait
- 直接注册到 HooksRegistry
- 无需文件扫描，启动时初始化

---

## 6. 四个内置插件实现

### 6.1 gRPC/Protobuf 解码插件

```rust
pub struct GrpcPlugin {
    descriptor_pool: DescriptorPool,
    decoder: ProtobufDecoder,
}

impl ProxyPlugin for GrpcPlugin {
    fn name(&self) -> &str { "grpc-protobuf-decoder" }
    fn capabilities(&self) -> PluginCapabilities { PluginCapabilities::ON_RESPONSE }
    fn on_response(&self, ctx: &HookContext) -> HookResult {
        if is_grpc_content_type(&ctx.headers) {
            match self.decoder.decode(&ctx.body) {
                Ok(decoded) => HookResult::Modified(ModifiedContent {
                    headers: None,
                    body: Some(decoded.into_bytes()),
                    intercept: false,
                    log_message: Some("gRPC: decoded protobuf".into()),
                }),
                Err(_) => HookResult::Continue,
            }
        } else {
            HookResult::Continue
        }
    }
}
```

### 6.2 流量限制插件

```rust
pub struct NetworkConditionsPlugin {
    rules: Vec<ThrottleRule>,
    global_preset: Option<NetworkPreset>,  // 3G/4G/Custom
}

pub struct ThrottleRule {
    host_pattern: glob::Pattern,
    delay_ms: u64,
    bandwidth_bps: Option<u64>,
    packet_loss_pct: f32,
}

impl ProxyPlugin for NetworkConditionsPlugin {
    fn name(&self) -> &str { "network-conditions" }
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities::ON_REQUEST | PluginCapabilities::ON_RESPONSE
    }
    fn on_request(&self, ctx: &HookContext) -> HookResult {
        if let Some(rule) = self.matching_rule(&ctx.host) {
            self.apply_throttle(rule); // 异步，非阻塞
        }
        HookResult::Continue
    }
}
```

### 6.3 Rhai 内置脚本 API

```rhai
// request - 只读请求上下文
print(request.method);   // "GET"
print(request.host);     // "api.example.com"
print(request.path);    // "/v1/users"

// ctx - 上下文工具
ctx.log("Processing: #{request.path}");
ctx.set_body('{"mocked": true}');
ctx.intercept("Mock response");
ctx.get_state("auth_token");
ctx.set_state("counter", 42);
```

### 6.4 配置文件

```toml
# ~/.proxybot/config.toml
[plugins]
enabled = true
auto_reload = true

[plugins.scripts]
dir = "~/.proxybot/scripts"

[plugins.wasm]
dir = "~/.proxybot/plugins"
trusted = []

[plugins.builtins]
grpc_decoder = { enabled = true, proto_dirs = ["~/.proxybot/protos"] }
network_conditions = { preset = "4G", rules = [] }
```

---

## 7. 数据流

```
手机流量 → ProxyCore::handle_request()
    │
    ▼
HooksRegistry::dispatch(Request, ctx)
    │ 按 HookOrder::Earliest → Normal → Late → Last 顺序执行
    │
    ▼
每个插件的 on_request() 按序执行
    │ HookResult::Modified(ctx) → 短路
    │ HookResult::Intercept(msg) → 短路，返回拦截响应
    │ HookResult::Continue → 执行下一个插件
    │
    ▼
所有插件返回 Continue → ProxyCore 继续正常处理
```

---

## 8. 错误处理

- **单个插件 panic**：`catch_unwind`，记录日志，继续执行下一个插件
- **WASM 超时/内存超限**：杀死插件实例，记录错误
- **Rhai 脚本错误**：捕获 Rhai 异常，返回 `Continue`
- **插件返回错误**：插件自己处理，不影响主流程

---

## 9. 文件结构

```
src-tauri/src/
├── plugin/
│   ├── mod.rs              # PluginManager, PluginInstance
│   ├── trait.rs            # ProxyPlugin trait, HookContext, HookResult
│   ├── registry.rs         # HooksRegistry
│   ├── rhai/
│   │   ├── mod.rs          # RhaiEngine
│   │   ├── builtin_api.rs  # request.*, ctx.*
│   │   └── builtins/       # 内置 Rhai 脚本
│   ├── wasm/
│   │   ├── mod.rs          # WasmRuntime
│   │   └── sandbox.rs     # 资源限制
│   └── builtins/
│       ├── mod.rs          # NativeRegistry
│       ├── grpc_plugin.rs  # gRPC/Protobuf 解码
│       └── network_plugin.rs  # 流量限制
src/components/plugins/     # (future) GUI 插件管理 UI
```

---

## 10. 与 proxy pipeline 集成

在 `proxy.rs` 的 `handle_request` / `handle_response` 中调用：

```rust
// proxy.rs
fn handle_request(&self, req: &HttpRequest) -> ProxyResult<Response> {
    let ctx = HookContext::from_request(req);

    match self.plugin_manager.on_request(&ctx) {
        HookResult::Modified(modified) => {
            // 用 modified 内容继续处理
            self.process_modified(modified, req)
        }
        HookResult::Intercept(reason) => {
            // 返回拦截响应
            Ok(Response::intercepted(reason))
        }
        HookResult::Continue => {
            // 正常处理流程
            self.forward_request(req)
        }
    }
}
```

---

## 11. 实现顺序

1. **ProxyPlugin trait + HooksRegistry** — 核心调度机制
2. **NativeRegistry + 2 个内置插件** — 验证接口，积累经验
3. **RhaiEngine** — 用户脚本支持
4. **WasmRuntime** — 第三方插件支持
5. **Config + 热重载** — 完善插件管理
