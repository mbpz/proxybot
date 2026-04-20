use std::sync::Arc;

mod app_rules;
mod cert;
mod dag;
mod db;
mod dns;
mod har;
mod infer;
mod network;
mod normalize;
mod pf;
mod proxy;
mod replay;
mod rules;
mod tun;

use cert::CertManager;
use db::DbState;
use dns::DnsState;
use proxy::ProxyState;
use replay::ReplayState;
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
    let replay_state = Arc::new(ReplayState::default());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(db_state.clone())
        .manage(cert_manager.clone())
        .manage(dns_state.clone())
        .manage(proxy_state.clone())
        .manage(tun_state.clone())
        .manage(rules_engine.clone())
        .manage(replay_state.clone())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
