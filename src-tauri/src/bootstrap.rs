//! Process and desktop application bootstrap.
//!
//! This is the single composition root for ProxyBot. Platform binaries stay
//! deliberately thin: they select a launch mode, while this Module owns state
//! construction, plugins, tray behavior, and the desktop IPC contract.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, State};
use tauri_plugin_notification::NotificationExt;

use crate::anomaly::AnomalyDetector;
use crate::cert::CertManager;
use crate::commands::network_conditions::NetworkConditionsState;
use crate::commands::ssl_bypass::FridaState;
use crate::db::DbState;
use crate::dns::DnsState;
use crate::proxy::ProxyState;
use crate::replay::ReplayState;
use crate::rules::RulesEngine;
use crate::workspace::WorkspaceManager;
use proxybot_core::AppConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
enum LaunchMode {
    Desktop,
    DesktopAcceptance(PathBuf),
    McpStdio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LaunchOptions {
    mode: LaunchMode,
    reverse_target: Option<String>,
}

impl LaunchOptions {
    fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let args: Vec<OsString> = args.into_iter().skip(1).collect();

        // MCP is a separate headless Adapter. Preserve its independence from
        // desktop-only flags so callers can safely pass a shared argument set.
        if args.iter().any(|arg| arg == "--mcp-stdio") {
            return Ok(Self {
                mode: LaunchMode::McpStdio,
                reverse_target: None,
            });
        }

        if let Some(index) = args.iter().position(|arg| arg == "--desktop-acceptance") {
            let workspace = args
                .get(index + 1)
                .ok_or_else(|| "--desktop-acceptance requires an isolated workspace".to_string())?;
            if workspace.is_empty() || workspace.to_string_lossy().starts_with('-') {
                return Err(
                    "--desktop-acceptance requires a non-empty isolated workspace".to_string(),
                );
            }
            return Ok(Self {
                mode: LaunchMode::DesktopAcceptance(PathBuf::from(workspace)),
                reverse_target: None,
            });
        }

        let mut reverse_target = None;
        let mut index = 0;
        while index < args.len() {
            let argument = args[index].to_string_lossy();
            if argument == "--reverse-target" {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--reverse-target requires a URL argument".to_string())?
                    .to_string_lossy()
                    .into_owned();
                if value.is_empty() {
                    return Err("--reverse-target requires a non-empty URL".to_string());
                }
                reverse_target = Some(value);
                index += 2;
                continue;
            }
            if let Some(value) = argument.strip_prefix("--reverse-target=") {
                if value.is_empty() {
                    return Err("--reverse-target requires a non-empty URL".to_string());
                }
                reverse_target = Some(value.to_string());
            }
            index += 1;
        }

        Ok(Self {
            mode: LaunchMode::Desktop,
            reverse_target,
        })
    }

    fn apply_to(&self, config: AppConfig) -> AppConfig {
        match &self.reverse_target {
            Some(target) => config.with_reverse_target(Some(target.clone())),
            None => config,
        }
    }
}

macro_rules! define_desktop_commands {
    ($($command:path),+ $(,)?) => {
        /// Canonical desktop IPC contract. The handler and contract tests are
        /// generated from this same list so they cannot drift independently.
        pub const DESKTOP_COMMANDS: &[&str] = &[$(stringify!($command)),+];

        fn desktop_invoke_handler() ->
            impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static
        {
            tauri::generate_handler![$($command),+]
        }
    };
}

define_desktop_commands![
    crate::proxy::start_proxy,
    crate::proxy::stop_proxy,
    crate::proxy::get_proxy_status,
    crate::proxy::export_cert,
    crate::proxy::get_ca_cert_pem,
    crate::proxy::regenerate_ca,
    crate::proxy::get_ca_metadata,
    crate::proxy::get_network_info,
    crate::proxy::setup_pf,
    crate::proxy::teardown_pf,
    crate::proxy::is_pf_enabled,
    crate::proxy::get_request_detail,
    crate::proxy::load_history,
    crate::proxy::save_history,
    crate::proxy::set_keep_running,
    crate::proxy::get_keep_running,
    crate::proxy::hide_window,
    crate::proxy::replay_request,
    crate::dns::get_dns_log,
    crate::dns::get_dns_upstream,
    crate::dns::set_dns_upstream,
    crate::dns::reload_dns_lists,
    crate::commands::device_setup::prepare_device_onboarding,
    crate::commands::device_setup::stop_device_onboarding,
    crate::anomaly::get_traffic_baseline,
    crate::anomaly::scan_request_anomalies,
    crate::alerts::get_alerts,
    crate::alerts::acknowledge_alert,
    crate::alerts::get_alert_count,
    crate::db::get_db_stats,
    crate::db::get_devices,
    crate::db::register_device,
    crate::db::update_device_last_seen,
    crate::db::update_device_stats,
    crate::db::set_device_rule_override,
    crate::db::get_device_by_mac,
    crate::rules::get_rules,
    crate::rules::save_rule,
    crate::rules::delete_rule,
    crate::rules::reorder_rules,
    crate::rules::list_rule_files,
    crate::rules::match_host,
    crate::har::export_har,
    crate::har::save_har_file,
    crate::commands::compose::compose_request,
    crate::replay::get_replay_targets,
    crate::replay::get_requests_for_replay,
    crate::replay::get_recorded_responses,
    crate::replay::start_replay,
    crate::normalize::get_normalized_traffic,
    crate::normalize::get_traffic_page,
    crate::dag::build_traffic_dag,
    crate::dag::get_traffic_dag,
    crate::dag::get_device_dag,
    crate::commands::graph::get_graph_data,
    build_topology_graph,
    get_topology_node_detail,
    crate::proxy::get_ws_frames,
    crate::infer::infer_api_semantics,
    crate::infer::store_inference_result,
    crate::infer::get_inferred_apis,
    crate::infer::get_openapi_spec,
    crate::infer::generate_openapi_yaml,
    crate::infer::evaluate_inference,
    crate::infer::get_evaluation_result,
    crate::state_machine::get_auth_state_machine,
    crate::mockgen::generate_mock_project,
    crate::mockgen::write_mock_project,
    crate::mockgen::get_mock_endpoints,
    crate::mockgen::start_mock_server,
    crate::scaffoldgen::generate_scaffold_project,
    crate::scaffoldgen::generate_scaffold_with_vision,
    crate::scaffoldgen::write_scaffold_project,
    crate::scaffoldgen::write_scaffold_project_with_vision,
    crate::scaffoldgen::evaluate_scaffold_project,
    crate::vision::analyze_screenshot,
    crate::vision::analyze_screenshot_base64,
    crate::vision::get_vision_analyses,
    crate::vision::delete_vision_analysis,
    crate::vision::fuse_vision_with_api,
    crate::deploy::generate_deployment_bundle,
    crate::deploy::write_deployment_bundle,
    crate::deploy::git_init_deployment,
    crate::deploy::get_last_deployment,
    crate::commands::ai_stats::get_ai_stats,
    crate::commands::ai_stats::get_ai_context_windows,
    start_dashboard,
    stop_dashboard,
    is_dashboard_running,
    get_dashboard_url,
    crate::commands::ssl_bypass::frida_list_devices,
    crate::commands::ssl_bypass::frida_list_processes,
    crate::commands::ssl_bypass::frida_inject_script,
    crate::commands::ssl_bypass::frida_detach,
    crate::commands::ssl_bypass::list_bypass_scripts,
    crate::commands::ssl_bypass::check_java_installed,
    crate::commands::ssl_bypass::check_adb_installed,
    crate::commands::ssl_bypass::patch_apk,
    crate::commands::filter::parse_filter,
    crate::commands::filter::list_filter_presets,
    crate::commands::filter::save_filter_preset,
    crate::commands::filter::delete_filter_preset,
    crate::commands::app_fingerprint::get_app_signatures,
    crate::commands::app_fingerprint::add_custom_rule,
    crate::commands::app_fingerprint::remove_custom_rule,
    crate::commands::network_conditions::get_network_profiles,
    crate::commands::network_conditions::set_active_profile,
    crate::commands::network_conditions::get_active_profile,
    crate::commands::network_conditions::add_condition_rule,
    crate::commands::network_conditions::remove_condition_rule,
    crate::commands::network_conditions::list_condition_rules,
    crate::commands::specgen::generate_spec,
    crate::commands::specgen::export_spec,
    crate::commands::specgen::run_replay_validation,
    crate::commands::specgen::update_specgen_config,
    crate::commands::specgen::get_specgen_config,
    crate::commands::specgen::get_traffic_records,
    crate::commands::specgen::set_active_session,
    crate::commands::specgen::get_active_session,
    crate::commands::tls_rules::get_tls_rules,
    crate::commands::tls_rules::add_tls_rule,
    crate::commands::tls_rules::delete_tls_rule,
    crate::commands::breakpoint::get_pending_breakpoints,
    crate::commands::breakpoint::resolve_breakpoint,
    crate::commands::breakpoint::cancel_all_breakpoints,
    crate::workspace::manager::init_workspace,
    crate::workspace::manager::export_workspace,
    crate::workspace::manager::import_workspace,
    crate::workspace::manager::list_workspaces,
    crate::workspace::manager::switch_workspace,
    crate::workspace::manager::workspace_status,
];

/// Select the process Adapter, then launch the application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let options = LaunchOptions::parse(std::env::args_os()).unwrap_or_else(|error| {
        eprintln!("proxybot: {error}");
        std::process::exit(2);
    });

    let acceptance_workspace = match &options.mode {
        LaunchMode::DesktopAcceptance(workspace) => Some(workspace.clone()),
        _ => None,
    };
    let config = match &acceptance_workspace {
        Some(workspace) => isolated_acceptance_config(workspace.clone()).unwrap_or_else(|error| {
            eprintln!("proxybot: could not isolate desktop acceptance: {error}");
            std::process::exit(2);
        }),
        None => AppConfig::load().unwrap_or_else(|error| {
            eprintln!("proxybot: invalid configuration: {error}");
            std::process::exit(2);
        }),
    };
    let config = options.apply_to(config);

    if matches!(options.mode, LaunchMode::McpStdio) {
        crate::mcp::transport::start_stdio_mode(config);
        return;
    }

    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
    log::info!("Starting ProxyBot desktop application");

    run_desktop(Arc::new(config), acceptance_workspace);
}

fn isolated_acceptance_config(workspace: PathBuf) -> Result<AppConfig, String> {
    if !workspace.is_absolute() {
        return Err("desktop acceptance workspace must be an absolute path".to_owned());
    }
    let metadata = std::fs::symlink_metadata(&workspace)
        .map_err(|error| format!("could not inspect acceptance workspace: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("desktop acceptance workspace must be a real directory".to_owned());
    }
    if std::fs::read_dir(&workspace)
        .map_err(|error| format!("could not read acceptance workspace: {error}"))?
        .next()
        .is_some()
    {
        return Err("desktop acceptance workspace must be empty".to_owned());
    }

    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("could not reserve a local proxy port: {error}"))?;
    let proxy_port = listener
        .local_addr()
        .map_err(|error| format!("could not inspect reserved proxy port: {error}"))?
        .port();
    drop(listener);

    let config = AppConfig::for_base_dir(workspace);
    let dns_port = config.dns_port;
    Ok(config.with_ports(proxy_port, dns_port))
}

fn run_desktop(config: Arc<AppConfig>, acceptance_workspace: Option<PathBuf>) {
    let db_state = Arc::new(DbState::open(&config.db_path).expect("Failed to initialize database"));
    match db_state.import_legacy_alerts(&config.legacy_alerts_path) {
        Ok(count) if count > 0 => log::info!("Imported {count} Alerts from the retired JSON store"),
        Ok(_) => {}
        Err(error) => log::warn!("Failed to import the retired Alert store: {error}"),
    }
    let cert_manager = Arc::new(
        CertManager::new(config.ca_dir.clone()).expect("Failed to initialize certificate manager"),
    );
    let rules_engine = Arc::new(RulesEngine::with_dir(config.rules_dir.clone()));
    let dns_state = Arc::new(
        DnsState::with_config(config.clone())
            .with_database(db_state.clone())
            .with_rules_engine(rules_engine.clone()),
    );
    let proxy_state = Arc::new(ProxyState::new());
    let pf_runtime_state = Arc::new(crate::pf::PfRuntimeState::new(&config));
    let mitm_runtime_state = Arc::new(crate::proxy::MitmRuntimeState::new());
    let keep_running_state = Arc::new(crate::proxy::KeepRunningState::new());
    let anomaly_detector = Arc::new(AnomalyDetector::with_stores(
        db_state.clone(),
        Arc::new(crate::anomaly::BaselineStore::with_path(
            config.baseline_path.clone(),
        )),
    ));
    let replay_state = Arc::new(ReplayState::default());
    let cert_server_state = Arc::new(crate::cert_server::CertServerState::new());
    let metrics = Arc::new(crate::metrics::counters::ProxyMetrics::new());
    let dashboard_server = Arc::new(crate::dashboard::DashboardServer::new(
        config.dashboard_port,
        metrics.clone(),
    ));
    let frida_manager = Arc::new(
        crate::frida::FridaManager::new()
            .map_err(|error| format!("Failed to initialize Frida: {error}"))
            .expect("Failed to initialize Frida runtime"),
    );
    let frida_state = FridaState(frida_manager);
    let network_conditions_engine = Arc::new(crate::network::NetworkConditionEngine::new());
    let network_conditions_state = NetworkConditionsState(network_conditions_engine);
    let app_state = Arc::new(crate::state::AppState::with_specs_dir(
        config.specs_dir.clone(),
    ));
    let workspace_manager = Arc::new(WorkspaceManager::with_paths(
        config.workspaces_dir.clone(),
        config.base_dir.clone(),
    ));
    let custom_app_rules = Arc::new(
        crate::commands::app_fingerprint::CustomAppRuleStore::from_path(
            config.app_signatures_path.clone(),
        ),
    );

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(db_state)
        .manage(cert_manager)
        .manage(dns_state)
        .manage(proxy_state)
        .manage(pf_runtime_state)
        .manage(mitm_runtime_state)
        .manage(keep_running_state)
        .manage(anomaly_detector)
        .manage(rules_engine.clone())
        .manage(replay_state)
        .manage(cert_server_state)
        .manage(metrics)
        .manage(dashboard_server)
        .manage(frida_state)
        .manage(network_conditions_state)
        .manage(app_state)
        .manage(workspace_manager)
        .manage(custom_app_rules)
        .manage(config)
        .setup(move |app| {
            setup_desktop(app, rules_engine)?;
            if let Some(workspace) = acceptance_workspace.clone() {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
                crate::acceptance::start(app.handle().clone(), workspace);
            }
            Ok(())
        })
        .invoke_handler(desktop_invoke_handler())
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    let shutdown_started = Arc::new(std::sync::atomic::AtomicBool::new(false));
    app.run(move |app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
            if !shutdown_started.swap(true, std::sync::atomic::Ordering::SeqCst) {
                api.prevent_exit();
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    shutdown_desktop_network_resources(&app_handle).await;
                    app_handle.exit(code.unwrap_or_default());
                });
            }
        }
    });
}

async fn shutdown_desktop_network_resources(app: &tauri::AppHandle) {
    let runtime = app
        .state::<Arc<crate::proxy::MitmRuntimeState>>()
        .inner()
        .clone();
    let app_state = app.state::<Arc<crate::state::AppState>>().inner().clone();
    if let Err(error) = crate::proxy::stop_proxy_runtime(runtime, app_state).await {
        log::error!("MITM Runtime shutdown failed: {error}");
    }

    let dns = app.state::<Arc<DnsState>>().inner().clone();
    let pf = app
        .state::<Arc<crate::pf::PfRuntimeState>>()
        .inner()
        .clone();
    let config = app.state::<Arc<AppConfig>>().inner().clone();
    if let Err(error) = crate::proxy::stop_pf_runtime(dns, pf, config).await {
        log::error!("PF/DNS shutdown failed: {error}");
    }

    let dashboard = app
        .state::<Arc<crate::dashboard::DashboardServer>>()
        .inner()
        .clone();
    if let Err(error) = dashboard.stop().await {
        log::error!("Dashboard shutdown failed: {error}");
    }

    let cert_server = app
        .state::<Arc<crate::cert_server::CertServerState>>()
        .inner()
        .clone();
    if let Err(error) = cert_server.stop() {
        log::error!("Certificate server shutdown failed: {error}");
    }
}

fn setup_desktop(
    app: &mut tauri::App,
    rules_engine: Arc<RulesEngine>,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = app.state::<Arc<DbState>>().inner().clone();
    let app_state = app.state::<Arc<crate::state::AppState>>().inner().clone();
    if let Err(error) = crate::commands::tls_rules::reload_tls_rules(&db, &app_state) {
        log::warn!("Failed to load TLS decryption rules at startup: {error}");
    }

    if let Err(error) = rules_engine.start_watcher() {
        log::warn!("Failed to watch Rule Files: {error}");
    }

    configure_tray(app)?;
    configure_close_to_tray(app);
    Ok(())
}

fn configure_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let start_item = MenuItem::with_id(app, "start", "Start Proxy", true, None::<&str>)?;
    let stop_item = MenuItem::with_id(app, "stop", "Stop Proxy", true, None::<&str>)?;
    let stats_item = MenuItem::with_id(app, "stats", "Traffic: 0", false, None::<&str>)?;
    let prefs_item = MenuItem::with_id(app, "prefs", "Preferences...", true, None::<&str>)?;
    let help_item = MenuItem::with_id(app, "help", "Help", true, None::<&str>)?;
    let inspect_item = MenuItem::with_id(app, "inspect", "Open Web Inspector", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &start_item,
            &stop_item,
            &stats_item,
            &prefs_item,
            &help_item,
            &inspect_item,
            &quit_item,
        ],
    )?;

    let Some(icon) = app.default_window_icon().cloned() else {
        log::warn!("Skipping tray icon because no default application icon is configured");
        return Ok(());
    };
    let tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("ProxyBot")
        .build(app)?;

    let app_handle = app.handle().clone();
    tray.on_menu_event(move |app, event| match event.id.as_ref() {
        "start" => start_proxy_from_tray(app, &app_handle),
        "stop" => stop_proxy_from_tray(app),
        "quit" => app_handle.exit(0),
        "inspect" => {
            if let Some(window) = app_handle.get_webview_window("main") {
                window.open_devtools();
            }
        }
        _ => {}
    });

    let app_handle = app.handle().clone();
    tray.on_tray_icon_event(move |_tray, event| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            show_main_window(&app_handle);
        }
    });

    Ok(())
}

fn start_proxy_from_tray(_app: &tauri::AppHandle, app_handle: &tauri::AppHandle) {
    let app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        match crate::proxy::start_proxy_for_app(&app_handle).await {
            Ok(_) => {
                let _ = app_handle
                    .notification()
                    .builder()
                    .title("ProxyBot")
                    .body("Proxy started")
                    .show();
            }
            Err(error) => log::error!("Failed to start proxy: {error}"),
        }
    });
}

fn stop_proxy_from_tray(app: &tauri::AppHandle) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        match crate::proxy::stop_proxy_for_app(&app_handle).await {
            Ok(_) => {
                let _ = app_handle
                    .notification()
                    .builder()
                    .title("ProxyBot")
                    .body("Proxy stopped")
                    .show();
            }
            Err(error) => log::error!("Failed to stop proxy: {error}"),
        }
    });
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn configure_close_to_tray(app: &tauri::App) {
    let Some(window) = app.get_webview_window("main") else {
        log::warn!("Main window is unavailable; close-to-tray is disabled");
        return;
    };
    let window_to_hide = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window_to_hide.hide();
        }
    });
}

#[tauri::command]
async fn start_dashboard(
    dashboard: State<'_, Arc<crate::dashboard::DashboardServer>>,
) -> Result<String, String> {
    let lan_ip = crate::network::get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string());
    dashboard.start().await?;
    Ok(format!(
        "http://{}:{}?token={}",
        lan_ip,
        dashboard.port(),
        dashboard.token()
    ))
}

#[tauri::command]
async fn stop_dashboard(
    dashboard: State<'_, Arc<crate::dashboard::DashboardServer>>,
) -> Result<String, String> {
    dashboard.stop().await?;
    Ok("Dashboard stopped".into())
}

#[tauri::command]
fn is_dashboard_running(dashboard: State<'_, Arc<crate::dashboard::DashboardServer>>) -> bool {
    dashboard.is_running()
}

#[tauri::command]
fn get_dashboard_url(
    dashboard: State<'_, Arc<crate::dashboard::DashboardServer>>,
) -> Result<String, String> {
    let lan_ip = crate::network::get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string());
    Ok(format!(
        "http://{}:{}?token={}",
        lan_ip,
        dashboard.port(),
        dashboard.token()
    ))
}

#[tauri::command]
fn build_topology_graph(
    db_state: State<'_, Arc<DbState>>,
    filter: crate::topology::TopologyFilter,
) -> Result<crate::topology::TopologyGraph, String> {
    crate::topology::builder::build_topology_graph(&db_state, &filter)
}

#[tauri::command]
fn get_topology_node_detail(
    db_state: State<'_, Arc<DbState>>,
    node_id: String,
    filter: crate::topology::TopologyFilter,
) -> Result<crate::topology::NodeDetail, String> {
    crate::topology::builder::get_topology_node_detail(&db_state, &node_id, &filter)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn defaults_to_desktop_mode() {
        let options = LaunchOptions::parse(args(&["proxybot"])).unwrap();
        assert_eq!(
            options,
            LaunchOptions {
                mode: LaunchMode::Desktop,
                reverse_target: None,
            }
        );
    }

    #[test]
    fn parses_both_reverse_target_forms() {
        for arguments in [
            args(&["proxybot", "--reverse-target", "http://127.0.0.1:3000"]),
            args(&["proxybot", "--reverse-target=http://127.0.0.1:3000"]),
        ] {
            let options = LaunchOptions::parse(arguments).unwrap();
            assert_eq!(options.mode, LaunchMode::Desktop);
            assert_eq!(
                options.reverse_target.as_deref(),
                Some("http://127.0.0.1:3000")
            );
        }
    }

    #[test]
    fn rejects_missing_reverse_target() {
        let error = LaunchOptions::parse(args(&["proxybot", "--reverse-target"])).unwrap_err();
        assert!(error.contains("requires a URL"));
    }

    #[test]
    fn desktop_acceptance_requires_and_owns_an_isolated_workspace() {
        let options = LaunchOptions::parse(args(&[
            "proxybot",
            "--desktop-acceptance",
            "/tmp/proxybot-acceptance",
            "--reverse-target=http://127.0.0.1:4000",
        ]))
        .unwrap();
        assert_eq!(
            options.mode,
            LaunchMode::DesktopAcceptance(PathBuf::from("/tmp/proxybot-acceptance"))
        );
        assert_eq!(options.reverse_target, None);

        let error = LaunchOptions::parse(args(&["proxybot", "--desktop-acceptance"])).unwrap_err();
        assert!(error.contains("isolated workspace"));
        let error = LaunchOptions::parse(args(&[
            "proxybot",
            "--desktop-acceptance",
            "--reverse-target=http://127.0.0.1:4000",
        ]))
        .unwrap_err();
        assert!(error.contains("non-empty isolated workspace"));
    }

    #[test]
    fn desktop_acceptance_refuses_a_nonempty_workspace_before_startup() {
        let workspace = tempfile::tempdir().unwrap();
        std::fs::write(workspace.path().join("user-data"), "keep").unwrap();

        let error = isolated_acceptance_config(workspace.path().to_path_buf()).unwrap_err();
        assert_eq!(error, "desktop acceptance workspace must be empty");
        assert_eq!(
            std::fs::read_to_string(workspace.path().join("user-data")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn mcp_mode_is_independent_from_desktop_options() {
        let options = LaunchOptions::parse(args(&[
            "proxybot",
            "--reverse-target",
            "http://127.0.0.1:3000",
            "--mcp-stdio",
        ]))
        .unwrap();
        assert_eq!(options.mode, LaunchMode::McpStdio);
        assert_eq!(options.reverse_target, None);
    }

    #[test]
    fn cli_reverse_target_overrides_the_config_value_without_environment_mutation() {
        let options = LaunchOptions::parse(args(&[
            "proxybot",
            "--reverse-target=http://127.0.0.1:4000",
        ]))
        .unwrap();
        let config = AppConfig::for_base_dir("/tmp/proxybot".into())
            .with_reverse_target(Some("http://127.0.0.1:3000".into()));

        assert_eq!(
            options.apply_to(config).reverse_target.as_deref(),
            Some("http://127.0.0.1:4000")
        );
    }

    #[test]
    fn desktop_command_contract_is_unique_and_deep() {
        let names: std::collections::HashSet<_> = DESKTOP_COMMANDS
            .iter()
            .map(|path| path.rsplit("::").next().unwrap())
            .collect();
        assert_eq!(names.len(), DESKTOP_COMMANDS.len());
        assert!(DESKTOP_COMMANDS.len() >= 130);
        for command in ["get_alerts", "acknowledge_alert", "get_alert_count"] {
            assert!(names.contains(command));
        }
        for duplicate in [
            "get_alerts_cmd",
            "acknowledge_alert_cmd",
            "get_alert_count_state_machine",
        ] {
            assert!(!names.contains(duplicate));
        }
    }
}
