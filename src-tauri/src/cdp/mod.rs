// src-tauri/src/cdp/mod.rs

use serde::{Deserialize, Serialize};

/// Chrome DevTools Protocol message
#[derive(Debug, Deserialize)]
pub struct CdpMessage {
    pub id: Option<i64>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// Chrome DevTools Protocol response
#[derive(Debug, Serialize)]
pub struct CdpResponse {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CdpError>,
}

#[derive(Debug, Serialize)]
pub struct CdpError {
    pub code: i64,
    pub message: String,
}

/// CDP event for broadcasting to clients
#[derive(Debug, Serialize)]
pub struct CdpEvent {
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdp_message_parse() {
        let msg = r#"{"id":1,"method":"Page.enable","params":{}}"#;
        let parsed: CdpMessage = serde_json::from_str(msg).unwrap();
        assert_eq!(parsed.id, Some(1));
        assert_eq!(parsed.method, "Page.enable");
    }

    #[test]
    fn test_cdp_response_serialize() {
        let resp = CdpResponse {
            id: 1,
            result: Some(serde_json::json!({"enabled": true})),
            error: None,
        };
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("\"id\":1"));
        assert!(s.contains("\"result\""));
    }
}
