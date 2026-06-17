use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpecError {
    #[error("session is empty")]
    EmptySession,

    #[error("DeepSeek call failed: {0}")]
    LlmUnavailable(String),

    #[error("LLM output failed schema validation after {0} retries")]
    SchemaValidationFailed(u32),

    #[error("render failed: {0}")]
    RenderFailed(String),

    #[error("replay failed: {0}")]
    ReplayFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}
