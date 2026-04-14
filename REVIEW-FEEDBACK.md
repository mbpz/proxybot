# Review Feedback

## Step 3 Pass 2

**Reviewer:** Richard
**Date:** 2026-04-14

### Must Fix Verification

#### 1. Shutdown wakeup (broadcast channel interrupt)

**Status:** RESOLVED

**Analysis:**

- `start_dns_server` (line 257) creates `broadcast::channel(1)` and stores the sender in `state.shutdown_tx`
- `run_dns_server` (line 213) subscribes via `shutdown_tx.subscribe()`, obtaining a receiver
- `tokio::select!` on lines 218-242 is structured correctly:
  - Branch `_ = shutdown_rx.recv()` breaks the loop on shutdown signal
  - Branch `result = socket.recv_from(&mut buf)` handles incoming packets
  - When broadcast fires, `shutdown_rx.recv()` completes and cancels the pending `recv_from`

- `stop_dns_server` (lines 273-279) sends on broadcast BEFORE setting `running = false`, ensuring the select loop is woken before the loop condition is re-evaluated

**Race analysis:**
1. `stop_dns_server` calls `tx.send(())` - this wakes the `recv_from` operation immediately
2. `recv_from` returns with an error (operation was cancelled), but the select sees the broadcast first and breaks
3. `stop_dns_server` then sets `running = false`
4. Loop condition is checked at next iteration (or after break), sees `running == false`, exits

No race between setting `running=false` and broadcast send that could leave the loop blocked.

#### 2. Unused socket removed

**Status:** RESOLVED

**Analysis:**

`_upstream_socket` is absent from the code. The single `socket` bound to `0.0.0.0:5300` is used for both:
- Receiving queries from clients (line 222: `socket.recv_from`)
- Forwarding to upstream 8.8.8.8:53 (line 155: `socket.send_to(data, UPSTREAM_DNS)`)

This is correct because UDP is connectionless - a single UDP socket can send to any destination and receive from any source.

### Additional Observations

1. **Error handling on shutdown**: The recv_from error handler (lines 235-239) checks `state.running` before logging, preventing spurious errors during shutdown. Correct.

2. **Double-start guard**: `start_dns_server` uses `swap(true)` to detect if already running, preventing duplicate server spawns. Correct.

3. **One-shot shutdown channel**: The broadcast channel has buffer size 1, which is sufficient since only one shutdown message is ever sent per server lifecycle.

### Conclusion

**Step 3 is clear.** Both Must Fix items are properly resolved.

---

## Step 4 Review

**Reviewer:** Richard
**Date:** 2026-04-14
**Ready for Builder:** NO

---

### Must Fix

#### 1. `app_rules.rs:50` — Incorrect subdomain matching causes false positives

The condition `host == domain || host.ends_with(domain)` is wrong for subdomain matching.

Consider host `"qq.com.evil.com"` and domain `"qq.com"`:
- `host.ends_with("qq.com")` returns **true** — this is a false positive. The attacker controls `qq.com.evil.com` which is not WeChat.

A proper subdomain match requires the dot boundary: `host == domain || host.ends_with(&format!(".{domain}"))`.

**Fix:** Change line 50 from:
```rust
if host == domain || host.ends_with(domain) {
```
to:
```rust
if host == domain || host.ends_with(&format!(".{domain}")) {
```

This affects every app: WeChat, Douyin, and Alipay. All domain rules are currently vulnerable to domain-suffix spoofing.

---

### Should Fix

#### 2. `app_rules.rs` — WeChat domain coverage is thin

WeChat has many more active domains beyond the six listed. Notable gaps:
- `wechatpay.com` / `wx.tenpay.com` — WeChat Pay
- `weapp.com` — Mini programs
- `wxa.com` — WeChat mini-program infrastructure

These are significant WeChat traffic sources that would fall into "Unknown" with current rules.

#### 3. `app_rules.rs` — Douyin domain coverage is thin

Missing:
- `douyinecdn.com` — Douyin CDN
- `tiktok.com` — TikTok international (same ByteDance infrastructure)
- `bytedance.com` / `bytedance.com.cn` — ByteDance corporate

#### 4. `app_rules.rs` — Alipay domain coverage is thin

Missing:
- `antgroup.com` — Ant Group (Alipay parent)
- `mybank.com` — Alipay's bank subsidiary

---

### Cleared

1. **proxy.rs:518-521, 609-612** — `classify_host()` is called on every intercepted request in both HTTPS CONNECT and HTTP paths. App name and icon are correctly attached to the `InterceptedRequest` payload in both handlers. No regression in proxy functionality.

2. **App.tsx:258-262** — Tab filtering logic is correct: "Unknown" tab filters for `!req.app_name`, individual app tabs match `req.app_name === selectedTab`.

3. **App.tsx:265-266** — App column display is correct: shows emoji + name when available, "-" otherwise.

4. **lib.rs:3,7** — `mod app_rules` is correctly declared and imported. No issues found.

---

### Conclusion

**Step 4 is NOT clear.** The subdomain matching bug at `app_rules.rs:50` is a security issue — it causes false positives that could mislead users about what traffic belongs to which app. This must be fixed before the step passes.
