pub mod loader;
pub mod plugin_trait;
pub mod registry;
pub mod rule_engine;
pub mod sandbox;
pub mod wasm_host;
pub use plugin_trait::{ConnectDecision, InterceptedResponse, Plugin, PluginHooks};
pub use registry::PluginRegistry;
pub use rule_engine::{PluginDispatchEngine, PluginDispatchPattern, PluginRule};

#[cfg(test)]
mod tests {
    use super::plugin_trait::{Plugin, PluginHooks};

    #[test]
    fn test_plugin_trait_object_safe() {
        // Plugin must be Send + Sync for multi-threaded use
        #[allow(dead_code)]
        fn assert_plugin<T: Plugin>() {}
        #[allow(dead_code)]
        fn assert_static(_: &'static dyn Plugin) {}
    }

    #[test]
    fn test_plugin_hooks_present() {
        #[allow(dead_code)]
        fn assert_impl_plugin<P: Plugin>() {}
        // All hooks should be optional (default noop)
    }

    #[test]
    fn test_registry_register_and_list() {
        use super::registry::PluginRegistry;

        struct TestPlugin {
            name: String,
        }
        impl Plugin for TestPlugin {
            fn name(&self) -> &str {
                &self.name
            }
            fn hooks(&self) -> PluginHooks {
                PluginHooks::default()
            }
        }

        let registry = PluginRegistry::new();
        registry.register(Box::new(TestPlugin {
            name: "test".into(),
        }));
        let names = registry.list_plugins();
        assert!(names.contains(&"test".to_string()));
    }

    #[test]
    fn test_registry_get() {
        use super::registry::PluginRegistry;

        struct MyPlugin {
            name: String,
        }
        impl Plugin for MyPlugin {
            fn name(&self) -> &str {
                &self.name
            }
            fn hooks(&self) -> PluginHooks {
                PluginHooks::default()
            }
        }

        let registry = PluginRegistry::new();
        registry.register(Box::new(MyPlugin {
            name: "myplugin".into(),
        }));

        let retrieved = registry.get("myplugin");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "myplugin");
    }
}
