//! Packaged desktop acceptance Adapter.
//!
//! This mode launches the real Tauri composition root but drives the product
//! through an isolated workspace instead of UI automation. It proves that the
//! packaged executable can prepare a CA, decrypt a local HTTPS request through
//! the production Capture Session, persist the Captured Request, stop, restart,
//! and cleanly stop again without a browser, external network, or user data.

use crate::cert::CertManager;
use crate::db::{CapturedRequestQuery, CapturedRequestRecord, DbState};
use crate::proxy::MitmRuntimeState;
use proxybot_core::{Rule, RuleAction, RulePattern, RulesEngine};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore};
use serde::Serialize;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

const REPORT_FILENAME: &str = "desktop-acceptance.json";
const REQUEST_PATH: &str = "/proxybot-acceptance";

#[derive(Debug, Serialize)]
struct CapturedRequestEvidence {
    id: i64,
    method: String,
    scheme: String,
    host: String,
    path: String,
    status: Option<u16>,
}

impl From<CapturedRequestRecord> for CapturedRequestEvidence {
    fn from(request: CapturedRequestRecord) -> Self {
        Self {
            id: request.id,
            method: request.method,
            scheme: request.scheme,
            host: request.host,
            path: request.path,
            status: request.response_status,
        }
    }
}

#[derive(Debug, Serialize)]
struct DesktopAcceptanceReport {
    schema_version: u8,
    product_version: &'static str,
    ca_prepared: bool,
    first_proxy_addr: SocketAddr,
    captured_request: CapturedRequestEvidence,
    stopped_cleanly: bool,
    restart_proxy_addr: SocketAddr,
    restart_stopped_cleanly: bool,
}

pub(crate) fn start(app: AppHandle, workspace: PathBuf) {
    tauri::async_runtime::spawn(async move {
        let result = run_journey(&app, &workspace).await;
        let (exit_code, document) = match result {
            Ok(report) => (
                0,
                serde_json::to_value(report).expect("serialize acceptance report"),
            ),
            Err(error) => (
                1,
                serde_json::json!({
                    "schema_version": 1,
                    "product_version": env!("CARGO_PKG_VERSION"),
                    "error": error,
                }),
            ),
        };

        let report_path = workspace.join(REPORT_FILENAME);
        let encoded = serde_json::to_vec_pretty(&document).expect("serialize acceptance document");
        if let Err(error) = std::fs::write(&report_path, encoded) {
            eprintln!(
                "desktop acceptance: could not write {}: {error}",
                report_path.display()
            );
            app.exit(1);
            return;
        }

        if exit_code == 0 {
            println!("desktop acceptance: ok ({})", report_path.display());
        } else {
            eprintln!("desktop acceptance: failed ({})", report_path.display());
        }
        app.exit(exit_code);
    });
}

async fn run_journey(app: &AppHandle, workspace: &Path) -> Result<DesktopAcceptanceReport, String> {
    let config = app.state::<Arc<proxybot_core::AppConfig>>().inner().clone();
    if config.base_dir != workspace {
        return Err(format!(
            "acceptance workspace mismatch: configured {}, requested {}",
            config.base_dir.display(),
            workspace.display()
        ));
    }

    let certs = app.state::<Arc<CertManager>>().inner().clone();
    let ca_pem = certs.get_ca_cert_pem();
    if ca_pem.trim().is_empty() || !config.ca_cert_path.is_file() {
        return Err("CA preparation did not produce ca/ca.pem".to_owned());
    }

    // A deterministic reject rule lets the test prove HTTPS decryption without
    // depending on an upstream service or accepting an invalid origin cert.
    let rules = app.state::<Arc<RulesEngine>>().inner().clone();
    rules.set_rules(vec![Rule {
        pattern: RulePattern::Domain,
        value: "localhost".to_owned(),
        action: RuleAction::Reject,
        name: "desktop acceptance HTTPS request".to_owned(),
        priority: 0,
        enabled: true,
        comment: "isolated packaged-app acceptance".to_owned(),
    }]);

    crate::proxy::start_proxy_for_app(app).await?;
    let runtime = app.state::<Arc<MitmRuntimeState>>().inner().clone();
    let first_proxy_addr = runtime
        .bound_addr()
        .await
        .ok_or_else(|| "Capture Session reported started without a bound address".to_owned())?;

    issue_decrypted_request(first_proxy_addr, &ca_pem).await?;
    let db = app.state::<Arc<DbState>>().inner().clone();
    let captured_request = wait_for_captured_request(&db).await?.into();

    crate::proxy::stop_proxy_for_app(app).await?;
    let stopped_cleanly = !runtime.is_running().await;
    if !stopped_cleanly {
        return Err("Capture Session still reports running after stop".to_owned());
    }

    crate::proxy::start_proxy_for_app(app).await?;
    let restart_proxy_addr = runtime
        .bound_addr()
        .await
        .ok_or_else(|| "restarted Capture Session has no bound address".to_owned())?;
    crate::proxy::stop_proxy_for_app(app).await?;
    let restart_stopped_cleanly = !runtime.is_running().await;
    if !restart_stopped_cleanly {
        return Err("restarted Capture Session still reports running after stop".to_owned());
    }

    Ok(DesktopAcceptanceReport {
        schema_version: 1,
        product_version: env!("CARGO_PKG_VERSION"),
        ca_prepared: true,
        first_proxy_addr,
        captured_request,
        stopped_cleanly,
        restart_proxy_addr,
        restart_stopped_cleanly,
    })
}

async fn issue_decrypted_request(proxy_addr: SocketAddr, ca_pem: &str) -> Result<(), String> {
    let proxy_addr = SocketAddr::from(([127, 0, 0, 1], proxy_addr.port()));
    let mut client = tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(proxy_addr))
        .await
        .map_err(|_| format!("timed out connecting to proxy at {proxy_addr}"))?
        .map_err(|error| format!("could not connect to proxy at {proxy_addr}: {error}"))?;
    client
        .write_all(b"CONNECT localhost:9 HTTP/1.1\r\nHost: localhost:9\r\n\r\n")
        .await
        .map_err(|error| format!("could not write CONNECT request: {error}"))?;
    let response = read_headers(&mut client).await?;
    if !response.starts_with(b"HTTP/1.1 200") {
        return Err(format!(
            "proxy rejected CONNECT setup: {}",
            String::from_utf8_lossy(&response)
        ));
    }

    let mut roots = RootCertStore::empty();
    for certificate in rustls_pemfile::certs(&mut std::io::Cursor::new(ca_pem.as_bytes())) {
        roots
            .add(certificate.map_err(|error| format!("invalid generated CA: {error}"))?)
            .map_err(|error| format!("could not trust generated CA: {error}"))?;
    }
    let tls_config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|error| format!("could not configure acceptance TLS client: {error}"))?
            .with_root_certificates(roots)
            .with_no_client_auth();
    let server_name = ServerName::try_from("localhost".to_owned())
        .map_err(|error| format!("invalid acceptance server name: {error}"))?;
    let mut tls = tokio::time::timeout(
        Duration::from_secs(5),
        TlsConnector::from(Arc::new(tls_config)).connect(server_name, client),
    )
    .await
    .map_err(|_| "timed out negotiating generated leaf certificate".to_owned())?
    .map_err(|error| format!("generated CA did not validate the leaf certificate: {error}"))?;
    tls.write_all(
        b"GET /proxybot-acceptance HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    )
    .await
    .map_err(|error| format!("could not write decrypted HTTPS request: {error}"))?;
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), tls.read_to_end(&mut response))
        .await
        .map_err(|_| "timed out reading intercepted HTTPS response".to_owned())?
        .map_err(|error| format!("could not read intercepted HTTPS response: {error}"))?;
    if !response.starts_with(b"HTTP/1.1 403") {
        return Err(format!(
            "decrypted HTTPS request did not traverse the reject rule: {}",
            String::from_utf8_lossy(&response)
        ));
    }
    Ok(())
}

async fn read_headers(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
    let mut response = Vec::new();
    while !response.ends_with(b"\r\n\r\n") {
        if response.len() >= 8 * 1024 {
            return Err("proxy CONNECT response headers exceeded 8 KiB".to_owned());
        }
        let mut byte = [0_u8; 1];
        let read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut byte))
            .await
            .map_err(|_| "timed out reading proxy CONNECT response".to_owned())?
            .map_err(|error| format!("could not read proxy CONNECT response: {error}"))?;
        if read == 0 {
            return Err("proxy closed before completing CONNECT response headers".to_owned());
        }
        response.push(byte[0]);
    }
    Ok(response)
}

async fn wait_for_captured_request(db: &DbState) -> Result<CapturedRequestRecord, String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let query = CapturedRequestQuery {
        host: Some("localhost".to_owned()),
        limit: Some(10),
        ..CapturedRequestQuery::default()
    };
    loop {
        if let Some(request) = db
            .captured_requests(&query)?
            .into_iter()
            .find(|request| request.path == REQUEST_PATH)
        {
            if request.scheme != "https" || request.response_status != Some(403) {
                return Err(format!(
                    "captured request was not decrypted HTTPS with status 403: scheme={}, status={:?}",
                    request.scheme, request.response_status
                ));
            }
            return Ok(request);
        }
        if Instant::now() >= deadline {
            return Err("decrypted request was not persisted within 5 seconds".to_owned());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
