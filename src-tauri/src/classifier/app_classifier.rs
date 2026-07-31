use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMatch {
    #[serde(rename = "appId")]
    pub app_id: String,
    #[serde(rename = "appName")]
    pub app_name: String,
    pub confidence: f32,
    pub source: MatchSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchSource {
    Sni,
    Fingerprint,
    Custom,
    Dns,
}

pub struct AppClassifier {
    signatures: Vec<AppSignature>,
    custom_rules: Vec<AppRule>,
}

#[derive(Debug, Clone)]
pub struct AppSignature {
    pub app_id: String,
    pub app_name: String,
    pub sni_patterns: Vec<String>,
    pub fingerprints: Vec<TlsFingerprint>,
    pub cipher_suites: Vec<String>,
    pub elliptic_curves: Vec<String>,
    pub alpn: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TlsFingerprint {
    pub client_version: String,
    pub cipher_suites: Vec<String>,
    pub extensions: Vec<String>,
    pub elliptic_curves: Vec<String>,
    pub alpn: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AppRule {
    pub app_id: String,
    pub app_name: String,
    pub conditions: Vec<RuleCondition>,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub enum RuleCondition {
    Sni { pattern: String },
    CipherSuite { value: String },
    Extension { name: String },
    Alpn { value: String },
}

impl Default for AppClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl AppClassifier {
    pub fn new() -> Self {
        Self {
            signatures: crate::classifier::signatures::builtin_signatures(),
            custom_rules: Vec::new(),
        }
    }

    pub fn classify(&self, hello: &ClientHello) -> Option<AppMatch> {
        // 1. Check SNI patterns (fast path)
        if let Some(sni) = &hello.sni {
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

        // 2. Check TLS fingerprint (精确匹配)
        let fp = extract_fingerprint(hello);
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

        // 3. Check custom rules
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

    pub fn add_custom_rule(&mut self, rule: AppRule) {
        self.custom_rules.push(rule);
    }
}

impl AppRule {
    pub fn matches(&self, hello: &ClientHello) -> bool {
        for condition in &self.conditions {
            match condition {
                RuleCondition::Sni { pattern } => {
                    if let Some(sni) = &hello.sni {
                        if !glob_match(pattern, sni) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                RuleCondition::CipherSuite { value } => {
                    // ClientHello vecs are typically small (10-20 entries), linear scan is fine
                    if !hello.cipher_suites.iter().any(|c| c == value) {
                        return false;
                    }
                }
                RuleCondition::Extension { name } => {
                    // ClientHello vecs are typically small (10-20 entries), linear scan is fine
                    if !hello.extensions.iter().any(|e| e == name) {
                        return false;
                    }
                }
                RuleCondition::Alpn { value } => {
                    // ClientHello vecs are typically small (10-20 entries), linear scan is fine
                    if !hello.alpn.iter().any(|a| a == value) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

pub struct ClientHello {
    client_version: String,
    sni: Option<String>,
    cipher_suites: Vec<String>,
    extensions: Vec<String>,
    elliptic_curves: Vec<String>,
    alpn: Vec<String>,
}

fn extract_fingerprint(hello: &ClientHello) -> TlsFingerprint {
    TlsFingerprint {
        client_version: hello.client_version.clone(),
        cipher_suites: hello.cipher_suites.clone(),
        extensions: hello.extensions.clone(),
        elliptic_curves: hello.elliptic_curves.clone(),
        alpn: hello.alpn.clone(),
    }
}

fn glob_match(pattern: &str, value: &str) -> bool {
    let mut re_pattern = String::with_capacity(pattern.len() + 2);
    re_pattern.push('^');
    let mut literal = String::new();
    for ch in pattern.chars() {
        match ch {
            '*' | '?' => {
                if !literal.is_empty() {
                    re_pattern.push_str(&regex::escape(&literal));
                    literal.clear();
                }
                if ch == '*' {
                    re_pattern.push_str(".*");
                } else {
                    re_pattern.push('.');
                }
            }
            _ => literal.push(ch),
        }
    }
    if !literal.is_empty() {
        re_pattern.push_str(&regex::escape(&literal));
    }
    re_pattern.push('$');
    regex::Regex::new(&re_pattern)
        .map(|re| re.is_match(value))
        .unwrap_or(false)
}
