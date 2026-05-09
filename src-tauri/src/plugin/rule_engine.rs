use crate::proxy::InterceptedRequest;

/// Rule pattern types for declarative plugin routing
#[derive(Clone, Debug)]
pub enum RulePattern {
    /// Domain suffix match: *.weixin.qq.com
    DomainSuffix(String),
    /// Domain keyword match: contains "weixin"
    DomainKeyword(String),
    /// Full URL pattern match
    UrlPattern {
        method: Option<String>,
        scheme: Option<String>,
        host: Option<String>,
        path: Option<String>,
    },
    /// Header match
    Header { key: String, value: String },
}

/// Glob matching for header values — * matches any sequence
fn glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(pos) = pattern.find('*') {
        let prefix = &pattern[..pos];
        let suffix = &pattern[pos + 1..];
        return value.starts_with(prefix) && value.ends_with(suffix);
    }
    pattern == value
}

/// Wildcard matching helper - supports * as single segment wildcard
fn wildcard_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let value_parts: Vec<&str> = value.split('/').collect();

    let mut pi = 0;
    let mut vi = 0;

    while pi < pattern_parts.len() && vi < value_parts.len() {
        let p = pattern_parts[pi];
        let v = value_parts[vi];
        if p == "*" {
            pi += 1;
            vi += 1;
        } else if p != v {
            return false;
        } else {
            pi += 1;
            vi += 1;
        }
    }
    pi == pattern_parts.len() && vi == value_parts.len()
}

impl RulePattern {
    /// Match this pattern against an intercepted request
    pub fn matches(&self, request: &InterceptedRequest) -> bool {
        match self {
            RulePattern::DomainSuffix(suffix) => request.host.ends_with(suffix),
            RulePattern::DomainKeyword(keyword) => request.host.contains(keyword),
            RulePattern::UrlPattern { method, scheme, host, path } => {
                method.as_ref().map_or(true, |m| m == "*" || m == &request.method)
                    && scheme.as_ref().map_or(true, |s| s == "*" || s == &request.scheme)
                    && host.as_ref().map_or(true, |h| h == "*" || h == &request.host)
                    && path.as_ref().map_or(true, |p| wildcard_match(p, &request.path))
            }
            RulePattern::Header { key, value } => {
                request.req_headers.iter().any(|(k, v)| k == key && glob_match(value, v))
            }
        }
    }
}

/// A single plugin routing rule
#[derive(Clone, Debug)]
pub struct PluginRule {
    pub id: u64,
    pub name: String,
    pub pattern: RulePattern,
    pub plugin_name: String,
    pub priority: u16,
    pub enabled: bool,
}

/// Rule engine for pattern-matched plugin dispatch
pub struct RuleEngine {
    rules: std::sync::RwLock<Vec<PluginRule>>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self { rules: std::sync::RwLock::new(Vec::new()) }
    }

    /// Match request against rules, return first matching rule (highest priority)
    pub fn match_request(&self, request: &InterceptedRequest) -> Option<PluginRule> {
        let rules = self.rules.read().unwrap();
        rules.iter()
            .filter(|r| r.enabled && r.pattern.matches(request))
            .min_by_key(|r| r.priority)
            .cloned()
    }

    /// Match request against all matching rules sorted by priority
    pub fn match_all(&self, request: &InterceptedRequest) -> Vec<PluginRule> {
        let rules = self.rules.read().unwrap();
        let mut matched: Vec<_> = rules.iter()
            .filter(|r| r.enabled && r.pattern.matches(request))
            .cloned()
            .collect();
        matched.sort_by_key(|r| r.priority);
        matched
    }

    /// Add a rule (insertion-sorted by priority, stable for equal priorities)
    pub fn add_rule(&self, rule: PluginRule) {
        let mut rules = self.rules.write().unwrap();
        let pos = rules.iter()
            .position(|r| r.priority > rule.priority)
            .unwrap_or(rules.len());
        rules.insert(pos, rule);
    }

    /// Remove a rule by id
    pub fn remove_rule(&self, id: u64) -> Option<PluginRule> {
        let mut rules = self.rules.write().unwrap();
        rules.iter().position(|r| r.id == id).map(|pos| rules.remove(pos))
    }

    /// List all rules sorted by priority
    pub fn list_rules(&self) -> Vec<PluginRule> {
        self.rules.read().unwrap().clone()
    }
}

impl Default for RuleEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(host: &str, method: &str, scheme: &str, path: &str, headers: Vec<(String, String)>) -> InterceptedRequest {
        InterceptedRequest {
            host: host.into(),
            method: method.into(),
            scheme: scheme.into(),
            path: path.into(),
            req_headers: headers,
            ..Default::default()
        }
    }

    #[test]
    fn test_domain_suffix_match() {
        let pattern = RulePattern::DomainSuffix("weixin.qq.com".into());
        let request = make_request("api.weixin.qq.com", "GET", "https", "/cgi-bin/token", vec![]);
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_domain_suffix_no_match() {
        let pattern = RulePattern::DomainSuffix("weixin.qq.com".into());
        let request = make_request("api.douyin.com", "GET", "https", "/", vec![]);
        assert!(!pattern.matches(&request));
    }

    #[test]
    fn test_domain_keyword_match() {
        let pattern = RulePattern::DomainKeyword("weixin".into());
        let request = make_request("api.weixin.qq.com", "GET", "https", "/", vec![]);
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_url_pattern_match() {
        let pattern = RulePattern::UrlPattern {
            method: Some("POST".into()),
            scheme: None,
            host: Some("api.example.com".into()),
            path: Some("/upload/*".into()),
        };
        let request = make_request("api.example.com", "POST", "https", "/upload/file.jpg", vec![]);
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_header_match() {
        let pattern = RulePattern::Header {
            key: "Authorization".into(),
            value: "Bearer *".into(),
        };
        let request = make_request(
            "api.example.com",
            "GET",
            "https",
            "/",
            vec![("Authorization".into(), "Bearer token123".into())],
        );
        assert!(pattern.matches(&request));
    }

    #[test]
    fn test_wildcard_match_segment() {
        assert!(wildcard_match("/upload/*", "/upload/file.jpg"));
        assert!(!wildcard_match("/upload/*", "/upload/deep/file.jpg"));
        assert!(wildcard_match("*", "anything"));
        assert!(wildcard_match("/a/*/c", "/a/b/c"));
    }
}
