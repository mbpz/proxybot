# 快速开始

## 安装

macOS：

```bash
curl -fsSL https://github.com/mbpz/proxybot/releases/latest/download/install.sh | sh
proxybot --version
```

默认安装到 `~/.local/bin`。

Windows x64：下载同一 CLI Release 中的 `proxybot-min-x86_64-pc-windows-msvc.zip`
及对应 `.sha256`，用 PowerShell `Get-FileHash -Algorithm SHA256` 核对后解压，
运行 `proxybot.exe start`。完整命令见[英文安装指南](../getting-started.md)。

## HTTP Proxy

```bash
proxybot start
```

设备 Wi-Fi HTTP 代理设为 `<电脑局域网 IP>:8088`，在设备上打开
`http://<电脑局域网 IP>:8090/` 安装并信任 CA，然后访问 `https://example.com`。

完成后关闭设备代理、Ctrl+C 停止，并移除测试 CA。

Gateway Beta 仅支持 macOS 且需要 sing-box，详见英文 [Gateway Beta](../gateway-beta.md)。
