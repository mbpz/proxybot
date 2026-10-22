# ProxyBot

**面向移动应用开发者的轻量 macOS MITM 流量调试工具。**

ProxyBot 让 iOS 或 Android 测试设备通过显式代理连接本机 Rust MITM Runtime，
以便检查、筛选、修改、重放和导出 HTTP、HTTPS 与 WebSocket Captured Request，
且不需要打包原生工具链或额外下载资源。

!!! warning "开发预览版"

    项目尚未形成稳定、经过公证的 macOS 分发。现有 GitHub Release 是预览产物；
    当前面向贡献者的受支持方式是从源码运行。

## 聚焦工作流

1. 在 Mac 上启动 ProxyBot。
2. 通过显式代理连接测试设备。
3. 需要检查 HTTPS 时安装并信任本地 CA。
4. 验证一个已知请求出现在 Capture → Requests。
5. 检查、修改、重放或导出 Captured Request。
6. 停止抓包并恢复设备网络设置。

[快速入门](getting-started.md){ .md-button .md-button--primary }
[查看产品路线图](roadmap.md){ .md-button }

## 核心能力

- HTTP、HTTPS 与 WebSocket 抓取
- Captured Request 历史、详情、筛选和导出
- Routing Rule、断点、Replay 与 Composer
- 请求详情操作：重放、从请求创建规则、复制为 cURL、导出 HAR
- 证书导出和本地设备配置服务
- DNS 支持的 Application Attribution
- 可复用的 Rust `proxybot-core` crate

默认界面只有三个 Product Destination：**Capture**、**Setup** 与 **Settings**。
Requests、Rules、Replay、DNS、Alerts 和 Analysis 位于 Capture 上下文；
Replay 与 Composer 共享 Replay 上下文。

## 产品边界

显式代理是默认配置方式。macOS `pf`、DNS、MCP、脚本、移动 Dashboard 与协议
分析属于 Advanced 能力。

Labs（AI 分析、Generate、Deploy、Graph/Topology 分析视图）属于实验特性，默认
关闭，不在受支持的首次抓包路径内。可在 **Settings → Labs** 中开启；未开启时
访问 Labs 路由只会显示"Labs are off"页面。
ProxyBot 只能用于你拥有或明确获准测试的设备和网络。抓包内容和本地 CA 材料
都属于敏感数据。

## 更多文档

- [快速入门](getting-started.md)
- [架构](architecture.md)
- [产品对比与借鉴](comparison.md)
- [产品路线图](roadmap.md)
- [架构决策](../adr/README.md)
- [变更日志](https://github.com/mbpz/proxybot/blob/main/CHANGELOG.md)
- [支持策略](https://github.com/mbpz/proxybot/blob/main/SUPPORT.md)
- [贡献指南](https://github.com/mbpz/proxybot/blob/main/CONTRIBUTING.md)
- [安全策略](https://github.com/mbpz/proxybot/blob/main/SECURITY.md)
