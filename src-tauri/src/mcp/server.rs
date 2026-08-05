// MCP Server implementation - handles JSON-RPC 2.0 requests via stdio transport

use super::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpState};
use crate::alerts::{Alert, AlertQuery, AlertSeverity};
use proxybot_core::{AppConfig, ApplicationClassifier};
use std::sync::Arc;

/// Main MCP server that handles protocol requests
pub struct McpServer {
    state: Arc<McpState>,
    classifier: ApplicationClassifier,
}

impl McpServer {
    pub fn new(state: Arc<McpState>) -> Self {
        Self {
            state,
            classifier: ApplicationClassifier::new(Vec::new()),
        }
    }

    pub fn with_config(state: Arc<McpState>, config: &AppConfig) -> Self {
        Self {
            state,
            classifier: ApplicationClassifier::from_paths(
                &config.app_rules_path,
                &config.app_signatures_path,
            ),
        }
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

    fn handle_initialize(
        &self,
        _params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "proxybot",
                "version": env!("CARGO_PKG_VERSION")
            }
        }))
    }

    fn handle_list_tools(
        &self,
        _params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
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
                            },
                            "device_id": {
                                "type": "number",
                                "description": "Only return alerts attributed to this device"
                            },
                            "severity": {
                                "type": "string",
                                "enum": ["Info", "Warning", "Critical"]
                            },
                            "acknowledged": {
                                "type": "boolean"
                            }
                        }
                    }
                },
                {
                    "name": "acknowledge_alert",
                    "description": "Acknowledge one persisted ProxyBot alert",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "alert_id": {
                                "type": "number",
                                "description": "Persisted alert identifier"
                            }
                        },
                        "required": ["alert_id"]
                    }
                }
            ]
        }))
    }

    fn handle_call_tool(
        &self,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
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
            "acknowledge_alert" => self.tool_acknowledge_alert(arguments),
            _ => Err(JsonRpcError::method_not_found(name)),
        }
    }

    fn tool_capture_traffic(
        &self,
        args: serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use serde_json::Value;

        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100);
        let filter = args.get("filter").and_then(|v| v.as_str());
        let since = args.get("since").and_then(|v| v.as_str());
        // Bound the limit so a malicious client cannot request an unbounded
        // result set. Cap matches the historical default used elsewhere.
        let query = crate::db::CapturedRequestQuery {
            since: since.map(str::to_owned),
            order: crate::db::CapturedRequestOrder::TimestampDescending,
            limit: Some(limit.min(1000) as usize),
            ..Default::default()
        };
        let requests: Vec<Value> = self
            .state
            .db
            .captured_requests(&query)
            .map_err(|error| JsonRpcError::internal_error(&error))?
            .into_iter()
            .map(|record| {
                serde_json::json!({
                    "id": record.id,
                    "method": record.method,
                    "host": record.host,
                    "path": record.path,
                    "status": record.response_status,
                    "timestamp": record.timestamp,
                    "app": record.app_tag,
                })
            })
            .collect();
        let total = requests.len() as u32;

        Ok(serde_json::json!({
            "requests": requests,
            "total": total,
            "filter_applied": filter,
        }))
    }

    fn tool_classify_request(
        &self,
        args: serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let host = args
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'host' parameter"))?;

        let sni = args.get("sni").and_then(|v| v.as_str());
        let dns_query = args.get("dns_query").and_then(|v| v.as_str());

        let attribution = self.classifier.classify_request(host, sni, dns_query);
        Ok(match attribution {
            Some(attribution) => serde_json::json!({
                "app": attribution.app_name,
                "app_id": attribution.app_id,
                "icon": attribution.app_icon,
                "confidence": attribution.confidence,
                "source": attribution.source,
                "rules_matched": attribution.evidence,
            }),
            None => serde_json::json!({
                "app": "Unknown",
                "confidence": 0.1,
                "rules_matched": [],
            }),
        })
    }

    fn tool_apply_rule(
        &self,
        args: serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let request_id = args
            .get("request_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'request_id' parameter"))?;

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'action' parameter"))?;

        let reason = args.get("reason").and_then(|v| v.as_str());

        if !["allow", "block", "log"].contains(&action) {
            return Err(JsonRpcError::invalid_params(
                "action must be one of: allow, block, log",
            ));
        }

        let rule_id = format!("rule_{}", chrono_lite_timestamp());

        let conn = self
            .state
            .db
            .conn
            .lock()
            .map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        conn.execute(
            "INSERT INTO app_rules (request_id, action, reason, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![request_id, action, reason.unwrap_or(""), chrono_lite_timestamp()],
        ).map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "rule_id": rule_id,
        }))
    }

    fn tool_get_devices(
        &self,
        _args: serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let conn = self
            .state
            .db
            .conn
            .lock()
            .map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, mac_address, last_seen_at, upload_bytes, download_bytes \
             FROM devices ORDER BY last_seen_at DESC",
            )
            .map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "mac_address": row.get::<_, String>(2)?,
                    "last_seen": row.get::<_, String>(3)?,
                    "upload_bytes": row.get::<_, i64>(4)?,
                    "download_bytes": row.get::<_, i64>(5)?,
                }))
            })
            .map_err(|e| JsonRpcError::internal_error(&e.to_string()))?;

        let devices: Vec<serde_json::Value> = rows.filter_map(|r| r.ok()).collect();

        Ok(serde_json::json!({
            "devices": devices,
        }))
    }

    fn tool_get_alerts(
        &self,
        args: serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let since = args.get("since").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50);
        // Bound both the result count and the `since` length so neither a
        // malicious caller nor a typo can blow up the query plan or the
        // response payload.
        let limit = limit.min(1000) as usize;
        let since = since.filter(|s| !s.is_empty() && s.len() <= 64);
        let severity = args
            .get("severity")
            .map(|value| serde_json::from_value::<AlertSeverity>(value.clone()))
            .transpose()
            .map_err(|error| JsonRpcError::invalid_params(&error.to_string()))?;
        let device_id = args.get("device_id").and_then(|value| value.as_i64());
        let acknowledged = args.get("acknowledged").and_then(|value| value.as_bool());
        let alerts = self
            .state
            .db
            .alerts(&AlertQuery {
                device_id,
                severity,
                since: since.map(str::to_owned),
                acknowledged,
                limit: Some(limit),
            })
            .map_err(|error| JsonRpcError::internal_error(&error))?
            .into_iter()
            .map(mcp_alert)
            .collect::<Vec<_>>();

        Ok(serde_json::json!({
            "alerts": alerts,
        }))
    }

    fn tool_acknowledge_alert(
        &self,
        args: serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let alert_id = args
            .get("alert_id")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| JsonRpcError::invalid_params("alert_id must be an integer"))?;
        let alert = self
            .state
            .db
            .acknowledge_alert(alert_id)
            .map_err(|error| JsonRpcError::invalid_params(&error))?;
        Ok(serde_json::json!({ "alert": mcp_alert(alert) }))
    }
}

/// Preserve the historical MCP aliases while exposing the canonical Alert fields.
fn mcp_alert(alert: Alert) -> serde_json::Value {
    let description = alert.details.clone();
    let timestamp = alert.created_at.clone();
    serde_json::json!({
        "id": alert.id,
        "device_id": alert.device_id,
        "severity": alert.severity,
        "alert_type": alert.alert_type,
        "details": alert.details,
        "created_at": alert.created_at,
        "acknowledged": alert.acknowledged,
        "title": alert.alert_type,
        "description": description,
        "source": "proxybot",
        "timestamp": timestamp,
    })
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
    use crate::alerts::{AlertType, NewAlert};
    use crate::db::DbState;
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn create_test_state() -> Arc<McpState> {
        // Create an in-memory database for testing
        let conn = Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        Arc::new(McpState {
            db: Arc::new(DbState {
                conn: Mutex::new(conn),
            }),
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
        assert_eq!(
            result.get("protocolVersion").and_then(|v| v.as_str()),
            Some("2024-11-05")
        );
        assert_eq!(
            result
                .get("serverInfo")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str()),
            Some("proxybot")
        );
        assert_eq!(
            result
                .get("serverInfo")
                .and_then(|v| v.get("version"))
                .and_then(|v| v.as_str()),
            Some(env!("CARGO_PKG_VERSION"))
        );
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
        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .expect("tools array expected");
        assert_eq!(tools.len(), 6);

        let tool_names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|v| v.as_str()))
            .collect();
        assert!(tool_names.contains(&"capture_traffic"));
        assert!(tool_names.contains(&"classify_request"));
        assert!(tool_names.contains(&"apply_rule"));
        assert!(tool_names.contains(&"get_devices"));
        assert!(tool_names.contains(&"get_alerts"));
        assert!(tool_names.contains(&"acknowledge_alert"));
    }

    #[test]
    fn desktop_domain_and_mcp_share_published_and_acknowledged_alerts() {
        let state = create_test_state();
        let published = state
            .db
            .publish_alert(NewAlert {
                device_id: None,
                severity: AlertSeverity::Warning,
                alert_type: AlertType::AuthAnomaly,
                details: "shared fact".to_owned(),
                occurrence_key: Some("cross-adapter-alert".to_owned()),
            })
            .unwrap();
        let server = McpServer::new(state.clone());

        let read = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_owned(),
            params: Some(serde_json::json!({
                "name": "get_alerts",
                "arguments": {"limit": 10}
            })),
        });
        let observed = &read.result.unwrap()["alerts"][0];
        assert_eq!(observed["id"], published.id);
        assert_eq!(observed["severity"], "Warning");
        assert_eq!(observed["alert_type"], "AuthAnomaly");
        assert_eq!(observed["acknowledged"], false);

        let acknowledged = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: Some(serde_json::json!(2)),
            method: "tools/call".to_owned(),
            params: Some(serde_json::json!({
                "name": "acknowledge_alert",
                "arguments": {"alert_id": published.id}
            })),
        });
        assert_eq!(acknowledged.result.unwrap()["alert"]["acknowledged"], true);
        assert!(state.db.alerts(&AlertQuery::default()).unwrap()[0].acknowledged);
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
        let result = resp
            .result
            .expect("capture_traffic should use the production schema");
        assert_eq!(result["requests"], serde_json::json!([]));
    }

    #[test]
    fn test_classify_wechat() {
        let classifier = ApplicationClassifier::default();
        let result = classifier
            .classify_request("wxapi.weixin.qq.com", None, None)
            .unwrap();
        assert_eq!(result.app_name, "WeChat");
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_classify_douyin() {
        let classifier = ApplicationClassifier::default();
        let result = classifier
            .classify_request("dm.tiktokv.com", None, None)
            .unwrap();
        assert_eq!(result.app_name, "Douyin");
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_classify_alipay() {
        let classifier = ApplicationClassifier::default();
        let result = classifier
            .classify_request("pay.alipay.com", None, None)
            .unwrap();
        assert_eq!(result.app_name, "Alipay");
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_classify_unknown() {
        let classifier = ApplicationClassifier::default();
        assert!(classifier
            .classify_request("random.unknown-site.com", None, None)
            .is_none());
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
// -------------------------------------------------------------------
// Security regression tests for SQL-injection fixes in #3.
//
// Verify that the parameterised queries in `tool_capture_traffic` and
// `tool_get_alerts` survive hostile input that would have broken the
// previous `format!`-based implementation.
// -------------------------------------------------------------------
#[cfg(test)]
mod sql_injection_regression {
    use super::*;
    use crate::db::DbState;
    use std::sync::{Arc, Mutex};

    /// Build an in-memory `McpServer` with a freshly-seeded schema.
    /// One http_requests row + one alerts row are inserted so the
    /// queries have something to return.
    fn fresh_server() -> (McpServer, Arc<McpState>) {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();

        // Insert a known http_requests row via the public API so we
        // automatically pick up the latest migration columns without
        // hard-coding the schema in the test.
        crate::db::record_http_request(
            &conn,
            "1718200000.000",
            "GET",
            "https",
            "api.example.com",
            "/v1/users",
            &[],
            None,
            Some(200),
            &[],
            None,
            Some(42),
            None,
            Some("Example"),
            None,
        )
        .unwrap();

        conn.execute(
            "INSERT INTO alerts (severity, alert_type, details, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["info", r#""NewDomain""#, "baseline", "2024-01-01 00:00:00"],
        )
        .unwrap();

        let db_state = Arc::new(DbState {
            conn: Mutex::new(conn),
        });
        let state = Arc::new(McpState::new(db_state));
        (McpServer::new(state.clone()), state)
    }

    /// Hostile `since` value: would have closed the `WHERE` clause and
    /// appended a `UNION SELECT` under the old format!-based code.
    /// With the parameterised query it is bound as a value and the
    /// string is treated as data, not SQL.
    const HOSTILE_SINCE: &str = "2024-01-01' OR 1=1 --";

    #[test]
    fn capture_traffic_neutralises_sql_injection_in_since() {
        let (server, state) = fresh_server();
        let args: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "since": HOSTILE_SINCE,
                "limit": 10,
            }))
            .unwrap();

        let resp = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "capture_traffic",
                "arguments": args,
            })),
        });

        // The hostile input must NOT cause an SQL error. It should
        // either be bound (returning 0 rows because the literal
        // string is not a valid timestamp) or be ignored — either
        // way the JSON-RPC call returns a successful result, not an
        // internal_error.
        let result = resp
            .result
            .expect("call should succeed; got error response");
        assert!(
            result.get("requests").is_some(),
            "missing requests field: {}",
            result
        );

        // Sanity: no alerts data should have leaked into the
        // http_requests response (which would happen if the UNION
        // had executed).
        let requests = result.get("requests").unwrap().as_array().unwrap();
        for r in requests {
            assert!(
                r.get("severity").is_none(),
                "alerts columns leaked into http_requests: {}",
                r
            );
        }

        // And the table state must be unchanged — the injection did
        // not execute its side-effects.
        let conn = state.db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM alerts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "alerts table was mutated by the injection");
    }

    #[test]
    fn capture_traffic_clamps_oversize_limit() {
        let (server, _state) = fresh_server();
        let args: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "limit": u64::MAX,
            }))
            .unwrap();
        let resp = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({"name": "capture_traffic", "arguments": args})),
        });
        let result = resp.result.expect("call should succeed");
        // Even with a huge limit we must never return more than 1000.
        let total = result.get("total").and_then(|v| v.as_u64()).unwrap();
        assert!(total <= 1000, "limit was not clamped: total={}", total);
    }

    #[test]
    fn get_alerts_neutralises_sql_injection_in_since() {
        let (server, state) = fresh_server();
        let args: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "since": HOSTILE_SINCE,
                "limit": 10,
            }))
            .unwrap();
        let resp = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({"name": "get_alerts", "arguments": args})),
        });
        let result = resp
            .result
            .expect("call should succeed; got error response");
        // The parameterised query is a no-op (no rows match the
        // literal string), so we get a clean empty list — never the
        // internals of the DB or an error.
        assert!(
            result.get("alerts").is_some(),
            "missing alerts field: {}",
            result
        );
        let alerts = result.get("alerts").unwrap().as_array().unwrap();
        assert!(alerts.is_empty(), "expected 0 alerts, got {}", alerts.len());

        // And the table state must be unchanged.
        let conn = state.db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM alerts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn get_alerts_rejects_oversize_since() {
        let (server, _state) = fresh_server();
        // 65-character `since` is rejected by the length cap, so the
        // query runs without the filter — the seeded row matches.
        let huge = "x".repeat(65);
        let args: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "since": huge,
            }))
            .unwrap();
        let resp = server.handle_request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({"name": "get_alerts", "arguments": args})),
        });
        let result = resp.result.expect("call should succeed");
        let alerts = result.get("alerts").unwrap().as_array().unwrap();
        // Length-capped `since` is dropped → the WHERE clause is
        // absent, so the seeded row is returned.
        assert_eq!(alerts.len(), 1);
    }
}
