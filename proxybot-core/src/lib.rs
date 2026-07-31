//! ProxyBot Core — MITM proxy engine library for Rust.
//!
//! This crate provides the pure-logic core of ProxyBot without any
//! Tauri, GUI, or desktop dependencies. It can be embedded in other
//! Rust projects that need MITM proxy capabilities.
//!
//! # Modules
//!
//! - [`types`] — Shared data types (InterceptedRequest, Rule, DnsEntry, etc.)
//! - [`config`] — Centralized configuration with env-var overrides
//! - [`app_classifier`] — Domain-based app identification (WeChat, Douyin, etc.)
//! - [`fingerprint`] — TLS ClientHello fingerprint types and the built-in signature library
//! - [`cert_manager`] — Root CA and per-host leaf certificate management
//! - [`rules_engine`] — Domain matching and priority-based rule evaluation
//! - [`proxy_engine`] — HTTP/HTTPS proxy engine (core logic)
//! - [`dns_state`] — DNS query tracking and correlation
//!
//! # Usage
//!
//! ```rust,no_run
//! use proxybot_core::{CertManager, RulesEngine};
//!
//! let cert_mgr = CertManager::new(None).unwrap();
//! let engine = RulesEngine::new();
//!
//! if let Some(action) = engine.match_host("api.example.com", None) {
//!     println!("Routing action: {action}");
//! }
//! ```

pub mod app_classifier;
pub mod body;
pub mod cert_manager;
pub mod config;
pub mod desktop_contract;
pub mod dns_state;
pub mod fingerprint;
pub mod proxy_engine;
pub mod rules_engine;
pub mod specgen;
pub mod tls_rules;
pub mod types;

// Re-export key types for convenience
pub use app_classifier::{
    classify, classify_host, classify_host_name, get_default_rules, load_app_rules, AppClassifier,
    AppMatchResult,
};
pub use cert_manager::CertManager;
pub use config::{dns_port, proxy_port, AppConfig};
pub use dns_state::DnsState;
pub use fingerprint::{
    default_fingerprint_set, get_default_signatures, glob_match, AppMatch, AppSignature,
    CustomAppRule, HelloInfo, MatchSource, RuleCondition, TlsFingerprint,
};
pub use proxy_engine::ProxyEngine;
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
    AppRule, BreakpointDecision, BreakpointRequest, BreakpointTarget, CaMetadata, DnsEntry,
    DnsUpstream, DnsUpstreamType, HostsEntry, InterceptedRequest, Rule, RuleAction, RuleEntry,
    RuleFile, RulePattern, WsFrame,
};
