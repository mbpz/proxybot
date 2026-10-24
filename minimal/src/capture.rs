//! Capture handler: turns hudsucker HTTP exchanges and WebSocket frames into
//! persisted records and live web events.

use crate::rules::Rules;
use crate::store::{Captured, RequestSummary, Store, WsFrameInput, MAX_BODY_BYTES};
use hudsucker::hyper::body::{Body as HttpBody, Bytes, Frame, SizeHint};
use hudsucker::hyper::header::{ACCEPT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, HOST};
use hudsucker::hyper::{Request, Response};
use hudsucker::tokio_tungstenite::tungstenite::Message;
use hudsucker::{
    Body, HttpContext, HttpHandler, RequestOrResponse, WebSocketContext, WebSocketHandler,
};
use http_body_util::{BodyExt, Full};
use std::io::Read;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, RwLock};

#[derive(Clone)]
pub struct CaptureState {
    pub store: Arc<Store>,
    pub events: broadcast::Sender<String>,
    pub rules: Arc<RwLock<Rules>>,
    /// Host suffixes that are forwarded without TLS interception (pinned apps).
    pub passthrough: Arc<RwLock<Vec<String>>>,
}

pub struct CaptureHandler {
    state: Arc<CaptureState>,
    pending: Option<Pending>,
}

struct Pending {
    ts_ms: i64,
    client_ip: Option<String>,
    scheme: String,
    method: String,
    host: String,
    path: String,
    app: Option<String>,
    app_source: Option<String>,
    req_headers: Vec<(String, String)>,
    req_body: Option<Vec<u8>>,
    req_truncated: bool,
    started: Instant,
}

impl Clone for CaptureHandler {
    fn clone(&self) -> Self {
        // hudsucker calls clone once per connection; each clone owns its pending
        // request slot so request/response correlation stays per connection.
        Self {
            state: self.state.clone(),
            pending: None,
        }
    }
}

impl CaptureHandler {
    pub fn new(state: Arc<CaptureState>) -> Self {
        Self {
            state,
            pending: None,
        }
    }
}

impl HttpHandler for CaptureHandler {
    fn should_intercept_tls(
        &mut self,
        ctx: &HttpContext,
        client_hello: hudsucker::rustls::server::ClientHello<'_>,
    ) -> impl std::future::Future<Output = bool> + Send {
        let state = self.state.clone();
        let host = client_hello.server_name().map(str::to_string);
        let client_ip = ctx.client_addr.ip().to_string();
        async move {
            let Some(host) = host else {
                return true;
            };
            let passthrough = {
                let list = state.passthrough.read().await;
                is_passthrough(&list, &host)
            };
            if let Err(error) =
                state
                    .store
                    .insert_tls_attempt(now_ms(), &host, Some(&client_ip), passthrough)
            {
                tracing::warn!(%error, "failed to persist TLS attempt");
            }
            if passthrough {
                tracing::info!(%host, "TLS passthrough (pinned app or configured)");
                false
            } else {
                true
            }
        }
    }

    fn handle_request(
        &mut self,
        ctx: &HttpContext,
        mut req: Request<Body>,
    ) -> impl std::future::Future<Output = RequestOrResponse> + Send {
        let state = self.state.clone();
        let client_ip = ctx.client_addr.ip().to_string();
        let headers: Vec<(String, String)> = req
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    String::from_utf8_lossy(value.as_bytes()).into_owned(),
                )
            })
            .collect();
        let method = req.method().as_str().to_string();
        let (host, path, scheme) = request_target(&req);

        // Ask upstream for uncompressed bodies so the inspector shows JSON/text
        // rather than gzip. This is a normal MITM debugging trade-off.
        req.headers_mut().insert(
            ACCEPT_ENCODING,
            hudsucker::hyper::header::HeaderValue::from_static("identity"),
        );

        async move {
            let app = {
                let rules = state.rules.read().await;
                rules.classify(&host)
            };
            let (head, body) = req.into_parts();
            let (body, mut captured, truncated) = capture_body(body, &head.headers).await;
            let mut headers = headers;
            if let Some(bytes) = captured.as_mut() {
                if let Some(encoding) = content_encoding(&head.headers) {
                    if let Some(decoded) = decode_content(bytes, &encoding) {
                        *bytes = decoded;
                        headers.push(("x-proxybot-decoded-from".into(), encoding));
                    }
                }
            }
            let req = Request::from_parts(head, body);
            self.pending = Some(Pending {
                ts_ms: now_ms(),
                client_ip: Some(client_ip),
                scheme,
                method,
                host,
                path,
                app: app.as_ref().map(|(app, _)| app.clone()),
                app_source: app.map(|(_, source)| source.to_string()),
                req_headers: headers,
                req_body: captured,
                req_truncated: truncated,
                started: Instant::now(),
            });
            RequestOrResponse::Request(req)
        }
    }

    fn handle_response(
        &mut self,
        _ctx: &HttpContext,
        res: Response<Body>,
    ) -> impl std::future::Future<Output = Response<Body>> + Send {
        let pending = self.pending.take();
        let state = self.state.clone();

        async move {
            let Some(pending) = pending else {
                return res;
            };
            let status = res.status().as_u16();
            let resp_headers: Vec<(String, String)> = res
                .headers()
                .iter()
                .map(|(name, value)| {
                    (
                        name.as_str().to_string(),
                        String::from_utf8_lossy(value.as_bytes()).into_owned(),
                    )
                })
                .collect();
            let content_type = res
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            let (parts, body) = res.into_parts();
            let response_encoding = content_encoding(&parts.headers);

            // Tee every response body: forward bytes unchanged while buffering up
            // to MAX_BODY_BYTES for the inspector. This covers chunked responses
            // and unknown lengths without breaking large downloads.
            let (tx, mut rx) = mpsc::unbounded_channel::<BodyCapture>();
            let tee = TeeBody::new(body, MAX_BODY_BYTES, tx);
            let res = Response::from_parts(parts, Body::from(tee.boxed()));

            let duration_ms = Some(pending.started.elapsed().as_millis() as u64);
            let captured = Captured {
                ts_ms: pending.ts_ms,
                client_ip: pending.client_ip,
                scheme: pending.scheme,
                method: pending.method,
                host: pending.host,
                path: pending.path,
                status: Some(status),
                duration_ms,
                app: pending.app,
                app_source: pending.app_source,
                req_headers: pending.req_headers,
                req_body: pending.req_body,
                req_truncated: pending.req_truncated,
                resp_headers,
                resp_body: None,
                resp_truncated: false,
                content_type,
                error: None,
            };
            let stored_resp_headers = captured.resp_headers.clone();
            match state.store.insert(&captured) {
                Ok(summary) => {
                    if let Ok(json) = serde_json::to_string(&Envelope {
                        kind: "request",
                        request: summary.clone(),
                    }) {
                        let _ = state.events.send(json);
                    }
                    let store = state.store.clone();
                    let mut headers = stored_resp_headers.clone();
                    tokio::spawn(async move {
                        if let Some(capture) = rx.recv().await {
                            let mut bytes = capture.bytes;
                            if let Some(encoding) = response_encoding.as_deref() {
                                if let Some(decoded) = decode_content(&bytes, encoding) {
                                    bytes = decoded;
                                    headers.push((
                                        "x-proxybot-decoded-from".into(),
                                        encoding.to_string(),
                                    ));
                                }
                            }
                            if let Err(error) = store.update_response_body(
                                summary.id,
                                &bytes,
                                capture.truncated,
                                &headers,
                            ) {
                                tracing::warn!(%error, "failed to persist response body");
                            }
                        }
                    });
                }
                Err(error) => tracing::warn!(%error, "failed to persist captured request"),
            }

            res
        }
    }
}

impl WebSocketHandler for CaptureHandler {
    fn handle_message(
        &mut self,
        ctx: &WebSocketContext,
        message: Message,
    ) -> impl std::future::Future<Output = Option<Message>> + Send {
        let state = self.state.clone();
        let meta = ws_meta(ctx);
        let (opcode, size, encoding, payload, truncated) = ws_payload(&message);

        async move {
            if let Some(meta) = meta {
                let frame = WsFrameInput {
                    ts_ms: now_ms(),
                    host: meta.host.clone(),
                    client_ip: meta.client_ip,
                    direction: meta.direction.to_string(),
                    opcode,
                    size,
                    encoding,
                    payload,
                    truncated,
                };
                if let Err(error) = state.store.insert_ws_frame(&frame) {
                    tracing::warn!(%error, "failed to persist WebSocket frame");
                } else {
                    let event = serde_json::json!({
                        "type": "ws",
                        "host": frame.host,
                    });
                    let _ = state.events.send(event.to_string());
                }
            }
            Some(message)
        }
    }
}

struct BodyCapture {
    bytes: Vec<u8>,
    truncated: bool,
}

/// A body wrapper that forwards frames and records up to `cap` bytes.
struct TeeBody {
    // Mutex makes the body `Sync` (required by http-body-util's `boxed`), and
    // `Pin<Box<_>>` keeps the inner body pollable without an `Unpin` bound.
    inner: Mutex<Option<Pin<Box<Body>>>>,
    buffer: Vec<u8>,
    truncated: bool,
    cap: usize,
    sink: Option<mpsc::UnboundedSender<BodyCapture>>,
}

impl TeeBody {
    fn new(inner: Body, cap: usize, sink: mpsc::UnboundedSender<BodyCapture>) -> Self {
        Self {
            inner: Mutex::new(Some(Box::pin(inner))),
            buffer: Vec::new(),
            truncated: false,
            cap,
            sink: Some(sink),
        }
    }

    fn finish(&mut self) {
        if let Some(sink) = self.sink.take() {
            let _ = sink.send(BodyCapture {
                bytes: std::mem::take(&mut self.buffer),
                truncated: self.truncated,
            });
        }
    }
}

impl HttpBody for TeeBody {
    type Data = Bytes;
    type Error = hudsucker::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        let mut guard = this.inner.lock().expect("tee body mutex poisoned");
        let Some(inner) = guard.as_mut() else {
            return Poll::Ready(None);
        };
        let result = inner.as_mut().poll_frame(cx);
        match &result {
            Poll::Ready(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    let remaining = this.cap.saturating_sub(this.buffer.len());
                    if data.len() > remaining {
                        this.truncated = true;
                    }
                    this.buffer
                        .extend_from_slice(&data[..data.len().min(remaining)]);
                }
            }
            Poll::Ready(Some(Err(_))) | Poll::Ready(None) => {
                guard.take();
                drop(guard);
                this.finish();
            }
            Poll::Pending => {}
        }
        result
    }

    fn is_end_stream(&self) -> bool {
        let guard = self.inner.lock().expect("tee body mutex poisoned");
        guard
            .as_ref()
            .map(|inner| inner.is_end_stream())
            .unwrap_or(true)
    }

    fn size_hint(&self) -> SizeHint {
        let guard = self.inner.lock().expect("tee body mutex poisoned");
        guard
            .as_ref()
            .map(|inner| inner.size_hint())
            .unwrap_or_default()
    }
}

impl Drop for TeeBody {
    fn drop(&mut self) {
        self.finish();
    }
}

#[derive(serde::Serialize)]
struct Envelope<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    request: RequestSummary,
}

struct WsMeta {
    host: String,
    client_ip: Option<String>,
    direction: &'static str,
}

fn ws_meta(ctx: &WebSocketContext) -> Option<WsMeta> {
    match ctx {
        WebSocketContext::ClientToServer { src, dst, .. } => Some(WsMeta {
            host: dst.host().unwrap_or("unknown").to_string(),
            client_ip: Some(src.ip().to_string()),
            direction: "c2s",
        }),
        WebSocketContext::ServerToClient { src, dst, .. } => Some(WsMeta {
            host: src.host().unwrap_or("unknown").to_string(),
            client_ip: Some(dst.ip().to_string()),
            direction: "s2c",
        }),
    }
}

const WS_PAYLOAD_CAP: usize = 32 * 1024;

fn ws_payload(message: &Message) -> (u8, usize, String, String, bool) {
    match message {
        Message::Text(text) => {
            let raw = text.as_bytes();
            (
                1,
                raw.len(),
                "utf8".into(),
                String::from_utf8_lossy(&raw[..raw.len().min(WS_PAYLOAD_CAP)]).into_owned(),
                raw.len() > WS_PAYLOAD_CAP,
            )
        }
        Message::Binary(bytes) => (
            2,
            bytes.len(),
            "hex".into(),
            hex_prefix(bytes, WS_PAYLOAD_CAP),
            bytes.len() > WS_PAYLOAD_CAP,
        ),
        Message::Ping(bytes) => (
            9,
            bytes.len(),
            "hex".into(),
            hex_prefix(bytes, WS_PAYLOAD_CAP),
            bytes.len() > WS_PAYLOAD_CAP,
        ),
        Message::Pong(bytes) => (
            10,
            bytes.len(),
            "hex".into(),
            hex_prefix(bytes, WS_PAYLOAD_CAP),
            bytes.len() > WS_PAYLOAD_CAP,
        ),
        Message::Close(_) => (8, 0, "utf8".into(), String::new(), false),
        Message::Frame(frame) => {
            let payload = frame.payload();
            (
                0,
                payload.len(),
                "hex".into(),
                hex_prefix(payload, WS_PAYLOAD_CAP),
                payload.len() > WS_PAYLOAD_CAP,
            )
        }
    }
}

fn hex_prefix(bytes: &[u8], cap: usize) -> String {
    let bytes = &bytes[..bytes.len().min(cap)];
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn request_target(req: &Request<Body>) -> (String, String, String) {
    let host = req
        .uri()
        .host()
        .map(str::to_string)
        .or_else(|| {
            req.headers()
                .get(HOST)
                .and_then(|value| value.to_str().ok())
                .map(|value| value.split(':').next().unwrap_or(value).to_string())
        })
        .unwrap_or_default();
    let path = req
        .uri()
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());
    let scheme = req.uri().scheme_str().unwrap_or("https").to_string();
    (host, path, scheme)
}

async fn capture_body(body: Body, headers: &hudsucker::hyper::HeaderMap) -> (Body, Option<Vec<u8>>, bool) {
    let length = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    let should_capture = matches!(length, Some(len) if len > 0 && len <= MAX_BODY_BYTES);
    if !should_capture {
        return (body, None, false);
    }

    match body.collect().await {
        Ok(collected) => {
            let bytes: Bytes = collected.to_bytes();
            let truncated = bytes.len() > MAX_BODY_BYTES;
            let body_bytes = bytes.slice(..bytes.len().min(MAX_BODY_BYTES));
            (
                Body::from(Full::new(body_bytes.clone())),
                Some(body_bytes.to_vec()),
                truncated,
            )
        }
        Err(error) => {
            tracing::warn!(%error, "failed to buffer exchange body; forwarding empty body");
            (Body::empty(), None, true)
        }
    }
}

fn is_passthrough(list: &[String], host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    list.iter().any(|suffix| {
        let suffix = suffix.trim().trim_start_matches("*.").to_ascii_lowercase();
        !suffix.is_empty() && (host == suffix || host.ends_with(&format!(".{suffix}")))
    })
}

fn content_encoding(headers: &hudsucker::hyper::HeaderMap) -> Option<String> {
    headers
        .get(hudsucker::hyper::header::CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty() && value != "identity")
}

fn decode_content(bytes: &[u8], encoding: &str) -> Option<Vec<u8>> {
    const LIMIT: u64 = 4 * 1024 * 1024;
    let mut output = Vec::new();
    match encoding {
        "gzip" | "x-gzip" => {
            let mut decoder = flate2::read::GzDecoder::new(bytes);
            decoder.by_ref().take(LIMIT).read_to_end(&mut output).ok()?;
        }
        "deflate" => {
            let mut decoder = flate2::read::ZlibDecoder::new(bytes);
            decoder.by_ref().take(LIMIT).read_to_end(&mut output).ok()?;
        }
        "br" => {
            let mut decoder = brotli::Decompressor::new(bytes, 4096);
            decoder.by_ref().take(LIMIT).read_to_end(&mut output).ok()?;
        }
        _ => return None,
    }
    Some(output)
}
