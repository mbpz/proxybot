//! Tauri Adapter for starting, observing, and stopping the core MITM Runtime.

use super::runtime_adapter::{bridge_capture_events, DesktopRuntimeHooks, PfOriginalDestination};
use super::BreakpointRequest;
use crate::cert::CertManager;
use crate::db::DbState;
use crate::dns::DnsState;
use crate::network::NetworkConditionEngine;
use crate::plugin::registry::PluginRegistry;
use crate::plugin::PluginDispatchEngine;
use crate::rules::RulesEngine;
use crate::scripting::ScriptEngine;
use crate::state::AppState;
use proxybot_core::{MitmRuntime, RunningMitm, RuntimeConfig};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Desktop ownership of the one live runtime handle.
pub struct MitmRuntimeState {
    running: tokio::sync::Mutex<Option<RunningMitm>>,
}

impl MitmRuntimeState {
    pub fn new() -> Self {
        Self {
            running: tokio::sync::Mutex::new(None),
        }
    }

    pub async fn is_running(&self) -> bool {
        self.running
            .lock()
            .await
            .as_ref()
            .is_some_and(RunningMitm::is_running)
    }
}

impl Default for MitmRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

const EXAMPLE_SCRIPT: &str = r#"// ProxyBot Example Script
// Return true to allow requests, false to block them.
// Available scope variables: method, scheme, host, path, query_params, status, resp_body

log(`Request: ${method} ${host}${path}`);

if host.contains("tiktok") || host.contains("douyin") {
    warn(`Blocked: ${host}`);
    false
} else {
    true
}
"#;

fn load_or_create_example_scripts(scripts: &ScriptEngine) {
    let scripts_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".proxybot")
        .join("scripts");
    if let Err(error) = std::fs::create_dir_all(&scripts_dir) {
        log::error!("Failed to create scripts directory: {error}");
    }
    let example_path = scripts_dir.join("example.rhai");
    if !example_path.exists() {
        if let Err(error) = std::fs::write(&example_path, EXAMPLE_SCRIPT) {
            log::error!("Failed to write example script: {error}");
        }
    }
    if let Err(error) = scripts.load_dir(&scripts_dir) {
        log::error!("Failed to load scripts: {error}");
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn start_proxy_runtime(
    app_handle: AppHandle,
    runtime_state: Arc<MitmRuntimeState>,
    cert_manager: Arc<CertManager>,
    dns_state: Arc<DnsState>,
    db_state: Arc<DbState>,
    rules_engine: Arc<RulesEngine>,
    app_state: Arc<AppState>,
    network: Arc<NetworkConditionEngine>,
) -> Result<String, String> {
    let mut running = runtime_state.running.lock().await;
    if running.as_ref().is_some_and(RunningMitm::is_running) {
        return Err("Proxy is already running".to_owned());
    }
    *running = None;

    let plugins = Arc::new(PluginRegistry::new());
    let plugin_rules = Arc::new(PluginDispatchEngine::new());
    let scripts = Arc::new(ScriptEngine::new());
    load_or_create_example_scripts(&scripts);
    let (breakpoint_tx, mut breakpoint_rx) = tokio::sync::mpsc::channel::<BreakpointRequest>(100);
    let hooks = Arc::new(DesktopRuntimeHooks::new(
        plugins,
        plugin_rules,
        scripts,
        network,
        breakpoint_tx,
    ));

    let runtime = MitmRuntime::new(
        RuntimeConfig::default(),
        cert_manager,
        rules_engine,
        Arc::clone(&app_state.tls_rules),
    )
    .with_hooks(hooks)
    .with_original_destination(Arc::new(PfOriginalDestination));

    // The core Interface returns only after bind succeeds.
    let mut handle = runtime.start().await.map_err(|error| error.to_string())?;
    let bound_addr = handle.bound_addr();
    let events = handle
        .take_events()
        .ok_or_else(|| "MITM Runtime event stream already taken".to_owned())?;

    let event_app = app_handle.clone();
    let event_db = Arc::clone(&db_state);
    let event_dns = Arc::clone(&dns_state);
    let event_state = Arc::clone(&app_state);
    tauri::async_runtime::spawn(async move {
        bridge_capture_events(events, event_app, event_db, event_dns, event_state).await;
    });

    let breakpoint_app = app_handle;
    let breakpoint_state = app_state;
    tauri::async_runtime::spawn(async move {
        while let Some(breakpoint) = breakpoint_rx.recv().await {
            let id = breakpoint_state.insert_breakpoint(
                breakpoint.target.clone(),
                breakpoint.request.clone(),
                breakpoint.decision_tx,
            );
            let _ = breakpoint_app.emit(
                "breakpoint:new",
                serde_json::json!({
                    "id": id,
                    "target": breakpoint.target,
                    "request": breakpoint.request,
                }),
            );
        }
    });

    *running = Some(handle);
    Ok(format!("Proxy listening on {bound_addr}"))
}

pub(crate) async fn stop_proxy_runtime(
    runtime_state: Arc<MitmRuntimeState>,
    app_state: Arc<AppState>,
) -> Result<String, String> {
    let handle = runtime_state
        .running
        .lock()
        .await
        .take()
        .ok_or_else(|| "Proxy is not running".to_owned())?;
    app_state.cancel_all_breakpoints();
    handle.shutdown().await;
    Ok("Proxy stopped".to_owned())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_proxy(
    app_handle: AppHandle,
    runtime_state: State<'_, Arc<MitmRuntimeState>>,
    cert_manager: State<'_, Arc<CertManager>>,
    dns_state: State<'_, Arc<DnsState>>,
    db_state: State<'_, Arc<DbState>>,
    rules_engine: State<'_, Arc<RulesEngine>>,
    app_state: State<'_, Arc<AppState>>,
    network: State<'_, crate::commands::network_conditions::NetworkConditionsState>,
) -> Result<String, String> {
    start_proxy_runtime(
        app_handle,
        runtime_state.inner().clone(),
        cert_manager.inner().clone(),
        dns_state.inner().clone(),
        db_state.inner().clone(),
        rules_engine.inner().clone(),
        app_state.inner().clone(),
        Arc::clone(&network.0),
    )
    .await
}

#[tauri::command]
pub async fn get_proxy_status(runtime: State<'_, Arc<MitmRuntimeState>>) -> Result<bool, String> {
    Ok(runtime.is_running().await)
}

#[tauri::command]
pub async fn stop_proxy(
    runtime: State<'_, Arc<MitmRuntimeState>>,
    app_state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    stop_proxy_runtime(runtime.inner().clone(), app_state.inner().clone()).await
}
