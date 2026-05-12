use std::sync::Arc;

use crate::ai::decoder::{AiDecoder, TokenUsage};
use crate::commands::ai_stats;
use crate::db::DbState;
use crate::proxy::InterceptedRequest;

/// Placeholder for future AI session tracking (e.g., multi-turn chat sessions).
#[derive(Debug, Clone)]
pub struct AiSession;

pub struct AiTracker {
    db: Arc<DbState>,
}

// Context window sizes (tokens)
const CONTEXT_WINDOWS: &[(&str, u64)] = &[
    ("gpt-4o", 128_000),
    ("gpt-4o-mini", 128_000),
    ("gpt-4-turbo", 128_000),
    ("gpt-3.5-turbo", 16_385),
    ("claude-3-5-sonnet", 200_000),
    ("claude-3-opus", 200_000),
    ("claude-3-sonnet", 200_000),
    ("claude-3-haiku", 200_000),
    ("gemini-1.5-pro", 2_097_152),
    ("gemini-1.5-flash", 1_048_576),
    ("llama-3.1-70b", 128_000),
    ("llama-3.1-8b", 128_000),
    ("command-r-plus", 128_000),
    ("command-r", 128_000),
];

impl AiTracker {
    pub fn new(db: Arc<DbState>) -> Self {
        Self { db }
    }

    pub fn context_window(model: &str) -> Option<u64> {
        CONTEXT_WINDOWS
            .iter()
            .find(|(name, _)| model.contains(name))
            .map(|(_, size)| *size)
    }

    /// Process a request: check if it's AI traffic, extract tokens, store, warn
    pub fn process_request(&self, req: &InterceptedRequest) {
        let provider = match &req.app_name {
            Some(name) if is_ai_provider(name) => name,
            _ => return,
        };

        // Try to extract model from request body
        let model = req.req_body.as_ref().and_then(|b| AiDecoder::extract_model(b));

        // Try to extract actual token usage from response body
        let usage = req
            .resp_body
            .as_ref()
            .and_then(|b| AiDecoder::extract_usage(b))
            .or_else(|| {
                // Fallback: estimate from request + response body sizes
                let prompt_est = req
                    .req_body
                    .as_ref()
                    .map(|b| AiDecoder::estimate_tokens(b))
                    .unwrap_or(0);
                let completion_est = req
                    .resp_body
                    .as_ref()
                    .map(|b| AiDecoder::estimate_tokens(b))
                    .unwrap_or(0);
                Some(TokenUsage {
                    prompt_tokens: prompt_est,
                    completion_tokens: completion_est,
                    total_tokens: prompt_est + completion_est,
                    model: model.clone(),
                    estimated: true,
                })
            });

        if let Some(ref usage) = usage {
            // Calculate cost
            let model_name = usage.model.as_deref().unwrap_or("unknown");
            let cost = ai_stats::estimate_api_cost(
                model_name,
                usage.prompt_tokens as usize,
                usage.completion_tokens as usize,
            );

            // Check context window
            let context_window = model.as_ref().and_then(|m| Self::context_window(m));

            let usage_pct =
                context_window.map(|cw| (usage.total_tokens as f64 / cw as f64) * 100.0);

            // Warn if >80% of context window
            if let Some(pct) = usage_pct {
                if pct > 80.0 {
                    log::warn!(
                        "[AI] {} model {} at {:.0}% context window ({}/{})",
                        provider,
                        model_name,
                        pct,
                        usage.total_tokens,
                        context_window.unwrap_or(0)
                    );
                }
            }

            // Store to DB
            if let Err(e) = self.db.conn.lock().unwrap().execute(
                "INSERT INTO ai_token_usage (timestamp, provider, model, request_id, prompt_tokens, completion_tokens, total_tokens, max_tokens, context_window, estimated, cost_usd) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    chrono::Utc::now().to_rfc3339(),
                    provider,
                    model_name,
                    req.id,
                    usage.prompt_tokens as i64,
                    usage.completion_tokens as i64,
                    usage.total_tokens as i64,
                    0i64,
                    context_window.unwrap_or(0) as i64,
                    usage.estimated,
                    cost,
                ],
            ) {
                log::error!("Failed to record AI token usage: {}", e);
            }
        }
    }
}

fn is_ai_provider(name: &str) -> bool {
    matches!(
        name,
        "OpenAI" | "Anthropic" | "Azure-OpenAI" | "Google-AI" | "Cohere" | "Groq"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_window_lookup() {
        assert_eq!(
            AiTracker::context_window("gpt-4o-2024-08-06"),
            Some(128_000)
        );
        assert_eq!(
            AiTracker::context_window("claude-3-5-sonnet-20241022"),
            Some(200_000)
        );
        assert_eq!(AiTracker::context_window("unknown-model"), None);
    }

    #[test]
    fn test_is_ai_provider() {
        assert!(is_ai_provider("OpenAI"));
        assert!(is_ai_provider("Anthropic"));
        assert!(!is_ai_provider("WeChat"));
    }
}
