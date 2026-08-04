//! Authoritative request/response extension execution Module.
//!
//! Plugin Dispatch Rules are matched once against the request snapshot, then
//! matching plugin callbacks run in priority order. Successful mutations are
//! committed cumulatively; panics and async timeouts fail open and roll back
//! only that callback. Rhai scripts run afterward in deterministic name order,
//! so later rewrites observe and may replace earlier bodies. Within one script
//! call, the existing rewrite signal takes precedence over its boolean result;
//! across script calls, a later Block wins and produces the Module-owned 403.

use crate::metrics::counters::ProxyMetrics;
use crate::network::{ConditionEffect, NetworkConditionEngine};
use crate::plugin::plugin_trait::{ConnectDecision, InterceptedResponse};
use crate::plugin::registry::PluginRegistry;
use crate::plugin::rule_engine::{PluginDispatchEngine, PluginRule};
use crate::proxy::InterceptedRequest;
use crate::scripting::engine::{ScriptEngine, ScriptResult};
use futures_util::FutureExt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_HOOK_TIMEOUT: Duration = Duration::from_secs(5);

/// Request outcome returned to the desktop Runtime Adapter.
#[derive(Debug)]
pub enum RequestExtensionOutcome {
    Continue,
    Respond(InterceptedResponse),
}

/// Deep Module owning all request/response extension ordering and failure
/// semantics. `plugin_hooks_total` counts actual callback attempts, while
/// `plugin_hooks_errors` counts panics and async timeouts.
pub struct RuntimeExtensionPipeline {
    plugins: Arc<PluginRegistry>,
    plugin_rules: Arc<PluginDispatchEngine>,
    scripts: Arc<ScriptEngine>,
    network: Arc<NetworkConditionEngine>,
    metrics: Arc<ProxyMetrics>,
    hook_timeout: Duration,
}

impl RuntimeExtensionPipeline {
    pub fn new(
        plugins: Arc<PluginRegistry>,
        plugin_rules: Arc<PluginDispatchEngine>,
        scripts: Arc<ScriptEngine>,
        network: Arc<NetworkConditionEngine>,
        metrics: Arc<ProxyMetrics>,
    ) -> Self {
        Self {
            plugins,
            plugin_rules,
            scripts,
            network,
            metrics,
            hook_timeout: DEFAULT_HOOK_TIMEOUT,
        }
    }

    #[cfg(test)]
    fn with_timeout(mut self, timeout: Duration) -> Self {
        self.hook_timeout = timeout;
        self
    }

    /// Execute connection hooks in deterministic plugin-name order.
    pub fn execute_connect(&self, host: &str) -> Option<ConnectDecision> {
        for plugin_name in self.plugins.list_plugins() {
            let Some(plugin) = self.plugins.get(&plugin_name) else {
                continue;
            };
            let Some(hook) = plugin.hooks().on_connect else {
                continue;
            };
            self.record_hook_attempt();
            match catch_unwind(AssertUnwindSafe(|| hook(host))) {
                Ok(ConnectDecision::Allow) => continue,
                Ok(decision) => return Some(decision),
                Err(_) => {
                    self.record_hook_error();
                    log::error!("Plugin '{plugin_name}' connect hook panicked");
                }
            }
        }
        None
    }

    /// Execute the one production request extension path.
    pub async fn execute_request(
        &self,
        request: &mut InterceptedRequest,
    ) -> RequestExtensionOutcome {
        let matched = self.plugin_rules.matching_rules(request);
        for rule in &matched {
            self.execute_request_plugin(rule, request).await;
        }

        for script_name in self.scripts.list_scripts() {
            match self.scripts.run_on_request(&script_name, request) {
                Ok(ScriptResult::Continue) => {}
                Ok(ScriptResult::RewriteBody(body)) => {
                    request.req_body = Some(body);
                }
                Ok(ScriptResult::Block) => {
                    log::info!("Script '{script_name}' blocked request to {}", request.host);
                    return RequestExtensionOutcome::Respond(blocked_response("request"));
                }
                Err(error) => log::error!("Script '{script_name}' request hook failed: {error}"),
            }
        }
        RequestExtensionOutcome::Continue
    }

    /// Execute the one production response extension path. A blocking script
    /// replaces the response with a deterministic 403 response.
    pub async fn execute_response(
        &self,
        request: &InterceptedRequest,
        response: &mut InterceptedResponse,
    ) {
        let matched = self.plugin_rules.matching_rules(request);
        for rule in &matched {
            self.execute_response_plugin(rule, response).await;
        }

        for script_name in self.scripts.list_scripts() {
            match self
                .scripts
                .run_on_response(&script_name, response, request)
            {
                Ok(ScriptResult::Continue) => {}
                Ok(ScriptResult::RewriteBody(body)) => {
                    response.body = Some(body);
                }
                Ok(ScriptResult::Block) => {
                    log::info!(
                        "Script '{script_name}' blocked response from {}",
                        request.host
                    );
                    *response = blocked_response("response");
                    return;
                }
                Err(error) => log::error!("Script '{script_name}' response hook failed: {error}"),
            }
        }
    }

    /// Compute the Network Condition effect through the same Module used by
    /// request and response extensions.
    pub fn traffic_effect(&self, host: &str, byte_count: usize) -> ConditionEffect {
        self.network.apply_for_host(host, byte_count)
    }

    async fn execute_request_plugin(&self, rule: &PluginRule, request: &mut InterceptedRequest) {
        let Some(plugin) = self.plugins.get(&rule.plugin_name) else {
            return;
        };
        let hooks = plugin.hooks();

        if let Some(hook) = hooks.on_request {
            self.record_hook_attempt();
            let mut candidate = request.clone();
            match catch_unwind(AssertUnwindSafe(|| hook(&mut candidate))) {
                Ok(()) => *request = candidate,
                Err(_) => {
                    self.record_hook_error();
                    log::error!(
                        "Plugin '{}' request hook panicked for rule {}",
                        rule.plugin_name,
                        rule.id
                    );
                }
            }
        }

        if let Some(hook) = hooks.on_request_async {
            self.record_hook_attempt();
            let mut candidate = request.clone();
            let future = match catch_unwind(AssertUnwindSafe(|| hook(&mut candidate))) {
                Ok(future) => future,
                Err(_) => {
                    self.record_hook_error();
                    log::error!(
                        "Plugin '{}' async request hook panicked for rule {}",
                        rule.plugin_name,
                        rule.id
                    );
                    return;
                }
            };
            match tokio::time::timeout(self.hook_timeout, AssertUnwindSafe(future).catch_unwind())
                .await
            {
                Ok(Ok(())) => *request = candidate,
                Ok(Err(_)) => {
                    self.record_hook_error();
                    log::error!(
                        "Plugin '{}' async request hook panicked for rule {}",
                        rule.plugin_name,
                        rule.id
                    );
                }
                Err(_) => {
                    self.record_hook_error();
                    log::warn!(
                        "Plugin '{}' async request hook timed out for rule {}",
                        rule.plugin_name,
                        rule.id
                    );
                }
            }
        }
    }

    async fn execute_response_plugin(&self, rule: &PluginRule, response: &mut InterceptedResponse) {
        let Some(plugin) = self.plugins.get(&rule.plugin_name) else {
            return;
        };
        let hooks = plugin.hooks();

        if let Some(hook) = hooks.on_response {
            self.record_hook_attempt();
            let mut candidate = response.clone();
            match catch_unwind(AssertUnwindSafe(|| hook(&mut candidate))) {
                Ok(()) => *response = candidate,
                Err(_) => {
                    self.record_hook_error();
                    log::error!(
                        "Plugin '{}' response hook panicked for rule {}",
                        rule.plugin_name,
                        rule.id
                    );
                }
            }
        }

        if let Some(hook) = hooks.on_response_async {
            self.record_hook_attempt();
            let mut candidate = response.clone();
            let future = match catch_unwind(AssertUnwindSafe(|| hook(&mut candidate))) {
                Ok(future) => future,
                Err(_) => {
                    self.record_hook_error();
                    log::error!(
                        "Plugin '{}' async response hook panicked for rule {}",
                        rule.plugin_name,
                        rule.id
                    );
                    return;
                }
            };
            match tokio::time::timeout(self.hook_timeout, AssertUnwindSafe(future).catch_unwind())
                .await
            {
                Ok(Ok(())) => *response = candidate,
                Ok(Err(_)) => {
                    self.record_hook_error();
                    log::error!(
                        "Plugin '{}' async response hook panicked for rule {}",
                        rule.plugin_name,
                        rule.id
                    );
                }
                Err(_) => {
                    self.record_hook_error();
                    log::warn!(
                        "Plugin '{}' async response hook timed out for rule {}",
                        rule.plugin_name,
                        rule.id
                    );
                }
            }
        }
    }

    fn record_hook_attempt(&self) {
        self.metrics
            .plugin_hooks_total
            .fetch_add(1, Ordering::Relaxed);
    }

    fn record_hook_error(&self) {
        self.metrics
            .plugin_hooks_errors
            .fetch_add(1, Ordering::Relaxed);
    }
}

fn blocked_response(phase: &str) -> InterceptedResponse {
    InterceptedResponse {
        status: Some(403),
        headers: vec![(
            "Content-Type".to_owned(),
            "text/plain; charset=utf-8".to_owned(),
        )],
        body: Some(format!("ProxyBot script blocked this {phase}\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{NetworkHostPattern, NetworkProfile, NewConditionRule};
    use crate::plugin::plugin_trait::{Plugin, PluginHooks};
    use crate::plugin::rule_engine::PluginDispatchPattern;

    struct MutatingPlugin {
        name: String,
        suffix: String,
        status: u16,
    }

    impl Plugin for MutatingPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn hooks(&self) -> PluginHooks {
            let request_suffix = self.suffix.clone();
            let response_suffix = self.suffix.clone();
            let status = self.status;
            PluginHooks {
                on_request: Some(Box::new(move |request| {
                    request.path.push_str(&request_suffix);
                    request
                        .req_headers
                        .push(("X-Plugin".to_owned(), request_suffix.clone()));
                })),
                on_response: Some(Box::new(move |response| {
                    let body = response.body.take().unwrap_or_default();
                    response.body = Some(format!("{body}{response_suffix}"));
                    response.status = Some(status);
                })),
                ..Default::default()
            }
        }
    }

    struct TimeoutPlugin;

    impl Plugin for TimeoutPlugin {
        fn name(&self) -> &str {
            "timeout"
        }

        fn hooks(&self) -> PluginHooks {
            PluginHooks {
                on_request_async: Some(Box::new(|request| {
                    request.path.push_str("/tentative");
                    Box::pin(async {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    })
                })),
                on_response_async: Some(Box::new(|response| {
                    response.status = Some(599);
                    Box::pin(async {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    })
                })),
                ..Default::default()
            }
        }
    }

    fn request() -> InterceptedRequest {
        InterceptedRequest {
            method: "GET".to_owned(),
            scheme: "https".to_owned(),
            host: "api.example.com".to_owned(),
            path: "/start".to_owned(),
            ..Default::default()
        }
    }

    fn matching_rule(id: u64, plugin_name: &str, priority: u16) -> PluginRule {
        PluginRule {
            id,
            name: format!("rule-{id}"),
            pattern: PluginDispatchPattern::DomainSuffix("example.com".to_owned()),
            plugin_name: plugin_name.to_owned(),
            priority,
            enabled: true,
        }
    }

    fn pipeline(
        plugins: Arc<PluginRegistry>,
        rules: Arc<PluginDispatchEngine>,
        scripts: Arc<ScriptEngine>,
        network: Arc<NetworkConditionEngine>,
        metrics: Arc<ProxyMetrics>,
    ) -> RuntimeExtensionPipeline {
        RuntimeExtensionPipeline::new(plugins, rules, scripts, network, metrics)
    }

    #[tokio::test]
    async fn priority_order_accumulates_request_and_response_mutations_and_metrics() {
        let plugins = Arc::new(PluginRegistry::new());
        plugins.register(Box::new(MutatingPlugin {
            name: "later".to_owned(),
            suffix: "B".to_owned(),
            status: 202,
        }));
        plugins.register(Box::new(MutatingPlugin {
            name: "earlier".to_owned(),
            suffix: "A".to_owned(),
            status: 201,
        }));
        let rules = Arc::new(PluginDispatchEngine::new());
        rules.add_rule(matching_rule(1, "later", 100));
        rules.add_rule(matching_rule(2, "earlier", 10));
        let metrics = Arc::new(ProxyMetrics::new());
        let pipeline = pipeline(
            plugins,
            rules,
            Arc::new(ScriptEngine::new()),
            Arc::new(NetworkConditionEngine::new()),
            Arc::clone(&metrics),
        );

        let mut request = request();
        assert!(matches!(
            pipeline.execute_request(&mut request).await,
            RequestExtensionOutcome::Continue
        ));
        assert_eq!(request.path, "/startAB");
        assert_eq!(request.req_headers[0].1, "A");
        assert_eq!(request.req_headers[1].1, "B");

        let mut response = InterceptedResponse {
            status: Some(200),
            body: Some("base".to_owned()),
            ..Default::default()
        };
        pipeline.execute_response(&request, &mut response).await;
        assert_eq!(response.body.as_deref(), Some("baseAB"));
        assert_eq!(response.status, Some(202));
        assert_eq!(metrics.plugin_hooks_total.load(Ordering::Relaxed), 4);
        assert_eq!(metrics.plugin_hooks_errors.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn scripts_rewrite_sequentially_and_block_with_module_owned_response() {
        let scripts = Arc::new(ScriptEngine::new());
        scripts
            .load_from_string("a-first", "rewrite_request_body(\"first\"); true")
            .unwrap();
        scripts
            .load_from_string(
                "b-second",
                "if req_body == \"first\" { rewrite_request_body(\"second\"); } true",
            )
            .unwrap();
        scripts.load_from_string("z-block", "false").unwrap();
        let pipeline = pipeline(
            Arc::new(PluginRegistry::new()),
            Arc::new(PluginDispatchEngine::new()),
            scripts,
            Arc::new(NetworkConditionEngine::new()),
            Arc::new(ProxyMetrics::new()),
        );

        let mut request = request();
        let RequestExtensionOutcome::Respond(blocked) =
            pipeline.execute_request(&mut request).await
        else {
            panic!("blocking script should synthesize a response");
        };
        assert_eq!(request.req_body.as_deref(), Some("second"));
        assert_eq!(blocked.status, Some(403));
        assert_eq!(
            blocked.body.as_deref(),
            Some("ProxyBot script blocked this request\n")
        );
    }

    #[tokio::test]
    async fn response_script_rewrite_and_block_are_explicit() {
        let scripts = Arc::new(ScriptEngine::new());
        scripts
            .load_from_string("a-rewrite", "rewrite_response_body(\"rewritten\"); true")
            .unwrap();
        scripts.load_from_string("z-block", "false").unwrap();
        let pipeline = pipeline(
            Arc::new(PluginRegistry::new()),
            Arc::new(PluginDispatchEngine::new()),
            scripts,
            Arc::new(NetworkConditionEngine::new()),
            Arc::new(ProxyMetrics::new()),
        );
        let mut response = InterceptedResponse {
            status: Some(200),
            body: Some("original".to_owned()),
            ..Default::default()
        };

        pipeline.execute_response(&request(), &mut response).await;
        assert_eq!(response.status, Some(403));
        assert_eq!(
            response.body.as_deref(),
            Some("ProxyBot script blocked this response\n")
        );
    }

    #[tokio::test]
    async fn async_timeouts_roll_back_mutations_and_count_attempts_and_errors() {
        let plugins = Arc::new(PluginRegistry::new());
        plugins.register(Box::new(TimeoutPlugin));
        let rules = Arc::new(PluginDispatchEngine::new());
        rules.add_rule(matching_rule(1, "timeout", 10));
        let metrics = Arc::new(ProxyMetrics::new());
        let pipeline = pipeline(
            plugins,
            rules,
            Arc::new(ScriptEngine::new()),
            Arc::new(NetworkConditionEngine::new()),
            Arc::clone(&metrics),
        )
        .with_timeout(Duration::from_millis(1));

        let mut request = request();
        let original_path = request.path.clone();
        pipeline.execute_request(&mut request).await;
        assert_eq!(request.path, original_path);

        let mut response = InterceptedResponse {
            status: Some(200),
            ..Default::default()
        };
        pipeline.execute_response(&request, &mut response).await;
        assert_eq!(response.status, Some(200));
        assert_eq!(metrics.plugin_hooks_total.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.plugin_hooks_errors.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn network_conditions_share_the_extension_module_seam() {
        let network = Arc::new(NetworkConditionEngine::new());
        network.add_profile(NetworkProfile {
            name: "fixed".to_owned(),
            latency_ms: 17,
            bandwidth_kbps: 0,
            packet_loss_pct: 0,
        });
        network.add_rule(NewConditionRule {
            pattern: NetworkHostPattern::Domain,
            value: "api.example.com".to_owned(),
            profile: "fixed".to_owned(),
            enabled: true,
        });
        let pipeline = pipeline(
            Arc::new(PluginRegistry::new()),
            Arc::new(PluginDispatchEngine::new()),
            Arc::new(ScriptEngine::new()),
            network,
            Arc::new(ProxyMetrics::new()),
        );

        let effect = pipeline.traffic_effect("api.example.com", 128);
        assert_eq!(effect.delay_ms, 17);
        assert!(!effect.drop);
    }
}
