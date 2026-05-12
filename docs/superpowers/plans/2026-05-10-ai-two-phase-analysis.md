# AI Two-Phase Analysis Implementation Plan

**Date:** 2026-05-10
**Feature:** AI Two-Phase Analysis
**Priority:** P1
**Estimated Duration:** 1 week

---

## 1. Overview

This plan implements a two-phase AI analysis system that first filters out noise (auth requests, CDN traffic, static assets) and then performs deep analysis (API structure, security issues, performance bottlenecks). The system integrates with the existing `infer.rs` module and streams responses to the UI.

## 2. Current Architecture

The existing `infer.rs` module at `src-tauri/src/infer.rs` provides:
- `ApiInterface` and `ApiModule` types
- `InferenceResult` for LLM response validation
- `generate_openapi_spec()` function
- Claude API integration for inference

The Gen tab currently sends traffic data to Claude and receives inferred API structures.

## 3. Two-Phase Design

### Phase 1: Smart Noise Filtering

Remove low-value requests before sending to LLM:
- Authentication requests (token refresh, login, OAuth)
- CDN/static asset requests (`.js`, `.css`, images, fonts)
- Health checks and ping requests
- Tracking/beacon requests (analytics, crash reporting)

**Benefits:**
- Reduces token usage by 60-80%
- Improves LLM focus on actual API traffic
- Faster response times

### Phase 2: Deep Analysis

After filtering, analyze remaining requests for:
- API structure and endpoint patterns
- Authentication mechanisms
- Security vulnerabilities (exposed keys, sensitive data in URLs)
- Performance issues (slow responses, large payloads)
- Error handling patterns

## 4. Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                     Traffic Analysis Pipeline                     │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                      Phase 1: Noise Filter                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ Auth Filter │  │ CDN Filter  │  │ Track Filter│  ...         │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                              │                                     │
│                    Filtered Traffic                               │
│                         (~20-40% of original)                     │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                     Phase 2: Deep Analysis                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ API Struct  │  │ SecAudit    │  │ PerfAudit   │  ...         │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                              │                                     │
│                    Analysis Results                               │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                        UI Streaming                               │
│                    (SSE or WebSocket)                             │
└──────────────────────────────────────────────────────────────────┘
```

## 5. Implementation Steps

### Day 1-2: Noise Filter Framework

**File:** `src-tauri/src/ai_analysis/mod.rs` (NEW)

Create module structure:

```rust
//! AI-powered traffic analysis module
//!
//! Two-phase analysis: noise filtering followed by deep analysis.

pub mod filter;
pub mod analysis;
pub mod streaming;

use serde::{Deserialize, Serialize};

/// Phase 1 filter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub filter_auth: bool,
    pub filter_cdn: bool,
    pub filter_static: bool,
    pub filter_tracking: bool,
    pub filter_health: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            filter_auth: true,
            filter_cdn: true,
            filter_static: true,
            filter_tracking: true,
            filter_health: true,
        }
    }
}

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub detect_api_structure: bool,
    pub detect_security_issues: bool,
    pub detect_performance_issues: bool,
    pub include_examples: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            detect_api_structure: true,
            detect_security_issues: true,
            detect_performance_issues: true,
            include_examples: true,
        }
    }
}
```

**File:** `src-tauri/src/ai_analysis/filter/mod.rs` (NEW)

```rust
//! Phase 1: Noise filtering

pub mod auth;
pub mod cdn;
pub mod static_assets;
pub mod tracking;

use crate::db::RequestRecord;

pub trait NoiseFilter: Send + Sync {
    /// Returns true if the request should be filtered out
    fn is_noise(&self, request: &RequestRecord) -> bool;

    /// Human-readable name for the filter
    fn name(&self) -> &'static str;

    /// Why this request was filtered (for debugging/logging)
    fn reason(&self, request: &RequestRecord) -> Option<String>;
}

/// Composite filter that applies multiple filters
pub struct CompositeFilter {
    filters: Vec:Box<dyn NoiseFilter>>,
}

impl CompositeFilter {
    pub fn new(config: &FilterConfig) -> Self {
        let mut filters = Vec::new();

        if config.filter_auth {
            filters.push(Box::new(auth::AuthFilter::new()));
        }
        if config.filter_cdn {
            filters.push(Box::new(cdn::CdnFilter::new()));
        }
        if config.filter_static {
            filters.push(Box::new(static_assets::StaticFilter::new()));
        }
        if config.filter_tracking {
            filters.push(Box::new(tracking::TrackingFilter::new()));
        }
        if config.filter_health {
            filters.push(Box::new(health::HealthFilter::new()));
        }

        Self { filters }
    }

    pub fn apply(&self, requests: &[RequestRecord]) -> FilterResult {
        let mut kept = Vec::new();
        let mut removed = Vec::new();

        for request in requests {
            let mut filtered = false;
            let mut filter_name = None;
            let mut reason = None;

            for filter in &self.filters {
                if filter.is_noise(request) {
                    filtered = true;
                    filter_name = Some(filter.name());
                    reason = filter.reason(request);
                    break;
                }
            }

            if filtered {
                removed.push(FilteredRequest {
                    request: request.clone(),
                    filter: filter_name.unwrap_or("unknown"),
                    reason: reason.unwrap_or_default(),
                });
            } else {
                kept.push(request.clone());
            }
        }

        FilterResult { kept, removed }
    }
}

#[derive(Debug)]
pub struct FilterResult {
    pub kept: Vec<RequestRecord>,
    pub removed: Vec<FilteredRequest>,
}

#[derive(Debug, Clone)]
pub struct FilteredRequest {
    pub request: RequestRecord,
    pub filter: &'static str,
    pub reason: String,
}
```

**File:** `src-tauri/src/ai_analysis/filter/auth.rs` (NEW)

```rust
//! Authentication request filter

use super::NoiseFilter;
use crate::db::RequestRecord;

pub struct AuthFilter;

impl AuthFilter {
    pub fn new() -> Self {
        Self
    }
}

impl NoiseFilter for AuthFilter {
    fn is_noise(&self, request: &RequestRecord) -> bool {
        let host = request.host.to_lowercase();
        let path = request.path.to_lowercase();

        // OAuth/token endpoints
        if path.contains("/oauth/") || path.contains("/token") || path.contains("/authorize") {
            return true;
        }

        // Login endpoints
        if path.contains("/login") || path.contains("/signin") || path.contains("/auth/") {
            return true;
        }

        // Refresh tokens
        if path.contains("/refresh") && path.contains("/token") {
            return true;
        }

        // SAML endpoints
        if path.contains("/saml/") || path.contains("/sso") {
            return true;
        }

        // Common auth hosts
        if host.contains("auth") || host.contains("login") || host.contains("sso") {
            return true;
        }

        false
    }

    fn name(&self) -> &'static str {
        "auth"
    }

    fn reason(&self, request: &RequestRecord) -> Option<String> {
        Some(format!("Auth-related: {} {}", request.method, request.path))
    }
}
```

**File:** `src-tauri/src/ai_analysis/filter/cdn.rs` (NEW)

```rust
//! CDN and static asset filter

use super::NoiseFilter;
use crate::db::RequestRecord;

pub struct CdnFilter;

impl CdnFilter {
    pub fn new() -> Self {
        Self
    }

    fn known_cdn_patterns() -> Vec<&'static str> {
        vec![
            "cloudflare.com",
            "akamai.com",
            "fastly.net",
            "cloudfront.net",
            "cdn.jsdelivr.net",
            "unpkg.com",
            "cdnjs.cloudflare.com",
            "ajax.googleapis.com",
            "fonts.googleapis.com",
            "fonts.gstatic.com",
        ]
    }

    fn static_extensions() -> Vec<&'static str> {
        vec![
            ".js", ".css", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".ico",
            ".woff", ".woff2", ".ttf", ".eot", ".map", ".webp", ".webm",
            ".mp4", ".mp3", ".wav", ".ogg",
        ]
    }
}

impl NoiseFilter for CdnFilter {
    fn is_noise(&self, request: &RequestRecord) -> bool {
        let host = request.host.to_lowercase();
        let path = request.path.to_lowercase();

        // Known CDN hosts
        for cdn in Self::known_cdn_patterns() {
            if host.contains(cdn) {
                return true;
            }
        }

        // Static file extensions
        for ext in Self::static_extensions() {
            if path.ends_with(ext) {
                return true;
            }
        }

        // Versioned static assets
        if path.contains("/static/") || path.contains("/assets/") || path.contains("/dist/") {
            if Self::static_extensions().iter().any(|ext| path.contains(ext)) {
                return true;
            }
        }

        false
    }

    fn name(&self) -> &'static str {
        "cdn"
    }

    fn reason(&self, request: &RequestRecord) -> Option<String> {
        Some(format!("CDN/Static: {} {}", request.host, request.path))
    }
}
```

**File:** `src-tauri/src/ai_analysis/filter/tracking.rs` (NEW)

```rust
//! Tracking and analytics request filter

use super::NoiseFilter;
use crate::db::RequestRecord;

pub struct TrackingFilter;

impl TrackingFilter {
    pub fn new() -> Self {
        Self
    }

    fn tracking_patterns() -> Vec<(&'static str, Vec<&'static str>)> {
        vec![
            ("google-analytics", vec!["google-analytics.com", "googletagmanager.com"]),
            ("facebook", vec!["facebook.com/tr", "connect.facebook.net"]),
            ("segment", vec!["segment.io", "segment.com"]),
            ("mixpanel", vec!["mixpanel.com", "api.mixpanel.com"]),
            ("amplitude", vec!["amplitude.com", "api.amplitude.com"]),
            ("sentry", vec!["sentry.io", "browser.sentry-cdn.com"]),
            ("datadog", vec!["datadoghq.com", "browser-intake-datadoghq.com"]),
            ("newrelic", vec!["newrelic.com", "js-agent.newrelic.com"]),
        ]
    }
}

impl NoiseFilter for TrackingFilter {
    fn is_noise(&self, request: &RequestRecord) -> bool {
        let host = request.host.to_lowercase();
        let path = request.path.to_lowercase();

        // Analytics/tracking hosts
        for (_, hosts) in Self::tracking_patterns() {
            for tracking_host in hosts {
                if host.contains(tracking_host) {
                    return true;
                }
            }
        }

        // Tracking path patterns
        let tracking_paths = [
            "/collect", "/telemetry", "/events", "/batch", "/track",
            "/analytics", "/metrics", "/beacon", "/pixel", "/spy",
        ];

        for tracking_path in tracking_paths {
            if path.contains(tracking_path) {
                return true;
            }
        }

        false
    }

    fn name(&self) -> &'static str {
        "tracking"
    }

    fn reason(&self, request: &RequestRecord) -> Option<String> {
        Some(format!("Tracking: {} {}", request.host, request.path))
    }
}
```

### Day 3-4: Deep Analysis Engine

**File:** `src-tauri/src/ai_analysis/analysis/mod.rs` (NEW)

```rust
//! Phase 2: Deep analysis of filtered traffic

use serde::{Deserialize, Serialize};
use crate::ai_analysis::filter::RequestRecord;
use crate::infer::InferenceResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub api_structure: Option<ApiStructure>,
    pub security_issues: Vec<SecurityIssue>,
    pub performance_issues: Vec<PerformanceIssue>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStructure {
    pub endpoints: Vec<Endpoint>,
    pub modules: Vec<ApiModule>,
    pub authentication: AuthMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub method: String,
    pub path: String,
    pub params: Vec<Param>,
    pub responses: Vec<Response>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub location: String,  // query, path, header, body
    pub type_: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub status: u16,
    pub content_type: Option<String>,
    pub schema: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiModule {
    pub name: String,
    pub description: String,
    pub endpoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    None,
    ApiKey(String),
    BearerToken,
    BasicAuth,
    OAuth,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub affected_requests: Vec<String>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceIssue {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub affected_requests: Vec<String>,
    pub recommendation: String,
}

pub struct DeepAnalyzer {
    config: AnalysisConfig,
}

impl DeepAnalyzer {
    pub fn new(config: AnalysisConfig) -> Self {
        Self { config }
    }

    pub async fn analyze(&self, requests: &[RequestRecord]) -> Result<AnalysisResult, String> {
        let mut result = AnalysisResult {
            api_structure: None,
            security_issues: Vec::new(),
            performance_issues: Vec::new(),
            summary: String::new(),
        };

        // Run analyses in parallel
        let api_structure = if self.config.detect_api_structure {
            Some(self.detect_api_structure(requests).await?)
        } else {
            None
        };

        let (security, performance) = tokio::join!(
            self.detect_security_issues(requests),
            self.detect_performance_issues(requests),
        );

        result.api_structure = api_structure;
        result.security_issues = security?;
        result.performance_issues = performance?;

        Ok(result)
    }

    async fn detect_api_structure(&self, requests: &[RequestRecord]) -> Result<ApiStructure, String> {
        // Convert requests to prompt format for LLM
        let prompt = self.build_api_detection_prompt(requests);

        let response = call_claude(prompt).await?;

        // Parse LLM response into ApiStructure
        self.parse_api_response(response)
    }

    async fn detect_security_issues(&self, requests: &[RequestRecord]) -> Result<Vec<SecurityIssue>, String> {
        // Check for exposed API keys in URLs
        let mut issues = Vec::new();

        for request in requests {
            // Check for API keys in query params
            if let Some(query) = request.path.split('?').nth(1) {
                for param in query.split('&') {
                    let param_lower = param.to_lowercase();
                    if param_lower.contains("key")
                        || param_lower.contains("token")
                        || param_lower.contains("secret")
                        || param_lower.contains("password")
                    {
                        if let Some(value) = param.split('=').nth(1) {
                            if !value.is_empty() && value.len() > 8 {
                                issues.push(SecurityIssue {
                                    severity: Severity::High,
                                    title: "Potential API Key in URL".to_string(),
                                    description: format!(
                                        "Sensitive parameter '{}' found in URL query string",
                                        param.split('=').next().unwrap_or("")
                                    ),
                                    affected_requests: vec![request.id.clone()],
                                    recommendation: "Move sensitive parameters to headers or use POST body".to_string(),
                                });
                            }
                        }
                    }
                }
            }

            // Check for HTTP (non-HTTPS) traffic
            if request.scheme == "http" && request.host.contains("api.") {
                issues.push(SecurityIssue {
                    severity: Severity::Medium,
                    title: "Non-HTTPS API Traffic".to_string(),
                    description: format!(
                        "API request to {} uses unencrypted HTTP",
                        request.host
                    ),
                    affected_requests: vec![request.id.clone()],
                    recommendation: "Use HTTPS for all API communication".to_string(),
                });
            }
        }

        Ok(issues)
    }

    async fn detect_performance_issues(&self, requests: &[RequestRecord]) -> Result<Vec<PerformanceIssue>, String> {
        let mut issues = Vec::new();

        for request in requests {
            // Check for large request bodies
            if request.request_size > 1_000_000 {
                issues.push(PerformanceIssue {
                    severity: Severity::Medium,
                    title: "Large Request Body".to_string(),
                    description: format!(
                        "Request body size is {} bytes",
                        request.request_size
                    ),
                    affected_requests: vec![request.id.clone()],
                    recommendation: "Consider compressing large request bodies".to_string(),
                });
            }

            // Check for large response bodies
            if let Some(size) = request.response_size {
                if size > 5_000_000 {
                    issues.push(PerformanceIssue {
                        severity: Severity::Medium,
                        title: "Large Response Body".to_string(),
                        description: format!(
                            "Response body size is {} bytes",
                            size
                        ),
                        affected_requests: vec![request.id.clone()],
                        recommendation: "Consider pagination or compression for large responses".to_string(),
                    });
                }
            }
        }

        Ok(issues)
    }

    fn build_api_detection_prompt(&self, requests: &[RequestRecord]) -> String {
        let sample_count = std::cmp::min(requests.len(), 50);
        let samples: Vec<String> = requests.iter().take(sample_count).map(|r| {
            format!("{} {} {} - {}",
                r.method,
                r.host,
                r.path,
                r.status
            )
        }).collect();

        format!(r#"Analyze these API requests and identify:
1. API structure (endpoints, methods, parameters)
2. Authentication mechanisms
3. Common patterns

Requests:
{}

Provide JSON output with:
- endpoints: array of {method, path, params}
- modules: grouping of related endpoints
- authentication: auth mechanism detected"#, samples.join("\n"))
    }
}
```

### Day 5: Streaming Response to UI

**File:** `src-tauri/src/ai_analysis/streaming/mod.rs` (NEW)

```rust
//! SSE streaming for analysis results

use axum::{
    response::sse::{Event, Sse},
    Router,
};
use tokio_stream::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::ai_analysis::{FilterConfig, AnalysisConfig};

pub async fn stream_analysis(
    requests: Vec<RequestRecord>,
    filter_config: FilterConfig,
    analysis_config: AnalysisConfig,
) -> Sse<impl StreamExt<Item = Result<Event, std::convert::Infallible>>> {
    let (tx, rx) = mpsc::channel(100);

    tokio::spawn(async move {
        // Phase 1: Filtering
        let filter = CompositeFilter::new(&filter_config);
        let filter_result = filter.apply(&requests);

        tx.send(Ok(Event::from_data(serde_json::json!({
            "phase": "filter_complete",
            "stats": {
                "total": requests.len(),
                "kept": filter_result.kept.len(),
                "removed": filter_result.removed.len(),
            },
            "removed_sample": filter_result.removed.iter().take(5).map(|r| {
                serde_json::json!({
                    "reason": r.filter,
                    "request": r.request.path,
                })
            }).collect::<Vec<_>>(),
        }).to_string()))).await;

        // Phase 2: Analysis (streaming progress)
        let analyzer = DeepAnalyzer::new(analysis_config);

        let analysis_result = analyzer.analyze(&filter_result.kept).await;

        match analysis_result {
            Ok(result) => {
                // Send API structure
                if let Some(api) = result.api_structure {
                    tx.send(Ok(Event::from_data(serde_json::json!({
                        "phase": "api_structure",
                        "data": api,
                    }).to_string()))).await;
                }

                // Send security issues
                if !result.security_issues.is_empty() {
                    tx.send(Ok(Event::from_data(serde_json::json!({
                        "phase": "security_issues",
                        "data": result.security_issues,
                    }).to_string()))).await;
                }

                // Send performance issues
                if !result.performance_issues.is_empty() {
                    tx.send(Ok(Event::from_data(serde_json::json!({
                        "phase": "performance_issues",
                        "data": result.performance_issues,
                    }).to_string()))).await;
                }

                // Send complete
                tx.send(Ok(Event::from_data(serde_json::json!({
                    "phase": "complete",
                    "summary": result.summary,
                }).to_string()))).await;
            }
            Err(e) => {
                tx.send(Ok(Event::from_data(serde_json::json!({
                    "phase": "error",
                    "error": e,
                }).to_string()))).await;
            }
        }
    });

    Sse::new(rx)
}
```

### Day 6-7: Integration with Existing Gen Tab

**File:** `src-tauri/src/commands/ai_analysis.rs` (NEW)

Add Tauri commands:

```rust
use crate::ai_analysis::{FilterConfig, AnalysisConfig};
use crate::ai_analysis::streaming::stream_analysis;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisOptions {
    pub filter_config: FilterConfig,
    pub analysis_config: AnalysisConfig,
    pub session_id: Option<String>,
}

#[tauri::command]
pub async fn start_traffic_analysis(
    options: AnalysisOptions,
    db_state: State<'_, DbState>,
) -> Result<String, String> {
    // Fetch requests from database
    let requests = fetch_requests(&db_state, options.session_id.as_deref())?;

    // Start streaming analysis
    let event_stream = stream_analysis(requests, options.filter_config, options.analysis_config).await;

    // Return SSE endpoint URL
    Ok(format!("/analysis/stream/{}", generate_stream_id()))
}

#[tauri::command]
pub fn get_analysis_status(stream_id: String) -> AnalysisStatus {
    // Track streaming progress
}
```

Modify `infer.rs` to use the new two-phase system:

```rust
// In infer.rs, add method that delegates to ai_analysis module
pub async fn infer_with_two_phase(
    requests: Vec<RequestRecord>,
    config: AnalysisConfig,
) -> Result<InferenceResult, String> {
    let filter_config = FilterConfig::default();
    let analysis_result = ai_analysis::analyze(requests, filter_config, config).await?;

    // Convert to InferenceResult format for compatibility
    Ok(InferenceResult {
        interfaces: analysis_result.api_structure.map(|s| s.endpoints.into_iter().map(|e| {
            crate::infer::ApiInterface {
                name: format!("{} {}", e.method, e.path),
                method: e.method,
                path: e.path,
                params: serde_json::to_string(&e.params).unwrap_or_default(),
                auth_required: matches!(analysis_result.api_structure.authentication, AuthMethod::None),
            }
        }).collect()).unwrap_or_default(),
        modules: analysis_result.api_structure.map(|s| s.modules).unwrap_or_default(),
        valid: true,
        errors: vec![],
        score: 0.8,
    })
}
```

## 6. Key Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/ai_analysis/mod.rs` | CREATE | Module root |
| `src-tauri/src/ai_analysis/filter/mod.rs` | CREATE | Filter framework |
| `src-tauri/src/ai_analysis/filter/auth.rs` | CREATE | Auth filter |
| `src-tauri/src/ai_analysis/filter/cdn.rs` | CREATE | CDN filter |
| `src-tauri/src/ai_analysis/filter/static_assets.rs` | CREATE | Static filter |
| `src-tauri/src/ai_analysis/filter/tracking.rs` | CREATE | Tracking filter |
| `src-tauri/src/ai_analysis/filter/health.rs` | CREATE | Health check filter |
| `src-tauri/src/ai_analysis/analysis/mod.rs` | CREATE | Analysis engine |
| `src-tauri/src/ai_analysis/streaming/mod.rs` | CREATE | SSE streaming |
| `src-tauri/src/commands/ai_analysis.rs` | CREATE | Tauri commands |
| `src-tauri/src/infer.rs` | MODIFY | Delegate to ai_analysis |
| `src-tauri/src/lib.rs` | MODIFY | Register ai_analysis module |
| `frontend/pages/GenTab.tsx` | MODIFY | Connect to streaming endpoint |

## 7. Dependencies

```toml
# Cargo.toml additions
tokio-stream = "0.1"
axum = { version = "0.7", features = ["sse"] }
```

## 8. Testing Strategy

### Unit Tests

- Test each noise filter in isolation with mock requests
- Test filter composition
- Test analysis result parsing

### Integration Tests

- Test full pipeline with sample traffic data
- Verify streaming output format

### UI Verification

- Verify streaming updates in Gen tab
- Verify filter stats display correctly

## 9. Performance Targets

| Metric | Target |
|--------|--------|
| Noise filtering | < 10ms for 1000 requests |
| API structure detection | < 2s for 100 filtered requests |
| Security scanning | < 500ms for 1000 requests |
| Total analysis | < 5s for typical session |

## 10. Timeline

| Day | Task |
|-----|------|
| 1 | Noise filter framework and auth filter |
| 2 | CDN, static, tracking, health filters |
| 3 | Deep analysis engine (API structure) |
| 4 | Security and performance analysis |
| 5 | SSE streaming to UI |
| 6-7 | Integration with Gen tab, testing |