# MCP Server Implementation Plan

**Date:** 2026-05-10
**Feature:** MCP Server (Model Context Protocol)
**Priority:** P0
**Estimated Duration:** 3-5 days

---

## 1. Overview

This plan implements a Model Context Protocol (MCP) server that exposes ProxyBot's traffic capture and classification capabilities as AI agent tools. The server allows Claude Desktop and Cursor AI to query ProxyBot's traffic data, classify requests, and apply rules in real-time.

## 2. Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                     AI Agent (Claude Desktop / Cursor)        │
└────────────────────────────┬────────────────────────────────┘
                             │ stdio / StreamableHTTP
┌────────────────────────────▼────────────────────────────────┐
│                    ProxyBot MCP Server                       │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  MCP Protocol Layer (JSON-RPC 2.0)                    │   │
│  │  - initialize / tools/list / tools/call               │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Tool Handlers                                        │   │
│  │  - capture_traffic      - classify_request            │   │
│  │  - apply_rule           - get_devices                  │   │
│  │  - get_alerts                                          │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  ProxyBot State (DbState, Classifier, ProxyState)      │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Transport Options

1. **stdio** (primary): For Claude Desktop integration. Spawned as a subprocess.
2. **StreamableHTTP** (optional): For Cursor AI and advanced use cases.

### Project Structure

```
src-tauri/src/
├── mcp/
│   ├── mod.rs                 # Module root
│   ├── server.rs              # MCP server implementation
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── stdio.rs           # stdio transport
│   │   └── http.rs            # StreamableHTTP transport
│   ├── protocol/
│   │   ├── mod.rs
│   │   ├── types.rs           # JSON-RPC types
│   │   └── handlers.rs        # Tool handlers
│   └── tools/
│       ├── mod.rs
│       ├── capture.rs         # capture_traffic tool
│       ├── classify.rs        # classify_request tool
│       ├── rules.rs           # apply_rule tool
│       ├── devices.rs         # get_devices tool
│       └── alerts.rs          # get_alerts tool
```

## 3. Implementation Steps

### Day 1: MCP Protocol Foundation

**File:** `src-tauri/src/mcp/mod.rs`

```rust
// Create the mcp module and expose tools
pub mod protocol;
pub mod transport;
pub mod tools;

use std::sync::Arc;
use tauri::State;
use crate::db::DbState;
use crate::classifier::ClassifierState;

pub struct McpState {
    pub db: Arc<DbState>,
    pub classifier: Arc<ClassifierState>,
}
```

**File:** `src-tauri/src/mcp/protocol/types.rs`

Define JSON-RPC 2.0 types:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
```

**File:** `src-tauri/src/mcp/transport/stdio.rs`

Implement stdio transport:

```rust
use std::io::{BufRead, BufReader, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::io;

pub struct StdioTransport;

impl StdioTransport {
    pub async fn run_server<F>(handler: F) -> io::Result<()>
    where
        F: Fn(String) -> String + Send + Sync + 'static,
    {
        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin).lines();
        let mut stdout = tokio::io::stdout();

        while let Ok(Some(line)) = reader.next_line().await {
            let response = handler(line);
            stdout.write_all(response.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
        }
        Ok(())
    }
}
```

### Day 2-3: Tool Handlers

**File:** `src-tauri/src/mcp/tools/capture.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureTrafficParams {
    pub limit: Option<u32>,
    pub filter: Option<String>,
    pub since: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CaptureTrafficResult {
    pub requests: Vec<RequestSummary>,
    pub total: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestSummary {
    pub id: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: u16,
    pub timestamp: String,
    pub app: Option<String>,
}

pub async fn capture_traffic(
    state: &McpState,
    params: CaptureTrafficParams,
) -> Result<CaptureTrafficResult, String> {
    let limit = params.limit.unwrap_or(100);
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT id, method, host, path, status, timestamp, app_tag
         FROM http_requests
         ORDER BY timestamp DESC
         LIMIT ?1"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([limit], |row| {
        Ok(RequestSummary {
            id: row.get(0)?,
            method: row.get(1)?,
            host: row.get(2)?,
            path: row.get(3)?,
            status: row.get(4)?,
            timestamp: row.get(5)?,
            app: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    let requests: Vec<_> = rows.filter_map(|r| r.ok()).collect();
    let total = requests.len() as u32;

    Ok(CaptureTrafficResult { requests, total })
}
```

**File:** `src-tauri/src/mcp/tools/classify.rs`

```rust
use serde::{Deserialize, Serialize};
use crate::classifier;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassifyRequestParams {
    pub host: String,
    pub sni: Option<String>,
    pub dns_query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassifyRequestResult {
    pub app: String,
    pub confidence: f32,
    pub rules_matched: Vec<String>,
}

pub async fn classify_request(
    state: &McpState,
    params: ClassifyRequestParams,
) -> Result<ClassifyRequestResult, String> {
    let app = classifier::classify_host(
        &params.host,
        params.sni.as_deref(),
        params.dns_query.as_deref(),
    );

    Ok(ClassifyRequestResult {
        app: app.name,
        confidence: app.confidence,
        rules_matched: app.matched_rules,
    })
}
```

**File:** `src-tauri/src/mcp/tools/rules.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyRuleParams {
    pub request_id: String,
    pub action: String,  // "allow" | "block" | "log"
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplyRuleResult {
    pub success: bool,
    pub rule_id: String,
}

pub async fn apply_rule(
    state: &McpState,
    params: ApplyRuleParams,
) -> Result<ApplyRuleResult, String> {
    let rule_id = format!("rule_{}", chrono_lite_timestamp());

    // Insert into app_rules table
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO app_rules (request_id, action, reason, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            params.request_id,
            params.action,
            params.reason.unwrap_or_default(),
            chrono_lite_timestamp(),
        ],
    ).map_err(|e| e.to_string())?;

    Ok(ApplyRuleResult {
        success: true,
        rule_id,
    })
}
```

**File:** `src-tauri/src/mcp/tools/devices.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetDevicesResult {
    pub devices: Vec<DeviceInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: i64,
    pub name: String,
    pub mac_address: String,
    pub last_seen: String,
    pub upload_bytes: i64,
    pub download_bytes: i64,
}

pub async fn get_devices(state: &McpState) -> Result<GetDevicesResult, String> {
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT id, name, mac_address, last_seen_at, upload_bytes, download_bytes
         FROM devices ORDER BY last_seen_at DESC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([], |row| {
        Ok(DeviceInfo {
            id: row.get(0)?,
            name: row.get(1)?,
            mac_address: row.get(2)?,
            last_seen: row.get(3)?,
            upload_bytes: row.get(4)?,
            download_bytes: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;

    let devices: Vec<_> = rows.filter_map(|r| r.ok()).collect();

    Ok(GetDevicesResult { devices })
}
```

**File:** `src-tauri/src/mcp/tools/alerts.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetAlertsParams {
    pub since: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetAlertsResult {
    pub alerts: Vec<Alert>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: String,  // "low" | "medium" | "high"
    pub title: String,
    pub description: String,
    pub source: String,
    pub timestamp: String,
}

pub async fn get_alerts(
    state: &McpState,
    params: GetAlertsParams,
) -> Result<GetAlertsResult, String> {
    // Query anomaly detection results
    let conn = state.db.conn.lock().map_err(|e| e.to_string())?;
    let limit = params.limit.unwrap_or(50);

    let mut stmt = conn.prepare(
        "SELECT id, severity, title, description, source, timestamp
         FROM anomalies WHERE timestamp > ?1
         ORDER BY timestamp DESC LIMIT ?2"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map([
        params.since.unwrap_or_else(|| "0".to_string()).as_str(),
        &limit.to_string(),
    ], |row| {
        Ok(Alert {
            id: row.get(0)?,
            severity: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            source: row.get(4)?,
            timestamp: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;

    let alerts: Vec<_> = rows.filter_map(|r| r.ok()).collect();

    Ok(GetAlertsResult { alerts })
}
```

### Day 4: Server Integration

**File:** `src-tauri/src/mcp/server.rs`

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::McpState;

pub struct McpServer {
    state: Arc<McpState>,
}

impl McpServer {
    pub fn new(state: Arc<McpState>) -> Self {
        Self { state }
    }

    pub async fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let method = &req.method;
        let id = req.id;

        let result = match method.as_str() {
            "initialize" => self.handle_initialize(req.params).await,
            "tools/list" => self.handle_list_tools().await,
            "tools/call" => self.handle_call_tool(req.params).await,
            _ => Err(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", method),
                data: None,
            }),
        };

        match result {
            Ok(result) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            },
            Err(error) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(error),
            },
        }
    }

    async fn handle_initialize(&self, _params: Option<serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "proxybot",
                "version": "1.2.0"
            }
        }))
    }

    async fn handle_list_tools(&self) -> Result<serde_json::Value, JsonRpcError> {
        Ok(serde_json::json!({
            "tools": [
                {
                    "name": "capture_traffic",
                    "description": "Get recent HTTP/HTTPS requests captured by ProxyBot",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "limit": { "type": "number", "description": "Max requests to return" },
                            "filter": { "type": "string", "description": "Filter expression" },
                            "since": { "type": "string", "description": "ISO timestamp" }
                        }
                    }
                },
                {
                    "name": "classify_request",
                    "description": "Classify a request by host/SNI to identify the app",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "host": { "type": "string" },
                            "sni": { "type": "string" },
                            "dns_query": { "type": "string" }
                        },
                        "required": ["host"]
                    }
                },
                {
                    "name": "apply_rule",
                    "description": "Apply a rule to a request (allow/block/log)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "request_id": { "type": "string" },
                            "action": { "type": "string", "enum": ["allow", "block", "log"] },
                            "reason": { "type": "string" }
                        },
                        "required": ["request_id", "action"]
                    }
                },
                {
                    "name": "get_devices",
                    "description": "List all devices connected through ProxyBot"
                },
                {
                    "name": "get_alerts",
                    "description": "Get security/anomaly alerts",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "since": { "type": "string" },
                            "limit": { "type": "number" }
                        }
                    }
                }
            ]
        }))
    }

    async fn handle_call_tool(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError {
            code: -32602,
            message: "Missing params".to_string(),
            data: None,
        })?;

        let name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: -32602,
                message: "Missing tool name".to_string(),
                data: None,
            })?;

        let arguments = params.get("arguments")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        match name {
            "capture_traffic" => {
                let p = serde_json::from_value(serde_json::Value::Object(arguments))
                    .map_err(|e| JsonRpcError { code: -32602, message: e.to_string(), data: None })?;
                let result = capture_traffic(&self.state, p).await
                    .map_err(|e| JsonRpcError { code: -32000, message: e.to_string(), data: None })?;
                Ok(serde_json::to_value(result).unwrap())
            },
            // ... other tools
            _ => Err(JsonRpcError {
                code: -32601,
                message: format!("Unknown tool: {}", name),
                data: None,
            })
        }
    }
}
```

### Day 5: CLI and Integration

**File:** `src-tauri/src/commands/mcp.rs`

Add Tauri command to start MCP server:

```rust
use std::sync::Arc;
use tokio::task;

#[tauri::command]
pub async fn start_mcp_server(
    transport: String,
    port: Option<u16>,
) -> Result<String, String> {
    match transport.as_str() {
        "stdio" => {
            // Run stdio server in background
            task::spawn(async {
                let server = McpServer::new(get_mcp_state().await);
                StdioTransport::run_server(|req| {
                    let response = tokio::runtime::Handle::current()
                        .block_on(server.handle_request(parse_jsonRpc(req)));
                    serde_json::to_string(&response).unwrap()
                }).await;
            });
            Ok("MCP server started (stdio mode)".to_string())
        },
        "http" => {
            let port = port.unwrap_or(9090);
            let addr = format!("0.0.0.0:{}", port);
            let server = McpServer::new(get_mcp_state().await);

            task::spawn(async move {
                let app = axum::Router::new()
                    .route("/mcp", axum::routing::post(handle_mcp_request))
                    .route("/mcp", axum::routing::get(handle_sse))
                    .with_state(Arc::new(server));

                axum::Server::bind(&addr.parse().unwrap())
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
            });

            Ok(format!("MCP server started on http://0.0.0.0:{}/mcp", port))
        },
        _ => Err("Invalid transport".to_string()),
    }
}
```

## 4. Key Files to Create/Modify

| File | Action |
|------|--------|
| `src-tauri/src/mcp/mod.rs` | CREATE - Module root |
| `src-tauri/src/mcp/server.rs` | CREATE - Main server |
| `src-tauri/src/mcp/protocol/mod.rs` | CREATE - Protocol types |
| `src-tauri/src/mcp/protocol/types.rs` | CREATE - JSON-RPC types |
| `src-tauri/src/mcp/protocol/handlers.rs` | CREATE - Tool handlers |
| `src-tauri/src/mcp/transport/mod.rs` | CREATE - Transport layer |
| `src-tauri/src/mcp/transport/stdio.rs` | CREATE - stdio transport |
| `src-tauri/src/mcp/transport/http.rs` | CREATE - HTTP transport |
| `src-tauri/src/mcp/tools/mod.rs` | CREATE - Tools module |
| `src-tauri/src/mcp/tools/capture.rs` | CREATE - capture_traffic |
| `src-tauri/src/mcp/tools/classify.rs` | CREATE - classify_request |
| `src-tauri/src/mcp/tools/rules.rs` | CREATE - apply_rule |
| `src-tauri/src/mcp/tools/devices.rs` | CREATE - get_devices |
| `src-tauri/src/mcp/tools/alerts.rs` | CREATE - get_alerts |
| `src-tauri/src/commands/mod.rs` | MODIFY - Add mcp command |
| `src-tauri/src/commands/mcp.rs` | CREATE - Tauri command |
| `src-tauri/src/lib.rs` | MODIFY - Register mcp module |

## 5. Dependencies to Add

```toml
# Cargo.toml
tokio = { version = "1", features = ["full"] }
axum = "0.7"
serde_json = "1"
```

## 6. Testing Strategy

1. **Unit tests**: Test each tool handler with mock DbState
2. **Integration tests**: Test full JSON-RPC roundtrip via stdio
3. **Manual verification**: Connect Claude Desktop and verify tools work

## 7. Configuration

### Claude Desktop Config

**File:** `~/.claude/desktop.json` (user's machine)

```json
{
  "mcpServers": {
    "proxybot": {
      "command": "cargo",
      "args": ["run", "--manifest-path", "/Users/doug/ai/system/proxybot/src-tauri/Cargo.toml", "--", "mcp", "stdio"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

Or with compiled binary:

```json
{
  "mcpServers": {
    "proxybot": {
      "command": "/Users/doug/.cargo/bin/proxybot",
      "args": ["mcp", "--transport", "stdio"]
    }
  }
}
```

## 8. Timeline

| Day | Task |
|-----|------|
| 1 | Protocol foundation (types, transport) |
| 2-3 | Tool handlers (5 tools) |
| 4 | Server integration, CLI entry point |
| 5 | Testing, Claude Desktop config docs |