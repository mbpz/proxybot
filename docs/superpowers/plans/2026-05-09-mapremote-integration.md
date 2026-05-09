# MapRemote 规则集成实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 集成 MapRemote 规则到 `handle_http()` - 规则匹配后转发请求到远程目标

**Architecture:** `handle_http()` 入口处调用同步的 `apply_request_rule()`，返回 `RuleApplication` 枚举。`MapRemote` variant 携带目标信息，由调用方在 `handle_http()` 中调用 `forward_map_remote()` 处理网络请求。

**Tech Stack:** Rust, tokio, rustls, hyper

---

## 文件变更概览

- **Modify:** `src-tauri/src/proxy.rs`
  - 修改 `RuleApplication` 枚举 (新增 `MapRemote` variant)
  - 修改 `apply_request_rule()` (同步化，返回 `RuleApplication::MapRemote` 而非执行网络请求)
  - 修改 `handle_http()` (入口处调用规则检查，处理 `MapRemote` case)
  - 移除死代码: `RuleResponse`, `build_map_local_response`, `build_http_response`, `parse_header_object`, `classify_request`

---

## Task 1: 修改 RuleApplication 枚举

**Files:**
- Modify: `src-tauri/src/proxy.rs:883-893`

- [ ] **Step 1: 查看当前 RuleApplication 定义**

```rust
// 当前定义 (line 883-893)
enum RuleApplication {
    Continue {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
    Respond {
        response_buf: Vec<u8>,
    },
}
```

- [ ] **Step 2: 替换为新定义**

```rust
enum RuleApplication {
    Continue {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
    MapRemote {
        target: RemoteTarget,
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
}
```

- [ ] **Step 3: 运行编译检查**

```bash
cargo check 2>&1 | grep -A5 "RuleApplication"
```

Expected: 编译错误 - `Respond` variant removed, code using it will fail

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/proxy.rs
git commit -m "refactor(proxy): update RuleApplication for MapRemote integration"
```

---

## Task 2: 修改 apply_request_rule() - 同步化

**Files:**
- Modify: `src-tauri/src/proxy.rs:905-1051`

- [ ] **Step 1: 查看当前 apply_request_rule 签名和 MapRemote 分支**

Current MapRemote handling (lines 970-981):
```rust
RuleAction::MapRemote(target) => {
    let remote = parse_remote_target(&target)?;
    let response_buf = forward_map_remote(
        &ctx.cert_manager,
        &remote,
        method,
        path,
        headers,
        body,
    )
    .await?;
    Ok(RuleApplication::Respond { response_buf })
}
```

- [ ] **Step 2: 修改 MapRemote 分支 - 移除网络调用**

```rust
RuleAction::MapRemote(target) => {
    let remote = parse_remote_target(&target)?;
    Ok(RuleApplication::MapRemote {
        target: remote,
        method: method.to_string(),
        path: path.to_string(),
        headers: headers.to_vec(),
        body: body.to_vec(),
    })
}
```

- [ ] **Step 3: 修改函数签名 - 移除 async**

```rust
// 从
async fn apply_request_rule(
// 改为
fn apply_request_rule(
```

- [ ] **Step 4: 移除所有 .await 调用**

在 `apply_request_rule` 内部，MapRemote 分支不再 await。检查其他分支（Reject、MapLocal、Breakpoint）是否也有 await，如有需要调整。

- [ ] **Step 5: 运行编译检查**

```bash
cargo check 2>&1 | head -50
```

Expected: 编译错误 - `RemoteTarget` 在 `MapRemote` variant 中需要可用

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/proxy.rs
git commit -m "refactor(proxy): make apply_request_rule sync, return MapRemote variant"
```

---

## Task 3: 修改 handle_http() - 集成规则检查

**Files:**
- Modify: `src-tauri/src/proxy.rs:1353-1556`

- [ ] **Step 1: 查看 handle_http 入口结构**

当前结构 (line 1365-1428):
```rust
async fn handle_http(...) {
    let target_addr = format!("{}:{}", host, port);
    log::info!("HTTP {} {} from {}", method, path, client_addr);
    let start = std::time::Instant::now();

    // Evaluate rules for breakpoint (line 1370)
    let breakpoint_override = if let Some(RuleAction::Breakpoint(target)) = ctx.rules_engine.match_host(host, None) {
        ...
    } else {
        None
    };

    let mut target_stream = TcpStream::connect(&target_addr).await
        .map_err(|e| format!("Failed to connect to {}: {}", target_addr, e))?;
    // ... 继续正常流程
}
```

- [ ] **Step 2: 在 target_addr 之前插入规则检查**

在 `log::info` 之后、`target_addr` 格式化之前添加：

```rust
    // Apply rules (MapRemote etc)
    match apply_request_rule(
        &ctx.rules_engine,
        client_addr,
        if port == 443 { "https" } else { "http" },
        host,
        method,
        path,
        headers,
        body,
    )? {
        RuleApplication::Continue { method: rule_method, path: rule_path, headers: rule_headers, body: rule_body } => {
            // Use rule-modified data for subsequent handling
        }
        RuleApplication::MapRemote { target, method: rule_method, path: rule_path, headers: rule_headers, body: rule_body } => {
            // Connect to remote target instead
            let response_buf = forward_map_remote(
                &ctx.cert_manager,
                &target,
                &rule_method,
                &rule_path,
                &rule_headers,
                &rule_body,
            ).await?;

            client_stream.write_all(&response_buf).await
                .map_err(|e| format!("Write MapRemote response failed: {}", e))?;

            // Record the request with MapRemote info
            let latency = start.elapsed().as_millis() as u64;
            let (status, resp_headers, resp_body) = parse_http_response(&response_buf)
                .unwrap_or((0u16, Vec::new(), Vec::new()));
            let req = build_intercepted_request(
                rule_method,
                if port == 443 { "https" } else { "http" }.to_string(),
                host.to_string(),
                rule_path,
                rule_headers,
                &rule_body,
                &response_buf,
                latency,
                client_addr,
                device_ctx.clone(),
                None,
            );
            emit_and_record(&ctx, req);

            return Ok(());
        }
    }
```

- [ ] **Step 3: 变量声明调整**

因为 `method`, `path`, `headers`, `body` 可能被规则修改，需要用新变量接收：

```rust
    let (use_method, use_path, use_headers, use_body) = match apply_request_rule(...) {
        RuleApplication::Continue { method, path, headers, body } => (method, path, headers, body),
        RuleApplication::MapRemote { target, method, path, headers, body } => {
            // 处理 MapRemote...
        }
    };
```

然后后续代码使用 `use_method`, `use_path` 等。

- [ ] **Step 4: 移除旧的 breakpoint 规则处理**

旧的 breakpoint 处理 (line 1370-1427) 中的规则匹配部分可以移除或简化，因为 `apply_request_rule` 已经处理了所有规则包括 Breakpoint。

**注意**: 需要保留 breakpoint 的 `decision_rx.await` 逻辑，因为它需要用户交互。这部分保持独立。

- [ ] **Step 5: 运行编译检查**

```bash
cargo check 2>&1 | head -80
```

Expected: 编译错误 - `emit_and_record` 未定义，需要实现

- [ ] **Step 6: 确认 emit_and_record 函数存在**

`emit_and_record` 已定义于 line 813，`build_intercepted_request` 已定义于 line 836。这些函数之前标记为未使用，现在将被启用。

- [ ] **Step 7: 再次编译检查**

```bash
cargo check 2>&1 | head -50
```

Expected: 应该通过（或只有警告）

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/proxy.rs
git commit -m "feat(proxy): integrate MapRemote rules into handle_http"
```

---

## Task 4: 移除死代码

**Files:**
- Modify: `src-tauri/src/proxy.rs`

- [ ] **Step 1: 确认死代码位置**

**注意**: `emit_and_record` (line 813) 和 `build_intercepted_request` (line 836) 现在被使用，**不要删除**。

```rust
// Line ~534
struct RuleResponse { ... }  // 未使用 - 删除

// Line ~583
fn infer_content_type(path: &Path) -> &'static str { ... }  // 未使用 - 删除

// Line ~595
fn expand_user_path(path: &str) -> PathBuf { ... }  // 未使用 - 删除

// Line ~604
fn render_mock_template(template: &str, req: &InterceptedRequest) -> String { ... }  // 未使用 - 删除

// Line ~616
fn parse_header_object(value: &serde_json::Value) -> Vec<(String, String)> { ... }  // 未使用 - 删除

// Line ~630
fn build_map_local_response(target: &str, req: &InterceptedRequest) -> Result<RuleResponse, String> { ... }  // MapLocal 暂不实现 - 删除

// Line ~682
fn build_http_response(response: &RuleResponse) -> Vec<u8> { ... }  // 未使用 - 删除

// Line ~895
fn classify_request(ctx: &ProxyContext, host: &str) -> Option<(String, String)> { ... }  // 未使用 - 删除
```

**保留** (现在被使用):
- `emit_and_record` (line 813)
- `build_intercepted_request` (line 836)

- [ ] **Step 2: 移除死代码**

删除以下内容：
```
struct RuleResponse { ... }  // line 534-538
fn infer_content_type(...)  // line 583-592
fn expand_user_path(...)    // line 595-601
fn render_mock_template(...)  // line 604-613
fn parse_header_object(...)   // line 616-628
fn build_map_local_response(...)  // line 630-679
fn build_http_response(...)  // line 682-696
fn classify_request(...)     // line 895-902
```

- [ ] **Step 3: 运行编译检查**

```bash
cargo check 2>&1 | head -50
```

Expected: 编译通过，警告减少

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/proxy.rs
git commit -m "chore(proxy): remove dead code"
```

---

## Task 5: 测试验证

**Files:**
- Modify: `src-tauri/src/proxy.rs` (if tests need adjustments)

- [ ] **Step 1: 运行现有测试**

```bash
cargo test 2>&1 | tail -30
```

Expected: 所有测试通过

- [ ] **Step 2: 运行编译检查**

```bash
cargo check 2>&1 | grep -E "^(error|warning:)" | head -20
```

Expected: 无 error，warnings 可接受

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/proxy.rs
git commit -m "test: verify MapRemote integration"
```

---

## 验证清单

- [ ] `cargo check` 通过
- [ ] `cargo test` 通过
- [ ] `RuleApplication::MapRemote` variant 正确携带 `RemoteTarget`
- [ ] `handle_http()` 在连接上游前调用 `apply_request_rule()`
- [ ] `MapRemote` case 调用 `forward_map_remote()` 并发送响应
- [ ] 死代码已移除
- [ ] 无新增编译警告

---

## 后续工作 (不在本计划范围内)

- MapLocal 实现
- HTTPS CONNECT 路径集成规则
- 规则匹配后的请求记录完整性
