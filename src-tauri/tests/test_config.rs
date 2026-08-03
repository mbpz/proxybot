//! Integration tests for the canonical process configuration.

use proxybot_core::{AppConfig, RuntimeConfig};

fn config() -> AppConfig {
    AppConfig::for_base_dir("/tmp/proxybot-config-test".into())
}

#[test]
fn config_defaults_are_stable() {
    let config = config();
    assert_eq!(config.proxy_port, 8088);
    assert_eq!(config.dns_port, 5300);
    assert_eq!(config.cert_server_port, 19876);
    assert_eq!(config.default_upstream_dns, "8.8.8.8:53");
    assert_eq!(config.max_dns_entries, 10_000);
    assert_eq!(config.max_stored_requests, 1_000);
}

#[test]
fn persistent_paths_share_one_base_directory() {
    let config = config();
    for path in [
        &config.db_path,
        &config.rules_dir,
        &config.ca_dir,
        &config.hosts_path,
        &config.blocklist_path,
        &config.app_rules_path,
        &config.app_signatures_path,
        &config.workspaces_dir,
    ] {
        assert!(path.starts_with(&config.base_dir));
    }
}

#[test]
fn mitm_runtime_config_is_derived_from_process_config() {
    let config = config()
        .with_ports(9080, 5353)
        .with_reverse_target(Some("http://127.0.0.1:3000".into()));
    let runtime = RuntimeConfig::from(&config);
    assert_eq!(runtime.bind_addr.port(), 9080);
    assert_eq!(
        runtime.reverse_target.as_deref(),
        Some("http://127.0.0.1:3000")
    );
}
