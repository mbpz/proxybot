//! Desktop Adapters for the core MITM Runtime Interface.

use super::capture_decode::{try_decode_graphql_body, try_decode_grpc_body};
use super::classify::classify_captured_request;
use super::hooks::{call_on_connect_hooks, call_on_request_hooks, call_on_response_hooks};
use super::requests::get_or_create_device;
use super::{BreakpointRequest, WsFrameEvent};
use crate::db::{mark_request_websocket, record_http_request, record_ws_frame, DbState};
use crate::dns::DnsState;
use crate::network::NetworkConditionEngine;
use crate::plugin::registry::PluginRegistry;
use crate::plugin::{InterceptedResponse, PluginDispatchEngine};
use crate::scripting::engine::{ScriptEngine, ScriptResult};
use crate::state::AppState;
use async_trait::async_trait;
use proxybot_core::{
    BreakpointDecision, BreakpointTarget, CaptureEvent, OriginalDestination,
    RuntimeConnectDecision, RuntimeHookDecision, RuntimeHooks, RuntimeRequest, RuntimeResponse,
    TrafficDirection, TrafficEffect,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpStream;

pub(super) struct DesktopRuntimeHooks {
    plugins: Arc<PluginRegistry>,
    plugin_rules: Arc<PluginDispatchEngine>,
    scripts: Arc<ScriptEngine>,
    network: Arc<NetworkConditionEngine>,
    breakpoint_tx: tokio::sync::mpsc::Sender<BreakpointRequest>,
}

impl DesktopRuntimeHooks {
    pub(super) fn new(
        plugins: Arc<PluginRegistry>,
        plugin_rules: Arc<PluginDispatchEngine>,
        scripts: Arc<ScriptEngine>,
        network: Arc<NetworkConditionEngine>,
        breakpoint_tx: tokio::sync::mpsc::Sender<BreakpointRequest>,
    ) -> Self {
        Self {
            plugins,
            plugin_rules,
            scripts,
            network,
            breakpoint_tx,
        }
    }
}

#[async_trait]
impl RuntimeHooks for DesktopRuntimeHooks {
    async fn on_connect(&self, host: &str, _port: u16) -> RuntimeConnectDecision {
        match call_on_connect_hooks(&self.plugins, host) {
            None | Some(crate::plugin::ConnectDecision::Allow) => RuntimeConnectDecision::Allow,
            Some(crate::plugin::ConnectDecision::Block) => RuntimeConnectDecision::Block,
            Some(crate::plugin::ConnectDecision::Redirect(authority)) => {
                RuntimeConnectDecision::Redirect(authority)
            }
        }
    }

    async fn on_request(&self, request: &mut RuntimeRequest) -> RuntimeHookDecision {
        let mut intercepted = request.as_intercepted();
        call_on_request_hooks(&self.plugins, &self.plugin_rules, &mut intercepted);
        request.method = intercepted.method;
        request.path = intercepted.path;
        request.headers = intercepted.req_headers;
        request.body = intercepted.req_body.unwrap_or_default().into_bytes();

        match self.scripts.run_all_on_request(&request.as_intercepted()) {
            ScriptResult::Continue => RuntimeHookDecision::Continue,
            ScriptResult::RewriteBody(body) => {
                request.body = body.into_bytes();
                RuntimeHookDecision::Continue
            }
            ScriptResult::Block => RuntimeHookDecision::Respond(RuntimeResponse {
                status: 403,
                headers: vec![(
                    "Content-Type".to_owned(),
                    "text/plain; charset=utf-8".to_owned(),
                )],
                body: b"ProxyBot script blocked this request\n".to_vec(),
            }),
        }
    }

    async fn on_response(&self, request: &RuntimeRequest, response: &mut RuntimeResponse) {
        let request = request.as_intercepted();
        let mut intercepted = InterceptedResponse {
            status: Some(response.status),
            headers: response.headers.clone(),
            body: String::from_utf8(response.body.clone()).ok(),
        };
        call_on_response_hooks(
            &self.plugins,
            &self.plugin_rules,
            &mut intercepted,
            &request,
        );
        if let ScriptResult::RewriteBody(body) =
            self.scripts.run_all_on_response(&intercepted, &request)
        {
            intercepted.body = Some(body);
        }
        if let Some(status) = intercepted.status {
            response.status = status;
        }
        response.headers = intercepted.headers;
        if let Some(body) = intercepted.body {
            response.body = body.into_bytes();
        }
    }

    async fn on_breakpoint(
        &self,
        request: proxybot_core::InterceptedRequest,
        target: BreakpointTarget,
    ) -> BreakpointDecision {
        let (decision_tx, decision_rx) = tokio::sync::oneshot::channel();
        if self
            .breakpoint_tx
            .send(BreakpointRequest {
                request,
                target,
                decision_tx,
            })
            .await
            .is_err()
        {
            return BreakpointDecision::Proceed;
        }
        decision_rx.await.unwrap_or(BreakpointDecision::Proceed)
    }

    fn traffic_effect(
        &self,
        host: &str,
        _direction: TrafficDirection,
        byte_count: usize,
    ) -> TrafficEffect {
        let effect = self.network.apply_for_host(host, byte_count);
        TrafficEffect {
            delay: Duration::from_millis(effect.delay_ms),
            drop: effect.drop,
        }
    }
}

pub(super) struct PfOriginalDestination;

impl OriginalDestination for PfOriginalDestination {
    fn original_destination(&self, stream: &TcpStream) -> Option<std::net::SocketAddr> {
        super::tls::get_original_dst_addr(stream)
    }
}

pub(super) async fn bridge_capture_events(
    mut events: tokio::sync::mpsc::Receiver<CaptureEvent>,
    app_handle: AppHandle,
    db: Arc<DbState>,
    dns: Arc<DnsState>,
    app_state: Arc<AppState>,
) {
    let ai_tracker = crate::ai::AiTracker::new(Arc::clone(&db));
    let mut request_ids = HashMap::<String, String>::new();

    while let Some(event) = events.recv().await {
        match event {
            CaptureEvent::Started { bound_addr } => {
                log::info!("MITM Runtime listening on {bound_addr}");
            }
            CaptureEvent::Stopped {
                aborted_connections,
            } => {
                use std::sync::atomic::Ordering;
                let metrics = &crate::metrics::counters::METRICS;
                metrics
                    .connections_closed
                    .fetch_add(aborted_connections as u64, Ordering::Relaxed);
                metrics.connections_active.store(0, Ordering::Relaxed);
            }
            CaptureEvent::ConnectionOpened { .. } => {
                use std::sync::atomic::Ordering;
                crate::metrics::counters::METRICS
                    .connections_total
                    .fetch_add(1, Ordering::Relaxed);
                crate::metrics::counters::METRICS
                    .connections_active
                    .fetch_add(1, Ordering::Relaxed);
            }
            CaptureEvent::ConnectionClosed { .. } => {
                use std::sync::atomic::Ordering;
                let metrics = &crate::metrics::counters::METRICS;
                metrics.connections_closed.fetch_add(1, Ordering::Relaxed);
                let _ = metrics.connections_active.fetch_update(
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                    |active| Some(active.saturating_sub(1)),
                );
            }
            CaptureEvent::Failed {
                request_id,
                host,
                error,
            } => {
                use std::sync::atomic::Ordering;
                crate::metrics::counters::METRICS
                    .errors_total
                    .fetch_add(1, Ordering::Relaxed);
                log::error!(
                    "MITM Runtime request {request_id} failed for {}: {error}",
                    host.as_deref().unwrap_or("unknown host")
                );
            }
            CaptureEvent::Completed(mut request) => {
                use std::sync::atomic::Ordering;
                let metrics = &crate::metrics::counters::METRICS;
                if request.scheme == "https" {
                    metrics.https_requests_total.fetch_add(1, Ordering::Relaxed);
                } else {
                    metrics.http_requests_total.fetch_add(1, Ordering::Relaxed);
                }
                metrics.record_method(&request.method);
                if let Some(status) = request.status {
                    metrics.record_status(status);
                }
                metrics.bytes_received.fetch_add(
                    request.req_body.as_deref().map_or(0, str::len) as u64,
                    Ordering::Relaxed,
                );
                metrics.bytes_sent.fetch_add(
                    request.resp_size.unwrap_or_default() as u64,
                    Ordering::Relaxed,
                );
                let runtime_id = request.id.clone();
                let client_ip = request.client_ip.clone().unwrap_or_default();
                if let Some((name, icon)) = classify_captured_request(
                    &request.host,
                    &request.scheme,
                    &client_ip,
                    request.upstream_ip.as_deref(),
                    &request.timestamp,
                    &dns,
                ) {
                    request.app_name = Some(name);
                    request.app_icon = Some(icon);
                }
                if !client_ip.is_empty() {
                    if let Some(device) = get_or_create_device(&db, &client_ip).await {
                        request.device_id = Some(device.device_id);
                        request.device_name = Some(device.device_name);
                    }
                }
                let response_body = request.resp_body.as_deref().unwrap_or_default().as_bytes();
                request.grpc_decoded = try_decode_grpc_body(&request.resp_headers, response_body);
                request.graphql_op =
                    try_decode_graphql_body(&request.req_headers, request.req_body.as_deref());

                if let Ok(connection) = db.conn.lock() {
                    let session_id = app_state.active_session_id_snapshot();
                    match record_http_request(
                        &connection,
                        &request.timestamp,
                        &request.method,
                        &request.scheme,
                        &request.host,
                        &request.path,
                        &request.req_headers,
                        request.req_body.as_deref(),
                        request.status,
                        &request.resp_headers,
                        request.resp_body.as_deref(),
                        request.latency_ms,
                        request.device_id,
                        request.app_name.as_deref(),
                        session_id.as_deref(),
                    ) {
                        Ok(row_id) => {
                            let desktop_id = row_id.to_string();
                            request_ids.insert(runtime_id, desktop_id.clone());
                            request.id = desktop_id.clone();
                            if request.is_websocket {
                                let _ = mark_request_websocket(&connection, &desktop_id);
                            }
                        }
                        Err(error) => log::error!("Failed to persist Captured Request: {error}"),
                    }
                }
                let _ = app_handle.emit("intercepted-request", &request);
                ai_tracker.process_request(&request);
            }
            CaptureEvent::Frame { request_id, frame } => {
                let desktop_id = request_ids.get(&request_id).cloned().unwrap_or(request_id);
                if let Ok(connection) = db.conn.lock() {
                    let _ = record_ws_frame(
                        &connection,
                        &desktop_id,
                        &frame.direction,
                        frame.opcode,
                        &frame.payload,
                        None,
                        frame.size,
                        &frame.timestamp,
                    );
                }
                let _ = app_handle.emit(
                    "ws-frame:new",
                    WsFrameEvent {
                        request_id: desktop_id,
                        frame,
                    },
                );
            }
        }
    }
}
