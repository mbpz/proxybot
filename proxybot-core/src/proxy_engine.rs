//! Standalone MITM Runtime Module.
//!
//! The runtime owns bind/start/shutdown, HTTP and HTTPS transaction ordering,
//! Routing Rule application, upstream TLS policy, and Capture Event emission.
//! Desktop-specific plugins, scripts, traffic shaping, persistence, and UI
//! delivery enter through [`RuntimeHooks`].

use crate::{
    body, BreakpointDecision, BreakpointTarget, CertManager, InterceptedRequest, RuleAction,
    RulesEngine, TlsAction, TlsRuleSet, WsFrame,
};
use async_trait::async_trait;
use rustls::client::danger as rustls_danger;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, ServerConfig, SignatureScheme};
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio::task::{JoinHandle, JoinSet};
use tokio_rustls::{TlsAcceptor, TlsConnector};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Whether upstream TLS certificates are verified.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UpstreamTlsPolicy {
    /// Verify against the public WebPKI root set.
    #[default]
    Verify,
    /// Accept any upstream certificate. This must be selected explicitly.
    DangerouslyAcceptInvalid,
}

/// Configuration known before the MITM Runtime binds its listener.
#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    pub bind_addr: SocketAddr,
    pub upstream_tls: UpstreamTlsPolicy,
    pub reverse_target: Option<String>,
    pub io_timeout: Duration,
    pub max_message_bytes: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from(([0, 0, 0, 0], crate::config::proxy_port())),
            upstream_tls: UpstreamTlsPolicy::Verify,
            reverse_target: crate::config::reverse_target(),
            io_timeout: Duration::from_secs(15),
            max_message_bytes: 16 * 1024 * 1024,
        }
    }
}

/// Request passed through the runtime hook and Routing Rule pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRequest {
    pub id: String,
    pub timestamp: String,
    pub method: String,
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub client_ip: IpAddr,
    pub upstream_ip: Option<IpAddr>,
}

impl RuntimeRequest {
    pub fn as_intercepted(&self) -> InterceptedRequest {
        InterceptedRequest {
            id: self.id.clone(),
            timestamp: self.timestamp.clone(),
            method: self.method.clone(),
            host: self.host.clone(),
            path: self.path.clone(),
            query_params: extract_query_params(&self.path),
            scheme: self.scheme.clone(),
            req_headers: self.headers.clone(),
            req_body: body_to_string(&self.body),
            client_ip: Some(self.client_ip.to_string()),
            upstream_ip: self.upstream_ip.map(|ip| ip.to_string()),
            ..InterceptedRequest::default()
        }
    }

    fn apply_intercepted(&mut self, request: InterceptedRequest) {
        self.method = request.method;
        self.path = request.path;
        self.headers = request.req_headers;
        self.body = request.req_body.unwrap_or_default().into_bytes();
    }
}

/// Mutable response exposed to runtime hooks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Hook decision made before a Routing Rule is applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeHookDecision {
    Continue,
    Respond(RuntimeResponse),
}

/// Connection-level decision made before an HTTPS tunnel is established.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeConnectDecision {
    Allow,
    Block,
    Redirect(String),
}

/// Traffic shaping result for one forwarded chunk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrafficEffect {
    pub delay: Duration,
    pub drop: bool,
}

/// Direction of a forwarded WebSocket or tunnel chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrafficDirection {
    ClientToUpstream,
    UpstreamToClient,
}

/// Desktop/headless extension Seam for behavior that is not protocol logic.
#[async_trait]
pub trait RuntimeHooks: Send + Sync + 'static {
    async fn on_connect(&self, _host: &str, _port: u16) -> RuntimeConnectDecision {
        RuntimeConnectDecision::Allow
    }

    async fn on_request(&self, _request: &mut RuntimeRequest) -> RuntimeHookDecision {
        RuntimeHookDecision::Continue
    }

    async fn on_response(&self, _request: &RuntimeRequest, _response: &mut RuntimeResponse) {}

    async fn on_breakpoint(
        &self,
        _request: InterceptedRequest,
        _target: BreakpointTarget,
    ) -> BreakpointDecision {
        BreakpointDecision::Proceed
    }

    fn traffic_effect(
        &self,
        _host: &str,
        _direction: TrafficDirection,
        _byte_count: usize,
    ) -> TrafficEffect {
        TrafficEffect::default()
    }
}

#[derive(Default)]
pub struct NoopRuntimeHooks;

#[async_trait]
impl RuntimeHooks for NoopRuntimeHooks {}

/// Platform Adapter for transparent-proxy destination recovery.
pub trait OriginalDestination: Send + Sync + 'static {
    fn original_destination(&self, stream: &TcpStream) -> Option<SocketAddr>;
}

#[derive(Default)]
pub struct NoOriginalDestination;

impl OriginalDestination for NoOriginalDestination {
    fn original_destination(&self, _stream: &TcpStream) -> Option<SocketAddr> {
        None
    }
}

/// Events emitted by the MITM Runtime through one stable capture id.
#[derive(Clone, Debug)]
pub enum CaptureEvent {
    Started {
        bound_addr: SocketAddr,
    },
    Stopped {
        aborted_connections: usize,
    },
    ConnectionOpened {
        client_addr: SocketAddr,
    },
    ConnectionClosed {
        client_addr: SocketAddr,
    },
    Completed(Box<InterceptedRequest>),
    Frame {
        request_id: String,
        frame: WsFrame,
    },
    Failed {
        request_id: String,
        host: Option<String>,
        error: String,
    },
}

/// Errors returned at the runtime Interface.
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("failed to bind MITM Runtime at {addr}: {source}")]
    Bind {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Protocol(String),
    #[error("{0}")]
    Tls(String),
    #[error("{0}")]
    Io(String),
}

/// Configured, not-yet-running MITM Runtime.
pub struct MitmRuntime {
    config: RuntimeConfig,
    certs: Arc<CertManager>,
    rules: Arc<RulesEngine>,
    tls_rules: Arc<RwLock<TlsRuleSet>>,
    hooks: Arc<dyn RuntimeHooks>,
    original_destination: Arc<dyn OriginalDestination>,
}

impl MitmRuntime {
    pub fn new(
        config: RuntimeConfig,
        certs: Arc<CertManager>,
        rules: Arc<RulesEngine>,
        tls_rules: Arc<RwLock<TlsRuleSet>>,
    ) -> Self {
        Self {
            config,
            certs,
            rules,
            tls_rules,
            hooks: Arc::new(NoopRuntimeHooks),
            original_destination: Arc::new(NoOriginalDestination),
        }
    }

    pub fn with_hooks(mut self, hooks: Arc<dyn RuntimeHooks>) -> Self {
        self.hooks = hooks;
        self
    }

    pub fn with_original_destination(
        mut self,
        original_destination: Arc<dyn OriginalDestination>,
    ) -> Self {
        self.original_destination = original_destination;
        self
    }

    /// Bind successfully before returning a running handle.
    pub async fn start(self) -> Result<RunningMitm, RuntimeError> {
        let listener = TcpListener::bind(self.config.bind_addr)
            .await
            .map_err(|source| RuntimeError::Bind {
                addr: self.config.bind_addr,
                source,
            })?;
        let bound_addr = listener
            .local_addr()
            .map_err(|error| RuntimeError::Io(error.to_string()))?;
        let (event_tx, event_rx) = mpsc::channel(512);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let context = Arc::new(RuntimeContext {
            config: self.config,
            certs: self.certs,
            rules: self.rules,
            tls_rules: self.tls_rules,
            hooks: self.hooks,
            original_destination: self.original_destination,
            events: event_tx.clone(),
        });
        event_tx
            .send(CaptureEvent::Started { bound_addr })
            .await
            .ok();
        let task = tokio::spawn(run_listener(listener, context, shutdown_rx));
        Ok(RunningMitm {
            bound_addr,
            events: Some(event_rx),
            shutdown: Some(shutdown_tx),
            task: Some(task),
        })
    }
}

/// Ownership handle for one live MITM Runtime.
pub struct RunningMitm {
    bound_addr: SocketAddr,
    events: Option<mpsc::Receiver<CaptureEvent>>,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl RunningMitm {
    pub fn bound_addr(&self) -> SocketAddr {
        self.bound_addr
    }

    pub fn is_running(&self) -> bool {
        self.task.as_ref().is_some_and(|task| !task.is_finished())
    }

    /// The event stream has one owner; desktop Adapters fan out after this Seam.
    pub fn take_events(&mut self) -> Option<mpsc::Receiver<CaptureEvent>> {
        self.events.take()
    }

    pub async fn shutdown(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for RunningMitm {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

/// Backward-compatible name for consumers that imported `ProxyEngine`.
pub type ProxyEngine = MitmRuntime;

struct RuntimeContext {
    config: RuntimeConfig,
    certs: Arc<CertManager>,
    rules: Arc<RulesEngine>,
    tls_rules: Arc<RwLock<TlsRuleSet>>,
    hooks: Arc<dyn RuntimeHooks>,
    original_destination: Arc<dyn OriginalDestination>,
    events: mpsc::Sender<CaptureEvent>,
}

async fn run_listener(
    listener: TcpListener,
    context: Arc<RuntimeContext>,
    mut shutdown: oneshot::Receiver<()>,
) {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((stream, client_addr)) => {
                    let context = Arc::clone(&context);
                    connections.spawn(async move {
                        let _ = context.events.send(CaptureEvent::ConnectionOpened { client_addr }).await;
                        if let Err(error) = handle_connection(Arc::clone(&context), stream, client_addr).await {
                            let _ = context.events.send(CaptureEvent::Failed {
                                request_id: next_request_id(),
                                host: None,
                                error: error.to_string(),
                            }).await;
                        }
                        let _ = context.events.send(CaptureEvent::ConnectionClosed { client_addr }).await;
                    });
                }
                Err(error) => log::error!("MITM Runtime accept failed: {error}"),
            },
            Some(result) = connections.join_next(), if !connections.is_empty() => {
                if let Err(error) = result {
                    log::warn!("MITM Runtime connection task failed: {error}");
                }
            }
            _ = &mut shutdown => {
                let aborted_connections = connections.len();
                connections.abort_all();
                while connections.join_next().await.is_some() {}
                let _ = context.events.send(CaptureEvent::Stopped { aborted_connections }).await;
                break;
            },
        }
    }
}

async fn handle_connection(
    context: Arc<RuntimeContext>,
    stream: TcpStream,
    client_addr: SocketAddr,
) -> Result<(), RuntimeError> {
    let mut hello = vec![0u8; 32 * 1024];
    let peeked = stream
        .peek(&mut hello)
        .await
        .map_err(|error| RuntimeError::Io(error.to_string()))?;
    hello.truncate(peeked);

    if hello.first() == Some(&0x16) {
        if let Some(original) = context.original_destination.original_destination(&stream) {
            let host =
                extract_sni_from_client_hello(&hello).unwrap_or_else(|| original.ip().to_string());
            let target =
                match connect_target(&context, &host, original.port(), Some(original)).await? {
                    Some(target) => target,
                    None => return Ok(()),
                };
            return handle_tls(context, stream, client_addr, host, target, false).await;
        }
    }

    let mut stream = stream;
    let request_bytes = read_http_message(
        &mut stream,
        Vec::new(),
        MessageKind::Request,
        &context.config,
    )
    .await?;
    let parsed = parse_http_request(&request_bytes)
        .ok_or_else(|| RuntimeError::Protocol("invalid HTTP request".to_owned()))?;

    if parsed.method.eq_ignore_ascii_case("CONNECT") {
        let (host, port) = parse_authority(&parsed.path, 443)?;
        let target = match connect_target(&context, &host, port, None).await? {
            Some(target) => target,
            None => {
                stream
                    .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
                    .await
                    .map_err(|error| RuntimeError::Io(error.to_string()))?;
                return Ok(());
            }
        };
        stream
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await
            .map_err(|error| RuntimeError::Io(error.to_string()))?;
        handle_tls(context, stream, client_addr, host, target, true).await
    } else {
        let authority = header_value(&parsed.headers, "host")
            .ok_or_else(|| RuntimeError::Protocol("HTTP request has no Host header".to_owned()))?;
        let (host, port) = parse_authority(authority, 80)?;
        let request = RuntimeRequest {
            id: next_request_id(),
            timestamp: timestamp_now(),
            method: parsed.method,
            scheme: "http".to_owned(),
            host,
            port,
            path: normalize_request_target(&parsed.path),
            headers: parsed.headers,
            body: parsed.body,
            client_ip: client_addr.ip(),
            upstream_ip: None,
        };
        execute_transaction(&context, &mut stream, request, None).await
    }
}

async fn connect_target(
    context: &RuntimeContext,
    host: &str,
    port: u16,
    original: Option<SocketAddr>,
) -> Result<Option<SocketAddr>, RuntimeError> {
    match context.hooks.on_connect(host, port).await {
        RuntimeConnectDecision::Allow => match original {
            Some(original) => Ok(Some(original)),
            None => resolve_target(host, port).await.map(Some),
        },
        RuntimeConnectDecision::Block => Ok(None),
        RuntimeConnectDecision::Redirect(authority) => {
            let (redirect_host, redirect_port) = parse_authority(&authority, port)?;
            resolve_target(&redirect_host, redirect_port)
                .await
                .map(Some)
        }
    }
}

async fn handle_tls(
    context: Arc<RuntimeContext>,
    mut stream: TcpStream,
    client_addr: SocketAddr,
    host: String,
    target: SocketAddr,
    _explicit_connect: bool,
) -> Result<(), RuntimeError> {
    let tls_action = context
        .tls_rules
        .read()
        .map(|rules| rules.decide(&host))
        .unwrap_or(TlsAction::Decrypt);
    if !tls_action.is_decrypt() {
        let mut upstream = TcpStream::connect(target)
            .await
            .map_err(|error| RuntimeError::Io(error.to_string()))?;
        if tls_action.should_log() {
            let capture = InterceptedRequest {
                id: next_request_id(),
                timestamp: timestamp_now(),
                method: "CONNECT".to_owned(),
                scheme: "https".to_owned(),
                host: host.clone(),
                path: "/".to_owned(),
                status: Some(200),
                client_ip: Some(client_addr.ip().to_string()),
                ..InterceptedRequest::default()
            };
            let _ = context
                .events
                .send(CaptureEvent::Completed(Box::new(capture)))
                .await;
        }
        return pipe_tunnel(&context, &host, &mut stream, &mut upstream).await;
    }

    let (cert_pem, key_pem) = context
        .certs
        .generate_host_cert(&host)
        .map_err(RuntimeError::Tls)?;
    let certs = rustls_pemfile::certs(&mut std::io::Cursor::new(cert_pem.as_bytes()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| RuntimeError::Tls(error.to_string()))?;
    let key = rustls_pemfile::private_key(&mut std::io::Cursor::new(key_pem.as_bytes()))
        .map_err(|error| RuntimeError::Tls(error.to_string()))?
        .ok_or_else(|| RuntimeError::Tls("leaf certificate has no private key".to_owned()))?;
    let server_config = ServerConfig::builder_with_provider(runtime_crypto_provider())
        .with_safe_default_protocol_versions()
        .map_err(|error| RuntimeError::Tls(error.to_string()))?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|error| RuntimeError::Tls(error.to_string()))?;
    let mut client_tls = TlsAcceptor::from(Arc::new(server_config))
        .accept(stream)
        .await
        .map_err(|error| RuntimeError::Tls(format!("client TLS handshake failed: {error}")))?;

    let request_bytes = read_http_message(
        &mut client_tls,
        Vec::new(),
        MessageKind::Request,
        &context.config,
    )
    .await?;
    let parsed = parse_http_request(&request_bytes)
        .ok_or_else(|| RuntimeError::Protocol("invalid decrypted HTTP request".to_owned()))?;
    let request = RuntimeRequest {
        id: next_request_id(),
        timestamp: timestamp_now(),
        method: parsed.method,
        scheme: "https".to_owned(),
        host,
        port: target.port(),
        path: normalize_request_target(&parsed.path),
        headers: parsed.headers,
        body: parsed.body,
        client_ip: client_addr.ip(),
        upstream_ip: Some(target.ip()),
    };
    let result = execute_transaction(&context, &mut client_tls, request, Some(target)).await;
    let close_result = client_tls
        .shutdown()
        .await
        .map_err(|error| RuntimeError::Io(error.to_string()));
    result?;
    close_result
}

async fn execute_transaction<C>(
    context: &Arc<RuntimeContext>,
    client: &mut C,
    mut request: RuntimeRequest,
    connect_override: Option<SocketAddr>,
) -> Result<(), RuntimeError>
where
    C: AsyncRead + AsyncWrite + Unpin + Send,
{
    let started = Instant::now();
    if let RuntimeHookDecision::Respond(response) = context.hooks.on_request(&mut request).await {
        return finish_synthetic(context, client, request, response, started).await;
    }

    let plan = plan_request(context, request).await?;
    match plan {
        RequestPlan::Respond { request, response } => {
            finish_synthetic(context, client, request, response, started).await
        }
        RequestPlan::Forward {
            mut request,
            target,
            response_breakpoint,
        } => {
            let target = if target.mapped {
                target
            } else {
                UpstreamTarget {
                    connect_override,
                    ..target
                }
            };
            let (mut upstream, response_bytes, upstream_addr) =
                forward_to_upstream(context, &request, &target).await?;
            request.upstream_ip = Some(upstream_addr.ip());
            let mut response = parse_http_response(&response_bytes).ok_or_else(|| {
                RuntimeError::Protocol("invalid upstream HTTP response".to_owned())
            })?;
            let original = response.clone();
            context.hooks.on_response(&request, &mut response).await;
            if response_breakpoint {
                apply_response_breakpoint(context, &request, &mut response).await;
            }
            let output = if response == original {
                response_bytes
            } else {
                build_http_response(&response)
            };
            client
                .write_all(&output)
                .await
                .map_err(|error| RuntimeError::Io(error.to_string()))?;

            let is_websocket = response.status == 101
                && is_ws_upgrade(&request.headers)
                && is_ws_upgrade(&response.headers);
            emit_capture(
                context,
                &request,
                &response,
                output.len(),
                started.elapsed(),
                is_websocket,
            )
            .await;
            if is_websocket {
                pipe_websocket(context, &request.host, &request.id, client, &mut upstream).await?;
            }
            Ok(())
        }
    }
}

async fn finish_synthetic<C>(
    context: &Arc<RuntimeContext>,
    client: &mut C,
    request: RuntimeRequest,
    mut response: RuntimeResponse,
    started: Instant,
) -> Result<(), RuntimeError>
where
    C: AsyncWrite + Unpin + Send,
{
    context.hooks.on_response(&request, &mut response).await;
    let bytes = build_http_response(&response);
    client
        .write_all(&bytes)
        .await
        .map_err(|error| RuntimeError::Io(error.to_string()))?;
    emit_capture(
        context,
        &request,
        &response,
        bytes.len(),
        started.elapsed(),
        false,
    )
    .await;
    Ok(())
}

enum RequestPlan {
    Respond {
        request: RuntimeRequest,
        response: RuntimeResponse,
    },
    Forward {
        request: RuntimeRequest,
        target: UpstreamTarget,
        response_breakpoint: bool,
    },
}

#[derive(Clone, Debug)]
struct UpstreamTarget {
    scheme: String,
    host: String,
    port: u16,
    path_prefix: String,
    mapped: bool,
    connect_override: Option<SocketAddr>,
}

async fn plan_request(
    context: &RuntimeContext,
    mut request: RuntimeRequest,
) -> Result<RequestPlan, RuntimeError> {
    let action = context
        .rules
        .match_host(&request.host, Some(request.client_ip));
    let action = match action {
        Some(action) => Some(action),
        None => context
            .config
            .reverse_target
            .as_deref()
            .map(|target| RuleAction::MapRemote(target.to_owned())),
    };

    let mut response_breakpoint = false;
    match action {
        None | Some(RuleAction::Direct | RuleAction::Proxy) => {}
        Some(RuleAction::Reject) => {
            return Ok(RequestPlan::Respond {
                request,
                response: text_response(403, "ProxyBot rule rejected this request\n"),
            });
        }
        Some(RuleAction::MapLocal(target)) => {
            let response = build_map_local_response(&target, &request)?;
            return Ok(RequestPlan::Respond { request, response });
        }
        Some(RuleAction::MapRemote(target)) => {
            let target = parse_remote_target(&target)?;
            request.path = combine_remote_path(&target.path_prefix, &request.path);
            return Ok(RequestPlan::Forward {
                request,
                target,
                response_breakpoint: false,
            });
        }
        Some(RuleAction::Breakpoint(target)) => {
            response_breakpoint =
                matches!(target, BreakpointTarget::Response | BreakpointTarget::Both);
            if matches!(target, BreakpointTarget::Request | BreakpointTarget::Both) {
                match context
                    .hooks
                    .on_breakpoint(request.as_intercepted(), target)
                    .await
                {
                    BreakpointDecision::Proceed => {}
                    BreakpointDecision::Modify(modified) => request.apply_intercepted(*modified),
                    BreakpointDecision::Drop => {
                        return Ok(RequestPlan::Respond {
                            request,
                            response: text_response(
                                403,
                                "ProxyBot breakpoint dropped this request\n",
                            ),
                        });
                    }
                }
            }
        }
    }

    Ok(RequestPlan::Forward {
        target: UpstreamTarget {
            scheme: request.scheme.clone(),
            host: request.host.clone(),
            port: request.port,
            path_prefix: String::new(),
            mapped: false,
            connect_override: None,
        },
        request,
        response_breakpoint,
    })
}

async fn apply_response_breakpoint(
    context: &RuntimeContext,
    request: &RuntimeRequest,
    response: &mut RuntimeResponse,
) {
    let mut capture = request.as_intercepted();
    capture.status = Some(response.status);
    capture.resp_headers = response.headers.clone();
    capture.resp_body = body_to_string(&response.body);
    match context
        .hooks
        .on_breakpoint(capture, BreakpointTarget::Response)
        .await
    {
        BreakpointDecision::Proceed => {}
        BreakpointDecision::Drop => *response = text_response(403, "ProxyBot response dropped\n"),
        BreakpointDecision::Modify(modified) => {
            if let Some(status) = modified.status {
                response.status = status;
            }
            response.headers = modified.resp_headers;
            response.body = modified.resp_body.unwrap_or_default().into_bytes();
        }
    }
}

async fn forward_to_upstream(
    context: &RuntimeContext,
    request: &RuntimeRequest,
    target: &UpstreamTarget,
) -> Result<(BoxedIo, Vec<u8>, SocketAddr), RuntimeError> {
    let address = if let Some(address) = target.connect_override {
        address
    } else {
        resolve_target(&target.host, target.port).await?
    };
    let tcp = TcpStream::connect(address)
        .await
        .map_err(|error| RuntimeError::Io(format!("connect to {address} failed: {error}")))?;
    let mut stream: BoxedIo = if target.scheme == "https" {
        let config = build_client_config(context.config.upstream_tls)?;
        let server_name = ServerName::try_from(target.host.clone())
            .map_err(|error| RuntimeError::Tls(error.to_string()))?;
        let tls = TlsConnector::from(Arc::new(config))
            .connect(server_name, tcp)
            .await
            .map_err(|error| RuntimeError::Tls(format!("upstream TLS failed: {error}")))?;
        Box::new(tls)
    } else {
        Box::new(tcp)
    };

    let path = combine_remote_path(&target.path_prefix, &request.path);
    let bytes = build_upstream_request(
        &request.method,
        &path,
        &target.host,
        &request.headers,
        &request.body,
    );
    stream
        .write_all(&bytes)
        .await
        .map_err(|error| RuntimeError::Io(error.to_string()))?;
    let response = read_http_message(
        &mut stream,
        Vec::new(),
        MessageKind::Response,
        &context.config,
    )
    .await?;
    Ok((stream, response, address))
}

async fn emit_capture(
    context: &RuntimeContext,
    request: &RuntimeRequest,
    response: &RuntimeResponse,
    response_size: usize,
    latency: Duration,
    is_websocket: bool,
) {
    let encoding = header_value(&response.headers, "content-encoding").unwrap_or("");
    let decoded = body::decompress(encoding, &response.body);
    let capture = InterceptedRequest {
        id: request.id.clone(),
        timestamp: request.timestamp.clone(),
        method: request.method.clone(),
        host: request.host.clone(),
        path: request.path.clone(),
        query_params: extract_query_params(&request.path),
        status: Some(response.status),
        latency_ms: Some(latency.as_millis() as u64),
        scheme: request.scheme.clone(),
        req_headers: request.headers.clone(),
        req_body: body_to_string(&request.body),
        resp_headers: response.headers.clone(),
        resp_body: body_to_string(&decoded),
        resp_size: Some(response_size),
        client_ip: Some(request.client_ip.to_string()),
        upstream_ip: request.upstream_ip.map(|ip| ip.to_string()),
        is_websocket,
        ..InterceptedRequest::default()
    };
    let _ = context
        .events
        .send(CaptureEvent::Completed(Box::new(capture)))
        .await;
}

trait AsyncIo: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> AsyncIo for T where T: AsyncRead + AsyncWrite + Unpin + Send {}
type BoxedIo = Box<dyn AsyncIo>;

async fn pipe_tunnel<C, U>(
    context: &RuntimeContext,
    host: &str,
    client: &mut C,
    upstream: &mut U,
) -> Result<(), RuntimeError>
where
    C: AsyncRead + AsyncWrite + Unpin + Send,
    U: AsyncRead + AsyncWrite + Unpin + Send,
{
    let mut client_buf = vec![0u8; 16 * 1024];
    let mut upstream_buf = vec![0u8; 16 * 1024];
    loop {
        tokio::select! {
            read = client.read(&mut client_buf) => {
                let size = read.map_err(|error| RuntimeError::Io(error.to_string()))?;
                if size == 0 { return Ok(()); }
                if apply_traffic_effect(context, host, TrafficDirection::ClientToUpstream, size).await {
                    upstream.write_all(&client_buf[..size]).await.map_err(|error| RuntimeError::Io(error.to_string()))?;
                }
            }
            read = upstream.read(&mut upstream_buf) => {
                let size = read.map_err(|error| RuntimeError::Io(error.to_string()))?;
                if size == 0 { return Ok(()); }
                if apply_traffic_effect(context, host, TrafficDirection::UpstreamToClient, size).await {
                    client.write_all(&upstream_buf[..size]).await.map_err(|error| RuntimeError::Io(error.to_string()))?;
                }
            }
        }
    }
}

async fn pipe_websocket<C, U>(
    context: &RuntimeContext,
    host: &str,
    request_id: &str,
    client: &mut C,
    upstream: &mut U,
) -> Result<(), RuntimeError>
where
    C: AsyncRead + AsyncWrite + Unpin + Send,
    U: AsyncRead + AsyncWrite + Unpin + Send,
{
    let mut client_buf = vec![0u8; 64 * 1024];
    let mut upstream_buf = vec![0u8; 64 * 1024];
    let mut client_pending = Vec::new();
    let mut upstream_pending = Vec::new();
    loop {
        tokio::select! {
            read = client.read(&mut client_buf) => {
                let size = read.map_err(|error| RuntimeError::Io(error.to_string()))?;
                if size == 0 { return Ok(()); }
                if apply_traffic_effect(context, host, TrafficDirection::ClientToUpstream, size).await {
                    upstream.write_all(&client_buf[..size]).await.map_err(|error| RuntimeError::Io(error.to_string()))?;
                }
                client_pending.extend_from_slice(&client_buf[..size]);
                emit_frames(context, request_id, "outgoing", &mut client_pending).await;
            }
            read = upstream.read(&mut upstream_buf) => {
                let size = read.map_err(|error| RuntimeError::Io(error.to_string()))?;
                if size == 0 { return Ok(()); }
                if apply_traffic_effect(context, host, TrafficDirection::UpstreamToClient, size).await {
                    client.write_all(&upstream_buf[..size]).await.map_err(|error| RuntimeError::Io(error.to_string()))?;
                }
                upstream_pending.extend_from_slice(&upstream_buf[..size]);
                emit_frames(context, request_id, "incoming", &mut upstream_pending).await;
            }
        }
    }
}

async fn apply_traffic_effect(
    context: &RuntimeContext,
    host: &str,
    direction: TrafficDirection,
    byte_count: usize,
) -> bool {
    let effect = context.hooks.traffic_effect(host, direction, byte_count);
    if !effect.delay.is_zero() {
        tokio::time::sleep(effect.delay).await;
    }
    !effect.drop
}

async fn emit_frames(
    context: &RuntimeContext,
    request_id: &str,
    direction: &str,
    pending: &mut Vec<u8>,
) {
    while let Some((header, total)) = parse_ws_frame_header(pending) {
        let payload = decode_ws_payload(&pending[..total], &header);
        let max = 256 * 1024;
        let truncated = payload.len() > max;
        let payload = if truncated { &payload[..max] } else { &payload };
        let frame = WsFrame {
            direction: direction.to_owned(),
            timestamp: timestamp_now(),
            payload: String::from_utf8_lossy(payload).to_string(),
            size: header.payload_len,
            opcode: header.opcode,
            truncated,
        };
        let _ = context
            .events
            .send(CaptureEvent::Frame {
                request_id: request_id.to_owned(),
                frame,
            })
            .await;
        pending.drain(..total);
    }
}

#[derive(Clone, Copy)]
enum MessageKind {
    Request,
    Response,
}

async fn read_http_message<S>(
    stream: &mut S,
    mut data: Vec<u8>,
    kind: MessageKind,
    config: &RuntimeConfig,
) -> Result<Vec<u8>, RuntimeError>
where
    S: AsyncRead + Unpin + ?Sized,
{
    let read = async {
        let mut buffer = vec![0u8; 16 * 1024];
        loop {
            if message_is_complete(&data, kind) {
                return Ok(data);
            }
            if data.len() >= config.max_message_bytes {
                return Err(RuntimeError::Protocol(format!(
                    "HTTP message exceeds {} bytes",
                    config.max_message_bytes
                )));
            }
            let size = stream
                .read(&mut buffer)
                .await
                .map_err(|error| RuntimeError::Io(error.to_string()))?;
            if size == 0 {
                return if data.is_empty() {
                    Err(RuntimeError::Protocol(
                        "connection closed before HTTP message".to_owned(),
                    ))
                } else {
                    Ok(data)
                };
            }
            data.extend_from_slice(&buffer[..size]);
        }
    };
    tokio::time::timeout(config.io_timeout, read)
        .await
        .map_err(|_| RuntimeError::Io("timed out reading HTTP message".to_owned()))?
}

fn message_is_complete(data: &[u8], kind: MessageKind) -> bool {
    let Some(header_end) = find_header_end(data) else {
        return false;
    };
    let headers = parse_header_lines(&data[..header_end]);
    if let Some(length) =
        header_value(&headers, "content-length").and_then(|value| value.parse::<usize>().ok())
    {
        return data.len() >= header_end + 4 + length;
    }
    if header_value(&headers, "transfer-encoding")
        .is_some_and(|value| value.to_ascii_lowercase().contains("chunked"))
    {
        return data[header_end + 4..]
            .windows(5)
            .any(|window| window == b"0\r\n\r\n");
    }
    match kind {
        MessageKind::Request => true,
        MessageKind::Response => {
            let status = parse_status(data).unwrap_or_default();
            status == 101 || status == 204 || status == 304
        }
    }
}

struct ParsedRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn parse_http_request(data: &[u8]) -> Option<ParsedRequest> {
    let header_end = find_header_end(data)?;
    let first_line_end = data.windows(2).position(|window| window == b"\r\n")?;
    let first_line = String::from_utf8_lossy(&data[..first_line_end]);
    let mut parts = first_line.split_whitespace();
    let method = parts.next()?.to_owned();
    let path = parts.next()?.to_owned();
    parts.next()?;
    Some(ParsedRequest {
        method,
        path,
        headers: parse_header_lines(&data[..header_end]),
        body: data[header_end + 4..].to_vec(),
    })
}

fn parse_http_response(data: &[u8]) -> Option<RuntimeResponse> {
    let header_end = find_header_end(data)?;
    Some(RuntimeResponse {
        status: parse_status(data)?,
        headers: parse_header_lines(&data[..header_end]),
        body: data[header_end + 4..].to_vec(),
    })
}

fn parse_status(data: &[u8]) -> Option<u16> {
    let first_line_end = data.windows(2).position(|window| window == b"\r\n")?;
    String::from_utf8_lossy(&data[..first_line_end])
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_header_lines(header: &[u8]) -> Vec<(String, String)> {
    String::from_utf8_lossy(header)
        .split("\r\n")
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect()
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn build_upstream_request(
    method: &str,
    path: &str,
    host: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Vec<u8> {
    let mut output = format!("{method} {path} HTTP/1.1\r\nHost: {host}\r\n").into_bytes();
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("host")
            || name.eq_ignore_ascii_case("connection")
            || name.eq_ignore_ascii_case("proxy-connection")
            || name.eq_ignore_ascii_case("content-length")
            || name.eq_ignore_ascii_case("transfer-encoding")
        {
            continue;
        }
        output.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    output.extend_from_slice(b"Connection: close\r\n");
    if !body.is_empty() {
        output.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    }
    output.extend_from_slice(b"\r\n");
    output.extend_from_slice(body);
    output
}

fn build_http_response(response: &RuntimeResponse) -> Vec<u8> {
    let mut output = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status,
        http_reason(response.status)
    )
    .into_bytes();
    for (name, value) in &response.headers {
        if name.eq_ignore_ascii_case("content-length")
            || name.eq_ignore_ascii_case("transfer-encoding")
            || name.eq_ignore_ascii_case("connection")
        {
            continue;
        }
        output.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    output.extend_from_slice(format!("Content-Length: {}\r\n", response.body.len()).as_bytes());
    output.extend_from_slice(b"Connection: close\r\n\r\n");
    output.extend_from_slice(&response.body);
    output
}

fn text_response(status: u16, body: &str) -> RuntimeResponse {
    RuntimeResponse {
        status,
        headers: vec![(
            "Content-Type".to_owned(),
            "text/plain; charset=utf-8".to_owned(),
        )],
        body: body.as_bytes().to_vec(),
    }
}

fn build_map_local_response(
    target: &str,
    request: &RuntimeRequest,
) -> Result<RuntimeResponse, RuntimeError> {
    if target.trim().is_empty() {
        return Err(RuntimeError::Protocol(
            "MAPLOCAL target is empty".to_owned(),
        ));
    }
    let path = expand_user_path(target);
    let raw = std::fs::read(&path).map_err(|error| {
        RuntimeError::Io(format!(
            "failed to read MAPLOCAL target {}: {error}",
            path.display()
        ))
    })?;
    let text = String::from_utf8(raw.clone()).ok();
    if let Some(text) = text.as_deref() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
            if value.as_object().is_some_and(|object| {
                object.contains_key("status")
                    || object.contains_key("headers")
                    || object.contains_key("body")
            }) {
                let status = value
                    .get("status")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|status| u16::try_from(status).ok())
                    .unwrap_or(200);
                let headers = value
                    .get("headers")
                    .and_then(serde_json::Value::as_object)
                    .map(|headers| {
                        headers
                            .iter()
                            .map(|(name, value)| {
                                (
                                    name.clone(),
                                    value
                                        .as_str()
                                        .map(str::to_owned)
                                        .unwrap_or_else(|| value.to_string()),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let body = value
                    .get("body")
                    .map(|body| {
                        render_template(body.as_str().unwrap_or(&body.to_string()), request)
                            .into_bytes()
                    })
                    .unwrap_or_default();
                return Ok(RuntimeResponse {
                    status,
                    headers,
                    body,
                });
            }
        }
    }
    let body = text
        .map(|text| render_template(&text, request).into_bytes())
        .unwrap_or(raw);
    Ok(RuntimeResponse {
        status: 200,
        headers: vec![(
            "Content-Type".to_owned(),
            infer_content_type(&path).to_owned(),
        )],
        body,
    })
}

fn render_template(template: &str, request: &RuntimeRequest) -> String {
    template
        .replace("{{request.method}}", &request.method)
        .replace("{{request.host}}", &request.host)
        .replace("{{request.path}}", &request.path)
        .replace(
            "{{request.body}}",
            &body_to_string(&request.body).unwrap_or_default(),
        )
        .replace("{{timestamp}}", &timestamp_now())
        .replace("{{request.id}}", &request.id)
}

fn parse_remote_target(target: &str) -> Result<UpstreamTarget, RuntimeError> {
    let (scheme, rest) = target.split_once("://").ok_or_else(|| {
        RuntimeError::Protocol("MAPREMOTE target must start with http:// or https://".to_owned())
    })?;
    if scheme != "http" && scheme != "https" {
        return Err(RuntimeError::Protocol(
            "MAPREMOTE target only supports http and https".to_owned(),
        ));
    }
    let (authority, prefix) = rest
        .split_once('/')
        .map(|(authority, path)| (authority, format!("/{}", path.trim_end_matches('/'))))
        .unwrap_or((rest, String::new()));
    let (host, port) = parse_authority(authority, if scheme == "https" { 443 } else { 80 })?;
    Ok(UpstreamTarget {
        scheme: scheme.to_owned(),
        host,
        port,
        path_prefix: prefix,
        mapped: true,
        connect_override: None,
    })
}

fn parse_authority(authority: &str, default_port: u16) -> Result<(String, u16), RuntimeError> {
    if authority.starts_with('[') {
        let end = authority
            .find(']')
            .ok_or_else(|| RuntimeError::Protocol("invalid IPv6 authority".to_owned()))?;
        let host = authority[1..end].to_owned();
        let port = authority[end + 1..]
            .strip_prefix(':')
            .map(|port| port.parse())
            .transpose()
            .map_err(|_| RuntimeError::Protocol("invalid authority port".to_owned()))?
            .unwrap_or(default_port);
        return Ok((host, port));
    }
    if authority.matches(':').count() == 1 {
        if let Some((host, port)) = authority.rsplit_once(':') {
            let port = port
                .parse()
                .map_err(|_| RuntimeError::Protocol("invalid authority port".to_owned()))?;
            return Ok((host.to_owned(), port));
        }
    }
    if authority.is_empty() {
        Err(RuntimeError::Protocol("empty authority".to_owned()))
    } else {
        Ok((authority.to_owned(), default_port))
    }
}

async fn resolve_target(host: &str, port: u16) -> Result<SocketAddr, RuntimeError> {
    tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| RuntimeError::Io(format!("resolve {host}:{port} failed: {error}")))?
        .next()
        .ok_or_else(|| RuntimeError::Io(format!("no address for {host}:{port}")))
}

fn build_client_config(policy: UpstreamTlsPolicy) -> Result<ClientConfig, RuntimeError> {
    let builder = ClientConfig::builder_with_provider(runtime_crypto_provider())
        .with_safe_default_protocol_versions()
        .map_err(|error| RuntimeError::Tls(error.to_string()))?;
    match policy {
        UpstreamTlsPolicy::Verify => {
            let mut roots = RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            Ok(builder.with_root_certificates(roots).with_no_client_auth())
        }
        UpstreamTlsPolicy::DangerouslyAcceptInvalid => Ok(builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerification))
            .with_no_client_auth()),
    }
}

fn runtime_crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

#[derive(Debug)]
struct NoVerification;

impl rustls_danger::ServerCertVerifier for NoVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls_danger::ServerCertVerified, rustls::Error> {
        Ok(rustls_danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<rustls_danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls_danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<rustls_danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls_danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}

fn extract_sni_from_client_hello(data: &[u8]) -> Option<String> {
    if data.len() < 43 || data[0] != 0x16 || data[5] != 0x01 {
        return None;
    }
    let mut position = 43;
    let session_length = *data.get(position)? as usize;
    position += 1 + session_length;
    let cipher_length =
        u16::from_be_bytes([*data.get(position)?, *data.get(position + 1)?]) as usize;
    position += 2 + cipher_length;
    let compression_length = *data.get(position)? as usize;
    position += 1 + compression_length;
    let extensions_length =
        u16::from_be_bytes([*data.get(position)?, *data.get(position + 1)?]) as usize;
    position += 2;
    let end = (position + extensions_length).min(data.len());
    while position + 4 <= end {
        let kind = u16::from_be_bytes([data[position], data[position + 1]]);
        let length = u16::from_be_bytes([data[position + 2], data[position + 3]]) as usize;
        position += 4;
        if position + length > end {
            return None;
        }
        if kind == 0 {
            position += 2;
            while position + 3 <= end {
                let name_type = data[position];
                let name_length =
                    u16::from_be_bytes([data[position + 1], data[position + 2]]) as usize;
                position += 3;
                if position + name_length > end {
                    return None;
                }
                if name_type == 0 {
                    return String::from_utf8(data[position..position + name_length].to_vec()).ok();
                }
                position += name_length;
            }
            return None;
        }
        position += length;
    }
    None
}

struct WsHeader {
    opcode: u8,
    payload_len: usize,
    mask: Option<[u8; 4]>,
    header_len: usize,
}

fn parse_ws_frame_header(data: &[u8]) -> Option<(WsHeader, usize)> {
    if data.len() < 2 {
        return None;
    }
    let opcode = data[0] & 0x0f;
    let masked = data[1] & 0x80 != 0;
    let mut payload_len = (data[1] & 0x7f) as usize;
    let mut offset = 2;
    if payload_len == 126 {
        payload_len = u16::from_be_bytes([*data.get(2)?, *data.get(3)?]) as usize;
        offset = 4;
    } else if payload_len == 127 {
        payload_len =
            usize::try_from(u64::from_be_bytes(data.get(2..10)?.try_into().ok()?)).ok()?;
        offset = 10;
    }
    let mask = if masked {
        let mask = data.get(offset..offset + 4)?;
        offset += 4;
        Some(mask.try_into().ok()?)
    } else {
        None
    };
    let total = offset.checked_add(payload_len)?;
    if data.len() < total {
        return None;
    }
    Some((
        WsHeader {
            opcode,
            payload_len,
            mask,
            header_len: offset,
        },
        total,
    ))
}

fn decode_ws_payload(data: &[u8], header: &WsHeader) -> Vec<u8> {
    let mut payload = data[header.header_len..header.header_len + header.payload_len].to_vec();
    if let Some(mask) = header.mask {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }
    payload
}

fn is_ws_upgrade(headers: &[(String, String)]) -> bool {
    header_value(headers, "upgrade").is_some_and(|value| value.eq_ignore_ascii_case("websocket"))
}

fn extract_query_params(path: &str) -> Option<String> {
    path.split_once('?').map(|(_, query)| query.to_owned())
}

fn body_to_string(body: &[u8]) -> Option<String> {
    String::from_utf8(body.to_vec()).ok()
}

fn normalize_request_target(path: &str) -> String {
    if let Some((_, remainder)) = path.split_once("://") {
        remainder
            .split_once('/')
            .map(|(_, path)| format!("/{path}"))
            .unwrap_or_else(|| "/".to_owned())
    } else {
        path.to_owned()
    }
}

fn combine_remote_path(prefix: &str, path: &str) -> String {
    let path = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    if prefix.is_empty() {
        path
    } else {
        format!("{}{}", prefix.trim_end_matches('/'), path)
    }
}

fn expand_user_path(path: &str) -> PathBuf {
    path.strip_prefix("~/")
        .map(|rest| {
            PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_owned())).join(rest)
        })
        .unwrap_or_else(|| PathBuf::from(path))
}

fn infer_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => "application/json; charset=utf-8",
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("txt") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn http_reason(status: u16) -> &'static str {
    match status {
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

fn next_request_id() -> String {
    generate_request_id(REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// Build a stable request id from timestamp and a process-local counter.
pub fn generate_request_id(counter: u64) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("req-{nanos}-{counter}")
}

/// Build the wire timestamp used by Captured Requests and frames.
pub fn timestamp_now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| format!("{}.{:03}", duration.as_secs(), duration.subsec_millis()))
        .unwrap_or_else(|_| "0.000".to_owned())
}

impl fmt::Debug for RunningMitm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RunningMitm")
            .field("bound_addr", &self.bound_addr)
            .field("is_running", &self.is_running())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rule, RulePattern};
    use tempfile::tempdir;
    use tokio::sync::oneshot as tokio_oneshot;

    fn runtime_with_rules(rules: Vec<Rule>) -> (MitmRuntime, tempfile::TempDir) {
        let root = tempdir().unwrap();
        let certs = Arc::new(CertManager::new(Some(root.path().join("ca"))).unwrap());
        let engine = Arc::new(RulesEngine::with_dir(root.path().join("rules")));
        engine.set_rules(rules);
        let runtime = MitmRuntime::new(
            RuntimeConfig {
                bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
                ..RuntimeConfig::default()
            },
            certs,
            engine,
            Arc::new(RwLock::new(TlsRuleSet::default())),
        );
        (runtime, root)
    }

    async fn next_completed(events: &mut mpsc::Receiver<CaptureEvent>) -> InterceptedRequest {
        loop {
            match events.recv().await {
                Some(CaptureEvent::Completed(capture)) => return *capture,
                Some(_) => continue,
                None => panic!("capture stream closed before a completed request"),
            }
        }
    }

    async fn read_headers(stream: &mut TcpStream) -> Vec<u8> {
        let mut response = Vec::new();
        let mut byte = [0u8; 1];
        while !response.ends_with(b"\r\n\r\n") {
            assert_eq!(stream.read(&mut byte).await.unwrap(), 1);
            response.push(byte[0]);
        }
        response
    }

    #[tokio::test]
    async fn start_reports_bound_address_and_shutdown_releases_it() {
        let (runtime, _root) = runtime_with_rules(Vec::new());
        let mut running = runtime.start().await.unwrap();
        let address = running.bound_addr();
        let mut events = running.take_events().unwrap();
        assert!(matches!(
            events.recv().await,
            Some(CaptureEvent::Started { bound_addr }) if bound_addr == address
        ));
        assert!(running.is_running());
        running.shutdown().await;
        assert!(matches!(
            events.recv().await,
            Some(CaptureEvent::Stopped { .. })
        ));
        let rebound = TcpListener::bind(address).await.unwrap();
        drop(rebound);
    }

    #[tokio::test]
    async fn shutdown_aborts_open_connection_tasks() {
        let (runtime, _root) = runtime_with_rules(Vec::new());
        let mut running = runtime.start().await.unwrap();
        let mut events = running.take_events().unwrap();
        let _ = events.recv().await;
        let _client = TcpStream::connect(running.bound_addr()).await.unwrap();
        assert!(matches!(
            events.recv().await,
            Some(CaptureEvent::ConnectionOpened { .. })
        ));

        running.shutdown().await;
        assert!(matches!(
            events.recv().await,
            Some(CaptureEvent::Stopped {
                aborted_connections: 1
            })
        ));
    }

    #[tokio::test]
    async fn start_returns_bind_error_before_reporting_success() {
        let occupied = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = occupied.local_addr().unwrap();
        let (mut runtime, _root) = runtime_with_rules(Vec::new());
        runtime.config.bind_addr = address;
        assert!(matches!(
            runtime.start().await,
            Err(RuntimeError::Bind { .. })
        ));
    }

    #[tokio::test]
    async fn routing_rule_rejects_http_and_emits_one_capture() {
        let rule = Rule {
            pattern: RulePattern::Domain,
            value: "example.test".to_owned(),
            action: RuleAction::Reject,
            name: "reject".to_owned(),
            priority: 1,
            enabled: true,
            comment: String::new(),
        };
        let (runtime, _root) = runtime_with_rules(vec![rule]);
        let mut running = runtime.start().await.unwrap();
        let mut events = running.take_events().unwrap();
        assert!(matches!(
            events.recv().await,
            Some(CaptureEvent::Started { .. })
        ));
        let mut client = TcpStream::connect(running.bound_addr()).await.unwrap();
        client
            .write_all(b"GET /private HTTP/1.1\r\nHost: example.test\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        assert!(response.starts_with(b"HTTP/1.1 403"));
        let capture = next_completed(&mut events).await;
        assert_eq!(capture.host, "example.test");
        assert_eq!(capture.status, Some(403));
        running.shutdown().await;
    }

    #[tokio::test]
    async fn routing_rule_rejects_decrypted_https_through_the_same_pipeline() {
        let rule = Rule {
            pattern: RulePattern::Domain,
            value: "localhost".to_owned(),
            action: RuleAction::Reject,
            name: "reject TLS".to_owned(),
            priority: 1,
            enabled: true,
            comment: String::new(),
        };
        let (runtime, _root) = runtime_with_rules(vec![rule]);
        let ca_pem = runtime.certs.get_ca_cert_pem();
        let mut running = runtime.start().await.unwrap();
        let mut events = running.take_events().unwrap();
        let _ = events.recv().await;

        let mut client = TcpStream::connect(running.bound_addr()).await.unwrap();
        client
            .write_all(b"CONNECT localhost:9 HTTP/1.1\r\nHost: localhost:9\r\n\r\n")
            .await
            .unwrap();
        assert!(read_headers(&mut client).await.starts_with(b"HTTP/1.1 200"));

        let mut roots = RootCertStore::empty();
        for certificate in rustls_pemfile::certs(&mut std::io::Cursor::new(ca_pem.as_bytes())) {
            roots.add(certificate.unwrap()).unwrap();
        }
        let tls_config = ClientConfig::builder_with_provider(runtime_crypto_provider())
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let server_name = ServerName::try_from("localhost".to_owned()).unwrap();
        let mut tls = TlsConnector::from(Arc::new(tls_config))
            .connect(server_name, client)
            .await
            .unwrap();
        tls.write_all(b"GET /private HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = Vec::new();
        tls.read_to_end(&mut response).await.unwrap();
        assert!(response.starts_with(b"HTTP/1.1 403"));

        let capture = next_completed(&mut events).await;
        assert_eq!(capture.scheme, "https");
        assert_eq!(capture.status, Some(403));
        assert!(capture.upstream_ip.is_some());
        running.shutdown().await;
    }

    struct RewritingHooks;

    #[async_trait]
    impl RuntimeHooks for RewritingHooks {
        async fn on_request(&self, request: &mut RuntimeRequest) -> RuntimeHookDecision {
            request.path = "/rewritten".to_owned();
            RuntimeHookDecision::Continue
        }

        async fn on_response(&self, _request: &RuntimeRequest, response: &mut RuntimeResponse) {
            response.status = 201;
            response.body = b"rewritten response".to_vec();
        }
    }

    #[tokio::test]
    async fn hooks_mutate_the_bytes_forwarded_in_both_directions() {
        let origin = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let origin_addr = origin.local_addr().unwrap();
        let (request_tx, request_rx) = tokio_oneshot::channel();
        let origin_task = tokio::spawn(async move {
            let (mut stream, _) = origin.accept().await.unwrap();
            let request = read_http_message(
                &mut stream,
                Vec::new(),
                MessageKind::Request,
                &RuntimeConfig::default(),
            )
            .await
            .unwrap();
            let _ = request_tx.send(request);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\noriginal",
                )
                .await
                .unwrap();
        });

        let (runtime, _root) = runtime_with_rules(Vec::new());
        let runtime = runtime.with_hooks(Arc::new(RewritingHooks));
        let mut running = runtime.start().await.unwrap();
        let mut events = running.take_events().unwrap();
        let _ = events.recv().await;
        let mut client = TcpStream::connect(running.bound_addr()).await.unwrap();
        let request =
            format!("GET /original HTTP/1.1\r\nHost: {origin_addr}\r\nConnection: close\r\n\r\n");
        client.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();

        let forwarded = request_rx.await.unwrap();
        assert!(forwarded.starts_with(b"GET /rewritten HTTP/1.1"));
        assert!(response.starts_with(b"HTTP/1.1 201"));
        assert!(response.ends_with(b"rewritten response"));
        let capture = next_completed(&mut events).await;
        assert_eq!(capture.path, "/rewritten");
        assert_eq!(capture.status, Some(201));
        assert_eq!(capture.upstream_ip.as_deref(), Some("127.0.0.1"));

        origin_task.await.unwrap();
        running.shutdown().await;
    }

    struct BlockingConnectHook;

    #[async_trait]
    impl RuntimeHooks for BlockingConnectHook {
        async fn on_connect(&self, _host: &str, _port: u16) -> RuntimeConnectDecision {
            RuntimeConnectDecision::Block
        }
    }

    #[tokio::test]
    async fn connect_hook_can_block_before_dns_resolution() {
        let (runtime, _root) = runtime_with_rules(Vec::new());
        let runtime = runtime.with_hooks(Arc::new(BlockingConnectHook));
        let running = runtime.start().await.unwrap();
        let mut client = TcpStream::connect(running.bound_addr()).await.unwrap();
        client
            .write_all(
                b"CONNECT deliberately-unresolvable.invalid:443 HTTP/1.1\r\nHost: deliberately-unresolvable.invalid\r\n\r\n",
            )
            .await
            .unwrap();
        let response = read_headers(&mut client).await;
        assert!(response.starts_with(b"HTTP/1.1 403"));
        running.shutdown().await;
    }

    #[test]
    fn message_codec_and_remote_target_are_deterministic() {
        let request = b"POST /x HTTP/1.1\r\nHost: example.com\r\nContent-Length: 3\r\n\r\nabc";
        assert!(message_is_complete(request, MessageKind::Request));
        let parsed = parse_http_request(request).unwrap();
        assert_eq!(parsed.body, b"abc");
        let target = parse_remote_target("https://api.example.com:8443/v1").unwrap();
        assert_eq!(target.host, "api.example.com");
        assert_eq!(target.port, 8443);
        assert_eq!(
            combine_remote_path(&target.path_prefix, "/users"),
            "/v1/users"
        );
    }
}
