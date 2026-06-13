//! Frida runtime integration.
//!
//! Manages device enumeration, process listing, session lifecycle,
//! and script injection using the frida-rust crate.
//!
//! `FridaManager` owns a `Frida` handle. A fresh `DeviceManager` is
//! created on each call rather than cached, because the frida 0.14
//! `DeviceManager` carries a raw `*mut` pointer (no `Send`/`Sync`)
//! and we need to share the manager across Tauri command threads.
//! The cost is negligible — `DeviceManager::obtain` just creates a
//! new FFI handle, and frida-core deduplicates work internally.
//!
//! Sessions opened by [`FridaManager::attach_and_inject`] are
//! tracked in a `HashMap` keyed by UUID so the caller (Tauri
//! command layer) can refer to them across IPC boundaries.

pub mod device;
pub mod session;

use std::collections::HashMap;
use std::sync::Mutex;

use frida::{DeviceManager, DeviceType as FridaDeviceType, Frida, ScriptOption};

use crate::frida::device::{DeviceInfo, DeviceType, ProcessInfo};
use crate::frida::session::SessionHandle;

/// Frida runtime manager.
///
/// Constructed once and shared by the Tauri command layer. Thread-safe
/// via an internal `Mutex` on the session table. The underlying
/// `Frida` handle is a zero-sized marker type and is `Send + Sync`.
pub struct FridaManager {
    frida: Frida,
    sessions: Mutex<HashMap<String, SessionHandle>>,
}

impl FridaManager {
    /// Create a new `FridaManager`.
    ///
    /// Initializes the Frida runtime via `Frida::obtain` (which is
    /// `unsafe` in the upstream crate — the runtime must be
    /// initialized before any other frida calls). Per the upstream
    /// contract, calling `obtain` multiple times is a no-op after
    /// the first call.
    pub fn new() -> Result<Self, String> {
        // SAFETY: `Frida::obtain` initializes the frida-core runtime.
        // It is safe to call multiple times; subsequent calls are
        // no-ops per the upstream contract.
        let frida = unsafe { Frida::obtain() };
        Ok(Self {
            frida,
            sessions: Mutex::new(HashMap::new()),
        })
    }

    /// Create a fresh `DeviceManager` bound to our `Frida` handle.
    fn device_manager(&self) -> DeviceManager<'_> {
        DeviceManager::obtain(&self.frida)
    }

    /// List all Frida devices (USB, remote, local).
    pub fn list_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        let mgr = self.device_manager();
        let devices = mgr.enumerate_all_devices();
        Ok(devices
            .into_iter()
            .map(|d| DeviceInfo {
                id: d.get_id().to_string(),
                name: d.get_name().to_string(),
                device_type: map_device_type(d.get_type()),
                is_connected: !d.is_lost(),
            })
            .collect())
    }

    /// Enumerate running processes on a specific device.
    pub fn list_processes(&self, device_id: &str) -> Result<Vec<ProcessInfo>, String> {
        let mgr = self.device_manager();
        let device = mgr
            .get_device_by_id(device_id)
            .map_err(|e| format!("device '{}' not found: {}", device_id, e))?;
        let processes = device.enumerate_processes();
        Ok(processes
            .into_iter()
            .map(|p| ProcessInfo {
                pid: p.get_pid(),
                name: p.get_name().to_string(),
                // The frida 0.14 `Process` type only exposes
                // `get_name()` and `get_pid()`. The upstream
                // frida-core `FridaProcess` does carry an
                // identifier (parameters), but the Rust binding
                // in 0.14 does not surface it. Reuse the name
                // as a best-effort identifier for the UI.
                identifier: p.get_name().to_string(),
                icon: None,
            })
            .collect())
    }

    /// Attach to a process and inject a Frida script.
    ///
    /// Returns a [`SessionHandle`] with a freshly-generated UUID that
    /// the caller can use to refer to the session for later detach.
    pub fn attach_and_inject(
        &self,
        device_id: &str,
        pid: u32,
        script_content: &str,
    ) -> Result<SessionHandle, String> {
        let mut script_option = ScriptOption::new();
        let mgr = self.device_manager();
        let device = mgr
            .get_device_by_id(device_id)
            .map_err(|e| format!("device '{}' not found: {}", device_id, e))?;
        let session = device
            .attach(pid)
            .map_err(|e| format!("failed to attach to pid {}: {}", pid, e))?;
        let script = session
            .create_script(script_content, &mut script_option)
            .map_err(|e| format!("failed to create script: {}", e))?;
        script
            .load()
            .map_err(|e| format!("failed to load script: {}", e))?;

        let handle = SessionHandle {
            session_id: uuid::Uuid::new_v4().to_string(),
            device_id: device_id.to_string(),
            pid,
            process_name: String::new(),
            attached_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        self.sessions
            .lock()
            .map_err(|e| format!("sessions lock poisoned: {}", e))?
            .insert(handle.session_id.clone(), handle.clone());
        Ok(handle)
    }

    /// Detach a previously-opened session.
    ///
    /// In the frida 0.14 binding, `Session` borrows from the
    /// `Device` and is not retained in our session table — we only
    /// know the `session_id`. Calling `detach` removes the
    /// bookkeeping entry. The actual `Session` is dropped when the
    /// underlying `Device` handle is dropped (or refreshed by a
    /// future device manager call). For a fuller detach story we
    /// would need to store the `Session` itself; that is deferred
    /// to a follow-up when the IPC layer signals "I'm done with
    /// this session" and the lifetime story is better understood.
    pub fn detach(&self, session_id: &str) -> Result<(), String> {
        self.sessions
            .lock()
            .map_err(|e| format!("sessions lock poisoned: {}", e))?
            .remove(session_id);
        Ok(())
    }
}

/// Map frida's `DeviceType` to our public `DeviceType`.
///
/// frida 0.14 uses `USB` (all-caps acronym); our public type uses
/// `Usb` to match the rest of the project. Other variants share
/// the same PascalCase names. The upstream enum is
/// `#[non_exhaustive]`, so we must include a wildcard — fall back
/// to `Local` for any future device type the frida crate adds.
fn map_device_type(t: FridaDeviceType) -> DeviceType {
    match t {
        FridaDeviceType::Local => DeviceType::Local,
        FridaDeviceType::Remote => DeviceType::Remote,
        FridaDeviceType::USB => DeviceType::Usb,
        _ => DeviceType::Local,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_device_type_local() {
        assert_eq!(map_device_type(FridaDeviceType::Local), DeviceType::Local);
    }

    #[test]
    fn map_device_type_remote() {
        assert_eq!(
            map_device_type(FridaDeviceType::Remote),
            DeviceType::Remote
        );
    }

    #[test]
    fn map_device_type_usb() {
        assert_eq!(map_device_type(FridaDeviceType::USB), DeviceType::Usb);
    }

    #[test]
    fn new_does_not_panic() {
        // Constructing the manager must not panic. `Frida::obtain`
        // is unsafe but documented as safe to call multiple times.
        let result = FridaManager::new();
        assert!(
            result.is_ok(),
            "FridaManager::new failed: {:?}",
            result.err()
        );
    }
}
