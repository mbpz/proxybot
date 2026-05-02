# ProxyBot 竞品分析报告

## 1. Executive Summary

本报告分析了 GitHub 上最热门的移动端 HTTPS/MITM 流量调试工具，按 stars 排序进行深度对比。

**ProxyBot 定位**: macOS 原生 TUI 工具，面向移动端（iOS/Android）HTTPS 流量抓包，支持 app 分类（微信/抖音/支付宝），pf 透明代理 + 内置 DNS 服务器。

**核心差异**: 竞品大多为通用工具，无 app 级别分类能力；ProxyBot 是唯一一个将 DNS 关联 + SNI 检测 + 域名规则库结合做 app 识别的开源项目。

---

## 2. 竞品总览（按 GitHub Stars 排序）

| 项目 | Stars | 定位 | Tech Stack | 平台 |
|------|-------|------|-----------|------|
| mitmproxy | 43,336 | 通用 MITM 代理 | Python | 全平台 |
| ffuf | 15,973 | Web Fuzzer | Go | 全平台 |
| Hetty | 10,127 | 安全研究 HTTP 工具 | Go | 全平台 |
| httpx | 9,874 | HTTP 探测工具 | Go | 全平台 |
| Mockoon | 8,231 | Mock API | TypeScript | 全平台 |
| anyproxy | 7,920 | 通用 HTTP/HTTPS 代理 | Node.js | 全平台 |
| **spy-debugger** | **7,620** | 微信/WebView 移动调试 | JavaScript | 全平台 |
| **Proxyman** | **6,799** | macOS 原生调试代理 | Obj-C/Swift | macOS/iOS/Android |
| betwixt | 4,562 | Chrome DevTools 风格代理 | JavaScript | 全平台 |
| HTTP Toolkit | 3,488 | 现代 HTTP 调试 UI | TypeScript/Electron | 全平台 |
| lightproxy | 3,186 | 跨平台代理 | TypeScript | macOS/Linux/Win |
| james | 1,442 | Web 调试代理 | JavaScript | 全平台 |
| atlantis | 1,500 | iOS 无代理抓包 | Swift | iOS |
| broxy | 1,011 | Go HTTP/HTTPS 代理 | Go | 全平台 |
| HTTP Toolkit Android | 607 | Android 自动拦截 | Kotlin | Android |

---

## 3. 功能对比矩阵

### 核心功能对比

| Feature | mitmproxy | spy-debugger | Proxyman | HTTP Toolkit | ProxyBot |
|---------|-----------|--------------|----------|--------------|----------|
| MITM HTTPS 拦截 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 移动端抓包 | ✅ | ✅ | ✅ | ✅ | ✅ |
| pf/透明代理 | ❌ | ❌ | ❌ | ❌ | ✅ |
| 内置 DNS 服务器 | ❌ | ❌ | ❌ | ❌ | ✅ |
| **App 分类** | ❌ | ❌ | ❌ | ❌ | ✅ |
| TUI 界面 | ✅ (mitmweb) | ❌ | ❌ (GUI) | ❌ (Electron) | ✅ |
| GUI 界面 | ✅ | ✅ | ✅ | ✅ | ❌ (Phase 2) |
| WebView 调试 | ❌ | ✅ | ✅ | ❌ | ❌ |
| iOS 无代理抓包 | ❌ | ❌ | ✅ (Atlantis) | ❌ | ❌ |
| Android 抓包 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 规则引擎 | ✅ (Filter) | ✅ (Rule) | ✅ | ✅ | ✅ |
| WebSocket 调试 | ✅ | ❌ | ✅ | ✅ | ✅ |
| HAR 导出 | ✅ | ❌ | ✅ | ✅ | ✅ |
| 流量重放 | ✅ | ❌ | ✅ | ✅ | ✅ |
| 自动化脚本 | ✅ | ❌ | ❌ | ✅ | ❌ |
| 请求修改/篡改 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 多级代理链 | ✅ | ❌ | ✅ | ✅ | ❌ |
| Docker 支持 | ✅ | ✅ | ❌ | ✅ | ❌ |

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

### 5.2 透明代理能力

mitmproxy、HTTP Toolkit 均需客户端手动设置 HTTP(S) 代理。
ProxyBot 通过 pf 透明代理实现:
- 手机无需任何代理配置
- 手机网关/DNS 指向 PC → 流量自动被抓
- 80/443 端口透明劫持

### 5.3 TUI 优先

唯一采用 TUI 界面的开源移动抓包工具:
- 键盘驱动，效率高
- 无需图形环境（远程服务器可用）
- Rust 实现，性能好
- 60fps 渲染，流畅

---

## 6. ProxyBot 竞争劣势

| 劣势 | 说明 | 缓解方案 |
|------|------|---------|
| 无 GUI | 非技术人员不友好 | Phase 2 Tauri React UI |
| 规则系统弱 | 无 map remote/local/breakpoint | 参考 Proxyman 规则格式 |
| 无 WebView 调试 | spy-debugger 特有功能 | 可考虑集成 |
| iOS 无代理抓包 | Proxyman Atlantis 独有 | 研究 VPN API |
| 无流量修改 | 仅监控 | Phase 2 加入 |
| 无自动 CA | 需手动安装 | 简化安装流程文档 |
| 文档少 | vs mitmproxy | 完善 README + 示例 |

---

## 7. 执行计划

### Phase 1: 强化现有能力

- [ ] **完善 README 对比表格**: 明确 vs mitmproxy/spy-debugger 的差异
- [ ] 添加竞品对比文档到 `docs/competitors/`
- [ ] 实现 spy-debugger 类似的 WebView 调试能力（可选）

### Phase 2: 缩小差距

- [ ] 实现 Proxyman 规则系统（Map Remote/Local/Breakpoint）
- [ ] 实现 Atlantis 类似的 iOS 无代理抓包（VPN API）
- [ ] Tauri React GUI（非技术人员友好）
- [ ] 流量修改/篡改能力

### Phase 3: 差异化创新

- [ ] app 分类能力开放为 SDK（其他工具可集成）
- [ ] 支持 Windows（pfctl → Windows Filtering Platform）
- [ ] 云端协作（多设备流量汇总）

---

## 8. 关键结论

1. **mitmproxy** 是开源 MITM 代理的事实标准，但 ProxyBot 的 app 分类 + 透明代理是独特差异点
2. **Proxyman** 是最接近商业品质的竞品，Atlantis iOS 无代理抓包是技术亮点
3. **spy-debugger** 虽停止维护，但微信调试概念有价值
4. **HTTP Toolkit** 的 UI 设计和自动 CA 配置值得借鉴
5. ProxyBot 的核心竞争优势是 **app 分类 + pf 透明代理 + TUI**，这三点是其他竞品没有的组合
6. ProxyBot 最大差距是**规则系统**和**无 GUI**，Phase 2 应优先解决
