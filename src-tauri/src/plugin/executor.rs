use std::sync::Arc;
use std::time::Duration;

use super::plugin_trait::InterceptedResponse;
use super::registry::PluginRegistry;
use super::rule_engine::PluginRule;
use crate::proxy::InterceptedRequest;

const HOOK_TIMEOUT: Duration = Duration::from_secs(5);

/// Executes plugin hooks for rule-matched requests/responses
pub struct HookExecutor;

impl HookExecutor {
    /// Execute on_request hooks for all matching rules (first match only)
    pub fn execute_request_sync(
        plugins: &Arc<PluginRegistry>,
        rules: &[PluginRule],
        request: &mut InterceptedRequest,
    ) {
        let matched = rules
            .iter()
            .filter(|r| r.enabled && r.pattern.matches(request))
            .min_by_key(|r| r.priority);

        if let Some(rule) = matched {
            if let Some(plugin) = plugins.get(&rule.plugin_name) {
                let hooks = plugin.hooks();
                if let Some(ref hook) = hooks.on_request {
                    hook(request);
                }
            }
        }
    }

    /// Execute on_request hooks for all matching rules
    pub fn execute_request_sync_all(
        plugins: &Arc<PluginRegistry>,
        rules: &[PluginRule],
        request: &mut InterceptedRequest,
    ) {
        let mut matched: Vec<_> = rules
            .iter()
            .filter(|r| r.enabled && r.pattern.matches(request))
            .collect();
        matched.sort_by_key(|r| r.priority);

        for rule in matched {
            if let Some(plugin) = plugins.get(&rule.plugin_name) {
                let hooks = plugin.hooks();
                if let Some(ref hook) = hooks.on_request {
                    hook(request);
                }
            }
        }
    }

    /// Execute on_response hooks for all matching rules
    pub fn execute_response_sync_all(
        plugins: &Arc<PluginRegistry>,
        rules: &[PluginRule],
        response: &mut InterceptedResponse,
    ) {
        let mut matched: Vec<_> = rules.iter().filter(|r| r.enabled).collect();
        matched.sort_by_key(|r| r.priority);

        for rule in matched {
            if let Some(plugin) = plugins.get(&rule.plugin_name) {
                let hooks = plugin.hooks();
                if let Some(ref hook) = hooks.on_response {
                    hook(response);
                }
            }
        }
    }

    /// Run on_request hooks for already-matched rules (no re-filtering).
    /// Rules should be pre-sorted by priority. Only runs enabled rules.
    pub fn run_request_hooks(
        plugins: &Arc<PluginRegistry>,
        matched_rules: &[PluginRule],
        request: &mut InterceptedRequest,
    ) {
        for rule in matched_rules {
            if !rule.enabled {
                continue;
            }
            if let Some(plugin) = plugins.get(&rule.plugin_name) {
                let hooks = plugin.hooks();
                if let Some(ref hook) = hooks.on_request {
                    hook(request);
                }
            }
        }
    }

    /// Run on_response hooks for already-matched rules (no re-filtering).
    /// Rules should be pre-sorted by priority. Only runs enabled rules.
    pub fn run_response_hooks(
        plugins: &Arc<PluginRegistry>,
        matched_rules: &[PluginRule],
        response: &mut InterceptedResponse,
    ) {
        for rule in matched_rules {
            if !rule.enabled {
                continue;
            }
            if let Some(plugin) = plugins.get(&rule.plugin_name) {
                let hooks = plugin.hooks();
                if let Some(ref hook) = hooks.on_response {
                    hook(response);
                }
            }
        }
    }

    /// Execute async on_request hooks for already-matched rules with timeout per hook.
    pub async fn run_request_hooks_async(
        plugins: Arc<PluginRegistry>,
        matched_rules: Vec<PluginRule>,
        mut request: InterceptedRequest,
    ) -> InterceptedRequest {
        for rule in matched_rules {
            if !rule.enabled {
                continue;
            }
            if let Some(plugin) = plugins.get(&rule.plugin_name) {
                let hooks = plugin.hooks();
                if let Some(ref hook) = hooks.on_request_async {
                    let _ = tokio::time::timeout(HOOK_TIMEOUT, hook(&mut request)).await;
                }
            }
        }
        request
    }
}

#[cfg(test)]
mod tests {
    use super::super::plugin_trait::{Plugin, PluginHooks};
    use super::super::registry::PluginRegistry;
    use super::super::rule_engine::{PluginRule, RulePattern};
    use super::*;
    use crate::proxy::InterceptedRequest;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestPlugin {
        name: String,
        call_count: Arc<AtomicUsize>,
    }
    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }
        fn hooks(&self) -> PluginHooks {
            let count = self.call_count.clone();
            PluginHooks {
                on_request: Some(Box::new(move |_| {
                    count.fetch_add(1, Ordering::SeqCst);
                })),
                ..Default::default()
            }
        }
    }

    fn make_request(host: &str) -> InterceptedRequest {
        InterceptedRequest {
            host: host.into(),
            method: "GET".into(),
            scheme: "https".into(),
            path: "/".into(),
            ..Default::default()
        }
    }

    #[test]
    fn test_executor_single_match() {
        let registry = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        registry.register(Box::new(TestPlugin {
            name: "test-plugin".into(),
            call_count: counter.clone(),
        }));

        let rule = PluginRule {
            id: 1,
            name: "Test Rule".into(),
            pattern: RulePattern::DomainSuffix("example.com".into()),
            plugin_name: "test-plugin".into(),
            priority: 100,
            enabled: true,
        };

        let mut request = make_request("api.example.com");
        HookExecutor::execute_request_sync(&Arc::new(registry), &[rule], &mut request);

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_executor_disabled_rule_skipped() {
        let registry = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        registry.register(Box::new(TestPlugin {
            name: "test-plugin".into(),
            call_count: counter.clone(),
        }));

        let rule = PluginRule {
            id: 1,
            name: "Test Rule".into(),
            pattern: RulePattern::DomainSuffix("example.com".into()),
            plugin_name: "test-plugin".into(),
            priority: 100,
            enabled: false,
        };

        let mut request = make_request("api.example.com");
        HookExecutor::execute_request_sync(&Arc::new(registry), &[rule], &mut request);

        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
}
