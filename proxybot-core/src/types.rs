//! Shared data types for ProxyBot — no Tauri or GUI dependencies.
//!
//! These types are the stable public API surface of proxybot-core.
//! They are re-exported by the Tauri crate and available to external Rust consumers.

use serde::{Deserialize, Serialize};

// ─── Request / Response ────────────────────────────────────────────────────

/// A captured HTTP request with its response and metadata.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InterceptedRequest {
    pub id: String,
    pub timestamp: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub query_params: Option<String>,
    pub status: Option<u16>,
    pub latency_ms: Option<u64>,
    pub scheme: String,
    pub req_headers: Vec<(String, String)>,
    pub req_body: Option<String>,
    pub resp_headers: Vec<(String, String)>,
    pub resp_body: Option<String>,
    pub resp_size: Option<usize>,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
    pub device_id: Option<i64>,
    pub device_name: Option<String>,
    pub client_ip: Option<String>,
    pub is_websocket: bool,
    pub ws_frames: Option<Vec<WsFrame>>,
    pub grpc_decoded: Option<String>,
    pub graphql_op: Option<String>,
}

/// A single WebSocket frame captured from a connection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WsFrame {
    pub direction: String,
    pub timestamp: String,
    pub payload: String,
    pub size: usize,
}

// ─── Breakpoint ────────────────────────────────────────────────────────────

/// Breakpoint target type for request/response interception.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BreakpointTarget {
    Request,
    Response,
    Both,
}

// ─── Rules ─────────────────────────────────────────────────────────────────

/// Rule action types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE", tag = "type", content = "target")]
pub enum RuleAction {
    Direct,
    Proxy,
    Reject,
    #[serde(rename = "MAPREMOTE")]
    MapRemote(String),
    #[serde(rename = "MAPLOCAL")]
    MapLocal(String),
    #[serde(rename = "BREAKPOINT")]
    Breakpoint(BreakpointTarget),
}

impl std::fmt::Display for RuleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleAction::Direct => write!(f, "DIRECT"),
            RuleAction::Proxy => write!(f, "PROXY"),
            RuleAction::Reject => write!(f, "REJECT"),
            RuleAction::MapRemote(ref target) => write!(f, "MAPREMOTE:{}", target),
            RuleAction::MapLocal(ref target) => write!(f, "MAPLOCAL:{}", target),
            RuleAction::Breakpoint(ref t) => write!(f, "BREAKPOINT:{:?}", t),
        }
    }
}

/// Rule pattern types for matching.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RulePattern {
    Domain,
    DomainSuffix,
    #[serde(rename = "DOMAIN-KEYWORD")]
    DomainKeyword,
    IpCidr,
    Geoip,
    RuleSet,
}

impl std::fmt::Display for RulePattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RulePattern::Domain => write!(f, "DOMAIN"),
            RulePattern::DomainSuffix => write!(f, "DOMAIN-SUFFIX"),
            RulePattern::DomainKeyword => write!(f, "DOMAIN-KEYWORD"),
            RulePattern::IpCidr => write!(f, "IP-CIDR"),
            RulePattern::Geoip => write!(f, "GEOIP"),
            RulePattern::RuleSet => write!(f, "RULE-SET"),
        }
    }
}

/// A single routing rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub pattern: RulePattern,
    pub value: String,
    pub action: RuleAction,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub comment: String,
}

fn default_priority() -> u8 {
    100
}

fn default_enabled() -> bool {
    true
}

/// Raw YAML structure for a single rule file.
#[derive(Debug, Deserialize, Serialize)]
pub struct RuleFile {
    pub rules: Vec<RuleEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RuleEntry {
    pub pattern: String,
    pub value: String,
    pub action: String,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub comment: String,
}

impl RuleEntry {
    /// Convert a raw RuleEntry to a validated Rule.
    pub fn to_rule(&self) -> Option<Rule> {
        let pattern = match self.pattern.to_uppercase().as_str() {
            "DOMAIN" => RulePattern::Domain,
            "DOMAIN-SUFFIX" => RulePattern::DomainSuffix,
            "DOMAIN-KEYWORD" => RulePattern::DomainKeyword,
            "IP-CIDR" => RulePattern::IpCidr,
            "GEOIP" => RulePattern::Geoip,
            "RULE-SET" => RulePattern::RuleSet,
            _ => {
                log::warn!("Unknown rule pattern: {}", self.pattern);
                return None;
            }
        };

        let action = match self.action.to_uppercase().as_str() {
            "DIRECT" => RuleAction::Direct,
            "PROXY" => RuleAction::Proxy,
            "REJECT" => RuleAction::Reject,
            "MAPREMOTE" => RuleAction::MapRemote(self.target.clone().unwrap_or_default()),
            "MAPLOCAL" => RuleAction::MapLocal(self.target.clone().unwrap_or_default()),
            "BREAKPOINT" => {
                let target = match self.target.as_deref() {
                    Some("RESPONSE") => BreakpointTarget::Response,
                    Some("BOTH") => BreakpointTarget::Both,
                    _ => BreakpointTarget::Request,
                };
                RuleAction::Breakpoint(target)
            }
            _ => {
                log::warn!("Unknown rule action: {}", self.action);
                return None;
            }
        };

        Some(Rule {
            pattern,
            value: self.value.clone(),
            action,
            name: self.name.clone(),
            priority: self.priority,
            enabled: self.enabled,
            comment: self.comment.clone(),
        })
    }
}

// ─── DNS ───────────────────────────────────────────────────────────────────

/// DNS upstream protocol type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DnsUpstreamType {
    PlainUdp,
    Doh,
}

/// DNS upstream configuration.
#[derive(Clone, Serialize, Deserialize)]
pub struct DnsUpstream {
    pub upstream_type: DnsUpstreamType,
    pub address: String,
}

impl Default for DnsUpstream {
    fn default() -> Self {
        Self {
            upstream_type: DnsUpstreamType::Doh,
            address: "https://1.1.1.1/dns-query".to_string(),
        }
    }
}

/// A single DNS query entry with app classification and routing action.
#[derive(Clone, Serialize)]
pub struct DnsEntry {
    pub domain: String,
    pub timestamp_ms: u64,
    pub app_name: Option<String>,
    pub app_icon: Option<String>,
    pub action: Option<String>,
    pub resolved_ips: Vec<String>,
}

/// A single hosts file entry (domain -> IP mapping).
#[derive(Clone, Debug)]
pub struct HostsEntry {
    pub domain: String,
    pub ip: String,
}

/// A single blocklist entry (domain pattern).
#[derive(Clone, Debug)]
pub struct BlocklistEntry {
    pub domain: String,
}

// ─── Certificates ──────────────────────────────────────────────────────────

/// CA certificate metadata for display.
#[derive(Serialize, Deserialize, Clone)]
pub struct CaMetadata {
    pub created_at: u64,
    pub serial: String,
}

// ─── App Classification ────────────────────────────────────────────────────

/// App classification rule for traffic filtering.
#[derive(Clone, Serialize, Deserialize)]
pub struct AppRule {
    pub name: String,
    pub icon: String,
    pub domains: Vec<String>,
}

// ─── Breakpoint Channel ────────────────────────────────────────────────────

/// A request paused at a breakpoint, waiting for user decision.
#[derive(Debug)]
pub struct BreakpointRequest {
    pub request: InterceptedRequest,
    pub target: BreakpointTarget,
}

/// User decision for a paused breakpoint.
#[derive(Clone, Debug)]
pub enum BreakpointDecision {
    Proceed,
    Modify(Box<InterceptedRequest>),
    Drop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_entry_parsing() {
        let entry = RuleEntry {
            pattern: "DOMAIN".to_string(),
            value: "example.com".to_string(),
            action: "DIRECT".to_string(),
            target: None,
            name: "test".to_string(),
            priority: 100,
            enabled: true,
            comment: "".to_string(),
        };
        let rule = entry.to_rule().unwrap();
        assert_eq!(rule.pattern, RulePattern::Domain);
        assert_eq!(rule.action, RuleAction::Direct);
    }

    #[test]
    fn test_rule_entry_mapremote() {
        let entry = RuleEntry {
            pattern: "DOMAIN-SUFFIX".to_string(),
            value: "api.example.com".to_string(),
            action: "MAPREMOTE".to_string(),
            target: Some("https://mock.local".to_string()),
            name: "mock".to_string(),
            priority: 50,
            enabled: true,
            comment: "".to_string(),
        };
        let rule = entry.to_rule().unwrap();
        assert_eq!(rule.pattern, RulePattern::DomainSuffix);
        assert_eq!(
            rule.action,
            RuleAction::MapRemote("https://mock.local".to_string())
        );
    }

    #[test]
    fn test_rule_entry_breakpoint() {
        let entry = RuleEntry {
            pattern: "DOMAIN".to_string(),
            value: "debug.example.com".to_string(),
            action: "BREAKPOINT".to_string(),
            target: Some("RESPONSE".to_string()),
            name: "debug".to_string(),
            priority: 1,
            enabled: true,
            comment: "".to_string(),
        };
        let rule = entry.to_rule().unwrap();
        assert_eq!(
            rule.action,
            RuleAction::Breakpoint(BreakpointTarget::Response)
        );
    }

    #[test]
    fn test_rule_action_display() {
        assert_eq!(RuleAction::Direct.to_string(), "DIRECT");
        assert_eq!(RuleAction::Proxy.to_string(), "PROXY");
        assert_eq!(RuleAction::Reject.to_string(), "REJECT");
        assert_eq!(
            RuleAction::MapRemote("https://x.com".to_string()).to_string(),
            "MAPREMOTE:https://x.com"
        );
    }

    #[test]
    fn test_intercepted_request_default() {
        let req = InterceptedRequest::default();
        assert!(req.id.is_empty());
        assert!(req.host.is_empty());
        assert!(req.ws_frames.is_none());
    }

    #[test]
    fn test_dns_upstream_default() {
        let upstream = DnsUpstream::default();
        assert_eq!(upstream.upstream_type, DnsUpstreamType::Doh);
        assert!(upstream.address.starts_with("https://"));
    }
}
