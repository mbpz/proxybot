//! Integration tests for AppConfig.

use proxybot_lib::config::AppConfig;

#[test]
fn test_config_load_returns_defaults() {
    let config = AppConfig::load();
    assert_eq!(config.proxy_port, 8088);
    assert_eq!(config.dns_port, 5300);
    assert_eq!(config.cert_server_port, 19876);
}

#[test]
fn test_config_paths_under_base_dir() {
    let config = AppConfig::load();
    assert!(config.db_path.starts_with(&config.base_dir));
    assert!(config.rules_dir.starts_with(&config.base_dir));
    assert!(config.ca_dir.starts_with(&config.base_dir));
    assert!(config.hosts_path.starts_with(&config.base_dir));
    assert!(config.blocklist_path.starts_with(&config.base_dir));
}

#[test]
fn test_config_dns_defaults() {
    let config = AppConfig::load();
    assert_eq!(config.default_upstream_dns, "8.8.8.8:53");
    assert!(config.default_doh_url.contains("1.1.1.1"));
    assert_eq!(config.max_dns_entries, 10000);
    assert_eq!(config.dns_timeout_secs, 5);
}

#[test]
fn test_config_storage_defaults() {
    let config = AppConfig::load();
    assert_eq!(config.max_stored_requests, 1000);
}

#[test]
fn test_proxy_port_helper() {
    use proxybot_lib::config::proxy_port;
    assert_eq!(proxy_port(), 8088);
}

#[test]
fn test_dns_port_helper() {
    use proxybot_lib::config::dns_port;
    assert_eq!(dns_port(), 5300);
}

#[test]
fn test_rules_dir_helper() {
    use proxybot_lib::config::rules_dir;
    let dir = rules_dir();
    assert!(dir.to_str().unwrap().contains(".proxybot"));
    assert!(dir.to_str().unwrap().contains("rules"));
}
