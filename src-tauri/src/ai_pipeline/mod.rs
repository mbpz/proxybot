//! AI Two-Phase Analysis Pipeline
//!
//! Implements a two-stage pipeline for analyzing HTTP traffic:
//! - Phase 1: NoiseFilter - removes static assets, CDN, third-party SDKs, deduplicates parameterized URLs
//! - Phase 2: ApiAnalyzer - runs LLM inference on filtered candidates to produce API specs
//!
//! This reduces token cost ~60% by filtering out noise before LLM inference.

pub mod analyzer;
pub mod cost;
pub mod filter;

pub use analyzer::{ApiAnalysisResult, ApiAnalyzer};
pub use cost::{
    estimate_cost, estimate_pipeline_cost, estimate_tokens_for_spec, estimate_tokens_for_text,
};
pub use filter::{FilterResult, NoiseCategory, NoiseFilter, NoiseItem};

use serde::Serialize;

/// Combined pipeline result from both phases
#[derive(Debug, Serialize)]
pub struct PipelineResult {
    /// Phase 1 filter result
    pub filter_result: FilterResult,
    /// Phase 2 analysis result (if run)
    pub analysis_result: Option<ApiAnalysisResult>,
    /// Session ID used
    pub session_id: String,
}

impl PipelineResult {
    /// Total requests processed
    pub fn total_requests(&self) -> usize {
        self.filter_result.candidates.len() + self.filter_result.noise.len()
    }

    /// Noise ratio (0.0 to 1.0)
    pub fn noise_ratio(&self) -> f64 {
        let total = self.total_requests() as f64;
        if total == 0.0 {
            0.0
        } else {
            self.filter_result.noise.len() as f64 / total
        }
    }
}

/// The complete two-phase AI analysis pipeline
pub struct AiPipeline;

impl AiPipeline {
    pub fn new() -> Self {
        Self
    }

    /// Run both phases: filter noise, then analyze candidates
    pub fn run(
        &self,
        requests: Vec<crate::proxy::InterceptedRequest>,
        session_id: &str,
    ) -> PipelineResult {
        // Phase 1: Filter noise
        let filter = NoiseFilter::new();
        let filter_result = filter.filter(requests);

        // Phase 2: Analyze candidates
        let analyzer = ApiAnalyzer::new();
        let analysis_result = analyzer.analyze(filter_result.candidates.clone());

        PipelineResult {
            filter_result,
            analysis_result: Some(analysis_result),
            session_id: session_id.to_string(),
        }
    }

    /// Run only Phase 1 (filter), returning the filter result
    pub fn filter_only(&self, requests: Vec<crate::proxy::InterceptedRequest>) -> FilterResult {
        let filter = NoiseFilter::new();
        filter.filter(requests)
    }
}

impl Default for AiPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::InterceptedRequest;

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
    fn test_pipeline_filter_only() {
        let requests = vec![
            make_request("GET", "cdn.example.com", "/css/app.css"),
            make_request("GET", "api.example.com", "/users/1"),
            make_request("GET", "api.example.com", "/users/2"),
        ];
        let pipeline = AiPipeline::new();
        let result = pipeline.filter_only(requests);
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.noise.len(), 2);
    }

    #[test]
    fn test_noise_ratio() {
        let requests = vec![
            make_request("GET", "cdn.example.com", "/img/logo.png"),
            make_request("GET", "cdn.example.com", "/js/vendor.js"),
            make_request("GET", "api.example.com", "/users"),
        ];
        let pipeline = AiPipeline::new();
        // run() returns PipelineResult which has noise_ratio()
        let result = pipeline.run(requests, "test-session");
        // 2 noise, 1 candidate = 2/3 ratio
        assert!((result.noise_ratio() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_pipeline_result_total_requests() {
        let requests = vec![
            make_request("GET", "cdn.example.com", "/css/app.css"),
            make_request("GET", "api.example.com", "/users"),
        ];
        let pipeline = AiPipeline::new();
        // run() returns PipelineResult which has total_requests()
        let result = pipeline.run(requests, "test-session");
        assert_eq!(result.total_requests(), 2);
    }

    #[test]
    fn test_pipeline_run() {
        let requests = vec![
            make_request("GET", "cdn.example.com", "/css/app.css"),
            make_request("GET", "api.example.com", "/users"),
        ];
        let pipeline = AiPipeline::new();
        let result = pipeline.run(requests, "test-session");
        assert_eq!(result.filter_result.candidates.len(), 1);
        assert!(result.analysis_result.is_some());
    }
}
