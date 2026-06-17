//! OpenAPI/AsyncAPI spec generation from captured traffic.
//!
//! See `docs/superpowers/specs/2026-06-16-openapi-asyncapi-generation-design.md`
//! for the design.

pub mod config;
pub mod coverage;
pub mod error;
pub mod extract;
pub mod llm;
pub mod render;
pub mod replay;
pub mod validate;

pub use config::SpecConfig;
pub use error::SpecError;
