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

#[cfg(feature = "frida-runtime")]
use std::collections::HashMap;
#[cfg(feature = "frida-runtime")]
use std::sync::Mutex;

#[cfg(feature = "frida-runtime")]
use frida::{
    DeviceManager, DeviceType as FridaDeviceType, Frida, Message, MessageLogLevel, ScriptHandler,
    ScriptOption,
};
#[cfg(feature = "frida-runtime")]
use tauri::Emitter;

#[cfg(feature = "frida-runtime")]
use crate::frida::device::DeviceType;
use crate::frida::device::{DeviceInfo, ProcessInfo};
use crate::frida::session::SessionHandle;

/// A Frida script message streamed to the UI via the `frida:message`
/// Tauri event.
///
/// Generated from frida 0.14 `Message` variants inside
/// `FridaManager::attach_and_inject`. The frontend renders these in
/// the live log panel.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FridaMessage {
    /// The log level or category name. For `Message::Log` this is
    /// "info" / "debug" / "warning" / "error". For `Message::Error`
    /// it is always "error". For other variants it is "info".
    pub level: String,
    /// The text payload. For `Message::Error` this includes the
    /// description and stack trace, joined by a newline.
    pub payload: String,
    /// Milliseconds since the Unix epoch when the message was
    /// received from Frida.
    pub timestamp_ms: u64,
}

/// Bridge between frida's `Message` signal and Tauri's event bus.
///
/// Implements `ScriptHandler` so it can be passed to
/// `Script::handle_message`. Clones the `AppHandle` so each
/// injection gets its own listener. The handler is `'static` and
/// `Send` because frida 0.14's signal callbacks can run on a
/// non-Tokio thread.
#[cfg(feature = "frida-runtime")]
struct FridaScriptMessageHandler {
    app_handle: tauri::AppHandle,
}

#[cfg(feature = "frida-runtime")]
impl ScriptHandler for FridaScriptMessageHandler {
    fn on_message(&mut self, message: &Message) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let msg = match message {
            Message::Log(log) => FridaMessage {
                level: log_level_to_string(&log.level).to_string(),
                payload: log.payload.clone(),
                timestamp_ms: now_ms,
            },
            Message::Error(err) => FridaMessage {
                level: "error".to_string(),
                payload: format!("{}\n{}", err.description, err.stack),
                timestamp_ms: now_ms,
            },
            Message::Send(_) => {
                // Send messages are RPC traffic; the spec only asks us
                // to surface log/error to the UI. Skip to avoid
                // noise from list_exports / call traffic.
                return;
            }
            Message::Other(value) => FridaMessage {
                level: "info".to_string(),
                payload: value.to_string(),
                timestamp_ms: now_ms,
            },
        };

        let _ = self.app_handle.emit("frida:message", &msg);
    }
}

#[cfg(feature = "frida-runtime")]
fn log_level_to_string(level: &MessageLogLevel) -> &'static str {
    match level {
        MessageLogLevel::Info => "info",
        MessageLogLevel::Debug => "debug",
        MessageLogLevel::Warning => "warning",
        MessageLogLevel::Error => "error",
    }
}

/// Frida runtime manager.
///
/// Constructed once and shared by the Tauri command layer. Thread-safe
/// via an internal `Mutex` on the session table. The underlying
/// `Frida` handle is a zero-sized marker type and is `Send + Sync`.
#[cfg(feature = "frida-runtime")]
pub struct FridaManager {
    frida: Frida,
    sessions: Mutex<HashMap<String, SessionHandle>>,
}

#[cfg(feature = "frida-runtime")]
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
    ///
    /// `app_handle` is used to install a message handler that emits
    /// each Frida `console.log` / runtime error to the UI as a
    /// `frida:message` Tauri event (spec §9.4).
    pub fn attach_and_inject(
        &self,
        device_id: &str,
        pid: u32,
        script_content: &str,
        app_handle: &tauri::AppHandle,
    ) -> Result<SessionHandle, String> {
        let mut script_option = ScriptOption::new();
        let mgr = self.device_manager();
        let device = mgr
            .get_device_by_id(device_id)
            .map_err(|e| format!("device '{}' not found: {}", device_id, e))?;
        let session = device
            .attach(pid)
            .map_err(|e| format!("failed to attach to pid {}: {}", pid, e))?;
        let mut script = session
            .create_script(script_content, &mut script_option)
            .map_err(|e| format!("failed to create script: {}", e))?;

        // Wire up the message handler BEFORE loading the script so
        // we do not miss the first log line emitted during startup.
        // frida 0.14 uses `handle_message` with a `ScriptHandler`
        // trait impl (not a closure-style `on_message`).
        script
            .handle_message(FridaScriptMessageHandler {
                app_handle: app_handle.clone(),
            })
            .map_err(|e| format!("failed to attach message handler: {}", e))?;

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

/// Frida Adapter used by builds that intentionally omit the native runtime.
///
/// Keeping the same Interface lets the desktop command contract remain stable:
/// callers receive a precise capability error instead of an unregistered IPC
/// command or an application startup failure.
#[cfg(not(feature = "frida-runtime"))]
pub struct FridaManager;

#[cfg(not(feature = "frida-runtime"))]
impl FridaManager {
    const UNAVAILABLE: &'static str =
        "Frida runtime is unavailable; rebuild ProxyBot with the `frida-runtime` feature";

    pub fn new() -> Result<Self, String> {
        Ok(Self)
    }

    pub fn list_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        Err(Self::UNAVAILABLE.to_string())
    }

    pub fn list_processes(&self, _device_id: &str) -> Result<Vec<ProcessInfo>, String> {
        Err(Self::UNAVAILABLE.to_string())
    }

    pub fn attach_and_inject(
        &self,
        _device_id: &str,
        _pid: u32,
        _script_content: &str,
        _app_handle: &tauri::AppHandle,
    ) -> Result<SessionHandle, String> {
        Err(Self::UNAVAILABLE.to_string())
    }

    pub fn detach(&self, _session_id: &str) -> Result<(), String> {
        Err(Self::UNAVAILABLE.to_string())
    }
}

/// Map frida's `DeviceType` to our public `DeviceType`.
///
/// frida 0.14 uses `USB` (all-caps acronym); our public type uses
/// `Usb` to match the rest of the project. Other variants share
/// the same PascalCase names. The upstream enum is
/// `#[non_exhaustive]`, so we must include a wildcard — fall back
/// to `Local` for any future device type the frida crate adds.
#[cfg(feature = "frida-runtime")]
fn map_device_type(t: FridaDeviceType) -> DeviceType {
    match t {
        FridaDeviceType::Local => DeviceType::Local,
        FridaDeviceType::Remote => DeviceType::Remote,
        FridaDeviceType::USB => DeviceType::Usb,
        _ => DeviceType::Local,
    }
}

#[cfg(all(test, feature = "frida-runtime"))]
mod tests {
    use super::*;

    #[test]
    fn map_device_type_local() {
        assert_eq!(map_device_type(FridaDeviceType::Local), DeviceType::Local);
    }

    #[test]
    fn map_device_type_remote() {
        assert_eq!(map_device_type(FridaDeviceType::Remote), DeviceType::Remote);
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

#[cfg(all(test, not(feature = "frida-runtime")))]
mod unavailable_tests {
    use super::*;

    #[test]
    fn adapter_returns_a_stable_capability_error() {
        let manager = FridaManager::new().unwrap();
        let errors = [
            manager.list_devices().unwrap_err(),
            manager.list_processes("local").unwrap_err(),
            manager.detach("session-1").unwrap_err(),
        ];

        assert!(errors
            .iter()
            .all(|error| error == FridaManager::UNAVAILABLE));
    }
}
