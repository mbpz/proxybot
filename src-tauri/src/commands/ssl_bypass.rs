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
    app_handle: tauri::AppHandle,
    state: State<'_, FridaState>,
) -> Result<SessionHandle, String> {
    let script = bypass_scripts::get_script(&script_id)
        .or_else(|| custom_scripts::load_custom_scripts().into_iter().find(|s| s.id == script_id))
        .ok_or_else(|| format!("Script '{}' not found", script_id))?;
    state
        .0
        .attach_and_inject(&device_id, pid, &script.script_content, &app_handle)
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

/// Patch an APK by injecting the Frida Gadget + a selected bypass script.
///
/// Looks up `script_id` against the built-in scripts first, then falls
/// back to user-saved custom scripts. Requires `apktool.jar` and
/// `libfrida-gadget.so` (arm64-v8a) to be present at the resource paths
/// resolved by `ApkPatcher::new()`.
#[tauri::command]
pub fn patch_apk(apk_path: String, script_id: String) -> Result<String, String> {
    use crate::ssl_bypass::apk_patcher::ApkPatcher;

    // Look up script (built-in or custom)
    let script = bypass_scripts::get_script(&script_id)
        .or_else(|| {
            custom_scripts::load_custom_scripts()
                .into_iter()
                .find(|s| s.id == script_id)
        })
        .ok_or_else(|| format!("Script '{}' not found", script_id))?;

    let patcher = ApkPatcher::new()?;
    patcher.patch_apk(&apk_path, &script.script_content)
}