# Comparison

ProxyBot vs other popular MITM proxy tools — analyzed across five dimensions: **技术架构 (tech stack)**, **原理 (mechanism)**, **实现方案 (implementation)**, **交互 (UX/interaction)**, **产品 (positioning)**.

## Feature Comparison (20+ Tools)

| Feature | ProxyBot | mitmproxy | Proxyman | proxelar | anything-analyzer | TokenTap |
|---------|:--------:|:---------:|:--------:|:--------:|:-----------------:|:--------:|
| TUI | ratatui | mitmweb | — | ratatui | — | Textual |
| GUI | Tauri+React | mitmweb | macOS native | Web (axum) | Electron | — |
| pf Transparent Proxy | ✅ | — | — | — | — | — |
| App Classification | DNS correlation | — | — | — | — | — |
| DNS Server | Built-in DoH/UDP | — | — | — | — | — |
| Script Engine | Rhai | Python | Scripting | Lua | JS Hook | — |
| AI Analysis | Gen tab | — | — | — | 5-mode AI | Token tracking |
| MCP Server | — | — | — | — | ✅ | — |
| macOS Support | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Windows Support | — | ✅ | ✅ | ✅ | ✅ | ✅ |
| Linux Support | — | ✅ | ✅ | ✅ | ✅ | ✅ |
| Rule Engine | 5 actions | Flow | Map Local/Remote | Lua only | — | — |
| Breakpoint Debugging | ✅ | ✅ | ✅ | Lua | — | — |
| HAR Export | ✅ | ✅ | ✅ | — | — | — |
| Mock Generation | Gen tab | — | Map Local | Lua | — | — |
| Filter DSL | AND/OR/NOT | Flow | domain filter | column:value | Domain filter | — |
| Code Export | cURL/fetch/Python/Go | ✅ | ✅ | — | — | — |
| QR code CA | — | — | — | ✅ | — | — |
| CDP browser | — | — | — | — | ✅ | — |
| Column-scoped filter | — | — | — | ✅ | — | — |
| Prometheus metrics | ✅ v1.1 | — | — | — | — | — |
| GraphQL decoder | ✅ v1.1 | — | ✅ | — | — | — |
| LLM token tracking | ✅ (partial) | — | — | — | — | ✅ |
| HTTP/3 QUIC | — | — | — | — | — | — |
| iOS VPN | ✅ v1.0 | — | ✅ Atlantis | — | — | — |
| ADB tunnel | ✅ | — | — | — | — | — |
| Team workspace | ✅ v1.0 | — | ✅ | — | — | — |

---

## Five-Dimension Deep Dive

### 1. 技术架构 (Tech Stack)

ProxyBot is the **only Tauri-based MITM proxy tool** in the market.

| Stack | Representative Tools | Performance | Memory |
|-------|----------------------|-------------|--------|
| **Rust + Tauri** | ProxyBot (only one) | ★★★★★ | ★★★★★ |
| Rust + TUI | proxelar, bandwhich | ★★★★★ | ★★★★★ |
| Go | Hetty, hyperfox, httpmon | ★★★★ | ★★★★ |
| Python asyncio | mitmproxy, proxy.py | ★★★ | ★★★ |
| Node.js | whistle, anyproxy | ★★★ | ★★ |
| Swift native | Proxyman, Rockxy | ★★★★ | ★★★★ |
| Electron | HTTP Toolkit, anything-analyzer | ★★★ | ★★ |

**Why Tauri matters**: Electron apps use 200-500MB RAM at idle. ProxyBot (Tauri) uses 30-50MB. For a tool developers run constantly, this is significant.

**proxelar as closest competitor**: Rust + ratatui + Lua + MITM. But proxelar lacks pf transparent proxy, DNS server, and app classification. The tech stack is similar but the product direction diverges.

### 2. 原理 (Mechanism)

ProxyBot's dual-layer capture system is unique:

```
Layer 1: pf transparent proxy (phone → Mac, no proxy config needed)
    ↓
Layer 2: MITM TLS termination (dynamic leaf certs)
    ↓
Layer 3: DNS correlation (identify app from DNS log)
    ↓
Layer 4: SNI inspection (TLS ClientHello domain extraction)
    ↓
Layer 5: Domain rule library (WeChat/Douyin/Alipay/AI services)
```

No competitor combines all five layers. The closest is nDPI (packet-level DPI) but it doesn't do MITM.

| Mechanism | How it works | Competitor |
|-----------|--------------|------------|
| **pf transparent proxy** | macOS pf redirects :80/:443 | ProxyBot unique |
| **MITM + dynamic certs** | rcgen root CA + per-host leaf | All tools |
| **DNS correlation** | Log DNS → correlate → app label | ProxyBot unique |
| **SNI inspection** | TLS ClientHello domain | All HTTPS tools |
| **TLS fingerprint** | JA3/JArm hash matching | classifier module |
| **CDP dual capture** | Browser + MITM unified | anything-analyzer |
| **Transport-layer** | TCP/UDP/DTLS interception | InterceptSuite only |

### 3. 实现方案 (Implementation)

ProxyBot's Rust + Tauri dual-interface is unique:

| Component | ProxyBot | mitmproxy | Proxyman | HTTP Toolkit |
|-----------|----------|-----------|----------|--------------|
| **Proxy core** | tokio + hyper + rustls | asyncio + custom TLS | SwiftNIO | Node.js mockttp |
| **GUI** | Tauri v2 + React 19 | mitmweb (React) | AppKit | Electron |
| **TUI** | ratatui | mitmweb | — | — |
| **Script** | Rhai (Rust-native) | Python | Scripting | — |
| **Database** | SQLite (rusqlite) | SQLite | SQLite | better-sqlite3 |
| **Certificate** | rcgen | OpenSSL | Security.framework | node-forge |
| **WebSocket** | tokio-tungstenite | ws | URLSessionWebSocket | ws |
| **gRPC** | prost (v1.0) | grpcio | — | — |
| **Metrics** | Prometheus (v1.1) | — | — | — |

**Implementation advantage**: ProxyBot ships two binaries — `proxybot-tui` (terminal) and `proxybot-gui` (Tauri). No other MITM tool offers this dual-interface.

### 4. 交互 (UX/Interaction)

The three-panel layout (HTTP Toolkit's pattern) is the UX benchmark:

```
┌─────────────────────────────────────────────────────────────────────┐
│  Request List    │  Overview           │  Detail                    │
│  ─────────────  │  ───────────────     │  ──────────────────────    │
│  GET /api/user  │  Method: GET         │  [Headers] [Body] [Timing]  │
│  POST /api/msg  │  Host: api.example   │                            │
│  GET /api/msgs  │  Status: 200         │  Request Headers:           │
│  ...            │  Time: 124ms         │  Accept: application/json   │
│                 │  Size: 2.4KB        │  Authorization: Bearer ...  │
│                 │  App: WeChat         │                            │
│                 │                     │  Response Body:             │
│                 │                     │  { "user": { "id": 123 } }  │
└─────────────────────────────────────────────────────────────────────┘
```

ProxyBot currently has a 60/40 split (list + detail). HTTP Toolkit's three-panel is the reference design.

**UX Differentiation**:

| UX Feature | Leader | ProxyBot |
|------------|--------|----------|
| Keyboard-driven TUI | ProxyBot | Best-in-class (ratatui) |
| Column-scoped filter | proxelar (`method:POST`) | Regex search |
| QR code CA install | proxelar, hyperfox | Not implemented |
| One-click CA | Proxyman, HTTP Toolkit | Basic wizard |
| Three-panel layout | HTTP Toolkit | 60/40 split |
| Virtual scroll | HTTP Toolkit, Proxyman | TanStack Virtual ✅ |
| Color-coded methods | HTTP Toolkit | Implemented in TUI |

### 5. 产品 (Positioning)

ProxyBot's market position: **macOS developer tool for mobile traffic debugging with app intelligence**.

| Positioning Dimension | ProxyBot | mitmproxy | Proxyman | HTTP Toolkit |
|------------------------|----------|-----------|----------|--------------|
| **Target platform** | macOS (Phase 1) | Cross-platform | Mac/iOS/Android | Cross-platform |
| **Target user** | Developer (mobile) | Security researcher | Developer (all) | Developer (all) |
| **Primary differentiator** | App classification + pf | Addon ecosystem | Native macOS | One-click setup |
| **Pricing** | MIT (free) | MIT | $69/year | $12/month |
| **Enterprise** | Team workspace ✅ | — | ✅ | ✅ |
| **Mobile-first** | ✅ pf transparent | Manual proxy | Atlantis VPN | ADB one-click |
| **AI integration** | Gen tab (LLM) | — | — | — |
| **MCP Server** | ❌ | — | — | — |

---

## ProxyBot Advantages

1. **Transparent proxy** — No per-app configuration needed. Just set the phone's gateway to your Mac and all traffic is captured.

2. **App classification** — Correlates DNS queries with observed traffic to group requests by app (WeChat, Douyin, Alipay, etc.).

3. **Rust-based TUI + GUI** — Yew WASM GUI requires no Node.js or npm. Tauri GUI adds full desktop experience.

4. **Built-in DNS server** — DoH + UDP with hosts file, blocklist, and query logging for app correlation.

5. **Dual interface** — TUI for terminal efficiency + Tauri GUI for visual exploration (unique among Rust proxies).

6. **pf transparent proxy** — proxelar requires manual proxy configuration; ProxyBot phone needs zero config.

## proxelar (966 ⭐) — Closest Rust Competitor

proxelar is the most architecturally similar open-source project (Rust + ratatui + MITM + scripting).

**proxelar Advantages:**
1. **Lua scripting** — More mature ecosystem, more available scripts
2. **Column-scoped filtering** — `method:POST host:api status:200` syntax is intuitive
3. **Three interfaces** — Terminal + TUI + Web GUI via single binary
4. **Cross-platform** — macOS, Linux, Windows
5. **Simple CA install** — Visit `http://proxel.ar` through the proxy

**ProxyBot Advantages over proxelar:**
1. **pf transparent proxy** — proxelar requires manual proxy configuration
2. **App classification** — DNS correlation + SNI + domain rules
3. **Built-in DNS server** — DoH + UDP with logging
4. **Rule engine** — YAML-based 5-action rules vs Lua-only
5. **Gen tab** — Mock API / frontend scaffold / Docker generation
6. **DAG analysis** — Request dependency graphs and auth state machines
7. **Tauri GUI** — Native desktop experience vs web-based GUI
8. **Device management** — Per-device tracking and policy override

## anything-analyzer (2,366 ⭐) — AI-First Competitor

anything-analyzer is the first proxy tool to make AI analysis a first-class feature.

**anything-analyzer Advantages:**
1. **AI auto-analysis** — 5 analysis modes (API reverse-engineering, security audit, perf, JS crypto)
2. **MCP Server** — Exposes proxy as MCP tools for AI agents (Claude Desktop, Cursor)
3. **JS Hook injection** — crypto.subtle, CryptoJS, SM2/3/4 interception
4. **Dual capture** — Browser CDP + MITM proxy unified into one session
5. **Built-in browser** — Multi-tab Chromium for direct web interaction

**ProxyBot Advantages over anything-analyzer:**
1. **Rust performance** — Electron has significant memory/CPU overhead
2. **pf transparent proxy** — Zero-config mobile capture
3. **App classification** — Automatic app identification
4. **TUI** — Terminal-first, keyboard-driven efficiency
5. **Breakpoint editing** — Request/response modification (anything-analyzer is read-only)
6. **Rhai scripting** — Server-side traffic transforms
7. **Gen tab** — Mock generation + Docker bundling

**anything-analyzer Threat**: MCP Server strategy is the most concerning — if AI agents start using proxy tools as infrastructure, ProxyBot needs its own MCP Server to stay relevant.

## mitmproxy Advantages

1. **Cross-platform** — Works on macOS, Windows, and Linux.
2. **More mature** — Longer development history, more documentation.
3. **Scriptable** — Python scripting for complex use cases.
4. **Addon ecosystem** — Largest plugin community.

## Proxyman Advantages

1. **Native macOS app** — Clean native UI.
2. **Easy CA install** — One-click certificate installation on iOS.
3. **Map Local** — Useful for mocking API responses during development.
4. **Atlantis** — iOS VPN-based no-proxy capture.

## InterceptSuite Advantages

1. **Transport-layer focus** — TCP/UDP/DTLS/TLS, not just HTTP
2. **IoT/Thick Client** — Specialized for non-web protocols
3. **Python extensions** — Custom protocol dissectors
4. **PCAP export** — Wireshark-compatible analysis

## Strategic Recommendations (Based on 20+ Tool Analysis)

### P0 — Critical (Must Do)
1. **MCP Server** — anything-analyzer shows this is the AI agent integration point. Without it, ProxyBot becomes invisible to AI agents.

### P1 — High Priority
2. **Column-scoped Filter DSL** — proxelar's `method:POST host:api` is more intuitive. Integrate into existing Filter DSL.
3. **AI two-phase analysis pipeline** — Phase 1 filter noise → Phase 2 deep analysis. Improves Gen tab quality.
4. **Three-panel GUI layout** — HTTP Toolkit's pattern is the UX benchmark. Upgrade from 60/40 split.

### P2 — Medium Priority
5. **QR code CA distribution** — hyperfox/proxelar show this reduces mobile setup friction.
6. **proxybot-core crate** — Library-first design (like gomitmproxy) expands ecosystem.
7. **Project file management** — InterceptSuite-style save/restore capture sessions.

### P3 — Future Research
8. **HTTP/3 & QUIC** — Zero open-source proxies support it. Major whitespace.
9. **CDP browser capture** — anything-analyzer's dual capture is powerful but complex.
10. **Transport-layer proxy** — InterceptSuite shows demand for non-HTTP MITM. |

## ProxyBot Advantages

1. **Transparent proxy** — No per-app configuration needed. Just set the phone's gateway to your Mac and all traffic is captured.

2. **App classification** — Correlates DNS queries with observed traffic to group requests by app (WeChat, Douyin, Alipay, etc.).

3. **Rust-based TUI + GUI** — Yew WASM GUI requires no Node.js or npm. Tauri GUI adds full desktop experience.

4. **Built-in DNS server** — DoH + UDP with hosts file, blocklist, and query logging for app correlation.

5. **Dual interface** — TUI for terminal efficiency + Tauri GUI for visual exploration (unique among Rust proxies).

## proxelar (966 ⭐) — Closest Rust Competitor

proxelar is the most architecturally similar open-source project (Rust + ratatui + MITM + scripting).

**proxelar Advantages:**
1. **Lua scripting** — More mature ecosystem, more available scripts
2. **Column-scoped filtering** — `method:POST host:api status:200` syntax is intuitive
3. **Three interfaces** — Terminal + TUI + Web GUI via single binary
4. **Cross-platform** — macOS, Linux, Windows
5. **Simple CA install** — Visit `http://proxel.ar` through the proxy

**ProxyBot Advantages over proxelar:**
1. **pf transparent proxy** — proxelar requires manual proxy configuration
2. **App classification** — DNS correlation + SNI + domain rules
3. **Built-in DNS server** — DoH + UDP with logging
4. **Rule engine** — YAML-based 5-action rules vs Lua-only
5. **Gen tab** — Mock API / frontend scaffold / Docker generation
6. **DAG analysis** — Request dependency graphs and auth state machines
7. **Tauri GUI** — Native desktop experience vs web-based GUI
8. **Device management** — Per-device tracking and policy override

## anything-analyzer (2,366 ⭐) — AI-First Competitor

anything-analyzer is the first proxy tool to make AI analysis a first-class feature.

**anything-analyzer Advantages:**
1. **AI auto-analysis** — 5 analysis modes (API reverse-engineering, security audit, perf, JS crypto)
2. **MCP Server** — Exposes proxy as MCP tools for AI agents (Claude Desktop, Cursor)
3. **JS Hook injection** — crypto.subtle, CryptoJS, SM2/3/4 interception
4. **Dual capture** — Browser CDP + MITM proxy unified into one session
5. **Built-in browser** — Multi-tab Chromium for direct web interaction

**ProxyBot Advantages over anything-analyzer:**
1. **Rust performance** — Electron has significant memory/CPU overhead
2. **pf transparent proxy** — Zero-config mobile capture
3. **App classification** — Automatic app identification
4. **TUI** — Terminal-first, keyboard-driven efficiency
5. **Breakpoint editing** — Request/response modification (anything-analyzer is read-only)
6. **Rhai scripting** — Server-side traffic transforms
7. **Gen tab** — Mock generation + Docker bundling

## mitmproxy Advantages

1. **Cross-platform** — Works on macOS, Windows, and Linux.
2. **More mature** — Longer development history, more documentation.
3. **Scriptable** — Python scripting for complex use cases.
4. **Addon ecosystem** — Largest plugin community.

## Proxyman Advantages

1. **Native macOS app** — Clean native UI.
2. **Easy CA install** — One-click certificate installation on iOS.
3. **Map Local** — Useful for mocking API responses during development.
4. **Atlantis** — iOS VPN-based no-proxy capture.

## InterceptSuite Advantages

1. **Transport-layer focus** — TCP/UDP/DTLS/TLS, not just HTTP
2. **IoT/Thick Client** — Specialized for non-web protocols
3. **Python extensions** — Custom protocol dissectors
4. **PCAP export** — Wireshark-compatible analysis