# 架构

ProxyBot 将可复用的 MITM Runtime 与平台、存储和界面 Adapter 分离。目标不是增加
层数，而是把抓包、持久化、桌面行为和实验能力封装在具有小 Interface 的深 Module
中。规范领域语言见仓库根目录的
[`CONTEXT.md`](https://github.com/mbpz/proxybot/blob/main/CONTEXT.md)。

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
桌面或 MCP 启动 Adapter，创建共享资源并负责进程退出。

## Module 与 Seam

### `proxybot-core`

可复用核心拥有：

- 校验后的 Process Config 与 Runtime Config
- MITM Runtime 及其生命周期句柄
- TLS 证书生成和拦截决策
- Routing Rule 模型与匹配
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
和用于快速 UI 测试的 BrowserMockAdapter。迁移尚未完成：部分页面仍直接调用
Tauri，或通过浅 Adapter 将错误转换为 `null`。

## 网络模式

### 显式代理 — Core

设备将 HTTP/HTTPS 连接直接发送给 MITM Runtime。这是默认模式，因为配置与清理
只发生在测试设备。

### macOS `pf` 与 DNS — Advanced

桌面 Adapter 可以安装独立 `pf` 重定向，并运行 DNS Server 产生 DNS Observation
与 Application Attribution。该模式会修改主机网络状态，可能需要提权，并要求明确
清理。

### TUN 与 iOS VPN — Labs

当前 TUN Implementation 能创建设备，但没有完整的数据包转发路径接入 MITM
Runtime。iOS 实验也依赖当前不存在的 Mac tunnel peer。两者都不是受支持的传输路径。

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
- `pf`、DNS、TUN、证书分发、Dashboard 与 MITM listener 都必须是具有明确
  所有权和清理行为的 Desktop Network Resource。
- MCP stdio 是本地 Adapter，但会向客户端暴露敏感的持久化数据。
- Android SSL Bypass 会修改应用，只能属于 Labs。
- 当前全局 Tauri API 与空 CSP 是迁移债务；目标是最小 capability 和非空 CSP。

## 验证边界

Rust 与 UI 测试覆盖 Module 和 BrowserMockAdapter。当前 Playwright 启动 Vite，
而不是打包后的 Tauri 应用。只有真实桌面验收流程通过安装、启动、证书配置、抓包、
停止、重启和清理后，Release 才能视为被验证。

按顺序执行的深化工作见[产品路线图](../roadmap.md)。
