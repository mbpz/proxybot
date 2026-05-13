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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(classify_host_with_rules("google.com", &rules), None);
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
}
