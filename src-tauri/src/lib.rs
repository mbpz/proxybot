use std::sync::Arc;

mod app_rules;
mod cert;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
