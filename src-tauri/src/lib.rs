use std::sync::Arc;

mod app_rules;
mod cert;
mod db;
mod dns;
mod har;
mod network;
mod pf;
mod proxy;
mod rules;
mod tun;

use cert::CertManager;
use db::DbState;
use dns::DnsState;
use proxy::ProxyState;
use rules::RulesEngine;
use tun::TunState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("Starting ProxyBot");

    let db_state = Arc::new(DbState::new().expect("Failed to initialize database"));
    let cert_manager = Arc::new(
        CertManager::new().expect("Failed to initialize certificate manager"),
    );
    let rules_engine = Arc::new(RulesEngine::new());
    rules_engine.clone().start_watcher();
    let dns_state = Arc::new(DnsState::with_db(db_state.clone()).with_rules_engine(rules_engine.clone()));
    let proxy_state = Arc::new(ProxyState::new());
    let tun_state = Arc::new(TunState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(db_state.clone())
        .manage(cert_manager.clone())
        .manage(dns_state.clone())
        .manage(proxy_state.clone())
        .manage(tun_state.clone())
        .manage(rules_engine.clone())
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
            dns::get_dns_upstream,
            dns::set_dns_upstream,
            dns::reload_dns_lists,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
