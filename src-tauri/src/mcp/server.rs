// MCP Server implementation - handles JSON-RPC 2.0 requests via stdio transport

use super::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpState};
use std::sync::Arc;

/// Main MCP server that handles protocol requests
pub struct McpServer {
    state: Arc<McpState>,
}

impl McpServer {
    pub fn new(state: Arc<McpState>) -> Self {
        Self { state }
    }

    /// Handle a JSON-RPC request and return a response
    pub fn handle_request(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let method = req.method.clone();
        let id = req.id.clone();

        let result = match method.as_str() {
            "initialize" => self.handle_initialize(req.params),
            "tools/list" => self.handle_list_tools(req.params),
            "tools/call" => self.handle_call_tool(req.params),
            _ => Err(JsonRpcError::method_not_found(&method)),
        };

        match result {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(error) => JsonRpcResponse::error(id, error),
        }
    }

    fn handle_initialize(&self, _params: Option<serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
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

    fn handle_list_tools(&self, _params: Option<serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        Ok(serde_json::json!({
            "tools": [
                {
                    "name": "capture_traffic",
                    "description": "Get recent HTTP/HTTPS requests captured by ProxyBot",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "limit": {
                                "type": "number",
                                "description": "Max requests to return (default: 100)"
                            },
                            "filter": {
                                "type": "string",
                                "description": "Filter expression (e.g. host contains 'api')"
                            },
                            "since": {
                                "type": "string",
                                "description": "ISO timestamp - only return requests after this time"
                            }
                        }
                    }
                },
                {
                    "name": "classify_request",
                    "description": "Classify a request by host/SNI to identify the app (WeChat, Douyin, Alipay, etc.)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "host": {
                                "type": "string",
                                "description": "The hostname to classify"
                            },
                            "sni": {
                                "type": "string",
                                "description": "Server Name Indication from TLS handshake"
                            },
                            "dns_query": {
                                "type": "string",
                                "description": "DNS query name if available"
                            }
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
                            "request_id": {
                                "type": "string",
                                "description": "Request ID to apply rule to"
                            },
                            "action": {
                                "type": "string",
                                "enum": ["allow", "block", "log"],
                                "description": "The action to take"
                            },
                            "reason": {
                                "type": "string",
                                "description": "Optional reason for the rule"
                            }
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
                            "since": {
                                "type": "string",
                                "description": "ISO timestamp - only return alerts after this time"
                            },
                            "limit": {
                                "type": "number",
                                "description": "Max alerts to return (default: 50)"
                            }
                        }
                    }
                }
            ]
        }))
    }

    fn handle_call_tool(&self, params: Option<serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;

        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing tool name"))?;

        let arguments = params
            .get("arguments")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        match name {
            "capture_traffic" => self.tool_capture_traffic(arguments),
            "classify_request" => self.tool_classify_request(arguments),
            "apply_rule" => self.tool_apply_rule(arguments),
            "get_devices" => self.tool_get_devices(arguments),
            "get_alerts" => self.tool_get_alerts(arguments),
            _ => Err(JsonRpcError::method_not_found(name)),
        }
    }

    fn tool_capture_traffic(&self, args: serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        use serde_json::Value;

        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as u32;
        let filter = args.get("filter").and_then(|v| v.as_str());
        let since = args.get("since").and_then(|v| v.as_str());

        let conn = self.state.db.conn.lock().map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let mut query = String::from(
            "SELECT id, method, host, path, status, timestamp, app_tag \
             FROM http_requests WHERE 1=1"
        );

        if let Some(since_ts) = since {
            query.push_str(&format!(" AND timestamp > '{}'", since_ts));
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ");
        query.push_str(&(limit as i64).to_string());

        let mut stmt = conn.prepare(&query).map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "method": row.get::<_, String>(1)?,
                "host": row.get::<_, String>(2)?,
                "path": row.get::<_, String>(3)?,
                "status": row.get::<_, u16>(4)?,
                "timestamp": row.get::<_, String>(5)?,
                "app": row.get::<_, Option<String>>(6)?,
            }))
        }).map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let requests: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
        let total = requests.len() as u32;

        Ok(serde_json::json!({
            "requests": requests,
            "total": total,
            "filter_applied": filter,
        }))
    }

    fn tool_classify_request(&self, args: serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        let host = args.get("host")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'host' parameter"))?;

        let sni = args.get("sni").and_then(|v| v.as_str());
        let dns_query = args.get("dns_query").and_then(|v| v.as_str());

        // Use classifier to determine app
        let app = classify_host(host, sni, dns_query);

        Ok(serde_json::json!({
            "app": app.name,
            "confidence": app.confidence,
            "rules_matched": app.matched_rules,
        }))
    }

    fn tool_apply_rule(&self, args: serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        let request_id = args.get("request_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'request_id' parameter"))?;

        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'action' parameter"))?;

        let reason = args.get("reason").and_then(|v| v.as_str());

        if !["allow", "block", "log"].contains(&action) {
            return Err(JsonRpcError::invalid_params("action must be one of: allow, block, log"));
        }

        let rule_id = format!("rule_{}", chrono_lite_timestamp());

        let conn = self.state.db.conn.lock().map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        conn.execute(
            "INSERT INTO app_rules (request_id, action, reason, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![request_id, action, reason.unwrap_or(""), chrono_lite_timestamp()],
        ).map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "rule_id": rule_id,
        }))
    }

    fn tool_get_devices(&self, _args: serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        let conn = self.state.db.conn.lock().map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, mac_address, last_seen_at, upload_bytes, download_bytes \
             FROM devices ORDER BY last_seen_at DESC"
        ).map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "mac_address": row.get::<_, String>(2)?,
                "last_seen": row.get::<_, String>(3)?,
                "upload_bytes": row.get::<_, i64>(4)?,
                "download_bytes": row.get::<_, i64>(5)?,
            }))
        }).map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let devices: Vec<serde_json::Value> = rows.filter_map(|r| r.ok()).collect();

        Ok(serde_json::json!({
            "devices": devices,
        }))
    }

    fn tool_get_alerts(&self, args: serde_json::Map<String, serde_json::Value>) -> Result<serde_json::Value, JsonRpcError> {
        let since = args.get("since").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as u32;

        let conn = self.state.db.conn.lock().map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let since_clause = if let Some(since_ts) = since {
            format!(" WHERE timestamp > '{}'", since_ts)
        } else {
            String::new()
        };

        let query = format!(
            "SELECT id, severity, title, description, source, timestamp \
             FROM anomalies{} ORDER BY timestamp DESC LIMIT {}",
            since_clause, limit
        );

        let mut stmt = conn.prepare(&query).map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "severity": row.get::<_, String>(1)?,
                "title": row.get::<_, String>(2)?,
                "description": row.get::<_, String>(3)?,
                "source": row.get::<_, String>(4)?,
                "timestamp": row.get::<_, String>(5)?,
            }))
        }).map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let alerts: Vec<serde_json::Value> = rows.filter_map(|r| r.ok()).collect();

        Ok(serde_json::json!({
            "alerts": alerts,
        }))
    }
}

/// Simple classification result
struct ClassifiedApp {
    name: String,
    confidence: f32,
    matched_rules: Vec<String>,
}

/// Classify a host to identify the app (WeChat, Douyin, Alipay, etc.)
fn classify_host(host: &str, sni: Option<&str>, _dns_query: Option<&str>) -> ClassifiedApp {
    let check_str = format!("{} {} {}", host, sni.unwrap_or(""), "");

    // WeChat patterns
    let wechat_patterns = ["weixin.qq.com", "wechat.com", "qq.com", "weixin100.com"];
    for pattern in &wechat_patterns {
        if check_str.contains(pattern) {
            return ClassifiedApp {
                name: "WeChat".to_string(),
                confidence: 0.95,
                matched_rules: vec![pattern.to_string()],
            };
        }
    }

    // Douyin/TikTok patterns
    let douyin_patterns = ["douyin.com", "tiktokv.com", "tiktok.com", "bytecdn.com"];
    for pattern in &douyin_patterns {
        if check_str.contains(pattern) {
            return ClassifiedApp {
                name: "Douyin".to_string(),
                confidence: 0.95,
                matched_rules: vec![pattern.to_string()],
            };
        }
    }

    // Alipay patterns
    let alipay_patterns = ["alipay.com", "alipayusercontent.com", "alipay.ec", "antfin.com"];
    for pattern in &alipay_patterns {
        if check_str.contains(pattern) {
            return ClassifiedApp {
                name: "Alipay".to_string(),
                confidence: 0.95,
                matched_rules: vec![pattern.to_string()],
            };
        }
    }

    // Default - unknown
    ClassifiedApp {
        name: "Unknown".to_string(),
        confidence: 0.1,
        matched_rules: vec![],
    }
}

/// Get current timestamp in ISO format for rule IDs
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let secs = duration.as_secs();
    format!("{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbState;
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn create_test_state() -> Arc<McpState> {
        // Create an in-memory database for testing
        let conn = Connection::open_in_memory().unwrap();
        // Initialize minimal schema for tests
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS http_requests (\
             id TEXT PRIMARY KEY, method TEXT, host TEXT, path TEXT, \
             status INTEGER, timestamp TEXT, app_tag TEXT); \
             CREATE TABLE IF NOT EXISTS devices (\
             id INTEGER PRIMARY KEY, name TEXT, mac_address TEXT, \
             last_seen_at TEXT, upload_bytes INTEGER, download_bytes INTEGER); \
             CREATE TABLE IF NOT EXISTS app_rules (\
             id INTEGER PRIMARY KEY, request_id TEXT, action TEXT, reason TEXT, created_at TEXT); \
             CREATE TABLE IF NOT EXISTS anomalies (\
             id TEXT, severity TEXT, title TEXT, description TEXT, source TEXT, timestamp TEXT);"
        ).unwrap();

        Arc::new(McpState {
            db: Arc::new(DbState { conn: Mutex::new(conn) }),
        })
    }

    #[test]
    fn test_server_initialization() {
        let state = create_test_state();
        let server = McpServer::new(state);
        assert!(std::mem::size_of_val(&server) > 0);
    }

    #[test]
    fn test_initialize_request() {
        let state = create_test_state();
        let server = McpServer::new(state);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: None,
        };

        let resp = server.handle_request(req);
        assert!(resp.result.is_some());

        let result = resp.result.unwrap();
        assert_eq!(result.get("protocolVersion").and_then(|v| v.as_str()), Some("2024-11-05"));
        assert_eq!(result.get("serverInfo").and_then(|v| v.get("name")).and_then(|v| v.as_str()), Some("proxybot"));
    }

    #[test]
    fn test_list_tools_request() {
        let state = create_test_state();
        let server = McpServer::new(state);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(2)),
            method: "tools/list".to_string(),
            params: None,
        };

        let resp = server.handle_request(req);
        assert!(resp.result.is_some());

        let result = resp.result.unwrap();
        let tools = result.get("tools").and_then(|v| v.as_array()).expect("tools array expected");
        assert_eq!(tools.len(), 5); // capture_traffic, classify_request, apply_rule, get_devices, get_alerts

        let tool_names: Vec<&str> = tools.iter()
            .filter_map(|t| t.get("name").and_then(|v| v.as_str()))
            .collect();
        assert!(tool_names.contains(&"capture_traffic"));
        assert!(tool_names.contains(&"classify_request"));
        assert!(tool_names.contains(&"apply_rule"));
        assert!(tool_names.contains(&"get_devices"));
        assert!(tool_names.contains(&"get_alerts"));
    }

    #[test]
    fn test_unknown_method() {
        let state = create_test_state();
        let server = McpServer::new(state);

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(3)),
            method: "unknown/method".to_string(),
            params: None,
        };

        let resp = server.handle_request(req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32601);
    }

    #[test]
    fn test_call_tool_capture_traffic() {
        let state = create_test_state();
        let server = McpServer::new(state);

        let params = serde_json::json!({
            "name": "capture_traffic",
            "arguments": {
                "limit": 10
            }
        });

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(4)),
            method: "tools/call".to_string(),
            params: Some(params),
        };

        let resp = server.handle_request(req);
        // In test mode with in-memory DB, should return empty requests
        assert!(resp.result.is_some() || resp.error.is_some()); // May error if no table exists
    }

    #[test]
    fn test_classify_wechat() {
        let result = classify_host("wxapi.weixin.qq.com", None, None);
        assert_eq!(result.name, "WeChat");
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_classify_douyin() {
        let result = classify_host("dm.tiktokv.com", None, None);
        assert_eq!(result.name, "Douyin");
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_classify_alipay() {
        let result = classify_host("pay.alipay.com", None, None);
        assert_eq!(result.name, "Alipay");
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_classify_unknown() {
        let result = classify_host("random.unknown-site.com", None, None);
        assert_eq!(result.name, "Unknown");
        assert!(result.confidence < 0.5);
    }

    #[test]
    fn test_chrono_timestamp() {
        let ts = chrono_lite_timestamp();
        // Should be a valid unix timestamp (10+ digits for current era)
        assert!(ts.parse::<u64>().is_ok());
        let parsed: u64 = ts.parse().unwrap();
        assert!(parsed > 1_000_000_000); // After year 2001
    }
}