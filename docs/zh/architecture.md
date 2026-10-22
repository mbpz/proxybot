# 架构

ProxyBot 将可复用的 MITM Runtime 与平台、存储和界面 Adapter 分离。目标不是增加
层数，而是把抓包、持久化、桌面行为和实验能力封装在具有小 Interface 的深 Module
中。规范领域语言见仓库根目录的
[`CONTEXT.md`](https://github.com/mbpz/proxybot/blob/main/CONTEXT.md)；具有约束力
的决策见 [`docs/adr/`](../adr/README.md)。

## 运行时组合

```text
Process Config
     |
     v
桌面 composition root ------------------------- MCP stdio Adapter
     |                                                |
     +--> macOS / Tauri Adapter                       |
     +--> SQLite Adapter <----------------------------+
     +--> 证书与 DNS 资源
     |
     v
MITM Runtime --> Capture Event --> 桌面 Runtime Adapter
     |                                  |
     |                                  +--> Captured Request 持久化
     |                                  +--> Application Attribution
     |                                  +--> Alert 与分析
     |                                  +--> 桌面事件
     v
上游服务
```

`src-tauri/src/bootstrap.rs` 是唯一 composition root：解析 Process Config，选择
桌面或 MCP 启动 Adapter，创建共享资源并负责进程退出。它刻意保持精简：不存在第二
个解析配置或打开数据库的入口。

构建是自包含的：不打包原生工具链或下载的 gadget 资源；桌面应用、MCP 模式和
验收流程都从仓库直接构建。

## Module 与 Seam

### `proxybot-core`

可复用核心拥有：

- 校验后的 Process Config 与 Runtime Config
- MITM Runtime 及其生命周期句柄
- TLS 证书生成和拦截决策
- Routing Rule 模型与匹配，包括 per-host TLS rule
- Capture Event 类型
- Application Attribution 和分析模型
- 规范生成基础能力

MITM Runtime 通过 `RuntimeHooks` 与 `OriginalDestination` Seam 接入外部能力，
不依赖 Tauri、SQLite 或 macOS `pf`。

### 桌面 Runtime Adapter

`src-tauri` 提供桌面 Implementation。Runtime Adapter 将 Capture Event 转换为
持久化 Captured Request、WebSocket 记录、Application Attribution、Alert、分析
输入和 Tauri 事件。

Runtime Extension Pipeline 负责有序的插件分派、Rhai 脚本、指标和 Network
Condition Rule；它与 Routing Rule 是不同概念。

### 持久化

SQLite 存储 Captured Request、Device、Alert、DNS Observation、配置与生成状态。
聚焦的查询 Module 应拥有 SQL 和映射；桌面与 MCP Adapter 不应直接共享
`Mutex<Connection>`。

### React 桌面 Adapter

React 应用负责桌面产品界面。生成式 Desktop Contract 提供类型化命令/事件元数据
和用于快速 UI 测试的 BrowserMockAdapter。生成模块目前覆盖 123 个桌面命令中的
36 个，以及 4 个事件。**迁移尚未完成：** 其他页面仍直接调用 Tauri；把错误转换成
`null` 的浅 `safeInvoke` 已删除，但这些调用点仍需迁移到类型化契约。

界面只有三个 Product Destination：**Capture**、**Setup** 与 **Settings**。

- **Capture** 拥有请求工作区及其上下文导航：Requests、Rules、Replay、DNS、
  Alerts、Analysis。Replay 与 Composer 共享 Replay 上下文。
- **Setup** 拥有设备接入流程；证书面板与已注册设备面板是保留的深链，从 Setup
  卡片进入。
- **Settings** 拥有通用设置、网络设置、DNS、Labs 开关和关于页面。

Labs 界面（AI 分析 `/ai`、Generate `/gen`、Deploy `/deploy`、Graph `/graph`、
Topology `/topology`）按需加载并做代码分割。它们默认关闭、不出现在默认导航中，
并由 `LabsGate` 拦截。开关是本地 UI 状态（`localStorage` 键
`proxybot.labs.enabled`），在 Settings → Labs 中管理；未开启时访问对应路由只显示
"Labs are off"页面，不涉及任何后端命令。

历史与实验路由仍作为合法深链保留，避免旧 URL 失效，但不参与默认导航。

General Settings 以及 Capture 的 DNS、Alerts 界面通过生成的 Desktop Contract
读取和修改数据、DNS Observation、流量基线与异常扫描。传输层和结果结构失败在
界面中保持为类型化错误，不会被转换成 false、空或 null 状态。

## 网络模式

### 显式代理 — Core

设备将 HTTP/HTTPS 连接直接发送给 MITM Runtime。这是默认模式，因为配置与清理
只发生在测试设备。

### macOS `pf` 与 DNS — Advanced

桌面 Adapter 可以安装独立 `pf` 重定向，并运行 DNS Server 产生 DNS Observation
与 Application Attribution。该模式会修改主机网络状态，可能需要提权，并要求明确
清理。

## Capture 生命周期

目标所有权模型是：

1. listener 成功 bind 或设备配置完成后，资源才被报告为 running；
2. 对运行中资源重复 start 会明确失败；
3. stop 是幂等的；
4. stop 只在拥有的 task、listener 与 handle 全部释放后返回；
5. 进程退出会排空 MITM Runtime 和桌面网络资源。

核心 Runtime 已拥有 listener 句柄。桌面层仍需把 Capture Event bridge 与
breakpoint task 纳入同一个 retained Capture Session Module。

## Captured Request 数据流

```text
客户端连接
    --> MITM Runtime
    --> 具有稳定 id 的 Capture Event
    --> 桌面 Runtime Adapter
    --> Application Attribution
    --> SQLite Captured Request
    --> Desktop Contract event
    --> Traffic workspace
```

分析 Implementation 消费不可变的 Captured Request Analysis 视图，避免 Graph、
Topology、认证和异常检测各自重新解释数据库记录。

## 安全边界

- 本地 CA 私钥和捕获到的凭据都是秘密。
- 修改主机全局网络前优先使用显式代理。
- `pf`、DNS、证书分发、Dashboard 与 MITM listener 都必须是具有明确
  所有权和清理行为的 Desktop Network Resource。
- MCP stdio 是本地 Adapter，但会向客户端暴露敏感的持久化数据。
- Labs gate 是导航/产品边界，不是安全边界：Labs 命令仍编译在桌面二进制中。
- 当前全局 Tauri API（`withGlobalTauri: true`）与空 CSP（`"csp": null`）仍然
  存在，属于迁移债务；目标是最小 capability 和非空 CSP。

## 验证边界

Rust 与 UI 测试覆盖 Module 和 BrowserMockAdapter。当前 Playwright 在浏览器中启动
Vite，并使用模拟桌面 Adapter，而不是打包后的 Tauri 应用。打包桌面验收流程执行
真实 `.app` 二进制与 Tauri composition root，在隔离工作区内证明 CA 准备、本地
HTTPS 解密抓取、SQLite 记录、停止、重启与清理，不需要外部网络。CI 从未签名的
app bundle 运行该流程。它不驱动可见 UI，也不证明签名安装；这些仍属于发布证据
要求。

按顺序执行的深化工作见[产品路线图](roadmap.md)，仍然有约束力的设计决策见
[ADRs](../adr/README.md)。
