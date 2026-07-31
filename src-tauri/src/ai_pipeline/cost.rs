//! Token estimation and cost calculation for AI analysis pipeline.

use serde_json::Value;

/// Estimate token count from a JSON spec.
/// Uses approximate ratio: 4 chars per token (typical for English text).
pub fn estimate_tokens_for_spec(spec: &Value) -> usize {
    let spec_str = serde_json::to_string(spec).unwrap_or_default();
    spec_str.chars().count() / 4
}

/// Estimate token count from raw text.
pub fn estimate_tokens_for_text(text: &str) -> usize {
    text.chars().count() / 4
}

/// Calculate estimated cost in USD for a given token count.
pub fn estimate_cost(
    provider: &str,
    model: &str,
    input_tokens: usize,
    output_tokens: usize,
) -> f64 {
    // Pricing per 1M tokens (as of 2024)
    let (input_rate, output_rate) = get_pricing(provider, model);
    (input_tokens as f64 * input_rate / 1_000_000.0)
        + (output_tokens as f64 * output_rate / 1_000_000.0)
}

fn get_pricing(provider: &str, model: &str) -> (f64, f64) {
    match (
        provider.to_lowercase().as_str(),
        model.to_lowercase().as_str(),
    ) {
        // OpenAI
        ("openai", "gpt-4o") => (5.0, 15.0),
        ("openai", "gpt-4-turbo") => (10.0, 30.0),
        ("openai", "gpt-4") => (30.0, 60.0),
        ("openai", "gpt-3.5-turbo") => (0.5, 1.5),
        // Anthropic
        ("anthropic", "claude-3-5-sonnet") => (3.0, 15.0),
        ("anthropic", "claude-3-opus") => (15.0, 75.0),
        ("anthropic", "claude-3-haiku") => (0.25, 1.25),
        // Google
        ("google", "gemini-1.5-pro") => (3.5, 10.5),
        ("google", "gemini-1.5-flash") => (0.075, 0.3),
        // Default fallback
        _ => (5.0, 15.0),
    }
}

/// Estimate total cost for processing requests through the pipeline.
pub fn estimate_pipeline_cost(
    requests: usize,
    avg_request_tokens: usize,
    provider: &str,
    model: &str,
) -> f64 {
    // Each request: input tokens for the request, output tokens for the spec
    let input = requests * avg_request_tokens;
    let output = requests * 200; // Rough estimate for spec output
    estimate_cost(provider, model, input, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_for_spec() {
        let spec = serde_json::json!({
            "openapi": "3.0.0",
            "info": {"title": "Test API", "version": "1.0.0"},
            "paths": {}
        });
        let tokens = estimate_tokens_for_spec(&spec);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_cost_openai() {
        let cost = estimate_cost("openai", "gpt-4o", 1000, 500);
        assert!(cost > 0.0);
        // 1000 * $5/1M + 500 * $15/1M = $0.005 + $0.0075 = $0.0125
        assert!(cost < 0.02);
    }

    #[test]
    fn test_estimate_pipeline_cost() {
        let cost = estimate_pipeline_cost(100, 500, "openai", "gpt-4o");
        assert!(cost > 0.0);
    }
}
