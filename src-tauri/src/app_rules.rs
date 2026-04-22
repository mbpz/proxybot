/// App classification rule for traffic filtering.
#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct AppRule {
    pub name: String,
    pub icon: String,
    pub domains: Vec<String>,
}

/// Predefined app classification rules.
fn get_default_rules() -> Vec<AppRule> {
    vec![
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
            ],
        },
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
            ],
        },
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
            ],
        },
    ]
}

/// Load app rules from JSON file at ~/.proxybot/app_rules.json.
/// Returns the loaded rules merged with defaults (loaded rules override defaults).
pub fn load_app_rules() -> Vec<AppRule> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let rules_path = std::path::PathBuf::from(home)
        .join(".proxybot")
        .join("app_rules.json");

    if rules_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&rules_path) {
            if let Ok(loaded) = serde_json::from_str::<Vec<AppRule>>(&content) {
                log::info!("Loaded {} app rules from {:?}", loaded.len(), rules_path);
                return loaded;
            } else {
                log::warn!("Failed to parse app_rules.json, using defaults");
            }
        }
    }

    get_default_rules()
}

/// Classify a host string against the app rules.
/// Returns Some((app_name, app_icon)) if a match is found, None otherwise.
pub fn classify_host(host: &str) -> Option<(String, String)> {
    let rules = load_app_rules();
    for rule in rules.iter() {
        for domain in &rule.domains {
            if host == domain.as_str() || host.ends_with(&format!(".{}", domain)) {
                return Some((rule.name.clone(), rule.icon.clone()));
            }
        }
    }
    None
}

/// Classify a host string against the app rules, returning just the app name.
#[allow(dead_code)]
pub fn classify_host_name(host: &str) -> Option<String> {
    classify_host(host).map(|(n, _)| n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert_eq!(classify_host("qq.com"), Some(("WeChat".to_string(), "💬".to_string())));
        assert_eq!(classify_host("douyin.com"), Some(("Douyin".to_string(), "🎵".to_string())));
        assert_eq!(classify_host("alipay.com"), Some(("Alipay".to_string(), "💳".to_string())));
    }

    #[test]
    fn test_subdomain_match() {
        assert_eq!(classify_host("weixin.qq.com"), Some(("WeChat".to_string(), "💬".to_string())));
        assert_eq!(classify_host("api.douyin.com"), Some(("Douyin".to_string(), "🎵".to_string())));
        assert_eq!(classify_host("mobile.alipay.com"), Some(("Alipay".to_string(), "💳".to_string())));
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
