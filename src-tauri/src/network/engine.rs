use std::sync::RwLock;
use std::collections::HashMap;
use crate::network::profile::NetworkProfile;
use crate::network::builtin_presets;

pub struct ConditionEffect {
    pub delay_ms: u64,
    pub drop: bool,
}

pub struct NetworkConditionEngine {
    profiles: RwLock<HashMap<String, NetworkProfile>>,
    active_profile: RwLock<Option<NetworkProfile>>,
}

impl NetworkConditionEngine {
    pub fn new() -> Self {
        let profiles: HashMap<String, NetworkProfile> = builtin_presets()
            .into_iter()
            .map(|p| (p.name.clone(), p))
            .collect();
        Self {
            profiles: RwLock::new(profiles),
            active_profile: RwLock::new(None),
        }
    }

    pub fn set_active(&self, name: &str) -> Result<(), String> {
        let profiles = self.profiles.read().unwrap();
        let profile = profiles.get(name)
            .ok_or_else(|| format!("Profile not found: {}", name))?;
        *self.active_profile.write().unwrap() = Some(profile.clone());
        Ok(())
    }

    pub fn disable(&self) {
        *self.active_profile.write().unwrap() = None;
    }

    pub fn get_active(&self) -> Option<NetworkProfile> {
        self.active_profile.read().unwrap().clone()
    }

    pub fn list_profiles(&self) -> Vec<NetworkProfile> {
        self.profiles.read().unwrap().values().cloned().collect()
    }

    pub fn add_profile(&self, profile: NetworkProfile) {
        self.profiles.write().unwrap().insert(profile.name.clone(), profile);
    }

    /// Compute condition effect for a read of N bytes
    pub fn apply(&self, read_size: usize) -> ConditionEffect {
        let profile = match self.active_profile.read().unwrap().as_ref() {
            Some(p) => p.clone(),
            None => return ConditionEffect { delay_ms: 0, drop: false },
        };

        // Packet loss: randomly drop
        let drop = if profile.packet_loss_pct > 0 {
            (rand::random::<u8>() % 100) < profile.packet_loss_pct
        } else {
            false
        };

        // Latency
        let latency_delay = profile.latency_ms;

        // Bandwidth cap: minimum time to transfer N bytes at capped rate
        let bandwidth_delay = if profile.bandwidth_kbps > 0 {
            // bits / (kbps * 1000) * 1000 = ms
            (read_size as u64 * 8 * 1000) / (profile.bandwidth_kbps * 1000)
        } else {
            0
        };

        let delay_ms = latency_delay.max(bandwidth_delay);

        ConditionEffect { delay_ms, drop }
    }
}

impl Default for NetworkConditionEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presets_exist() {
        let engine = NetworkConditionEngine::new();
        let profiles = engine.list_profiles();
        assert!(profiles.iter().any(|p| p.name == "3G"));
        assert!(profiles.iter().any(|p| p.name == "4G"));
        assert!(profiles.iter().any(|p| p.name == "2G"));
    }

    #[test]
    fn test_set_active() {
        let engine = NetworkConditionEngine::new();
        engine.set_active("3G").unwrap();
        let active = engine.get_active().unwrap();
        assert_eq!(active.name, "3G");
        assert_eq!(active.latency_ms, 300);
    }

    #[test]
    fn test_disable() {
        let engine = NetworkConditionEngine::new();
        engine.set_active("3G").unwrap();
        engine.disable();
        assert!(engine.get_active().is_none());
    }

    #[test]
    fn test_apply_no_profile() {
        let engine = NetworkConditionEngine::new();
        let effect = engine.apply(1024);
        assert_eq!(effect.delay_ms, 0);
        assert!(!effect.drop);
    }

    #[test]
    fn test_apply_latency() {
        let engine = NetworkConditionEngine::new();
        let mut profile = engine.profiles.read().unwrap().get("3G").unwrap().clone();
        profile.packet_loss_pct = 0;
        profile.bandwidth_kbps = 0;
        *engine.active_profile.write().unwrap() = Some(profile);

        let effect = engine.apply(1024);
        assert_eq!(effect.delay_ms, 300);
        assert!(!effect.drop);
    }

    #[test]
    fn test_apply_bandwidth_cap() {
        let engine = NetworkConditionEngine::new();
        // 8 kbps = 1 KB/s, so 1024 bytes should take ~1000ms
        let profile = NetworkProfile {
            name: "test".into(),
            latency_ms: 0,
            bandwidth_kbps: 8,
            packet_loss_pct: 0,
        };
        *engine.active_profile.write().unwrap() = Some(profile);

        let effect = engine.apply(1024);
        // 1024 bytes * 8 bits / 8000 bps = 1.024s ≈ 1024ms
        assert!(effect.delay_ms >= 900);
    }
}
