# 产品对比与借鉴

ProxyBot 不以功能数量取胜。它的机会是成为一个聚焦的 macOS 移动应用流量调试
工具，让测试设备尽快产生一条可信、可操作的 Captured Request。

## 定位对比

| 项目 | 主要优势 | ProxyBot 应借鉴 | 不应照搬 |
| --- | --- | --- | --- |
| [mitmproxy](https://github.com/mitmproxy/mitmproxy) | 成熟、可编程，多种清晰分工的界面 | 明确的配置/证书/验证流程，逐步披露抓包模式 | 首次使用时展示所有高级模式 |
| [HTTP Toolkit](https://github.com/httptoolkit/httptoolkit) | 引导用户只拦截目标客户端 | 一键配置思路和流量降噪 | 在 macOS 路径可靠前扩展平台 |
| [Proxelar](https://github.com/emanuele-em/proxelar) | 脚本化本地流量工作台，Quick Start 简洁 | 证书安装页、冒烟请求、诚实限制、可复现打包 | 桌面主流程完成前维护多个同级界面 |
| [whistle](https://github.com/avwo/whistle) | 规则与插件生态 | 深化规则语义和扩展点 | 把扩展复杂度暴露为主产品 |
| [Anything Analyzer](https://github.com/DeepLifeStudio/anything-analyzer) | 统一 Session 与 AI 辅助分析 | 统一 Capture Session 边界 | AI-first 和全来源抓包范围 |

## ProxyBot 应保留的聚焦优势

1. macOS 桌面应用与可复用 Rust MITM Runtime。
2. Device-aware Captured Request，以及 DNS 支持的 Application Attribution。
3. Routing Rule、断点、Replay、Composer 与导出组成一个调试闭环。
4. 显式代理成功后，再按需启用透明路由和自动化 Adapter。

隐藏前置条件或不完整模式会直接破坏这一优势。

## 默认模式顺序

1. **显式代理** — Core，默认且完整记录。
2. **macOS `pf` + DNS** — Advanced，会修改主机网络且可能需要提权。
3. **MCP、Dashboard、脚本** — Advanced 自动化 Adapter。
4. **SSL Bypass、AI、生成与部署** — Labs，直到真实端到端
   路径和支持边界被验证。

## 持久的评估标准

后续对比不再使用易过期的 Star 数或巨大勾选矩阵，而是要求证据：

| 用户结果 | 所需证据 |
| --- | --- |
| 安装 | 签名/公证产物、校验和、干净 Mac 冒烟测试 |
| 首次抓包 | 计时的设备配置流程，以已知 HTTPS 请求结束 |
| 定位问题 | 稳定的详情和 device/host/method/status 筛选 |
| 修改行为 | Routing Rule 或断点通过本地 fixture 验证 |
| 复现问题 | Replay 结果和导出与原请求可比对 |
| 清理 | 正常、失败和退出路径都能停止抓包并恢复网络 |
| 扩展 | 具有契约测试的稳定 Interface，而不只是一个页面 |

具体执行顺序见[产品路线图](../roadmap.md)。
