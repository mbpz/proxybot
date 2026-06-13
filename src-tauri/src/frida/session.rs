//! Frida session types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHandle {
    pub session_id: String,
    pub device_id: String,
    pub pid: u32,
    pub process_name: String,
    pub attached_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_handle_serialization() {
        let handle = SessionHandle {
            session_id: "sess-123".to_string(),
            device_id: "usb-1234".to_string(),
            pid: 5678,
            process_name: "com.example.app".to_string(),
            attached_at: 1718200000,
        };
        let json = serde_json::to_string(&handle).unwrap();
        assert!(json.contains("\"session_id\":\"sess-123\""));
        assert!(json.contains("\"device_id\":\"usb-1234\""));
        assert!(json.contains("\"pid\":5678"));
        assert!(json.contains("\"process_name\":\"com.example.app\""));
        assert!(json.contains("\"attached_at\":1718200000"));
    }

    #[test]
    fn test_session_handle_clone() {
        let handle = SessionHandle {
            session_id: "sess-1".to_string(),
            device_id: "dev-1".to_string(),
            pid: 100,
            process_name: "app".to_string(),
            attached_at: 0,
        };
        let cloned = handle.clone();
        assert_eq!(cloned.session_id, "sess-1");
        assert_eq!(cloned.pid, 100);
    }
}
