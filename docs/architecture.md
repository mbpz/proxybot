# Architecture

ProxyBot separates a reusable MITM Runtime from platform and presentation
Adapters. The architecture goal is not more layers; it is to keep capture,
persistence, desktop behavior, and experiments behind deep Modules with small
Interfaces.

The canonical domain language is defined in
[`CONTEXT.md`](https://github.com/mbpz/proxybot/blob/main/CONTEXT.md).

## Runtime composition

```text
Process Config
     |
     v
desktop composition root ---------------------- MCP stdio Adapter
     |                                                |
     +--> macOS / Tauri Adapters                      |
     +--> SQLite Adapters <---------------------------+
     +--> certificate and DNS resources
     |
     v
MITM Runtime --> Capture Event --> desktop Runtime Adapter
     |                                  |
     |                                  +--> Captured Request persistence
     |                                  +--> Application Attribution
     |                                  +--> Alerts and analysis
     |                                  +--> desktop event delivery
     v
upstream server
```

`src-tauri/src/bootstrap.rs` is the single composition root. It parses Process
Config, selects the desktop or MCP launch Adapter, creates shared resources, and
owns process shutdown.

## Modules and seams

### `proxybot-core`

The reusable core owns:

- validated Process Config and Runtime Config
- the MITM Runtime and its lifecycle handle
- TLS certificate generation and interception decisions
- Routing Rule models and matching
- Capture Event types
- Application Attribution and analysis models
- specification-generation primitives

The MITM Runtime exposes `RuntimeHooks` and `OriginalDestination` Seams instead
of depending on Tauri, SQLite, or macOS `pf`.

### Desktop Runtime Adapter

`src-tauri` supplies the desktop Implementations. The Runtime Adapter translates
Capture Events into persisted Captured Requests, WebSocket records, application
attribution, alerts, analysis inputs, and Tauri events.

The Runtime Extension Pipeline owns ordered plugin dispatch, Rhai scripts,
metrics, and Network Condition Rules. It is separate from Routing Rules.

### Persistence

SQLite stores Captured Requests, Devices, Alerts, DNS Observations, configuration,
and generation state. Focused query Modules should own SQL and mapping. Desktop
and MCP Adapters should consume those Interfaces instead of sharing raw
`Mutex<Connection>` access.

### React desktop Adapter

The React application renders the desktop product. A generated Desktop Contract
provides typed command and event metadata plus a BrowserMockAdapter for fast UI
tests. Migration is incomplete: some screens still call Tauri directly or use a
shallow Adapter that converts errors into `null`.

The shell exposes five Product Destinations: Capture, Setup, Rules, Replay, and
Settings. Requests, DNS, Alerts, Graph, and Topology share the Capture context;
Replay and Composer share the Replay context. Historical and experimental routes
remain valid deep links so saved URLs do not break, but they do not compete in
the default navigation.

General Settings and the Capture/Settings DNS surfaces cross the generated
Desktop Contract Interface for reads, mutations, and DNS Observations. Transport
and result-shape failures remain typed errors in the UI; they are not converted
into false, empty, or null settings.

## Network modes

### Explicit proxy — Core

The device sends HTTP and HTTPS connections directly to the MITM Runtime. This
is the default because its configuration and cleanup are local to the test
device.

### macOS `pf` and DNS — Advanced

The desktop Adapter can install a dedicated `pf` redirect and run a DNS server
for DNS Observation and Application Attribution. This mode changes host network
state, may require elevated privileges, and needs explicit cleanup.

## Capture lifecycle

The intended ownership model is:

1. a retained resource is reported running only after its listener binds or its
   device setup completes;
2. repeated start fails clearly;
3. stop is idempotent;
4. stop returns only after owned tasks, listeners, and handles are released;
5. process exit drains the MITM Runtime and desktop network resources.

The core runtime already owns its listener handle. The desktop layer still needs
to make the Capture Event bridge and breakpoint task part of the same retained
Capture Session Module.

## Captured Request data flow

```text
client connection
    --> MITM Runtime
    --> stable-id Capture Event
    --> desktop Runtime Adapter
    --> Application Attribution
    --> SQLite Captured Request
    --> Desktop Contract event
    --> Traffic workspace
```

Analysis Implementations consume an immutable Captured Request Analysis view so
Graph, Topology, authentication, and anomaly logic do not each reinterpret raw
database rows.

## Security boundaries

- The local CA private key and captured credentials are secrets.
- Explicit proxy is preferred before host-wide routing changes.
- `pf`, DNS, certificate distribution, dashboard, and MITM listeners are
  Desktop Network Resources with explicit ownership and cleanup.
- MCP stdio is a local Adapter but can expose sensitive persisted data to its
  client.
- Android SSL-bypass tooling changes applications and belongs in Labs.
- The current Tauri global API and null CSP are migration debt; the target is a
  minimal capability set and non-null policy.

## Verification boundary

Rust and UI tests cover Modules and the BrowserMockAdapter. Current Playwright
tests start Vite rather than a packaged Tauri application. The Packaged Desktop
Acceptance lane executes the real `.app` binary and Tauri composition root in an
isolated workspace, then proves CA preparation, decrypted local HTTPS capture,
SQLite observation, stop, restart, and cleanup without external network access.
It does not drive visible UI controls or prove a signed installation; those
remain release evidence requirements.

See the [product roadmap](roadmap.md) for the ordered deepening work.
