# ADR-0003: Capture Event and SQLite persistence ownership

- **Status:** Accepted
- **Date:** 2026-10-22
- **Deciders:** ProxyBot maintainers

## Context

The MITM runtime emits lifecycle events for every accepted connection and
WebSocket exchange. The desktop application must show history, the MCP adapter
must query the same history, and analysis features (Graph, Topology, alerts,
replay) must observe identical facts. Letting each feature read the database
with its own SQL — or, worse, letting each feature re-interpret runtime types —
produces incompatible semantics and silent divergence.

## Decision

1. The MITM runtime emits `CaptureEvent`s with a stable id; it does not write
   the database and does not depend on SQLite.
2. The desktop runtime adapter translates `CaptureEvent`s into persisted
   Captured Requests, WebSocket records, application attribution, alerts,
   analysis inputs, and desktop events.
3. Persisted facts are owned by SQLite. Focused query modules own the SQL and
   mapping for their aggregate; desktop commands and the MCP adapter consume
   those interfaces instead of duplicating queries or sharing raw
   `Mutex<Connection>` access.
4. Desktop and MCP must expose the same persisted facts and terminology. A
   capability that queries persistence through a second path is considered
   unfinished.
5. Analysis implementations consume an immutable Captured Request Analysis
   view; they do not each reinterpret raw rows.

## Consequences

- History, export, MCP queries, and analysis see one consistent set of records.
- A schema change has one owner and one migration point.
- The desktop and MCP adapters remain thin; business semantics live in shared
  modules.
- Deeper adoption is ongoing: query modules are the target seam, and remaining
  ad-hoc SQL must move behind them before MCP can be considered a pure adapter.

## Evidence

- `proxybot-core/src/proxy_engine.rs` (`CaptureEvent`)
- `src-tauri/src/db.rs` (schema and persistence)
- `src-tauri/src/db/captured_requests.rs` (focused query module)
- `src-tauri/src/mcp/server.rs` (MCP reads persisted facts)
- `docs/architecture.md` ("Persistence", "Captured Request data flow")
- `docs/mcp-integration.md` (MCP boundary and known gaps)
