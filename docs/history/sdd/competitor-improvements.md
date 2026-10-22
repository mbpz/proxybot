# ProxyBot 取长补短行动计划

## 竞品分析结论

ProxyBot 核心优势（竞品无此组合）:
1. **App 分类** — DNS 关联 + SNI 检测 + 域名规则库
2. **pf 透明代理** — 手机无需设代理
3. **TUI 键盘驱动** — 高效，可远程

ProxyBot 最大短板（需补齐）:
| 短板 | 竞品参考 | 优先级 |
|------|---------|--------|
| 规则系统弱 | Proxyman/mitmproxy | P0 |
| 无 GUI | Proxyman/HTTP Toolkit | P0 |
| 无流量篡改 | mitmproxy | P1 |
| iOS 无代理抓包 | Proxyman Atlantis | P1 |
| 自动 CA 配置 | HTTP Toolkit | P2 |
| 无 WebView 调试 | spy-debugger | P2 |
| 文档不足 | mitmproxy | P2 |

---

## P0: 必须补齐（Phase 2 核心）

### 1. 强化规则系统

**现状**: 只有 FILTER（过滤），无请求修改能力

**竞品参考**:
- Proxyman: Map Remote / Map Local / Breakpoint / Rewrite
- mitmproxy: Flow expressions + inline scripts
- HTTP Toolkit: Request matching + mock responses

**建议实现**:

```rust
// 规则动作扩展
pub enum RuleAction {
    Direct,           // 现有
    Proxy,            // 现有
    Reject,           // 现有
    // 新增:
    MapRemote(String), // 映射到远程地址
    MapLocal(String),  // 映射到本地文件/mock
    Breakpoint,        // 断点拦截（暂停等待修改）
    Rewrite(String),   // 重写规则（正则替换）
}
```

**借鉴点**:
- Proxyman 规则格式兼容（.plist -> JSON）
- mitmproxy 的 Flow 表达式语言（按 method/host/path/header 过滤）
- 规则热重载（anyproxy 的方案）

---

### 2. GUI 界面（Tauri + React）

**现状**: 纯 TUI，非技术人员不友好

**竞品参考**:
- Proxyman: 原生 macOS Swift，流畅但仅 macOS
- HTTP Toolkit: Electron，跨平台但性能一般
- mitmproxy: mitmweb，Python Web

**建议实现**:
- Tauri v2 + React + TypeScript + shadcn/ui
- 复用现有 Rust 核心逻辑（proxy/db/rules/dns）
- WebView 层仅做 UI，不重复实现业务逻辑
- TUI 保留（服务器场景优势）

**借鉴点**:
- HTTP Toolkit UI 审美（深色主题，清晰的信息层级）
- Proxyman 证书管理 UI（一键安装 CA）

---

## P1: 重要增强

### 3. 流量篡改（Breakpoint）

**现状**: 仅监控，无修改能力

**竞品参考**:
- mitmproxy: 可在请求/响应前后断点，可修改任意字段
- Proxyman: Breakpoint 交互式编辑

**建议实现**:

```rust
// 新增 BreakpointState
pub struct BreakpointState {
    pub paused_request: Option<InterceptedRequest>,
    pub paused_response: Option<InterceptedRequest>,
    pub edit_buffer: EditBuffer,
}

// TUI 新增 breakpoint 模式
// - [b] toggle breakpoint on selected request
// - 断点触发时暂停渲染，等待编辑
// - Enter = 发送修改后的请求/响应
// - Esc = 取消并继续
```

**借鉴点**:
- mitmproxy 的交互式编辑体验
- 支持修改: method, url, headers, body

---

### 4. iOS 无代理抓包（VPN API）

**现状**: 需手机设置代理

**竞品参考**:
- Proxyman Atlantis: 使用 NEPacketTunnelProvider（VPN API）实现无需代理抓包
- HTTP Toolkit Android: adb reverse + VPN Service

**技术方案**:

```swift
// iOS: NEPacketTunnelProvider
// macOS ProxyBot 端:
//   1. 启动 VPN Server（类似 pf 但用 NEPacketTunnelProvider）
//   2. iOS 安装 Configuration Profile 连接此 VPN
//   3. VPN tunnel 捕获所有流量
//   4. 通过 tunnel 转发到 ProxyBot 处理
```

**实现难度**: 高（需要 macOS/iOS 端都实现 NEPacketTunnelProvider）

**替代方案**:
- HTTP Toolkit 的 adb reverse 方案（Android）
- 先实现 Android 的类似功能

---

## P2: 体验优化

### 5. 自动 CA 配置

**现状**: 用户需手动安装 CA 证书

**竞品参考**:
- HTTP Toolkit: 自动检测 + 一键安装
- Proxyman: Certificate Provider Extensions（iOS 自动弹窗安装）

**建议实现**:

```rust
// 新增自动 CA 配置引导
pub enum CertInstallStep {
    DetectPlatform,
    GenerateCert,           // 已有
    OpenCertSettings,        // 打开系统证书设置页面
    WaitForTrust,           // 等待用户确认
    VerifyConnection,        // 验证 CA 是否生效
}
```

**借鉴点**:
- HTTP Toolkit 的引导式安装流程
- adb 命令打开 Android 证书安装页面

---

### 6. WebView 调试

**现状**: 无 WebView 调试能力

**竞品参考**:
- spy-debugger: weinre 方案，微信 WebView 调试
- Proxyman: iOS WebKit 远程调试

**建议实现**:

```rust
// 新增 WebView 调试模块
pub struct WebViewDebugState {
    pub remote_debugger_url: String,
    pub inspected_tab_id: Option<u64>,
}

// TUI 新增 [w] 进入 WebView 调试模式
// - 列出所有 WebView（通过 CDP 发现）
// - 点击选择后用 remotedebug-adt 转发
// - 类似 Chrome DevTools 的远程调试体验
```

**借鉴点**:
- spy-debugger 的微信 jssdk 注入
- Proxyman 的 iOS WebKit 调试协议

---

### 7. 文档完善

**竞品参考**:
- mitmproxy: 文档最完善，有官方博客、视频教程
- HTTP Toolkit: 清晰的 Quick Start

**建议补充**:

```
docs/
├── README.md
├── INSTALL.md           # 安装指南（各平台）
├── QUICKSTART.md        # 快速入门
├── ADVANCED.md          # 高级用法（规则/脚本）
├── COMPETITORS.md       # 竞品对比（本分析）
└── TROUBLESHOOTING.md   # 常见问题
```

---

## 具体任务分解

### 立即可执行（1-2天）

- [ ] **文档**: 完善 README，包含 Quick Start 和竞品对比
- [ ] **体验**: 简化 CA 安装流程（新增引导式安装命令）
- [ ] **TUI**: 过滤历史记录（最近 N 个过滤条件快速切换）

### 本季度计划（1-2周）

- [ ] **规则系统 P1**: 实现 MapRemote / MapLocal 规则动作
- [ ] **规则系统 P2**: 实现 Breakpoint 断点拦截
- [ ] **文档**: 补充 INSTALL.md / TROUBLESHOOTING.md

### Phase 2 规划（1-2月）

- [ ] **GUI**: Tauri v2 + React 实现（复用 Rust 核心）
- [ ] **Android**: adb reverse 实现无代理抓包
- [ ] **iOS**: VPN API 研究和原型

---

## 参考项目核心技术亮点

| 亮点 | 来源 | 可借鉴程度 |
|------|------|----------|
| Flow 表达式语言 | mitmproxy | 高（规则引擎升级） |
| Atlantis VPN API | Proxyman | 高（iOS 无代理抓包） |
| 自动 CA 配置 | HTTP Toolkit | 高（安装体验优化） |
| 微信 jssdk 注入 | spy-debugger | 中（移动端特色） |
| 项目制管理 | Hetty | 低（暂不需要） |
| Mock Server 内置 | HTTP Toolkit | 中（Gen tab 已覆盖） |

---

## 第四轮竞品分析新增行动计划 (May 2026)

基于 proxelar、anything-analyzer、hyperfox、InterceptSuite、forwarder、gomitmproxy 的深度分析。

### P0: 必须立即实现

#### 1. MCP Server 集成

**现状**: ProxyBot 无法被 AI Agent 直接调用
**竞品参考**: anything-analyzer 内置 MCP Server，将抓包/分析能力暴露为 MCP 工具
**建议实现**:

```rust
// MCP Server 暴露的工具
#[mcp_tool]
async fn capture_traffic(url: String) -> Vec<Request> { ... }

#[mcp_tool]
async fn classify_app(host: String) -> AppTag { ... }

#[mcp_tool]
async fn analyze_api(session_id: String) -> ApiSpec { ... }

#[mcp_tool]
async fn get_rules() -> Vec<Rule> { ... }
```

**收益**: Claude Desktop / Cursor / Windsurf 可直接调用 ProxyBot 进行流量分析
**预计工期**: 3-5 天

---

### P1: 高优先级增强

#### 2. AI 两阶段分析管道

**现状**: Gen tab 的 LLM 推断直接处理全部请求，噪声多、token 成本高
**竞品参考**: anything-analyzer Phase 1 (智能过滤) → Phase 2 (深度分析)
**建议实现**:

```
Phase 1: 智能过滤
├── 去重 (相同 endpoint + method)
├── 过滤静态资源 (images/css/fonts)
├── 过滤第三方 SDK 请求 (analytics/crash)
├── 聚类相似请求 (RESTful 参数化)
└── 输出: 去噪后的候选 API 列表

Phase 2: 深度分析
├── API 端点文档生成
├── 鉴权流程识别 (OAuth/JWT/API Key)
├── 请求/响应 schema 推断
├── 生成复现代码 (Python/curl/Go)
└── Token 使用量估算 (AI API)
```

**收益**: AI 推断质量提升，token 成本降低 ~60%
**预计工期**: 1 周

#### 3. 列作用域过滤 DSL

**现状**: Filter DSL 基于 AND/OR/NOT + glob，语法较复杂
**竞品参考**: proxelar `method:POST host:api status:200 type:json`
**建议实现**:

```rust
// 集成到现有 Filter DSL
// 简化语法: column:value 自动映射到对应列
"method:POST host:*.example.com status:2* type:json duration:slow"

// 保留高级 DSL 用于复杂查询
"(method:GET OR method:POST) AND (status:4* OR status:5*)"
```

**收益**: 过滤效率提升 3-5x，降低学习曲线
**预计工期**: 2-3 天

---

### P2: 体验优化

#### 4. QR code CA 分发

**现状**: CA 证书需手动 AirDrop/邮件，步骤多
**竞品参考**: hyperfox 生成 QR code 供手机扫码下载；proxelar `http://proxel.ar` 内置 HTTP 下载
**建议实现**:

```rust
// CA 安装向导
1. 生成 QR code (CA PEM URL)
2. 启动本地 HTTP server 提供证书下载
3. 手机扫码 → 下载 → 安装 profile
4. 验证安装 (检测请求是否被成功解密)
```

**收益**: CA 安装从 5 步降到 2 步 (扫码 → 授权)
**预计工期**: 1 天

#### 5. proxybot-core 独立 crate

**现状**: 核心代理引擎与 Tauri/TUI 紧耦合
**竞品参考**: gomitmproxy (AdGuard)、mitmproxy-rs 将 MITM 封装为独立库
**建议实现**:

```toml
# Cargo.toml
[workspace]
members = [
    "proxybot-core",    # 核心代理引擎 (可独立发布)
    "proxybot-tui",     # TUI 界面
    "proxybot-tauri",   # Tauri GUI
    "proxybot-mcp",     # MCP Server
]
```

**收益**: 
- 其他 Rust 项目可直接引用 proxybot-core
- 降低贡献门槛 (不需要理解 GUI 代码)
- 扩大生态影响力
**预计工期**: 1 周

#### 6. 项目文件管理

**现状**: 所有流量写入单一 SQLite，无会话隔离
**竞品参考**: InterceptSuite 项目制管理；hyperfox 每会话独立 DB
**建议实现**:

```rust
// Workspace 管理
pub struct Workspace {
    pub name: String,
    pub db_path: PathBuf,      // 独立 SQLite
    pub rules: Vec<Rule>,      // 工作区规则
    pub devices: Vec<Device>,  // 关联设备
    pub created_at: DateTime,
}
```

**收益**: 按测试任务/项目隔离流量，支持导出分享
**预计工期**: 3-4 天

---

### P3: 研究性任务

#### 7. HTTP/3 & QUIC 支持

**现状**: 零个开源代理支持 HTTP/3 解密
**竞品参考**: 无 (蓝海)
**研究方向**:
- QUIC 连接迁移 vs MITM
- HTTP/3 QPACK header 压缩
- 0-RTT 会话恢复的安全影响
**预计工期**: 2+ 周研究

#### 8. 传输层 TCP/UDP 代理

**现状**: ProxyBot 仅处理 HTTP/HTTPS/WSS
**竞品参考**: InterceptSuite TCP/UDP/DTLS
**建议**: 研究 MQTT/CoAP/gRPC-streaming 等非 HTTP 协议代理可行性
**预计工期**: 2-3 周原型

---

## 更新后的优先级总览

| # | 方向 | 来源 | 优先级 | 预计工期 |
|---|------|------|--------|----------|
| 1 | **MCP Server** | anything-analyzer | P0 | 3-5 天 |
| 2 | **AI 两阶段分析管道** | anything-analyzer | P1 | 1 周 |
| 3 | **列作用域过滤 DSL** | proxelar | P1 | 2-3 天 |
| 4 | **QR code CA 分发** | hyperfox/proxelar | P2 | 1 天 |
| 5 | **proxybot-core crate** | gomitmproxy/mitmproxy-rs | P2 | 1 周 |
| 6 | **项目文件管理** | InterceptSuite/hyperfox | P2 | 3-4 天 |
| 7 | **HTTP/3 & QUIC** | (蓝海) | P3 | 2+ 周 |
| 8 | **传输层代理** | InterceptSuite | P3 | 2-3 周 |
