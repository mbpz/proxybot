# Network Conditions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement network condition simulation (latency, bandwidth, packet loss) for proxy traffic testing.

**Architecture:** NetworkConditionEngine holds presets and active profile. Injection happens in pipe_tcp_bidirectional and handle_https_connect via simple sleep/delay after reads.

**Tech Stack:** Rust (tokio::time, rand crate)

---

## File Structure

```
src-tauri/src/network/
├── mod.rs       # Module exports, built-in presets
├── profile.rs   # NetworkProfile, ConditionRule structs
└── engine.rs    # NetworkConditionEngine with apply()
```

Modify: `src/lib.rs` (add `pub mod network;`), `src/proxy.rs` (inject conditions)

---

## Dependencies

Add to `Cargo.toml`:
```toml
rand = "0.8"  # for packet loss randomization
```

---

## Tasks

### Task 1: NetworkProfile and NetworkConditionEngine

**Files:**
- Create: `src/network/mod.rs`
- Create: `src/network/profile.rs`
- Create: `src/network/engine.rs`
- Modify: `src/lib.rs`

Implement `NetworkProfile`, `ConditionRule`, `NetworkConditionEngine` with:
- `new()` with built-in presets (2G, 3G, 4G, WiFi, Edge)
- `set_active(name)`, `disable()`, `get_active()`, `list_profiles()`
- `apply(read_size)` returns `ConditionEffect { delay_ms, drop }`

### Task 2: Inject into pipe_tcp_bidirectional and handle_https_connect

**Files:**
- Modify: `src/proxy.rs`

Add `network: Arc<NetworkConditionEngine>` to `ProxyContext`. In pipe functions, after each read, call `apply(n)` to get delay/drop, apply accordingly.

### Task 3: CLI commands

**Files:**
- Modify: `src/bin/proxybot-tui.rs`

Add `network` subcommand with preset/latency/bandwidth/loss/off/status.

### Task 4: Tests

**Files:**
- Modify: `src/network/engine.rs` (add test module)

Test latency, bandwidth cap, packet loss statistical, preset application.
