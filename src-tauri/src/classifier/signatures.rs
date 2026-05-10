use super::app_classifier::AppSignature;

pub fn builtin_signatures() -> Vec<AppSignature> {
    vec![
        AppSignature {
            app_id: "tiktok".to_string(),
            app_name: "TikTok".to_string(),
            sni_patterns: vec![
                "*.tiktokv.com".to_string(),
                "*.tiktok.com".to_string(),
                "*.byteoversea.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec![
                "TLS_AES_128_GCM_SHA256".to_string(),
                "TLS_AES_256_GCM_SHA384".to_string(),
            ],
            elliptic_curves: vec!["x25519".to_string()],
            alpn: vec![],
        },
        AppSignature {
            app_id: "wechat".to_string(),
            app_name: "WeChat".to_string(),
            sni_patterns: vec![
                "*.weixin.qq.com".to_string(),
                "*.wechat.com".to_string(),
                "*.qq.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec!["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string()],
            elliptic_curves: vec![],
            alpn: vec![],
        },
        AppSignature {
            app_id: "douyin".to_string(),
            app_name: "Douyin".to_string(),
            sni_patterns: vec![
                "*.douyin.com".to_string(),
                "*.bytedance.com".to_string(),
                "*.tiktokv.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec!["TLS_AES_128_GCM_SHA256".to_string()],
            elliptic_curves: vec!["x25519".to_string()],
            alpn: vec![],
        },
        AppSignature {
            app_id: "alipay".to_string(),
            app_name: "Alipay".to_string(),
            sni_patterns: vec![
                "*.alipay.com".to_string(),
                "*.alipayusercontent.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec!["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string()],
            elliptic_curves: vec![],
            alpn: vec![],
        },
        AppSignature {
            app_id: "amazon".to_string(),
            app_name: "Amazon".to_string(),
            sni_patterns: vec![
                "*.amazon.com".to_string(),
                "*.amazonaws.com".to_string(),
                "*.amazon.co.uk".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec![
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
            ],
            elliptic_curves: vec![],
            alpn: vec!["h2".to_string()],
        },
        AppSignature {
            app_id: "apple".to_string(),
            app_name: "Apple Services".to_string(),
            sni_patterns: vec![
                "*.apple.com".to_string(),
                "*.icloud.com".to_string(),
                "*.mzstatic.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec!["TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string()],
            elliptic_curves: vec![],
            alpn: vec!["h2".to_string(), "http/1.1".to_string()],
        },
        // AI Services - OpenAI
        AppSignature {
            app_id: "openai".to_string(),
            app_name: "OpenAI".to_string(),
            sni_patterns: vec![
                "api.openai.com".to_string(),
                "openai.com".to_string(),
                "oaistg.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec![],
            elliptic_curves: vec![],
            alpn: vec![],
        },
        // AI Services - Anthropic
        AppSignature {
            app_id: "anthropic".to_string(),
            app_name: "Anthropic".to_string(),
            sni_patterns: vec!["api.anthropic.com".to_string(), "anthropic.com".to_string()],
            fingerprints: vec![],
            cipher_suites: vec![],
            elliptic_curves: vec![],
            alpn: vec![],
        },
        // AI Services - Azure OpenAI
        AppSignature {
            app_id: "azure-openai".to_string(),
            app_name: "Azure-OpenAI".to_string(),
            sni_patterns: vec![
                "openai.azure.com".to_string(),
                "cognitiveservices.azure.com".to_string(),
            ],
            fingerprints: vec![],
            cipher_suites: vec![],
            elliptic_curves: vec![],
            alpn: vec![],
        },
        // AI Services - Google AI
        AppSignature {
            app_id: "google-ai".to_string(),
            app_name: "Google-AI".to_string(),
            sni_patterns: vec!["generativelanguage.googleapis.com".to_string()],
            fingerprints: vec![],
            cipher_suites: vec![],
            elliptic_curves: vec![],
            alpn: vec![],
        },
        // AI Services - Cohere
        AppSignature {
            app_id: "cohere".to_string(),
            app_name: "Cohere".to_string(),
            sni_patterns: vec!["api.cohere.ai".to_string()],
            fingerprints: vec![],
            cipher_suites: vec![],
            elliptic_curves: vec![],
            alpn: vec![],
        },
        // AI Services - Groq
        AppSignature {
            app_id: "groq".to_string(),
            app_name: "Groq".to_string(),
            sni_patterns: vec!["api.groq.com".to_string()],
            fingerprints: vec![],
            cipher_suites: vec![],
            elliptic_curves: vec![],
            alpn: vec![],
        },
    ]
}
