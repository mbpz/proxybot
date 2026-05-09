pub mod plugin_trait;
pub use plugin_trait::{Plugin, PluginHooks, ConnectDecision, InterceptedResponse};

#[cfg(test)]
mod tests {
    use super::plugin_trait::{Plugin, ConnectDecision, InterceptedResponse, PluginHooks};

    #[test]
    fn test_plugin_trait_object_safe() {
        // Plugin must be Send + Sync for multi-threaded use
        fn assert_plugin<T: Plugin>() {}
        fn assert_static(_: &'static dyn Plugin) {}
    }

    #[test]
    fn test_plugin_hooks_present() {
        fn assert_impl_plugin<P: Plugin>() {}
        // All hooks should be optional (default noop)
    }
}