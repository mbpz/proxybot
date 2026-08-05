//! Shared data types for ProxyBot — no Tauri or GUI dependencies.
//!
//! These types are the stable public API surface of proxybot-core.
//! They are re-exported by the Tauri crate and available to external Rust consumers.

use serde::{Deserialize, Serialize};

// ─── Request / Response ────────────────────────────────────────────────────

crate::desktop_contract_type! {
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
        pub upstream_ip: Option<String>,
        pub is_websocket: bool,
        pub ws_frames: Option<Vec<WsFrame>>,
        pub grpc_decoded: Option<String>,
        pub graphql_op: Option<String>,
    }
}

crate::desktop_contract_type! {
    /// A single WebSocket frame captured from a connection.
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct WsFrame {
        pub direction: String,
        pub timestamp: String,
        pub payload: String,
        pub size: usize,
        #[serde(default)]
        pub opcode: u8,
        #[serde(default)]
        pub truncated: bool,
    }
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

impl crate::desktop_contract::WireType for BreakpointTarget {
    fn type_script_type() -> String {
        "BreakpointTarget".to_owned()
    }
}

impl crate::desktop_contract::DesktopContractType for BreakpointTarget {
    const NAME: &'static str = "BreakpointTarget";

    fn type_script_declaration() -> String {
        "export type BreakpointTarget = \"REQUEST\" | \"RESPONSE\" | \"BOTH\";\n".to_owned()
    }
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

impl crate::desktop_contract::WireType for RuleAction {
    fn type_script_type() -> String {
        "RuleAction".to_owned()
    }
}

impl crate::desktop_contract::DesktopContractType for RuleAction {
    const NAME: &'static str = "RuleAction";

    fn type_script_declaration() -> String {
        "export type RuleAction =\n\
         \x20 | { type: \"DIRECT\" }\n\
         \x20 | { type: \"PROXY\" }\n\
         \x20 | { type: \"REJECT\" }\n\
         \x20 | { type: \"MAPREMOTE\"; target: string }\n\
         \x20 | { type: \"MAPLOCAL\"; target: string }\n\
         \x20 | { type: \"BREAKPOINT\"; target: BreakpointTarget };\n"
            .to_owned()
    }
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
pub enum RulePattern {
    #[serde(rename = "DOMAIN")]
    Domain,
    #[serde(rename = "DOMAIN-SUFFIX")]
    DomainSuffix,
    #[serde(rename = "DOMAIN-KEYWORD")]
    DomainKeyword,
    #[serde(rename = "IP-CIDR")]
    IpCidr,
    #[serde(rename = "GEOIP")]
    Geoip,
    #[serde(rename = "RULE-SET")]
    RuleSet,
}

impl crate::desktop_contract::WireType for RulePattern {
    fn type_script_type() -> String {
        "RulePattern".to_owned()
    }
}

impl crate::desktop_contract::DesktopContractType for RulePattern {
    const NAME: &'static str = "RulePattern";

    fn type_script_declaration() -> String {
        "export type RulePattern = \"DOMAIN\" | \"DOMAIN-SUFFIX\" | \"DOMAIN-KEYWORD\" | \"IP-CIDR\" | \"GEOIP\" | \"RULE-SET\";\n".to_owned()
    }
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

crate::desktop_contract_type! {
    /// A single routing rule.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

impl From<&Rule> for RuleEntry {
    fn from(rule: &Rule) -> Self {
        let (action, target) = match &rule.action {
            RuleAction::Direct => ("DIRECT", None),
            RuleAction::Proxy => ("PROXY", None),
            RuleAction::Reject => ("REJECT", None),
            RuleAction::MapRemote(target) => ("MAPREMOTE", Some(target.clone())),
            RuleAction::MapLocal(target) => ("MAPLOCAL", Some(target.clone())),
            RuleAction::Breakpoint(target) => {
                let target = match target {
                    BreakpointTarget::Request => "REQUEST",
                    BreakpointTarget::Response => "RESPONSE",
                    BreakpointTarget::Both => "BOTH",
                };
                ("BREAKPOINT", Some(target.to_owned()))
            }
        };

        Self {
            pattern: rule.pattern.to_string(),
            value: rule.value.clone(),
            action: action.to_owned(),
            target,
            name: rule.name.clone(),
            priority: rule.priority,
            enabled: rule.enabled,
            comment: rule.comment.clone(),
        }
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

impl crate::desktop_contract::WireType for DnsUpstreamType {
    fn type_script_type() -> String {
        "DnsUpstreamType".to_owned()
    }
}

impl crate::desktop_contract::DesktopContractType for DnsUpstreamType {
    const NAME: &'static str = "DnsUpstreamType";

    fn type_script_declaration() -> String {
        "export type DnsUpstreamType = \"plainudp\" | \"doh\";\n".to_owned()
    }
}

crate::desktop_contract_type! {
    /// DNS upstream configuration.
    #[derive(Clone, Serialize, Deserialize)]
    pub struct DnsUpstream {
        pub upstream_type: DnsUpstreamType,
        pub address: String,
    }
}

impl Default for DnsUpstream {
    fn default() -> Self {
        Self {
            upstream_type: DnsUpstreamType::Doh,
            address: "https://1.1.1.1/dns-query".to_string(),
        }
    }
}

crate::desktop_contract_type! {
    /// A DNS query/answer observation available to Application Attribution.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct DnsObservation {
        pub domain: String,
        pub timestamp_ms: u64,
        pub app_name: Option<String>,
        pub app_icon: Option<String>,
        pub action: Option<String>,
        pub resolved_ips: Vec<String>,
        pub client_ip: Option<String>,
    }
}

/// Compatibility name used by the desktop DNS log Interface.
pub type DnsEntry = DnsObservation;

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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    use crate::desktop_contract::DesktopContractType;

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
    fn rule_wire_shape_uses_domain_terms_and_tagged_actions() {
        let rule = Rule {
            pattern: RulePattern::DomainSuffix,
            value: "example.com".to_owned(),
            action: RuleAction::MapRemote("https://mock.local".to_owned()),
            name: "mock".to_owned(),
            priority: 50,
            enabled: true,
            comment: String::new(),
        };

        assert_eq!(
            serde_json::to_value(&rule).unwrap(),
            serde_json::json!({
                "pattern": "DOMAIN-SUFFIX",
                "value": "example.com",
                "action": { "type": "MAPREMOTE", "target": "https://mock.local" },
                "name": "mock",
                "priority": 50,
                "enabled": true,
                "comment": ""
            })
        );
        assert_eq!(
            serde_json::to_value(RuleAction::Direct).unwrap(),
            serde_json::json!({ "type": "DIRECT" })
        );
    }

    #[test]
    fn rule_types_render_the_desktop_contract() {
        assert!(Rule::type_script_declaration().contains("pattern: RulePattern"));
        assert!(RuleAction::type_script_declaration().contains("type: \"BREAKPOINT\""));
        assert!(RulePattern::type_script_declaration().contains("\"IP-CIDR\""));
    }

    #[test]
    fn rule_entry_round_trips_targeted_actions() {
        let rule = Rule {
            pattern: RulePattern::DomainKeyword,
            value: "api".to_owned(),
            action: RuleAction::Breakpoint(BreakpointTarget::Response),
            name: "debug".to_owned(),
            priority: 1,
            enabled: true,
            comment: String::new(),
        };

        let entry = RuleEntry::from(&rule);
        assert_eq!(entry.action, "BREAKPOINT");
        assert_eq!(entry.target.as_deref(), Some("RESPONSE"));
        assert_eq!(entry.to_rule(), Some(rule));
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
