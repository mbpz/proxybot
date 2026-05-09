use std::net::ToSocketAddrs;

use serde::{Deserialize, Serialize};

// ── Relationship to `crate::replay` ─────────────────────────────────────────
// `crate::replay::ReplayTarget` is a **host summary** (host, request_count,
// path_count) built from the DB — used for browsing recorded traffic.
//
// `ReplayTargetConfig` (below) is a **user-editable replay specification**
// (method, URL, headers, body, expected status, enabled flag) — used for
// defining custom replay scenarios.  Likewise `crate::replay::ReplayResult`
// carries a detailed diff against recorded responses, while `ReplayOutcome`
// is a simple per-target status report.
//
// The two pairs serve different domains and cannot be unified without a
// significant refactor that would touch the existing DB-backed replay engine.
// ─────────────────────────────────────────────────────────────────────────────

/// User-configured replay target specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayTargetConfig {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    #[serde(rename = "expected_status")]
    pub expected_status: Option<u16>,
    pub enabled: bool,
}

/// Result of executing a replay against a single target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayOutcome {
    #[serde(rename = "target_id")]
    pub target_id: String,
    pub status: u16,
    #[serde(rename = "duration_ms")]
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub fn save_replay_target(target: ReplayTargetConfig) -> Result<(), String> {
    // TODO: persist to config / DB
    let _ = target;
    Ok(())
}

#[tauri::command]
pub fn delete_replay_target(_id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn toggle_replay_target(_id: String, _enabled: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn execute_replay(targets: Vec<ReplayTargetConfig>) -> Result<Vec<ReplayOutcome>, String> {
    let mut outcomes = Vec::new();
    let client = reqwest::Client::new();
    for target in targets {
        let start = std::time::Instant::now();
        let outcome = match execute_one(&client, &target).await {
            Ok(response) => {
                let status = response.status().as_u16();
                ReplayOutcome {
                    target_id: target.id.clone(),
                    status,
                    duration_ms: start.elapsed().as_millis() as u64,
                    success: target
                        .expected_status
                        .map(|e| status == e)
                        .unwrap_or(status < 400),
                    error: None,
                }
            }
            Err(e) => ReplayOutcome {
                target_id: target.id.clone(),
                status: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                success: false,
                error: Some(e),
            },
        };
        outcomes.push(outcome);
    }
    Ok(outcomes)
}

fn validate_url(url: &str) -> Result<(), String> {
    let parsed =
        reqwest::Url::parse(url).map_err(|e| format!("invalid URL: {}", e))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "URL has no host".to_string())?;
    let port = parsed.port().unwrap_or(80);
    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("failed to resolve host: {}", e))?;
    for addr in addrs {
        let blocked = match addr.ip() {
            std::net::IpAddr::V4(ipv4) => {
                ipv4.is_loopback() || ipv4.is_private() || ipv4.is_link_local()
            }
            std::net::IpAddr::V6(ipv6) => {
                ipv6.is_loopback()
                    || ipv6.is_unique_local()
                    || ipv6.is_unicast_link_local()
            }
        };
        if blocked {
            return Err("invalid target: internal/private IP not allowed".to_string());
        }
    }
    Ok(())
}

async fn execute_one(
    client: &reqwest::Client,
    target: &ReplayTargetConfig,
) -> Result<reqwest::Response, String> {
    validate_url(&target.url)?;
    let method = reqwest::Method::from_bytes(target.method.as_bytes())
        .unwrap_or(reqwest::Method::GET);
    let mut req = client.request(method, &target.url);
    for (k, v) in &target.headers {
        req = req.header(k.as_str(), v.as_str());
    }
    if let Some(body) = &target.body {
        req = req.body(body.clone());
    }
    req.send().await.map_err(|e| e.to_string())
}
