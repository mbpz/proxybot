# Review Request — Step 3: DNS Server

**Ready for Review: YES**

## Summary

Built embedded DNS server for ProxyBot: listens on UDP 5300 (pf redirects 53->5300), parses DNS queries to extract domain names, forwards all queries to 8.8.8.8:53, stores last 10000 queries in memory, emits Tauri event for live UI updates.

## New File

**src-tauri/src/dns.rs** (lines 1-280)
- `DnsEntry { domain: String, timestamp_ms: u64 }` struct with serde Serialize
- `DnsState { entries: Arc<Mutex<VecDeque<DnsEntry>>>, running: AtomicBool }` shared state
- `parse_dns_query(buf: &[u8]) -> Option<String>` - RFC 1035 QNAME parser for Question Section
- `handle_dns_query()` - forwards to 8.8.8.8:53 with 3s timeout, relays response back
- `run_dns_server()` - main UDP loop using tokio::net::UdpSocket
- `start_dns_server()` / `stop_dns_server()` - start/stop control via AtomicBool
- `get_dns_log()` - Tauri command returning last 50 entries

## Modified Files

**src-tauri/src/lib.rs** (lines 1-36)
- Added `mod dns;` declaration
- Added `dns::DnsState` to Tauri managed state (`Arc<DnsState>`)
- Added `dns::get_dns_log` to Tauri invoke handler

**src-tauri/src/pf.rs** (lines 13-43)
- Added `DNS_PORT: u16 = 5300` constant
- Added DNS redirect rule to anchor rules: `rdr on {iface} proto udp from any to any port 53 -> 127.0.0.1 port 5300`

**src-tauri/src/proxy.rs** (lines 1-10, 853-880)
- Added `use crate::dns;` and `use crate::dns::DnsState;`
- `setup_pf` command: starts DNS server via `dns::start_dns_server()` after pf setup succeeds
- `teardown_pf` command: stops DNS server via `dns::stop_dns_server()` before pf teardown

**src/App.tsx** (lines 1-30, 38-55, 167-180, 250-295)
- Added `DnsEntry` interface
- Added `dnsQueries` state array
- Added `dns-query` event listener for live updates
- Added `get_dns_log` invoke call on mount
- Added DNS status indicator in Setup panel ("Listening on UDP 5300" / "Not running")
- Added DNS Queries section with table (Time + Domain), shows last 50 entries

**src/App.css** (lines 169-260, 415-445)
- Added `.dns-status`, `.dns-indicator`, `.dns-running`, `.dns-stopped` styles for Setup panel
- Added `.dns-log` section styles with `.dns-table` layout
- Added dark mode support for all DNS elements

## Architecture

```
DNS redirect flow:
┌──────────┐    ┌──────────┐    ┌─────────────┐
│   Phone  │───>│ pf (mac) │───>│ ProxyBot     │
│  (Wi-Fi) │    │ redirect │    │ DNS :5300    │
└──────────┘    └──────────┘    └─────────────┘
   port 53 ──> 127.0.0.1:5300
                     │
                     v
              ┌─────────────┐
              │  8.8.8.8:53 │
              │  (forward)  │
              └─────────────┘
```

## Key Implementation Details

- **No third-party DNS libraries** - uses only `tokio::net::UdpSocket`
- **RFC 1035 QNAME parsing** - manual parse of DNS Question Section (length-prefixed labels)
- **pf redirect 53->5300** - avoids needing root to bind port 53
- **DNS start/stop wired to pf** - `setup_pf` starts DNS, `teardown_pf` stops DNS
- **Live UI updates** - Tauri event `dns-query` emitted on each new query

## Build Verification

- `cargo check` in src-tauri/: **0 errors, 0 warnings**
- Frontend TypeScript: not explicitly checked, but follows existing patterns

## Fixes Applied

1. **Shutdown wakeup for recv_from** — Added `tokio::sync::broadcast` channel (`shutdown_tx: Arc<Mutex<Option<broadcast::Sender<()>>>>` in `DnsState`). `start_dns_server` creates the channel and stores the sender; `run_dns_server` subscribes to the receiver and uses `tokio::select!` to break on shutdown signal. `stop_dns_server` sends on the channel before setting `running` to false, ensuring `recv_from` is interrupted immediately.

2. **Removed unused `_upstream_socket`** — Deleted the `_upstream_socket` binding in `run_dns_server`. The single UDP socket bound to `0.0.0.0:5300` is sufficient for both receiving from phone and sending to upstream `8.8.8.8:53` (UDP is connectionless).
