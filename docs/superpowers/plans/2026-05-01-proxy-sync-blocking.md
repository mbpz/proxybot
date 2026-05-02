# Proxy Sync Blocking - Breakpoint Trigger Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement proxy sync blocking so that when a rule triggers `RuleAction::Breakpoint`, the proxy pauses and waits for TUI user decision before proceeding.

**Architecture:** Add bidirectional breakpoint channel between proxy and TUI. Proxy evaluates rules per-request, sends breakpoint requests to TUI, and awaits decisions before continuing. TUI receives breakpoint events, shows paused request, and signals Go/Cancel back to proxy.

**Tech Stack:** Rust, tokio channels, Arc<RulesEngine>

---

## File Structure

```
src-tauri/src/
├── proxy.rs              # ProxyContext, run_proxy, handle_client, breakpoint channel
├── rules.rs              # RuleAction::Breakpoint(BreakpointTarget)
└── bin/
    └── proxybot-tui.rs  # TUI breakpoint receiver and decision handler
```

---

## Task 1: Add BreakpointChannel types and ProxyContext extension

**Files:**
- Modify: `src-tauri/src/proxy.rs`

- [ ] **Step 1: Add BreakpointRequest and BreakpointDecision types at top of proxy.rs**

After the existing struct definitions (around line 68), add:

```rust
/// Breakpoint target - which request/response to pause on
#[derive(Clone, Debug)]
pub enum BreakpointTarget {
    Request,
    Response,
    Both,
}

/// Request sent from proxy to TUI when breakpoint triggers
#[derive(Clone, Debug)]
pub struct BreakpointRequest {
    pub request: InterceptedRequest,
    pub target: BreakpointTarget,
}

/// Decision from TUI to proxy after user handles breakpoint
#[derive(Clone, Debug)]
pub enum BreakpointDecision {
    Proceed,              // Continue with original/modified request
    Modify(InterceptedRequest), // Continue with modified request
    Drop,                 // Cancel the request
}
```

- [ ] **Step 2: Add breakpoint_tx to ProxyContext**

Modify `ProxyContext` struct (line 69):

```rust
struct ProxyContext {
    event_tx: broadcast::Sender<InterceptedRequest>,
    breakpoint_tx: tokio::sync::mpsc::Sender<BreakpointRequest>,
    #[allow(dead_code)]
    cert_manager: Arc<CertManager>,
    dns_state: Arc<DnsState>,
    db_state: Arc<DbState>,
    rules_engine: Arc<RulesEngine>,
}
```

- [ ] **Step 3: Update run_proxy to accept breakpoint_tx**

Modify `run_proxy` signature and ProxyContext creation:

```rust
async fn run_proxy(
    event_tx: broadcast::Sender<InterceptedRequest>,
    breakpoint_tx: tokio::sync::mpsc::Sender<BreakpointRequest>,
    cert_manager: Arc<CertManager>,
    dns_state: Arc<DnsState>,
    db_state: Arc<DbState>,
    rules_engine: Arc<RulesEngine>,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    // ...
    let ctx = ProxyContext {
        event_tx: event_tx.clone(),
        breakpoint_tx,      // NEW
        cert_manager: cert_manager.clone(),
        dns_state: dns_state.clone(),
        db_state: db_state.clone(),
        rules_engine,        // NEW
    };
}
```

- [ ] **Step 4: Update start_proxy_core signature and call**

```rust
pub fn start_proxy_core(
    cert_manager: Arc<CertManager>,
    dns_state: Arc<DnsState>,
    db_state: Arc<DbState>,
    rules_engine: Arc<RulesEngine>,
) -> Result(
    (broadcast::Receiver<InterceptedRequest>, tokio::sync::mpsc::Receiver<BreakpointRequest>),
    tokio::sync::oneshot::Sender<()>
), String> {
    // ...
    let (bp_tx, bp_rx) = tokio::sync::mpsc::channel(100);
    // pass bp_tx to run_proxy, return bp_rx alongside event_rx
}
```

- [ ] **Step 5: Build verification**

Run: `cargo build --lib 2>&1 | head -40`
Expected: SUCCESS (or errors to fix)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/proxy.rs
git commit -m "feat(breakpoint): add BreakpointChannel types and ProxyContext extension"
```

---

## Task 2: Add rule evaluation in handle_client

**Files:**
- Modify: `src-tauri/src/proxy.rs`

- [ ] **Step 1: Add import for RuleAction at top of proxy.rs**

Find the imports section and add:
```rust
use crate::rules::{RuleAction, BreakpointTarget};
```

- [ ] **Step 2: Add rule evaluation in handle_client before processing**

In `handle_client` (line ~1089), after parsing headers and before connecting to target:

```rust
// Evaluate rules for this host
let host_header = headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("Host"));
if let Some((_, host_value)) = host_header {
    let (host_part, _) = host_value.split_once(':').unwrap_or((host_value, ""));
    let host = host_part.trim();
    if let Some(RuleAction::Breakpoint(target)) = ctx.rules_engine.match_host(host, None) {
        // Create a request object for the breakpoint
        let req = InterceptedRequest {
            id: request_id.clone(),
            timestamp: timestamp_now(),
            method: method.to_string(),
            scheme: "http".to_string(),
            host: host.to_string(),
            path: path.to_string(),
            req_headers: headers.clone(),
            req_body: Some(body_to_string(body)),
            resp_headers: Vec::new(),
            resp_body: None,
            status: None,
            latency_ms: 0,
            device_id: device_ctx.as_ref().map(|d| d.device_id),
            app_name: device_ctx.as_ref().map(|d| d.device_name.clone()),
            device_name: None,
            is_websocket: false,
            ws_frames: None,
        };

        // Send breakpoint request and wait for decision
        let (decision_tx, decision_rx) = tokio::sync::oneshot::channel();
        if ctx.breakpoint_tx.send(BreakpointRequest {
            request: req.clone(),
            target: target.clone(),
        }).await.is_err() {
            // TUI not listening, proceed normally
        } else {
            match decision_rx.await {
                Ok(BreakpointDecision::Drop) => {
                    log::info!("Breakpoint: request dropped by user");
                    return;
                }
                Ok(BreakpointDecision::Modify(mut modified_req)) => {
                    // Replace method/path/headers with modified values
                    let modified_method = modified_req.method.clone();
                    let modified_path = modified_req.path.clone();
                    method = Box::leak(modified_method.into_boxed_str());
                    path = Box::leak(modified_path.into_boxed_str());
                    // Update headers
                }
                Ok(BreakpointDecision::Proceed) => {
                    // Continue with original request
                }
                Err(_) => {
                    // TUI dropped, proceed
                }
            }
        }
    }
}
```

- [ ] **Step 3: Build verification**

Run: `cargo build --lib 2>&1 | head -40`
Expected: SUCCESS (or errors to fix - likely rule evaluation happens in `handle_http` not `handle_client`)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/proxy.rs
git commit -m "feat(breakpoint): add rule evaluation in handle_client for breakpoint trigger"
```

---

## Task 3: Update TUI main loop to handle breakpoint decisions

**Files:**
- Modify: `src-tauri/src/bin/proxybot-tui.rs`

- [ ] **Step 1: Update start_proxy to return breakpoint_rx**

Modify `start_proxy` function:

```rust
fn start_proxy(
    app: &TuiApp,
) -> Result<(
    tokio::sync::broadcast::Receiver<InterceptedRequest>,
    tokio::sync::mpsc::Receiver<proxybot_lib::proxy::BreakpointRequest>,
), String> {
    // ...
    let (event_rx, bp_rx, shutdown_tx) = start_proxy_core(
        app.cert_manager.clone(),
        app.dns_state.clone(),
        app.db_state.clone(),
        app.rules_engine.clone(),
    )?;
    *app.shutdown_tx.lock().unwrap() = Some(shutdown_tx);
    Ok((event_rx, bp_rx))
}
```

- [ ] **Step 2: Add breakpoint receiver task in main loop**

After starting proxy (around line 135), add:

```rust
let breakpoint_rx = match start_proxy(app) {
    Ok((event_rx, bp_rx)) => {
        Some(bp_rx)
    }
    Err(e) => { /* error handling */ }
};
```

Create a task to handle breakpoint events:

```rust
let bp_rx = breakpoint_rx.expect("breakpoint receiver");
let bp_tx = app.breakpoint_decision_tx.clone();
tokio::spawn(async move {
    while let Some(bp_req) = bp_rx.recv().await {
        // Set breakpoint state in app
        app.traffic.breakpoint.queue.push(bp_req.request.clone());
        app.traffic.breakpoint.mode = match bp_req.target {
            proxybot_lib::proxy::BreakpointTarget::Request =>
                proxybot_lib::tui::BreakpointMode::RequestPaused,
            proxybot_lib::proxy::BreakpointTarget::Response =>
                proxybot_lib::tui::BreakpointMode::ResponsePaused,
            proxybot_lib::proxy::BreakpointTarget::Both =>
                proxybot_lib::tui::BreakpointMode::RequestPaused,
        };
        app.traffic.breakpoint.current_edit = Some(bp_req.request);

        // Wait for user decision via breakpoint_go/breakpoint_cancel
        // The decision_tx is stored in app state
    }
});
```

- [ ] **Step 3: Add breakpoint_decision_tx to TuiApp**

In `src-tauri/src/tui/mod.rs`, add to `TuiApp`:

```rust
pub struct TuiApp {
    // ... existing fields
    pub breakpoint_decision_tx: tokio::sync::mpsc::Sender<BreakpointDecision>,
}
```

- [ ] **Step 4: Update InputAction::BreakpointGo to signal decision**

In proxybot-tui.rs, update `BreakpointGo` handling:

```rust
InputAction::BreakpointGo => {
    use proxybot_lib::tui::BreakpointMode;
    // Send Proceed decision
    if let Some(ref tx) = app.breakpoint_decision_tx {
        let _ = tx.send(BreakpointDecision::Proceed).await;
    }
    // ... existing queue clearing
}
```

- [ ] **Step 5: Update InputAction::BreakpointCancel to signal Drop**

```rust
InputAction::BreakpointCancel => {
    use proxybot_lib::tui::BreakpointMode;
    // Send Drop decision
    if let Some(ref tx) = app.breakpoint_decision_tx {
        let _ = tx.send(BreakpointDecision::Drop).await;
    }
    // ... existing queue clearing
}
```

- [ ] **Step 6: Build verification**

Run: `cargo build --bin proxybot-tui 2>&1 | head -50`
Expected: SUCCESS or errors to fix

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/bin/proxybot-tui.rs src-tauri/src/tui/mod.rs
git commit -m "feat(breakpoint): TUI handles breakpoint decisions via decision channel"
```

---

## Task 4: Integration test

- [ ] **Step 1: Add a breakpoint rule**

```bash
# In ~/.proxybot/rules/ or via TUI
# Add a rule: host matches "example.com" -> breakpoint
```

- [ ] **Step 2: Build and run**

```bash
cd src-tauri
cargo build --bin proxybot-tui --release
./target/release/proxybot-tui
```

- [ ] **Step 3: Test flow**

1. Start proxy (r)
2. Make request to breakpoint-matched host (e.g., from device)
3. Request should pause at breakpoint
4. Press g to send, c to cancel
5. Check proxy behavior matches decision

---

## Implementation Checkpoint

| Task | Status |
|------|--------|
| Task 1: ProxyContext + Channel types | ⬜ |
| Task 2: Rule evaluation in handle_client | ⬜ |
| Task 3: TUI breakpoint decision handler | ⬜ |
| Task 4: Integration test | ⬜ |