use super::plugin_trait::Plugin;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_string();
        self.plugins
            .write()
            .unwrap()
            .insert(name, Arc::from(plugin));
    }

    pub fn list_plugins(&self) -> Vec<String> {
        let mut names: Vec<_> = self.plugins.read().unwrap().keys().cloned().collect();
        names.sort();
        names
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.read().unwrap().get(name).map(Arc::clone)
    }

    pub fn unregister(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.write().unwrap().remove(name)
    }
}
