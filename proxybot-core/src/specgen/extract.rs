//! Heuristic extraction: turn concrete paths into templates and cluster params.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PathTemplate {
    pub template: String,   // e.g. "/api/users/{id}"
    pub method: String,
    pub param_names: Vec<String>,
}

/// Turn a concrete path into a template by replacing numeric/alnum ids with `{name}`.
///
/// Heuristic: any segment that is purely digits or hex/uuid-shaped becomes a parameter.
/// The parameter name is the previous segment + "Id" (or "Param" if no previous).
pub fn template_path(path: &str) -> PathTemplate {
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    let mut templated = Vec::with_capacity(segments.len());
    let mut params = Vec::new();

    for (i, seg) in segments.iter().enumerate() {
        if is_param_like(seg) {
            let name = if i == 0 {
                "param".to_string()
            } else {
                sanitize_param_name(segments[i - 1])
            };
            templated.push(format!("{{{}}}", name));
            params.push(name);
        } else {
            templated.push(seg.to_string());
        }
    }
    let template = format!("/{}", templated.join("/"));
    PathTemplate {
        template,
        method: String::new(),
        param_names: params,
    }
}

fn is_param_like(seg: &str) -> bool {
    if seg.is_empty() {
        return false;
    }
    if seg.parse::<u64>().is_ok() {
        return true;
    }
    if seg.len() == 32 || seg.len() == 36 {
        return seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    }
    false
}

fn sanitize_param_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    format!("{}Id", cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_segment_becomes_param() {
        let t = template_path("/api/users/42");
        assert_eq!(t.template, "/api/users/{usersId}");
        assert_eq!(t.param_names, vec!["usersId".to_string()]);
    }

    #[test]
    fn plain_path_unchanged() {
        let t = template_path("/api/v3/user/profile");
        assert_eq!(t.template, "/api/v3/user/profile");
        assert!(t.param_names.is_empty());
    }

    #[test]
    fn uuid_segment_becomes_param() {
        let t = template_path("/api/sessions/123e4567-e89b-12d3-a456-426614174000");
        assert!(t.template.contains("{sessionsId}"));
    }

    #[test]
    fn first_segment_numeric_uses_param() {
        let t = template_path("/42/details");
        assert_eq!(t.template, "/{param}/details");
    }
}

/// Cluster a list of (method, path) records by templated path.
/// Returns a map: template -> { method -> example_concrete_path }.
pub fn cluster_paths(records: &[(String, String)]) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (method, path) in records {
        let tpl = template_path(path);
        let entry = out.entry(tpl.template).or_default();
        entry.entry(method.to_uppercase()).or_insert_with(|| path.clone());
    }
    out
}

/// Pull request-body keys (top-level) from a JSON body string.
pub fn body_keys(body: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
        .unwrap_or_default()
}

#[cfg(test)]
mod cluster_tests {
    use super::*;

    #[test]
    fn clusters_same_template_together() {
        let recs = vec![
            ("GET".into(), "/api/users/1".into()),
            ("GET".into(), "/api/users/2".into()),
            ("POST".into(), "/api/users".into()),
        ];
        let out = cluster_paths(&recs);
        assert_eq!(out.len(), 2);
        assert!(out.contains_key("/api/users/{usersId}"));
        assert!(out["/api/users/{usersId}"].contains_key("GET"));
        assert!(out.contains_key("/api/users"));
        assert!(out["/api/users"].contains_key("POST"));
    }

    #[test]
    fn body_keys_returns_top_level_keys() {
        let keys = body_keys(r#"{"name": "x", "age": 1}"#);
        assert_eq!(keys, vec!["age".to_string(), "name".to_string()]);
    }

    #[test]
    fn body_keys_handles_invalid_json() {
        assert!(body_keys("not json").is_empty());
    }
}
