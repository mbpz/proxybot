# ProxyBot 竞品分析报告 (v2.0 — May 2026)

## 1. Executive Summary

本报告分析了移动端 HTTPS/MITM 流量调试工具市场，按 stars 排序进行深度对比。

**ProxyBot 定位**: macOS 原生 TUI/Rust 工具，面向移动端（iOS/Android）HTTPS 流量抓包，支持 app 分类（微信/抖音/支付宝），pf 透明代理 + 内置 DNS 服务器。

**核心差异**: 竞品大多为通用工具，无 app 级别分类能力；ProxyBot 是唯一一个将 DNS 关联 + SNI 检测 + 域名规则库结合做 app 识别的开源项目。

---

## 2. 竞品总览（按 GitHub Stars 排序，2026-05 核实）

| 项目 | Stars | 定位 | Tech Stack | 平台 |
|------|-------|------|-----------|------|
| mitmproxy | 43,451 | 通用 MITM 代理 | Python | 全平台 |
| whistle | 15,505 | HTTP/HTTPS/WS 调试代理 | Node.js | 全平台 |
| bandwhich | 11,726 | 终端带宽监控 | Rust | 全平台 |
| Hetty | 10,127 | 安全研究 HTTP 工具 | Go | 全平台 |
| Mockoon | 8,231 | Mock API | TypeScript | 全平台 |
| anyproxy | 7,920 | 通用 HTTP/HTTPS 代理 | Node.js | 全平台 |
| **Proxyman** | **6,811** | macOS 原生调试代理 | Swift | macOS/iOS/Android |
| spy-debugger | 7,620 | 微信/WebView 移动调试 | JavaScript | 全平台 |
| HTTP Toolkit | 3,494 | 现代 HTTP 调试 UI | TypeScript/Electron | 全平台 |
| lightproxy | 3,186 | 跨平台代理 | TypeScript | macOS/Linux/Win |
| proxy.py | 3,523 | 轻量代理框架 | Python | 全平台 |
| betwixt | 4,562 | Chrome DevTools 风格代理 | JavaScript | 全平台 |
| broxy | 1,011 | Go HTTP/HTTPS 代理 | Go | 全平台 |
| atlantis | 1,500 | iOS 无代理抓包 | Swift | iOS |

---

## 2.1 新增竞品深度分析 (2026)

### 2.1.1 whistle (15,505 ⭐) — Node.js 全能代理

**定位**: 功能最丰富的 Node.js HTTP 代理，插件生态强大。

**优点**:
- 插件生态最丰富（类似 mitmproxy addon）
- 内置 weinre 远程调试
- Composer 请求编辑器
- hosts 文件模拟
- 规则系统强大（基于值匹配）

**缺点**:
- Node.js 性能不如 Go/Rust
- 界面基于 Web UI
- 无 app 分类能力

**创新点**:
- 插件热插拔架构
- 内置远程 WebView 调试（weinre）
- 多协议支持（HTTP/HTTPS/WebSocket/SOCKS）

**ProxyBot 可借鉴**: 插件架构设计、远程调试集成

---

### 2.1.2 bandwhich (11,726 ⭐) — 进程级带宽监控

**定位**: Rust 编写的终端带宽监控工具，进程级流量归属。

**优点**:
- Rust 实现高性能
- TUI 界面，与 ProxyBot 技术栈相近
- 进程级流量监控
- 实时网络连接可视化

**缺点**:
- 非 MITM 代理
- 无流量拦截/修改能力
- 无 app 概念

**创新点**:
- 进程 → IP → 流量关联
- Rust TUI 实践（ratatui/bacon）

**ProxyBot 可借鉴**: TUI 架构模式、进程-流量关联思路

---

### 2.1.3 proxy.py (3,523 ⭐) — 轻量插件框架

**定位**: 轻量级 Python 代理框架，强调零依赖和可插拔性。

**优点**:
- 零外部依赖
- GROUT 隧道（ngrok 替代）
- DNS-over-HTTPS
- 插件系统
- 支持 TLS  interception

**缺点**:
- 功能较基础
- 无 GUI
- 无 app 分类

**创新点**:
- GROUT 隧道反向代理
- DNS-over-HTTPS 内置

**ProxyBot 可借鉴**: TLS interception 实现、GROUT 隧道概念

---

## 3. 功能对比矩阵

### 核心功能对比

| Feature | mitmproxy | whistle | Proxyman | HTTP Toolkit | ProxyBot |
|---------|-----------|---------|----------|--------------|----------|
| MITM HTTPS 拦截 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 移动端抓包 | ✅ | ✅ | ✅ | ✅ | ✅ |
| pf/透明代理 | ❌ | ❌ | ❌ | ❌ | ✅ |
| 内置 DNS 服务器 | ❌ | ❌ | ❌ | ❌ | ✅ |
| **App 分类** | ❌ | ❌ | ❌ | ❌ | ✅ |
| TUI 界面 | ✅ (mitmweb) | ❌ | ❌ (GUI) | ❌ (Electron) | ✅ |
| GUI 界面 | ✅ | ✅ | ✅ | ✅ | ✅ |
| WebView 调试 | ❌ | ✅ | ✅ | ❌ | ❌ |
| iOS 无代理抓包 | ❌ | ❌ | ✅ (Atlantis) | ❌ | ❌ |
| Android 抓包 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 规则引擎 | ✅ (Flow) | ✅ (Plugin) | ✅ | ✅ | ✅ |
| WebSocket 调试 | ✅ | ✅ | ✅ | ✅ | ✅ |
| HAR 导出 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 流量重放 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 自动化脚本 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 请求修改/篡改 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 多级代理链 | ✅ | ✅ | ✅ | ✅ | ❌ |
| Docker 支持 | ✅ | ✅ | ❌ | ✅ | ❌ |
| 插件系统 | ✅ (Addon) | ✅ (Plugin) | ❌ | ❌ | ❌ (v1.0) |
| 进程级监控 | ❌ | ❌ | ❌ | ❌ | ❌ |
| GROUT/隧道 | ❌ | ❌ | ❌ | ❌ | ❌ |
| DNS-over-HTTPS | ❌ | ❌ | ❌ | ❌ | ✅ |
| 流量生成 Mock | ❌ | ✅ | ✅ | ✅ | ✅ |

### 用户体验对比

| UX | mitmproxy | spy-debugger | Proxyman | HTTP Toolkit | ProxyBot |
|----|-----------|--------------|----------|--------------|----------|
| 安装复杂度 | 低 | 中 | 低 (brew) | 低 | 低 |
| 无需配置代理 | ❌ | ✅ | 部分 (Atlantis) | ❌ | ❌ |
| CA 安装流程 | 手动 | 手动 | 一键 | 自动 | 手动 |
| 移动端配置步骤 | 多 | 少 | 少 | 中 | 中 |
| 学习曲线 | 高 | 低 | 低 | 低 | 中 |
| 实时流量速度 | 快 | 快 | 快 | 快 | 快 |
| 搜索/过滤 | 强 | 弱 | 强 | 强 | 中 |
| app 识别可见性 | ❌ | ❌ | ❌ | ❌ | ✅ |

---

## 4. 竞品深度分析

### 4.1 mitmproxy (43,336 ⭐) — 行业标准

**定位**: 面向安全研究人员和开发者的通用 MITM 代理，事实上的开源标准。

**优点**:
- 功能最全面，脚本 API 最强大
- mitmweb 提供 GUI，mitmdump 支持无界面批量处理
- 生态最成熟，文档最完善
- 支持 HTTP/2、WebSocket、IPv6
- 社区最大，插件丰富

**缺点**:
- 无 app 分类能力
- TUI 界面原始（类似 vim 操作）
- 移动端配置需手动设置代理
- 规则系统基于 URL pattern，不支持复杂逻辑
- pf 透明代理需要额外工具配合

**创新点**:
- Flow 表达式语言（类似 SQL 的流量查询）
- Addon 系统（可拦截、修改、重放任何流量）
- 配置迁移（mitmproxy 配置文件可复用）

**ProxyBot 可借鉴**: 流量过滤表达式、addon 脚本能力

---

### 4.2 spy-debugger (7,620 ⭐) — 微信调试起家

**定位**: 专注移动端 WebView 调试，微信/H5 页面调试神器。

**优点**:
- 微信真机调试一键完成
- WebView style 调试（iOS Safari Remote Debugging 风格）
- 零配置代理，手机无需设置代理
- Node.js 实现，部署简单

**缺点**:
- 停止维护（2021 年后基本无更新）
- 无独立 GUI，靠命令行 + Chrome DevTools
- 不支持 HTTPS 明文查看（仅日志）
- 无规则引擎
- 无 app 分类
- 无流量重放/修改

**创新点**:
- 微信专用调试协议支持
- 无需设置系统代理的weinre 方案

**ProxyBot 可借鉴**: 移动端零配置体验、微信生态集成

---

### 4.3 Proxyman (6,799 ⭐) — macOS 原生最强

**定位**:  macOS 原生应用，最接近 Charles Proxy 的开源替代。

**优点**:
- 原生 macOS GUI，用户体验最佳
- iOS/Android 全平台支持
- **Atlantis**: iOS 无需设置代理即可抓包（VPN API）
- HTTPS 一键解密
- 规则系统强大（Map Remote/Local/Breakpoint）
- Charles 兼容格式导入

**缺点**:
- 主要面向 macOS/Linux
- 无 TUI，CLI 能力弱
- 无 DNS 服务器
- 无 app 分类
- 无 pf 集成（透明代理需另外配置）

**创新点**:
- VPN API 实现 iOS 无代理抓包（Atlantis）
- 原生 Swift 实现，性能好
- Certificate Provider Extensions（iOS 自动安装 CA）

**ProxyBot 可借鉴**: Atlantis iOS 无代理抓包技术、原生 GUI 实现

---

### 4.4 HTTP Toolkit (3,488 ⭐) — 现代 UI

**定位**: 面向开发者的现代 HTTP 调试工具，Electron 实现。

**优点**:
- UI 最现代（对齐开发工具审美）
- Android adb 一键配置
- 自动 HTTPS 拦截（无需手动 CA）
- Request matching & mocking
- 开源且活跃

**缺点**:
- Electron 性能一般
- 无 TUI
- 无 DNS 服务器
- 无 app 分类
- 无 pf 集成

**创新点**:
- Mock server 内置
- Automatic HTTPS interception（智能 CA 配置）
- Rule-based request matching

**ProxyBot 可借鉴**: 自动 CA 配置流程、Android adb 集成

---

### 4.5 Hetty (10,127 ⭐) — 安全研究

**定位**: 面向安全研究团队的 HTTP 工具，mitmproxy 替代。

**优点**:
- Go 实现，性能好
- 项目制管理（多个项目隔离）
- MITM 能力完整
- CLI + Web UI

**缺点**:
- 无移动端特殊支持
- 无 DNS 服务器
- 无 app 分类
- 2023 年才发布，生态不成熟

**创新点**:
- 项目制管理（多测试任务隔离）
- Go 全栈实现

---

### 4.6 anyproxy (7,920 ⭐) — 阿里系

**定位**: 阿里巴巴出品，全功能 Node.js 代理。

**优点**:
- 规则系统灵活（JavaScript）
- Web UI 可视化
- 支持 HTTPS
- 平台全覆盖

**缺点**:
- Node.js 性能不如 Go/Rust
- 停止维护（2021 年）
- 无移动端特殊支持
- 无 app 分类

**创新点**:
- 规则热重载
- Web 管理界面

---

## 5. ProxyBot 竞争优势

### 5.1 差异化核心: App 分类

**ProxyBot 是唯一一个**实现 app 级别流量分类的开源 MITM 工具。

实现原理:
1. DNS 查询日志 → 记录哪个 app 发了 DNS 请求
2. SNI 检测 → TLS ClientHello 中提取域名
3. 域名规则库 → 微信/抖音/支付宝等 app 的域名映射
4. 关联分析 → 同一时间线的 DNS + 连接 → app 标签

竞品对比:
- mitmproxy: 仅能按 URL/Host 过滤，无法关联 app
- Proxyman: 支持 Source App 过滤（macOS 限），但非自动分类
- HTTP Toolkit: 无 app 概念
- whistle: 通过插件支持，但非开箱即用

### 5.2 透明代理能力

mitmproxy、HTTP Toolkit 均需客户端手动设置 HTTP(S) 代理。
ProxyBot 通过 pf 透明代理实现:
- 手机无需任何代理配置
- 手机网关/DNS 指向 PC → 流量自动被抓
- 80/443 端口透明劫持

### 5.3 TUI 优先 + Rust 性能

唯一采用 Rust + TUI 界面的开源移动抓包工具:
- 键盘驱动，效率高
- 无需图形环境（远程服务器可用）
- Rust 实现，性能好
- 60fps 渲染，流畅

对比 bandwhich（Rust TUI）: ProxyBot 是唯一一个既保留 TUI 又提供 GUI 的竞品。

### 5.4 DNS-over-HTTPS 支持

proxy.py 展示了 DNS-over-HTTPS 在代理中的价值。
ProxyBot 已内置 DoH 支持，结合透明代理形成完整流量控制。

### 5.5 Mock 生成能力

ProxyBot Gen tab 已实现自动化 mock 生成，领先于大多数竞品。
whistle 和 HTTP Toolkit 有 mock 能力但依赖手动配置。

---

## 6. ProxyBot 竞争劣势

| 劣势 | 说明 | 缓解方案 |
|------|------|---------|
| 无 GUI | 非技术人员不友好 | ✅ Tauri React GUI (v0.6+) |
| 规则系统弱 | 无 map remote/local/breakpoint | ✅ 参考 Proxyman 规则格式 (v0.7+) |
| 无 WebView 调试 | whistle/proxyman 特有功能 | 可考虑集成 |
| iOS 无代理抓包 | Proxyman Atlantis 独有 | 研究 VPN API |
| 流量修改 | 仅监控 | ✅ Breakpoint (v0.5+) |
| 无自动 CA | 需手动安装 | ✅ 简化安装流程 (v0.6+) |
| 文档少 | vs mitmproxy | 完善 README + 示例 |
| 无插件系统 | mitmproxy/whistle 优势 | v1.0 计划 |
| 无多级代理链 | mitmproxy 优势 | 后续版本考虑 |
| 无进程级监控 | bandwhich 特色 | 暂无计划 |
| 无 GROUT 隧道 | proxy.py 特色 | 暂无计划 |

---

## 7. 新兴竞争格局 (2025-2026)

### AI + 代理结合趋势

随着 LLM API 调用增加，部分新兴工具开始专注 AI 流量调试:
- **LLM API 网关** (如 litellm 46k stars): 统一管理 AI API 调用
- AI 流量分类、token 成本分析成为新需求

### ProxyBot 机会: AI 流量分类

ProxyBot 的 app 分类技术可扩展到 AI 服务识别:
- OpenAI API 调用特征（api.openai.com）
- Anthropic API 调用特征
- Azure OpenAI 特征
- Token 使用量估算

---

## 8. 执行计划 (更新版)

### Phase 1: 已完成 ✅
- [x] 规则系统升级（MapRemote/MapLocal/Breakpoint）
- [x] Tauri GUI Alpha/Beta
- [x] 自动 CA 配置引导
- [x] Android adb reverse

### Phase 2: 已完成 ✅
- [x] **v1.0: 插件系统** — RuleEngine 规则路由 + 优先级 + 热重载 + async hooks
- [x] **v1.0: Network conditions** — 延迟/带宽/丢包模拟 (2G/3G/4G/WiFi/Edge 预设)
- [x] **v1.0: Team collaboration** — Workspace 导出/导入 tar.gz 共享配置
- [x] **v1.0: Rhai 脚本引擎** — on_request/on_response hooks + 沙箱脚本
- [x] **v1.0: gRPC/Protobuf 解码** — gRPC frame + protobuf wire format parser
- [x] **v1.0: iOS VPN** — TCP bridge server + PacketTunnelProvider + .mobileconfig

### Phase 3: 差异化创新
- [ ] AI 流量分类（OpenAI/Anthropic/Azure）
- [ ] 支持 Windows（pfctl → Windows Filtering Platform）
- [ ] 云端协作（多设备流量汇总）

---

## 9. 关键结论 (更新版)

1. **mitmproxy** 是开源 MITM 代理的事实标准，ProxyBot 的 app 分类 + 透明代理是独特差异点
2. **Proxyman** 是最接近商业品质的竞品，Atlantis iOS 无代理抓包是技术亮点
3. **whistle** 的插件生态和远程调试能力值得学习
4. **HTTP Toolkit** 的 UI 设计和自动 CA 配置值得借鉴
5. **bandwhich** 证明 Rust TUI 可行，ProxyBot 在此基础上叠加了 MITM 能力
6. ProxyBot 核心竞争优势是 **app 分类 + pf 透明代理 + TUI + DNS DoH**，这四点组合是其他竞品没有的
7. ProxyBot 最大差距已缩小（GUI、规则系统、断点），v1.0 应聚焦插件系统和 AI 流量分类

---

## 10. 竞品深度研究 (2026)

### 10.1 whistle 插件架构深度分析

**架构核心**: whistle 采用"一切皆插件"的设计，核心功能（HTTP解析、代理）也是插件。

**Hook 体系**:
```
onRequest → onResponse → onConnect → onServer → onSocket → onError
```

**插件契约**:
```javascript
module.exports = {
  onRequest: async (req, res, session) => { /* */ },
  onResponse: async (req, res, session) => { /* */ },
  onConnect: async (req, socket, head, session) => { /* */ }
};
```

**规则路由**: `pattern pluginName` 声明式语法，类似 Express middleware 但更强大。

**ProxyBot 可借鉴**:
1. **Hook 优先级链**: onRequest → proxy → response 链式模型
2. **规则路由**: 插件注册 URL pattern 而非显式启用/禁用
3. **热重载**: 规则文件变更自动重载

---

### 10.2 HTTP Toolkit UI 设计深度分析

**视觉风格**:
- 深色主题 + JetBrains Mono 等宽字体
- HTTP 方法颜色编码（GET=绿, POST=蓝, DELETE=红）
- Status code 色带（2xx=绿, 4xx=橙, 5xx=红）

**布局结构**: 三栏式
- 左侧: 请求列表（虚拟滚动）
- 中间: 请求概览（方法、路径、状态、时间线）
- 右侧: 详情面板（Headers/Body/Timing/Location tabbed）

**CA 配置引导**:
1. 平台选择（图标式）
2. 可视化安装说明（含截图）
3. 验证请求确认
4. 成功/失败反馈

**ADB 集成**:
- `adb reverse tcp:8888 tcp:8888` USB 直连
- QR code 生成 WiFi 代理配置
- 环境变量自动注入 Node.js/Python

**ProxyBot 可借鉴**:
1. **三栏布局** + 颜色编码方法/状态
2. **CA 安装向导**: 平台检测 → 可视化引导 → 验证 → 确认
3. **ADB 一键配置**: 自动检测设备 + QR 代理配置

---

### 10.3 Proxyman Atlantis VPN 深度分析

**架构图**:
```
iOS Device                    Mac ProxyBot
+--------+                  +------------------+
| NEPacket|===[TLS]=======>|  ProxyBot MITM   |
| Tunnel  |   Tunnel        |  (Rust + rustls) |
|Provider |                 +------------------+
+--------+
```

**关键实现**:
1. `NEPacketTunnelProvider` 子类作为 iOS Network Extension
2. `packetFlow.readPackets()` 读取 IP 包
3. TLS 隧道转发到 Mac
4. Mac 端终止 TLS，执行 MITM

**简化方案**（ProxyBot 可用）:
- 不做设备端 MITM，直接转发原始包到 Mac
- 用 TCP socket 而非 TLS（简化初期）
- 复用现有 `proxy.rs` 基础设施

**ProxyBot 已有基础**:
- `ios/PacketTunnel/PacketTunnelProvider.swift` 已有骨架代码
- 缺少：网络转发逻辑、TLS 隧道

**实施阶段**:
1. TCP bridge（2-3 周 MVP）
2. 包封装和重组（production 需 6-8 周）

---

### 10.4 新增竞品: proxfy

**定位**: CLI HTTPS proxy for mobile debugging，轻量级 Charles 替代品。

**特点**:
- 轻量级 CLI 工具
- 面向移动端调试
- 简约设计

**ProxyBot 启示**: 移动端调试是核心场景，ProxyBot 的 app 分类是差异化优势。

---

## 11. 第三轮竞品搜集 (May 2026)

深入挖掘 2025-2026 年新兴项目，聚焦创新架构和差异化功能。

### 11.1 新兴竞品速览

| 项目 | Stars | 语言 | 定位 | 创新点 |
|------|-------|------|------|--------|
| [TokenTap](https://github.com/jmuncor/tokentap) | 797 | Python | LLM API 流量拦截 | Token 感知、上下文窗口追踪、零配置 |
| [Rockxy](https://github.com/RockxyApp/Rockxy) | 404 | Swift | macOS 原生代理 | SwiftUI+SwiftNIO、GraphQL 内省 |
| [KtorMonitor](https://github.com/CosminMihuMDC/KtorMonitor) | 217 | Kotlin | SDK 层拦截器 | Compose Multiplatform、非代理模式 |
| [httpmon](https://github.com/kostyay/httpmon) | 80 | Go | 终端 HTTP 代理 | Bubble Tea TUI、.proto 加载、JS 脚本 |
| [int3rceptor](https://github.com/S1b-Team/int3rceptor) | 4 | Rust+Vue | 渗透测试代理 | Rust 核心 + Vue UI、Fuzzing |
| [intercept](https://github.com/mrceha/intercept) | — | Go | 轻量代理 | 单二进制、Web Dashboard |
| [mitmproxy-rs](https://github.com/josexy/mitmproxy-rs) | — | Rust | MITM 库 | 可嵌入、库优先设计 |
| [go-traffic-proxy-analyzer](https://github.com/tahsinmert/go-traffic-proxy-analyzer) | — | Go | 流量分析 | 内置 Metrics+Alerting |

### 11.2 关键发现

**1. LLM/AI 流量调试是新战场**
- TokenTap 4 个月冲到 797 stars，证明 AI API 流量分析是刚需
- ProxyBot 已有 AI 签名分类（OpenAI/Anthropic/Azure），可增强 token 计数和上下文窗口追踪
- 机遇：做第一个同时支持 LLM 流量分类 + token 成本估算 + 提示词审计的开源代理

**2. GraphQL 支持已成标配**
- Rockxy 内置 GraphQL 自省解码，mock-smith 支持浏览器端 GraphQL 拦截
- 尚无人结构化解码 GraphQL Subscription (WebSocket) 流量
- ProxyBot 已有 WebSocket frame viewer，扩展到 GraphQL-WS 协议解析成本低

**3. HTTP/3 & QUIC 是最大空白**
- 11 个新兴项目中零个支持 HTTP/3
- QUIC 基于 UDP，传统 HTTP 代理模型不适用，需要全新方案
- 这是 ProxyBot 可以建立技术护城河的领域

**4. Web Dashboard 模式兴起**
- intercept、int3rceptor、httpmon 都提供 Web UI 作为 TUI/CUI 的补充
- ProxyBot 已有 Tauri GUI + TUI，增加轻量 Web dashboard（类似 mitmweb）可覆盖远程访问场景

**5. 库优先 + 可嵌入趋势**
- mitmproxy-rs 将 MITM 能力封装为库，供其他 Rust 项目集成
- KtorMonitor 走 SDK 拦截器路线，在应用层而非网络层工作
- ProxyBot 可参考：将核心 proxy engine 拆为独立 crate，允许被其他项目引用

**6. Prometheus Metrics + Alerting**
- go-traffic-proxy-analyzer 内置实时指标和告警
- ProxyBot 已有 Alerts 面板，可增加 Prometheus `/metrics` 端点供外部监控集成

### 11.3 竞品对比矩阵 v3 (新增维度)

| 维度 | ProxyBot | httpmon | Rockxy | TokenTap | intercept | mitmproxy-rs |
|------|----------|---------|--------|----------|-----------|--------------|
| TUI | ✅ ratatui | ✅ Bubble Tea | — | ✅ Textual | — | — |
| GUI | ✅ Tauri | — | ✅ SwiftUI | — | ✅ Web | — |
| Web Dashboard | — | — | — | — | ✅ | — |
| GraphQL 解码 | — | — | ✅ | — | — | — |
| LLM Token 追踪 | 部分(签名) | — | — | ✅ | — | — |
| gRPC/Protobuf | ✅ v1.0 | ✅ .proto | — | — | — | — |
| HTTP/3 QUIC | — | — | — | — | — | — |
| Metrics 导出 | — | — | — | — | — | — |
| 单二进制部署 | ✅ | ✅ | — | — | ✅ | ✅ |
| 库/嵌入式 | — | — | — | — | — | ✅ |
| 脚本引擎 | ✅ Rhai | ✅ JS | — | — | — | — |
| App 分类 | ✅ | — | — | — | — | — |
| pf 透明代理 | ✅ | — | — | — | — | — |

### 11.4 ProxyBot 增强机会

基于此轮调研，识别出 5 个高价值增强方向：

| # | 方向 | 竞品参考 | 优先级 | 预计工期 |
|---|------|---------|--------|----------|
| 1 | **GraphQL 解码器** | Rockxy 内置 GraphQL | P1 | 3-4 天 |
| 2 | **Prometheus Metrics** | go-traffic-proxy-analyzer | P1 | 1-2 天 |
| 3 | **LLM Token 追踪增强** | TokenTap 上下文窗口 | P2 | 3-4 天 |
| 4 | **Web Dashboard (mitmweb 风格)** | intercept 单二进制 | P2 | 1-2 周 |
| 5 | **HTTP/3 & QUIC 研究** | (无竞品，新领域) | P3 | 研究性 2+ 周 |

**ProxyBot 的护城河依然牢固**: App 分类 + pf 透明代理 + TUI+GUI 双界面 + Rust 核心的组合在 2026 年竞品中仍然独特。新增的 HTTP/3 空白和 LLM 流量追踪是两个关键战略机会。

---

## 12. 第四轮竞品搜集 (May 2026)

深入挖掘 6 个前 3 轮未覆盖的高价值项目。

### 12.1 新增竞品速览

| 项目 | Stars | 语言 | 定位 | 创新点 |
|------|-------|------|------|--------|
| [proxelar](https://github.com/emanuele-em/proxelar) | 966 | Rust | 可编程 MITM 代理 | Rust+ratatui+Lua、三界面(TUI/CLI/Web)、列作用域过滤 |
| [anything-analyzer](https://github.com/Mouseww/anything-analyzer) | 2,366 | TypeScript | 全场景协议分析 | 浏览器CDP+MITM双通道、AI逆向分析、MCP Server集成 |
| [hyperfox](https://github.com/malfunkt/hyperfox) | 1,631 | Go | HTTP/HTTPS 录制代理 | SQLite 录制、QR code CA 分发、Web UI + REST API |
| [InterceptSuite](https://github.com/InterceptSuite/InterceptSuite) | 772 | C# | 传输层 MITM | TCP/UDP/DTLS/TLS、IoT/Thick Client、Python 扩展 API |
| [forwarder](https://github.com/saucelabs/forwarder) | 280 | Go | 生产级 MITM | PAC 支持、HTTP/2/WebSocket/SSE/TCP、Sauce Labs 生产环境使用 |
| [gomitmproxy](https://github.com/AdguardTeam/gomitmproxy) | 344 | Go | 库优先 MITM | AdGuard 出品、可嵌入、自定义证书存储、onRequest/onResponse |

---

### 12.2 proxelar (966 ⭐) — Rust 可编程 MITM 代理

**定位**: 最接近 ProxyBot 架构的竞品。Rust 核心 + ratatui TUI + Lua 脚本。

**技术架构**:
```
Your App → Proxelar :8080 → Internet
                │
          Inspect · Modify · Mock (Lua)
```
- 核心: Rust + hyper + rustls + tokio
- TUI: ratatui (与 ProxyBot 相同框架)
- Web GUI: axum + WebSocket 实时推送
- 脚本: Lua (mlua crate)，非 Rhai
- 安装: Homebrew / Cargo / Docker

**核心功能**:
- **Lua 脚本**: on_request / on_response hooks，可修改/阻断/mock
- **三界面**: terminal (stdout)、TUI (ratatui)、Web GUI (axum)
- **列作用域过滤**: `time:14:`, `proto:https`, `method:POST`, `host:github`, `path:/api`, `status:404`, `type:json`, `size:1KB`, `duration:slow` — 比正则更直观
- **正向/反向代理**: CONNECT 隧道或上游 URI 重写
- **WebSocket 检查**: 按方向/opcode/payload 浏览帧
- **CA 自动安装**: 访问 `http://proxel.ar` 下载并安装根证书
- **请求重放**: TUI 内 `r` 键重放选中请求

**TUI 快捷键**:
| 键 | 动作 |
|----|------|
| `j/k/↑/↓` | 导航列表 |
| `Enter` | 打开详情面板 |
| `Tab` | 切换 Request/Response/Frames |
| `/` | 过滤 (plain text 或 column:value) |
| `r` | 重放请求 |
| `g/G` | 跳转顶部/底部 |
| `c` | 清空请求列表 |

**与 ProxyBot 对比**:

| 维度 | proxelar | ProxyBot |
|------|---------|----------|
| 语言 | Rust | Rust |
| TUI 框架 | ratatui | ratatui |
| GUI | axum Web GUI | Tauri v2 + React |
| 脚本 | Lua (mlua) | Rhai |
| 过滤 | 列作用域 DSL | 正则搜索 |
| 透明代理 | ❌ | ✅ pf |
| DNS 服务器 | ❌ | ✅ 内置 DoH/UDP |
| App 分类 | ❌ | ✅ DNS+SNI+规则 |
| 设备管理 | ❌ | ✅ 多设备注册 |
| 规则引擎 | 仅 Lua 脚本 | ✅ YAML 规则 + 5 动作 |
| Mock 生成 | Lua return | ✅ Gen tab (API/前端/Docker) |
| DAG 分析 | ❌ | ✅ 流量 DAG + Auth 状态机 |
| HAR 导出 | ❌ | ✅ |
| 安装方式 | brew/cargo/docker | brew/cargo/build |
| Stars | 966 | — |

**ProxyBot 可借鉴**:
1. **列作用域过滤 DSL** — `method:POST host:api` 比纯正则更直观，可集成到 Filter DSL
2. **三界面策略** — proxelar 的 terminal/TUI/GUI 三选一设计验证了多界面路线的可行性
3. **CA auto-install wizard** — `http://proxel.ar` 方案比手动导出更友好
4. **WebSocket 帧浏览** — 按方向/opcode 分类比 ProxyBot 当前的 text/hex 视图更结构化

**威胁评估**: 中。proxelar 是最接近 ProxyBot 的开源竞品，但缺少透明代理、DNS 服务器和 App 分类这三个 ProxyBot 的核心差异化能力。proxelar 的 Lua 脚本在生态上比 Rhai 更成熟（更多现成脚本可复用）。

---

### 12.3 anything-analyzer (2,366 ⭐) — AI 驱动的全场景协议分析

**定位**: 不只是抓包工具，是 AI 自动逆向分析平台。4 个月冲到 2.3k stars。

**技术架构**:
```
网页(Chrome CDP) + 桌面应用(MITM) + 终端(curl) + 脚本(Python/Node) + 手机(Wi-Fi代理)
                              ↓
                    统一会话 Session (SQLite)
                              ↓
                AI 两阶段分析 (过滤 → 深度分析)
                              ↓
              MCP Server → AI Agent/IDE 直接调用
```
- 框架: Electron 35 + React 19 + TypeScript
- 数据库: better-sqlite3
- 浏览器: Chrome DevTools Protocol (CDP) Fetch domain
- 代理: 内置 MITM HTTPS (node-forge TLS)
- AI: OpenAI / Anthropic / 自定义 LLM (Chat Completions + Responses API)
- MCP: Client (stdio + StreamableHTTP) + 内置 MCP Server

**核心差异化能力**:

1. **双通道统一捕获** — CDP (浏览器) + MITM (外部) → 同一 Session
2. **AI 两阶段分析** — Phase 1 智能过滤噪声 → Phase 2 深度分析
3. **5 种分析模式** — 自动识别 / API 逆向 / 安全审计 / 性能分析 / JS 加密逆向
4. **JS Hook 注入** — 自动拦截 fetch、XHR、crypto.subtle、CryptoJS、SM2/3/4
5. **加密代码提取** — 从 JS 文件中自动提取加密相关代码片段
6. **MCP Server** — 将抓包能力暴露为 MCP 工具，Claude Desktop/Cursor 可直接调用
7. **流式 AI 输出 + 多轮追问** — 报告实时显示，可追问细节
8. **内嵌浏览器** — 多标签页，OAuth 弹窗自动捕获

**5 维分析**:

| 维度 | 评分 | 说明 |
|------|------|------|
| 技术架构 | ★★★★ | 双通道(CDP+MITM)设计巧妙，但 Electron 性能开销大 |
| 设计理念 | ★★★★★ | "抓包→AI分析"闭环创新，从工具到平台的跃迁 |
| 实现方案 | ★★★★ | CDP+MITM 混合架构独特，JS Hook 注入技术领先 |
| 交互设计 | ★★★★ | Ant Design 现代 UI，但 Electron 不够原生 |
| 产品定位 | ★★★★★ | 唯一将 AI 分析作为一等公民的抓包工具 |

**与 ProxyBot 对比**:

| 维度 | anything-analyzer | ProxyBot |
|------|-------------------|----------|
| 核心技术 | Electron + TypeScript | Rust + Tauri |
| 性能 | 中 (Electron) | 高 (Rust 原生) |
| 抓包方式 | CDP + MITM 双通道 | pf 透明代理 |
| AI 能力 | ★★★★★ 自动逆向分析 | ★★ Gen tab (LLM 推断) |
| MCP 集成 | ✅ Client + Server | ❌ |
| JS Hook 注入 | ✅ crypto.subtle/CryptoJS | ❌ |
| 移动端 | Wi-Fi 代理 | pf 透明代理 (零配置) |
| App 分类 | ❌ | ✅ |
| 平台 | Win/Mac/Linux | macOS (Win 计划中) |
| 开源 | MIT | MIT |

**ProxyBot 可借鉴**:
1. **MCP Server 集成** (P1) — 将 ProxyBot 的抓包/分类/规则能力暴露为 MCP 工具，让 Claude/Cursor 直接操作
2. **AI 两阶段分析** (P1) — Phase 1 过滤噪声请求 → Phase 2 深度分析，提升 Gen tab 的 AI 推断质量
3. **流式 AI 输出** (P2) — Gen tab 的 LLM 推断改为流式显示，改善长等待体验
4. **JS Hook 注入思路** (P3) — 可用于客户端加密逆向场景

**威胁评估**: 低-中。anything-analyzer 的 AI 分析是其核心差异，但 Electron 性能和 pf 透明代理的缺失使其无法直接竞争移动端场景。然而其 MCP Server 策略值得警惕——如果抓包工具变成 AI Agent 的基础设施，先发优势会迅速放大。

---

### 12.4 hyperfox (1,631 ⭐) — HTTP/HTTPS 录制代理

**定位**: 轻量级 Go 代理，专注 HTTP/HTTPS 流量录制和回放。

**技术架构**:
```
Client → Hyperfox (:1080 HTTP / :10443 HTTPS) → Internet
                   ↓
              SQLite DB (per-session)
                   ↓
         Web UI (:1984) + REST API (:4891)
```
- 核心: Go 单二进制
- 存储: SQLite (每会话一个 DB 文件)
- UI: Web UI (Go 内嵌) + REST API
- CA: 内置 root CA 生成

**核心功能**:
- 透明 HTTP 代理 (端口 1080) + HTTPS MITM (端口 10443)
- 每会话自动创建 SQLite 数据库
- Web UI 带 QR code（方便手机访问）
- REST API（随机 key 认证）
- DNS 解析器 override（绕过系统 DNS）
- ARP spoofing 支持（LAN 内 MITM 攻击场景）

**创新点**:
- **QR code CA 分发** — 生成 QR code 让手机扫码安装 CA，降低移动端配置门槛
- **移动端优先 UI** — Web UI 适配手机屏幕，`-ui-addr` 绑定 LAN IP 后输出 QR code
- **per-session 数据库** — 每次启动创建独立 DB，便于按测试任务隔离

**ProxyBot 可借鉴**:
1. **QR code CA 分发** (P2) — 在 GUI 的 CA 安装向导中生成 QR code，手机扫码即可下载安装
2. **per-session 数据库** (P3) — 类似 InterceptSuite 的项目文件管理，支持多测试任务隔离
3. **移动端 Web Dashboard** (P2) — 轻量 Web 界面供手机查看实时流量

**威胁评估**: 低。hyperfox 开发活跃度低（2020 年后更新缓慢），功能集远小于 ProxyBot。但 QR code 和移动端 Web UI 思路值得借鉴。

---

### 12.5 InterceptSuite (772 ⭐) — 传输层 MITM

**定位**: 非 HTTP 协议的 MITM 代理，IoT/Thick Client/数据库 TLS 专用。

**技术架构**:
```
IoT Device / Thick Client / DB → InterceptSuite (TCP/UDP/DTLS/TLS) → Server
                                          ↓
                              Python Extension API (协议解析)
```
- 核心: C (性能关键路径) + C# (Avalonia .NET GUI)
- TLS: OpenSSL
- GUI: Avalonia .NET (跨平台原生)
- 扩展: Python Extension API

**核心差异化**:
- **传输层拦截**: TCP/UDP/DTLS/TLS，不仅是 HTTP
- **STARTTLS 自动检测** (Pro): SMTP/IMAP/PostgreSQL/MySQL 的 TLS 升级自动识别
- **DTLS 支持** (Pro): IoT 和实时通信协议的 UDP+TLS
- **Python 扩展 API**: 自定义协议解析器
- **PCAP 导出** (Pro): 兼容 Wireshark 分析
- **项目文件管理** (Pro): 保存和组织捕获会话

**定价**: 免费版 (社区) + Professional ($300 一次性)

**与 ProxyBot 对比**:

| 维度 | InterceptSuite | ProxyBot |
|------|---------------|----------|
| 协议层 | TCP/UDP/DTLS/TLS | HTTP/HTTPS/WSS |
| 目标场景 | IoT/Thick Client/DB | 移动 App HTTP 流量 |
| 核心语言 | C + C# | Rust |
| GUI | Avalonia .NET 原生 | Tauri + React |
| TUI | ❌ | ✅ ratatui |
| HTTP 专用功能 | 有限 | ✅ 完整 |
| 扩展性 | Python | Rhai 脚本 |
| 开源策略 | AGPL + Pro 付费 | MIT |
| App 分类 | ❌ | ✅ |
| 透明代理 | ❌ | ✅ pf |

**ProxyBot 可借鉴**:
1. **项目文件管理** (P2) — 类似 InterceptSuite 的项目制管理，支持保存/恢复捕获会话
2. **Python 扩展 API** (P3) — 除 Rhai 外增加 Python 脚本支持（通过 PyO3）
3. **PCAP 导出** (P3) — 导出流量为 PCAP 格式供 Wireshark 分析
4. **STARTTLS 升级检测** (P3) — 检测非 443 端口的 TLS 升级（SMTP/IMAP 等）

**威胁评估**: 低。InterceptSuite 聚焦传输层，与 ProxyBot 的 HTTP/移动端定位互补而非竞争。但其 Python 扩展 API 和项目文件管理是值得借鉴的产品设计。

---

### 12.6 forwarder (280 ⭐) + gomitmproxy (344 ⭐) — Go 库生态

**forwarder** (Sauce Labs):
- 生产级 MITM 代理，用于 Sauce Connect Proxy
- 支持 HTTP/HTTPS/HTTP2/WebSocket/SSE/TCP
- **PAC (Proxy Auto-Config)** 支持 — 按 URL pattern 路由
- Go 库 + CLI 二进制

**gomitmproxy** (AdGuard):
- AdGuard Home 的 MITM 核心提取为独立库
- **库优先设计** — 供 Go 项目嵌入，非独立应用
- onRequest/onResponse handlers
- 自定义证书存储接口
- 代理鉴权 (Basic Auth)

**两个项目共同揭示的趋势**:
- **库优先**: MITM 能力封装为库，供其他项目集成（mitmproxy-rs 同样思路）
- **PAC 支持**: 按 URL pattern 的智能路由

**ProxyBot 可借鉴**:
1. **核心引擎拆分为独立 crate** (P2) — `proxybot-core` 作为独立 Rust crate，供其他 Rust 项目引用
2. **PAC 风格路由** (P3) — 在规则引擎中增加 PAC-like URL pattern 匹配
3. **代理鉴权** (P3) — 代理层增加 Basic Auth 保护

**威胁评估**: 极低。这两个是库而非独立工具，与 ProxyBot 的应用定位不冲突。

---

### 12.7 第四轮竞品对比矩阵

| 维度 | ProxyBot | proxelar | anything-analyzer | hyperfox | InterceptSuite | forwarder | gomitmproxy |
|------|----------|----------|-------------------|----------|---------------|-----------|-------------|
| 核心语言 | Rust | Rust | TypeScript | Go | C# + C | Go | Go |
| TUI | ✅ ratatui | ✅ ratatui | — | — | — | — | — |
| GUI | ✅ Tauri | ✅ Web | ✅ Electron | ✅ Web | ✅ Avalonia | — | — |
| MITM TLS | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 透明代理 | ✅ pf | ❌ | ❌ | /etc/hosts | ❌ | ❌ | ❌ |
| DNS 服务器 | ✅ | ❌ | ❌ | ✅(override) | ❌ | ❌ | ❌ |
| App 分类 | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 脚本引擎 | ✅ Rhai | ✅ Lua | ✅ JS Hook | ❌ | ✅ Python | ❌ | ✅ Go |
| 规则引擎 | ✅ YAML 5动作 | Lua 脚本 | ❌ | ❌ | ❌ | ✅ PAC | ❌ |
| AI 分析 | ✅ Gen tab | ❌ | ✅★★★★★ | ❌ | ❌ | ❌ | ❌ |
| MCP Server | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| CDP 浏览器 | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| 请求编辑 | ✅ Breakpoint | ✅ Lua | ❌(只读) | ❌ | ✅ | ✅ | ✅ |
| WebSocket | ✅ frames | ✅ frames | ✅(隧道) | ❌ | ❌ | ✅ | ❌ |
| HTTP/2 | ✅ | ❌ | ❌ | ✅ | ❌ | ✅ | ❌ |
| Mock 生成 | ✅ Gen tab | ✅ Lua | ❌ | ❌ | ❌ | ❌ | ❌ |
| DAG 分析 | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 跨平台 | macOS | All | All | All | All | All | All |
| Stars | — | 966 | 2,366 | 1,631 | 772 | 280 | 344 |

---

### 12.8 第四轮关键洞察

**1. proxelar 是最接近的直接竞品**
- Rust + ratatui + 脚本 + MITM 的技术栈几乎完全重叠
- proxelar 的优势: Lua 生态、列作用域过滤 DSL、三界面设计、更简洁的 CA 安装
- ProxyBot 的优势: 透明代理、DNS 服务器、App 分类、规则引擎、Gen tab、DAG 分析
- **建议**: 密切关注 proxelar 发展，重点强化 ProxyBot 的差异化能力

**2. AI+抓包融合是确定趋势**
- anything-analyzer 用 4 个月证明 "AI 自动分析 + 抓包" 是强烈的市场需求
- ProxyBot 的 Gen tab 已有 LLM 推断基础，升级到两阶段分析 + MCP Server 成本可控
- **建议**: P1 优先级实现 MCP Server + AI 两阶段分析管道

**3. MCP Server 是新的分发渠道**
- anything-analyzer 将抓包能力作为 MCP 工具暴露给 AI Agent
- 这创造了新的使用场景: AI Agent 自动抓包分析 → 生成复现代码
- **建议**: P1 优先级实现 ProxyBot MCP Server

**4. 库优先 + 可嵌入是架构趋势**
- gomitmproxy、mitmproxy-rs、forwarder 都选择库优先
- 降低集成门槛，扩大生态影响力
- **建议**: P2 提取 proxybot-core 为独立 crate

**5. 传输层代理是蓝海**
- InterceptSuite 在 TCP/UDP/DTLS 领域几乎没有竞品
- ProxyBot 目前仅覆盖 HTTP/HTTPS/WSS
- **建议**: P3 研究 TCP/TLS 通用代理能力

**6. 移动端 CA 安装体验是关键瓶颈**
- hyperfox 的 QR code + proxelar 的 `http://proxel.ar` 都简化了 CA 安装
- ProxyBot 目前仍需手动 AirDrop/邮件
- **建议**: P2 实现 QR code + 内置 HTTP 下载 CA

---

### 12.9 增强优先级更新 (第四轮后)

| # | 方向 | 竞品参考 | 优先级 | 预计工期 | 状态 |
|---|------|---------|--------|----------|------|
| 1 | **MCP Server** | anything-analyzer 内置 MCP | **P0** | 3-5 天 | 新增 |
| 2 | **AI 两阶段分析管道** | anything-analyzer Phase 1/2 | **P1** | 1 周 | 新增 |
| 3 | **列作用域过滤 DSL** | proxelar column:value | **P1** | 2-3 天 | 新增 |
| 4 | **QR code CA 分发** | hyperfox QR code | **P2** | 1 天 | 新增 |
| 5 | **proxybot-core crate** | gomitmproxy/mitmproxy-rs | **P2** | 1 周 | 新增 |
| 6 | **项目文件管理** | InterceptSuite 项目制 | **P2** | 3-4 天 | 新增 |
| 7 | **HTTP/3 QUIC 研究** | (空白领域) | P3 | 2+ 周 | 保持 |
| 8 | **传输层 TCP/UDP 代理** | InterceptSuite | P3 | 2-3 周 | 新增 |

---

### 12.10 总结: ProxyBot 竞争定位 v4

经过 4 轮竞品分析 (覆盖 ~25 个项目)，ProxyBot 的核心定位清晰:

```
ProxyBot = pf 透明代理 + DNS 服务器 + App 分类 + TUI+GUI 双界面 + Rust 核心
         + 规则引擎 + Breakpoint + Mock 生成 + DAG 分析 + Rhai 脚本
         + (即将) MCP Server + AI 两阶段分析 + GraphQL 解码 + Prometheus
```

**不可替代的护城河** (竞品均无):
1. **pf 透明代理** — 手机零配置抓包
2. **DNS 服务器 + DoH** — 内置 DNS 日志 + App 关联
3. **App 分类** — WeChat/Douyin/Alipay/AI 服务自动识别
4. **TUI + GUI 双界面** — 终端效率 + 桌面体验

**需要追赶的能力**:
1. **MCP Server** (P0) — anything-analyzer 先行
2. **AI 分析深度** (P1) — anything-analyzer 的两阶段分析更成熟
3. **过滤体验** (P1) — proxelar 的列作用域 DSL 更直观
4. **CA 安装体验** (P2) — hyperfox/proxelar 的 QR code 更友好
