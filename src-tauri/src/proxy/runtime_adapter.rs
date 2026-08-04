//! Desktop Adapters for the core MITM Runtime Interface.

use super::capture_decode::{try_decode_graphql_body, try_decode_grpc_body};
use super::classify::classify_captured_request;
use super::requests::get_or_create_device;
use super::{BreakpointRequest, WsFrameEvent};
use crate::db::{DbState, NewCapturedRequest, NewWebSocketFrame};
use crate::dns::DnsState;
use crate::metrics::counters::ProxyMetrics;
use crate::plugin::{ConnectDecision, InterceptedResponse};
use crate::runtime_extensions::{RequestExtensionOutcome, RuntimeExtensionPipeline};
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
    extensions: Arc<RuntimeExtensionPipeline>,
    breakpoint_tx: tokio::sync::mpsc::Sender<BreakpointRequest>,
}

impl DesktopRuntimeHooks {
    pub(super) fn new(
        extensions: Arc<RuntimeExtensionPipeline>,
        breakpoint_tx: tokio::sync::mpsc::Sender<BreakpointRequest>,
    ) -> Self {
        Self {
            extensions,
            breakpoint_tx,
        }
    }
}

#[async_trait]
impl RuntimeHooks for DesktopRuntimeHooks {
    async fn on_connect(&self, host: &str, _port: u16) -> RuntimeConnectDecision {
        match self.extensions.execute_connect(host) {
            None | Some(ConnectDecision::Allow) => RuntimeConnectDecision::Allow,
            Some(ConnectDecision::Block) => RuntimeConnectDecision::Block,
            Some(ConnectDecision::Redirect(authority)) => {
                RuntimeConnectDecision::Redirect(authority)
            }
        }
    }

    async fn on_request(&self, request: &mut RuntimeRequest) -> RuntimeHookDecision {
        let mut intercepted = request.as_intercepted();
        let outcome = self.extensions.execute_request(&mut intercepted).await;
        request.method = intercepted.method;
        request.path = intercepted.path;
        request.headers = intercepted.req_headers;
        request.body = intercepted.req_body.unwrap_or_default().into_bytes();
        match outcome {
            RequestExtensionOutcome::Continue => RuntimeHookDecision::Continue,
            RequestExtensionOutcome::Respond(response) => {
                RuntimeHookDecision::Respond(runtime_response(response))
            }
        }
    }

    async fn on_response(&self, request: &RuntimeRequest, response: &mut RuntimeResponse) {
        let request = request.as_intercepted();
        let mut intercepted = InterceptedResponse {
            status: Some(response.status),
            headers: response.headers.clone(),
            body: String::from_utf8(response.body.clone()).ok(),
        };
        self.extensions
            .execute_response(&request, &mut intercepted)
            .await;
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
        let effect = self.extensions.traffic_effect(host, byte_count);
        TrafficEffect {
            delay: Duration::from_millis(effect.delay_ms),
            drop: effect.drop,
        }
    }
}

fn runtime_response(response: InterceptedResponse) -> RuntimeResponse {
    RuntimeResponse {
        status: response.status.unwrap_or(403),
        headers: response.headers,
        body: response.body.unwrap_or_default().into_bytes(),
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
    metrics: Arc<ProxyMetrics>,
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
                metrics
                    .connections_closed
                    .fetch_add(aborted_connections as u64, Ordering::Relaxed);
                metrics.connections_active.store(0, Ordering::Relaxed);
            }
            CaptureEvent::ConnectionOpened { .. } => {
                use std::sync::atomic::Ordering;
                metrics.connections_total.fetch_add(1, Ordering::Relaxed);
                metrics.connections_active.fetch_add(1, Ordering::Relaxed);
            }
            CaptureEvent::ConnectionClosed { .. } => {
                use std::sync::atomic::Ordering;
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
                metrics.errors_total.fetch_add(1, Ordering::Relaxed);
                log::error!(
                    "MITM Runtime request {request_id} failed for {}: {error}",
                    host.as_deref().unwrap_or("unknown host")
                );
            }
            CaptureEvent::Completed(mut request) => {
                use std::sync::atomic::Ordering;
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

                let session_id = app_state.active_session_id_snapshot();
                let mut persisted = NewCapturedRequest::from_intercepted(&request);
                persisted.session_id = session_id.as_deref();
                match db.record_captured_request(persisted) {
                    Ok(row_id) => {
                        let desktop_id = row_id.to_string();
                        request_ids.insert(runtime_id, desktop_id.clone());
                        request.id = desktop_id.clone();
                        if request.is_websocket {
                            let _ = db.mark_captured_request_websocket(&desktop_id);
                        }
                    }
                    Err(error) => log::error!("Failed to persist Captured Request: {error}"),
                }
                let _ = app_handle.emit("intercepted-request", &request);
                ai_tracker.process_request(&request);
            }
            CaptureEvent::Frame { request_id, frame } => {
                let desktop_id = request_ids.get(&request_id).cloned().unwrap_or(request_id);
                let _ = db.record_websocket_frame(NewWebSocketFrame {
                    request_id: &desktop_id,
                    direction: &frame.direction,
                    opcode: frame.opcode,
                    payload: &frame.payload,
                    payload_binary: None,
                    size: frame.size,
                    timestamp: &frame.timestamp,
                });
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
