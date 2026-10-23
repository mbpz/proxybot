//! Capture handler: turns hudsucker HTTP exchanges into persisted requests and
//! live web events.

use crate::rules::Rules;
use crate::store::{Captured, RequestSummary, Store, MAX_BODY_BYTES};
use hudsucker::hyper::body::Bytes;
use hudsucker::hyper::header::{ACCEPT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, HOST};
use hudsucker::hyper::{Request, Response};
use hudsucker::{Body, HttpContext, HttpHandler, RequestOrResponse};
use http_body_util::{BodyExt, Full};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};

#[derive(Clone)]
pub struct CaptureState {
    pub store: Arc<Store>,
    pub events: broadcast::Sender<String>,
    pub rules: Arc<RwLock<Rules>>,
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
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);

        // Ask upstream for uncompressed bodies so the inspector shows JSON/text
        // rather than gzip. This is a normal MITM debugging trade-off.
        req.headers_mut()
            .insert(ACCEPT_ENCODING, hudsucker::hyper::header::HeaderValue::from_static("identity"));

        async move {
            let app = {
                let rules = state.rules.read().await;
                rules.classify(&host)
            };
            let (head, body) = req.into_parts();
            let (body, captured, truncated) = capture_body(body, &head.headers).await;
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
            let _ = content_type;
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
            let (body, captured, truncated) = capture_body(body, &parts.headers).await;
            let res = Response::from_parts(parts, body);

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
                resp_body: captured,
                resp_truncated: truncated,
                content_type,
                error: None,
            };
            publish(&state, captured);
            res
        }
    }
}

fn publish(state: &CaptureState, captured: Captured) {
    match state.store.insert(&captured) {
        Ok(summary) => {
            if let Ok(json) = serde_json::to_string(&Envelope {
                kind: "request",
                request: summary,
            }) {
                let _ = state.events.send(json);
            }
        }
        Err(error) => tracing::warn!(%error, "failed to persist captured request"),
    }
}

#[derive(serde::Serialize)]
struct Envelope<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    request: RequestSummary,
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
    let scheme = req
        .uri()
        .scheme_str()
        .unwrap_or("https")
        .to_string();
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
