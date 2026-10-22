# ProxyBot 产品路线图

**状态：** 轻量化收敛后的产品收敛计划

**最近评审：** 2026-10-22

**主要用户：** 在 Mac 上调试测试设备的移动应用开发者

本文档是产品与交付的规范来源。`docs/history/sdd/` 与
`docs/history/superpowers/` 下的历史计划只解释早期实验，不证明某个能力已受支持
或已发布。

1.3.0 是轻量化收敛版本：移除了原生工具链，并把默认导航收敛为三个目的地。相关
决策记录在 [ADR-0007](../adr/0007-lightweight-positioning.md) 与
[ADR-0005](../adr/0005-product-tiers.md)；轻量化之前的仓库状态归档在 tag
`archive/pre-lightweight-2026-10-22`。

## 产品论点

ProxyBot 应该是从「测试设备行为异常」到一条可信 Captured Request 的最短路径，
开发者可以检查、修改、重放和导出它。

受支持的工作流刻意保持狭窄：

1. 在 macOS 上启动一个 Capture Session。
2. 通过显式代理连接 iOS 或 Android 测试设备。
3. 需要 HTTPS 检查时安装并信任本地 CA。
4. 先验证一个已知 HTTP 请求，再验证一个 HTTPS 请求。
5. 按 device、application、host、method 或 status 找到 Captured Request。
6. 检查、修改、重放或导出它。
7. 停止抓包并恢复设备网络设置。

首次成功解密的请求，比页面、协议、生成器或实验抓包模式的数量更重要。

## 产品契约

### Core

Core 是受支持路径，必须保持可见、有文档，并由真实桌面验收测试覆盖。界面恰好
暴露三个 Product Destination：**Capture**、**Setup**、**Settings**。请求派生的
工具位于 Capture 上下文，而不是作为同级导航项竞争。

| 领域 | 用户结果 | 目标界面 |
| --- | --- | --- |
| Capture | 启动、停止、恢复并理解抓包状态 | 持久 Capture Session 控件；Capture 目的地 |
| Setup | 连接设备、安装 CA、验证流量并清理 | 单一引导式 Device Onboarding；Setup 目的地 |
| Inspect | 搜索并理解 Captured Request | Traffic 工作区，带上下文化 DNS 与 Application Attribution |
| Modify | 安全地应用 Routing Rule 或断点 | Rules 与内联断点控件（Capture 上下文） |
| Reproduce | 重发请求或组合新请求 | Replay 与 Composer（Replay 上下文） |
| Share | 导出可用的脱敏产物 | HAR 与请求代码导出；请求详情操作 |
| Configure | 修改端口、存储、TLS、更新与 Labs 开关 | Settings 目的地 |

### Advanced

Advanced 能力在 Core 工作流稳定后记录，且永远不是首次成功的前提：

- macOS `pf` 透明路由与内置 DNS Server
- MCP stdio Adapter
- Runtime Extension Pipeline 与 Rhai 脚本
- 移动 Dashboard
- 协议解码与流量分析

### Labs

Labs 是实验，不是产品承诺。它们默认关闭、不进入默认导航，也不出现在发布声明中。
用户在 **Settings → Labs** 中开启（本地 `localStorage` 键
`proxybot.labs.enabled`），之后可从 Settings 与命令面板（⌘K）进入。未开启时访问
Labs 路由只显示"Labs are off"页面。

当前 Labs：

- AI 分析（模型与 token 用量检查）
- Generate（spec、mock、scaffold）与 Deploy 生成界面
- Graph 与 Topology 分析视图，直到它们成为一致的分析工作流

被移除的能力不会在没有新 ADR、受支持用户旅程、失败模型与真实 Adapter 测试的
情况下重新引入。1.3.0 轻量化已移除其中一个实验性原生工具链，记录见 ADR-0007。

Labs 能力毕业需要：负责人、受支持的用户旅程、失败模型、文档，以及通过真实
Adapter 的测试。

## 轻量化定位

ProxyBot 以可靠的 macOS 首次抓包工作流竞争，而不是功能数量。1.3.0 收敛使其具体化：

- **不打包原生工具链。** 构建与运行不需要原生工具链或二次资源下载；一次干净的 `pnpm install` 就是全部依赖。
- **构建/打包阶段不下载产物。** Tauri bundle 直接从仓库构建；发布资产是签名 DMG、
  校验和与 SBOM。
- **默认界面更小。** 三个目的地；请求派生的工具留在 Capture 上下文。Labs 专用
  依赖（mermaid、vis-network、recharts、highlight.js）按需加载并代码分割，初始
  shell chunk 约 467 kB（1.3.0 分割前约 1.9 MB）。
- **二进制与依赖图更小。** 1.3.0 清理移除了原生工具链栈和不可达的占位模块，
  减少了 Rust 直接依赖与编译源码树。

不在轻量化产品范围内：浏览器级自动化或 DevTools 替代品、通用抓包分析器、VPN，
或 AI-first 流量平台。

## 收敛现状

1.3.0 已解决：

- 默认导航变为三个 Core 目的地；Rules、Replay、DNS、Alerts、Analysis 移入
  Capture 上下文，深链保持有效。
- 设备接入合并为一个流程：LAN 发现、临时 CA 服务、显式代理、平台说明、QR、
  验证与清理都在 Setup；证书与设备面板从 Setup 卡片进入。
- 移除了不受支持的 TUN 模式与断连的 iOS VPN 实验。
- 包元数据、更新检查、MCP 元数据、Tauri、Rust 版本与发布 Tag 都来自单一 Product
  Version。
- 发布流水线使用 Tauri bundler，包含 Developer ID 签名、公证、校验和、SBOM、
  来源证明与打包桌面验收。
- 一个实验性原生工具链及其资源管线已移除（ADR-0007）。
- 文档收敛：ADR、CHANGELOG、SUPPORT、CODEOWNERS，历史文档归档且不参与站点构建，
  并重写了仓库指南。

仍未完成：

- 尚未产出签名、公证、安装验证后的 Release；阻塞于 Apple 凭据配置
  （[issue #27](https://github.com/mbpz/proxybot/issues/27)）与物理 iOS/Android
  验证证据（[issue #28](https://github.com/mbpz/proxybot/issues/28)）。
- Desktop Contract 仅部分覆盖：123 个桌面命令中 36 个已类型化。
- Tauri 安全债务仍在：`withGlobalTauri: true` 与 `"csp": null`。
- 打包验收只证明后端旅程；可见 UI 交互与签名安装证据仍是发布门禁。
- 浏览器 e2e 仍使用模拟 Adapter。

## 与同类项目的关系

借鉴交互原则，而不是照搬功能广度：显式代理作为默认路径（mitmproxy）、围绕目标
客户端组织接入（HTTP Toolkit）、简洁 Quick Start 与诚实限制（Proxelar）、规则
可扩展但不暴露复杂度（whistle）、统一 Session 边界但不做 AI-first 全来源抓包
（Anything Analyzer）。

## Rockxy Community 兼容计划

[设计](https://github.com/mbpz/proxybot/blob/main/docs/history/superpowers/specs/2026-08-19-rockxy-community-parity-design.md)、
[里程碑计划](https://github.com/mbpz/proxybot/blob/main/docs/history/superpowers/plans/2026-08-19-rockxy-community-parity-milestones.md)、
[证据账本](https://github.com/mbpz/proxybot/blob/main/docs/parity/rockxy-community-evidence.md) 与
[机器可读矩阵](https://github.com/mbpz/proxybot/blob/main/docs/parity/rockxy-community-matrix.md) 定义 Rockxy Community 兼容
计划，固定于 `RockxyApp/Rockxy@6a676d631820b577cf3a651c78d856733a7df995`，严格
限定为公共 Community 的 clean-room 工作。

矩阵是能力证据、状态、owner 与验收的权威；本路线图仍是产品优先级权威。功能数量
对等不能豁免 ProxyBot 的首次成功、脱敏、安全或发布门禁。

## 执行顺序

### P0 — 让首次抓包真实且可重复

状态：本地旅程已就位；发布证据门禁仍未完成。

1. **Capture Session Module**：统一状态、前置条件、start/stop、失败与恢复接口，
   由桌面 shell 与 menu-bar 共用；已完成 shell/tray tracer slice；剩余：把前置
   条件与失败指引收进同一 Module，并让 Rust 侧拥有 Capture Event bridge 与
   breakpoint task。
2. **Device Onboarding Module**：已完成 LAN 发现、临时证书分发、Setup 页面、
   iOS CA-only 交付、Android 信任指引与重复证书/QR 入口清理；剩余：在物理设备上
   验证并保留发布证据。
3. **隔离不完整网络实验**：已完成 TUN 与断连 iOS VPN 的移除。
4. **单一 Release/Install/Update 来源**：已完成版本单一来源与 Tauri bundler
   签名/公证流水线定义；剩余：在两种架构上成功运行托管流水线并记录安装/启动证据
   （issue #27）。
5. **真实桌面验收旅程**：已完成打包可执行文件的隔离验收（CI 从未签名 app bundle
   运行，Release 从每个已验证 DMG 运行）；剩余：可见 UI 交互与签名安装证据。

**P0 退出条件**

- 新用户能在五分钟内看到一条解密后的测试请求，且不依赖未记录的前置条件。
- 主窗口显示抓包状态和可恢复的错误信息。
- 每个已发布版本在元数据、UI、MCP、Tag 与产物之间一致。
- 没有任何 Core 文档把 Labs 能力描述为已发布。

### P1 — 让核心调试闭环连贯

1. 默认导航收敛：已完成三个 Product Destination；Requests、Rules、Replay、DNS、
   Alerts、Analysis 位于 Capture 上下文；Replay 与 Composer 共享 Replay 上下文；
   深链保持有效。
2. 让 Captured Request 持久化成为 history、desktop、MCP 与分析共用的单一查询 Seam。
3. 统一 Traffic、导出、MCP、Graph、Topology 与 Alerts 的过滤语言和查询语义。
4. 把生产 UI 调用迁移到生成的 Desktop Contract：`safeInvoke` 已移除；已完成
   General Settings、DNS、Alerts 与 Capture Session 的类型化；剩余 123 个命令中
   的 87 个仍直接 `invoke`，需要类型化错误。
5. 增加 redaction-first HAR 导出与可复现的 issue-report bundle。

**P1 退出条件**

- 默认侧栏没有重复目的地或实验产品。
- 同一过滤条件在每个 Adapter 返回相同 Captured Request。
- Tauri 命令失败是类型化产品状态，而不是静默空数据。
- issue report 默认可以包含诊断信息而不含密钥或抓包正文。

### P2 — 提升可靠性与架构深度

1. 让运行中的桌面 MITM Module 拥有 core runtime、Capture Event bridge、breakpoint
   task 与确定性退出。
2. SQLite 细节保持在聚焦的 Captured Request、Alert、Device、Routing Rule
   Interface 之后，避免 Adapter 共享 `Mutex<Connection>`。
3. MCP 保持为同一域 Module 之上的 headless Adapter；移除重复 SQL，或者实现真正的
   headless Capture Session，或者删除该声明。
4. 把 BrowserMockAdapter 套件变成快速契约覆盖，并为跨 Seam 行为保留较小的真实
   Tauri 验收通道。
5. 当变更影响公共 Interface、持久化模型、安全边界或发布契约时，用 `docs/adr/`
   记录架构决策 —— 已完成 ADR-0001 至 ADR-0008。

**P2 退出条件**

- stop 只在所有 task 与网络资源释放后返回。
- Desktop 与 MCP 暴露相同的持久事实与术语。
- 主要 Interface 有契约测试和明确 owner。
- 安全敏感配置使用非空 CSP 与最小 Tauri capability。

### P3 — 成长为开源项目，而不是功能目录

1. 发布 changelog、支持矩阵、兼容性策略与发布节奏 —— 已完成
   CHANGELOG/SUPPORT/CONTRIBUTING/SECURITY/CODEOWNERS 与 ADR；剩余仓库
   description/topics 与已验证升级路径。
2. 只有在 P0 旅程稳定后才增加截图或首次抓包录像。
3. 从 Core 路线图维护 good-first-issue 与 help-wanted 任务。
4. 度量选择加入、保护隐私的产品结果：安装成功率、首次 Captured Request 时间、
   setup 失败阶段与无崩溃 Capture Session。
5. 根据真实用户需求逐个毕业 Labs 能力。

**P3 退出条件**

- Release 带 notes、校验和、支持声明与已验证升级路径。
- 贡献者只依赖仓库文档即可选择、构建、测试并提交 Core issue。
- 路线图变更引用用户证据或已接受的架构决策。

## 架构深化候选

按用户影响与杠杆排序；详细 Interface 属于已接受的设计或 ADR。

1. Capture Session 生命周期与 shell/tray Adapter — 部分深化
2. Device Onboarding 与证书分发生命周期 — 本地旅程完成；物理设备证据待补
3. 移除不完整的 TUN/iOS VPN 产品面 — 已完成
4. React 应用全面采用生成的 Desktop Contract — 进行中
5. MCP 作为共享域 Module 之上的 Adapter — 待办
6. Release/Install/Update 作为单一可复现 Module — 流水线完成，托管证据待补
7. 桌面 MITM 拥有全部子 task — 待办
8. 真实用户旅程作为验收测试 Interface — 后端旅程已完成

## 当前周期的非目标

- 成为通用 VPN、抓包分析器或浏览器级自动化 / DevTools 替代品
- 实现所有传输协议
- 在漏洞或规避广度上与安全套件竞争
- 让 AI、代码生成或部署生成成为主产品
- 重新引入已移除的实验性原生工具链（见 ADR-0007）
- 在 macOS 首次抓包可靠前支持 Windows 或 Linux
- 承诺拦截证书固定或平台保护的应用
- 为解锁功能而打包或下载原生工具链

## 路线图治理

- 「受支持」要求：有文档的旅程、真实 Adapter 测试、维护者与失败/清理行为。
- 「完成」要求退出条件在发布产物上通过，而不只是单元或浏览器 mock 测试。
- 产品新增必须说明什么离开默认界面，或为什么新复杂度属于 Core。
- 历史计划可以解释决策，但不能覆盖本路线图、`CONTEXT.md`、当前代码或发布证据。
- Labs 能力在毕业前绝不出现在 release note 或默认导航中。
