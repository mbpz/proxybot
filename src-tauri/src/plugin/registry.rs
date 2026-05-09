use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use super::plugin_trait::Plugin;

pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}

impl PluginRegistry {
    pub fn new() -> Self { Self { plugins: RwLock::new(HashMap::new()) } }

    pub fn register(&self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_string();
        self.plugins.write().unwrap().insert(name, Arc::from(plugin));
    }

    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.read().unwrap().keys().cloned().collect()
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.read().unwrap().get(name).map(|p| Arc::clone(p))
    }

    pub fn unregister(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.write().unwrap().remove(name)
    }
}