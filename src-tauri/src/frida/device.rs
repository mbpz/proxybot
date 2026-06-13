//! Device and process types for Frida.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceType {
    Usb,
    Remote,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub is_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub identifier: String,
    pub icon: Option<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_serialization() {
        let device = DeviceInfo {
            id: "usb-1234".to_string(),
            name: "Pixel 6".to_string(),
            device_type: DeviceType::Usb,
            is_connected: true,
        };
        let json = serde_json::to_string(&device).unwrap();
        assert!(json.contains("\"id\":\"usb-1234\""));
        assert!(json.contains("\"name\":\"Pixel 6\""));
        assert!(json.contains("\"device_type\":\"Usb\""));
        assert!(json.contains("\"is_connected\":true"));
    }

    #[test]
    fn test_process_info_serialization() {
        let proc = ProcessInfo {
            pid: 1234,
            name: "com.example.app".to_string(),
            identifier: "com.example.app".to_string(),
            icon: None,
        };
        let json = serde_json::to_string(&proc).unwrap();
        assert!(json.contains("\"pid\":1234"));
        assert!(json.contains("\"name\":\"com.example.app\""));
    }

    #[test]
    fn test_device_type_serialization() {
        let usb = DeviceType::Usb;
        let json = serde_json::to_string(&usb).unwrap();
        assert_eq!(json, "\"Usb\"");

        let remote = DeviceType::Remote;
        let json = serde_json::to_string(&remote).unwrap();
        assert_eq!(json, "\"Remote\"");

        let local = DeviceType::Local;
        let json = serde_json::to_string(&local).unwrap();
        assert_eq!(json, "\"Local\"");
    }
}
