#![cfg_attr(not(dev), windows_subsystem = "windows")]

use proxybot_lib::{
    anomaly::AnomalyDetector, cert::CertManager, db::DbState, dns::DnsState, mcp::transport,
    proxy::ProxyState, replay::ReplayState, rules::RulesEngine, tun::TunState,
    workspace::WorkspaceManager,
};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Emitter;

fn main() {
    // Check for MCP stdio mode (headless CLI usage)
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp-stdio") {
        transport::start_stdio_mode();
        return;
    }

    // Reverse-proxy mode (v1.3 G-4 part 2): every unmatched request
    // gets forwarded to the configured local backend. Set via flag
    // for one-shot testing or PROXYBOT_REVERSE_TARGET for daemon
    // mode. The env var is what `proxybot_core::config::reverse_target`
    // reads; the flag is just a convenience for CLI users.
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--reverse-target" {
            if let Some(url) = args.get(i + 1) {
                // Safety: in practice, this CLI is single-threaded at
                // startup, so the env mutation is benign.
                unsafe {
                    std::env::set_var("PROXYBOT_REVERSE_TARGET", url);
                }
                log::info!("Reverse-proxy mode enabled → {}", url);
                i += 2;
                continue;
            } else {
                eprintln!("--reverse-target requires a URL argument");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting ProxyBot GUI");

    let db_state = Arc::new(DbState::new().expect("Failed to initialize database"));
    let cert_manager =
        Arc::new(CertManager::new().expect("Failed to initialize certificate manager"));
    let rules_engine = Arc::new(RulesEngine::new());
    let dns_state =
        Arc::new(DnsState::with_db(db_state.clone()).with_rules_engine(rules_engine.clone()));
    let proxy_state = Arc::new(ProxyState::new());
    let keep_running_state = Arc::new(proxybot_lib::proxy::KeepRunningState::new());
    let anomaly_detector = Arc::new(AnomalyDetector::new());
    let tun_state = Arc::new(TunState::new());
    let replay_state = Arc::new(ReplayState::default());
    let workspace_manager = Arc::new(WorkspaceManager::new());

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
        .manage(workspace_manager.clone())
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
            tray.on_menu_event(move |_app, event| match event.id.as_ref() {
                "start" => {
                    let _ = app_handle.emit("tray-start-proxy", ());
                }
                "stop" => {
                    let _ = app_handle.emit("tray-stop-proxy", ());
                }
                "quit" => {
                    app_handle.exit(0);
                }
                _ => {}
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
            proxybot_lib::init_workspace,
            proxybot_lib::export_workspace,
            proxybot_lib::import_workspace,
            proxybot_lib::list_workspaces,
            proxybot_lib::switch_workspace,
            proxybot_lib::workspace_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
