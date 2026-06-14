//! App classification — domain-based app identification.
//!
//! Maps hostnames to apps via a rule library. Rules can be extended
//! by placing an `app_rules.json` file in the config directory.
//!
//! # Default rules cover:
//! - Chinese social: WeChat, Douyin, Kuaishou, Weibo, Xiaohongshu, Bilibili, Zhihu
//! - E-commerce: Taobao, JD, Pinduoduo, Meituan, Alipay
//! - Services: Baidu, Didi, NetEase, Tencent Video, iQiyi
//! - AI providers: OpenAI, Anthropic, Azure, Google, Cohere, Groq, DeepSeek, Moonshot, Zhipu, MiniMax
//! - Plus (v0.9.0): TikTok, Instagram, Snapchat, Telegram, Netflix, Spotify,
//!   Amazon, Microsoft, Apple, Twitter/X, Meta, PayPal, Stripe, GitHub, etc.
//!
//! In addition to domain matching this module exposes a TLS-aware
//! [`AppClassifier`] that combines SNI patterns, ClientHello fingerprints,
//! and user-defined custom rules — see [`classify`].

use crate::fingerprint::{
    get_default_signatures, glob_match, AppMatch, AppSignature, CustomAppRule, HelloInfo,
    MatchSource, TlsFingerprint,
};
use crate::types::AppRule;

/// Load app rules — first from `app_rules.json` if present, otherwise defaults.
pub fn load_app_rules() -> Vec<AppRule> {
    // Try JSON file first
    if let Some(rules) = load_app_rules_from_file() {
        log::info!("Loaded {} app rules from file", rules.len());
        return rules;
    }
    get_default_rules()
}

fn load_app_rules_from_file() -> Option<Vec<AppRule>> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(home)
        .join(".proxybot")
        .join("app_rules.json");

    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(rules) = serde_json::from_str::<Vec<AppRule>>(&content) {
                return Some(rules);
            }
            log::warn!("Failed to parse app_rules.json, using defaults");
        }
    }
    None
}

/// Classify a host string against the app rules.
/// Returns Some((app_name, app_icon)) if a match is found, None otherwise.
pub fn classify_host(host: &str) -> Option<(String, String)> {
    let rules = load_app_rules();
    classify_host_with_rules(host, &rules)
}

/// Classify a host using provided rules (avoids reloading rules on each call).
pub fn classify_host_with_rules(host: &str, rules: &[AppRule]) -> Option<(String, String)> {
    for rule in rules {
        for domain in &rule.domains {
            if host_matches_domain(host, domain) {
                return Some((rule.name.clone(), rule.icon.clone()));
            }
        }
    }
    None
}

/// Check if a host matches a domain rule (exact or subdomain).
pub fn host_matches_domain(host: &str, domain: &str) -> bool {
    if host == domain {
        return true;
    }
    // Subdomain match: "api.weixin.qq.com" matches "weixin.qq.com"
    // but "qq.com.evil.com" must NOT match "qq.com"
    if host.ends_with(&format!(".{}", domain)) {
        return true;
    }
    false
}

/// Classify a host, returning just the app name.
pub fn classify_host_name(host: &str) -> Option<String> {
    classify_host(host).map(|(n, _)| n)
}

/// Get the default built-in app classification rules.
pub fn get_default_rules() -> Vec<AppRule> {
    vec![
        // ─── Social & Communication ─────────────────────────────────────
        AppRule {
            name: "WeChat".to_string(),
            icon: "💬".to_string(),
            domains: vec![
                "weixin.qq.com".to_string(),
                "wechat.com".to_string(),
                "qq.com".to_string(),
                "wechatcdn.com".to_string(),
                "wxs.qq.com".to_string(),
                "longurl.cn".to_string(),
                "wechatpay.com".to_string(),
                "wx.tenpay.com".to_string(),
                "weapp.com".to_string(),
                "wxa.com".to_string(),
                "weixinbridge.com".to_string(),
                "mmbiz.qpic.cn".to_string(),
                "mp.weixin.qq.com".to_string(),
                "shmily.qq.com".to_string(),
            ],
        },
        AppRule {
            name: "Weibo".to_string(),
            icon: "📣".to_string(),
            domains: vec![
                "weibo.com".to_string(),
                "weibo.cn".to_string(),
                "weibocdn.com".to_string(),
                "sinaimg.cn".to_string(),
                "sina.com.cn".to_string(),
                "sinajs.cn".to_string(),
            ],
        },
        AppRule {
            name: "QQ".to_string(),
            icon: "🐧".to_string(),
            domains: vec![
                "qzone.qq.com".to_string(),
                "qpic.cn".to_string(),
                "qlogo.cn".to_string(),
                "gtimg.cn".to_string(),
                "idqqimg.com".to_string(),
                "tencent-cloud.com".to_string(),
                "tencent.com".to_string(),
            ],
        },
        // ─── Short Video ────────────────────────────────────────────────
        AppRule {
            name: "Douyin".to_string(),
            icon: "🎵".to_string(),
            domains: vec![
                "douyin.com".to_string(),
                "tiktokv.com".to_string(),
                "tiktok.com".to_string(),
                "bytecdn.com".to_string(),
                "douyinvod.com".to_string(),
                "byted-static.com".to_string(),
                "douyinecdn.com".to_string(),
                "bytedance.com".to_string(),
                "byteimg.com".to_string(),
                "feiliao.com".to_string(),
            ],
        },
        AppRule {
            name: "Kuaishou".to_string(),
            icon: "📹".to_string(),
            domains: vec![
                "kuaishou.com".to_string(),
                "kuaishoupay.com".to_string(),
                "yximgs.com".to_string(),
                "kwimgs.com".to_string(),
            ],
        },
        // ─── E-commerce ─────────────────────────────────────────────────
        AppRule {
            name: "Taobao".to_string(),
            icon: "🛒".to_string(),
            domains: vec![
                "taobao.com".to_string(),
                "tmall.com".to_string(),
                "alicdn.com".to_string(),
                "alibaba.com".to_string(),
                "alibabacloud.com".to_string(),
                "tbcdn.cn".to_string(),
                "tbcache.com".to_string(),
                "taobaocdn.com".to_string(),
            ],
        },
        AppRule {
            name: "JD".to_string(),
            icon: "🐕".to_string(),
            domains: vec![
                "jd.com".to_string(),
                "360buyimg.com".to_string(),
                "jdpay.com".to_string(),
                "jcloud.com".to_string(),
                "jdwl.com".to_string(),
            ],
        },
        AppRule {
            name: "Pinduoduo".to_string(),
            icon: "🔶".to_string(),
            domains: vec![
                "pinduoduo.com".to_string(),
                "yangkeduo.com".to_string(),
                "pddpic.com".to_string(),
            ],
        },
        AppRule {
            name: "Meituan".to_string(),
            icon: "🛵".to_string(),
            domains: vec![
                "meituan.com".to_string(),
                "meituan.net".to_string(),
                "dianping.com".to_string(),
                "mtimg.com".to_string(),
            ],
        },
        // ─── Lifestyle ──────────────────────────────────────────────────
        AppRule {
            name: "Xiaohongshu".to_string(),
            icon: "📕".to_string(),
            domains: vec![
                "xiaohongshu.com".to_string(),
                "xhscdn.com".to_string(),
                "xhslink.com".to_string(),
            ],
        },
        AppRule {
            name: "Didi".to_string(),
            icon: "🚗".to_string(),
            domains: vec![
                "didi.cn".to_string(),
                "didiglobal.com".to_string(),
                "didistatic.com".to_string(),
            ],
        },
        // ─── Content & Video ────────────────────────────────────────────
        AppRule {
            name: "Bilibili".to_string(),
            icon: "📺".to_string(),
            domains: vec![
                "bilibili.com".to_string(),
                "biliapi.net".to_string(),
                "biliapi.com".to_string(),
                "hdslb.com".to_string(),
                "bilivideo.com".to_string(),
            ],
        },
        AppRule {
            name: "iQiyi".to_string(),
            icon: "🎬".to_string(),
            domains: vec![
                "iqiyi.com".to_string(),
                "iqiyipic.com".to_string(),
                "qiyipic.com".to_string(),
            ],
        },
        AppRule {
            name: "TencentVideo".to_string(),
            icon: "▶️".to_string(),
            domains: vec![
                "qqvideo.com".to_string(),
                "video.qq.com".to_string(),
                "smtcdns.net".to_string(),
            ],
        },
        AppRule {
            name: "NetEase".to_string(),
            icon: "🎶".to_string(),
            domains: vec![
                "163.com".to_string(),
                "126.net".to_string(),
                "127.net".to_string(),
                "netease.com".to_string(),
                "music.126.net".to_string(),
            ],
        },
        // ─── Search & Info ──────────────────────────────────────────────
        AppRule {
            name: "Baidu".to_string(),
            icon: "🔍".to_string(),
            domains: vec![
                "baidu.com".to_string(),
                "baidustatic.com".to_string(),
                "bdstatic.com".to_string(),
                "bcebos.com".to_string(),
                "baidubce.com".to_string(),
            ],
        },
        AppRule {
            name: "Zhihu".to_string(),
            icon: "🤔".to_string(),
            domains: vec![
                "zhihu.com".to_string(),
                "zhimg.com".to_string(),
                "zhihuishu.com".to_string(),
            ],
        },
        // ─── Finance ────────────────────────────────────────────────────
        AppRule {
            name: "Alipay".to_string(),
            icon: "💳".to_string(),
            domains: vec![
                "alipay.com".to_string(),
                "alipayusercontent.com".to_string(),
                "alipay.com.cn".to_string(),
                "alicdn.com".to_string(),
                "antgroup.com".to_string(),
                "mybank.com".to_string(),
                "alipaylog.com".to_string(),
            ],
        },
        // ─── AI Providers ───────────────────────────────────────────────
        AppRule {
            name: "OpenAI".to_string(),
            icon: "O".to_string(),
            domains: vec![
                "api.openai.com".to_string(),
                "openai.com".to_string(),
                "oaistg.com".to_string(),
            ],
        },
        AppRule {
            name: "Anthropic".to_string(),
            icon: "A".to_string(),
            domains: vec!["api.anthropic.com".to_string(), "anthropic.com".to_string()],
        },
        AppRule {
            name: "Azure-OpenAI".to_string(),
            icon: "Z".to_string(),
            domains: vec![
                "openai.azure.com".to_string(),
                "cognitiveservices.azure.com".to_string(),
            ],
        },
        AppRule {
            name: "Google-AI".to_string(),
            icon: "G".to_string(),
            domains: vec!["generativelanguage.googleapis.com".to_string()],
        },
        AppRule {
            name: "Cohere".to_string(),
            icon: "C".to_string(),
            domains: vec!["api.cohere.ai".to_string()],
        },
        AppRule {
            name: "Groq".to_string(),
            icon: "Q".to_string(),
            domains: vec!["api.groq.com".to_string()],
        },
        AppRule {
            name: "DeepSeek".to_string(),
            icon: "D".to_string(),
            domains: vec!["api.deepseek.com".to_string(), "deepseek.com".to_string()],
        },
        AppRule {
            name: "Moonshot".to_string(),
            icon: "M".to_string(),
            domains: vec!["api.moonshot.cn".to_string(), "moonshot.cn".to_string()],
        },
        AppRule {
            name: "Zhipu".to_string(),
            icon: "Z".to_string(),
            domains: vec!["open.bigmodel.cn".to_string(), "bigmodel.cn".to_string()],
        },
        AppRule {
            name: "MiniMax".to_string(),
            icon: "M".to_string(),
            domains: vec!["api.minimax.chat".to_string(), "minimax.chat".to_string()],
        },
        // ─── v0.9.0: Short Video & Social (international) ──────────────
        AppRule {
            name: "TikTok".to_string(),
            icon: "🎵".to_string(),
            domains: vec![
                "tiktok.com".to_string(),
                "tiktokv.com".to_string(),
                "byteoversea.com".to_string(),
                "musical.ly".to_string(),
                "snssdk.com".to_string(),
                "amemv.com".to_string(),
            ],
        },
        AppRule {
            name: "Instagram".to_string(),
            icon: "📷".to_string(),
            domains: vec![
                "instagram.com".to_string(),
                "cdninstagram.com".to_string(),
                "ig.me".to_string(),
            ],
        },
        AppRule {
            name: "Snapchat".to_string(),
            icon: "👻".to_string(),
            domains: vec![
                "snapchat.com".to_string(),
                "snapkit.com".to_string(),
                "snap.com".to_string(),
                "bitmoji.com".to_string(),
            ],
        },
        AppRule {
            name: "Telegram".to_string(),
            icon: "✈️".to_string(),
            domains: vec![
                "telegram.org".to_string(),
                "t.me".to_string(),
                "telegra.ph".to_string(),
                "telegram.me".to_string(),
            ],
        },
        AppRule {
            name: "Twitter".to_string(),
            icon: "🐦".to_string(),
            domains: vec![
                "twitter.com".to_string(),
                "x.com".to_string(),
                "twimg.com".to_string(),
                "t.co".to_string(),
                "abs.twimg.com".to_string(),
                "pbs.twimg.com".to_string(),
            ],
        },
        AppRule {
            name: "Facebook".to_string(),
            icon: "📘".to_string(),
            domains: vec![
                "facebook.com".to_string(),
                "fb.com".to_string(),
                "fb.me".to_string(),
                "fbcdn.net".to_string(),
                "fbsbx.com".to_string(),
                "messenger.com".to_string(),
            ],
        },
        AppRule {
            name: "WhatsApp".to_string(),
            icon: "💚".to_string(),
            domains: vec![
                "whatsapp.com".to_string(),
                "whatsapp.net".to_string(),
                "wa.me".to_string(),
            ],
        },
        // ─── v0.9.0: Streaming & Media ──────────────────────────────────
        AppRule {
            name: "Netflix".to_string(),
            icon: "🎬".to_string(),
            domains: vec![
                "netflix.com".to_string(),
                "nflxvideo.net".to_string(),
                "nflxso.net".to_string(),
                "nflximg.net".to_string(),
            ],
        },
        AppRule {
            name: "Spotify".to_string(),
            icon: "🎧".to_string(),
            domains: vec![
                "spotify.com".to_string(),
                "spotifycdn.com".to_string(),
                "scdn.co".to_string(),
                "spoti.fi".to_string(),
            ],
        },
        AppRule {
            name: "YouTube".to_string(),
            icon: "📺".to_string(),
            domains: vec![
                "youtube.com".to_string(),
                "youtu.be".to_string(),
                "ytimg.com".to_string(),
                "googlevideo.com".to_string(),
                "youtube-nocookie.com".to_string(),
                "youtube-ui.l.google.com".to_string(),
            ],
        },
        // ─── v0.9.0: E-commerce (international) ────────────────────────
        AppRule {
            name: "Amazon".to_string(),
            icon: "🛒".to_string(),
            domains: vec![
                "amazon.com".to_string(),
                "amazonaws.com".to_string(),
                "amazon.co.uk".to_string(),
                "amazon.de".to_string(),
                "amazon.co.jp".to_string(),
                "amazonaws.com.cn".to_string(),
                "cloudfront.net".to_string(),
                "media-amazon.com".to_string(),
            ],
        },
        AppRule {
            name: "eBay".to_string(),
            icon: "🏷️".to_string(),
            domains: vec!["ebay.com".to_string(), "ebayimg.com".to_string(), "ebaystatic.com".to_string()],
        },
        // ─── v0.9.0: Tech / Cloud / Dev ────────────────────────────────
        AppRule {
            name: "Apple".to_string(),
            icon: "🍎".to_string(),
            domains: vec![
                "apple.com".to_string(),
                "icloud.com".to_string(),
                "mzstatic.com".to_string(),
                "apple-cloudkit.com".to_string(),
                "apple-mapkit.com".to_string(),
                "itunes.com".to_string(),
                "me.com".to_string(),
            ],
        },
        AppRule {
            name: "Microsoft".to_string(),
            icon: "🪟".to_string(),
            domains: vec![
                "microsoft.com".to_string(),
                "live.com".to_string(),
                "outlook.com".to_string(),
                "office.com".to_string(),
                "office365.com".to_string(),
                "office.net".to_string(),
                "msn.com".to_string(),
                "bing.com".to_string(),
                "azure.com".to_string(),
                "azureedge.net".to_string(),
                "windows.com".to_string(),
                "windowsupdate.com".to_string(),
            ],
        },
        AppRule {
            name: "Google".to_string(),
            icon: "🔎".to_string(),
            domains: vec![
                "google.com".to_string(),
                "googleapis.com".to_string(),
                "gstatic.com".to_string(),
                "googleusercontent.com".to_string(),
                "gmail.com".to_string(),
                "googledrive.com".to_string(),
                "docs.google.com".to_string(),
                "ggpht.com".to_string(),
            ],
        },
        AppRule {
            name: "GitHub".to_string(),
            icon: "🐙".to_string(),
            domains: vec![
                "github.com".to_string(),
                "github.io".to_string(),
                "githubusercontent.com".to_string(),
                "githubassets.com".to_string(),
            ],
        },
        // ─── v0.9.0: Finance / Payments ────────────────────────────────
        AppRule {
            name: "PayPal".to_string(),
            icon: "💵".to_string(),
            domains: vec![
                "paypal.com".to_string(),
                "paypalobjects.com".to_string(),
            ],
        },
        AppRule {
            name: "Stripe".to_string(),
            icon: "💸".to_string(),
            domains: vec!["stripe.com".to_string(), "stripe.network".to_string()],
        },
    ]
}

// ─── AppClassifier (TLS-aware) ─────────────────────────────────────────────

/// Classifier that combines the legacy domain rules with the v0.9.0
/// TLS-fingerprint + custom-rule pipeline.
///
/// Cheap to construct — designed to be cloned into per-connection state
/// or held in a `OnceCell`/read-only shared.
#[derive(Debug, Clone)]
pub struct AppClassifier {
    signatures: Vec<AppSignature>,
    custom_rules: Vec<CustomAppRule>,
    /// Pre-built HashSet of every default fingerprint for O(1) `contains`.
    default_fp_set: std::collections::HashSet<TlsFingerprint>,
}

impl AppClassifier {
    /// Build a classifier from the default signature library and an
    /// optional list of user-defined custom rules.
    pub fn new(custom_rules: Vec<CustomAppRule>) -> Self {
        let signatures = get_default_signatures();
        let default_fp_set = signatures
            .iter()
            .flat_map(|s| s.fingerprints.iter().cloned())
            .collect();
        Self {
            signatures,
            custom_rules,
            default_fp_set,
        }
    }

    pub fn signatures(&self) -> &[AppSignature] {
        &self.signatures
    }

    pub fn custom_rules(&self) -> &[CustomAppRule] {
        &self.custom_rules
    }

    /// Run the priority chain:
    /// 1. Exact TLS fingerprint match → confidence 1.0, source `Fingerprint`
    /// 2. SNI pattern match → confidence 0.9, source `Sni`
    /// 3. User custom rule → confidence from rule, source `Custom`
    ///
    /// Fingerprint is checked first because it's the most specific
    /// signal — an exact fingerprint match is less likely to be a
    /// coincidence than a wild-carded SNI.
    pub fn classify(&self, hello: &HelloInfo) -> Option<AppMatch> {
        // 1. TLS fingerprint (exact) — highest confidence
        let fp = hello.fingerprint();
        if !fp.cipher_suites.is_empty() && self.default_fp_set.contains(&fp) {
            for sig in &self.signatures {
                if sig.fingerprints.contains(&fp) {
                    return Some(AppMatch {
                        app_id: sig.app_id.clone(),
                        app_name: sig.app_name.clone(),
                        confidence: 1.0,
                        source: MatchSource::Fingerprint,
                    });
                }
            }
        }

        // 2. SNI patterns
        if let Some(sni) = hello.sni.as_deref() {
            for sig in &self.signatures {
                for pattern in &sig.sni_patterns {
                    if glob_match(pattern, sni) {
                        return Some(AppMatch {
                            app_id: sig.app_id.clone(),
                            app_name: sig.app_name.clone(),
                            confidence: 0.9,
                            source: MatchSource::Sni,
                        });
                    }
                }
            }
        }

        // 3. Custom user rules
        for rule in &self.custom_rules {
            if rule.matches(hello) {
                return Some(AppMatch {
                    app_id: rule.app_id.clone(),
                    app_name: rule.app_name.clone(),
                    confidence: rule.confidence,
                    source: MatchSource::Custom,
                });
            }
        }

        None
    }
}

impl Default for AppClassifier {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

/// Convenience: classify a hello using the default signature library
/// (no custom rules). Equivalent to `AppClassifier::default().classify(hello)`.
pub fn classify(hello: &HelloInfo) -> Option<AppMatch> {
    AppClassifier::default().classify(hello)
}

/// Backward-compat alias — `AppMatchResult == AppMatch`.
pub type AppMatchResult = AppMatch;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_fingerprint_set;
    use crate::fingerprint::RuleCondition;

    #[test]
    fn test_host_matches_domain_exact() {
        assert!(host_matches_domain("qq.com", "qq.com"));
        assert!(!host_matches_domain("qqq.com", "qq.com"));
    }

    #[test]
    fn test_host_matches_domain_subdomain() {
        assert!(host_matches_domain("api.weixin.qq.com", "weixin.qq.com"));
        assert!(host_matches_domain("web.wechat.com", "wechat.com"));
    }

    #[test]
    fn test_host_matches_domain_false_positive() {
        // Must NOT match look-alike domains
        assert!(!host_matches_domain("qq.com.evil.com", "qq.com"));
        assert!(!host_matches_domain("douyin.com.fake.com", "douyin.com"));
        assert!(!host_matches_domain("alipay.com.phishing.com", "alipay.com"));
    }

    #[test]
    fn test_classify_exact_match() {
        let rules = get_default_rules();
        assert_eq!(
            classify_host_with_rules("qq.com", &rules),
            Some(("WeChat".to_string(), "💬".to_string()))
        );
        assert_eq!(
            classify_host_with_rules("douyin.com", &rules),
            Some(("Douyin".to_string(), "🎵".to_string()))
        );
        assert_eq!(
            classify_host_with_rules("taobao.com", &rules),
            Some(("Taobao".to_string(), "🛒".to_string()))
        );
    }

    #[test]
    fn test_classify_subdomain_match() {
        let rules = get_default_rules();
        assert_eq!(
            classify_host_with_rules("api.weixin.qq.com", &rules),
            Some(("WeChat".to_string(), "💬".to_string()))
        );
        assert_eq!(
            classify_host_with_rules("api.m.jd.com", &rules),
            Some(("JD".to_string(), "🐕".to_string()))
        );
    }

    #[test]
    fn test_classify_new_apps() {
        let rules = get_default_rules();
        // Kuaishou
        assert_eq!(
            classify_host_with_rules("api.kuaishou.com", &rules),
            Some(("Kuaishou".to_string(), "📹".to_string()))
        );
        // Xiaohongshu
        assert_eq!(
            classify_host_with_rules("www.xiaohongshu.com", &rules),
            Some(("Xiaohongshu".to_string(), "📕".to_string()))
        );
        // Bilibili
        assert_eq!(
            classify_host_with_rules("api.bilibili.com", &rules),
            Some(("Bilibili".to_string(), "📺".to_string()))
        );
        // Baidu
        assert_eq!(
            classify_host_with_rules("www.baidu.com", &rules),
            Some(("Baidu".to_string(), "🔍".to_string()))
        );
        // Meituan
        assert_eq!(
            classify_host_with_rules("api.meituan.com", &rules),
            Some(("Meituan".to_string(), "🛵".to_string()))
        );
        // DeepSeek
        assert_eq!(
            classify_host_with_rules("api.deepseek.com", &rules),
            Some(("DeepSeek".to_string(), "D".to_string()))
        );
    }

    #[test]
    fn test_unknown_domain() {
        let rules = get_default_rules();
        assert_eq!(classify_host_with_rules("example.com", &rules), None);
        // google.com is now a known rule — use a genuinely unknown host.
        assert_eq!(
            classify_host_with_rules("this-host-does-not-exist.test", &rules),
            None
        );
    }

    #[test]
    fn test_all_rules_have_domains() {
        let rules = get_default_rules();
        for rule in &rules {
            assert!(
                !rule.domains.is_empty(),
                "App '{}' has no domains",
                rule.name
            );
            assert!(!rule.name.is_empty(), "App has empty name");
            assert!(!rule.icon.is_empty(), "App '{}' has empty icon", rule.name);
        }
    }

    // ─── v0.9.0 default rules + AppClassifier tests ──────────────────

    #[test]
    fn default_rules_include_v090_apps() {
        let rules = get_default_rules();
        let names: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
        for required in [
            "TikTok",
            "Instagram",
            "Snapchat",
            "Telegram",
            "Twitter",
            "Netflix",
            "Spotify",
            "YouTube",
            "Amazon",
            "Apple",
            "Microsoft",
            "Google",
            "GitHub",
            "PayPal",
            "Stripe",
        ] {
            assert!(
                names.contains(&required),
                "expected default rule for {} — got {:?}",
                required,
                names
            );
        }
    }

    #[test]
    fn classify_sni_match_returns_sni_source() {
        let c = AppClassifier::default();
        let hello = HelloInfo {
            sni: Some("api.tiktokv.com".into()),
            ..Default::default()
        };
        let m = c.classify(&hello).expect("expected TikTok match");
        assert_eq!(m.app_id, "tiktok");
        assert_eq!(m.source, MatchSource::Sni);
        assert!((m.confidence - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn classify_fingerprint_match_wins_over_sni() {
        let c = AppClassifier::default();
        // A hello whose fingerprint matches the built-in TikTok entry
        let hello = HelloInfo {
            sni: Some("api.tiktokv.com".into()),
            cipher_suites: vec!["TLS_AES_128_GCM_SHA256".into()],
            extensions: vec![
                "server_name".into(),
                "application_layer_protocol_negotiation".into(),
            ],
            elliptic_curves: vec!["x25519".into(), "secp256r1".into()],
            alpn: vec!["h2".into(), "http/1.1".into()],
            client_version: Some("TLS 1.3".into()),
        };
        let m = c.classify(&hello).expect("expected match");
        assert_eq!(m.source, MatchSource::Fingerprint);
        assert!((m.confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn classify_no_match_returns_none() {
        let c = AppClassifier::default();
        let hello = HelloInfo {
            sni: Some("example.com".into()),
            cipher_suites: vec!["TLS_AES_128_GCM_SHA256".into()],
            ..Default::default()
        };
        assert!(c.classify(&hello).is_none());
    }

    #[test]
    fn classify_custom_rule_runs_last() {
        let rule = CustomAppRule {
            app_id: "internal".into(),
            app_name: "Internal Tool".into(),
            icon: "I".into(),
            conditions: vec![RuleCondition::Sni {
                pattern: "*.internal.corp".into(),
            }],
            confidence: 0.75,
        };
        let c = AppClassifier::new(vec![rule]);
        let hello = HelloInfo {
            sni: Some("api.internal.corp".into()),
            ..Default::default()
        };
        let m = c.classify(&hello).expect("expected custom match");
        assert_eq!(m.source, MatchSource::Custom);
        assert_eq!(m.app_id, "internal");
        assert!((m.confidence - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn classify_top_level_classify_function() {
        // Mirror of classify_sni_match — uses the free function form.
        let hello = HelloInfo {
            sni: Some("api.weixin.qq.com".into()),
            ..Default::default()
        };
        let m = classify(&hello).expect("expected WeChat match");
        assert_eq!(m.app_id, "wechat");
        assert_eq!(m.source, MatchSource::Sni);
    }

    #[test]
    fn default_fingerprint_set_is_non_empty() {
        let set = default_fingerprint_set();
        assert!(
            !set.is_empty(),
            "default fingerprint set should include at least one entry"
        );
    }
}
