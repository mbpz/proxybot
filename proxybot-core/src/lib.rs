//! ProxyBot Core — MITM proxy engine library for Rust.
//!
//! This crate provides the pure-logic core of ProxyBot without any
//! Tauri, GUI, or desktop dependencies. It can be embedded in other
//! Rust projects that need MITM proxy capabilities.
//!
//! # Modules
//!
//! - [`types`] — Shared data types (InterceptedRequest, Rule, DnsEntry, etc.)
//! - [`config`] — Immutable process configuration and environment Adapter
//! - [`app_classifier`] — Domain-based app identification (WeChat, Douyin, etc.)
//! - [`fingerprint`] — TLS ClientHello fingerprint types and the built-in signature library
//! - [`cert_manager`] — Root CA and per-host leaf certificate management
//! - [`rules_engine`] — Domain matching and priority-based rule evaluation
//! - [`proxy_engine`] — HTTP/HTTPS proxy engine (core logic)
//! - [`application_identity`] — Application Attribution and DNS correlation
//!
//! # Usage
//!
//! ```rust,no_run
//! use proxybot_core::{AppConfig, CertManager, RulesEngine};
//!
//! let config = AppConfig::load().unwrap();
//! let cert_mgr = CertManager::new(config.ca_dir.clone()).unwrap();
//! let engine = RulesEngine::with_dir(config.rules_dir);
//!
//! if let Some(action) = engine.match_host("api.example.com", None) {
//!     println!("Routing action: {action}");
//! }
//! ```

pub mod app_classifier;
pub mod application_identity;
pub mod body;
pub mod cert_manager;
pub mod config;
pub mod desktop_contract;
pub mod fingerprint;
pub mod proxy_engine;
pub mod rules_engine;
pub mod specgen;
pub mod tls_rules;
pub mod types;

// Re-export key types for convenience
pub use app_classifier::{
    canonicalize_host, classify, classify_host, classify_host_name, get_default_rules,
    load_app_rules, load_app_rules_from, load_custom_app_rules, load_custom_app_rules_from,
    AppClassifier, AppMatchResult, ApplicationClassifier,
};
pub use application_identity::{
    AttributionEngine, AttributionInput, DEFAULT_DNS_CORRELATION_WINDOW_MS,
    DEFAULT_DNS_OBSERVATION_CAPACITY,
};
pub use cert_manager::CertManager;
pub use config::{
    AppConfig, ConfigError, EnvironmentSource, ProcessEnvironment, DEFAULT_CERT_SERVER_PORT,
    DEFAULT_DASHBOARD_PORT, DEFAULT_DNS_PORT, DEFAULT_PROXY_PORT,
};
pub use fingerprint::AppMatch as ApplicationAttribution;
pub use fingerprint::{
    default_fingerprint_set, get_default_signatures, glob_match, AppMatch, AppSignature,
    CustomAppRule, HelloInfo, MatchSource, RuleCondition, TlsFingerprint,
};
pub use proxy_engine::{
    CaptureEvent, MitmRuntime, NoOriginalDestination, NoopRuntimeHooks, OriginalDestination,
    ProxyEngine, RunningMitm, RuntimeConfig, RuntimeConnectDecision, RuntimeError,
    RuntimeHookDecision, RuntimeHooks, RuntimeRequest, RuntimeResponse, TrafficDirection,
    TrafficEffect, UpstreamTlsPolicy,
};
pub use rules_engine::{
    host_matches_domain, match_host_pattern, match_pattern, match_rule, HostPattern, RulesEngine,
    RulesError,
};
pub use specgen::{
    build_spec, build_spec_heuristic, AsyncApiChannel, AsyncApiDoc, AsyncApiExample,
    AsyncApiMessage, CoverageReport, OpenApiDoc, OpenApiInfo, OpenApiMediaType, OpenApiOperation,
    OpenApiParameter, OpenApiPathItem, OpenApiResponse, OpenApiSchema, OpenApiServer,
    ReplayFailure, ReplayReport, SpecConfig, SpecError, SpecOutput, SpecRequest, SpecResult,
    SpecSource, TrafficKind, TrafficRecord,
};
pub use tls_rules::{TlsAction, TlsRule, TlsRuleSet};
pub use types::{
    AppRule, BlocklistEntry, BreakpointDecision, BreakpointRequest, BreakpointTarget, CaMetadata,
    DnsEntry, DnsObservation, DnsUpstream, DnsUpstreamType, HostsEntry, InterceptedRequest, Rule,
    RuleAction, RuleEntry, RuleFile, RulePattern, WsFrame,
};
