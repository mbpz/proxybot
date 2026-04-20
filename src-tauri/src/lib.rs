use std::sync::Arc;

mod app_rules;
mod cert;
mod db;
mod dns;
mod network;
mod pf;
mod proxy;

use cert::CertManager;
use db::DbState;
use dns::DnsState;
use proxy::ProxyState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting ProxyBot");

    let db_state = Arc::new(DbState::new().expect("Failed to initialize database"));
    let cert_manager = Arc::new(
        CertManager::new().expect("Failed to initialize certificate manager"),
    );
    let dns_state = Arc::new(DnsState::new());
    let proxy_state = Arc::new(ProxyState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(db_state.clone())
        .manage(cert_manager.clone())
        .manage(dns_state.clone())
        .manage(proxy_state.clone())
        .invoke_handler(tauri::generate_handler![
            proxy::start_proxy,
            proxy::get_ca_cert_path,
            proxy::get_ca_cert_pem,
            proxy::regenerate_ca,
            proxy::get_ca_metadata,
            proxy::get_network_info,
            proxy::setup_pf,
            proxy::teardown_pf,
            proxy::is_pf_enabled,
            dns::get_dns_log,
            db::get_db_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
