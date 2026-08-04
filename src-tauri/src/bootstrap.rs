//! Process and desktop application bootstrap.
//!
//! This is the single composition root for ProxyBot. Platform binaries stay
//! deliberately thin: they select a launch mode, while this Module owns state
//! construction, plugins, tray behavior, and the desktop IPC contract.

use std::ffi::OsString;
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
use crate::tun::TunState;
use crate::workspace::WorkspaceManager;
use proxybot_core::AppConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
enum LaunchMode {
    Desktop,
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
    crate::proxy::get_ca_cert_path,
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
    crate::cert_server::start_cert_server,
    crate::commands::device_setup::generate_device_qr,
    crate::anomaly::get_traffic_baseline,
    crate::anomaly::scan_request_anomalies,
    crate::anomaly::get_alerts,
    crate::anomaly::acknowledge_alert,
    crate::anomaly::get_alert_count,
    crate::db::get_db_stats,
    crate::db::get_devices,
    crate::db::register_device,
    crate::db::update_device_last_seen,
    crate::db::update_device_stats,
    crate::db::set_device_rule_override,
    crate::db::get_device_by_mac,
    crate::tun::setup_tun,
    crate::tun::teardown_tun,
    crate::tun::is_tun_enabled,
    crate::rules::get_rules,
    crate::rules::save_rule,
    crate::rules::delete_rule,
    crate::rules::reorder_rules,
    crate::rules::list_rule_files,
    crate::rules::match_host,
    crate::har::export_har,
    crate::har::save_har_file,
    crate::commands::client_setup::detect_clients,
    crate::commands::client_setup::get_proxy_config_command,
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
    crate::state_machine::get_alerts_cmd,
    crate::state_machine::acknowledge_alert_cmd,
    crate::state_machine::get_alert_count_state_machine,
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

    let config = AppConfig::load().unwrap_or_else(|error| {
        eprintln!("proxybot: invalid configuration: {error}");
        std::process::exit(2);
    });
    let config = options.apply_to(config);

    if options.mode == LaunchMode::McpStdio {
        crate::mcp::transport::start_stdio_mode(config);
        return;
    }

    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
    log::info!("Starting ProxyBot desktop application");

    run_desktop(Arc::new(config));
}

fn run_desktop(config: Arc<AppConfig>) {
    let db_state = Arc::new(DbState::open(&config.db_path).expect("Failed to initialize database"));
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
    let mitm_runtime_state = Arc::new(crate::proxy::MitmRuntimeState::new());
    let keep_running_state = Arc::new(crate::proxy::KeepRunningState::new());
    let anomaly_detector = Arc::new(AnomalyDetector::with_stores(
        Arc::new(crate::anomaly::AlertStore::with_path(
            config.alerts_path.clone(),
        )),
        Arc::new(crate::anomaly::BaselineStore::with_path(
            config.baseline_path.clone(),
        )),
    ));
    let tun_state = Arc::new(TunState::new());
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(db_state)
        .manage(cert_manager)
        .manage(dns_state)
        .manage(proxy_state)
        .manage(mitm_runtime_state)
        .manage(keep_running_state)
        .manage(anomaly_detector)
        .manage(tun_state)
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
        .setup(move |app| setup_desktop(app, rules_engine))
        .invoke_handler(desktop_invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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

fn start_proxy_from_tray(app: &tauri::AppHandle, app_handle: &tauri::AppHandle) {
    let app_handle = app_handle.clone();
    let runtime = app
        .state::<Arc<crate::proxy::MitmRuntimeState>>()
        .inner()
        .clone();
    let certs = app.state::<Arc<CertManager>>().inner().clone();
    let dns = app.state::<Arc<DnsState>>().inner().clone();
    let db = app.state::<Arc<DbState>>().inner().clone();
    let rules = app.state::<Arc<RulesEngine>>().inner().clone();
    let app_state = app.state::<Arc<crate::state::AppState>>().inner().clone();
    let network = app.state::<NetworkConditionsState>().inner().0.clone();
    let config = app.state::<Arc<AppConfig>>().inner().clone();
    let metrics = app
        .state::<Arc<crate::metrics::counters::ProxyMetrics>>()
        .inner()
        .clone();
    tauri::async_runtime::spawn(async move {
        match crate::proxy::start_proxy_runtime(
            app_handle.clone(),
            runtime,
            certs,
            dns,
            db,
            rules,
            app_state,
            network,
            config,
            metrics,
        )
        .await
        {
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
    let runtime = app
        .state::<Arc<crate::proxy::MitmRuntimeState>>()
        .inner()
        .clone();
    let app_state = app.state::<Arc<crate::state::AppState>>().inner().clone();
    tauri::async_runtime::spawn(async move {
        match crate::proxy::stop_proxy_runtime(runtime, app_state).await {
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
fn stop_dashboard(
    dashboard: State<'_, Arc<crate::dashboard::DashboardServer>>,
) -> Result<String, String> {
    dashboard.stop();
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
        assert!(DESKTOP_COMMANDS.len() >= 140);
    }
}
