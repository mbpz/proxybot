//! Plugin hook invocation helpers (on_request, on_response, on_connect).

use super::InterceptedRequest;
use crate::metrics::counters::METRICS;
use crate::plugin::{HookExecutor, InterceptedResponse, RuleEngine};
use crate::plugin::registry::PluginRegistry;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Call on_request hooks using rule-based dispatch
pub(super) fn call_on_request_hooks(
    plugins: &Arc<PluginRegistry>,
    rule_engine: &Arc<RuleEngine>,
    request: &mut InterceptedRequest,
) {
    let matched = rule_engine.match_all(request);
    HookExecutor::run_request_hooks(plugins, &matched, request);
    METRICS.plugin_hooks_total.fetch_add(1, Ordering::Relaxed);
}

/// Call on_response hooks using rule-based dispatch.
/// Uses the original request context for host-based rule matching, since HTTP
/// responses do not carry a `Host` header.
pub(super) fn call_on_response_hooks(
    plugins: &Arc<PluginRegistry>,
    rule_engine: &Arc<RuleEngine>,
    response: &mut InterceptedResponse,
    request: &InterceptedRequest,
) {
    let matched = rule_engine.match_all(request);
    HookExecutor::run_response_hooks(plugins, &matched, response);
    METRICS.plugin_hooks_total.fetch_add(1, Ordering::Relaxed);
}

/// Call on_connect hooks for all registered plugins
pub(super) fn call_on_connect_hooks(
    plugins: &Arc<PluginRegistry>,
    host: &str,
) -> Option<crate::plugin::ConnectDecision> {
    for plugin_name in plugins.list_plugins() {
        if let Some(plugin) = plugins.get(&plugin_name) {
            if let Some(ref hook) = plugin.hooks().on_connect {
                let decision = hook(host);
                METRICS.plugin_hooks_total.fetch_add(1, Ordering::Relaxed);
                match decision {
                    crate::plugin::ConnectDecision::Allow => continue,
                    crate::plugin::ConnectDecision::Block => return Some(decision),
                    crate::plugin::ConnectDecision::Redirect(_) => return Some(decision),
                }
            }
        }
    }
    None
}
