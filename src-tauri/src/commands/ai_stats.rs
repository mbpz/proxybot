use std::sync::Arc;

use crate::db::DbState;

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
        "gpt-4o" => ModelPricing {
            input_per_m: 2.50,
            output_per_m: 10.00,
        },
        "gpt-4o-mini" => ModelPricing {
            input_per_m: 0.15,
            output_per_m: 0.60,
        },
        "gpt-4-turbo" => ModelPricing {
            input_per_m: 10.00,
            output_per_m: 30.00,
        },
        "gpt-3.5-turbo" => ModelPricing {
            input_per_m: 0.50,
            output_per_m: 1.50,
        },

        // Anthropic
        "claude-3-5-sonnet" => ModelPricing {
            input_per_m: 3.00,
            output_per_m: 15.00,
        },
        "claude-3-5-haiku" => ModelPricing {
            input_per_m: 0.80,
            output_per_m: 4.00,
        },
        "claude-3-opus" => ModelPricing {
            input_per_m: 15.00,
            output_per_m: 75.00,
        },
        "claude-3-sonnet" => ModelPricing {
            input_per_m: 3.00,
            output_per_m: 15.00,
        },
        "claude-3-haiku" => ModelPricing {
            input_per_m: 0.25,
            output_per_m: 1.25,
        },

        // Google
        "gemini-1.5-pro" => ModelPricing {
            input_per_m: 1.25,
            output_per_m: 5.00,
        },
        "gemini-1.5-flash" => ModelPricing {
            input_per_m: 0.075,
            output_per_m: 0.30,
        },

        // Groq
        "llama-3.1-70b" => ModelPricing {
            input_per_m: 0.65,
            output_per_m: 2.75,
        },
        "llama-3.1-8b" => ModelPricing {
            input_per_m: 0.20,
            output_per_m: 0.80,
        },

        // Cohere
        "command-r-plus" => ModelPricing {
            input_per_m: 2.50,
            output_per_m: 12.50,
        },
        "command-r" => ModelPricing {
            input_per_m: 0.50,
            output_per_m: 1.50,
        },

        // Default
        _ => ModelPricing {
            input_per_m: 1.00,
            output_per_m: 2.00,
        },
    }
}

/// Estimate API cost for a model with given token counts
pub fn estimate_api_cost(model: &str, input_tokens: usize, output_tokens: usize) -> f64 {
    let pricing = get_model_pricing(model);
    (input_tokens as f64 / 1_000_000.0 * pricing.input_per_m)
        + (output_tokens as f64 / 1_000_000.0 * pricing.output_per_m)
}

/// Get context window sizes as a static map (shared with Tauri frontend).
pub fn get_context_windows_map() -> std::collections::HashMap<&'static str, u64> {
    let mut map = std::collections::HashMap::new();
    map.insert("gpt-4o", 128_000);
    map.insert("gpt-4o-mini", 128_000);
    map.insert("gpt-4-turbo", 128_000);
    map.insert("gpt-3.5-turbo", 16_385);
    map.insert("claude-3-5-sonnet", 200_000);
    map.insert("claude-3-opus", 200_000);
    map.insert("claude-3-sonnet", 200_000);
    map.insert("claude-3-haiku", 200_000);
    map.insert("gemini-1.5-pro", 2_097_152);
    map.insert("gemini-1.5-flash", 1_048_576);
    map.insert("llama-3.1-70b", 128_000);
    map.insert("llama-3.1-8b", 128_000);
    map.insert("command-r-plus", 128_000);
    map.insert("command-r", 128_000);
    map
}

/// Get aggregated AI token usage stats from the database.
#[tauri::command]
pub fn get_ai_stats(state: tauri::State<'_, Arc<DbState>>) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT provider, model, SUM(total_tokens) as total, SUM(cost_usd) as cost, COUNT(*) as requests FROM ai_token_usage GROUP BY provider, model",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "provider": row.get::<_, String>(0)?,
                "model": row.get::<_, String>(1)?,
                "total_tokens": row.get::<_, i64>(2)?,
                "cost_usd": row.get::<_, f64>(3)?,
                "requests": row.get::<_, i64>(4)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(serde_json::json!({ "stats": rows }))
}

/// Get context window sizes for display in the frontend.
#[tauri::command]
pub fn get_ai_context_windows() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "gpt-4o": 128_000,
        "gpt-4o-mini": 128_000,
        "gpt-4-turbo": 128_000,
        "gpt-3.5-turbo": 16_385,
        "claude-3-5-sonnet": 200_000,
        "claude-3-opus": 200_000,
        "claude-3-sonnet": 200_000,
        "claude-3-haiku": 200_000,
        "gemini-1.5-pro": 2_097_152,
        "gemini-1.5-flash": 1_048_576,
        "llama-3.1-70b": 128_000,
        "llama-3.1-8b": 128_000,
        "command-r-plus": 128_000,
        "command-r": 128_000,
    }))
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

    #[test]
    fn test_get_model_pricing_known() {
        let p = get_model_pricing("gpt-4o");
        assert_eq!(p.input_per_m, 2.50);
        assert_eq!(p.output_per_m, 10.00);
    }

    #[test]
    fn test_context_windows_api() {
        let windows = get_context_windows_map();
        assert_eq!(windows.get("gpt-4o"), Some(&128_000));
        assert_eq!(windows.get("claude-3-5-sonnet"), Some(&200_000));
        assert_eq!(windows.get("gemini-1.5-pro"), Some(&2_097_152));
    }
}
