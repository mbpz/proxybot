# MCP Server Design Specification

**Date:** 2026-05-10
**Feature:** MCP Server (Model Context Protocol)
**Status:** Design

---

## 1. Overview

This document specifies the design for ProxyBot's MCP server implementation. The MCP server exposes ProxyBot's traffic capture, classification, and rule management capabilities as tools that can be invoked by AI agents via the Model Context Protocol.

## 2. Protocol Specification

### 2.1 JSON-RPC 2.0

The MCP server uses JSON-RPC 2.0 for all communication. Both requests and responses follow the specification.

**Request Format:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "capture_traffic",
    "arguments": { "limit": 50 }
  }
}
```

**Response Format:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "requests": [...],
    "total": 50
  }
}
```

**Error Format:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Database error: connection locked",
    "data": null
  }
}
```

### 2.2 Error Codes

| Code | Name | Description |
|------|------|-------------|
| -32600 | InvalidRequest | The JSON sent is not a valid Request object |
| -32601 | MethodNotFound | The method does not exist/is not available |
| -32602 | InvalidParams | Invalid method parameter(s) |
| -32603 | InternalError | Internal JSON-RPC error |
| -32000 | ServerError | Application-specific error (DB, network, etc.) |

## 3. Tool Schemas

### 3.1 capture_traffic

Get recent HTTP/HTTPS requests captured by ProxyBot.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "limit": {
      "type": "number",
      "description": "Maximum number of requests to return (default: 100, max: 1000)",
      "minimum": 1,
      "maximum": 1000
    },
    "filter": {
      "type": "string",
      "description": "Filter expression in ProxyBot DSL (e.g., 'host:api.* status:2*')"
    },
    "since": {
      "type": "string",
      "description": "ISO 8601 timestamp to filter requests after"
    }
  }
}
```

**Output Schema:**
```json
{
  "type": "object",
  "properties": {
    "requests": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "id": { "type": "string", "description": "Unique request identifier" },
          "method": { "type": "string", "description": "HTTP method (GET, POST, etc.)" },
          "host": { "type": "string", "description": "Target host" },
          "path": { "type": "string", "description": "URL path" },
          "status": { "type": "number", "description": "HTTP status code" },
          "timestamp": { "type": "string", "description": "ISO 8601 timestamp" },
          "app": { "type": "string", "description": "Classified app name (WeChat, Douyin, etc.)" }
        }
      }
    },
    "total": { "type": "number", "description": "Total matching requests" }
  }
}
```

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "capture_traffic",
    "arguments": {
      "limit": 50,
      "filter": "host:api.* status:2*"
    }
  }
}
```

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "requests": [
      {
        "id": "req_1699234567_abc123",
        "method": "POST",
        "host": "api.weixin.qq.com",
        "path": "/cgi-bin/token",
        "status": 200,
        "timestamp": "2026-05-10T14:30:00Z",
        "app": "WeChat"
      }
    ],
    "total": 1
  }
}
```

### 3.2 classify_request

Classify a request by host/SNI to identify which app it belongs to.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "host": {
      "type": "string",
      "description": "Target hostname"
    },
    "sni": {
      "type": "string",
      "description": "TLS SNI value (optional, inferred from host if missing)"
    },
    "dns_query": {
      "type": "string",
      "description": "DNS query name if available from DNS log"
    }
  },
  "required": ["host"]
}
```

**Output Schema:**
```json
{
  "type": "object",
  "properties": {
    "app": {
      "type": "string",
      "description": "Classified app name (WeChat, Douyin, Alipay, Unknown)"
    },
    "confidence": {
      "type": "number",
      "description": "Classification confidence (0.0 to 1.0)"
    },
    "rules_matched": {
      "type": "array",
      "items": { "type": "string" },
      "description": "List of domain rules that matched"
    }
  }
}
```

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "app": "WeChat",
    "confidence": 0.95,
    "rules_matched": ["*.weixin.qq.com", "*.wechat.com"]
  }
}
```

### 3.3 apply_rule

Apply a rule to a specific request or pattern of requests.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "request_id": {
      "type": "string",
      "description": "Request identifier (optional if pattern provided)"
    },
    "pattern": {
      "type": "string",
      "description": "Filter pattern to match multiple requests (optional if request_id provided)"
    },
    "action": {
      "type": "string",
      "enum": ["allow", "block", "log", "unblock"],
      "description": "Action to apply"
    },
    "reason": {
      "type": "string",
      "description": "Human-readable reason for the rule"
    }
  },
  "required": ["action"]
}
```

**Output Schema:**
```json
{
  "type": "object",
  "properties": {
    "success": { "type": "boolean" },
    "rule_id": { "type": "string", "description": "Generated rule identifier" },
    "matched_count": { "type": "number", "description": "Number of requests matched (for pattern rules)" }
  }
}
```

**Example Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "apply_rule",
    "arguments": {
      "pattern": "host:ads.*",
      "action": "block",
      "reason": "Block advertising traffic"
    }
  }
}
```

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "success": true,
    "rule_id": "rule_1699234567_xyz789",
    "matched_count": 15
  }
}
```

### 3.4 get_devices

List all devices connected through ProxyBot.

**Input Schema:** None (no parameters)

**Output Schema:**
```json
{
  "type": "object",
  "properties": {
    "devices": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "id": { "type": "number" },
          "name": { "type": "string", "description": "Device name or MAC-based identifier" },
          "mac_address": { "type": "string" },
          "last_seen": { "type": "string", "description": "ISO 8601 timestamp" },
          "upload_bytes": { "type": "number" },
          "download_bytes": { "type": "number" }
        }
      }
    }
  }
}
```

### 3.5 get_alerts

Get security and anomaly detection alerts.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "since": {
      "type": "string",
      "description": "ISO 8601 timestamp to get alerts after (default: last hour)"
    },
    "limit": {
      "type": "number",
      "description": "Maximum number of alerts (default: 50, max: 200)"
    },
    "severity": {
      "type": "string",
      "enum": ["low", "medium", "high"],
      "description": "Filter by severity level"
    }
  }
}
```

**Output Schema:**
```json
{
  "type": "object",
  "properties": {
    "alerts": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "id": { "type": "string" },
          "severity": { "type": "string", "enum": ["low", "medium", "high"] },
          "title": { "type": "string" },
          "description": { "type": "string" },
          "source": { "type": "string", "description": "Which system generated the alert" },
          "timestamp": { "type": "string", "description": "ISO 8601 timestamp" }
        }
      }
    }
  }
}
```

## 4. Transport Specification

### 4.1 stdio Transport (Primary)

The stdio transport is used for Claude Desktop and CLI integration.

**Protocol:**
1. AI agent spawns ProxyBot as a subprocess with `proxybot mcp stdio`
2. Communication via stdin/stdout using newline-delimited JSON
3. Each line is one JSON-RPC message
4. No batch requests (one request at a time for simplicity)

**Startup:**
```bash
$ proxybot mcp stdio
{"jsonrpc":"2.0","method":"initialize","id":0,"params":{"protocolVersion":"2024-11-05","clientInfo":{"name":"claude","version":"1.0"}}}
{"jsonrpc":"2.0","id":0,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"proxybot","version":"1.2.0"}}}
```

### 4.2 StreamableHTTP Transport (Optional)

For Cursor AI and advanced integrations.

**Endpoints:**
- `POST /mcp` - Send JSON-RPC requests
- `GET /mcp` - SSE stream for server-initiated notifications

**Request:**
```http
POST /mcp HTTP/1.1
Content-Type: application/json

{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json

{"jsonrpc":"2.0","id":1,"result":{"tools":[...]}}
```

## 5. Error Handling

### 5.1 Application Errors

Application-level errors (database failures, network issues) return code -32000 with descriptive message:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Database error: unable to acquire lock",
    "data": {
      "retry_after_ms": 100
    }
  }
}
```

### 5.2 Graceful Degradation

- If DNS classification fails, fall back to SNI-based classification
- If SNI is missing, fall back to host-based classification with lower confidence
- If database is locked, retry with exponential backoff (max 3 attempts)

## 6. Claude Desktop Configuration

### 6.1 Installation Steps

1. Ensure ProxyBot is built: `cargo build --release`
2. Locate the binary path (typically `~/.cargo/bin/proxybot`)
3. Add to Claude Desktop configuration

### 6.2 Configuration File

**Path:** `~/.claude/settings.json` (or via Claude Desktop UI)

```json
{
  "mcpServers": {
    "proxybot": {
      "command": "/Users/doug/.cargo/bin/proxybot",
      "args": ["mcp", "--transport", "stdio"],
      "env": {
        "RUST_LOG": "proxybot::mcp=debug"
      }
    }
  }
}
```

**Alternative (from source):**
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

### 6.3 Verification

After configuration, test with:
```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | proxybot mcp stdio
```

Expected: JSON-RPC response with tool list.

## 7. Security Considerations

### 7.1 Local Access Only

- stdio transport is local-only (no network exposure)
- HTTP transport binds to localhost only (127.0.0.1)

### 7.2 No Authentication (Current)

MCP server assumes local access. Future versions may add:
- Unix socket authentication
- API key for HTTP transport

### 7.3 Data Privacy

- No traffic data leaves the local machine via MCP
- All queries are executed against local SQLite database
- AI agent only receives metadata, not full request/response bodies

## 8. Implementation Notes

### 8.1 Concurrency

- Use `tokio::sync::Mutex` for shared state (DbState, ClassifierState)
- Each tool handler is async and can run concurrently
- Connection pool for database (WAL mode allows concurrent reads)

### 8.2 Performance

- Tool responses should complete in < 100ms for typical queries
- Use database indexes on timestamp, host, status columns
- Cache classifier results for repeated host patterns

### 8.3 Logging

- Log all tool invocations with parameters (debug level)
- Log errors with full context (error level)
- Include request_id correlation for debugging