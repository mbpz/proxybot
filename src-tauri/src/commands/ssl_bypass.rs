//! Tauri commands for SSL bypass operations.

use std::sync::Arc;
use tauri::State;

use crate::frida::device::{DeviceInfo, ProcessInfo};
use crate::frida::session::SessionHandle;
use crate::frida::FridaManager;
use crate::ssl_bypass::bypass_scripts;
use crate::ssl_bypass::custom_scripts;

/// Shared FridaManager state, wrapped in Arc for thread-safe sharing.
pub struct FridaState(pub Arc<FridaManager>);

#[tauri::command]
pub fn frida_list_devices(
    state: State<'_, FridaState>,
) -> Result<Vec<DeviceInfo>, String> {
    state.0.list_devices()
}

#[tauri::command]
pub fn frida_list_processes(
    device_id: String,
    state: State<'_, FridaState>,
) -> Result<Vec<ProcessInfo>, String> {
    state.0.list_processes(&device_id)
}

#[tauri::command]
pub fn frida_inject_script(
    device_id: String,
    pid: u32,
    script_id: String,
    state: State<'_, FridaState>,
) -> Result<SessionHandle, String> {
    let script = bypass_scripts::get_script(&script_id)
        .or_else(|| custom_scripts::load_custom_scripts().into_iter().find(|s| s.id == script_id))
        .ok_or_else(|| format!("Script '{}' not found", script_id))?;
    state.0.attach_and_inject(&device_id, pid, &script.script_content)
}

#[tauri::command]
pub fn frida_detach(
    session_id: String,
    state: State<'_, FridaState>,
) -> Result<(), String> {
    state.0.detach(&session_id)
}

#[tauri::command]
pub fn list_bypass_scripts() -> Vec<bypass_scripts::BypassScript> {
    let mut all = bypass_scripts::get_all_builtin_scripts();
    all.extend(custom_scripts::load_custom_scripts());
    all
}

#[tauri::command]
pub fn check_java_installed() -> bool {
    std::process::Command::new("java")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn check_adb_installed() -> bool {
    std::process::Command::new("adb")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}