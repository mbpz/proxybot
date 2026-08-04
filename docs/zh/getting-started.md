# 快速入门

本指南使用显式代理，因为它的配置和清理最小、最可预测。当前 Release 仍是开发
预览产物，因此受支持的贡献者路径是从源码运行。

## 准备工作

你需要：

- 安装了 Xcode Command Line Tools 的 Mac
- 稳定版 Rust 工具链
- Node.js 20 或更高版本、pnpm 10
- 与 Mac 处于同一 Wi-Fi 的 iOS 或 Android 测试设备
- 检查该设备及其流量的明确授权

不要在个人主力设备或生产设备上安装 ProxyBot CA。

## 1. 运行 ProxyBot

```bash
git clone https://github.com/mbpz/proxybot.git
cd proxybot
pnpm install --frozen-lockfile
pnpm tauri dev
```

打开 **Setup**，选择 iOS 或 Android，并按同一页面中的编号步骤操作。
**Start Capture** 控制 MITM Runtime；**Prepare iOS/Android Setup** 则独立发现
活动局域网接口并启动临时 CA 下载服务。一个流程失败不代表另一个已经运行。

macOS 菜单栏也提供 Capture Session 的 Start/Stop 操作，并与主窗口状态同步。

## 2. 准备 Mac

点击 **Prepare iOS Setup** 或 **Prepare Android Setup**。ProxyBot 会显示当前活动
接口的准确局域网地址和代理端口。不要在手机上填写 `127.0.0.1`，它代表手机自身。

如果准备失败，请确认 Mac 有可用的局域网地址，且防火墙允许 ProxyBot。切换网络后，
先停止 Setup Server，再重新准备。

## 3. 配置显式代理

编辑测试设备当前连接的 Wi-Fi，将 HTTP 代理设为 **手动**：

- **服务器：** Setup 页面显示的局域网地址
- **端口：** Setup 页面显示的端口（默认 `8088`）
- **认证：** 关闭

首次配置不要修改网关或 DNS；它们属于 Advanced `pf` + DNS 模式。

在设备上打开 `http://example.com`。Traffic 页面应出现一条 Captured Request。
如果没有出现，先检查两台设备是否在同一网络、代理是否运行，以及 macOS 防火墙
是否允许该应用。

## 4. 为 HTTPS 安装 CA

在临时 Setup Server 运行时扫描 Setup 页面中的二维码。iOS 只下载 ProxyBot CA，
ProxyBot 不会通过托管描述文件修改 Wi-Fi 或 DNS；Android 会打开本地配置指南和
CA 下载页。

在测试设备上安装 CA。iOS 还需前往 **设置 → 通用 → 关于本机 → 证书信任设置**，
对 ProxyBot CA 启用完全信任。

Android 是否信任用户 CA 取决于系统版本和应用配置。证书固定的应用可能在任一
平台拒绝连接；这是应用安全边界，ProxyBot 不保证能够解密。

打开 `https://example.com`，请求和响应应出现在 Traffic 页面。不要公开 CA 私钥、
捕获到的凭据或敏感请求正文。安装完成后点击 **Stop Setup Server**；这不会停止抓包。

## 5. 调试请求

- 在 Traffic 中按 host、method、status、device 或 application 筛选。
- 在详情中检查 headers 和 body。
- 需要修改行为时使用 Routing Rule 或断点。
- 使用 Replay 或 Composer 复现请求。
- 导出前先删除密钥、令牌和个人数据。

## 6. 清理

1. 在主窗口选择 **Stop Capture**，或从 ProxyBot 菜单栏选择 **Stop Proxy**。
2. 将设备 Wi-Fi 代理恢复为 **关闭**。
3. 如果 Setup Server 仍在运行，点击 **Stop Setup Server**。
4. 不再使用时，从测试设备移除 ProxyBot 描述文件和 CA。
5. 如果启用了 Advanced `pf`，退出前先关闭它。

## 已知限制

- 项目尚未通过受维护的 Homebrew tap 分发。
- 现有 GitHub ZIP 还不代表目标中的签名、公证和安装冒烟测试流水线。
- Playwright 使用模拟桌面 Adapter，不能证明真实设备流程。
- SSL Bypass 属于 Labs，不是受支持的配置路径。

下一步可阅读[架构](architecture.md)、[产品对比](comparison.md)与
[产品路线图](../roadmap.md)。
