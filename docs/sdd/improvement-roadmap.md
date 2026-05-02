# ProxyBot 改进路线图 SDD

## Status: Draft

## 1. Context

ProxyBot 当前定位：macOS TUI 工具，pf 透明代理 + DNS 服务器 + App 分类（微信/抖音/支付宝）。

竞品分析结论（见 `competitors-analysis.md`）：
- **核心优势**: App 分类、pf 透明代理、TUI
- **最大短板**: 规则系统弱、无 GUI、无流量篡改

本 SDD 制定从现在到 Phase 2 结束的完整改进计划。

---

## 2. 改进优先级矩阵

| 优先级 | 改进项 | 竞品参考 | 工作量 | 价值 |
|--------|--------|---------|--------|------|
| P0 | 规则系统升级 | mitmproxy / Proxyman | M | 高 |
| P0 | Tauri GUI | Proxyman / HTTP Toolkit | L | 高 |
| P1 | Breakpoint 断点拦截 | mitmproxy | S | 中 |
| P1 | Android 无代理抓包 | HTTP Toolkit | M | 中 |
| P1 | 自动 CA 配置引导 | HTTP Toolkit | S | 中 |
| P2 | iOS VPN API | Proxyman Atlantis | L | 高 |
| P2 | WebView 调试 | spy-debugger | M | 低 |
| P2 | 文档完善 | mitmproxy | S | 中 |

---

## 3. 规则系统升级（Phase 1 核心）

### 3.1 目标

从现有"仅 FILTER（过滤）"升级为完整规则引擎，支持：
- 5 种动作: Direct / Proxy / Reject / MapRemote / MapLocal
- 7 种匹配: Domain / Domain-Suffix / Domain-Keyword / IP-CIDR / GeoIP / RuleSet / Header
- 热重载
- 规则优先级

### 3.2 架构

```rust
// lib.rs / rules.rs

/// 规则动作
pub enum RuleAction {
    Direct,                 // 直连不过代理
    Proxy,                 // 使用代理
    Reject,                // 拒绝连接
    MapRemote(String),     // 映射到远程地址
    MapLocal(String),      // 映射到本地文件/mock
}

/// 规则匹配模式
pub enum RulePattern {
    Domain,           // 精确域名
    DomainSuffix,    // 域名后缀 (e.g., .google.com)
    DomainKeyword,   // 域名关键词
    IpCidr,          // IP 段
    Geoip,           // 地理位置
    RuleSet(String), // 外部规则集
    Header { key: String, value: String }, // Header 匹配
}

/// 单条规则
pub struct Rule {
    pub id: u64,
    pub name: String,
    pub value: String,       // 匹配目标
    pub pattern: RulePattern,
    pub action: RuleAction,
    pub enabled: bool,
    pub priority: u8,        // 0-255，数字越小优先级越高
    pub comment: String,
}

/// 规则引擎
pub struct RulesEngine {
    rules: Vec<Rule>,
    file_watcher: Option<notify::Watcher>,
}
```

### 3.3 MapRemote 实现

```rust
// 伪代码: proxy.rs 或 rules.rs

fn handle_map_remote(req: &InterceptedRequest, target: &str) -> Response {
    // 解析 target: "https://mock.example.com/api/v1"
    // 构造新请求发送到 target
    // 返回 target 的响应
}
```

### 3.4 MapLocal 实现

```rust
fn handle_map_local(req: &InterceptedRequest, file_path: &str) -> Response {
    // 读取本地文件 (JSON/MOCK)
    // 根据 request headers 动态渲染
    // 返回 mock 响应
}

fn render_mock_template(content: &str, req: &InterceptedRequest) -> String {
    // 支持 {{request.method}}, {{request.path}}, {{request.body}}
    // 支持 {{timestamp}}, {{uuid}}
}
```

### 3.5 文件格式

```json
// ~/.proxybot/rules/default.json
{
  "version": 1,
  "rules": [
    {
      "name": "Mock微信API",
      "value": "api.weixin.qq.com",
      "pattern": "DomainSuffix",
      "action": "MapLocal",
      "target": "~/.proxybot/mocks/weixin_api.json",
      "enabled": true,
      "priority": 100,
      "comment": "Mock微信API响应"
    },
    {
      "name": "拒绝广告域名",
      "value": "ads.example.com",
      "pattern": "DomainSuffix",
      "action": "Reject",
      "enabled": true,
      "priority": 50
    }
  ]
}
```

### 3.6 实施步骤

1. 扩展 `RuleAction` 枚举，添加 `MapRemote` / `MapLocal`
2. 扩展 `RulePattern` 枚举，添加 `Header` 匹配
3. 实现 `RulesEngine::apply_map_remote()`
4. 实现 `RulesEngine::apply_map_local()`
5. 实现 mock 模板渲染（Handlebars 或 Tera）
6. 规则文件格式从 YAML 迁移到 JSON（统一）
7. TUI 规则 tab 新增 MapRemote/MapLocal action 选择器
8. 添加规则导入/导出（兼容 Proxyman 格式）

---

## 4. Tauri GUI（Phase 2 核心）

### 4.1 目标

实现跨平台 GUI（非 Electron），复用 Rust 核心逻辑，TUI 保留用于服务器场景。

### 4.2 架构

```
┌─────────────────────────────────────────────────┐
│                  Tauri WebView                   │
│  ┌─────────────────────────────────────────────┐ │
│  │            React + TypeScript UI              │ │
│  │  - shadcn/ui 组件                           │ │
│  │  - TanStack Table (流量列表)                │ │
│  │  - React Query (状态管理)                   │ │
│  └─────────────────────────────────────────────┘ │
│                       │ IPC (invoke)             │
└───────────────────────┼─────────────────────────┘
                        │
┌───────────────────────┼─────────────────────────┐
│                 Rust Core                        │
│  ┌─────────┐ ┌──────┐ ┌──────┐ ┌────────────┐  │
│  │  proxy  │ │  db  │ │ dns  │ │ rules      │  │
│  │  core   │ │      │ │      │ │ engine     │  │
│  └─────────┘ └──────┘ └──────┘ └────────────┘  │
│                                                   │
│  ┌─────────┐ ┌──────┐ ┌──────┐ ┌────────────┐  │
│  │ pf/     │ │cert  │ │tun/  │ │ app        │  │
│  │ network │ │mgr   │ │tun   │ │ classifier │  │
│  └─────────┘ └──────┘ └──────┘ └────────────┘  │
└───────────────────────────────────────────────────┘
```

### 4.3 TUI vs GUI 分工

| 功能 | TUI | GUI |
|------|-----|-----|
| 流量监控 | ✅ | ✅ |
| 规则编辑 | ✅ | ✅ |
| 设备管理 | ✅ | ✅ |
| 证书管理 | ✅ | ✅ |
| 流量详情 | 文本 | 图形化 |
| 流量篡改 | Breakpoint | 图形化表单 |
| Mock 管理 | 命令行 | 图形化 + 文件上传 |
| 截图/导出 | ❌ | ✅ |

### 4.4 实施步骤

1. 初始化 Tauri v2 项目，配置 React + TypeScript
2. 配置 shadcn/ui + 深色主题
3. 实现 IPC 桥接（复用现有 Rust 命令）
4. 流量列表 UI（TanStack Table）
5. 规则编辑器 UI（图形化 rule builder）
6. 设备管理 UI（表格 + 详情面板）
7. 证书管理 UI（一键导入 CA）
8. Mock 管理 UI（文件上传 + 模板预览）

---

## 5. Breakpoint 断点拦截

### 5.1 目标

支持在请求/响应阶段暂停、查看、修改、继续发送。

### 5.2 状态机

```
                    ┌──────────────┐
                    │   IDLE      │
                    └──────┬───────┘
                           │ 用户按 [b]
                           ▼
                    ┌──────────────┐
         ┌─────────│  REQUEST     │─────────┐
         │         │  BREAKPOINT   │         │
         │         └──────┬───────┘         │
         │                │                  │
    用户按 [c]           │ 用户按 [g]        │ 用户按 [e]
    (取消请求)            │ (继续发送)         │ (编辑)
         │                │                  ▼
         │                │         ┌──────────────┐
         │                │         │   EDITING    │
         │                │         │  (Buffer)    │
         │                │         └──────┬───────┘
         │                │                │
         │                │         用户按 [Enter]
         │                │         (发送修改)
         │                │                │
         ▼                ▼                ▼
    ┌─────────┐    ┌──────────────┐  ┌──────────┐
    │ REJECTED│    │  FORWARDED  │  │ MODIFIED │
    └─────────┘    │  (已发送)    │  └──────────┘
                   └──────────────┘
                           │
                           ▼
                   ┌──────────────┐
                   │  RESPONSE    │
                   │  BREAKPOINT  │ (如果有响应断点)
                   └──────────────┘
```

### 5.3 TUI 实现

```rust
// TuiApp 新增状态
pub struct TuiApp {
    // ... 现有字段
    pub breakpoint: BreakpointState,
}

pub struct BreakpointState {
    pub mode: BreakpointMode,
    pub paused_req: Option<InterceptedRequest>,
    pub edit_buffer: InterceptedRequest,
}

#[derive(PartialEq)]
pub enum BreakpointMode {
    None,
    RequestPaused,   // 请求已暂停，等待处理
    ResponsePaused,  // 响应已暂停，等待处理
    Editing,         // 用户正在编辑
}
```

### 5.4 TUI Breakpoint 快捷键

| 快捷键 | 动作 |
|--------|------|
| `b` | 对选中请求开启/关闭断点 |
| `g` | 继续发送（GO） |
| `c` | 取消请求（CANCEL） |
| `e` | 编辑请求（EDIT） |
| `Tab` | 切换编辑字段（method/url/headers/body） |
| `↑/↓` | 编辑 header 时切换 header |
| `Enter` | 发送修改后的请求 |

---

## 6. Android 无代理抓包

### 6.1 目标

手机无需设置 HTTP 代理，通过 adb reverse 实现透明劫持。

### 6.2 技术方案

**HTTP Toolkit 方案**:
1. Android 开启开发者模式 + USB 调试
2. `adb reverse tcp:8088 tcp:8088` 将手机流量转发到 Mac
3. Android 设备通过 USB 连接，不需要在同一局域网

**ProxyBot 扩展**:
```rust
// tun.rs 新增
pub struct AdbReverse {
    device_serial: String,
    local_port: u16,
    remote_port: u16,
}

impl AdbReverse {
    pub fn new(device_serial: &str) -> Self { ... }
    pub fn start(&self) -> Result<(), String> { ... }
    pub fn stop(&self) { ... }
}
```

### 6.3 实施步骤

1. 实现 `AdbReverse` 结构体（调用 adb 命令）
2. 检测连接设备列表 (`adb devices`)
3. GUI 新增"设备"面板，显示已连接 Android 设备
4. 一键 `adb reverse` 按钮
5. 自动检测设备代理状态

---

## 7. 自动 CA 配置引导

### 7.1 目标

用户友好的一键 CA 证书安装引导，减少"证书错误"问题。

### 7.2 流程

```
┌─────────────────────────────────────────────┐
│  Step 1: 生成 CA 证书                        │
│  [Proceed]                                  │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│  Step 2: 安装 CA 到系统                      │
│                                             │
│  macOS:                                     │
│  1. 打开 ~/.proxybot/ca.crt                 │
│  2. 添加到系统登录项 Keychain               │
│  3. 始终信任该证书                          │
│                                             │
│  [Open Certificate] [Proceed] [Skip]        │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│  Step 3: 验证安装                           │
│  [Test Connection]                           │
└─────────────────────────────────────────────┘
```

### 7.3 TUI 实现

```rust
// cert.rs / tui/mod.rs

pub enum CertInstallStep {
    Generate,
    InstallInstructions,
    Verify,
}

pub struct CertWizardState {
    pub current_step: CertInstallStep,
    pub cert_path: PathBuf,
    pub test_result: Option<bool>,
}
```

### 7.4 实施步骤

1. 实现 `CertWizardState` 和步骤管理
2. TUI 新增 `[c]` cert wizard 命令（Certs tab）
3. 检测系统类型，生成对应安装指令
4. 实现连接测试 (`curl --cacert ca.crt https://example.com`)
5. GUI 集成系统证书安装 API

---

## 8. iOS VPN API（长期目标）

### 8.1 目标

iOS 设备无需设置代理，通过 VPN API 抓包。

### 8.2 Proxyman Atlantis 技术方案

```
┌─────────────────┐         VPN Tunnel         ┌─────────────────┐
│   iOS Device    │◄──────────────────────────►│   Mac ProxyBot   │
│                 │                            │                 │
│ NEPacketTunnel  │   所有流量走此 Tunnel      │  处理并转发      │
│    Provider     │                            │                 │
└─────────────────┘                            └─────────────────┘
```

### 8.3 实施步骤

1. 研究 macOS NEPacketTunnelProvider API
2. 实现 PacketTunnelProvider（Rust -> FFI 或 Swift）
3. 实现 ProxyBot VPN Server（接收 tunnel 流量）
4. iOS 端 Configuration Profile 自动配置
5. TUI/GUI 新增 VPN 连接管理

**难度**: 高（需要 Apple Developer Program + 网络扩展权限）

---

## 9. WebView 调试

### 9.1 目标

调试移动端 WebView 页面，类似 spy-debugger 的 weinre 能力。

### 9.2 技术方案

```
┌─────────────┐     CDP      ┌─────────────┐
│   iOS       │◄────────────►│  DevTools   │
│  WebView    │   WebSocket  │   Frontend  │
└─────────────┘              └─────────────┘
```

### 9.3 实施步骤

1. 实现 CDP (Chrome DevTools Protocol) 服务端
2. 实现 WebKit 远程调试桥接
3. TUI 新增 WebView tab（显示页面列表）
4. 点击页面后启动远程 DevTools
5. 复用现有 Browser DevTools 前端（开源）

**难度**: 中（CDP 实现较复杂）

---

## 10. 文件结构

```
docs/sdd/
├── competitors-analysis.md     # 竞品分析（已完成）
├── competitor-improvements.md  # 取长补短计划（已完成）
├── improvement-roadmap.md      # 本文件 - 详细实施计划
├── rule-system-sdd.md         # [待生成] 规则系统详细设计
├── gui-sdd.md                 # [待生成] Tauri GUI 详细设计
└── breakpoint-sdd.md          # [待生成] Breakpoint 详细设计
```

---

## 11. 里程碑

| 版本 | 内容 | 目标 |
|------|------|------|
| v0.2.0 | 规则系统升级（MapRemote/MapLocal） | 1-2周 |
| v0.3.0 | Breakpoint 断点 + 自动 CA 引导 | 2-3周 |
| v0.4.0 | Tauri GUI Alpha | 1-2月 |
| v0.5.0 | Android adb reverse + GUI 完善 | 2-3月 |
| v1.0.0 | Phase 2 完成：GUI + 规则系统 + 断点 | 半年 |

---

## 12. 验证

```bash
# 规则系统
cargo build --lib && cargo test --lib

# Tauri GUI
cd src-webview && npm run tauri build

# Breakpoint
cargo build --bin proxybot-tui && ./proxybot-tui
# 按 b 测试断点功能

# Android adb
adb devices  # 确认设备连接
adb reverse tcp:8088 tcp:8088  # 手动测试
```
