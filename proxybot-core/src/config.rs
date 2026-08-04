//! Process configuration and path ownership for ProxyBot.
//!
//! [`AppConfig`] is an immutable value assembled once by a composition root.
//! The core does not retain a process-global snapshot: desktop, MCP, and tests
//! choose an [`EnvironmentSource`] Adapter and pass the resulting value to the
//! Modules they construct.

use std::collections::HashMap;
use std::path::PathBuf;

use thiserror::Error;

pub const DEFAULT_PROXY_PORT: u16 = 8088;
pub const DEFAULT_DNS_PORT: u16 = 5300;
pub const DEFAULT_CERT_SERVER_PORT: u16 = 19876;
pub const DEFAULT_DASHBOARD_PORT: u16 = 9980;

/// Read-only source used while constructing process configuration.
pub trait EnvironmentSource {
    fn value(&self, name: &str) -> Option<String>;
}

/// Production Adapter backed by the current process environment.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessEnvironment;

impl EnvironmentSource for ProcessEnvironment {
    fn value(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

/// Deterministic environment Adapter for tests and embedders.
impl EnvironmentSource for HashMap<String, String> {
    fn value(&self, name: &str) -> Option<String> {
        self.get(name).cloned()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("{name} must be a valid non-zero port, got {value:?}")]
    InvalidPort { name: &'static str, value: String },
    #[error("PROXYBOT_HOME must not be empty")]
    EmptyBaseDir,
}

/// Canonical immutable process configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    // Network listeners.
    pub proxy_port: u16,
    pub dns_port: u16,
    pub cert_server_port: u16,
    pub dashboard_port: u16,

    // Persistent paths. Every default lives under `base_dir`.
    pub base_dir: PathBuf,
    pub db_path: PathBuf,
    pub rules_dir: PathBuf,
    pub ca_dir: PathBuf,
    pub ca_cert_path: PathBuf,
    pub hosts_path: PathBuf,
    pub blocklist_path: PathBuf,
    pub app_rules_path: PathBuf,
    pub app_signatures_path: PathBuf,
    pub filter_presets_path: PathBuf,
    pub history_path: PathBuf,
    pub replay_targets_path: PathBuf,
    /// Retired JSON Alert store, retained only as a one-time import source.
    pub legacy_alerts_path: PathBuf,
    pub baseline_path: PathBuf,
    pub exports_dir: PathBuf,
    pub deployments_dir: PathBuf,
    pub scaffold_projects_dir: PathBuf,
    pub mock_projects_dir: PathBuf,
    pub specs_dir: PathBuf,
    pub scripts_dir: PathBuf,
    pub bypass_scripts_dir: PathBuf,
    pub workspaces_dir: PathBuf,

    // DNS.
    pub max_dns_entries: usize,
    pub dns_timeout_secs: u64,
    pub default_upstream_dns: String,
    pub default_doh_url: String,

    // Storage and generation limits.
    pub max_stored_requests: usize,
    pub max_tokens: usize,
    pub replay_buffer_size: usize,

    // macOS packet filter.
    pub pf_anchor_file: PathBuf,
    pub pf_anchor_name: String,

    // Optional reverse-proxy target.
    pub reverse_target: Option<String>,
}

impl AppConfig {
    /// Load and validate the production process environment.
    pub fn load() -> Result<Self, ConfigError> {
        Self::from_source(&ProcessEnvironment)
    }

    /// Build configuration through an explicit environment Seam.
    pub fn from_source(source: &impl EnvironmentSource) -> Result<Self, ConfigError> {
        let base_dir = match source.value("PROXYBOT_HOME") {
            Some(value) if value.trim().is_empty() => return Err(ConfigError::EmptyBaseDir),
            Some(value) => PathBuf::from(value),
            None => PathBuf::from(source.value("HOME").unwrap_or_else(|| ".".to_owned()))
                .join(".proxybot"),
        };
        let proxy_port = parse_port(source, "PROXYBOT_PORT", DEFAULT_PROXY_PORT)?;
        let dns_port = parse_port(source, "PROXYBOT_DNS_PORT", DEFAULT_DNS_PORT)?;

        Ok(Self::for_base_dir(base_dir)
            .with_ports(proxy_port, dns_port)
            .with_reverse_target(source.value("PROXYBOT_REVERSE_TARGET")))
    }

    /// Deterministic constructor for tests and non-environment Adapters.
    pub fn for_base_dir(base_dir: PathBuf) -> Self {
        Self {
            proxy_port: DEFAULT_PROXY_PORT,
            dns_port: DEFAULT_DNS_PORT,
            cert_server_port: DEFAULT_CERT_SERVER_PORT,
            dashboard_port: DEFAULT_DASHBOARD_PORT,
            db_path: base_dir.join("proxybot.db"),
            rules_dir: base_dir.join("rules"),
            ca_dir: base_dir.join("ca"),
            ca_cert_path: base_dir.join("ca.crt"),
            hosts_path: base_dir.join("hosts"),
            blocklist_path: base_dir.join("blocklist.txt"),
            app_rules_path: base_dir.join("app_rules.json"),
            app_signatures_path: base_dir.join("app_signatures.json"),
            filter_presets_path: base_dir.join("filter_presets.json"),
            history_path: base_dir.join("history.json"),
            replay_targets_path: base_dir.join("replay_targets.json"),
            legacy_alerts_path: base_dir.join("alerts.json"),
            baseline_path: base_dir.join("baseline.json"),
            exports_dir: base_dir.join("exports"),
            deployments_dir: base_dir.join("deployments"),
            scaffold_projects_dir: base_dir.join("scaffold_projects"),
            mock_projects_dir: base_dir.join("mock_projects"),
            specs_dir: base_dir.join("specs"),
            scripts_dir: base_dir.join("scripts"),
            bypass_scripts_dir: base_dir.join("bypass-scripts"),
            workspaces_dir: base_dir.join("workspaces"),
            base_dir,
            max_dns_entries: 10_000,
            dns_timeout_secs: 5,
            default_upstream_dns: "8.8.8.8:53".to_owned(),
            default_doh_url: "https://1.1.1.1/dns-query".to_owned(),
            max_stored_requests: 1_000,
            max_tokens: 4_096,
            replay_buffer_size: 8_192,
            pf_anchor_file: PathBuf::from("/etc/pf.anchors/proxybot"),
            pf_anchor_name: "com.apple/proxybot".to_owned(),
            reverse_target: None,
        }
    }

    pub fn with_ports(mut self, proxy_port: u16, dns_port: u16) -> Self {
        self.proxy_port = proxy_port;
        self.dns_port = dns_port;
        self
    }

    pub fn with_reverse_target(mut self, reverse_target: Option<String>) -> Self {
        self.reverse_target = reverse_target.filter(|value| !value.trim().is_empty());
        self
    }
}

fn parse_port(
    source: &impl EnvironmentSource,
    name: &'static str,
    default: u16,
) -> Result<u16, ConfigError> {
    let Some(value) = source.value(name) else {
        return Ok(default);
    };
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or(ConfigError::InvalidPort { name, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(values: &[(&str, &str)]) -> HashMap<String, String> {
        values
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn defaults_are_derived_from_home() {
        let config = AppConfig::from_source(&environment(&[("HOME", "/users/test")])).unwrap();
        assert_eq!(config.base_dir, PathBuf::from("/users/test/.proxybot"));
        assert_eq!(config.proxy_port, DEFAULT_PROXY_PORT);
        assert_eq!(config.dns_port, DEFAULT_DNS_PORT);
        assert_eq!(
            config.app_signatures_path,
            config.base_dir.join("app_signatures.json")
        );
    }

    #[test]
    fn proxybot_home_is_the_base_directory() {
        let config = AppConfig::from_source(&environment(&[
            ("HOME", "/ignored"),
            ("PROXYBOT_HOME", "/runtime/proxybot"),
            ("PROXYBOT_PORT", "9080"),
            ("PROXYBOT_DNS_PORT", "5353"),
            ("PROXYBOT_REVERSE_TARGET", "http://127.0.0.1:3000"),
        ]))
        .unwrap();
        assert_eq!(config.base_dir, PathBuf::from("/runtime/proxybot"));
        assert_eq!(
            config.db_path,
            PathBuf::from("/runtime/proxybot/proxybot.db")
        );
        assert_eq!(config.proxy_port, 9080);
        assert_eq!(config.dns_port, 5353);
        assert_eq!(
            config.reverse_target.as_deref(),
            Some("http://127.0.0.1:3000")
        );
    }

    #[test]
    fn invalid_ports_fail_fast() {
        let error =
            AppConfig::from_source(&environment(&[("PROXYBOT_PORT", "not-a-port")])).unwrap_err();
        assert_eq!(
            error,
            ConfigError::InvalidPort {
                name: "PROXYBOT_PORT",
                value: "not-a-port".to_owned(),
            }
        );
    }
}
