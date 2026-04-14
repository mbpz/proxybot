/// App classification rule for traffic filtering.
pub struct AppRule {
    pub name: &'static str,
    pub icon: &'static str,
    pub domains: &'static [&'static str],
}

/// Predefined app classification rules.
pub static APP_RULES: &[AppRule] = &[
    AppRule {
        name: "WeChat",
        icon: "💬",
        domains: &[
            "weixin.qq.com",
            "wechat.com",
            "qq.com",
            "wechatcdn.com",
            "wxs.qq.com",
            "longurl.cn",
            "wechatpay.com",
            "wx.tenpay.com",
            "weapp.com",
            "wxa.com",
        ],
    },
    AppRule {
        name: "Douyin",
        icon: "🎵",
        domains: &[
            "douyin.com",
            "tiktokv.com",
            "tiktok.com",
            "bytecdn.com",
            "douyinvod.com",
            "byted-static.com",
            "douyinecdn.com",
            "bytedance.com",
        ],
    },
    AppRule {
        name: "Alipay",
        icon: "💳",
        domains: &[
            "alipay.com",
            "alipayusercontent.com",
            "alipay.com.cn",
            "alicdn.com",
            "antgroup.com",
            "mybank.com",
        ],
    },
];

/// Classify a host string against the app rules.
/// Returns Some((app_name, app_icon)) if a match is found, None otherwise.
pub fn classify_host(host: &str) -> Option<(&'static str, &'static str)> {
    for rule in APP_RULES.iter() {
        for &domain in rule.domains {
            if host == domain || host.ends_with(&format!(".{}", domain)) {
                return Some((rule.name, rule.icon));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert_eq!(classify_host("qq.com"), Some(("WeChat", "💬")));
        assert_eq!(classify_host("douyin.com"), Some(("Douyin", "🎵")));
        assert_eq!(classify_host("alipay.com"), Some(("Alipay", "💳")));
    }

    #[test]
    fn test_subdomain_match() {
        assert_eq!(classify_host("weixin.qq.com"), Some(("WeChat", "💬")));
        assert_eq!(classify_host("api.douyin.com"), Some(("Douyin", "🎵")));
        assert_eq!(classify_host("mobile.alipay.com"), Some(("Alipay", "💳")));
    }

    #[test]
    fn test_false_positive_subdomain() {
        // These should NOT match — they are look-alike domains
        assert_eq!(classify_host("qq.com.evil.com"), None);
        assert_eq!(classify_host("weixin.qq.com.evil.com"), None);
        assert_eq!(classify_host("douyin.com.fake.com"), None);
        assert_eq!(classify_host("alipay.com.phishing.com"), None);
    }

    #[test]
    fn test_unknown() {
        assert_eq!(classify_host("baike.baidu.com"), None);
        assert_eq!(classify_host("google.com"), None);
    }
}
