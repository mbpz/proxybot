//! Phase 2: LLM-based API analysis for the AI analysis pipeline.
//!
//! Takes filtered candidate requests from Phase 1 (NoiseFilter) and produces
//! an API specification via LLM inference.

use crate::ai_pipeline::cost::estimate_tokens_for_spec;
use serde::Serialize;

/// Result of Phase 2 LLM inference
#[derive(Debug, Serialize)]
pub struct ApiAnalysisResult {
    /// Generated API specification (OpenAPI/JSON)
    pub spec: serde_json::Value,
    /// Estimated tokens used
    pub tokens_used: usize,
    /// Estimated cost in USD
    pub cost_usd: f64,
    /// Number of requests analyzed
    pub requests_analyzed: usize,
}

impl ApiAnalysisResult {
    /// Create a new analysis result from candidates.
    /// Note: Actual LLM inference requires ANTHROPIC_API_KEY env var.
    /// This method returns a placeholder result for testing.
    pub fn from_candidates(candidates: &[crate::proxy::InterceptedRequest]) -> Self {
        let count = candidates.len();
        // Create a placeholder spec structure
        let spec = serde_json::json!({
            "info": {
                "title": "API Analysis Result",
                "description": format!("Analysis of {} requests", count)
            },
            "requests_analyzed": count,
            "endpoints": candidates.iter().map(|req| {
                serde_json::json!({
                    "method": req.method,
                    "path": req.path,
                    "host": req.host
                })
            }).collect::<Vec<_>>()
        });
        let tokens_used = estimate_tokens_for_spec(&spec);

        ApiAnalysisResult {
            spec,
            tokens_used,
            cost_usd: tokens_used as f64 * 0.000015,
            requests_analyzed: count,
        }
    }
}

/// LLM-based API analyzer that produces specs from filtered requests.
pub struct ApiAnalyzer;

impl ApiAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Analyze cleaned candidate requests to produce an API spec.
    /// Returns a result with token/cost estimates.
    /// Note: Full LLM inference requires the run_ai_pipeline Tauri command
    /// which has access to the database state.
    pub fn analyze(
        &self,
        candidates: Vec<crate::proxy::InterceptedRequest>,
    ) -> ApiAnalysisResult {
        if candidates.is_empty() {
            return ApiAnalysisResult {
                spec: serde_json::json!({"paths": {}, "info": {"title": "Empty API"}}),
                tokens_used: 0,
                cost_usd: 0.0,
                requests_analyzed: 0,
            };
        }

        ApiAnalysisResult::from_candidates(&candidates)
    }
}

impl Default for ApiAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_result_struct() {
        let result = ApiAnalysisResult {
            spec: serde_json::json!({"paths": {}}),
            tokens_used: 100,
            cost_usd: 0.0015,
            requests_analyzed: 5,
        };
        assert_eq!(result.requests_analyzed, 5);
        assert_eq!(result.tokens_used, 100);
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = ApiAnalyzer::new();
        let _ = analyzer;
    }

    #[test]
    fn test_analyzer_empty() {
        let analyzer = ApiAnalyzer::new();
        let result = analyzer.analyze(vec![]);
        assert_eq!(result.requests_analyzed, 0);
    }

    #[test]
    fn test_analyzer_with_candidates() {
        use crate::proxy::InterceptedRequest;
        let candidates = vec![
            InterceptedRequest {
                id: "1".to_string(),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
                method: "GET".to_string(),
                host: "api.example.com".to_string(),
                path: "/users".to_string(),
                ..Default::default()
            },
        ];
        let analyzer = ApiAnalyzer::new();
        let result = analyzer.analyze(candidates);
        assert_eq!(result.requests_analyzed, 1);
    }
}