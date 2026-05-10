// ProxyBot MCP Server - stdio-based transport for Claude Desktop

pub mod server;

pub mod transport;

pub use server::McpServer;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// MCP server state shared across all tool handlers
pub struct McpState {
    pub db: Arc<crate::db::DbState>,
}

impl McpState {
    pub fn new(db: Arc<crate::db::DbState>) -> Self {
        Self { db }
    }

    /// Create an insecure McpState for CLI stdio mode.
    /// Uses an in-memory database that doesn't persist data.
    pub fn new_insecure() -> Self {
        Self {
            db: Arc::new(
                crate::db::DbState::new_in_memory(std::sync::Mutex::new(()))
                    .expect("Failed to create in-memory database"),
            ),
        }
    }
}

// JSON-RPC 2.0 types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    pub fn invalid_params(msg: &str) -> Self {
        Self {
            code: -32602,
            message: msg.to_string(),
            data: None,
        }
    }

    pub fn internal_error(msg: &str) -> Self {
        Self {
            code: -32603,
            message: msg.to_string(),
            data: None,
        }
    }
}

impl JsonRpcResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, err: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_deserialize() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/list");
        assert!(req.params.is_some());
    }

    #[test]
    fn test_jsonrpc_request_without_id() {
        let json = r#"{"jsonrpc":"2.0","method":"tools/list"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, None);
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let resp = JsonRpcResponse::success(Some(serde_json::json!(1)), serde_json::json!({"tools": []}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let resp = JsonRpcResponse::error(
            Some(serde_json::json!(1)),
            JsonRpcError::method_not_found("unknown"),
        );
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32601);
    }

    #[test]
    fn test_jsonrpc_error_method_not_found() {
        let err = JsonRpcError::method_not_found("tools/call");
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("tools/call"));
    }

    #[test]
    fn test_jsonrpc_error_invalid_params() {
        let err = JsonRpcError::invalid_params("missing required field");
        assert_eq!(err.code, -32602);
    }

    #[test]
    fn test_mcp_state_creation() {
        // McpState requires DbState, which requires database initialization
        // This test verifies the struct layout compiles correctly
        // Full integration would require a test database setup
    }
}