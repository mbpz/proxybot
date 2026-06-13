//! Frida runtime integration.
//!
//! Manages device enumeration, process listing, session lifecycle,
//! and script injection using the frida-rust crate.
//!
//! The FridaManager is a stub in this task — Task 6 wires the real
//! frida crate. The types in `device` and `session` are usable.

pub mod device;
pub mod session;

use std::collections::HashMap;
use std::sync::Mutex;

use crate::frida::device::DeviceInfo;
use crate::frida::session::SessionHandle;

pub struct FridaManager {
    devices: Mutex<Vec<DeviceInfo>>,
    sessions: Mutex<HashMap<String, SessionHandle>>,
}

impl FridaManager {
    pub fn new() -> Self {
        Self {
            devices: Mutex::new(Vec::new()),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        Ok(self.devices.lock().map_err(|e| e.to_string())?.clone())
    }

    pub fn attach(
        &self,
        device_id: String,
        pid: u32,
        script_content: String,
    ) -> Result<SessionHandle, String> {
        let handle = SessionHandle {
            session_id: uuid::Uuid::new_v4().to_string(),
            device_id,
            pid,
            process_name: String::new(),
            attached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        self.sessions
            .lock()
            .map_err(|e| e.to_string())?
            .insert(handle.session_id.clone(), handle.clone());
        Ok(handle)
    }

    pub fn detach(&self, session_id: &str) -> Result<(), String> {
        self.sessions
            .lock()
            .map_err(|e| e.to_string())?
            .remove(session_id);
        Ok(())
    }
}
