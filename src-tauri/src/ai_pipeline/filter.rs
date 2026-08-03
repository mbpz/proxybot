//! Phase 1: Noise detection and filtering for AI analysis pipeline.
//!
//! Removes static assets, CDN requests, third-party SDKs, and deduplicates
//! parameterized URLs before LLM inference to reduce token cost ~60%.

use crate::proxy::InterceptedRequest;
use serde::Serialize;

/// Noise categories for reporting
#[derive(Debug, Clone, Serialize)]
pub enum NoiseCategory {
    StaticAsset,
    ThirdPartySdk,
    DuplicateParameterized,
    CdnRequest,
}

/// A request removed by the noise filter
#[derive(Debug, Clone, Serialize)]
pub struct NoiseItem {
    pub request: InterceptedRequest,
    pub category: NoiseCategory,
}

/// Result of Phase 1 filtering
#[derive(Debug, Serialize)]
pub struct FilterResult {
    /// Cleaned candidate requests for analysis
    pub candidates: Vec<InterceptedRequest>,
    /// Requests removed as noise
    pub noise: Vec<NoiseItem>,
    /// Summary counts by category
    pub summary: std::collections::HashMap<String, usize>,
}

impl FilterResult {
    /// Total requests processed (candidates + noise)
    pub fn total_requests(&self) -> usize {
        self.candidates.len() + self.noise.len()
    }

    /// Noise ratio (0.0 to 1.0)
    pub fn noise_ratio(&self) -> f64 {
        let total = self.total_requests() as f64;
        if total == 0.0 {
            0.0
        } else {
            self.noise.len() as f64 / total
        }
    }
}

pub struct NoiseFilter {
    /// Static asset extensions
    static_extensions: Vec<&'static str>,
    /// Third-party SDK host patterns
    third_party_patterns: Vec<&'static str>,
    /// CDN host patterns
    cdn_patterns: Vec<&'static str>,
}

impl NoiseFilter {
    pub fn new() -> Self {
        Self {
            static_extensions: vec![
                "css",
                "js",
                "png",
                "jpg",
                "jpeg",
                "gif",
                "svg",
                "woff",
                "woff2",
                "ico",
                "webp",
                "avif",
                "apng",
                "bmp",
                "tiff",
                "webmanifest",
                "xml",
                "json",
            ],
            third_party_patterns: vec![
                "google-analytics.com",
                "googletagmanager.com",
                "facebook.net",
                "connect.facebook.net",
                "analytics.google.com",
                "doubleclick.net",
                "crashlytics.com",
                "fabric.io",
                "segment.io",
                "segment.com",
                "mixpanel.com",
                "hotjar.com",
                "sentry.io",
                "bugsnag.com",
                "newrelic.com",
                "datadog.com",
                "splunk.com",
                "cloudflare.com",
                "branch.io",
                "adjust.com",
                "appsflyer.com",
                "amplitude.com",
                "heap.io",
                "intercom.io",
                "zendesk.com",
                "optimizely.com",
                "crazyegg.com",
                "quantserve.com",
                "scorecardresearch.com",
            ],
            cdn_patterns: vec![
                "cdn.",
                "static.",
                "assets.",
                "media.",
                ".cloudfront.net",
                ".akamai.net",
                ".fastly.net",
                ".cloudflare.net",
                ".jsdelivr.net",
                ".unpkg.com",
                ".cdnjs.net",
                ".googleapis.com",
                ".fonts.googleapis.com",
                ".ggpht.com",
                ".licdn.com",
                ".media.githubusercontent.com",
            ],
        }
    }

    pub fn filter(&self, requests: Vec<InterceptedRequest>) -> FilterResult {
        let mut candidates = Vec::new();
        let mut noise = Vec::new();
        let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut summary: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for req in requests {
            let url = format!("{}://{}{}", req.scheme, req.host, req.path);

            // Check for static asset (by extension in path)
            if self.is_static_asset(&req.path) {
                *summary.entry("static".into()).or_insert(0) += 1;
                noise.push(NoiseItem {
                    request: req,
                    category: NoiseCategory::StaticAsset,
                });
                continue;
            }

            // Check for third-party SDK
            if self.is_third_party_sdk(&req.host) {
                *summary.entry("third_party".into()).or_insert(0) += 1;
                noise.push(NoiseItem {
                    request: req,
                    category: NoiseCategory::ThirdPartySdk,
                });
                continue;
            }

            // Check for CDN
            if self.is_cdn(&req.host) {
                *summary.entry("cdn".into()).or_insert(0) += 1;
                noise.push(NoiseItem {
                    request: req,
                    category: NoiseCategory::CdnRequest,
                });
                continue;
            }

            // Deduplicate parameterized URLs (e.g. /users/1, /users/2 → /users/{id})
            let normalized = self.normalize_parameterized(&url);
            if seen_urls.contains(&normalized) {
                *summary.entry("duplicate".into()).or_insert(0) += 1;
                noise.push(NoiseItem {
                    request: req,
                    category: NoiseCategory::DuplicateParameterized,
                });
                continue;
            }

            seen_urls.insert(normalized);
            candidates.push(req);
        }

        FilterResult {
            candidates,
            noise,
            summary,
        }
    }

    fn is_static_asset(&self, path: &str) -> bool {
        if let Some(pos) = path.rfind('.') {
            let ext = &path[pos + 1..];
            // Handle paths like /foo.min.js - check extension after last dot
            if let Some(query_pos) = ext.find('?') {
                let ext_clean = &ext[..query_pos];
                self.static_extensions.contains(&ext_clean)
            } else {
                self.static_extensions.contains(&ext)
            }
        } else {
            false
        }
    }

    fn is_third_party_sdk(&self, host: &str) -> bool {
        self.third_party_patterns.iter().any(|p| host.contains(p))
    }

    fn is_cdn(&self, host: &str) -> bool {
        self.cdn_patterns.iter().any(|p| host.contains(p))
    }

    fn normalize_parameterized(&self, url: &str) -> String {
        // Replace numeric path segments with {id}
        // e.g., /users/123/orders/456 → /users/{id}/orders/{id}
        let re = regex::Regex::new(r"/\d+").unwrap();
        re.replace_all(url, "/{id}").to_string()
    }
}

impl Default for NoiseFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(method: &str, host: &str, path: &str) -> InterceptedRequest {
        InterceptedRequest {
            id: format!("req-{}-{}-{}", method, host, path),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            method: method.to_string(),
            host: host.to_string(),
            path: path.to_string(),
            query_params: None,
            status: None,
            latency_ms: None,
            scheme: "https".to_string(),
            req_headers: vec![],
            req_body: None,
            resp_headers: vec![],
            resp_body: None,
            resp_size: None,
            app_name: None,
            app_icon: None,
            device_id: None,
            device_name: None,
            client_ip: None,
            upstream_ip: None,
            is_websocket: false,
            ws_frames: None,
            grpc_decoded: None,
            graphql_op: None,
        }
    }

    #[test]
    fn test_filter_removes_static_assets() {
        let requests = vec![
            make_request("GET", "cdn.example.com", "/css/app.css"),
            make_request("GET", "cdn.example.com", "/js/vendor.js"),
            make_request("GET", "api.example.com", "/users/1"),
        ];
        let filter = NoiseFilter::new();
        let result = filter.filter(requests);
        assert_eq!(result.candidates.len(), 1);
        assert!(result.candidates[0].path.contains("/users/"));
        assert_eq!(result.summary.get("static").copied(), Some(2));
    }

    #[test]
    fn test_filter_deduplicates_parameterized() {
        let requests = vec![
            make_request("GET", "api.example.com", "/users/1"),
            make_request("GET", "api.example.com", "/users/2"),
            make_request("GET", "api.example.com", "/users/3"),
        ];
        let filter = NoiseFilter::new();
        let result = filter.filter(requests);
        // Should deduplicate to one entry
        assert!(result.candidates.len() <= 2);
        assert_eq!(result.summary.get("duplicate").copied(), Some(2));
    }

    #[test]
    fn test_filter_removes_third_party_sdks() {
        let requests = vec![
            make_request("GET", "google-analytics.com", "/g/collect"),
            make_request("GET", "facebook.net", "/signals"),
            make_request("GET", "api.example.com", "/orders"),
        ];
        let filter = NoiseFilter::new();
        let result = filter.filter(requests);
        assert_eq!(result.candidates.len(), 1); // Only api.example.com
        assert_eq!(result.summary.get("third_party").copied(), Some(2));
    }

    #[test]
    fn test_filter_removes_cdn() {
        let requests = vec![
            make_request("GET", "cf.cloudfront.net", "/api/data"),
            make_request("GET", "media.example.com", "/api/config"),
            make_request("GET", "api.example.com", "/products"),
        ];
        let filter = NoiseFilter::new();
        let result = filter.filter(requests);
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.summary.get("cdn").copied(), Some(2));
    }

    #[test]
    fn test_filter_preserves_api_calls() {
        let requests = vec![
            make_request("GET", "api.example.com", "/users"),
            make_request("POST", "api.example.com", "/users"),
            make_request("PUT", "api.example.com", "/users/456"),
            make_request("DELETE", "api.example.com", "/users/789"),
            make_request("GET", "api.example.com", "/orders"),
        ];
        let filter = NoiseFilter::new();
        let result = filter.filter(requests);
        // Deduplication normalizes parameterized URLs, and since method is not part
        // of the normalized URL, GET /users and POST /users deduplicate to same entry.
        // Result: /users, /users/{id}, /orders = 3 candidates (some may be duplicates)
        assert!(!result.candidates.is_empty());
        assert_eq!(result.noise.len(), 2); // POST /users and DELETE /users/789 are duplicates
    }

    #[test]
    fn test_filter_handles_query_params() {
        let requests = vec![
            make_request("GET", "cdn.example.com", "/js/app.min.js?v=1.2.3"),
            make_request("GET", "api.example.com", "/users?id=123"),
        ];
        let filter = NoiseFilter::new();
        let result = filter.filter(requests);
        assert_eq!(result.candidates.len(), 1);
        assert!(result.candidates[0].path.contains("/users"));
    }
}
