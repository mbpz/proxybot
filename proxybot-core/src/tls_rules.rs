//! Per-host TLS decryption policy.
//!
//! Decides, for a given hostname, whether the proxy should MITM
//! (decrypt) the connection, tunnel it through without decrypting
//! but still log the CONNECT metadata, or pass it through entirely
//! untouched and uncaptured.
//!
//! This solves two recurring mobile-capture problems:
//!
//! - **Certificate-pinned apps** (WeChat 8.0+, Alipay, banking apps)
//!   refuse a MITM leaf cert and crash. Mark their hosts `Bypass`.
//! - **Telemetry / analytics SDKs** (Bugly, Sensors, Firebase) flood
//!   the capture with noise. Mark them `Passthrough` to drop them
//!   from the log entirely.
//!
//! Patterns reuse [`crate::fingerprint::glob_match`] so `*.weixin.qq.com`
//! works the same way it does in the app-classification rule sets.
//!
//! # Integration
//!
//! Pure logic — the proxy's HTTPS handler calls [`TlsRuleSet::decide`]
//! with the SNI/CONNECT host before deciding whether to generate a
//! leaf certificate. The Tauri layer persists the rules in SQLite
//! and rebuilds a `TlsRuleSet` whenever the user edits them.

use serde::{Deserialize, Serialize};

use crate::fingerprint::glob_match;

/// What to do with a TLS connection to a given host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TlsAction {
    /// MITM the connection with a per-host leaf certificate and
    /// capture the decrypted request/response. This is the default
    /// when no rule matches.
    Decrypt,
    /// Tunnel the bytes straight through without decrypting, but
    /// still record the CONNECT metadata (host, byte counts). Use
    /// for cert-pinned apps that would otherwise fail the handshake.
    Bypass,
    /// Tunnel straight through AND record nothing. Use for
    /// high-volume hosts the user doesn't want in the capture at all.
    Passthrough,
}

impl TlsAction {
    /// True when the connection should be MITM-decrypted.
    pub fn is_decrypt(self) -> bool {
        matches!(self, TlsAction::Decrypt)
    }

    /// True when the connection's CONNECT metadata should still be
    /// logged (Decrypt and Bypass log; Passthrough does not).
    pub fn should_log(self) -> bool {
        !matches!(self, TlsAction::Passthrough)
    }
}

/// A single host pattern → action mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsRule {
    /// Hostname glob, e.g. `*.weixin.qq.com` or `api.example.com`.
    pub pattern: String,
    pub action: TlsAction,
}

/// An ordered set of TLS rules. The first matching rule wins, so
/// callers should place more-specific patterns before broader ones
/// (e.g. `api.example.com: Decrypt` before `*.example.com: Bypass`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TlsRuleSet {
    rules: Vec<TlsRule>,
}

impl TlsRuleSet {
    /// Build a rule set from an ordered list of rules.
    pub fn new(rules: Vec<TlsRule>) -> Self {
        Self { rules }
    }

    /// Decide the action for `host`. Returns the first matching
    /// rule's action, or [`TlsAction::Decrypt`] when nothing matches
    /// (MITM-by-default preserves today's behaviour for any host the
    /// user hasn't explicitly carved out).
    pub fn decide(&self, host: &str) -> TlsAction {
        // Strip a trailing `:port` if the caller passed an authority
        // rather than a bare host — patterns are written against the
        // hostname only.
        let host = host.rsplit_once(':').map_or(host, |(h, _)| h);
        for rule in &self.rules {
            if glob_match(&rule.pattern, host) {
                return rule.action;
            }
        }
        TlsAction::Decrypt
    }

    /// Borrow the underlying rules (for display / persistence).
    pub fn rules(&self) -> &[TlsRule] {
        &self.rules
    }

    /// True when no rules are configured — lets the proxy skip the
    /// lookup entirely on the hot path.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ruleset() -> TlsRuleSet {
        TlsRuleSet::new(vec![
            // Specific decrypt before the broad bypass.
            TlsRule {
                pattern: "api.example.com".into(),
                action: TlsAction::Decrypt,
            },
            TlsRule {
                pattern: "*.example.com".into(),
                action: TlsAction::Bypass,
            },
            TlsRule {
                pattern: "*.bugly.qq.com".into(),
                action: TlsAction::Passthrough,
            },
        ])
    }

    #[test]
    fn no_rules_defaults_to_decrypt() {
        let rs = TlsRuleSet::default();
        assert_eq!(rs.decide("anything.com"), TlsAction::Decrypt);
        assert!(rs.is_empty());
    }

    #[test]
    fn first_match_wins_specific_before_wildcard() {
        let rs = ruleset();
        // Exact match hits the Decrypt rule even though the wildcard
        // below would also match.
        assert_eq!(rs.decide("api.example.com"), TlsAction::Decrypt);
        // Other subdomains fall to the wildcard Bypass.
        assert_eq!(rs.decide("cdn.example.com"), TlsAction::Bypass);
    }

    #[test]
    fn wildcard_matches_subdomains_only() {
        let rs = ruleset();
        // `*.example.com` matches the apex too (glob_match treats the
        // bare suffix as a match).
        assert_eq!(rs.decide("example.com"), TlsAction::Bypass);
        assert_eq!(rs.decide("a.b.example.com"), TlsAction::Bypass);
    }

    #[test]
    fn passthrough_for_telemetry() {
        let rs = ruleset();
        assert_eq!(rs.decide("android.bugly.qq.com"), TlsAction::Passthrough);
    }

    #[test]
    fn unmatched_host_defaults_to_decrypt() {
        let rs = ruleset();
        assert_eq!(rs.decide("google.com"), TlsAction::Decrypt);
    }

    #[test]
    fn authority_with_port_is_stripped() {
        let rs = ruleset();
        assert_eq!(rs.decide("api.example.com:443"), TlsAction::Decrypt);
        assert_eq!(rs.decide("cdn.example.com:8443"), TlsAction::Bypass);
    }

    #[test]
    fn action_helpers() {
        assert!(TlsAction::Decrypt.is_decrypt());
        assert!(!TlsAction::Bypass.is_decrypt());
        assert!(TlsAction::Decrypt.should_log());
        assert!(TlsAction::Bypass.should_log());
        assert!(!TlsAction::Passthrough.should_log());
    }

    #[test]
    fn ruleset_serde_roundtrip() {
        let rs = ruleset();
        let json = serde_json::to_string(&rs).unwrap();
        let back: TlsRuleSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.decide("api.example.com"), TlsAction::Decrypt);
        assert_eq!(back.rules().len(), 3);
    }
}
