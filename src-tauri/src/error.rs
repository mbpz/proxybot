//! Centralized error types for ProxyBot.
//!
//! All library errors use the `AppError` enum instead of `Result<T, String>`.
//! The binary layer (proxybot-tui.rs) continues to use `Result<T, String>`.

use thiserror::Error;

/// Central application error type.
/// Variants cover all major subsystems.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] DbError),

    #[error("certificate error: {0}")]
    Cert(#[from] CertError),

    #[error("proxy error: {0}")]
    Proxy(#[from] ProxyError),

    #[error("DNS error: {0}")]
    Dns(#[from] DnsError),

    #[error("rules error: {0}")]
    Rules(#[from] RulesError),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Database errors.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("connection failed: {0}")]
    Connection(String),

    #[error("query failed: {0}")]
    Query(String),

    #[error("migration failed: {0}")]
    Migration(String),
}

/// Certificate errors.
#[derive(Debug, Error)]
pub enum CertError {
    #[error("CA not found: {0}")]
    NotFound(String),

    #[error("generation failed: {0}")]
    Generation(String),

    #[error("export failed: {0}")]
    Export(String),

    #[error("invalid certificate: {0}")]
    Invalid(String),
}

/// Proxy errors.
#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("bind failed: {0}")]
    Bind(String),

    #[error("connection failed: {0}")]
    Connection(String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("startup failed: {0}")]
    Startup(String),
}

/// DNS errors.
#[derive(Debug, Error)]
pub enum DnsError {
    #[error("server error: {0}")]
    Server(String),

    #[error("upstream error: {0}")]
    Upstream(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("lookup failed: {0}")]
    Lookup(String),
}

/// Rules engine errors.
#[derive(Debug, Error)]
pub enum RulesError {
    #[error("load failed: {0}")]
    Load(String),

    #[error("parse failed: {0}")]
    Parse(String),

    #[error("file not found: {0}")]
    NotFound(String),

    #[error("save failed: {0}")]
    Save(String),
}

