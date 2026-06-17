//! DeepSeek V3 client with JSON-schema constrained output.

use crate::specgen::error::SpecError;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

pub const DEEPSEEK_URL: &str = "https://api.deepseek.com/v1/chat/completions";
pub const DEEPSEEK_MODEL: &str = "deepseek-chat";

#[derive(Debug, Clone)]
pub struct DeepSeekClient {
    pub api_key: String,
    pub endpoint: String,
    pub http: Client,
}

impl DeepSeekClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            endpoint: DEEPSEEK_URL.to_string(),
            http: Client::new(),
        }
    }

    /// Call DeepSeek with a JSON schema constraint, return parsed JSON.
    /// Retries up to `max_retries` on transport / HTTP errors.
    pub async fn call_with_schema(
        &self,
        system_prompt: &str,
        user_payload: &str,
        json_schema: &Value,
        max_retries: u32,
    ) -> Result<Value, SpecError> {
        let body = serde_json::json!({
            "model": DEEPSEEK_MODEL,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user",   "content": user_payload }
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": { "strict": true, "schema": json_schema }
            }
        });

        let mut last_err: Option<SpecError> = None;
        for attempt in 0..=max_retries {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(500 * (1 << attempt))).await;
            }
            match self.try_once(&body).await {
                Ok(v) => return Ok(v),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or(SpecError::LlmUnavailable("unknown".into())))
    }

    async fn try_once(&self, body: &Value) -> Result<Value, SpecError> {
        let resp = self
            .http
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| SpecError::LlmUnavailable(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(SpecError::LlmUnavailable(format!("HTTP {status}: {text}")));
        }
        let parsed: ChatResponse = resp
            .json()
            .await
            .map_err(|e| SpecError::LlmUnavailable(e.to_string()))?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| SpecError::LlmUnavailable("no choices".into()))?;
        serde_json::from_str(&content).map_err(|e| SpecError::LlmUnavailable(e.to_string()))
    }
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn parses_successful_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{
                    "message": { "content": "{\"paths\":{}}" }
                }]
            })))
            .mount(&server)
            .await;

        let client = DeepSeekClient {
            api_key: "sk-test".into(),
            endpoint: format!("{}/v1/chat/completions", server.uri()),
            http: Client::new(),
        };
        let result = client
            .call_with_schema("sys", "user", &json!({"type": "object"}), 0)
            .await
            .unwrap();
        assert_eq!(result, json!({"paths": {}}));
    }

    #[tokio::test]
    async fn retries_on_500_then_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "choices": [{ "message": { "content": "{\"ok\":true}" } }]
            })))
            .mount(&server)
            .await;

        let client = DeepSeekClient {
            api_key: "sk-test".into(),
            endpoint: format!("{}/v1/chat/completions", server.uri()),
            http: Client::new(),
        };
        let result = client
            .call_with_schema("s", "u", &json!({"type": "object"}), 2)
            .await
            .unwrap();
        assert_eq!(result, json!({"ok": true}));
    }
}
