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
