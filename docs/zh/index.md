# ProxyBot

**面向移动应用开发者的 macOS 桌面流量调试工具。**

ProxyBot 让 iOS 或 Android 测试设备连接到本机 Rust MITM Runtime，以便检查、
筛选、修改、重放和导出 HTTP、HTTPS 与 WebSocket Captured Request。

!!! warning "开发预览版"

    项目尚未形成稳定、经过公证的 macOS 分发。现有 GitHub Release 是预览产物；
    当前面向贡献者的受支持方式是从源码运行。

## 收敛后的核心流程

1. 在 Mac 上启动 ProxyBot。
2. 通过显式代理连接测试设备。
3. 需要检查 HTTPS 时安装并信任本地 CA。
4. 验证一个已知请求出现在 Traffic 页面。
5. 检查、修改、重放或导出 Captured Request。
6. 停止抓包并恢复设备网络设置。

[快速入门](getting-started.md){ .md-button .md-button--primary }
[查看产品路线图](../roadmap.md){ .md-button }

## 核心能力

- HTTP、HTTPS 与 WebSocket 抓取
- Captured Request 历史、详情、筛选和导出
- Routing Rule、断点、Replay 与 Composer
- 证书导出和本地设备配置服务
- DNS 支持的 Application Attribution
- 可复用的 Rust `proxybot-core` crate

## 产品边界

显式代理是默认配置方式。macOS `pf`、DNS、MCP、脚本、移动 Dashboard 与协议
分析属于 Advanced。TUN/iOS VPN、SSL Bypass、AI、生成与部署功能属于 Labs，
不在受支持的首次抓包路径内。

ProxyBot 只能用于你拥有或明确获准测试的设备和网络。抓包内容和本地 CA 材料
都属于敏感数据。

## 更多文档

- [快速入门](getting-started.md)
- [架构](architecture.md)
- [产品对比与借鉴](comparison.md)
- [产品路线图](../roadmap.md)
- [贡献指南](https://github.com/mbpz/proxybot/blob/main/CONTRIBUTING.md)
- [安全策略](https://github.com/mbpz/proxybot/blob/main/SECURITY.md)
