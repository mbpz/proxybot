#![cfg_attr(not(dev), windows_subsystem = "windows")]

use proxybot_lib::{
    anomaly::AnomalyDetector,
    cert::CertManager,
    db::DbState,
    deploy::{generate_deployment_bundle, write_deployment_bundle},
    dns::DnsState,
    mockgen::{generate_mock_project, write_mock_project, get_mock_endpoints, start_mock_server},
    proxy::ProxyState,
    replay::ReplayState,
    rules::RulesEngine,
    scaffoldgen::{evaluate_scaffold_project, generate_scaffold_project, generate_scaffold_with_vision, write_scaffold_project, write_scaffold_project_with_vision},
    tun::TunState,
    vision::{analyze_screenshot, analyze_screenshot_base64, get_vision_analyses, delete_vision_analysis, fuse_vision_with_api},
};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, Separator};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting ProxyBot GUI");

    let db_state = Arc::new(DbState::new().expect("Failed to initialize database"));
    let cert_manager = Arc::new(CertManager::new().expect("Failed to initialize certificate manager"));
    let rules_engine = Arc::new(RulesEngine::new());
    let dns_state = Arc::new(DnsState::with_db(db_state.clone()).with_rules_engine(rules_engine.clone()));
    let proxy_state = Arc::new(ProxyState::new());
    let keep_running_state = Arc::new(proxybot_lib::proxy::KeepRunningState::new());
    let anomaly_detector = Arc::new(AnomalyDetector::new());
    let tun_state = Arc::new(TunState::new());
    let replay_state = Arc::new(ReplayState::default());

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
        .setup(|app| {
            let start_item = MenuItem::with_id(app, "start", "Start Proxy", true, None::<&str>)?;
            let stop_item = MenuItem::with_id(app, "stop", "Stop Proxy", true, None::<&str>)?;
            let stats_item = MenuItem::with_id(app, "stats", "Traffic: 0", false, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&start_item, &stop_item, &stats_item, &quit_item])?;

            let tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("ProxyBot GUI")
                .build(app)?;

            let app_handle = app.handle().clone();
            tray.on_menu_event(move |_app, event| {
                match event.id.as_ref() {
                    "start" => { let _ = app_handle.emit("tray-start-proxy", ()); }
                    "stop" => { let _ = app_handle.emit("tray-stop-proxy", ()); }
                    "quit" => { app_handle.exit(0); }
                    _ => {}
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            proxybot_lib::proxy::start_proxy,
            proxybot_lib::proxy::stop_proxy,
            proxybot_lib::proxy::get_proxy_status,
            proxybot_lib::proxy::get_ca_cert_path,
            proxybot_lib::proxy::get_ca_cert_pem,
            proxybot_lib::proxy::regenerate_ca,
            proxybot_lib::db::get_devices,
            proxybot_lib::db::set_device_rule_override,
            proxybot_lib::rules::get_rules,
            proxybot_lib::rules::save_rule,
            proxybot_lib::rules::delete_rule,
            proxybot_lib::db::get_db_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}