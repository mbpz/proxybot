//! User-tunable knobs for `build_spec`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecConfig {
    pub deepseek_api_key: Option<String>,
    pub max_traffic_records: usize,
    pub max_retry: u32,
    pub enable_replay_validation: bool,
    pub mock_port: Option<u16>,
}

impl Default for SpecConfig {
    fn default() -> Self {
        Self {
            deepseek_api_key: None,
            max_traffic_records: 50,
            max_retry: 2,
            enable_replay_validation: true,
            mock_port: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_match_spec() {
        let c = SpecConfig::default();
        assert_eq!(c.max_traffic_records, 50);
        assert_eq!(c.max_retry, 2);
        assert!(c.enable_replay_validation);
        assert!(c.deepseek_api_key.is_none());
        assert!(c.mock_port.is_none());
    }

    #[test]
    fn roundtrips_through_yaml() {
        let c = SpecConfig {
            deepseek_api_key: Some("sk-abc".into()),
            max_traffic_records: 100,
            max_retry: 3,
            enable_replay_validation: false,
            mock_port: Some(19999),
        };
        let yaml = serde_yaml::to_string(&c).unwrap();
        let back: SpecConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back.deepseek_api_key, c.deepseek_api_key);
        assert_eq!(back.max_traffic_records, 100);
        assert_eq!(back.mock_port, Some(19999));
    }
}
