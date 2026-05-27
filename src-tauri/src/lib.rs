use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, State};
use tauri_plugin_notification::NotificationExt;

pub mod adb;
pub mod ai;
pub mod anomaly;
pub mod app_rules;
pub mod cdp;
pub mod cert;
pub mod cert_server;
pub mod classifier;
pub mod commands;
pub mod config;
pub mod dag;
pub mod dashboard;
pub mod db;
pub mod deploy;
pub mod dns;
pub mod error;
pub mod filter;
pub mod graphql;
pub mod har;
pub mod history;
pub mod infer;
pub mod metrics;
pub mod ai_pipeline;
pub mod mcp;
pub mod mockgen;
pub mod network;
pub mod normalize;
pub mod pf;
pub mod plugin;
pub mod protobuf;
pub mod proxy;
pub mod replay;
pub mod rules;
pub mod scaffoldgen;
pub mod scripting;
pub mod state_machine;
pub mod transport;
pub mod tun;
pub mod vision;
pub mod vpn;
pub mod workspace;
pub use classifier::*;
pub use commands::client_setup::*;
pub use commands::compose::*;
pub use commands::filter::*;
pub use commands::graph::*;
pub use commands::replay::*;
pub use commands::ws_frames::*;
// gui module removed - Yew frontend is compiled separately via wasm-pack
// See src/gui/ directory for Yew code (compiled to pkg/ via wasm-pack build)

use anomaly::{
    acknowledge_alert, get_alert_count, get_alerts, get_traffic_baseline, scan_request_anomalies,
    AnomalyDetector,
};
use cert::CertManager;
use db::DbState;
use deploy::{generate_deployment_bundle, write_deployment_bundle};
use dns::DnsState;
#[allow(unused_imports)]
use mockgen::{generate_mock_project, get_mock_endpoints, start_mock_server, write_mock_project};
use proxy::ProxyState;
use replay::ReplayState;
use rules::RulesEngine;
use scaffoldgen::{
    evaluate_scaffold_project, generate_scaffold_project, generate_scaffold_with_vision,
    write_scaffold_project, write_scaffold_project_with_vision,
};
use tun::TunState;
use vision::{
    analyze_screenshot, analyze_screenshot_base64, delete_vision_analysis, fuse_vision_with_api,
    get_vision_analyses,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting ProxyBot");

    let db_state = Arc::new(DbState::new().expect("Failed to initialize database"));
    let cert_manager =
        Arc::new(CertManager::new().expect("Failed to initialize certificate manager"));
    let rules_engine = Arc::new(RulesEngine::new());
    let dns_state =
        Arc::new(DnsState::with_db(db_state.clone()).with_rules_engine(rules_engine.clone()));
    let proxy_state = Arc::new(ProxyState::new());
    let keep_running_state = Arc::new(proxy::KeepRunningState::new());
    let anomaly_detector = Arc::new(AnomalyDetector::new());
    let tun_state = Arc::new(TunState::new());
    let replay_state = Arc::new(ReplayState::default());
    let dashboard_server = Arc::new(dashboard::DashboardServer::new(9980));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(db_state.clone())
        .manage(cert_manager.clone())
        .manage(dns_state.clone())
        .manage(proxy_state.clone())
        .manage(keep_running_state.clone())
        .manage(anomaly_detector.clone())
        .manage(tun_state.clone())
        .manage(rules_engine.clone())
        .manage(replay_state.clone())
        .manage(dashboard_server.clone())
        .setup(move |app| {
            // Start file watcher in a dedicated thread with its own Tokio runtime
            // (notify's internal thread outlives the app's runtime)
            let rules_engine = rules_engine.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create Tokio runtime for file watcher");
                rt.block_on(async move {
                    rules_engine.start_watcher();
                });
            });
            let start_item = MenuItem::with_id(app, "start", "Start Proxy", true, None::<&str>)?;
            let stop_item = MenuItem::with_id(app, "stop", "Stop Proxy", true, None::<&str>)?;
            let stats_item = MenuItem::with_id(app, "stats", "Traffic: 0", false, None::<&str>)?;
            let prefs_item = MenuItem::with_id(app, "prefs", "Preferences...", true, None::<&str>)?;
            let help_item = MenuItem::with_id(app, "help", "Help", true, None::<&str>)?;
            let inspect_item =
                MenuItem::with_id(app, "inspect", "Open Web Inspector", true, None::<&str>)?;
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

            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("ProxyBot")
                .build(app)?;

            let app_handle = app.handle().clone();
            tray.on_menu_event(move |app, event| match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "start" => {
                    match proxy::start_proxy(
                        app_handle.clone(),
                        app.state::<Arc<CertManager>>().clone(),
                        app.state::<Arc<DnsState>>().clone(),
                        app.state::<Arc<DbState>>().clone(),
                        app.state::<Arc<RulesEngine>>().clone(),
                    ) {
                        Ok(_) => {
                            let _ = app
                                .notification()
                                .builder()
                                .title("ProxyBot")
                                .body("Proxy started")
                                .show();
                        }
                        Err(e) => {
                            log::error!("Failed to start proxy: {}", e);
                        }
                    }
                }
                "stop" => match proxy::stop_proxy() {
                    Ok(_) => {
                        let _ = app
                            .notification()
                            .builder()
                            .title("ProxyBot")
                            .body("Proxy stopped")
                            .show();
                    }
                    Err(e) => {
                        log::error!("Failed to stop proxy: {}", e);
                    }
                },
                "quit" => {
                    app_handle.exit(0);
                }
                "inspect" => {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.eval("if (window.devtools) window.devtools.open();");
                    }
                }
                _ => {}
            });

            let app_handle2 = app.handle().clone();
            tray.on_tray_icon_event(move |_tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    if let Some(window) = app_handle2.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                // Note: Double-click handling removed - MouseButtonState::DoubleClick not available
            });

            let _ = tray;

            // Close-to-tray window behavior
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_clone.hide();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            proxy::start_proxy,
            proxy::stop_proxy,
            proxy::get_proxy_status,
            proxy::export_cert,
            proxy::get_ca_cert_path,
            proxy::get_ca_cert_pem,
            proxy::regenerate_ca,
            proxy::get_ca_metadata,
            proxy::get_network_info,
            proxy::setup_pf,
            proxy::teardown_pf,
            proxy::is_pf_enabled,
            proxy::get_request_detail,
            proxy::load_history,
            proxy::save_history,
            proxy::set_keep_running,
            proxy::get_keep_running,
            proxy::hide_window,
            proxy::replay_request,
            dns::get_dns_log,
            dns::get_dns_upstream,
            dns::set_dns_upstream,
            dns::reload_dns_lists,
            cert_server::start_cert_server,
            get_traffic_baseline,
            scan_request_anomalies,
            get_alerts,
            acknowledge_alert,
            get_alert_count,
            db::get_db_stats,
            db::get_devices,
            db::register_device,
            db::update_device_last_seen,
            db::update_device_stats,
            db::set_device_rule_override,
            db::get_device_by_mac,
            tun::setup_tun,
            tun::teardown_tun,
            tun::is_tun_enabled,
            rules::get_rules,
            rules::save_rule,
            rules::delete_rule,
            rules::reorder_rules,
            rules::list_rule_files,
            rules::match_host,
            har::export_har,
            har::save_har_file,
            detect_clients,
            get_proxy_config_command,
            compose_request,
            replay::get_replay_targets,
            replay::get_requests_for_replay,
            replay::get_recorded_responses,
            replay::start_replay,
            normalize::get_normalized_traffic,
            normalize::get_traffic_page,
            dag::build_traffic_dag,
            dag::get_traffic_dag,
            dag::get_device_dag,
            get_graph_data,
            infer::infer_api_semantics,
            infer::store_inference_result,
            infer::get_inferred_apis,
            infer::get_openapi_spec,
            infer::generate_openapi_yaml,
            infer::evaluate_inference,
            infer::get_evaluation_result,
            state_machine::get_auth_state_machine,
            state_machine::get_alerts_cmd,
            state_machine::acknowledge_alert_cmd,
            mockgen::generate_mock_project,
            mockgen::write_mock_project,
            mockgen::get_mock_endpoints,
            mockgen::start_mock_server,
            generate_scaffold_project,
            generate_scaffold_with_vision,
            write_scaffold_project,
            write_scaffold_project_with_vision,
            evaluate_scaffold_project,
            analyze_screenshot,
            analyze_screenshot_base64,
            get_vision_analyses,
            delete_vision_analysis,
            fuse_vision_with_api,
            generate_deployment_bundle,
            write_deployment_bundle,
            commands::ai_stats::get_ai_stats,
            commands::ai_stats::get_ai_context_windows,
            start_dashboard,
            stop_dashboard,
            is_dashboard_running,
            get_dashboard_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn start_dashboard(dashboard: State<'_, Arc<dashboard::DashboardServer>>) -> Result<String, String> {
    let lan_ip = crate::network::get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string());
    dashboard.start().await?;
    Ok(format!("http://{}:{}?token={}", lan_ip, dashboard.port(), dashboard.token()))
}

#[tauri::command]
fn stop_dashboard(dashboard: State<'_, Arc<dashboard::DashboardServer>>) -> Result<String, String> {
    dashboard.stop();
    Ok("Dashboard stopped".into())
}

#[tauri::command]
fn is_dashboard_running(dashboard: State<'_, Arc<dashboard::DashboardServer>>) -> bool {
    dashboard.is_running()
}

#[tauri::command]
fn get_dashboard_url(dashboard: State<'_, Arc<dashboard::DashboardServer>>) -> Result<String, String> {
    let lan_ip = crate::network::get_local_ip().unwrap_or_else(|| "0.0.0.0".to_string());
    Ok(format!("http://{}:{}?token={}", lan_ip, dashboard.port(), dashboard.token()))
}
