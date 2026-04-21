# PRD: NetMind Agent — System Roadmap v1.0

## Introduction

NetMind Agent is a multi-phase system that captures, analyzes, and replicates mobile app traffic. The system progresses through four phases: (1) transparent proxy with TLS decryption, (2) virtual soft router with policy routing, (3) AI-driven traffic analysis and API inference, and (4) automatic mock API + frontend scaffolding generation. The goal is a system where any app running on a phone connected to the same network can be fully captured, understood, and replicated without access to the original backend.

**Problem being solved:** Reverse engineers, QA teams, and developers currently spend weeks manually capturing and documenting app APIs. NetMind Agent automates this pipeline end-to-end.

**Target users:** Individual developers, security researchers, QA engineers, and mobile app developers who need to understand or replicate app behavior.

---

## Goals

- **G-1:** Phase 1 delivers a working transparent proxy where a PC acts as the phone's gateway, capturing all HTTP/HTTPS traffic with TLS decryption in real-time.
- **G-2:** Phase 2 adds policy routing, DNS splitting, multi-device management, and traffic recording/replay.
- **G-3:** Phase 3 uses LLMs to automatically infer API semantics, build dependency graphs, generate OpenAPI specs, and detect anomalies.
- **G-4:** Phase 4 generates runnable mock backends and frontend scaffolds from analyzed traffic.
- **G-5:** The system is cross-platform from day one (macOS + Windows) with a unified Rust core.
- **G-6:** AI capabilities are integrated incrementally — basic app classification in Phase 1, richer inference in Phase 3.

---

## User Stories

### US-001: Transparent proxy setup (Phase 1)
**Description:** As a developer, I want to install NetMind Agent on my PC with one click so my phone can use the PC as a gateway without manual network configuration.

**Acceptance Criteria:**
- [ ] PC installer works on macOS and Windows without manual dependency setup
- [ ] Agent prompts for admin privileges on first launch and configures IP forwarding automatically
- [ ] Agent starts a tray icon on system boot
- [ ] Agent shows a QR code that configures the phone's Wi-Fi gateway + DNS to point to the PC IP
- [ ] iOS and Android configuration guides are accessible from the QR code landing page

### US-002: HTTPS decryption (Phase 1)
**Description:** As a developer, I want the proxy to decrypt HTTPS traffic so I can inspect request/response bodies in plain text.

**Acceptance Criteria:**
- [ ] On first launch, agent generates a self-signed root CA and shows an installation wizard
- [ ] iOS users can install the CA via MDM Profile link; Android users follow a certificate-trust guide
- [ ] For devices where CA installation is not possible (Android 7+, iOS without MDM), agent falls back to VPN mode
- [ ] Each intercepted connection uses a leaf certificate signed by the root CA
- [ ] Root CA can be regenerated; existing leaf certificates remain valid during the same session

### US-003: Traffic dashboard (Phase 1)
**Description:** As a developer, I want to see live HTTP/WebSocket traffic in a web UI so I can filter and inspect requests in real-time.

**Acceptance Criteria:**
- [ ] Web UI displays a scrollable request list with method, URL, status code, and timestamp
- [ ] Clicking a request shows full headers, query params, and body with JSON/HTML formatter
- [ ] Requests are groupable by Host with keyword filter text box
- [ ] Images in responses render as thumbnails
- [ ] WebSocket frames display as bidirectional event entries

### US-004: App classification (Phase 1)
**Description:** As a developer, I want the system to automatically tag traffic by app (WeChat, Douyin, Alipay) so I can filter by source app.

**Acceptance Criteria:**
- [ ] System classifies connections by correlating DNS queries with subsequent TCP/TLS connections
- [ ] Domain rules cover: `*.weixin.qq.com`, `*.wechat.com`, `*.qq.com` (WeChat); `*.douyin.com`, `*.tiktokv.com` (Douyin); `*.alipay.com`, `*.alipaycdn.com` (Alipay)
- [ ] SNI inspection provides secondary classification when DNS correlation is ambiguous
- [ ] Each request shows its assigned app tag in the traffic list
- [ ] App classification logic is extensible for additional apps via a rules file

### US-005: Rule engine — routing policies (Phase 2)
**Description:** As a user, I want to define routing rules so traffic can be direct, proxied, or blocked based on domain/IP/CIDR/geo.

**Acceptance Criteria:**
- [ ] Rule file supports DOMAIN, DOMAIN-SUFFIX, IP-CIDR, GEOIP keywords
- [ ] Rules are loaded from YAML and hot-reloaded on change without restart
- [ ] Each rule maps to an action: DIRECT, PROXY, REJECT
- [ ] Rules are ordered; first matching rule wins
- [ ] Rule editor UI in Web UI allows creating, editing, and reordering rules

### US-006: DNS enhanced layer (Phase 2)
**Description:** As a user, I want the system to resolve DNS intelligently so domains can be routed differently based on resolved IP.

**Acceptance Criteria:**
- [ ] Agent runs a local DNS server (dnsmasq-style) that logs all queries
- [ ] Upstream DNS supports DoH and DoT in addition to plain UDP
- [ ] Local hosts file entries and ad-block rules are supported
- [ ] Domain-to-DNS-resolution mappings feed the routing engine

### US-007: Multi-device management (Phase 2)
**Description:** As a user, I want to manage multiple phones independently so each device gets its own policy and statistics.

**Acceptance Criteria:**
- [ ] Devices are identified by MAC address and assigned a human-readable name
- [ ] Each device has its own rule set, speed limit, and traffic statistics
- [ ] Web UI shows a topology diagram of connected devices
- [ ] Device list persists across restarts

### US-008: Traffic recording and replay (Phase 2)
**Description:** As a QA engineer, I want to record traffic and replay it against a mock server so I can test without the real backend.

**Acceptance Criteria:**
- [ ] Traffic can be exported as HAR or pcap
- [ ] User can select a host and replay all recorded requests against a local mock
- [ ] Replay shows diff between recorded response and mock response
- [ ] Replay supports looping and variable delay injection

### US-009: AI — traffic structured storage (Phase 3)
**Description:** As a developer, I want traffic stored as structured records so AI can query and analyze it.

**Acceptance Criteria:**
- [ ] Each HTTP exchange is normalized into a schema: method, path, query params, request headers, request body (parsed), response headers, response body (parsed), timing
- [ ] Storage supports JSON, Protobuf, and GraphQL body variants
- [ ] A request DAG is constructed from temporal ordering and token/ID dependencies across a session
- [ ] Storage is queryable via FastAPI endpoints

### US-010: AI — API semantic inference (Phase 3)
**Description:** As a developer, I want the LLM to look at a sequence of related requests and tell me what API names and parameters mean so I don't have to guess.

**Acceptance Criteria:**
- [ ] Given a request sequence, LLM outputs a structured JSON: `{ "interfaces": [{ "name": "userProfile", "method": "GET", "path": "/api/v3/user/profile", "params": {...} }] }`
- [ ] LLM identifies auth token passing chains (login → token → resource calls)
- [ ] LLM groups requests into functional modules (e.g., "payments", "social", "feed")
- [ ] An Evaluator agent checks LLM output consistency and flags mismatches
- [ ] OpenAPI and AsyncAPI specs are generated from the inferred semantics

### US-011: AI — state machine modeling (Phase 3)
**Description:** As a developer, I want the system to show me the login → token → resource lifecycle so I understand the app's auth flow.

**Acceptance Criteria:**
- [ ] System automatically constructs a state machine from observed traffic: states are resources or sessions, transitions are authenticated calls
- [ ] Diagram is exported as a visual graph (Mermaid or similar)
- [ ] Anomalous state transitions (e.g., calling a resource before login) are flagged

### US-012: AI — anomaly detection and privacy scanning (Phase 3)
**Description:** As a security researcher, I want the system to alert me when traffic looks suspicious or leaks private data.

**Acceptance Criteria:**
- [ ] System learns per-device traffic baseline; new domains trigger alerts
- [ ] Detection rules flag exfiltration patterns: repeated GPS coordinates, IDFA, phone number formats
- [ ] Alerts appear in the dashboard with severity level and details
- [ ] Alert history is logged and queryable

### US-013: Mock API generation (Phase 4)
**Description:** As a developer, I want the system to take analyzed traffic and generate a working mock API server so I can replace the real backend.

**Acceptance Criteria:**
- [ ] OpenAPI spec → Hono or FastAPI code generation with one click
- [ ] Recorded responses are embedded as fixture data
- [ ] Mock server handles stateful sequences (ordered responses, conditional responses based on request body)
- [ ] Mock server runs in a Docker container with one command

### US-014: Frontend scaffold generation (Phase 4)
**Description:** As a developer, I want the system to generate a React frontend scaffold that calls my mock API so I have a working UI to iterate on.

**Acceptance Criteria:**
- [ ] LLM generates React components keyed to each inferred API endpoint
- [ ] Routing and state management are inferred from API semantics
- [ ] Generated scaffold includes E2E Playwright tests
- [ ] Generator → Evaluator闭环 runs: generated code is replay-tested against real traffic, diffs auto-correct the scaffold

### US-015: Vision-based UI replication (Phase 4)
**Description:** As a developer, I want to upload a screenshot of the app and have the system infer the UI structure so the frontend scaffold matches the real UI.

**Acceptance Criteria:**
- [ ] Screenshot upload → Vision model analysis → component structure JSON
- [ ] Vision output maps to generated React component hierarchy
- [ ] Traffic analysis and screenshot analysis are fused to produce a cohesive replica

### US-016: One-click deployment bundle (Phase 4)
**Description:** As a developer, I want one command to produce a Docker Compose file containing mock API + frontend + DB so I can ship the replica immediately.

**Acceptance Criteria:**
- [ ] `docker compose up` brings up mock API, frontend, and SQLite/Postgres
- [ ] Git repo is initialized with the generated project
- [ ] CI template (GitHub Actions) runs the Playwright E2E suite as a gate

---

## Functional Requirements

### Phase 1 — Transparent Proxy

- **FR-1:** PC installs a Tauri v2 + React agent that binds to ports 80/443 as a transparent proxy
- **FR-2:** On first launch, agent generates an RSA 2048 root CA stored at `~/.proxybot/ca.pem`
- **FR-3:** Agent configures macOS pf / Windows netsh to redirect port 80/443 traffic to the local proxy
- **FR-4:** Agent starts a local DNS server on port 53 logging all queries with timestamps
- **FR-5:** TLS MITM is performed using rustls with dynamically generated leaf certificates per destination
- **FR-6:** VPN mode (TUN interface) is used as fallback when pf/netsh redirect is unavailable
- **FR-7:** Web UI at `localhost:19999` shows live traffic with host grouping and keyword filtering
- **FR-8:** WebSocket connections are captured and displayed frame-by-frame
- **FR-9:** App classification uses DNS-to-connection correlation with domain rules and SNI inspection
- **FR-10:** Traffic data is stored in SQLite WAL mode for concurrent read/write

### Phase 2 — Soft Router

- **FR-11:** Rule engine loads YAML files from `~/.proxybot/rules/` with DOMAIN/DOMAIN-SUFFIX/IP-CIDR/GEOIP support
- **FR-12:** Rules hot-reload within 2 seconds of file change via file watcher
- **FR-13:** DNS server supports DoH (`https://1.1.1.1/dns-query`) and DoT (`tls://1.1.1.1`) upstreams
- **FR-14:** Local hosts file at `~/.proxybot/hosts` and ad-block list are merged into DNS responses
- **FR-15:** Device registry stores MAC address → human-readable name mapping in SQLite
- **FR-16:** Per-device rule assignment, byte counters, and speed limits are stored and enforced
- **FR-17:** Traffic recording exports to HAR 1.2 format; replay injects recorded responses
- **FR-18:** Replay diff mode highlights header and body differences between recorded and served responses

### Phase 3 — AI Analysis

- **FR-19:** Traffic normalizer extracts HTTP exchanges into a typed schema stored in SQLite
- **FR-20:** Request DAG is built from timestamp ordering plus extracted ID/token references (e.g., `access_token`, `sessionId`)
- **FR-21:** Claude API is called per session with all requests as context; response is a structured JSON per FR-010
- **FR-22:** Evaluator agent validates LLM output: checks that all requests are covered, IDs are consistent, and paths are correctly named
- **FR-23:** OpenAPI 3.1 spec is generated from LLM output; AsyncAPI spec for WebSocket/SSE flows
- **FR-24:** State machine visualizer outputs Mermaid diagram for auth flows
- **FR-25:** Anomaly detector compares new traffic against 7-day rolling baseline per device
- **FR-26:** Privacy scanner regex-matches request/response bodies for IDFA, phone number, lat/long patterns
- **FR-27:** Alerts are stored in SQLite and displayed in the Web UI dashboard with severity badges

### Phase 4 — App Synthesis

- **FR-28:** OpenAPI spec → Hono or FastAPI project via code generation; fixtures populated from recorded responses
- **FR-29:** Mock server supports ordered sequences and conditional responses (match request body field X → return response Y)
- **FR-30:** LLM generates a React + Vite scaffold with routes inferred from API paths
- **FR-31:** Playwright E2E tests are auto-generated and run as CI gate
- **FR-32:** Docker Compose is produced with services: mock-api, frontend, postgres
- **FR-33:** Git repo initialization with CI template (GitHub Actions) is automated
- **FR-34:** Vision model analysis of uploaded screenshots produces component tree JSON
- **FR-35:** Scaffold generation fuses traffic-based API knowledge with Vision-based layout knowledge

---

## Non-Goals

- **NG-1:** This PRD does NOT cover iOS/Android native app development — only the PC-side proxy and web UI.
- **NG-2:** Phase 4 frontend scaffold generation is not required for the Phase 1-3 MVP; it is a deferrable goal.
- **NG-3:** The system is NOT a general-purpose VPN service — it is scoped to traffic capture and analysis on a local network.
- **NG-4:** No real-time blocking of traffic based on AI inference in Phase 1 — classification is for display and filtering only.
- **NG-5:** Root CA installation on iOS without MDM / Android without root is handled by VPN fallback, not full certificate trust. This is intentional to keep Phase 1 achievable without enterprise MDM.
- **NG-6:** The system does NOT persist traffic data long-term in Phase 1 — session-only. Long-term storage is a Phase 2 concern.

---

## Technical Considerations

### Stack per Phase

| Phase | Core | UI | Storage | AI |
|-------|------|-----|---------|-----|
| Phase 1 | Rust (hyper + rustls + tokio) | Tauri v2 + React + shadcn/ui | SQLite WAL | — |
| Phase 2 | Rust + sing-box integration | React Web UI | SQLite WAL | — |
| Phase 3 | FastAPI + Qdrant | React Web UI | SQLite + Qdrant | Claude API (structured output) |
| Phase 4 | Hono/FastAPI + Claude Vision | React scaffold | PostgreSQL | Claude API (codegen) |

### Key Integration Points

- **macOS pf:** configured via `pfctl` with an anchor file managed by the agent
- **Windows netsh:** interfacial proxy mode via `netsh interface portproxy`
- **TUN mode:** tokio-tun crate for VPN fallback on both platforms
- **DNS logging:** agent runs a local UDP/TCP DNS server that logs queries before forwarding
- **Root CA generation:** rcgen crate generates x509 certificates; exported as PEM for phone installation
- **sing-box:** Phase 2 embeds sing-box as a library for routing core; no separate process

### Performance Targets

- Proxy latency overhead < 50ms per request (Phase 1)
- DNS query log ingestion rate > 1000 queries/second (Phase 2)
- LLM inference for a 50-request session < 30 seconds (Phase 3)
- Mock server startup < 5 seconds from bundle (Phase 4)

---

## Success Metrics

- **SM-1:** A new user can install the agent, configure the phone, and see live HTTPS traffic in the dashboard within 15 minutes on macOS.
- **SM-2:** WeChat, Douyin, and Alipay traffic is automatically tagged within 5 seconds of first connection.
- **SM-3:** Phase 2 rule hot-reload completes within 2 seconds of file save.
- **SM-4:** A 50-request session generates a correct OpenAPI spec (validated against recorded traffic) in Phase 3.
- **SM-5:** Phase 4 mock server passes ≥ 80% of recorded traffic replay without manual fixture correction.
- **SM-6:** Cross-platform parity: both macOS and Windows agents pass the same traffic capture test suite.

---

## Open Questions

- **Q-1:** Should VPN mode use a TUN interface that creates a virtual NIC, or a userspace VPN (like mitmproxy's `--mode transparent` + `--set httpionly`) that avoids kernel-level changes?
- **Q-2:** For Phase 2 multi-device, should each device get an isolated proxy port range, or should the routing engine enforce device policy via MAC → VLAN tagging?
- **Q-3:** In Phase 3, should the LLM run on-device (via local model) or always use the Claude API? Cost vs. privacy trade-off.
- **Q-4:** For Phase 4 scaffold generation, should the output be a single monorepo or separate `mock-api/` and `frontend/` repos?
- **Q-5:** Should Phase 4's Vision-based UI replication generate React code directly, or output a JSON schema that a separate code renderer interprets?
