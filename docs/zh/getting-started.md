# 快速入门

## 系统要求

- macOS（需要使用 `pf` 透明代理）
- Rust 工具链（从源码构建时需要）
- Homebrew（推荐安装方式）

## 安装

### Homebrew（推荐）

```bash
brew install --cask mbpz/tap/proxybot
```

### 从源码构建

```bash
git clone https://github.com/mbpz/proxybot.git
cd proxybot/src-tauri
cargo build --release --bin proxybot
./target/release/proxybot
```

## 设备设置

### 第一步：将手机连接到 Mac 的网络

确保您的 iOS/Android 设备与 Mac 处于同一 WiFi 网络。

### 第二步：配置设备网关

在手机上设置：
- **网关**：Mac 的 IP 地址
- **DNS**：Mac 的 IP 地址

查看 Mac 的 IP 地址：
```bash
ipconfig getifaddr en0
```

### 第三步：安装 CA 证书

1. 启动 ProxyBot
2. 导航到 **证书** 标签页
3. 导出 CA 证书
3. 通过 AirDrop 将证书发送到手机
4. 在 iOS 上：**设置 → 通用 → 关于 → 证书信任设置** → 启用对 ProxyBot CA 的完全信任

### 第四步：开始代理

1. 在 ProxyBot 中点击 **启动代理**
2. 实时观察手机流量

## 下一步

- 了解 [键盘快捷键](keyboard-shortcuts.md)
- 探索 [架构](architecture.md)
- 对比 [其他工具](comparison.md)
