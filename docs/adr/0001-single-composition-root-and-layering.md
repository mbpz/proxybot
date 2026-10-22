# ADR-0001: Single composition root and layered adapters

- **Status:** Accepted
- **Date:** 2026-10-22
- **Deciders:** ProxyBot maintainers

## Context

ProxyBot runs the same capture engine in two modes: a Tauri desktop application
and a headless MCP stdio adapter. Both need the same MITM runtime, certificate
handling, routing rules, and persistence, but they must not duplicate process
setup, configuration parsing, or shutdown behavior. The reusable engine is also
published as `proxybot-core` and must stay free of Tauri, SQLite, and macOS
packet-filter dependencies.

## Decision

1. `src-tauri/src/bootstrap.rs` is the only composition root. It parses the
   `Process Config`, selects the desktop or MCP launch adapter, creates shared
   resources, and owns process shutdown.
2. `proxybot-core` exposes runtime seams (`RuntimeHooks`,
   `OriginalDestination`) instead of depending on platform or presentation
   concerns.
3. `src-tauri` supplies the desktop implementations: persistence, network
   resources, Tauri commands, and event delivery. Adapters translate core types;
   they do not fork runtime behavior.
4. New launch modes must be added behind the same composition root. A second
   root that parses configuration or opens the database independently is
   rejected.

## Consequences

- The capture engine is testable without a GUI and can be reused by future
  frontends.
- Desktop and MCP observe the same configuration and the same persisted facts.
- The composition root is a critical file: it must stay an assembly point, not
  accumulate domain logic.
- Adding a platform (for example Windows later) or a new adapter requires no
  changes to the core runtime contract.

## Evidence

- `src-tauri/src/bootstrap.rs`
- `proxybot-core/src/lib.rs`
- `proxybot-core/src/proxy_engine.rs` (`RuntimeHooks`, `OriginalDestination`)
- `docs/architecture.md` ("Runtime composition", "Modules and seams")
