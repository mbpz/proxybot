use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::Manager;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

mod anomaly;
mod app_rules;
mod cert;
mod cert_server;
mod dag;
mod db;
mod deploy;
mod dns;
mod har;
mod history;
mod infer;
mod mockgen;
mod network;
mod normalize;
mod pf;
mod proxy;
mod replay;
mod rules;
mod scaffoldgen;
mod state_machine;
mod tun;
mod vision;

use anomaly::{AnomalyDetector, get_alerts, acknowledge_alert, get_alert_count, get_traffic_baseline, scan_request_anomalies};
use cert::CertManager;
use db::DbState;
use deploy::{generate_deployment_bundle, write_deployment_bundle};
use dns::DnsState;
#[allow(unused_imports)]
use mockgen::{generate_mock_project, write_mock_project, get_mock_endpoints, start_mock_server};
use proxy::ProxyState;
use replay::ReplayState;
use rules::RulesEngine;
use scaffoldgen::{evaluate_scaffold_project, generate_scaffold_project, write_scaffold_project};
use tun::TunState;
use vision::{analyze_screenshot, analyze_screenshot_base64, get_vision_analyses, delete_vision_analysis, fuse_vision_with_api};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting ProxyBot");

    let db_state = Arc::new(DbState::new().expect("Failed to initialize database"));
    let cert_manager = Arc::new(
        CertManager::new().expect("Failed to initialize certificate manager"),
    );
    let rules_engine = Arc::new(RulesEngine::new());
    let dns_state = Arc::new(DnsState::with_db(db_state.clone()).with_rules_engine(rules_engine.clone()));
    let proxy_state = Arc::new(ProxyState::new());
    let keep_running_state = Arc::new(proxy::KeepRunningState::new());
    let anomaly_detector = Arc::new(AnomalyDetector::new());
    let tun_state = Arc::new(TunState::new());
    let replay_state = Arc::new(ReplayState::default());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(db_state.clone())
        .manage(cert_manager.clone())
        .manage(dns_state.clone())
        .manage(proxy_state.clone())
        .manage(keep_running_state.clone())
        .manage(anomaly_detector.clone())
        .manage(tun_state.clone())
        .manage(rules_engine.clone())
        .manage(replay_state.clone())
        .setup(move |app| {
            // Start file watcher in a dedicated thread with its own Tokio runtime
            // (notify's internal thread outlives the app's runtime)
            let rules_engine = rules_engine.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime for file watcher");
                rt.block_on(async move {
                    rules_engine.start_watcher();
                });
            });
            let show_item = MenuItem::with_id(app, "show", "Show ProxyBot", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("ProxyBot")
                .build(app)?;

            let app_handle = app.handle().clone();
            tray.on_menu_event(move |_app, event| {
                match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app_handle.exit(0);
                    }
                    _ => {}
                }
            });

            let app_handle2 = app.handle().clone();
            tray.on_tray_icon_event(move |_tray, event| {
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                    if let Some(window) = app_handle2.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            });

            let _ = tray;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            proxy::start_proxy,
            proxy::stop_proxy,
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
            replay::get_replay_targets,
            replay::get_requests_for_replay,
            replay::get_recorded_responses,
            replay::start_replay,
            normalize::get_normalized_traffic,
            normalize::get_traffic_page,
            dag::build_traffic_dag,
            dag::get_traffic_dag,
            dag::get_device_dag,
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
            write_scaffold_project,
            evaluate_scaffold_project,
            analyze_screenshot,
            analyze_screenshot_base64,
            get_vision_analyses,
            delete_vision_analysis,
            fuse_vision_with_api,
            generate_deployment_bundle,
            write_deployment_bundle,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}