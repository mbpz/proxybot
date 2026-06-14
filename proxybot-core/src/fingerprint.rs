//! TLS ClientHello fingerprint types and the built-in app signature library.
//!
//! Provides the data structures used by [`crate::app_classifier::classify`]
//! to identify apps by TLS handshake features and SNI hostname.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// TLS ClientHello fingerprint — describes the TLS-layer features of a
/// connection. Two clients producing the same fingerprint likely come
/// from the same app/library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TlsFingerprint {
    /// TLS record version (e.g. `"TLS 1.3"`, `"TLS 1.2"`).
    pub client_version: String,
    /// Offered cipher suite names (e.g. `"TLS_AES_128_GCM_SHA256"`).
    pub cipher_suites: Vec<String>,
    /// TLS extension type names (e.g. `"server_name"`).
    pub extensions: Vec<String>,
    /// Supported elliptic curves (e.g. `"x25519"`, `"secp256r1"`).
    pub elliptic_curves: Vec<String>,
    /// ALPN protocol names (e.g. `"h2"`, `"http/1.1"`).
    pub alpn: Vec<String>,
}

impl TlsFingerprint {
    /// Convenience constructor — fields are stored in declaration order.
    pub fn new(
        client_version: String,
        cipher_suites: Vec<String>,
        extensions: Vec<String>,
        elliptic_curves: Vec<String>,
        alpn: Vec<String>,
    ) -> Self {
        Self {
            client_version,
            cipher_suites,
            extensions,
            elliptic_curves,
            alpn,
        }
    }
}

/// Source of a classification match — useful for confidence scoring and
/// downstream UI hints.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MatchSource {
    /// Matched by SNI hostname pattern.
    Sni,
    /// Matched by exact TLS fingerprint equality.
    Fingerprint,
    /// Matched by a user-defined custom rule.
    Custom,
    /// Matched by legacy DNS correlation.
    Dns,
}

/// Result of a successful classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppMatch {
    pub app_id: String,
    pub app_name: String,
    /// 0.0 (uncertain) to 1.0 (exact match).
    pub confidence: f32,
    pub source: MatchSource,
}

/// Simplified TLS ClientHello view used by the classifier.
///
/// We don't own a real ClientHello parser in this crate yet — the proxy
/// layer will fill in these fields from a parsed handshake. Keeping the
/// type narrow lets us unit-test the matching logic without a real TLS
/// stack.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HelloInfo {
    pub sni: Option<String>,
    pub cipher_suites: Vec<String>,
    pub extensions: Vec<String>,
    pub elliptic_curves: Vec<String>,
    pub alpn: Vec<String>,
    pub client_version: Option<String>,
}

impl HelloInfo {
    /// Build a fingerprint from this hello (uses empty defaults for
    /// missing fields so the result is still hashable).
    pub fn fingerprint(&self) -> TlsFingerprint {
        TlsFingerprint {
            client_version: self.client_version.clone().unwrap_or_default(),
            cipher_suites: self.cipher_suites.clone(),
            extensions: self.extensions.clone(),
            elliptic_curves: self.elliptic_curves.clone(),
            alpn: self.alpn.clone(),
        }
    }
}

/// Built-in app signature — combines SNI patterns, TLS fingerprints,
/// and (later) certificate issuer patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSignature {
    pub app_id: String,
    pub app_name: String,
    pub icon: String,
    /// Glob-style host patterns (e.g. `*.tiktokv.com`).
    pub sni_patterns: Vec<String>,
    /// Exact-match TLS fingerprints.
    pub fingerprints: Vec<TlsFingerprint>,
}

/// User-defined custom classification rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomAppRule {
    pub app_id: String,
    pub app_name: String,
    pub icon: String,
    pub conditions: Vec<RuleCondition>,
    pub confidence: f32,
}

/// A single condition inside a custom rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleCondition {
    Sni { pattern: String },
    CipherSuite { value: String },
    Alpn { value: String },
}

impl CustomAppRule {
    /// Test whether a hello satisfies ALL of this rule's conditions.
    pub fn matches(&self, hello: &HelloInfo) -> bool {
        self.conditions.iter().all(|c| c.matches(hello))
    }
}

impl RuleCondition {
    pub fn matches(&self, hello: &HelloInfo) -> bool {
        match self {
            RuleCondition::Sni { pattern } => hello
                .sni
                .as_deref()
                .map(|s| glob_match(pattern, s))
                .unwrap_or(false),
            RuleCondition::CipherSuite { value } => hello.cipher_suites.iter().any(|c| c == value),
            RuleCondition::Alpn { value } => hello.alpn.iter().any(|a| a == value),
        }
    }
}

/// Minimal glob match — supports a single leading `*.` only. Sufficient
/// for SNI patterns like `*.tiktokv.com` and `api.openai.com`.
pub fn glob_match(pattern: &str, s: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("*.") {
        s == suffix || s.ends_with(&format!(".{}", suffix))
    } else {
        s == pattern
    }
}

/// Built-in signature library — covers the apps listed in spec §3.3
/// plus a handful of commonly seen ones. Empty fingerprint vectors
/// mean the app only matches by SNI; the classification still works
/// for that tier.
pub fn get_default_signatures() -> Vec<AppSignature> {
    vec![
        AppSignature {
            app_id: "tiktok".into(),
            app_name: "TikTok".into(),
            icon: "🎵".into(),
            sni_patterns: vec![
                "*.tiktokv.com".into(),
                "*.tiktok.com".into(),
                "*.byteoversea.com".into(),
                "*.bytedance.com".into(),
                "*.bytecdn.com".into(),
            ],
            fingerprints: vec![TlsFingerprint::new(
                "TLS 1.3".into(),
                vec!["TLS_AES_128_GCM_SHA256".into()],
                vec!["server_name".into(), "application_layer_protocol_negotiation".into()],
                vec!["x25519".into(), "secp256r1".into()],
                vec!["h2".into(), "http/1.1".into()],
            )],
        },
        AppSignature {
            app_id: "wechat".into(),
            app_name: "WeChat".into(),
            icon: "💬".into(),
            sni_patterns: vec![
                "*.weixin.qq.com".into(),
                "*.wechat.com".into(),
                "*.qq.com".into(),
                "*.wechatcdn.com".into(),
                "*.wxs.qq.com".into(),
            ],
            fingerprints: vec![TlsFingerprint::new(
                "TLS 1.2".into(),
                vec!["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".into()],
                vec!["server_name".into()],
                vec!["x25519".into()],
                vec!["h2".into()],
            )],
        },
        AppSignature {
            app_id: "douyin".into(),
            app_name: "Douyin".into(),
            icon: "🎵".into(),
            sni_patterns: vec![
                "*.douyin.com".into(),
                "*.bytedance.com".into(),
                "*.tiktokv.com".into(),
                "*.douyinvod.com".into(),
            ],
            fingerprints: vec![TlsFingerprint::new(
                "TLS 1.3".into(),
                vec!["TLS_AES_128_GCM_SHA256".into()],
                vec!["server_name".into()],
                vec!["x25519".into()],
                vec!["h2".into()],
            )],
        },
        AppSignature {
            app_id: "alipay".into(),
            app_name: "Alipay".into(),
            icon: "💳".into(),
            sni_patterns: vec![
                "*.alipay.com".into(),
                "*.alipayusercontent.com".into(),
                "*.antgroup.com".into(),
            ],
            fingerprints: vec![TlsFingerprint::new(
                "TLS 1.2".into(),
                vec!["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".into()],
                vec!["server_name".into()],
                vec!["secp256r1".into()],
                vec!["h2".into(), "http/1.1".into()],
            )],
        },
        AppSignature {
            app_id: "amazon".into(),
            app_name: "Amazon".into(),
            icon: "🛒".into(),
            sni_patterns: vec![
                "*.amazon.com".into(),
                "*.amazonaws.com".into(),
                "*.amazon.co.uk".into(),
                "*.amazon.de".into(),
                "*.amazon.jp".into(),
            ],
            fingerprints: vec![TlsFingerprint::new(
                "TLS 1.3".into(),
                vec![
                    "TLS_AES_256_GCM_SHA384".into(),
                    "TLS_CHACHA20_POLY1305_SHA256".into(),
                ],
                vec!["server_name".into()],
                vec!["x25519".into()],
                vec!["h2".into()],
            )],
        },
        AppSignature {
            app_id: "apple".into(),
            app_name: "Apple Services".into(),
            icon: "🍎".into(),
            sni_patterns: vec![
                "*.apple.com".into(),
                "*.icloud.com".into(),
                "*.mzstatic.com".into(),
                "*.apple-cloudkit.com".into(),
            ],
            fingerprints: vec![TlsFingerprint::new(
                "TLS 1.2".into(),
                vec!["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".into()],
                vec!["server_name".into()],
                vec!["secp256r1".into()],
                vec!["h2".into(), "http/1.1".into()],
            )],
        },
    ]
}

/// All distinct fingerprints across the default signature library —
/// useful for O(1) `contains` lookups.
pub fn default_fingerprint_set() -> HashSet<TlsFingerprint> {
    get_default_signatures()
        .into_iter()
        .flat_map(|s| s.fingerprints.into_iter())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_fingerprint_eq_and_hash() {
        let a = TlsFingerprint::new(
            "TLS 1.3".into(),
            vec!["TLS_AES_128_GCM_SHA256".into()],
            vec!["server_name".into()],
            vec!["x25519".into()],
            vec!["h2".into()],
        );
        let b = a.clone();
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn glob_match_handles_wildcard_prefix() {
        assert!(glob_match("*.tiktokv.com", "api.tiktokv.com"));
        assert!(glob_match("*.tiktokv.com", "tiktokv.com"));
        assert!(!glob_match("*.tiktokv.com", "tiktokv.com.evil.com"));
        assert!(glob_match("api.openai.com", "api.openai.com"));
        assert!(!glob_match("api.openai.com", "openai.com"));
    }

    #[test]
    fn hello_info_builds_fingerprint() {
        let hello = HelloInfo {
            sni: Some("api.example.com".into()),
            cipher_suites: vec!["TLS_AES_128_GCM_SHA256".into()],
            extensions: vec!["server_name".into()],
            elliptic_curves: vec!["x25519".into()],
            alpn: vec!["h2".into()],
            client_version: Some("TLS 1.3".into()),
        };
        let fp = hello.fingerprint();
        assert_eq!(fp.client_version, "TLS 1.3");
        assert_eq!(fp.cipher_suites, vec!["TLS_AES_128_GCM_SHA256".to_string()]);
    }

    #[test]
    fn default_signatures_contains_expected_apps() {
        let sigs = get_default_signatures();
        let ids: Vec<&str> = sigs.iter().map(|s| s.app_id.as_str()).collect();
        assert!(ids.contains(&"tiktok"));
        assert!(ids.contains(&"wechat"));
        assert!(ids.contains(&"douyin"));
        assert!(ids.contains(&"alipay"));
        assert!(ids.contains(&"amazon"));
        assert!(ids.contains(&"apple"));
    }

    #[test]
    fn custom_rule_matches_all_conditions() {
        let rule = CustomAppRule {
            app_id: "myapp".into(),
            app_name: "My App".into(),
            icon: "M".into(),
            conditions: vec![
                RuleCondition::Sni { pattern: "*.mycompany.com".into() },
                RuleCondition::CipherSuite { value: "TLS_AES_128_GCM_SHA256".into() },
            ],
            confidence: 0.8,
        };
        let hello = HelloInfo {
            sni: Some("api.mycompany.com".into()),
            cipher_suites: vec!["TLS_AES_128_GCM_SHA256".into()],
            ..Default::default()
        };
        assert!(rule.matches(&hello));

        let partial = HelloInfo {
            sni: Some("api.mycompany.com".into()),
            cipher_suites: vec!["TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256".into()],
            ..Default::default()
        };
        assert!(!rule.matches(&partial));
    }
}
