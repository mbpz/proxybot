//! Central configuration for proxybot-core.
//!
//! All magic numbers, ports, paths, and timeouts are defined here.
//! Consumers call accessor functions rather than referencing fields directly.
//!
//! Environment variable overrides are supported:
//! - `PROXYBOT_HOME` — override base directory (default: `~/.proxybot`)
//! - `PROXYBOT_PORT` — override proxy port (default: 8088)
//! - `PROXYBOT_DNS_PORT` — override DNS port (default: 5300)

use std::path::PathBuf;
use std::sync::LazyLock;

/// Global singleton config — initialized once on first access.
static CONFIG: LazyLock<AppConfig> = LazyLock::new(AppConfig::load);

/// Centralized application configuration.
#[derive(Debug, Clone)]
pub struct AppConfig {
    // ─── Ports ───────────────────────────────────────────────────────────────
    pub proxy_port: u16,
    pub dns_port: u16,
    pub cert_server_port: u16,

    // ─── Paths (all under ~/.proxybot by default) ───────────────────────────
    pub base_dir: PathBuf,
    pub db_path: PathBuf,
    pub rules_dir: PathBuf,
    pub ca_dir: PathBuf,
    pub hosts_path: PathBuf,
    pub blocklist_path: PathBuf,
    pub app_rules_path: PathBuf,
    pub exports_dir: PathBuf,
    pub deployments_dir: PathBuf,
    pub scaffold_projects_dir: PathBuf,
    pub mock_projects_dir: PathBuf,

    // ─── DNS ────────────────────────────────────────────────────────────────
    pub max_dns_entries: usize,
    pub dns_timeout_secs: u64,
    pub default_upstream_dns: String,
    pub default_doh_url: String,

    // ─── Storage ────────────────────────────────────────────────────────────
    pub max_stored_requests: usize,

    // ─── pf (macOS firewall) ───────────────────────────────────────────────
    pub pf_anchor_file: PathBuf,
    pub pf_anchor_name: String,

    // ─── API inference ──────────────────────────────────────────────────────
    pub max_tokens: usize,

    // ─── Replay buffer ──────────────────────────────────────────────────────
    pub replay_buffer_size: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::load()
    }
}

impl AppConfig {
    /// Load configuration from environment / defaults.
    pub fn load() -> Self {
        let home = std::env::var("PROXYBOT_HOME").unwrap_or_else(|_| {
            std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
        });
        let base = PathBuf::from(&home).join(".proxybot");

        let proxy_port = std::env::var("PROXYBOT_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8088);

        let dns_port = std::env::var("PROXYBOT_DNS_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5300);

        Self {
            // Ports
            proxy_port,
            dns_port,
            cert_server_port: 19876,

            // Paths
            base_dir: base.clone(),
            db_path: base.join("proxybot.db"),
            rules_dir: base.join("rules"),
            ca_dir: base.join("ca"),
            hosts_path: base.join("hosts"),
            blocklist_path: base.join("blocklist.txt"),
            app_rules_path: base.join("app_rules.json"),
            exports_dir: base.join("exports"),
            deployments_dir: base.join("deployments"),
            scaffold_projects_dir: base.join("scaffold_projects"),
            mock_projects_dir: base.join("mock_projects"),

            // DNS
            max_dns_entries: 10000,
            dns_timeout_secs: 5,
            default_upstream_dns: "8.8.8.8:53".to_string(),
            default_doh_url: "https://1.1.1.1/dns-query".to_string(),

            // Storage
            max_stored_requests: 1000,

            // pf
            pf_anchor_file: PathBuf::from("/etc/pf.anchors/proxybot"),
            pf_anchor_name: "com.apple/proxybot".to_string(),

            // API
            max_tokens: 4096,

            // Replay
            replay_buffer_size: 8192,
        }
    }
}

// ─── Accessor functions ─────────────────────────────────────────────────────

/// Returns the configured proxy port.
pub fn proxy_port() -> u16 {
    CONFIG.proxy_port
}

/// Returns the configured DNS port.
pub fn dns_port() -> u16 {
    CONFIG.dns_port
}

/// Returns the certificate server port.
pub fn cert_server_port() -> u16 {
    CONFIG.cert_server_port
}

/// Returns the database path.
pub fn db_path() -> PathBuf {
    CONFIG.db_path.clone()
}

/// Returns the rules directory path.
pub fn rules_dir() -> PathBuf {
    CONFIG.rules_dir.clone()
}

/// Returns the CA directory path.
pub fn ca_dir() -> PathBuf {
    CONFIG.ca_dir.clone()
}

/// Returns the CA certificate export path.
pub fn ca_cert_path() -> PathBuf {
    CONFIG.base_dir.join("ca.crt")
}

/// Returns the hosts file path.
pub fn hosts_path() -> PathBuf {
    CONFIG.hosts_path.clone()
}

/// Returns the blocklist path.
pub fn blocklist_path() -> PathBuf {
    CONFIG.blocklist_path.clone()
}

/// Returns the app_rules JSON path.
pub fn app_rules_path() -> PathBuf {
    CONFIG.app_rules_path.clone()
}

/// Returns the exports directory path.
pub fn exports_dir() -> PathBuf {
    CONFIG.exports_dir.clone()
}

/// Returns the deployments directory path.
pub fn deployments_dir() -> PathBuf {
    CONFIG.deployments_dir.clone()
}

/// Returns the scaffold projects directory path.
pub fn scaffold_projects_dir() -> PathBuf {
    CONFIG.scaffold_projects_dir.clone()
}

/// Returns the mock projects directory path.
pub fn mock_projects_dir() -> PathBuf {
    CONFIG.mock_projects_dir.clone()
}

/// Returns the pf anchor file path.
pub fn pf_anchor_file() -> PathBuf {
    CONFIG.pf_anchor_file.clone()
}

/// Returns the pf anchor name.
pub fn pf_anchor_name() -> String {
    CONFIG.pf_anchor_name.clone()
}

/// Returns the default upstream DNS address.
pub fn default_upstream_dns() -> String {
    CONFIG.default_upstream_dns.clone()
}

/// Returns the default DoH URL.
pub fn default_doh_url() -> String {
    CONFIG.default_doh_url.clone()
}

/// Returns the max DNS entries.
pub fn max_dns_entries() -> usize {
    CONFIG.max_dns_entries
}

/// Returns the DNS timeout in seconds.
pub fn dns_timeout_secs() -> u64 {
    CONFIG.dns_timeout_secs
}

/// Returns the max stored requests.
pub fn max_stored_requests() -> usize {
    CONFIG.max_stored_requests
}

/// Returns the API inference max tokens.
pub fn max_tokens() -> usize {
    CONFIG.max_tokens
}

/// Returns the replay buffer size.
pub fn replay_buffer_size() -> usize {
    CONFIG.replay_buffer_size
}

/// Returns the base config directory.
pub fn base_dir() -> PathBuf {
    CONFIG.base_dir.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::load();
        assert_eq!(config.proxy_port, 8088);
        assert_eq!(config.dns_port, 5300);
        assert_eq!(config.max_dns_entries, 10000);
        assert!(config
            .base_dir
            .to_string_lossy()
            .ends_with(".proxybot"));
    }

    #[test]
    fn test_accessor_functions() {
        assert_eq!(proxy_port(), 8088);
        assert_eq!(dns_port(), 5300);
        assert_eq!(max_dns_entries(), 10000);
        assert_eq!(max_tokens(), 4096);
        assert_eq!(replay_buffer_size(), 8192);
    }
}
