use serde_json::Value;

pub struct AiDecoder;

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub model: Option<String>,
    pub estimated: bool,
}

impl AiDecoder {
    /// Check if a request body looks like an AI API call
    pub fn is_ai_request(body: &str) -> bool {
        if let Ok(val) = serde_json::from_str::<Value>(body) {
            val.get("model").is_some()
                && (val.get("messages").is_some() || val.get("prompt").is_some())
        } else {
            false
        }
    }

    /// Extract model name from request body
    pub fn extract_model(body: &str) -> Option<String> {
        serde_json::from_str::<Value>(body)
            .ok()
            .and_then(|v| v.get("model")?.as_str().map(String::from))
    }

    /// Extract token usage from response body (OpenAI/Anthropic/Google formats)
    pub fn extract_usage(body: &str) -> Option<TokenUsage> {
        let val: Value = serde_json::from_str(body).ok()?;

        // OpenAI format: { "usage": { "prompt_tokens": N, "completion_tokens": M, "total_tokens": T } }
        if let Some(usage) = val.get("usage") {
            let prompt = usage
                .get("prompt_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let completion = usage
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let total = usage
                .get("total_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            // Check for Anthropic format: { "usage": { "input_tokens": N, "output_tokens": M } }
            let input = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            let output = usage
                .get("output_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            if prompt > 0 || completion > 0 || total > 0 {
                return Some(TokenUsage {
                    prompt_tokens: if prompt > 0 {
                        prompt
                    } else {
                        input
                    },
                    completion_tokens: if completion > 0 {
                        completion
                    } else {
                        output
                    },
                    total_tokens: if total > 0 {
                        total
                    } else {
                        prompt + completion
                    },
                    model: val.get("model").and_then(|v| v.as_str()).map(String::from),
                    estimated: false,
                });
            }

            if input > 0 || output > 0 {
                return Some(TokenUsage {
                    prompt_tokens: input,
                    completion_tokens: output,
                    total_tokens: input + output,
                    model: val.get("model").and_then(|v| v.as_str()).map(String::from),
                    estimated: false,
                });
            }
        }

        // Google format: { "usageMetadata": { "promptTokenCount": N, "candidatesTokenCount": M, "totalTokenCount": T } }
        if let Some(usage) = val.get("usageMetadata") {
            let prompt = usage
                .get("promptTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let completion = usage
                .get("candidatesTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let total = usage
                .get("totalTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if prompt > 0 || completion > 0 || total > 0 {
                return Some(TokenUsage {
                    prompt_tokens: prompt,
                    completion_tokens: completion,
                    total_tokens: if total > 0 {
                        total
                    } else {
                        prompt + completion
                    },
                    model: val.get("model").and_then(|v| v.as_str()).map(String::from),
                    estimated: false,
                });
            }
        }

        None
    }

    /// Estimate tokens as fallback (4 chars ~ 1 token)
    pub fn estimate_tokens(text: &str) -> u64 {
        (text.chars().count() as u64).div_ceil(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ai_request_openai() {
        let body = r#"{"model": "gpt-4o", "messages": [{"role": "user", "content": "hello"}]}"#;
        assert!(AiDecoder::is_ai_request(body));
    }

    #[test]
    fn test_is_ai_request_not_ai() {
        let body = r#"{"foo": "bar"}"#;
        assert!(!AiDecoder::is_ai_request(body));
    }

    #[test]
    fn test_extract_model() {
        let body = r#"{"model": "gpt-4o", "messages": []}"#;
        assert_eq!(AiDecoder::extract_model(body), Some("gpt-4o".to_string()));
    }

    #[test]
    fn test_extract_usage_openai() {
        let body =
            r#"{"model": "gpt-4o", "usage": {"prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150}}"#;
        let usage = AiDecoder::extract_usage(body).unwrap();
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
        assert_eq!(usage.model, Some("gpt-4o".to_string()));
        assert!(!usage.estimated);
    }

    #[test]
    fn test_extract_usage_anthropic() {
        let body = r#"{"model": "claude-3-5-sonnet-20241022", "usage": {"input_tokens": 200, "output_tokens": 80}}"#;
        let usage = AiDecoder::extract_usage(body).unwrap();
        assert_eq!(usage.prompt_tokens, 200);
        assert_eq!(usage.completion_tokens, 80);
        assert_eq!(usage.total_tokens, 280);
        assert!(!usage.estimated);
    }

    #[test]
    fn test_extract_usage_google() {
        let body = r#"{"usageMetadata": {"promptTokenCount": 300, "candidatesTokenCount": 120, "totalTokenCount": 420}}"#;
        let usage = AiDecoder::extract_usage(body).unwrap();
        assert_eq!(usage.prompt_tokens, 300);
        assert_eq!(usage.completion_tokens, 120);
        assert_eq!(usage.total_tokens, 420);
        assert!(!usage.estimated);
    }

    #[test]
    fn test_estimate_tokens() {
        let text = "Hello, world! This is a test.";
        let tokens = AiDecoder::estimate_tokens(text);
        assert!(tokens > 0);
        assert!(tokens <= text.chars().count() as u64);
    }
}
