//! Core MITM proxy engine — transport-agnostic proxy logic.
//!
//! This module provides the core proxy engine struct and lifecycle.
//! The full async proxy loop lives in the Tauri crate (`src-tauri/src/proxy.rs`)
//! because it depends on Tauri's event system, IPC channels, and plugin registry.
//!
//! External consumers can use this struct as a handle for starting/stopping
//! a proxy instance managed by their own async runtime.

use std::sync::atomic::{AtomicBool, Ordering};

/// MITM proxy engine — controls proxy lifecycle and exposes runtime state.
pub struct ProxyEngine {
    running: AtomicBool,
}

impl ProxyEngine {
    /// Create a new ProxyEngine instance.
    pub fn new() -> Self {
        Self {
            running: AtomicBool::new(false),
        }
    }

    /// Returns true if the proxy is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Mark the proxy as started.
    pub fn mark_started(&self) {
        self.running.store(true, Ordering::SeqCst);
    }

    /// Mark the proxy as stopped.
    pub fn mark_stopped(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Default for ProxyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Build request ID from timestamp and counter.
pub fn generate_request_id(counter: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("req-{}-{}", nanos, counter)
}

/// Build ISO-8601 timestamp string.
pub fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| format!("{}.{:03}", dur.as_secs(), dur.subsec_millis()))
        .unwrap_or_else(|_| "0.000".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_engine_lifecycle() {
        let engine = ProxyEngine::new();
        assert!(!engine.is_running());
        engine.mark_started();
        assert!(engine.is_running());
        engine.mark_stopped();
        assert!(!engine.is_running());
    }

    #[test]
    fn test_generate_request_id() {
        let id = generate_request_id(1);
        assert!(id.starts_with("req-"));
    }

    #[test]
    fn test_timestamp_now() {
        let ts = timestamp_now();
        assert!(!ts.is_empty());
        assert!(ts.contains('.'));
    }
}
