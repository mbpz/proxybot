use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::Manager;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

mod app_rules;
mod cert;
mod cert_server;
mod dns;
mod har;
mod history;
mod network;
mod pf;
mod proxy;

use cert::CertManager;
use dns::DnsState;
use proxy::{KeepRunningState, ProxyState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting ProxyBot");

    let cert_manager = Arc::new(
        CertManager::new().expect("Failed to initialize certificate manager"),
    );
    let dns_state = Arc::new(DnsState::new());
    let proxy_state = Arc::new(ProxyState::new());
    let keep_running_state = Arc::new(KeepRunningState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(cert_manager.clone())
        .manage(dns_state.clone())
        .manage(proxy_state.clone())
        .manage(keep_running_state.clone())
        .setup(|app| {
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
            proxy::get_network_info,
            proxy::setup_pf,
            proxy::teardown_pf,
            proxy::get_request_detail,
            proxy::export_har,
            proxy::load_history,
            proxy::save_history,
            proxy::set_keep_running,
            proxy::get_keep_running,
            proxy::hide_window,
            proxy::replay_request,
            dns::get_dns_log,
            cert_server::start_cert_server,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
