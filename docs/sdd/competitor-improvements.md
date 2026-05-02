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
