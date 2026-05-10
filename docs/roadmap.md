# ProxyBot Roadmap

## 1. Hero Section

**ProxyBot — macOS HTTPS MITM proxy for developers**

ProxyBot captures and decrypts all HTTPS/WSS traffic from your phone via macOS pf transparent proxy. Set your phone's gateway and DNS to your Mac's IP, install the CA certificate once, and watch every request flow through — classified by app (WeChat, Douyin, Alipay) and domain.

**Demo concept:** Phone on the left, Mac running the TUI on the right. Traffic appears in real-time as you use apps on the phone.

---

## 2. What's Shipped (v0.10.x)

### TUI (v0.4.x+): Nine keyboard-driven tabs

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

### Tauri GUI (v0.6.0+): React + shadcn/ui desktop app

**Traffic Page** — Virtual-scrolled request table (TanStack Virtual), 60/40 split with detail panel. Filter bar (method/host/search). App classification badges.
**Composer** — 40/60 split edit-and-send view. Method/URL/Headers/Body form with live response preview.
**Replay** — Targets table with enable/disable, ReplayModal editor, batch execution with reqwest engine.
**WS Frame Viewer** — WebSocket frame list with direction arrows, opcode names, Text/Hex toggle, HexDump view.
**Graph** — WaterfallChart (recharts), DependencyGraph (vis-network), AuthStateMachine (mermaid).
**Filter DSL** — Lexer+Parser+Evaluator. Syntax: `method:GET AND status:2* OR host:*example.com`.
**Code Export** — Copy as cURL, fetch(), Python requests, Go http.
**Client Setup** — Detect installed browsers/Node.js/Python, copy proxy config commands.
**App Classification** — TLS fingerprint + SNI pattern matching (TikTok, WeChat, Douyin, Alipay, Amazon, Apple).

---

## 3. Competitive Comparison

| | ProxyBot | mitmproxy | Proxyman | HTTP Toolkit |
|--|--|--|--|--|
| TUI | macOS-native ratatui | NCurses/mitmweb | Mac GUI only | CLI + web UI |
| App classification | WeChat/Douyin/Alipay | — | — | — |
| pf transparent proxy | macOS pf integration | Manual proxy config | Mac GUI proxy | — |
| Breakpoint | ✅ Full | Full | Full | Full |
| Auto CA install | Wizard guide | Manual | One-click | One-click |
| Tauri GUI | ✅ v0.6+ | — | Mac GUI | — |
| System tray | ✅ With notifications | — | ✅ | — |
| ADB reverse tunnel | ✅ Android USB | — | — | — |
| Code export | ✅ v0.10 | ✅ | ✅ | ✅ |
| Syntax highlighting | ✅ v0.10 | — | ✅ | ✅ |
| Client setup | ✅ v0.10 | ✅ | ✅ | ✅ |

**ProxyBot's edge:** pf transparent proxy means no per-app proxy configuration on the phone. App classification groups traffic automatically. TUI is first-class on macOS.

---

## 4. Roadmap (Milestones)

| Version | Focus | Features |
|---------|-------|----------|
| **v0.4.x (DONE)** | TUI complete | All 9 tabs shipped, pf + DNS, basic breakpoint intercept |
| **v0.5.0 (DONE)** | Breakpoint Editing | Full TUI breakpoint UI — pause, edit request/response, continue. Android adb reverse support via USB |
| **v0.6.0 (DONE)** | Tauri GUI Alpha | React UI traffic panel, proxybot-gui binary, CA wizard, system tray with notifications |
| **v0.7.0 (DONE)** | Rules Engine | MapRemote/MapLocal/Respond rules integrated into handle_http pipeline. apply_request_rule() sync rule engine with hot-reload |
| **v0.8.0 (DONE)** | Tauri GUI Complete | Full GUI: Rules editor, Devices management, Certs UI, complete parity with TUI |
| **v0.9.0 (DONE)** | Advanced Features | Filter DSL (AND/OR/NOT/glob), WS frame viewer (text/hex), Replay engine (reqwest), TLS fingerprint classifier (6 apps), Dependency graph (DAG/waterfall/auth), Traffic list (virtual scroll) |
| **v0.10.0 (DONE)** | Quick Wins | Code export (cURL/fetch/Python/Go ✅), Request Composer (split-view ✅), Syntax highlighting (highlight.js ✅), Client setup wizard (detect browsers ✅) |
| **v1.0.0 (DONE)** | Phase 2 Complete | Plugin system v2.0 ✅, Network conditions ✅, Team workspace ✅, Rhai scripting ✅, gRPC/Protobuf decoder ✅, iOS VPN ✅ |
| **v1.1.0 (DONE)** | Phase 3 Start | GraphQL decoder ✅, Prometheus metrics ✅, LLM token tracking ✅, Web dashboard ✅, HTTP/3 research ✅ |
| **v1.2.0 (IN PROGRESS)** | AI + MCP | MCP Server (P0 ✅ implemented: stdio transport + JSON-RPC server + 5 tools), AI two-phase analysis (P1 ✅planned), Column-scoped filter DSL (P1 ✅planned), QR code CA distribution (P2 ✅planned) |
| **v1.3.0** | Architecture | proxybot-core standalone crate (library-first), Project file management (save/restore sessions), Mobile web dashboard (lightweight mitmweb-style) |
| **v2.0.0** | Platform | Windows support (WFP transparent proxy), HTTP/3 & QUIC research/prototype, Transport-layer TCP/UDP proxy |

### v1.2.0 Implementation Plans

| Feature | Plan | Spec | Status |
|---------|------|------|--------|
| **MCP Server** (P0) | `plans/2026-05-10-mcp-server.md` | `specs/2026-05-10-mcp-server-design.md` | ✅ Implemented (src-tauri/src/mcp/) |
| **AI Two-Phase Analysis** (P1) | `plans/2026-05-10-ai-two-phase-analysis.md` | — | ✅ Planned |
| **Column-Scoped Filter DSL** (P1) | `plans/2026-05-10-column-filter-dsl.md` | `specs/2026-05-09-advanced-filter-dsl.md` | ✅ Planned |
| **QR Code CA Distribution** (P2) | `plans/2026-05-10-qr-ca-distribution.md` (in specs) | `specs/2026-05-10-qr-ca-distribution.md` | ✅ Planned |

### MCP Server Implementation Details (v1.2.0 P0)

**Files created:**
- `src-tauri/src/mcp/mod.rs` — JSON-RPC 2.0 types, McpState struct, re-exports
- `src-tauri/src/mcp/server.rs` — McpServer with 5 tool handlers (capture_traffic, classify_request, apply_rule, get_devices, get_alerts)
- `src-tauri/src/mcp/transport.rs` — stdio transport with `start_stdio_mode()`
- `src-tauri/src/mcp/protocol/mod.rs` — Protocol re-exports
- `src-tauri/src/db.rs` — Added `new_in_memory()` for CLI mode

**Tools exposed:**
| Tool | Description | Status |
|------|-------------|--------|
| `capture_traffic` | Get recent HTTP/HTTPS requests with limit/filter/since | ✅ |
| `classify_request` | Classify host/SNI to identify app (WeChat/Douyin/Alipay/AI) | ✅ |
| `apply_rule` | Apply allow/block/log rule to a request | ✅ |
| `get_devices` | List all connected devices | ✅ |
| `get_alerts` | Get security/anomaly alerts | ✅ |

**Usage:** `cargo run --bin proxybot-tui -- mcp-stdio` (once binary entry point added)

## 3.1 Competitive Deep-Dive (May 2026)

Researched 6 comparable projects for architecture, interaction, and product insights.

| Project | Stars | Lang | UI | Key Differentiator |
|---------|-------|------|----|--------------------|
| [mitmproxy](https://github.com/mitmproxy/mitmproxy) | 43.5k | Python | TUI+Web | Industry standard, addon system, Python scripting |
| [whistle](https://github.com/avwo/whistle) | 15.5k | Node.js | Web UI | Plugin ecosystem, Weinre remote debugging, Composer |
| [bandwhich](https://github.com/imsnif/bandwhich) | 11.7k | Rust | TUI | Process-level bandwidth attribution |
| [Proxyman](https://github.com/ProxymanApp/Proxyman) | 6.8k | Swift | macOS native | SwiftNIO performance, iOS app, team workspace |
| [HTTP Toolkit](https://github.com/httptoolkit/httptoolkit) | 3.5k | TS+React | Electron | Modular (mockttp lib), one-click client setup |
| [proxy.py](https://github.com/abhinavsingh/proxy.py) | 3.5k | Python | CLI | Zero-dependency, plugin framework, GROUT tunnel |

### Feature Gap Analysis

| Feature | ProxyBot | mitmproxy | Proxyman | HTTP Toolkit | whistle | proxy.py |
|---------|----------|-----------|----------|--------------|---------|----------|
| Transparent proxy (pf) | ✅ | — | — | — | — | — |
| App classification | ✅ | — | — | — | — | — |
| TUI | ✅ ratatui | ✅ NCurses | — | — | — | — |
| Tauri GUI | ✅ | — | — | — | — | — |
| ADB tunnel | ✅ | — | — | — | — | — |
| Plugin system | ✅ v2.0 | ✅ addons | — | — | ✅ plugins | ✅ plugins |
| Scripting hooks | ✅ Rhai v1.0 | ✅ Python | ✅ Scripting | — | ✅ rules | ✅ plugins |
| Request composer | ✅ v0.10 | — | ✅ Compose | ✅ Send | ✅ Composer | — |
| Diff tool | — | — | ✅ | — | — | — |
| DNS spoofing | — | — | ✅ | — | — | ✅ |
| Network throttling | ✅ v1.0 | — | ✅ | ✅ | — | — |
| Protobuf/gRPC | ✅ v1.0 | ✅ | ✅ | — | — | — |
| GraphQL decoder | ✅ v1.1 | — | ✅ | — | — | — |
| Prometheus metrics | ✅ v1.1 | — | — | — | — | — |
| Team workspace | ✅ v1.0 | — | ✅ | — | — | — |
| gRPC-web/WebSocket frame | ✅ WS | ✅ | ✅ | ✅ | ✅ | — |
| HAR export | ✅ | ✅ | ✅ | ✅ | ✅ | — |
| Mock API generation | ✅ Gen tab | — | Map Local | ✅ mockttp | ✅ | ✅ |
| Remote debugging | — | — | — | — | ✅ Weinre | — |
| Tunnel (ngrok alt) | — | — | — | — | — | ✅ GROUT |
| API diff / regression | — | — | — | — | — | — |
| Code export (cURL/fetch) | ✅ v0.10 | ✅ | ✅ | ✅ | ✅ | — |
| One-click client setup | ✅ v0.10 | ✅ | ✅ | ✅ | — | — |
| iOS standalone app | — | — | ✅ | — | — | — |
| Syntax highlighting | ✅ v0.10 | — | ✅ | ✅ | ✅ | — |

### Five-Dimension Analysis

#### 1. 技术架构 (Tech Stack)

| Stack Type | Tools | ProxyBot Position |
|------------|-------|------------------|
| Python (async) | mitmproxy, proxy.py | v1.0 uses Rust async (tokio) — superior I/O performance |
| Node.js | whistle, anyproxy | v1.0 Rust — no Node.js dependency, lower memory |
| Go | Hetty, hyperfox, forwarder | Comparable performance; ProxyBot has TUI advantage |
| Rust | proxelar, bandwhich, int3rceptor | **Only** Rust + Tauri + MITM combination |
| Electron | HTTP Toolkit, anything-analyzer | ProxyBot (Tauri) has 10x lower memory footprint |
| Swift native | Proxyman, Rockxy | ProxyBot Tauri is cross-platform |

**Key finding: ProxyBot is the ONLY Tauri-based MITM proxy tool.** Clash Verge (116k stars) proves Tauri works for proxy/network tools but they are VPN routing tools — none do traffic interception/decryption. ProxyBot is first-mover in this space.

#### 2. 原理 (Mechanism)

| Mechanism | How It Works | ProxyBot |
|-----------|--------------|----------|
| **pf transparent proxy** | macOS pf redirects :80/:443 to local proxy | ✅ Unique — no manual proxy config on phone |
| **MITM TLS termination** | Dynamic leaf certs signed by root CA | ✅ |
| **DNS correlation** | Log DNS queries → correlate with connections → identify app | ✅ Unique — no competitor does this |
| **SNI inspection** | TLS ClientHello domain extraction | ✅ |
| **CDP capture** | Chrome DevTools Protocol browser interception | anything-analyzer (not ProxyBot) |
| **Transport-layer** | TCP/UDP/DTLS interception (not just HTTP) | InterceptSuite only |

**Mechanism Insight**: ProxyBot's pf transparent proxy + DNS correlation is a two-layer capture system no competitor has.

#### 3. 实现方案 (Implementation)

| Implementation | mitmproxy | Proxyman | HTTP Toolkit | ProxyBot |
|---------------|-----------|----------|--------------|----------|
| Proxy core | Python asyncio | SwiftNIO | Node.js mockttp | Rust tokio+hyper+rustls |
| GUI framework | mitmweb (React) | AppKit | Electron | **Tauri v2 + React** |
| TUI framework | NCurses | — | — | ratatui |
| Script engine | Python | Scripting | — | Rhai |
| Database | SQLite | SQLite | better-sqlite3 | SQLite |
| Certificate | rcgen | Security framework | node-forge | rcgen |

**Implementation Insight**: ProxyBot is the only MITM tool with dual interfaces (TUI + Tauri GUI). Tauri provides native desktop experience without Electron overhead.

#### 4. 交互 (UX/Interaction)

| UX Pattern | Leader | ProxyBot Status |
|-------------|--------|-----------------|
| **Three-panel layout** (list/overview/detail) | HTTP Toolkit | v1.0 has 60/40 split — needs three-panel upgrade |
| **Color-coded methods** (GET=green, POST=blue) | HTTP Toolkit | Implemented in TUI |
| **One-click CA install** | Proxyman, HTTP Toolkit | Basic wizard — needs improvement |
| **QR code CA distribution** | proxelar, hyperfox | Not implemented |
| **Column-scoped filter** | proxelar (`method:POST`) | RegEx search — proxelar wins |
| **Keyboard-driven TUI** | ProxyBot (ratatui) | Best-in-class |
| **Virtual scroll large lists** | HTTP Toolkit, Proxyman | TanStack Virtual — implemented |

**UX Gap**: proxelar's `column:value` filter syntax is more intuitive than ProxyBot's regex. HTTP Toolkit's three-panel layout is the reference design.

#### 5. 产品 (Positioning)

| Positioning | Leader | ProxyBot |
|-------------|--------|----------|
| Target platform | mitmproxy (cross-platform) | macOS-only (Phase 1) |
| Target user | Security researcher | Developer (mobile debugging) |
| Primary differentiator | Addon ecosystem | App classification + pf transparent |
| Pricing | Free (most) | MIT open source |
| Enterprise features | Burp Suite ($449/yr) | Team workspace ✅ |
| AI integration | anything-analyzer | Gen tab (LLM inference) |

**Product Insight**: ProxyBot's positioning is "developer tool for mobile traffic debugging with app intelligence." This is unique — no competitor combines transparent proxy + app classification + mobile focus.

---

## 3.2 Competitive Insights from Deep-Dive

### From anything-analyzer (2,366 stars) — AI + MCP Architecture
- **MCP Server integration** — exposes proxy as MCP tools for AI agents (Claude Desktop, Cursor)
- **AI two-phase pipeline** — Phase 1 filters noise → Phase 2 deep analysis
- **CDP + MITM dual capture** — browser interception unified with proxy interception
- **ProxyBot opportunity**: P0 priority — implement MCP Server to expose capture/classify/rules as tools

### From proxelar (966 stars) — Rust TUI + Column Filter
- **Column-scoped DSL** — `method:POST host:api status:200` is more intuitive than regex
- **Three interfaces** — terminal/TUI/Web GUI via single binary
- **Lua scripting** — more mature ecosystem than Rhai
- **ProxyBot opportunity**: Integrate column:value syntax into Filter DSL

### From whistle (15.5k stars) — Plugin Architecture
- **Hook chain model**: `onRequest → onResponse → onConnect → onServer → onSocket → onError`
- **Rules-based routing**: `pattern pluginName` declarative dispatch
- **Hot-reload**: Rules file changes trigger automatic reload
- **ProxyBot opportunity**: Implement whistle-style hook priorities + rules-based plugin dispatch

### From HTTP Toolkit (3.5k stars) — UI/UX Excellence
- **Three-panel layout**: Request list | Overview | Details (tabs: Headers/Body/Timing)
- **Color coding**: GET=green, POST=blue, DELETE=red; 2xx=green, 4xx=orange, 5xx=red
- **CA wizard**: Platform detection → Visual guide → Verification → Confirmation
- **ADB integration**: `adb reverse` + QR-coded WiFi proxy setup
- **ProxyBot opportunity**: Adopt three-panel GUI, enhance CA wizard, improve ADB UX

### From hyperfox (1,631 stars) — QR Code CA Distribution
- **QR code CA install** — 手机扫码安装CA，降低移动端配置门槛
- **移动端 Web Dashboard** — 适配手机屏幕的轻量界面
- **ProxyBot opportunity**: P2 实现 QR code + 内置 HTTP 下载 CA

### From Proxyman Atlantis — iOS VPN Simplification
- **Minimal approach**: Don't do on-device MITM, just forward packets to Mac's existing MITM
- **Protocol**: Raw IP over TCP (simplified) with length-prefixed framing
- **Already has**: `ios/PacketTunnel/PacketTunnelProvider.swift` skeleton with entitlements
- **ProxyBot opportunity**: TCP bridge MVP (2-3 weeks), skip on-device TLS termination

### Competitive Gap Matrix (Updated)

| Feature | ProxyBot | whistle | HTTP Toolkit | Proxyman | anything-analyzer | Gap Priority |
|---------|----------|---------|--------------|----------|-------------------|--------------|
| MCP Server | ❌ | ❌ | ❌ | ❌ | ✅ | **P0** (新增) |
| AI two-phase analysis | ❌ | ❌ | ❌ | ❌ | ✅ | P1 (新增) |
| Plugin system | ✅ v2.0 | Full | — | — | — | ✅ DONE |
| CA wizard | Basic | — | Full | One-click | — | P2 |
| Column-scoped filter | Regex | — | — | ✅ | — | P1 |
| QR code CA | ❌ | — | — | — | — | P2 (新增) |
| ADB integration | USB | — | Full | — | — | P2 |
| iOS VPN | ✅ v1.0 | — | — | ✅ Atlantis | — | ✅ DONE |
| Network conditions | ✅ v1.0 | — | ✅ | ✅ | — | ✅ DONE |
| Team workspace | ✅ v1.0 | — | ✅ | — | — | ✅ DONE |
| WebView debugging | CDP stub | ✅ | — | ✅ | ✅ | P3 |
| CDP browser capture | ❌ | — | — | — | ✅ | P3 |
| HTTP/3 QUIC | ❌ | ❌ | ❌ | ❌ | ❌ | P3 |

**ProxyBot's moat remains intact**: App classification (WeChat/Douyin/Alipay/AI services), pf transparent proxy, Rust TUI+GUI dual interface. These are unique to ProxyBot.

---

## 3.3 Emerging Competitors Round 3 (May 2026)

Discovered 8 new projects filling gaps not covered by earlier research:

| Project | Stars | Key Innovation |
|---------|-------|---------------|
| [TokenTap](https://github.com/jmuncor/tokentap) | 797 | LLM API traffic interceptor, token-aware, context window tracking |
| [Rockxy](https://github.com/RockxyApp/Rockxy) | 404 | Native macOS Swift proxy, GraphQL introspection, Charles alternative |
| [KtorMonitor](https://github.com/CosminMihuMDC/KtorMonitor) | 217 | SDK-level interceptor, Compose Multiplatform (not network proxy) |
| [httpmon](https://github.com/kostyay/httpmon) | 80 | Go Bubble Tea TUI, .proto gRPC decoding, JS scripting hooks |
| [int3rceptor](https://github.com/S1b-Team/int3rceptor) | 4 | Rust+Vue.js hybrid, pentesting-first (Burp alternative) |
| [intercept](https://github.com/mrceha/intercept) | — | Go single-binary, web dashboard, zero-dependency |
| [mitmproxy-rs](https://github.com/josexy/mitmproxy-rs) | — | Library-first MITM for Rust, embeddable |
| [go-traffic-proxy-analyzer](https://github.com/tahsinmert/go-traffic-proxy-analyzer) | — | Built-in Prometheus metrics + alerting |

### New Strategic Opportunities

| Opportunity | Rationale | Gap |
|-------------|-----------|-----|
| **GraphQL decoder** | Rockxy has it; no open-source tool decodes GraphQL-WS subscription traffic | P1 |
| **Prometheus metrics** | go-traffic-proxy-analyzer pattern; makes ProxyBot CI/CD-friendly | P1 |
| **LLM token tracking** | TokenTap hit 797 stars in 4 months; ProxyBot already has AI signatures | P2 |
| **Web dashboard** | intercept pattern; complementary to TUI+Tauri GUI | P2 |
| **HTTP/3 & QUIC** | ZERO open-source proxies support it; major whitespace | P3 (research) |

---

## 3.4 Emerging Competitors Round 4 (May 2026)

Discovered 6 more high-value projects not covered in earlier rounds:

| Project | Stars | Key Innovation |
|---------|-------|---------------|
| [proxelar](https://github.com/emanuele-em/proxelar) | 966 | Rust+ratatui+Lua MITM, column-scoped filter DSL, TUI+CLI+Web three interfaces |
| [anything-analyzer](https://github.com/Mouseww/anything-analyzer) | 2,366 | AI-powered auto reverse-engineering, MCP Server for AI agents, CDP+MITM dual capture |
| [hyperfox](https://github.com/malfunkt/hyperfox) | 1,631 | QR code CA distribution, per-session SQLite DB, mobile-friendly web UI |
| [InterceptSuite](https://github.com/InterceptSuite/InterceptSuite) | 772 | TCP/UDP/DTLS/TLS transport-layer MITM, IoT/Thick Client focus, Python extensions |
| [forwarder](https://github.com/saucelabs/forwarder) | 280 | PAC auto-config, HTTP/2/WebSocket/SSE/TCP, production use at Sauce Labs |
| [gomitmproxy](https://github.com/AdguardTeam/gomitmproxy) | 344 | Library-first Go MITM by AdGuard, embeddable, custom cert storage |

### New Strategic Opportunities (Round 4)

| Opportunity | Rationale | Gap |
|-------------|-----------|-----|
| **MCP Server** | anything-analyzer proves proxy-as-AI-tool is high-demand; ProxyBot can expose capture/classify/rules as MCP tools | P0 |
| **AI two-phase analysis** | anything-analyzer's filter→deep-analysis pipeline improves Gen tab quality significantly | P1 |
| **Column-scoped filter DSL** | proxelar's `method:POST host:api` syntax is more intuitive than regex; integrate into existing Filter DSL | P1 |
| **QR code CA distribution** | hyperfox/proxelar both use QR codes for mobile CA install; ProxyBot still uses manual AirDrop | P2 |
| **proxybot-core crate** | gomitmproxy/mitmproxy-rs show library-first expands ecosystem; extract core proxy engine as standalone Rust crate | P2 |
| **Project file management** | InterceptSuite saves/restores capture sessions; ProxyBot could add workspace persistence | P2 |
| **HTTP/3 & QUIC** | Still zero open-source proxies support it; remains major whitespace opportunity | P3 |
| **Transport-layer proxy** | InterceptSuite proves demand for non-HTTP MITM (MQTT/CoAP/gaming/DB protocols) | P3 |

---

## 3.2 新兴竞争格局 (2026)

### AI 流量分类机会
- LLM API 调用爆发（OpenAI/Anthropic/Azure/Groq）
- AI 流量分类成为差异化功能（v1.0 已添加签名）
- Token 成本估算功能（v1.0 已添加 ai_stats）

### 移动端零配置趋势
- Proxyman Atlantis: iOS 无代理抓包
- HTTP Toolkit: Android adb 一键配置
- ProxyBot 机会: 简化移动端配置流程

### 竞品威胁
- **mitmproxy** 生态持续扩展，Flow 表达式更强大
- **Proxyman** 商业化加速，功能追赶
- **HTTP Toolkit** Electron 性能问题可能转向 Tauri

---

### Key Improvements for ProxyBot

**Priority 2 — Architecture Upgrades (v0.11.0)**

5. **Plugin system** — Rust trait-based plugin API
   - `ProxyPlugin` trait: on_request, on_response, on_connect, on_error hooks
   - Plugin discovery: scan `~/.proxybot/plugins/` for .wasm or .so files
   - Hot-reload: watch plugin directory for changes
   - Sandbox: WASM runtime for untrusted plugins
   - Files: `src-tauri/src/plugin/` (loader, registry, sandbox, wasm_runtime)
   - API surface: read/modify headers, read body, inject response, log

6. **Scripting hooks** — Rhai scripting engine
   - Rhai (Rust-native, no unsafe) for user-defined traffic transforms
   - Editor UI with syntax highlighting and live validation
   - Hook points: request received, response received, before forward
   - Script API: `request.headers`, `request.body`, `response.status`, `ctx.log()`
   - Files: `src-tauri/src/scripting/` (engine, api, sandbox), `src/components/scripts/ScriptEditor.tsx`

7. **gRPC/Protobuf support** — Decode protobuf bodies
   - Detect `content-type: application/grpc` and `application/x-protobuf`
   - Protobuf descriptor discovery: upload .proto or auto-detect via reflection
   - Decode protobuf to JSON display, show field names and types
   - gRPC-Web support (base64 + binary frames)
   - Files: `src-tauri/src/protobuf/` (decoder, descriptor, grpc_web)

8. **Network condition simulation** — Throttle + latency + packet loss
   - Presets: 3G (1.6Mbps/768kbps/300ms), 4G (20Mbps/10Mbps/100ms), Custom
   - Per-host or global throttle rules
   - Buffered delay with configurable jitter
   - Visual indicator in status bar when throttling active
   - Files: `src-tauri/src/network_conditions.rs`, `src/components/conditions/NetworkConditions.tsx`

**Priority 3 — Product Expansion (v1.0.0+)**

9. **Process-level attribution** — Which app sent this request
   - ADB: run `ps` on device, map PID→package name
   - iOS: use NEPacketTunnel flow metadata (iOS 15+)
   - Show app icon + name in traffic list (AppBadge component already exists)
   - Per-app traffic statistics: bytes, request count, latency distribution

10. **Team collaboration** — Share proxy configurations
    - Export/import: HAR, ProxyBot project file, CA certificate bundle
    - Rule sets: share MapRemote/MapLocal configurations as YAML/JSON
    - Mock configs: export generated mocks with Docker bundle

11. **iOS VPN mode** — On-device capture
    - NEPacketTunnel provider (Swift, packaged separately)
    - Local VPN → forward to local proxy → MITM on device
    - No Mac required for basic capture
    - Certificate install via MDM profile

---

## 4.1 Enhanced Detection (from rkn-block-checker analysis)

Inspired by [rkn-block-checker](https://github.com/MayersScott/rkn-block-checker) architecture for deep censorship detection:

### Multi-Layer Diagnosis System

```
check_url() progressive detection:
├── DNS layer: system DNS vs DoH comparison → detect DNS poisoning
├── TCP layer: port connectivity → detect IP blacklist/TCP reset
├── TLS layer: SNI handshake → detect TLS DPI (SNI filtering)
└── HTTP layer: status code + body signature → detect HTTP stub pages (451, ISP inject)
```

### Verdict + Confidence System

| Verdict | Description | Confidence Calibration |
|---------|-------------|------------------------|
| `OK` | Connection normal | HIGH if all layers pass |
| `DNS_BLOCK` | System DNS fails, DoH succeeds | HIGH - dual signal confirmation |
| `TCP_RESET` | TCP RST received | MEDIUM - known censorship pattern |
| `TLS_BLOCK` | TLS handshake dropped on ClientHello | MEDIUM - SNI-based DPI signature |
| `HTTP_STUB` | HTTP 451 or ISP stub page marker | HIGH - explicit signal |
| `TIMEOUT` | Connection timeout | LOW - ambiguous |
| `DOWN` | Generic failure | LOW - multiple possible causes |

### Stub Page Detection

```python
STUB_MARKERS = [...]  # ISP signature strings for body matching

def looks_like_stub(body_snippet: str) -> bool:
    return any(marker in body_snippet for marker in STUB_MARKERS)
```

### DNS Comparison (ProxyBot already has foundation)

```rust
// System DNS vs DoH对比检测DNS污染
sys_ips = resolve_system_all(host)
doh_ips = resolve_doh_all(host)

if sys_ips.is_disjoint(doh_ips) {
    → DNS mismatch detected (transparent DNS rewriting)
}
```

### Implementation Priority

1. **Stub page signatures** - Add ISP stub markers to response analyzer
2. **Confidence scoring** - Add Verdict + Confidence to anomaly alerts
3. **DNS对比检测** - Leverage existing DNS log for poisoning detection
4. **TLS DPI detection** - Detect握手被重置的审查模式
5. **流式诊断报告** - 分层展示诊断结果

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
