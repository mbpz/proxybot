# ProxyBot

轻量级 macOS MITM 流量调试工具，由一个 Rust CLI 和本地 Web UI 组成。

```bash
curl -fsSL https://github.com/mbpz/proxybot/releases/latest/download/install.sh | sh
proxybot start
```

- HTTP Proxy 是默认支持模式，无需管理员权限；
- Gateway Beta 需要 root 和用户自行安装的 sing-box；
- UDP/QUIC/DNS 当前不解密；
- 详细说明请优先参考英文页面，中文页面仍在补充中。

[快速开始](getting-started.md){ .md-button .md-button--primary }
[Gateway Beta](../gateway-beta.md){ .md-button }
[当前状态](../status.md){ .md-button }
