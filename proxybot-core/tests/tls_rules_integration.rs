//! Integration test for the per-host TLS decryption policy.
//!
//! Pins the realistic mobile-capture scenario the feature targets:
//! a mix of cert-pinned apps (Bypass), telemetry (Passthrough), and
//! the app's own API (Decrypt), with first-match-wins precedence
//! between a specific Decrypt rule and a broad Bypass wildcard.

use proxybot_core::{TlsAction, TlsRule, TlsRuleSet};

fn mobile_ruleset() -> TlsRuleSet {
    TlsRuleSet::new(vec![
        // The app's own API decrypts even though the broad WeChat
        // wildcard below would otherwise bypass it.
        TlsRule {
            pattern: "api.weixin.qq.com".into(),
            action: TlsAction::Decrypt,
        },
        // Cert-pinned WeChat surfaces — bypass so the app doesn't
        // crash on our leaf cert.
        TlsRule {
            pattern: "*.weixin.qq.com".into(),
            action: TlsAction::Bypass,
        },
        // Bugly crash reporting — drop entirely.
        TlsRule {
            pattern: "*.bugly.qq.com".into(),
            action: TlsAction::Passthrough,
        },
        // Alipay, fully pinned.
        TlsRule {
            pattern: "*.alipay.com".into(),
            action: TlsAction::Bypass,
        },
    ])
}

#[test]
fn app_api_decrypts_despite_broad_bypass() {
    let rs = mobile_ruleset();
    assert_eq!(rs.decide("api.weixin.qq.com"), TlsAction::Decrypt);
    // Other WeChat hosts hit the wildcard Bypass.
    assert_eq!(rs.decide("mp.weixin.qq.com"), TlsAction::Bypass);
    assert_eq!(rs.decide("short.weixin.qq.com"), TlsAction::Bypass);
}

#[test]
fn telemetry_is_passthrough() {
    let rs = mobile_ruleset();
    let a = rs.decide("android.bugly.qq.com");
    assert_eq!(a, TlsAction::Passthrough);
    // Passthrough hosts are NOT logged.
    assert!(!a.should_log());
}

#[test]
fn pinned_apps_bypass_but_still_log() {
    let rs = mobile_ruleset();
    let a = rs.decide("mobilegw.alipay.com");
    assert_eq!(a, TlsAction::Bypass);
    // Bypass still records CONNECT metadata.
    assert!(a.should_log());
    assert!(!a.is_decrypt());
}

#[test]
fn unconfigured_host_decrypts_by_default() {
    let rs = mobile_ruleset();
    let a = rs.decide("api.github.com");
    assert_eq!(a, TlsAction::Decrypt);
    assert!(a.is_decrypt());
    assert!(a.should_log());
}

#[test]
fn empty_ruleset_decrypts_everything() {
    let rs = TlsRuleSet::default();
    assert!(rs.is_empty());
    assert_eq!(rs.decide("anything.example.com"), TlsAction::Decrypt);
}
