# MapRemote 规则集成设计方案

## Status: Approved

## 1. 概述

将已定义的 MapRemote 规则功能集成到 `handle_http` 请求处理流程中，实现请求的远程映射能力。

**目标**：
- 规则引擎返回 `RuleApplication::MapRemote` 时，调用方负责发送请求到远程目标
- 保持网络/逻辑分离：`apply_request_rule()` 保持同步，不做网络操作
- 完整保留原始请求语义（headers、查询参数等）

## 2. 架构

```
handle_http()
    │
    ├── apply_request_rule() → RuleApplication
    │       │
    │       ├── Continue { method, path, headers, body }
    │       │       └── 继续正常流程，连接上游服务器
    │       │
    │       └── MapRemote { target, method, path, headers, body }
    │               └── 调用 forward_map_remote() 发送到远程目标
    │
    └── 后续处理...
```

### 2.1 关键设计决策

- **同步规则检查**：`apply_request_rule()` 保持同步，不做网络操作
- **调用方负责网络**：`RuleApplication::MapRemote` 携带完整信息，调用方调用 `forward_map_remote()`
- **完整语义保留**：MapRemote 保留所有原始 headers

## 3. RuleApplication 枚举修改

**文件**: `src/proxy.rs`

```rust
enum RuleApplication {
    /// 继续正常流程，连接上游服务器
    Continue {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
    /// 发送HTTP响应给客户端后结束（用于Reject、MapLocal、Breakpoint Drop）
    Respond {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
    /// 转发请求到远程目标
    MapRemote {
        target: RemoteTarget,
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
}
```

**说明**:
- `Continue` - Direct/Proxy/BreakpointDecision::Proceed 使用，继续正常流程
- `Respond` - Reject/MapLocal/BreakpointDecision::Drop 使用，直接返回HTTP响应
- `MapRemote` - 转发到远程目标，调用方调用 `forward_map_remote()`

## 4. handle_http 集成点

**位置**: `src/proxy.rs` 约 line 1364

在获取 `target_addr` 之前调用规则检查：

```rust
async fn handle_http(
    ctx: ProxyContext,
    device_ctx: Option<DeviceContext>,
    client_stream: TcpStream,
    client_addr: SocketAddr,
    method: &str,
    path: &str,
    host: &str,
    port: u16,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<(), String> {
    let start = std::time::Instant::now();

    // 规则检查（新增）
    match apply_request_rule(
        &ctx.rules_engine,
        client_addr,
        "http",
        host,
        method,
        path,
        headers,
        body,
    )? {
        RuleApplication::Continue { method, path, headers, body } => {
            // 继续现有逻辑，使用规则返回的修改后数据
        }
        RuleApplication::Respond { status, headers: resp_headers, body: resp_body } => {
            // 直接发送响应给客户端，不连接上游
            let response = build_http_response(status, resp_headers, resp_body);
            client_stream.write_all(&response).await
                .map_err(|e| format!("Write rule response failed: {}", e))?;
            return Ok(());
        }
        RuleApplication::MapRemote { target, method, path, headers, body } => {
            // 发送请求到远程目标
            let response_buf = forward_map_remote(
                &ctx.cert_manager,
                &target,
                &method,
                &path,
                &headers,
                &body,
            ).await?;

            // 发送响应给客户端
            client_stream.write_all(&response_buf).await?;

            // 记录请求（使用修改后的数据构建 InterceptedRequest）
            let req = build_intercepted_request(...);
            emit_and_record(&ctx, req);

            return Ok(());
        }
    }

    // 后续正常流程...
}
```

## 5. forward_map_remote 调用整合

`forward_map_remote` 函数已定义（line 770），需要在 `handle_http` 中调用：

```rust
async fn forward_map_remote(
    cert_manager: &CertManager,
    target: &RemoteTarget,
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<Vec<u8>, String> {
    // 现有实现保持不变
}
```

## 6. 移除死代码

以下函数/结构体将移除（已定义但未使用）：

| 项目 | 类型 | 原因 |
|------|------|------|
| `RuleResponse` | 结构体 | 仅 `build_http_response` 使用，后者未使用 |
| `build_map_local_response` | 函数 | MapLocal 暂不实现 |
| `build_http_response` | 函数 | 未被调用 |
| `parse_header_object` | 函数 | 未被调用 |
| `classify_request` | 函数 | 已有 `app_rules::classify_host` |

## 7. 实施步骤

1. **修改 `RuleApplication` 枚举** - 添加 `MapRemote` 变体
2. **修改 `apply_request_rule()` 返回类型** - 返回 `RuleApplication` 而非 `Option<RuleAction>`
3. **实现 `MapRemote` 分支处理** - 在 `handle_http` 中添加规则检查和转发逻辑
4. **移除死代码** - 删除未使用的辅助函数
5. **测试验证** - 验证规则匹配和转发功能

## 8. 验证

```bash
# 编译检查
cargo check

# 运行测试
cargo test

# 手动测试：配置 MapRemote 规则，发送 HTTP 请求，验证转发
```

## 9. 后续工作

- MapLocal 实现（本地 mock 响应）
- HTTPS CONNECT 路径集成规则检查
- 规则匹配后的请求记录完整性（记录原始目标 vs 实际目标）
