//! Proxy listener lifecycle: bind, accept, dispatch to per-connection handler,
//! shut down. Also the Tauri- and TUI-flavored start helpers that wire up
//! broadcast channels and the global PROXY_RUNNING flag.

use super::handler::handle_client;
use super::{BreakpointRequest, InterceptedRequest, WsFrameEvent, PROXY_RUNNING};
use crate::cert::CertManager;
use crate::config::proxy_port;
use crate::db::DbState;
use crate::dns::DnsState;
use crate::metrics::counters::METRICS;
use crate::network::NetworkConditionEngine;
use crate::plugin::registry::PluginRegistry;
use crate::plugin::PluginDispatchEngine;
use crate::rules::RulesEngine;
use crate::scripting::ScriptEngine;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

// Runtime dependencies are explicit here so desktop and headless bootstrap
// paths cannot silently receive different services.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_proxy(
    event_tx: broadcast::Sender<InterceptedRequest>,
    breakpoint_tx: tokio::sync::mpsc::Sender<BreakpointRequest>,
    ws_frame_tx: broadcast::Sender<(String, super::WsFrame)>,
    cert_manager: Arc<CertManager>,
    dns_state: Arc<DnsState>,
    db_state: Arc<DbState>,
    rules_engine: Arc<RulesEngine>,
    plugins: Arc<PluginRegistry>,
    plugin_rules: Arc<PluginDispatchEngine>,
    network: Arc<NetworkConditionEngine>,
    scripts: Arc<ScriptEngine>,
    metrics: Arc<crate::metrics::ProxyMetrics>,
    active_session_id: Arc<std::sync::Mutex<Option<String>>>,
    tls_rules: Arc<std::sync::RwLock<proxybot_core::TlsRuleSet>>,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    use super::ProxyContext;

    let addr = format!("0.0.0.0:{}", proxy_port());
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    log::info!("Proxy listening on {}", addr);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, client_addr)) => {
                        metrics.connections_total.fetch_add(1, Ordering::Relaxed);
                        metrics.connections_active.fetch_add(1, Ordering::Relaxed);
                        let ctx = ProxyContext {
                            event_tx: event_tx.clone(),
                            breakpoint_tx: breakpoint_tx.clone(),
                            ws_frame_tx: ws_frame_tx.clone(),
                            cert_manager: cert_manager.clone(),
                            dns_state: dns_state.clone(),
                            db_state: db_state.clone(),
                            rules_engine: rules_engine.clone(),
                            plugins: plugins.clone(),
                            plugin_rules: plugin_rules.clone(),
                            network: network.clone(),
                            scripts: scripts.clone(),
                            metrics: metrics.clone(),
                            active_session_id: active_session_id.clone(),
                            tls_rules: tls_rules.clone(),
                        };
                        let m = metrics.clone();
                        tokio::spawn(async move {
                            handle_client(ctx, stream, client_addr).await;
                            m.connections_closed.fetch_add(1, Ordering::Relaxed);
                            m.connections_active.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    Err(e) => {
                        log::error!("Accept failed: {}", e);
                    }
                }
            }
            _ = &mut shutdown_rx => {
                log::info!("Proxy shutdown signal received");
                break;
            }
        }
    }
    Ok(())
}

const EXAMPLE_SCRIPT: &str = r#"// ProxyBot Example Script
// Return true to allow requests, false to block them.
// Available scope variables: method, scheme, host, path, query_params, status, resp_body

// Log all requests
log(`Request: ${method} ${host}${path}`);

// Block TikTok domains
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
    if let Err(e) = std::fs::create_dir_all(&scripts_dir) {
        log::error!("Failed to create scripts directory: {}", e);
    }
    let example_path = scripts_dir.join("example.rhai");
    if !example_path.exists() {
        if let Err(e) = std::fs::write(&example_path, EXAMPLE_SCRIPT) {
            log::error!("Failed to write example script: {}", e);
        }
    }
    if let Err(e) = scripts.load_dir(&scripts_dir) {
        log::error!("Failed to load scripts: {}", e);
    }
}

#[tauri::command]
pub fn start_proxy(
    app_handle: AppHandle,
    cert_manager: State<'_, Arc<CertManager>>,
    dns_state: State<'_, Arc<DnsState>>,
    db_state: State<'_, Arc<DbState>>,
    rules_engine: State<'_, Arc<RulesEngine>>,
    app_state: State<'_, Arc<crate::state::AppState>>,
) -> Result<String, String> {
    // Prevent starting proxy multiple times
    if PROXY_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("Proxy is already running".to_string());
    }

    let cm = cert_manager.inner().clone();
    let ds = dns_state.inner().clone();
    let db = db_state.inner().clone();
    let re = rules_engine.inner().clone();
    // Share the AppState's active-session Arc so updates from the UI
    // (`set_active_session` command) are visible to the capture path
    // without restarting the proxy.
    let active_session_id = app_state.inner().active_session_id.clone();
    // Same sharing trick for the per-host TLS policy: edits via the
    // tls_rules commands rebuild this set so the HTTPS handler sees
    // them without a proxy restart.
    let tls_rules = app_state.inner().tls_rules.clone();

    // Create broadcast channel for events
    let (event_tx, mut event_rx) = broadcast::channel::<InterceptedRequest>(100);

    // Create broadcast channel for live WS frame events. Subscribed
    // once below and forwarded as `ws-frame:new` Tauri events.
    let (ws_frame_tx, mut ws_frame_rx) = broadcast::channel::<(String, super::WsFrame)>(256);

    // Create empty plugin registry (stub - plugins not yet registered)
    let plugins = Arc::new(PluginRegistry::new());

    // Create plugin rule engine (loads rules from config if available)
    let plugin_rules = Arc::new(PluginDispatchEngine::new());

    // Create network condition engine
    let network_engine = Arc::new(NetworkConditionEngine::new());

    // Create metrics (use global singleton so CLI metrics command sees the same counters)
    let metrics = METRICS.clone();

    // Create scripting engine and load scripts from ~/.proxybot/scripts/
    let scripts = Arc::new(ScriptEngine::new());
    load_or_create_example_scripts(&scripts);

    // Breakpoint channel: rules engine sends BreakpointRequest into
    // bp_tx; the bridge task below routes them into AppState and
    // notifies the UI.
    let (bp_tx, mut bp_rx) = tokio::sync::mpsc::channel::<BreakpointRequest>(100);

    // Spawn task to forward events to Tauri frontend
    let app_handle_clone = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        while let Ok(req) = event_rx.recv().await {
            let _ = app_handle_clone.emit("intercepted-request", &req);
        }
    });

    // Bridge: read bp_rx, stash each in AppState, emit a Tauri event
    // so the UI panel wakes up. The oneshot stays in AppState until
    // resolve_breakpoint sends a decision through it.
    let bp_app_handle = app_handle.clone();
    let bp_state = app_state.inner().clone();
    tauri::async_runtime::spawn(async move {
        while let Some(bp) = bp_rx.recv().await {
            let id =
                bp_state.insert_breakpoint(bp.target.clone(), bp.request.clone(), bp.decision_tx);
            let _ = bp_app_handle.emit(
                "breakpoint:new",
                serde_json::json!({
                    "id": id,
                    "target": bp.target,
                    "request": bp.request,
                }),
            );
        }
    });

    // Spawn task to forward live WS frames to the frontend
    // as `ws-frame:new` events. Payload shape: { request_id, frame }
    let ws_app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        while let Ok((request_id, frame)) = ws_frame_rx.recv().await {
            let _ = ws_app_handle.emit("ws-frame:new", WsFrameEvent { request_id, frame });
        }
    });

    // AI token tracking - subscribe to request events
    let mut ai_event_rx = event_tx.subscribe();
    let ai_tracker = Arc::new(crate::ai::AiTracker::new(db.clone()));
    tauri::async_runtime::spawn(async move {
        loop {
            match ai_event_rx.recv().await {
                Ok(req) => {
                    ai_tracker.process_request(&req);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    log::warn!("AI tracker lagged by {} messages", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    tauri::async_runtime::spawn(async move {
        // Keep shutdown_tx alive by dropping it at the end
        let _shutdown_tx = shutdown_tx;
        if let Err(e) = run_proxy(
            event_tx,
            bp_tx,
            ws_frame_tx,
            cm,
            ds,
            db,
            re,
            plugins,
            plugin_rules,
            network_engine,
            scripts,
            metrics,
            active_session_id,
            tls_rules,
            shutdown_rx,
        )
        .await
        {
            log::error!("Proxy error: {}", e);
        }
        PROXY_RUNNING.store(false, Ordering::SeqCst);
    });

    Ok(format!("Proxy starting on port {}", proxy_port()))
}

#[tauri::command]
pub fn get_proxy_status() -> bool {
    PROXY_RUNNING.load(Ordering::SeqCst)
}

pub type ProxyCoreChannels = (
    broadcast::Receiver<InterceptedRequest>,
    tokio::sync::mpsc::Receiver<BreakpointRequest>,
    tokio::sync::oneshot::Sender<()>,
);

/// Start the proxy core for TUI (no Tauri dependency).
/// Creates a broadcast channel and returns the receiver so TUI can subscribe to events.
pub fn start_proxy_core(
    cert_manager: Arc<CertManager>,
    dns_state: Arc<DnsState>,
    db_state: Arc<DbState>,
    rules_engine: Arc<RulesEngine>,
    plugins: Arc<PluginRegistry>,
    plugin_rules: Arc<PluginDispatchEngine>,
    network_engine: Arc<NetworkConditionEngine>,
) -> Result<ProxyCoreChannels, String> {
    if PROXY_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("Proxy already running".to_string());
    }

    let (event_tx, event_rx) = broadcast::channel::<InterceptedRequest>(100);
    let (ws_frame_tx, _ws_frame_rx) = broadcast::channel::<(String, super::WsFrame)>(256);
    let (bp_tx, bp_rx) = tokio::sync::mpsc::channel::<BreakpointRequest>(100);
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let cm = cert_manager.clone();
    let ds = dns_state.clone();
    let db = db_state.clone();
    let ne = network_engine.clone();
    let metrics = METRICS.clone();
    // TUI startup has no UI to drive session switches — start with
    // an empty Arc and leave it permanently None. Captures land
    // with NULL `session_id` and surface via `get_traffic_records("")`.
    let active_session_id = Arc::new(std::sync::Mutex::new(None));
    // TUI has no rule-editing UI either — start with an empty rule
    // set so every host is decrypted (today's behaviour).
    let tls_rules = Arc::new(std::sync::RwLock::new(proxybot_core::TlsRuleSet::default()));

    // Create scripting engine and load scripts from ~/.proxybot/scripts/
    let scripts = Arc::new(ScriptEngine::new());
    load_or_create_example_scripts(&scripts);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = run_proxy(
                event_tx,
                bp_tx,
                ws_frame_tx,
                cm,
                ds,
                db,
                rules_engine,
                plugins,
                plugin_rules,
                ne,
                scripts,
                metrics,
                active_session_id,
                tls_rules,
                shutdown_rx,
            )
            .await
            {
                log::error!("Proxy error: {}", e);
            }
            PROXY_RUNNING.store(false, Ordering::SeqCst);
        });
    });

    Ok((event_rx, bp_rx, shutdown_tx))
}
