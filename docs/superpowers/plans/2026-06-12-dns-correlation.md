# DNS-to-Connection Correlation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Widen the existing DNS→app correlation window from 5s to 5min and add a new correlation path that resolves app tags for IP-literal connections (e.g. CDN endpoints) by matching the connection's resolved IP against recent DNS resolutions.

**Architecture:** Replace the hard-coded `5000u64` window in `DnsState::correlate_app` with a module-level `CORRELATION_WINDOW_MS` constant. Extend `DnsEntry` with an optional `client_ip` field so the new IP-based correlation can be filtered to a single device. Add a new `correlate_app_for_ip` method that scans `entries` for the most recent DNS query whose `resolved_ips` contains the connection's IP, within the window and matching the same `client_ip`. Add a `classify_connection` helper that combines the host-string and IP paths, then refactor the three existing call sites in `proxy/https.rs` and `proxy/http.rs` to use it.

**Tech Stack:** Rust (Tauri 2), tokio (UdpSocket), existing `DnsState` / `app_rules` modules. No new dependencies. No DB schema change. No UI change.

**Working directory:** This plan assumes the implementer is at the repo root with the Tauri workspace already building (`cargo build` succeeds). All file paths are relative to the repo root.

---

## File Structure

Files modified by this plan:

| File | Responsibility | Changes |
|---|---|---|
| `src-tauri/src/dns.rs` | DNS server, query state, correlation methods | +1 const, +1 field, +2 methods, +tests, refactor 3 internal `record_query` call sites to thread `client_ip` |
| `src-tauri/src/proxy/https.rs` | HTTPS proxy capture | -3 lines, +4 lines: replace `.or_else(correlate_app)` with `.or_else(classify_connection)` and capture `resolved_ip` from upstream `TcpStream::peer_addr()` |
| `src-tauri/src/proxy/http.rs` | HTTP proxy capture (2 sites) | Same as above at lines ~241 and ~346 |

No new files. No DB migration. No frontend change. No new Tauri commands.

---

## Task 1: Add `CORRELATION_WINDOW_MS` constant and widen `correlate_app` to 5 minutes

**Files:**
- Modify: `src-tauri/src/dns.rs:227-251` (the `correlate_app` function body)
- Modify: `src-tauri/src/dns.rs:1-50` (top of file, to add the const)
- Test: `src-tauri/src/dns.rs:1008+` (existing `mod tests`)

The existing `correlate_app` hard-codes `let window_ms = 5000u64;`. We extract the value to a module-level const, then bump it to 5 minutes.

- [ ] **Step 1: Add a failing test that proves the window is at least 5 minutes**

Append to `mod tests` in `src-tauri/src/dns.rs` (the closing `}` is at the very end of the file):

```rust
    #[test]
    fn test_correlate_app_window_is_at_least_five_minutes() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        // Push a synthetic DNS entry: 4 minutes ago, classified as WeChat
        let entry = DnsEntry {
            domain: "weixin.qq.com".to_string(),
            timestamp_ms: now_ms - 4 * 60 * 1000,
            app_name: Some("WeChat".to_string()),
            app_icon: Some("\u{1F4AC}".to_string()),
            action: None,
            resolved_ips: vec!["1.2.3.4".to_string()],
        };
        state.entries.lock().unwrap().push_back(entry);

        // 4 minutes after the entry, the host should still correlate.
        assert_eq!(
            state.correlate_app("api.weixin.qq.com", now_ms),
            Some(("WeChat".to_string(), "\u{1F4AC}".to_string()))
        );
    }
```

- [ ] **Step 2: Run the test and confirm it fails (window is currently 5s)**

```bash
cargo test -p proxybot test_correlate_app_window_is_at_least_five_minutes
```

Expected output: a test failure with `assertion failed: ... == Some(("WeChat", ...))` returning `None`. The 4-minute-old entry is outside the 5-second window.

- [ ] **Step 3: Add the constant at module scope**

In `src-tauri/src/dns.rs`, immediately after the `use` block (search for `use tauri::{AppHandle, Emitter, State};` — the const goes right after the existing `use` lines), insert:

```rust
/// Time window (in milliseconds) for DNS-to-connection correlation.
/// A DNS query observed within this window of a captured request can be
/// used to infer the request's app tag when SNI/Host does not match a
/// rule directly. 5 minutes matches typical DNS TTLs for app CDNs.
pub const CORRELATION_WINDOW_MS: u64 = 300_000;
```

- [ ] **Step 4: Replace the hard-coded window in `correlate_app`**

In `src-tauri/src/dns.rs`, find the line `let window_ms = 5000u64;` inside `correlate_app` (currently at line 229) and replace it with:

```rust
        let window_ms = CORRELATION_WINDOW_MS;
```

- [ ] **Step 5: Re-run the test and confirm it passes**

```bash
cargo test -p proxybot test_correlate_app_window_is_at_least_five_minutes
```

Expected output: `1 passed`.

- [ ] **Step 6: Run the full dns test module to confirm no regressions**

```bash
cargo test -p proxybot --lib dns
```

Expected: all existing dns tests still pass, plus the new one.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/dns.rs
git commit -m "feat(dns): widen correlate_app window from 5s to 5min

Match typical DNS TTLs so connections that arrive tens of seconds
after a lookup still inherit the app tag."
```

---

## Task 2: Add `client_ip` field to `DnsEntry` and populate it in `record_query`

**Files:**
- Modify: `src-tauri/src/dns.rs:50-58` (`DnsEntry` struct)
- Modify: `src-tauri/src/dns.rs:280-298` (`record_query` body)
- Modify: `src-tauri/src/dns.rs:725`, `:741`, `:794` (3 call sites of `record_query`)
- Test: `src-tauri/src/dns.rs:1008+` (existing `mod tests`)

We need to know which client made each DNS query, so the new IP-based correlation can avoid attributing one phone's tags to another on the same WiFi. `handle_dns_query` already receives `src: SocketAddr`; we just need to thread that into `record_query` and store it on the entry.

- [ ] **Step 1: Add a failing test that checks `client_ip` is populated**

Append to `mod tests` in `src-tauri/src/dns.rs`:

```rust
    #[test]
    fn test_dns_entry_has_client_ip_field() {
        let entry = DnsEntry {
            domain: "weixin.qq.com".to_string(),
            timestamp_ms: 0,
            app_name: Some("WeChat".to_string()),
            app_icon: Some("\u{1F4AC}".to_string()),
            action: None,
            resolved_ips: vec!["1.2.3.4".to_string()],
            client_ip: Some("192.168.1.5".to_string()),
        };
        assert_eq!(entry.client_ip.as_deref(), Some("192.168.1.5"));
    }
```

- [ ] **Step 2: Run the test and confirm it fails (field does not exist)**

```bash
cargo test -p proxybot test_dns_entry_has_client_ip_field
```

Expected: compile error referencing `client_ip` on `DnsEntry`. Note this is a *compile* failure, not an assertion failure — `cargo test` exits non-zero with a `field 'client_ip' is not a member of struct 'DnsEntry'` message.

- [ ] **Step 3: Add the `client_ip` field to `DnsEntry`**

In `src-tauri/src/dns.rs`, find the `DnsEntry` struct (currently at line 50) and append a new field at the end of the struct, after `resolved_ips`:

```rust
    pub resolved_ips: Vec<String>,
    pub client_ip: Option<String>, // Source LAN IP of the DNS query
}
```

- [ ] **Step 4: Update the `record_query` function signature and body**

Find `record_query` (currently at line 280). Change its signature to accept `client_ip: &str`:

```rust
fn record_query(
    state: &DnsState,
    domain: String,
    response_ips: &[String],
    client_ip: &str,
    app_handle: &AppHandle,
) {
```

In the `DnsEntry { ... }` literal inside `record_query`, add the new field at the end:

```rust
    let entry = DnsEntry {
        domain: domain.clone(),
        timestamp_ms: timestamp_ms_val,
        app_name: app_name.clone(),
        app_icon: app_icon.clone(),
        action: action.clone(),
        resolved_ips: response_ips.to_vec(),
        client_ip: Some(client_ip.to_string()),
    };
```

- [ ] **Step 5: Update the three `record_query` call sites**

All three are in `handle_dns_query`. Each receives `src: SocketAddr` as a parameter. The pattern at each call site is `record_query(state, domain, &response_ips, app_handle);` (or with `&[]`). Change each to pass `src.ip().to_string()` as the new `client_ip` argument.

Three call sites to update (in `src-tauri/src/dns.rs`):
- Line 725 (in the hosts-file branch): change `record_query(state, domain, &response_ips, app_handle);` to `record_query(state, domain, &response_ips, &src.ip().to_string(), app_handle);`
- Line 741 (in the blocked branch): change `record_query(state, domain, &[], app_handle);` to `record_query(state, domain, &[], &src.ip().to_string(), app_handle);`
- Line 794 (in the upstream-success branch): change `record_query(state, domain, &response_ips, app_handle);` to `record_query(state, domain, &response_ips, &src.ip().to_string(), app_handle);`

- [ ] **Step 6: Update the `DnsEntry { ... }` literal in the existing `test_correlate_app_window_is_at_least_five_minutes` test**

The test from Task 1 constructs a `DnsEntry` literal. Add `client_ip` to it:

```rust
        let entry = DnsEntry {
            domain: "weixin.qq.com".to_string(),
            timestamp_ms: now_ms - 4 * 60 * 1000,
            app_name: Some("WeChat".to_string()),
            app_icon: Some("\u{1F4AC}".to_string()),
            action: None,
            resolved_ips: vec!["1.2.3.4".to_string()],
            client_ip: None,
        };
```

(Use `None` — the test exercises the host-string path, which does not consult `client_ip`.)

Also add the same `client_ip: None,` line to the test you added in Step 1 of this task, so the test compiles.

- [ ] **Step 7: Run all dns tests to confirm compile + green**

```bash
cargo test -p proxybot --lib dns
```

Expected: all dns tests pass, including the new `test_dns_entry_has_client_ip_field`.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/dns.rs
git commit -m "feat(dns): track client_ip on DnsEntry

Thread the UDP peer address through record_query so future correlation
logic can be filtered per-device."
```

---

## Task 3: Add `correlate_app_for_ip` (TDD)

**Files:**
- Modify: `src-tauri/src/dns.rs` (add method after `correlate_app`)
- Test: `src-tauri/src/dns.rs:1008+` (existing `mod tests`)

The new method takes `(client_ip, resolved_ip, request_ts_ms)` and scans `entries` for the most recent entry whose `resolved_ips` contains `resolved_ip`, whose `client_ip` matches, and whose `timestamp_ms` is within `CORRELATION_WINDOW_MS`. Returns the entry's `(app_name, app_icon)` if both are `Some`.

- [ ] **Step 1: Add the failing tests**

Append to `mod tests` in `src-tauri/src/dns.rs`:

```rust
    // ------------------------------------------------------------------
    // correlate_app_for_ip tests
    // ------------------------------------------------------------------

    fn push_entry(
        state: &DnsState,
        domain: &str,
        timestamp_ms: u64,
        client_ip: Option<&str>,
        resolved_ips: Vec<&str>,
        app_name: Option<&str>,
    ) {
        let entry = DnsEntry {
            domain: domain.to_string(),
            timestamp_ms,
            app_name: app_name.map(str::to_string),
            app_icon: app_name.map(|_| "\u{1F4AC}".to_string()),
            action: None,
            resolved_ips: resolved_ips.into_iter().map(str::to_string).collect(),
            client_ip: client_ip.map(str::to_string),
        };
        state.entries.lock().unwrap().push_back(entry);
    }

    #[test]
    fn test_correlate_app_for_ip_in_window() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        push_entry(
            &state,
            "weixin.qq.com",
            now_ms - 60_000, // 1 minute ago
            Some("192.168.1.5"),
            vec!["1.2.3.4"],
            Some("WeChat"),
        );
        let result = state.correlate_app_for_ip("192.168.1.5", "1.2.3.4", now_ms);
        assert_eq!(result, Some(("WeChat".to_string(), "\u{1F4AC}".to_string())));
    }

    #[test]
    fn test_correlate_app_for_ip_out_of_window() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        push_entry(
            &state,
            "weixin.qq.com",
            now_ms - 6 * 60 * 1000, // 6 minutes ago
            Some("192.168.1.5"),
            vec!["1.2.3.4"],
            Some("WeChat"),
        );
        assert_eq!(state.correlate_app_for_ip("192.168.1.5", "1.2.3.4", now_ms), None);
    }

    #[test]
    fn test_correlate_app_for_ip_picks_latest() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        // Older entry for Alipay
        push_entry(
            &state,
            "alipay.com",
            now_ms - 2 * 60 * 1000,
            Some("192.168.1.5"),
            vec!["1.2.3.4"],
            Some("Alipay"),
        );
        // Newer entry for WeChat at the same IP
        push_entry(
            &state,
            "weixin.qq.com",
            now_ms - 30_000,
            Some("192.168.1.5"),
            vec!["1.2.3.4"],
            Some("WeChat"),
        );
        let result = state.correlate_app_for_ip("192.168.1.5", "1.2.3.4", now_ms);
        assert_eq!(result, Some(("WeChat".to_string(), "\u{1F4AC}".to_string())));
    }

    #[test]
    fn test_correlate_app_for_ip_no_match() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        push_entry(
            &state,
            "weixin.qq.com",
            now_ms - 30_000,
            Some("192.168.1.5"),
            vec!["1.2.3.4"],
            Some("WeChat"),
        );
        // Query for a different IP
        assert_eq!(state.correlate_app_for_ip("192.168.1.5", "5.6.7.8", now_ms), None);
    }

    #[test]
    fn test_correlate_app_for_ip_wrong_client() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        push_entry(
            &state,
            "weixin.qq.com",
            now_ms - 30_000,
            Some("192.168.1.6"), // different client
            vec!["1.2.3.4"],
            Some("WeChat"),
        );
        // Phone 1.5 queries — should not see 1.6's tag
        assert_eq!(state.correlate_app_for_ip("192.168.1.5", "1.2.3.4", now_ms), None);
    }

    #[test]
    fn test_correlate_app_for_ip_empty_state() {
        let state = DnsState::new();
        assert_eq!(state.correlate_app_for_ip("192.168.1.5", "1.2.3.4", 0), None);
    }

    #[test]
    fn test_correlate_app_for_ip_skips_future_entries() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        // Entry in the future (clock skew)
        push_entry(
            &state,
            "weixin.qq.com",
            now_ms + 60_000,
            Some("192.168.1.5"),
            vec!["1.2.3.4"],
            Some("WeChat"),
        );
        // Past entry that should match
        push_entry(
            &state,
            "alipay.com",
            now_ms - 30_000,
            Some("192.168.1.5"),
            vec!["1.2.3.4"],
            Some("Alipay"),
        );
        let result = state.correlate_app_for_ip("192.168.1.5", "1.2.3.4", now_ms);
        assert_eq!(result, Some(("Alipay".to_string(), "\u{1F4AC}".to_string())));
    }
```

- [ ] **Step 2: Run the tests and confirm they all fail (method does not exist)**

```bash
cargo test -p proxybot test_correlate_app_for_ip
```

Expected: compile error `no method named 'correlate_app_for_ip' found for struct 'DnsState'`.

- [ ] **Step 3: Implement `correlate_app_for_ip`**

In `src-tauri/src/dns.rs`, immediately after the `correlate_app` method (currently ends around line 251), insert:

```rust
    /// Correlate a connection whose target is a literal IP (e.g. CDN
    /// endpoint) against recent DNS resolutions for the same client.
    /// Returns the app_name and app_icon of the most recent DNS query
    /// from `client_ip` whose `resolved_ips` includes `resolved_ip`,
    /// within `CORRELATION_WINDOW_MS` of `request_timestamp_ms`.
    pub fn correlate_app_for_ip(
        &self,
        client_ip: &str,
        resolved_ip: &str,
        request_timestamp_ms: u64,
    ) -> Option<(String, String)> {
        let entries = match self.entries.lock() {
            Ok(g) => g,
            Err(_) => return None,
        };
        for entry in entries.iter().rev() {
            // Skip entries in the future (clock skew).
            if request_timestamp_ms < entry.timestamp_ms {
                continue;
            }
            // Entries are sorted by insertion; once we pass the window,
            // older ones cannot match.
            if request_timestamp_ms - entry.timestamp_ms > CORRELATION_WINDOW_MS {
                break;
            }
            // Must be the same client.
            if entry.client_ip.as_deref() != Some(client_ip) {
                continue;
            }
            // Must include the resolved IP we're correlating against.
            if !entry.resolved_ips.iter().any(|ip| ip == resolved_ip) {
                continue;
            }
            if let (Some(name), Some(icon)) = (&entry.app_name, &entry.app_icon) {
                return Some((name.clone(), icon.clone()));
            }
        }
        None
    }
```

- [ ] **Step 4: Run the tests and confirm they all pass**

```bash
cargo test -p proxybot test_correlate_app_for_ip
```

Expected: 7 tests passed.

- [ ] **Step 5: Run the full dns test module to confirm no regressions**

```bash
cargo test -p proxybot --lib dns
```

Expected: all dns tests still pass, including the host-string and window-bump regression tests from earlier tasks.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/dns.rs
git commit -m "feat(dns): add correlate_app_for_ip for IP-literal connections

Scans entries for the most recent DNS query from the same client whose
resolved_ips contains the connection's IP, within the 5-minute window.
Used as the third arm of the classification fallback chain."
```

---

## Task 4: Add `classify_connection` helper that combines host-string and IP paths

**Files:**
- Modify: `src-tauri/src/dns.rs` (add helper after `correlate_app_for_ip`)
- Test: `src-tauri/src/dns.rs:1008+` (existing `mod tests`)

The proxy call sites today do `app_rules::classify_host(target_host).or_else(|| dns_state.correlate_app(target_host, ts))`. We want to add the IP fallback without making each call site write the full three-arm chain. The helper does the two DNS-side arms.

- [ ] **Step 1: Add a failing test for the helper**

Append to `mod tests` in `src-tauri/src/dns.rs`:

```rust
    // ------------------------------------------------------------------
    // classify_connection tests
    // ------------------------------------------------------------------

    #[test]
    fn test_classify_connection_host_string_wins_over_ip() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        // DNS entry: domain = alipay.com, resolved to 1.2.3.4
        push_entry(
            &state,
            "alipay.com",
            now_ms - 30_000,
            Some("192.168.1.5"),
            vec!["1.2.3.4"],
            Some("Alipay"),
        );
        // Connection to api.alipay.com (host-string match should win)
        let result =
            state.classify_connection("api.alipay.com", "192.168.1.5", Some("1.2.3.4"), now_ms);
        assert_eq!(result, Some(("Alipay".to_string(), "\u{1F4AC}".to_string())));
    }

    #[test]
    fn test_classify_connection_falls_back_to_ip() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        // DNS entry: domain = weixin.qq.com, resolved to 1.2.3.4
        push_entry(
            &state,
            "weixin.qq.com",
            now_ms - 30_000,
            Some("192.168.1.5"),
            vec!["1.2.3.4"],
            Some("WeChat"),
        );
        // Connection target is the IP literal directly
        let result =
            state.classify_connection("1.2.3.4", "192.168.1.5", Some("1.2.3.4"), now_ms);
        assert_eq!(result, Some(("WeChat".to_string(), "\u{1F4AC}".to_string())));
    }

    #[test]
    fn test_classify_connection_no_match_returns_none() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        // No DNS entries at all
        let result =
            state.classify_connection("unknown.com", "192.168.1.5", Some("1.2.3.4"), now_ms);
        assert_eq!(result, None);
    }

    #[test]
    fn test_classify_connection_no_resolved_ip_skips_ip_path() {
        let state = DnsState::new();
        let now_ms: u64 = 1_000_000_000_000;
        push_entry(
            &state,
            "weixin.qq.com",
            now_ms - 30_000,
            Some("192.168.1.5"),
            vec!["1.2.3.4"],
            Some("WeChat"),
        );
        // Connection without a known peer IP (None) — host path still works
        let result =
            state.classify_connection("api.weixin.qq.com", "192.168.1.5", None, now_ms);
        assert_eq!(result, Some(("WeChat".to_string(), "\u{1F4AC}".to_string())));
    }
```

- [ ] **Step 2: Run the tests and confirm they fail**

```bash
cargo test -p proxybot test_classify_connection
```

Expected: compile error `no method named 'classify_connection' found for struct 'DnsState'`.

- [ ] **Step 3: Implement `classify_connection`**

In `src-tauri/src/dns.rs`, immediately after `correlate_app_for_ip`, insert:

```rust
    /// Two-step DNS-side classification for a captured connection.
    /// Tries the host-string path first (`correlate_app`), then the
    /// resolved-IP path (`correlate_app_for_ip`) if the first missed
    /// and a `resolved_ip` is known. Used by the proxy to combine
    /// both DNS-side fallbacks into a single call.
    pub fn classify_connection(
        &self,
        target_host: &str,
        client_ip: &str,
        resolved_ip: Option<&str>,
        request_timestamp_ms: u64,
    ) -> Option<(String, String)> {
        if let Some(hit) = self.correlate_app(target_host, request_timestamp_ms) {
            return Some(hit);
        }
        if let Some(ip) = resolved_ip {
            return self.correlate_app_for_ip(client_ip, ip, request_timestamp_ms);
        }
        None
    }
```

- [ ] **Step 4: Run the tests and confirm they pass**

```bash
cargo test -p proxybot test_classify_connection
```

Expected: 4 tests passed.

- [ ] **Step 5: Run the full dns test module to confirm no regressions**

```bash
cargo test -p proxybot --lib dns
```

Expected: all dns tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/dns.rs
git commit -m "feat(dns): add classify_connection helper

Wraps correlate_app + correlate_app_for_ip into a single call so the
proxy sites don't need to chain two or_else arms."
```

---

## Task 5: Refactor the three proxy call sites to use `classify_connection` and capture `resolved_ip`

**Files:**
- Modify: `src-tauri/src/proxy/https.rs:317-330` (HTTPS capture site)
- Modify: `src-tauri/src/proxy/http.rs:236-249` (HTTP capture site #1)
- Modify: `src-tauri/src/proxy/http.rs:340-354` (HTTP capture site #2)

At each of the three call sites, the current code is roughly:

```rust
let app_info = app_rules::classify_host(target_host).or_else(|| {
    let request_ts_ms = SystemTime::now()...;
    ctx.dns_state.correlate_app(&target_host, request_ts_ms)
});
```

We replace it with a single call to `ctx.dns_state.classify_connection(...)` and pass `resolved_ip` from the upstream TcpStream's `peer_addr()`. The `peer_addr()` call requires that we capture the upstream stream somewhere accessible at the classification point. The exact binding depends on the existing code structure (the HTTPS site has `target_host` and a connected `stream` in scope; the HTTP sites have `host` and a connected upstream stream). The implementer must locate the right place to bind `peer_addr()` — typical pattern:

```rust
let resolved_ip = upstream_stream
    .as_ref()
    .and_then(|s| s.peer_addr().ok())
    .map(|a| a.ip().to_string());
```

- [ ] **Step 1: Refactor `proxy/https.rs`**

Two edits.

**Edit A — capture the resolved IP after the upstream TLS stream is created:**

Find the block in `proxy/https.rs` that creates `upstream_tls_stream` (currently at line 174) and the `tokio::io::split(...)` that consumes it (currently at line 187). Between these two, add a single line that captures the peer IP:

```rust
    let resolved_ip: Option<String> = upstream_tls_stream
        .peer_addr()
        .ok()
        .map(|a| a.ip().to_string());
```

This binding lives at the function scope and is visible at the `app_info` block in Step 1B.

**Edit B — replace the `app_info` block with the three-arm fallback:**

Find the `app_info` block in `proxy/https.rs` (currently around lines 317-330) and replace it with:

```rust
    // Classify by direct domain match first, then fall back to DNS correlation
    // (host-string, then IP).
    let app_info = {
        let request_ts_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        app_rules::classify_host(&target_host).or_else(|| {
            ctx.dns_state.classify_connection(
                &target_host,
                &client_addr.ip().to_string(),
                resolved_ip.as_deref(),
                request_ts_ms,
            )
        })
    };
    let (app_name, app_icon) = app_info
        .map(|(n, i)| (Some(n), Some(i)))
        .unwrap_or((None, None));
```

If `client_addr` is not yet in scope at the `app_info` block (the existing code defines it on a line just below), move the `let client_ip = client_addr.ip().to_string();` line (currently around line 331) up to before this block. The `app_info` block only needs `&client_addr.ip().to_string()` — it can stay inline.

- [ ] **Step 2: Verify `proxy/https.rs` compiles**

```bash
cargo check -p proxybot
```

Expected: zero errors. There may be pre-existing warnings — those are not the responsibility of this task.

- [ ] **Step 3: Refactor `proxy/http.rs` site #1 (around line 240)**

The pattern is the same as `proxy/https.rs`. Find the `app_info` block (line 236-249 in the current file). Capture the peer IP from `target_stream` (created at line 94, still in scope and not consumed — `TcpStream::peer_addr()` works directly):

```rust
let resolved_ip: Option<String> = target_stream
    .peer_addr()
    .ok()
    .map(|a| a.ip().to_string());
let app_info = {
    let request_ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    app_rules::classify_host(host).or_else(|| {
        ctx.dns_state.classify_connection(
            host,
            &client_addr.ip().to_string(),
            resolved_ip.as_deref(),
            request_ts_ms,
        )
    })
};
```

(The HTTP path has `host: &str` rather than `&target_host`. Use the variable name as it appears at the call site.)

- [ ] **Step 4: Refactor `proxy/http.rs` site #2 (around line 340)**

Same pattern as site #1. `target_stream` is in scope and not consumed. Bind the peer IP and replace the `app_info` block:

```rust
let resolved_ip: Option<String> = target_stream
    .peer_addr()
    .ok()
    .map(|a| a.ip().to_string());
let app_info = {
    let request_ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    app_rules::classify_host(host).or_else(|| {
        ctx.dns_state.classify_connection(
            host,
            &client_addr.ip().to_string(),
            resolved_ip.as_deref(),
            request_ts_ms,
        )
    })
};
```

The surrounding code passes `app_info` to `build_intercepted_request(...)` — no change to that call's signature.

- [ ] **Step 5: Verify both `proxy/http.rs` sites compile**

```bash
cargo check -p proxybot
```

Expected: zero errors.

- [ ] **Step 6: Run the full dns + proxy test suite to confirm no regressions**

```bash
cargo test -p proxybot --lib
```

Expected: all tests pass (existing correlate_app smoke test, the new tests added in Tasks 1-4, and any other lib tests).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/proxy/https.rs src-tauri/src/proxy/http.rs
git commit -m "refactor(proxy): use classify_connection and capture resolved_ip

Replaces two-arm correlate_app fallback with single classify_connection
call that also handles the IP-literal case. Captures the upstream TCP
peer address once at each capture site."
```

---

## Task 6: Final verification

**Files:** none modified

- [ ] **Step 1: Run `cargo build` for the full workspace**

```bash
cargo build
```

Expected: zero errors, zero new warnings. (Pre-existing warnings are out of scope.)

- [ ] **Step 2: Run the full test suite**

```bash
cargo test
```

Expected: all tests pass in both `proxybot` (Tauri lib) and `proxybot_core` crates.

- [ ] **Step 3: Run `cargo clippy` if available**

```bash
cargo clippy -p proxybot --no-deps -- -D warnings
```

Expected: zero new clippy warnings. (Existing warnings are out of scope; if `clippy` is not installed, skip this step.)

- [ ] **Step 4: Final commit if any cleanup was needed**

```bash
git status
# If there are uncommitted tweaks:
git add -A
git commit -m "chore: post-implementation cleanup"
```

If there are no uncommitted changes, skip this step.

---

## Manual verification (out-of-band)

The spec's §7.3 calls for manual testing on a real device. This is not a TDD task — it should be done by the user after the implementation lands.

Steps for the user (not the implementer):

1. Phone on WiFi, gateway set to PC's IP, CA installed.
2. Open WeChat → confirm requests in the ProxyBot traffic list are tagged `💬 WeChat` (not `Unknown`).
3. Open Alipay → `💰 Alipay`.
4. Open Douyin → `🎵 Douyin`.
5. Compare the count of `Unknown` (or `app_name = NULL`) requests before and after this change. A noticeable drop in `Unknown` for HTTPS connections to known app CDNs is the success signal.

If the user reports `Unknown` counts are unchanged, the most likely cause is that the phone hasn't been re-DNS'd in over 5 minutes (cached IP). Wait 5+ minutes and re-test, or reboot the phone to clear DNS cache.

---

## References

- Spec: `docs/superpowers/specs/2026-06-12-dns-correlation-design.md`
- Existing correlation method: `src-tauri/src/dns.rs:227` (pre-change line numbers)
- Call sites (pre-change): `src-tauri/src/proxy/https.rs:325`, `src-tauri/src/proxy/http.rs:241`, `src-tauri/src/proxy/http.rs:346`
- `ProxyContext` with `dns_state: Arc<DnsState>`: `src-tauri/src/proxy/mod.rs:97-102`
- Rule library: `src-tauri/src/app_rules.rs`, `proxybot-core/src/app_classifier.rs`
