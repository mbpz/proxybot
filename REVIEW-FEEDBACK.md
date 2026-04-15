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

---

## Step 4 Pass 2

**Reviewer:** Richard
**Date:** 2026-04-14

### Confirmations

**1. Subdomain boundary fix (app_rules.rs:59)**

The fix `host == domain || host.ends_with(&format!(".{domain}"))` is **correct**.

- `host = "qq.com.evil.com"`, domain = `"qq.com"`: `"qq.com.evil.com".ends_with(".qq.com")` is `false` — no false positive.
- `host = "weixin.qq.com"`, domain = `"qq.com"`: `"weixin.qq.com".ends_with(".qq.com")` is `true` — legitimate subdomain correctly matched.
- The format string `".{domain}"` ensures a dot boundary, preventing suffix-match spoofing.

**2. False positive test coverage (app_rules.rs:86-92)**

`test_false_positive_subdomain` explicitly covers the attack case:
```rust
assert_eq!(classify_host("qq.com.evil.com"), None);
assert_eq!(classify_host("weixin.qq.com.evil.com"), None);
assert_eq!(classify_host("douyin.com.fake.com"), None);
assert_eq!(classify_host("alipay.com.phishing.com"), None);
```
These four assertions verify the fix is tested. All four would have **failed** before the fix and **pass** after.

**3. classify_host() in both paths**

- HTTPS CONNECT (proxy.rs:518): `app_rules::classify_host(&target_host)` — called after MITM handshake with `target_host` from the CONNECT request.
- Transparent HTTP (proxy.rs:609): `app_rules::classify_host(host)` — called with the resolved host from headers or DIOCNATLOOK fallback.

Both paths attach `app_name`/`app_icon` to `InterceptedRequest` and emit it to the frontend. Confirmed present in both code paths.

**4. App.tsx InterceptedRequest fields**

```typescript
interface InterceptedRequest {
  app_name?: string;
  app_icon?: string;
}
```
Both fields are optional (`?:`), matching the Rust side `Option<String>` serialized as nullable fields. Correct.

**5. "Unknown" tab filter**

```typescript
if (selectedTab === "all") return true;
if (selectedTab === "Unknown") return !req.app_name;
return req.app_name === selectedTab;
```
When `app_name` is `undefined`/`null`, `!req.app_name` is `true` — requests with no app classification land in the "Unknown" tab. Correct.

---

### Conclusion

**Step 4 is clear.** All five items verified correct. The Must Fix from Pass 1 (subdomain boundary bug) is resolved, tests cover the false positive case, and both frontend and backend handle the "Unknown" app case correctly.

---

## Step 5 Pass 2

**Reviewer:** Richard
**Date:** 2026-04-15

### All Four Must-Fix Items Verified

#### 1. MITM WebSocket relay — Upgrade request forwarded to upstream

**Lines 751-755:** `upstream_tls_stream.write_all(&http_data).await` forwards the browser's HTTP upgrade request to the upstream server before any response is sent to the browser.

**Flow confirmed:**
1. Browser sends WebSocket upgrade request to proxy
2. Proxy forwards upgrade request to upstream (line 752)
3. Proxy reads 101 response from upstream (lines 758-765)
4. Proxy sends 101 to browser (lines 881-885)
5. Proxy starts frame relay (line 888)

Per RFC 6455 MITM proxy behavior. **FIX VERIFIED.**

#### 2. 101 response relay — 101 from upstream, not locally generated

**Lines 774-776:** The code reads the response from upstream into `upstream_response` and checks `response_str.starts_with("HTTP/1.1 101")`.

**Lines 865-879:** The 101 sent to the browser is constructed from:
- Upstream's `Sec-WebSocket-Protocol` if present (line 860)
- `Sec-WebSocket-Accept` computed from client's key (line 863) — correct per RFC 6455

The proxy does NOT generate a 101 independently; it is derived from upstream's response. **FIX VERIFIED.**

#### 3. Sec-WebSocket-Protocol — Properly included when negotiated

**Lines 854-860:** Upstream's protocol extracted, fallback to client's protocol:
```rust
let upstream_protocol = response_str.lines()
    .find(|line| line.starts_with("Sec-WebSocket-Protocol:"))
    .map(|line| line.trim_start_matches("Sec-WebSocket-Protocol:").trim().to_string());
let final_protocol = upstream_protocol.or(ws_protocol);
```

**Lines 874-877:** Included in 101 response when present:
```rust
if let Some(ref proto) = final_protocol {
    upgrade_response.push_str(&format!("Sec-WebSocket-Protocol: {}\r\n", proto));
}
```

Per RFC 6455 Section 4.1 (server must echo protocol if accepting). **FIX VERIFIED.**

#### 4. base64 crate — Correctly imported and used

**Line 27:** `use base64::Engine;`
**Line 111:** `base64::engine::general_purpose::STANDARD.encode(data)`

`compute_ws_accept_key` (line 107) calls `base64_encode(&result)`, which uses the standard RFC 4648 alphabet. **FIX VERIFIED.**

---

### Conclusion

**Step 5 is clear.** All four Must-Fix items from Pass 1 are resolved:
1. Upgrade request forwarded to upstream before 101
2. 101 relay from upstream to browser
3. Sec-WebSocket-Protocol negotiated correctly
4. base64 crate replaces custom encoder

No remaining blockers.

---

## Step 5 Review

**Reviewer:** Richard
**Date:** 2026-04-15
**Ready for Builder:** NO

---

### Must Fix

#### 1. `proxy.rs:759-781` — WebSocket upgrade request never forwarded to upstream server

This is a fundamental protocol error that will break WebSocket functionality for WSS connections.

**The bug:**

After TLS handshakes complete (lines 680-740), the proxy reads the HTTP request from the browser (`http_n`, lines 743-756) and checks if it's a WebSocket upgrade (`is_websocket_upgrade`, line 759). If it is, the proxy sends a 101 Switching Protocols response directly to the browser (lines 764-777) and then calls `handle_websocket_relay`.

The problem: the browser's HTTP WebSocket upgrade request is **never forwarded to the upstream server**. The `upstream_tls_stream` is a live TLS connection to the target server, but the proxy never writes the HTTP upgrade request to it. The upstream server has no idea this is supposed to be a WebSocket connection — it just sees an open TLS connection with random bytes (WebSocket frames) arriving.

**Expected RFC 6455 behavior for a MITM proxy:**
1. Proxy receives WebSocket upgrade request from browser
2. Proxy forwards the HTTP upgrade request to the upstream server
3. Proxy receives 101 response from upstream server
4. Proxy forwards 101 response to browser
5. Browser and server complete handshake (both now know it's WebSocket)
6. Proxy relays WebSocket frames bidirectionally

**What the code actually does:**
1. Proxy receives WebSocket upgrade request from browser
2. Proxy sends 101 to browser immediately (upstream never contacted)
3. Proxy starts relaying WebSocket frames

The upstream server sees TLS traffic with WebSocket frames but has not agreed to the WebSocket protocol. This will cause:
- The server to potentially misinterpret WebSocket frame bytes as application data
- Server responses that are not proper WebSocket frames
- Connection failures or silent data corruption

**How to fix:**

After detecting WebSocket upgrade at line 759, instead of immediately sending 101 to the browser, the proxy must:
1. Write the HTTP upgrade request (stored in `http_data`) to `upstream_tls_stream`
2. Read the HTTP response from `upstream_tls_stream`
3. Check if it's a 101 response
4. If yes, send 101 to browser and start frame relay
5. If no, fall back to the blind relay path

The upstream connection currently uses `tokio_rustls::client::TlsStream` which is a raw TLS stream. The proxy needs to perform the HTTP upgrade handshake with the upstream server before completing the handshake with the browser.

---

### Should Fix

#### 2. `proxy.rs:764-771` — 101 response omits `Sec-WebSocket-Protocol` header when offered by client

RFC 6455 Section 4.1 requires that if the client includes `Sec-WebSocket-Protocol` in its request and the server wishes to accept it, the server MUST include the same protocol token in its 101 response.

The current 101 response (lines 764-771) only includes `Upgrade`, `Connection`, and `Sec-WebSocket-Accept`. If a client sends `Sec-WebSocket-Protocol: chat`, the response should be:

```
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: <accept>
Sec-WebSocket-Protocol: chat

```

**Impact:** Some WebSocket clients or servers may fail or behave unexpectedly if protocol negotiation is not completed correctly. This is a compliance issue but may not block basic WSS functionality.

#### 3. `proxy.rs:100-123` — Custom base64 implementation

The `base64_encode` function is a hand-rolled implementation. While the RFC 6455 formula itself appears correct (SHA1 of key + magic GUID, base64 encoded), the custom base64 encoder has not been verified against the standard alphabet (RFC 4648).

**Recommendation:** Use the `base64` crate from crates.io instead of a custom implementation. This eliminates the risk of encoding bugs that could cause handshake failures.

---

### Cleared

1. **`is_websocket_upgrade()` (lines 67-87)** — Correctly detects WebSocket upgrade by checking for `Upgrade: websocket` and `Connection: Upgrade` headers using case-insensitive comparison. Correctly extracts `Sec-WebSocket-Key`. No issues found.

2. **`compute_ws_accept_key()` (lines 91-98)** — RFC 6455 formula is correct: `base64(SHA1(key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))`. The magic GUID string is exactly as specified in RFC 6455. No issues found.

3. **`handle_websocket_relay()` frame handling** — Ping frames correctly cause Pong response directly to sender rather than being relayed (lines 456-462, 543-549). Close frames are relayed to peer and then the local connection is closed (lines 448-455, 535-542). Text and Binary frames are emitted as `intercepted-wss` events and relayed. This is correct per RFC 6455.

4. **tokio-tungstenite integration** — `WebSocketStream::from_raw_socket` is called with correct `Role::Server` for browser stream and `Role::Client` for upstream stream (lines 422-424). The bidirectional relay using `tokio::select!` with mpsc channels (lines 430-605) is a valid approach for concurrent bidirectional relay.

5. **WssMessage event emission** — `intercepted-wss` event is emitted with all required fields: `id`, `timestamp`, `host`, `direction` ("up"/"down"), `size`, `content`, `app_name`, `app_icon` (lines 475-485, 562-572). App classification via `app_rules::classify_host(&target_host)` is correctly applied at the start of the relay.

6. **App.tsx WSS tab** — WSS messages are correctly stored in a separate state (`wssMessages`, max 200), displayed in a separate tab section, filtered by app tabs (All/WeChat/Douyin/Alipay/Unknown), with direction shown as arrow (up/down) and content preview truncated to 50 characters. The tab is separated from HTTP requests. No issues found.

7. **`WssMessage` Rust struct (lines 49-58) and TypeScript interface (App.tsx lines 19-28)** — Both match exactly with the same field names and types (with Rust `Option<String>` becoming TypeScript `?:`). No issues found.

8. **Upstream TLS handshake (lines 701-737)** — Client TLS config correctly uses `NoVerification` for MITM mode. SNI is correctly set via `Box::leak`. TLS connection to upstream is established before WebSocket upgrade is checked. This portion is correct.

---

### Conclusion

**Step 5 is NOT clear.** The WebSocket upgrade request is not forwarded to the upstream server, causing a protocol error. The proxy sends a 101 response to the browser without contacting the upstream server about the upgrade. This will cause WSS connections to fail or malfunction. This must be fixed before the step passes.

---

## Step 6 Review

**Reviewer:** Richard
**Date:** 2026-04-15
**Ready for Builder:** YES

---

### Must Fix

None.

---

### Should Fix

#### 1. `proxy.rs:1460-1461` — `get_request_detail` is dead code

The Tauri command `get_request_detail(id)` is implemented and registered in `lib.rs:37`, but the frontend never calls it. The detail panel uses `selectedRequest` directly from the `intercepted-request` event payload (App.tsx lines 312-394). The command retrieves from `REQUEST_STORE` which is populated correctly, but nothing consumes it.

This is not a bug — the panel works because the event payload contains all fields. However, the command exists without purpose. If `REQUEST_STORE` is ever cleared (it is not currently), the panel would still show stale data from the event payload.

**Recommendation:** Either wire `get_request_detail` into the panel's tab switching (call it when switching to Headers or Body tab to get the latest data), or remove it. For now, this is minor.

---

### Escalate to Architect

None.

---

### Cleared

1. **REQUEST_STORE thread safety (proxy.rs:38-39, 199-201, 1066-1067, 1164-1165)** — `LazyLock<DashMap<String, InterceptedRequest>>` is correctly used for global concurrent storage. `store_request(req.clone())` is called BEFORE `emit("intercepted-request", ...)` in both the HTTPS CONNECT blind relay path (line 1066) and the HTTP path (line 1164). DashMap's internal locking handles concurrent access between the Tauri main thread and tokio workers. `get_request_detail` clones on return. All correct.

2. **Body capture — 10KB cap and UTF-8 fallback (proxy.rs:151-160, 189-195)** — `decode_body` truncates to `MAX_BODY_SIZE` (10 * 1024) before calling `String::from_utf8`. If UTF-8 fails, falls back to `[Binary N bytes]`. Empty body results in `None` (shown as "(no body)" in UI). In `handle_http` (line 1140), `extract_response_body` is applied before `decode_body` via `.map()`. All correct.

3. **Header parsing (proxy.rs:163-186, 141-148)** — `parse_response_headers` finds `\r\n\r\n` boundary via `data.windows(4).position(...)`, splits header block by `\n`, trims trailing `\r` from each line, extracts `name: value` pairs using `String::from_utf8_lossy` for safe conversion, and calls `format_headers` to produce `"Name: value\r\n..."` format. Returns `None` if no `\r\n\r\n` found. Correct.

4. **`get_request_detail` lookup (proxy.rs:1460-1461)** — Returns `REQUEST_STORE.get(&id).map(|entry| entry.value().clone())`. Correctly returns `Option<InterceptedRequest>` with a cloned copy.

5. **UI detail panel — three tabs and close behavior (App.tsx:312-394, App.css:533-639)** — Detail panel renders with three tabs (General/Headers/Body), tab switching via `detailTab` state, overlay click handler `onClick={() => setSelectedRequest(null)}` closes the panel, panel `onClick={(e) => e.stopPropagation()}` prevents close when clicking inside, close button calls `setSelectedRequest(null)`. CSS: `.detail-panel-overlay` is fixed positioning with flex-end alignment (slides in from right). `.detail-tab.tab-active` uses `color: #0071e3; border-bottom-color: #0071e3`. All correct.

6. **No regression in request event emission (proxy.rs:1066-1067, 1164-1165)** — In both the HTTPS CONNECT blind relay path and HTTP path, `store_request(req.clone())` is called before `ctx.app_handle.emit("intercepted-request", &req)`. The `InterceptedRequest` struct fields are all populated before storage. No existing request event emission is broken.

---

### Conclusion

**Step 6 is clear.** All six focus areas pass. DashMap is used correctly with proper store-before-emit ordering, body capture respects the 10KB limit and UTF-8 fallback, header parsing handles the HTTP format correctly, `get_request_detail` correctly retrieves from the store, the UI panel has correct tab structure and close behavior, and no regression in existing request event emission. The `get_request_detail` command is unused by the frontend but is not causing any bug.
