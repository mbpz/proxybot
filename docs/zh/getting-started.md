# 快速开始

## 安装

```bash
curl -fsSL https://github.com/mbpz/proxybot/releases/latest/download/install.sh | sh
proxybot --version
```

默认安装到 `~/.local/bin`。

## HTTP Proxy

```bash
proxybot start
```

设备 Wi-Fi HTTP 代理设为 `<Mac 局域网 IP>:8088`，在设备上打开
`http://<Mac IP>:8090/` 安装并信任 CA，然后访问 `https://example.com`。

完成后关闭设备代理、Ctrl+C 停止，并移除测试 CA。

Gateway Beta 需要 sing-box，详见英文 [Gateway Beta](../gateway-beta.md)。
