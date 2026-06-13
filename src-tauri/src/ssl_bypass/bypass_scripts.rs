//! Built-in Frida bypass scripts.
//!
//! Each script is a JavaScript string that runs in the Frida JS runtime
//! on the target Android device. Scripts hook specific Java/Android APIs
//! to bypass SSL certificate pinning.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BypassScript {
    pub id: String,
    pub name: String,
    pub description: String,
    pub target_framework: Vec<String>,
    pub script_content: String,
    pub is_builtin: bool,
}

const OKHTTP3_SCRIPT: &str = r#"
(function() {
    try {
        var CertificatePinner = Java.use("okhttp3.CertificatePinner");
        CertificatePinner.check.overload("java.lang.String", "java.util.List").implementation = function() {
            console.log("[ProxyBot] OkHttp3 CertificatePinner.check bypassed");
        };
        console.log("[ProxyBot] OkHttp3 bypass installed");
    } catch(e) {
        console.log("[ProxyBot] OkHttp3 bypass failed: " + e);
    }
})();
"#;

const CONSCRYPT_SCRIPT: &str = r#"
(function() {
    try {
        var TrustManagerImpl = Java.use("com.android.org.conscrypt.TrustManagerImpl");
        TrustManagerImpl.verifyChain.implementation = function() {
            console.log("[ProxyBot] Conscrypt verifyChain bypassed");
            return arguments[0];
        };
        console.log("[ProxyBot] Conscrypt bypass installed");
    } catch(e) {
        console.log("[ProxyBot] Conscrypt bypass failed: " + e);
    }
})();
"#;

const WEBVIEW_SCRIPT: &str = r#"
(function() {
    try {
        var WebViewClient = Java.use("android.webkit.WebViewClient");
        WebViewClient.onReceivedSslError.implementation = function(view, handler, error) {
            console.log("[ProxyBot] WebView SSL error bypassed");
            handler.proceed();
        };
        console.log("[ProxyBot] WebView bypass installed");
    } catch(e) {
        console.log("[ProxyBot] WebView bypass failed: " + e);
    }
})();
"#;

const FLUTTER_SCRIPT: &str = r#"
(function() {
    try {
        var SSL_CTX_set_custom_verify = Module.findExportByName("libssl.so", "SSL_CTX_set_custom_verify");
        if (SSL_CTX_set_custom_verify) {
            Interceptor.attach(SSL_CTX_set_custom_verify, {
                onEnter: function(args) {
                    args[2] = new NativeFunction(function() { return 0; }, 'int', []);
                }
            });
            console.log("[ProxyBot] Flutter SSL_CTX_set_custom_verify bypassed");
        }
    } catch(e) {
        console.log("[ProxyBot] Flutter bypass failed: " + e);
    }
})();
"#;

const REACT_NATIVE_SCRIPT: &str = r#"
(function() {
    try {
        var CertificatePinner = Java.use("okhttp3.CertificatePinner");
        CertificatePinner.check.overload("java.lang.String", "java.util.List").implementation = function() {
            console.log("[ProxyBot] RN OkHttp bypassed");
        };
        console.log("[ProxyBot] React Native bypass installed");
    } catch(e) {
        console.log("[ProxyBot] React Native bypass failed: " + e);
    }
})();
"#;

const UNIVERSAL_SCRIPT: &str = r#"
(function() {
    try {
        var X509TrustManager = Java.use("javax.net.ssl.X509TrustManager");
        var TrustManagerImpl = Java.use("com.android.org.conscrypt.TrustManagerImpl");
        TrustManagerImpl.verifyChain.implementation = function() {
            return arguments[0];
        };
        console.log("[ProxyBot] Universal bypass installed");
    } catch(e) {
        console.log("[ProxyBot] Universal bypass failed: " + e);
    }
})();
"#;

/// Return all built-in bypass scripts.
pub fn get_all_builtin_scripts() -> Vec<BypassScript> {
    vec![
        BypassScript {
            id: "okhttp3".to_string(),
            name: "OkHttp3 CertificatePinner".to_string(),
            description: "Bypasses OkHttp3 certificate pinning by hooking CertificatePinner.check".to_string(),
            target_framework: vec!["okhttp3".to_string()],
            script_content: OKHTTP3_SCRIPT.to_string(),
            is_builtin: true,
        },
        BypassScript {
            id: "conscrypt".to_string(),
            name: "Conscrypt TrustManager".to_string(),
            description: "Bypasses Conscrypt/Java TLS certificate verification".to_string(),
            target_framework: vec!["conscrypt".to_string(), "java-tls".to_string()],
            script_content: CONSCRYPT_SCRIPT.to_string(),
            is_builtin: true,
        },
        BypassScript {
            id: "webview".to_string(),
            name: "WebView SSL Error".to_string(),
            description: "Bypasses WebView SSL errors by calling handler.proceed()".to_string(),
            target_framework: vec!["webview".to_string()],
            script_content: WEBVIEW_SCRIPT.to_string(),
            is_builtin: true,
        },
        BypassScript {
            id: "flutter".to_string(),
            name: "Flutter SSL Pinning".to_string(),
            description: "Bypasses Flutter/BoringSSL SSL pinning via native hook".to_string(),
            target_framework: vec!["flutter".to_string()],
            script_content: FLUTTER_SCRIPT.to_string(),
            is_builtin: true,
        },
        BypassScript {
            id: "react_native".to_string(),
            name: "React Native Network".to_string(),
            description: "Bypasses React Native network security via OkHttp3 hook".to_string(),
            target_framework: vec!["react-native".to_string()],
            script_content: REACT_NATIVE_SCRIPT.to_string(),
            is_builtin: true,
        },
        BypassScript {
            id: "universal".to_string(),
            name: "Universal (System-level)".to_string(),
            description: "Universal X509TrustManager bypass for any TLS library".to_string(),
            target_framework: vec!["any".to_string()],
            script_content: UNIVERSAL_SCRIPT.to_string(),
            is_builtin: true,
        },
    ]
}

/// Look up a built-in script by id.
pub fn get_script(id: &str) -> Option<BypassScript> {
    get_all_builtin_scripts().into_iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_builtin_scripts_count() {
        let scripts = get_all_builtin_scripts();
        assert_eq!(scripts.len(), 6);
    }

    #[test]
    fn test_get_script_by_id() {
        let script = get_script("okhttp3").unwrap();
        assert_eq!(script.id, "okhttp3");
        assert_eq!(script.name, "OkHttp3 CertificatePinner");
        assert!(script.is_builtin);
    }

    #[test]
    fn test_get_script_unknown_id() {
        assert!(get_script("unknown").is_none());
    }

    #[test]
    fn test_script_content_not_empty() {
        for script in get_all_builtin_scripts() {
            assert!(!script.script_content.is_empty(), "{} has empty content", script.id);
        }
    }

    #[test]
    fn test_script_content_contains_hook() {
        for script in get_all_builtin_scripts() {
            let content = &script.script_content;
            let has_hook = content.contains("Java.use")
                || content.contains("Interceptor.attach")
                || content.contains("implementation =")
                || content.contains("Module.findExportByName");
            assert!(has_hook, "{} script content has no recognizable hook", script.id);
        }
    }
}
