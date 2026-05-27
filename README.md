# ProxyBot

**A native macOS desktop HTTPS MITM proxy for developers.**

Phone and PC on the same LAN — set your phone's gateway and DNS to your Mac's IP, install the CA certificate once, and watch every request flow through — classified by app (WeChat, Douyin, Alipay, AI providers) and domain.

> **Unique in the market**: ProxyBot is the only MITM proxy combining **pf transparent proxy** (zero-config on phone) + **built-in DNS server** (app correlation) + **app classification** (WeChat/Douyin/Alipay/AI) + **Native Tauri desktop** (15MB vs 200MB Electron). After analyzing 25+ competitors, no other tool has this combination.

## Features

- **Native macOS GUI** — React/shadcn desktop app, ~15MB memory footprint
- **Transparent HTTPS/WSS interception** with MITM SSL
- **App classification** by DNS correlation + domain rules (WeChat, Douyin, Alipay)
- **Built-in DNS server** to log phone's DNS queries
- **macOS pf integration** for transparent proxy routing
- **MCP Server** — stdio transport for Claude Desktop integration

## Installation

One-command install via Homebrew — no manual download, no gatekeeper dialog:

```bash
# Desktop GUI (Tauri React app) — goes straight to /Applications
brew install --cask mbpz/tap/proxybot
```

> **No gatekeeper popup**: The Cask installs the `.app` directly to `/Applications` without triggering "software was blocked" dialogs. Quarantine xattr is cleared automatically.

```bash
git clone https://github.com/mbpz/proxybot.git
cd proxybot/src-tauri
cargo build --release --bin proxybot
./target/release/proxybot
```

## Architecture

The proxy core and GUI share the same Rust codebase:

```
proxy.rs         # MITM proxy — hyper + rustls + tokio
dns.rs           # DNS server — logging, forwarding, blocklist
db.rs            # SQLite — request history, device registry
rules.rs         # YAML rules — hot-reload, action dispatch
classifier/      # App classification — domain + TLS fingerprint
gui/             # React/shadcn desktop UI
plugin/          # Plugin system — hook dispatch, rule engine
ai_pipeline/     # AI analysis — NoiseFilter + ApiAnalyzer
mcp/             # MCP Server — stdio transport for Claude Desktop
```

**Key design**: The proxy is the hub. `ProxyContext` carries all subsystems (cert_manager, dns_state, db_state, rules_engine, plugins). The GUI consumes a broadcast channel for real-time updates.

## FAQ

### WeChat shows "Certificate Error" or connections fail

WeChat uses certificate pinning and may reject the intercepted certificate. This is expected behavior for apps with strong SSL pinning. The traffic will still be logged at the DNS and SNI level for classification purposes, but full HTTPS content inspection may not work for these apps.

### The proxy starts but I see no traffic

1. Verify your iPhone is using your Mac as the proxy and DNS server
2. Check that both devices are on the same LAN
3. Try opening `http://example.com` in Safari on the iPhone (not HTTPS)
4. Check ProxyBot's DNS tab to see if DNS queries are being received

### How does ProxyBot classify traffic?

ProxyBot uses a multi-stage classification pipeline:
1. **DNS correlation** — When your phone makes a DNS query, ProxyBot logs it. The subsequent connection is tagged with the app that made the DNS request.
2. **SNI inspection** — The TLS ClientHello message contains the requested domain (SNI), which ProxyBot extracts before encryption.
3. **Domain rules** — Known app domains are mapped: WeChat (`*.weixin.qq.com`, `*.wechat.com`), Douyin (`*.douyin.com`, `*.tiktokv.com`), Alipay (`*.alipay.com`).

### Does ProxyBot work on Windows?

Not yet. Windows support is planned for Phase 2.

### How do I uninstall?

1. Stop the proxy in ProxyBot
2. Remove the app: `rm -rf /Applications/ProxyBot.app`
3. Optionally remove the CA from your iPhone: Settings > General > About > Certificate Trust Settings
4. Optionally remove config data: `rm -rf ~/Library/Application\ Support/com.proxybot.app/`

## Competitive Positioning

ProxyBot is the **only** open-source proxy that combines pf transparent proxy + built-in DNS server + app classification + native Tauri GUI. After 4 rounds of competitive research covering ~25 projects, the moat is clear:

| Capability | ProxyBot | mitmproxy | Proxyman | proxelar | anything-analyzer |
|------------|:--------:|:---------:|:--------:|:--------:|:-----------------:|
| pf transparent proxy | ✅ | — | — | — | — |
| Built-in DNS + DoH | ✅ | — | — | — | — |
| App classification | ✅ | — | — | — | — |
| Native Tauri GUI | ✅ | — | — | — | — |
| Rust core | ✅ | — | — | ✅ | — |
| MCP Server | ✅ | — | — | — | ✅ |
| AI analysis | Gen tab | — | — | — | ✅★★★★★ |

**Closest competitor**: [proxelar](https://github.com/emanuele-em/proxelar) (Rust + ratatui + Lua) — shares the Rust+TUI stack but lacks transparent proxy and app classification.
**AI threat**: [anything-analyzer](https://github.com/Mouseww/anything-analyzer) — AI-first approach with MCP Server, but Electron-based and lacks transparent proxy.

See [full competitive analysis](docs/sdd/competitors-analysis.md) for details.

## Roadmap

### v0.4.x ✅
- TUI 9-tab system, pf transparent proxy, DNS server
- App classification (WeChat/Douyin/Alipay)
- Breakpoint interception (basic)

### v0.5.0 ✅
- Breakpoint editing (request/response edit before send)
- Android adb reverse capture

### v0.6.0 ✅
- Tauri GUI Alpha (React + Rust core)

### v0.7.0 ✅
- Rules engine integrated: MapRemote/MapLocal/Respond
- `apply_request_rule()` sync rule engine with hot-reload

### v0.8.0 ✅
- Tauri GUI complete: Rules editor, Devices management, Certs UI
- Full parity with TUI features

### v0.9.0 ✅
- Filter DSL (AND/OR/NOT/glob), WebSocket frame viewer, Replay engine, DAG visualization

### v0.10.0 ✅
- Code export (cURL/fetch/Python/Go), Request Composer, Syntax highlighting, Client setup wizard

### v1.0.0 ✅
- Plugin system v2.0, Network conditions, Team workspace, Rhai scripting, gRPC/Protobuf decoder, iOS VPN

### v1.1.0 ✅
- GraphQL decoder, Prometheus metrics, LLM token tracking, Web dashboard

### v1.2.0 ✅
- **MCP Server** — stdio transport + 5 tools (capture_traffic, classify_request, apply_rule, get_devices, get_alerts)
- **AI two-phase analysis** — NoiseFilter + ApiAnalyzer + Cost estimation
- **Column-scoped filter DSL** — `method:POST host:api` + text search
- **QR code CA** — SVG QR code generation for mobile CA install

### v1.3.0 ✅ (部分完成)
- ✅ **proxybot-core** standalone crate — Library-first packaging
- ✅ **Project file management** — Save/restore capture sessions (workspace module)
- ❌ **Mobile web dashboard** — Lightweight mitmweb-style remote access

### v2.0.0 (部分完成)
- ❌ Windows support (WFP transparent proxy)
- ❌ HTTP/3 & QUIC research/prototype
- ✅ **Transport-layer TCP/UDP proxy** — Protocol detection, SNI extraction, pass-through

## Development

### Running Tests

```bash
cd src-tauri
cargo test                 # 220+ unit + integration tests
cargo build --release --bin proxybot
```

### GUI Architecture

The Tauri GUI uses a state-driven architecture:

- `proxybot-gui.rs` holds all subsystem handles (`db_state`, `cert_manager`, `rules_engine`, etc.) as `Arc`
- React components consume state via Tauri IPC invoke calls
- Real-time updates via Tauri event system
- The main loop handles Tauri app lifecycle

### Key Design Patterns

- **Split pane**: Traffic tab uses 60/40 split between request list and detail panel
- **Hot reload**: Rules engine watches rule files and reloads automatically
- **Async event channel**: Broadcast channel for real-time request updates
- **MCP stdio mode**: `proxybot --mcp-stdio` for headless Claude Desktop integration
