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
