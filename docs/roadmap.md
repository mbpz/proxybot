# ProxyBot Roadmap

## 1. Hero Section

**ProxyBot — macOS HTTPS MITM proxy for developers**

ProxyBot captures and decrypts all HTTPS/WSS traffic from your phone via macOS pf transparent proxy. Set your phone's gateway and DNS to your Mac's IP, install the CA certificate once, and watch every request flow through — classified by app (WeChat, Douyin, Alipay) and domain.

**Demo concept:** Phone on the left, Mac running the TUI on the right. Traffic appears in real-time as you use apps on the phone.

---

## 2. What's Shipped (v0.4.x)

Nine functional tabs, all accessible from the keyboard-driven TUI:

### Traffic

Real-time request list with method/host/status/app filter. Regex search. 60/40 split between request list and detail panel. pf/DNS toggle controls.

**Shortcut:** `p` toggle pf, `n` toggle DNS, `/` focus search, `x` clear filters, `Enter` load detail

### Rules

Five action types: **Direct** (bypass proxy), **Proxy** (forward to upstream), **Reject** (drop connection), **MapRemote** (forward to custom remote), **MapLocal** (serve from local file/mock). Hot-reload on file change. Rule table with inline modal editor.

**Shortcut:** `a` add, `e` edit, `d` delete, `s` save

### Devices

Per-device table showing MAC address, last seen, and bytes up/down. Per-device rule override — enter edit mode on any device to assign a custom rule action. WeChat/Douyin/Alipay classification badge per device.

**Shortcut:** `e` edit rule override, `Enter` confirm, `Esc` cancel

### Certs

One-click CA certificate export to `~/.proxybot/ca.crt`. Shows fingerprint, expiry, and serial number. Regenerate CA with fresh key pair. AirDrop or email the certificate to your phone.

**Shortcut:** `r` regenerate CA, `e` export PEM

### DNS

Upstream resolver selector: plain UDP or DoH (DNS-over-HTTPS). Blocklist toggle. Hosts file entries with lock. Live query log showing recent lookups with response latency.

**Shortcut:** `s` toggle DNS server, `b` toggle blocklist, `u` cycle upstream

### Alerts

SEV1 (critical), SEV2 (warning), SEV3 (info) anomaly detection with baseline profiling. Alert table with source, description, severity badge. ACK/clear controls. Baseline stats show normal traffic patterns.

**Shortcut:** `a` acknowledge, `c` clear acknowledged

### Replay

Replay targets table with start/stop controls. HAR export of captured traffic. Diff view comparing replayed response against original — highlights header and body differences.

**Shortcut:** `s` start replay, `x` stop replay, `e` export HAR, `d` show diff

### Graph

ASCII DAG visualization of request dependency graph (domains, timing, status codes). Auth state machine detection — identifies login sequences and session token flows. Toggle between DAG and auth views.

**Shortcut:** `g` DAG view, `a` auth state machine, `r` refresh

### Gen

Mock API generation from captured traffic. Frontend scaffold generator (React + TypeScript boilerplate). Docker bundle generator — packages mock backend with Dockerfile and docker-compose. Open output folder directly.

**Shortcut:** `m` generate mock API, `f` generate frontend scaffold, `d` generate Docker, `o` open output

---

## 3. Competitive Comparison

| | ProxyBot | mitmproxy | Proxyman | HTTP Toolkit |
|--|--|--|--|--|
| TUI | macOS-native ratatui | NCurses/mitmweb | Mac GUI only | CLI + web UI |
| App classification | WeChat/Douyin/Alipay | — | — | — |
| pf transparent proxy | macOS pf integration | Manual proxy config | Mac GUI proxy | — |
| Breakpoint | In Progress | Full | Full | Full |
| Auto CA install | Manual export | Manual | One-click | One-click |
| Tauri GUI | Planned | — | Mac GUI | — |

**ProxyBot's edge:** pf transparent proxy means no per-app proxy configuration on the phone. App classification groups traffic automatically. TUI is first-class on macOS.

---

## 4. Roadmap (Milestones)

| Version | Focus | Features |
|---------|-------|----------|
| **v0.4.x (NOW)** | TUI complete | All 9 tabs shipped, pf + DNS, basic breakpoint intercept |
| **v0.5.0** | Breakpoint Editing | Full TUI breakpoint UI — pause, edit request/response, continue. Android adb reverse support |
| **v0.6.0** | Tauri GUI Alpha | React UI for traffic + rules + devices, system CA install integration |
| **v1.0.0** | Phase 2 Complete | Full GUI with traffic editor, iOS VPN API via NEPacketTunnel, WebView debugging |

---

## 5. Installation

```bash
brew install mbpz/proxybot/proxybot-tui
```

Then connect your phone to the same WiFi network as your Mac:

1. **Set gateway:** WiFi settings > Configure Proxy > Manual — set Server to your Mac IP, Port to `8088`
2. **Set DNS:** WiFi settings > Configure DNS — set to your Mac IP
3. **Install CA:** Export cert from Certs tab, AirDrop to phone, enable full trust in Settings > General > About > Certificate Trust Settings
4. **Start capturing:** Run `proxybot-tui`, press `r` to start the proxy

Find your Mac IP with: `ipconfig getifaddr en0`

---

## 6. Architecture

```
Phone --[WiFi]--> Mac (pf redirect :80/:443) --> ProxyBot (MITM) --> Internet
                        |
                        +--> DNS Server (log queries, correlate with apps)
```

- **pf** redirects all port 80/443 traffic from the phone to ProxyBot's local proxy port
- **MITM** terminates TLS with dynamically-generated leaf certs signed by the root CA
- **DNS server** logs queries from the phone, correlated with subsequent connections for app classification
- **Classification engine** maps domains to apps: WeChat (`*.weixin.qq.com`, `*.wechat.com`), Douyin (`*.douyin.com`, `*.tiktokv.com`), Alipay (`*.alipay.com`, `*.alipayusercontent.com`)
