//! Spec source classification + coverage report.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecSource {
    /// Fully produced by DeepSeek.
    Llm,
    /// Fully produced by `extract` heuristics (LLM unavailable).
    Heuristic,
    /// Mixed: LLM + extract filled in the gaps.
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub total_requests: usize,
    pub covered_in_openapi: usize,
    pub covered_in_asyncapi: usize,
    pub uncovered_paths: Vec<String>,
    pub coverage_rate: f32,
}

impl CoverageReport {
    pub fn compute(
        total: usize,
        openapi_paths: &[String],
        asyncapi_channels: &[String],
        all_request_paths: &[String],
    ) -> Self {
        let covered_openapi = all_request_paths
            .iter()
            .filter(|p| openapi_paths.iter().any(|t| path_template_matches(t, p)))
            .count();
        let covered_asyncapi = all_request_paths
            .iter()
            .filter(|p| asyncapi_channels.iter().any(|t| path_template_matches(t, p)))
            .count();
        let uncovered = all_request_paths
            .iter()
            .filter(|p| {
                !openapi_paths.iter().any(|t| path_template_matches(t, p))
                    && !asyncapi_channels.iter().any(|t| path_template_matches(t, p))
            })
            .cloned()
            .collect();
        let rate = if total == 0 {
            0.0
        } else {
            (covered_openapi + covered_asyncapi) as f32 / total as f32
        };
        Self {
            total_requests: total,
            covered_in_openapi: covered_openapi,
            covered_in_asyncapi: covered_asyncapi,
            uncovered_paths: uncovered,
            coverage_rate: rate,
        }
    }
}

/// True when a path like `/api/users/{id}` matches a concrete `/api/users/42`.
fn path_template_matches(template: &str, concrete: &str) -> bool {
    let t: Vec<&str> = template.split('/').collect();
    let c: Vec<&str> = concrete.split('/').collect();
    if t.len() != c.len() {
        return false;
    }
    t.iter().zip(c.iter()).all(|(tt, cc)| *tt == *cc || (tt.starts_with('{') && tt.ends_with('}')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_source_serializes() {
        let yaml = serde_yaml::to_string(&SpecSource::Hybrid).unwrap();
        assert!(yaml.contains("Hybrid"));
    }

    #[test]
    fn path_template_matches_handles_params() {
        assert!(path_template_matches("/users/{id}", "/users/42"));
        assert!(!path_template_matches("/users/{id}", "/users/42/posts"));
        assert!(path_template_matches("/api/v3/user/profile", "/api/v3/user/profile"));
    }

    #[test]
    fn coverage_with_full_match_is_one() {
        let r = CoverageReport::compute(
            2,
            &["/users/{id}".into()],
            &[],
            &["/users/42".into(), "/users/43".into()],
        );
        assert_eq!(r.covered_in_openapi, 2);
        assert!((r.coverage_rate - 1.0).abs() < 0.0001);
        assert!(r.uncovered_paths.is_empty());
    }

    #[test]
    fn coverage_reports_uncovered() {
        let r = CoverageReport::compute(
            3,
            &["/users/{id}".into()],
            &[],
            &["/users/42".into(), "/posts/1".into(), "/comments/9".into()],
        );
        assert_eq!(r.covered_in_openapi, 1);
        assert_eq!(r.uncovered_paths.len(), 2);
    }
}