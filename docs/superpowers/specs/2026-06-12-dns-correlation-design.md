# DNS-to-Connection Correlation Design

**Date:** 2026-06-12
**Author:** Claude
**Status:** Implemented (v1.3.x)

---

## 1. Context

ProxyBot captures HTTP/HTTPS/WSS traffic from a phone whose gateway points at the PC. Each request is tagged with an `app_name` derived from SNI / `Host` header against a domain rule library (WeChat, Douyin, Alipay, Baidu, etc.). The built-in DNS server also tags every DNS query it sees.

Today these two signals are joined by `DnsState::correlate_app` (`src-tauri/src/dns.rs:227`), which runs as a fallback after `classify_host` misses. It is wired in at three call sites: `proxy/https.rs:325`, `proxy/http.rs:241`, `proxy/http.rs:346`.

**Gaps observed:**

1. The correlation window is **5 seconds** (`window_ms = 5000u64`). DNS TTLs are typically 60–300 s, and a phone frequently opens TCP connections tens of seconds after the lookup. Many legitimate requests fall outside the window and stay tagged `Unknown`.
2. The match key is **host-string only** — `host == domain` or `host.ends_with(".domain")`. When the phone dials an IP literal (e.g. a CDN endpoint like `https://1.2.3.4/...` with `Host: api.weixin.qq.com`), the proxy sees `target_host = "1.2.3.4"`. `classify_host("1.2.3.4")` returns `None`, and `correlate_app("1.2.3.4", ...)` returns `None` because no DNS entry has `domain = "1.2.3.4"`. The connection is left untagged even though a recent DNS lookup for `api.weixin.qq.com` resolved to exactly that IP.

This spec widens the window to 5 minutes and adds a second correlation path keyed on `(client_ip, resolved_ip)`, both falling back from the SNI/Host match.

## 2. Goals & Non-Goals

**Goals**

- Tag connections whose SNI/Host did not match any rule, when a recent DNS lookup (within 5 min) for the same client can be associated with the request.
- Tag IP-literal connections whose resolved IP appeared in a recent DNS response for a classified domain.
- Keep the change additive — no breaking change to existing SNI matches, no DB schema change, no new Tauri commands, no UI change.

**Non-goals**

- Source attribution ("WeChat via SNI" vs "via DNS") in the UI — out of scope; the spec keeps a single `app_name` per request.
- Cross-device correlation. The IP path filters by `client_ip` to avoid attributing one phone's tags to another on the same WiFi.
- Hostile-client / DNS-rebinding defenses. The phone is on a trusted LAN and the tag is for UI, not security enforcement.
- Backfilling tags on historical rows. New rows only.

## 3. Architecture

### 3.1 Correlation precedence

At every request capture site the lookup is:

1. `app_rules::classify_host(target_host)` — direct SNI/Host match against the domain rule library. **Wins if `Some`.**
2. `dns_state.correlate_app(target_host, request_ts_ms)` — host-string DNS match (existing). Used only if step 1 missed.
3. `dns_state.correlate_app_for_ip(client_ip, resolved_ip, request_ts_ms)` — new. Used only if steps 1 and 2 missed.
4. `None` — request stays untagged. Existing behavior.

Steps 1–2 are unchanged in semantics. Step 3 is the new arm.

### 3.2 Window

A single `pub const CORRELATION_WINDOW_MS: u64 = 300_000;` (5 minutes) governs both the host-string path (`correlate_app`) and the new IP path (`correlate_app_for_ip`). The existing local `5000u64` in `correlate_app` is replaced with the constant.

### 3.3 Data structures

`DnsEntry` gains one field:

```rust
pub struct DnsEntry {
    // ... existing fields ...
    pub client_ip: Option<String>, // NEW: source of the DNS query
}
```

`record_query` (in `src-tauri/src/dns.rs`) is updated to populate `client_ip` from the UDP socket's peer address at the time of the query. Existing `entries` consumers that don't need `client_ip` (e.g. the UI DNS log) ignore it; future ones (e.g. per-device stats) can use it.

No new collections are needed. Both correlation functions iterate the existing `entries: Arc<Mutex<VecDeque<DnsEntry>>>` deque reverse-chronologically and break on the first entry that is older than the window.

### 3.4 New API

In `src-tauri/src/dns.rs`:

```rust
/// Correlate a request whose target_host is a literal IP (e.g. CDN) against
/// recent DNS resolutions. Returns the app_name/app_icon of the most recent
/// DNS query for the same client whose resolved_ips includes `resolved_ip`
/// within CORRELATION_WINDOW_MS.
pub fn correlate_app_for_ip(
    &self,
    client_ip: &str,
    resolved_ip: &str,
    request_timestamp_ms: u64,
) -> Option<(String, String)>
```

Pseudocode:

```rust
let entries = self.entries.lock().ok()?;
for entry in entries.iter().rev() {
    if request_timestamp_ms < entry.timestamp_ms { continue; }  // clock skew
    if request_timestamp_ms - entry.timestamp_ms > CORRELATION_WINDOW_MS { break; }
    if entry.client_ip.as_deref() != Some(client_ip) { continue; }
    if !entry.resolved_ips.iter().any(|ip| ip == resolved_ip) { continue; }
    if let (Some(name), Some(icon)) = (&entry.app_name, &entry.app_icon) {
        return Some((name.clone(), icon.clone()));
    }
}
None
```

The loop returns the **most recent** matching entry (reverse iteration, first hit wins). Matches the "latest only" requirement.

## 4. Data Flow

### 4.1 Common case (SNI matches a rule)

Phone → WeChat opens, app DNS-queries `api.weixin.qq.com`. Built-in DNS server records `DnsEntry { domain: "api.weixin.qq.com", timestamp_ms, app_name: Some("WeChat"), resolved_ips: ["1.2.3.4"], client_ip: Some("192.168.1.5") }`.

Phone connects to `api.weixin.qq.com:443` through the proxy.

`proxy/https.rs:325` runs:
- `classify_host("api.weixin.qq.com")` → `Some(("WeChat", "💬"))` — used. The new code paths are not invoked.

### 4.2 IP-literal case (SNI/Host didn't match, but a DNS lookup for the same IP exists)

Phone → some app dials a CDN IP directly: `https://1.2.3.4/foo` (TLS SNI and `Host` header still `api.weixin.qq.com`, but the CONNECT request is `CONNECT 1.2.3.4:443`). A recent DNS lookup for `api.weixin.qq.com` resolved to `1.2.3.4`.

`proxy/https.rs:325` runs:
- `classify_host("1.2.3.4")` → `None` (no rule matches an IP).
- `correlate_app("1.2.3.4", ts)` → `None` (no DNS entry has `domain = "1.2.3.4"`).
- `correlate_app_for_ip("192.168.1.5", "1.2.3.4", ts)` → scans `entries`, finds the `api.weixin.qq.com` entry with `resolved_ips` containing `"1.2.3.4"` and `client_ip = "192.168.1.5"` and `timestamp_ms` within 5 min → returns `Some(("WeChat", "💬"))`.

The captured request is persisted with `app_name = "WeChat"`.

### 4.3 No DNS yet

Phone → app dials `1.2.3.4` without any prior DNS query (rare, but possible if the app pre-cached the IP and bypasses DNS).

- All three correlation arms return `None`. The request is persisted with `app_name = None`. UI shows "Unknown" (or whatever the existing tag for None is).

## 5. Implementation Notes

### 5.1 Files changed

**`src-tauri/src/dns.rs`**
- Add `const CORRELATION_WINDOW_MS: u64 = 300_000;` at module scope.
- In `correlate_app`, replace `let window_ms = 5000u64;` with `let window_ms = CORRELATION_WINDOW_MS;`.
- Add `pub fn correlate_app_for_ip(...)` per §3.4.
- Add `pub client_ip: Option<String>` to `DnsEntry`.
- In `record_query`, capture the UDP socket's peer address and store it in `entry.client_ip`.

**`src-tauri/src/proxy/https.rs`** (line ~325)
- After the upstream TCP connect succeeds, capture `let resolved_ip = stream.peer_addr().ok().map(|a| a.ip().to_string());`.
- Change the `.or_else()` chain to:
  ```rust
  let app_info = crate::app_rules::classify_host(&target_host)
      .or_else(|| ctx.dns_state.correlate_app(&target_host, request_ts_ms))
      .or_else(|| {
          resolved_ip.as_deref()
              .and_then(|ip| ctx.dns_state.correlate_app_for_ip(&client_ip, ip, request_ts_ms))
      });
  ```
- Same shape applies to `proxy/http.rs:241` and `proxy/http.rs:346`.

**`src-tauri/src/proxy/mod.rs`**
- No public API change. `ProxyContext` already carries `Arc<DnsState>`.

### 5.2 Storage

- No DB migration. `app_name` column already exists in `http_requests` (`proxy/commands.rs:109`).
- `client_ip` is **not** persisted. It lives only in the in-memory `entries` deque and is used for the IP-correlation filter. The existing UI display can ignore it.
- `entries` is bounded by `max_dns_entries` from `config.rs`. Adding one field is a few extra bytes per entry — no meaningful memory impact.

### 5.3 Ordering caveat

The new IP-correlation function depends on the DNS lookup being observed **before** the connection capture. On the phone:

- iOS / Android typically DNS-resolve just-in-time, immediately before the TCP connect. The race window is single-digit milliseconds.
- If the phone cached the IP and the DNS server hasn't seen a fresh query in >5 min, the correlation misses. This is acceptable — it matches the user's "5 min, latest only" choice, and a phone that hasn't re-DNS'd in 5+ min has likely moved on.
- The existing host-string path covers the common case where the SNI matches.

## 6. Error Handling

- **`entries.lock()` poisoned**: `.lock().unwrap()` matches existing style; if it ever fires, the proxy panics, which is the same behavior as today. The correlation simply doesn't fire.
- **`peer_addr()` fails**: the `resolved_ip` is `None`, the third arm of the `.or_else()` chain is skipped, and the request is tagged as if the IP path didn't exist. Logs a single `warn!` line. (Some HTTP/1.0 clients dial the proxy without specifying an upstream port; this codepath is already edge-case.)
- **Clock skew** between the DNS server's `timestamp_ms` and the request's `request_timestamp_ms`: handled by `if request_timestamp_ms < entry.timestamp_ms { continue; }` — entries in the future are skipped, the loop proceeds. The reverse case (request far in the past) is bounded by the 5-min window.
- **Multi-device on same WiFi**: the `client_ip` filter prevents one phone's DNS lookups from tagging another phone's connections when both happen to hit the same shared CDN IP. If the proxy is between the phone and the DNS server, the `client_ip` in the DNS entry is the phone's LAN IP, which is what we want.
- **DNS rebinding / hostile clients**: out of scope. The phone is on a trusted LAN.

## 7. Testing

### 7.1 Unit tests (`src-tauri/src/dns.rs`)

- `correlate_app_for_ip_in_window` — entry at t=0, query at t=4 min → returns app.
- `correlate_app_for_ip_out_of_window` — entry at t=0, query at t=6 min → `None`.
- `correlate_app_for_ip_picks_latest` — two entries, both with the same IP, returns the later one's app.
- `correlate_app_for_ip_no_match` — entry exists but `resolved_ips` does not include our IP → `None`.
- `correlate_app_for_ip_wrong_client` — entry has `client_ip = "192.168.1.6"`, query for `192.168.1.5` → `None`.
- `correlate_app_for_ip_empty_state` — `entries` is empty → `None`.
- `correlate_app_for_ip_skips_future_entries` — entry at t=10s, query at t=5s → `None` for that entry, continues to earlier ones.

**Regression (window bump):**
- `correlate_app_in_window_at_4min` — entry at t=0, query at t=4 min → `Some` (used to return `None`).
- `correlate_app_out_of_window_at_5min1s` — entry at t=0, query at t=5 min 1 s → `None`.

### 7.2 Integration tests

- `dns_then_https_to_domain`: `record_query` for `weixin.qq.com` resolves to `["1.2.3.4"]`; simulate proxy capture with `target_host = "weixin.qq.com"`; expect `app_name = "WeChat"`.
- `dns_then_https_to_ip`: same DNS record; capture with `target_host = "1.2.3.4"` and `peer_ip = "1.2.3.4"`; expect `app_name = "WeChat"`.
- `no_dns_ip_direct`: no DNS record; capture with `target_host = "1.2.3.4"` and `peer_ip = "1.2.3.4"`; expect `app_name = None`.

**Implementation note (added after the plan was executed):** The three integration scenarios above are subsumed by the four `classify_connection` unit tests in `src-tauri/src/dns.rs` (`test_classify_connection_host_string_wins_over_ip` covers the first, `test_classify_connection_falls_back_to_ip` covers the second, `test_classify_connection_no_match_returns_none` covers the third, and `test_classify_connection_no_resolved_ip_skips_ip_path` is a fourth case). Spinning up a real `TcpStream` + `UdpSocket` for an end-to-end integration test was deferred in favor of (a) the helper-level unit tests, which exercise the same logic with a controlled `DnsState`, and (b) the manual real-device test in §7.3, which covers the full proxy flow on actual hardware.

### 7.3 Manual (real device)

- Phone on WiFi, gateway set to PC, CA installed.
- Open WeChat → UI traffic list shows requests tagged `💬 WeChat`.
- Open Alipay → `💰 Alipay`.
- Open Douyin → `🎵 Douyin`.
- Visit a non-classified site (e.g. example.com) → `Unknown` (or whatever the existing None tag is).
- Verify the count of "Unknown" requests in the traffic list decreases compared to the pre-change state, especially for HTTPS connections to known app CDNs.

## 8. Open Questions

None at this time. The user's constraints (5-min window, SNI first, hide source, synchronous at capture) are reflected in §3.1 and §3.2.

## 9. References

- `src-tauri/src/dns.rs:227` — existing `correlate_app` (host-string path).
- `src-tauri/src/proxy/https.rs:325`, `src-tauri/src/proxy/http.rs:241`, `src-tauri/src/proxy/http.rs:346` — call sites.
- `src-tauri/src/proxy/mod.rs:97-102` — `ProxyContext` with `dns_state: Arc<DnsState>`.
- `src-tauri/src/app_rules.rs` — `classify_host` (rule library).
- `proxybot-core/src/app_classifier.rs` — pure-Rust rule library (the Tauri `app_rules` re-exports it).

---

## 10. Implementation Notes (self-review, 2026-06-14)

Spec self-review pass completed after the bulk of the implementation had landed. Audit-by-grep at the time of self-review:

| Spec item | Status | Location |
|-----------|--------|----------|
| `CORRELATION_WINDOW_MS = 300_000` constant | ✅ done | `src-tauri/src/dns.rs:30` |
| `DnsEntry.client_ip` field | ✅ done | `src-tauri/src/dns.rs:66` |
| `record_query()` populates `client_ip` from UDP peer | ✅ done | `src-tauri/src/dns.rs:360-385` |
| `correlate_app_for_ip()` | ✅ done | `src-tauri/src/dns.rs:272` |
| `classify_connection()` (host → IP chain) | ✅ done | `src-tauri/src/dns.rs:294` |
| `classify_captured_request()` shared proxy helper | ✅ done | `src-tauri/src/proxy/classify.rs:11` |
| HTTP capture call site | ✅ done | `src-tauri/src/proxy/http.rs:241` |
| MapRemote capture call site | ✅ done | `src-tauri/src/proxy/http.rs:349` |
| HTTPS CONNECT call site | ✅ done | `src-tauri/src/proxy/https.rs:324` |
| **WebSocket upgrade call site** | ✅ done (this PR) | `src-tauri/src/proxy/http.rs:159` (commit `4fd5575`) |
| Unit tests for `correlate_app_for_ip` (10 cases) | ✅ done | `src-tauri/src/dns.rs:1595-1777` |
| Unit tests for `classify_connection` | ✅ done | `src-tauri/src/dns.rs` (same module) |

**Surface area actually touched:** 5 files (1 spec, 1 plan, 1 new shared helper, 3 call sites — of which 2 were already wired before the self-review, 1 was the WS-upgrade gap closed in this pass). No DB migration, no Tauri command, no UI change.

**Deviation from spec for the WS-upgrade path:** §3.1 lists three call sites. The WS-upgrade branch at `proxy/http.rs:159` was a fourth site that the original audit missed — it called `app_rules::classify_host` directly. Closed in commit `4fd5575`. The IP-fallback arm is unavailable at this site because `target_stream` is not yet established at WS-upgrade time; passing `None` for `resolved_ip` keeps the host-string arm of the chain intact.

**Validation:** `cargo test --lib` → 614 passed, 0 failed. `cargo check --lib` → 0 errors.
