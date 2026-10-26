//! Local web UI and JSON API.

use crate::rules::Rules;
use crate::store::{HarExport, ListFilter, RequestSummary, Store, TlsHost, WsFrameRecord, WsHost};
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

const INDEX_HTML: &str = include_str!("../web/index.html");
const CA_PAGE_HTML: &str = include_str!("../web/ca.html");

#[derive(Clone)]
pub struct WebState {
    pub store: Arc<Store>,
    pub events: broadcast::Sender<String>,
    pub rules: Arc<RwLock<Rules>>,
    pub rules_path: PathBuf,
    pub ca_pem: Arc<String>,
    pub setup: Arc<SetupInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupInfo {
    pub lan_ip: String,
    pub quic_policy: String,
    pub proxy_port: u16,
    pub transparent_port: u16,
    pub web_port: u16,
    pub ca_port: u16,
    pub ca_url: String,
    pub ca_page_url: String,
    pub gateway_active: bool,
}

pub async fn serve(state: WebState, addr: SocketAddr) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index))
        .route("/ca.crt", get(ca_cert))
        .route("/api/setup", get(setup))
        .route("/api/requests", get(list))
        .route("/api/requests/{id}", get(detail))
        .route("/api/stream", get(stream))
        .route("/api/ws/hosts", get(ws_hosts))
        .route("/api/ws", get(ws_frames))
        .route("/api/tls/hosts", get(tls_hosts))
        .route("/api/rules", get(get_rules).put(put_rules))
        .route("/api/har", get(har))
        .route("/api/clear", post(clear))
        .route("/api/qr.svg", get(qr))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "web UI listening");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Minimal HTTP server reachable from the test device on the LAN. It only
/// serves the CA download page and certificate; the main UI stays on loopback.
pub async fn serve_ca_page(state: WebState, addr: SocketAddr) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(ca_page))
        .route("/ca.crt", get(ca_cert))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "device CA page listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn ca_page(State(state): State<WebState>) -> Html<String> {
    let setup = &state.setup;
    let body = CA_PAGE_HTML
        .replace("{{PROXY}}", &format!("{}:{}", setup.lan_ip, setup.proxy_port))
        .replace("{{CA_URL}}", &setup.ca_url)
        .replace("{{GATEWAY}}", &setup.lan_ip);
    Html(body)
}

async fn ca_cert(State(state): State<WebState>) -> Response {
    (
        [
            (
                header::CONTENT_TYPE,
                "application/x-x509-ca-cert; charset=utf-8",
            ),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"proxybot-ca.crt\"",
            ),
        ],
        state.ca_pem.as_bytes().to_vec(),
    )
        .into_response()
}

async fn setup(State(state): State<WebState>) -> Json<SetupInfo> {
    Json((*state.setup).clone())
}

async fn list(
    State(state): State<WebState>,
    Query(filter): Query<ListFilter>,
) -> Result<Json<Vec<RequestSummary>>, (StatusCode, String)> {
    state
        .store
        .list(&filter)
        .map(Json)
        .map_err(internal_error)
}

async fn detail(
    State(state): State<WebState>,
    Path(id): Path<i64>,
) -> Result<Response, (StatusCode, String)> {
    match state.store.detail(id).map_err(internal_error)? {
        Some(detail) => Ok(Json(detail).into_response()),
        None => Err((StatusCode::NOT_FOUND, "request not found".into())),
    }
}

async fn stream(
    State(state): State<WebState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.events.subscribe();
    let stream = BroadcastStream::new(receiver).filter_map(|item| {
        item.ok()
            .map(|message| Ok(Event::default().event("request").data(message)))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn get_rules(State(state): State<WebState>) -> Response {
    let rules = state.rules.read().await;
    (
        [(header::CONTENT_TYPE, "text/yaml; charset=utf-8")],
        rules.to_yaml(),
    )
        .into_response()
}

async fn put_rules(State(state): State<WebState>, body: String) -> Result<Response, (StatusCode, String)> {
    let parsed = Rules::parse(&body)
        .map_err(|error| (StatusCode::BAD_REQUEST, format!("invalid rules: {error}")))?;
    std::fs::write(&state.rules_path, &body).map_err(|error| internal_error(error.into()))?;
    *state.rules.write().await = parsed;
    Ok((StatusCode::OK, "rules updated").into_response())
}

async fn har(
    State(state): State<WebState>,
    Query(filter): Query<ListFilter>,
) -> Result<Response, (StatusCode, String)> {
    let har: HarExport = state.store.har(&filter).map_err(internal_error)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"proxybot.har\"",
            ),
        ],
        Json(har),
    )
        .into_response())
}

async fn clear(State(state): State<WebState>) -> Result<StatusCode, (StatusCode, String)> {
    state.store.clear().map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct WsQuery {
    host: Option<String>,
    limit: Option<usize>,
}

async fn tls_hosts(
    State(state): State<WebState>,
) -> Result<Json<Vec<TlsHost>>, (StatusCode, String)> {
    state.store.tls_hosts().map(Json).map_err(internal_error)
}

async fn ws_hosts(
    State(state): State<WebState>,
) -> Result<Json<Vec<WsHost>>, (StatusCode, String)> {
    state.store.ws_hosts().map(Json).map_err(internal_error)
}

async fn ws_frames(
    State(state): State<WebState>,
    Query(query): Query<WsQuery>,
) -> Result<Json<Vec<WsFrameRecord>>, (StatusCode, String)> {
    let host = query.host.unwrap_or_default();
    if host.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let limit = query.limit.unwrap_or(500).clamp(1, 2000);
    state
        .store
        .ws_frames(&host, limit)
        .map(Json)
        .map_err(internal_error)
}

#[derive(Deserialize)]
struct QrQuery {
    data: Option<String>,
}

async fn qr(State(state): State<WebState>, Query(query): Query<QrQuery>) -> Result<Response, (StatusCode, String)> {
    let data = query.data.unwrap_or_else(|| state.setup.ca_page_url.clone());
    let code = qrcode::QrCode::new(data.as_bytes())
        .map_err(|error| (StatusCode::BAD_REQUEST, format!("qr error: {error}")))?;
    let svg = code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(200, 200)
        .dark_color(qrcode::render::svg::Color("#e6edf3"))
        .light_color(qrcode::render::svg::Color("#0d1117"))
        .build();
    Ok((
        [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
        svg,
    )
        .into_response())
}

fn internal_error(error: anyhow::Error) -> (StatusCode, String) {
    tracing::error!(%error, "web API error");
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
