//! Global application state shared with Tauri commands.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use proxybot_core::SpecConfig;

/// Wrapper newtype so `AppState` can live in `tauri::State` even though
/// the inner config is a plain data type. Following the pattern used by
/// `NetworkConditionsState` in `commands/network_conditions.rs`.
pub struct AppState {
    /// User-tunable spec generation knobs (API key, retry counts,
    /// replay toggles, mock port). Wrapped in `Mutex` so commands can
    /// update it at runtime via `update_specgen_config`.
    pub specgen_config: Arc<Mutex<SpecConfig>>,
}

impl AppState {
    /// Default-initialised state. The specgen config starts at
    /// `SpecConfig::default()`; callers can mutate the LLM key and
    /// other knobs at runtime via the `update_specgen_config` command.
    pub fn new() -> Self {
        Self {
            specgen_config: Arc::new(Mutex::new(SpecConfig::default())),
        }
    }

    /// Snapshot the current specgen config. Used by commands that
    /// only need read access (build_spec, run_replay_validation).
    pub fn specgen_config_snapshot(&self) -> SpecConfig {
        self.specgen_config
            .lock()
            .expect("specgen_config mutex poisoned")
            .clone()
    }

    /// Replace the specgen config wholesale.
    pub fn set_specgen_config(&self, new_cfg: SpecConfig) {
        let mut guard = self
            .specgen_config
            .lock()
            .expect("specgen_config mutex poisoned");
        *guard = new_cfg;
    }

    /// Directory where generated spec JSON files are persisted. Each
    /// session gets its own file: `<specs_dir>/<session_id>.json`.
    pub fn specs_dir(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".proxybot").join("specs")
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_specgen_config_matches_core_default() {
        let s = AppState::new();
        let cfg = s.specgen_config_snapshot();
        assert_eq!(cfg.max_traffic_records, 50);
        assert_eq!(cfg.max_retry, 2);
        assert!(cfg.enable_replay_validation);
        assert!(cfg.deepseek_api_key.is_none());
        assert!(cfg.mock_port.is_none());
    }

    #[test]
    fn set_specgen_config_updates_snapshot() {
        let s = AppState::new();
        let new_cfg = SpecConfig {
            deepseek_api_key: Some("sk-test".into()),
            max_traffic_records: 100,
            max_retry: 5,
            enable_replay_validation: false,
            mock_port: Some(19999),
        };
        s.set_specgen_config(new_cfg.clone());
        let snap = s.specgen_config_snapshot();
        assert_eq!(snap.deepseek_api_key.as_deref(), Some("sk-test"));
        assert_eq!(snap.max_traffic_records, 100);
        assert_eq!(snap.mock_port, Some(19999));
    }

    #[test]
    fn specs_dir_ends_with_specs() {
        let s = AppState::new();
        let p = s.specs_dir();
        assert!(p.ends_with("specs"));
    }
}
