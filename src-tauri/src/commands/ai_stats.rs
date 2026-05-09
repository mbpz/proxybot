/// Estimate token count using simple character ratio
/// AI APIs typically use ~4 chars/token for English text
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// AI model pricing per 1M tokens (approximate, Q4 2025)
/// Input and output prices differ per model
pub struct ModelPricing {
    pub input_per_m: f64,
    pub output_per_m: f64,
}

pub fn get_model_pricing(model: &str) -> ModelPricing {
    match model {
        // OpenAI
        "gpt-4o" => ModelPricing { input_per_m: 2.50, output_per_m: 10.00 },
        "gpt-4o-mini" => ModelPricing { input_per_m: 0.15, output_per_m: 0.60 },
        "gpt-4-turbo" => ModelPricing { input_per_m: 10.00, output_per_m: 30.00 },
        "gpt-3.5-turbo" => ModelPricing { input_per_m: 0.50, output_per_m: 1.50 },

        // Anthropic
        "claude-3-5-sonnet" => ModelPricing { input_per_m: 3.00, output_per_m: 15.00 },
        "claude-3-5-haiku" => ModelPricing { input_per_m: 0.80, output_per_m: 4.00 },
        "claude-3-opus" => ModelPricing { input_per_m: 15.00, output_per_m: 75.00 },
        "claude-3-sonnet" => ModelPricing { input_per_m: 3.00, output_per_m: 15.00 },
        "claude-3-haiku" => ModelPricing { input_per_m: 0.25, output_per_m: 1.25 },

        // Google
        "gemini-1.5-pro" => ModelPricing { input_per_m: 1.25, output_per_m: 5.00 },
        "gemini-1.5-flash" => ModelPricing { input_per_m: 0.075, output_per_m: 0.30 },

        // Groq
        "llama-3.1-70b" => ModelPricing { input_per_m: 0.65, output_per_m: 2.75 },
        "llama-3.1-8b" => ModelPricing { input_per_m: 0.20, output_per_m: 0.80 },

        // Cohere
        "command-r-plus" => ModelPricing { input_per_m: 2.50, output_per_m: 12.50 },
        "command-r" => ModelPricing { input_per_m: 0.50, output_per_m: 1.50 },

        // Default
        _ => ModelPricing { input_per_m: 1.00, output_per_m: 2.00 },
    }
}

/// Estimate API cost for a model with given token counts
pub fn estimate_api_cost(model: &str, input_tokens: usize, output_tokens: usize) -> f64 {
    let pricing = get_model_pricing(model);
    (input_tokens as f64 / 1_000_000.0 * pricing.input_per_m) +
    (output_tokens as f64 / 1_000_000.0 * pricing.output_per_m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        let text = "Hello, world!";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
        assert!(tokens <= text.chars().count());
    }

    #[test]
    fn test_estimate_api_cost_gpt4o() {
        let cost = estimate_api_cost("gpt-4o", 1000, 500);
        // 1000/1M * 2.50 + 500/1M * 10.00 = 0.0025 + 0.005 = 0.0075
        assert!((cost - 0.0075).abs() < 0.001);
    }

    #[test]
    fn test_estimate_api_cost_unknown_model() {
        let cost = estimate_api_cost("unknown-model", 1000, 1000);
        assert!(cost > 0.0);
    }
}
