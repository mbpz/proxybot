use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Result returned to the frontend after composing and sending a single request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposerResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u64,
}

#[tauri::command]
pub async fn compose_request(
    method: String,
    url: String,
    headers: HashMap<String, String>,
    body: String,
) -> Result<ComposerResponse, String> {
    // Validate URL and block internal IPs (reuses validate_url from replay module)
    validate_url(&url)?;

    let client = reqwest::Client::new();
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET);
    let mut req = client.request(reqwest_method, &url);

    for (k, v) in &headers {
        if !k.is_empty() {
            req = req.header(k.as_str(), v.as_str());
        }
    }
    if !body.is_empty() {
        req = req.body(body.clone());
    }

    let start = std::time::Instant::now();
    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let resp_headers: HashMap<String, String> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let resp_body = resp.text().await.unwrap_or_default();
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(ComposerResponse {
                status,
                headers: resp_headers,
                body: resp_body,
                duration_ms,
            })
        }
        Err(e) => Err(e.to_string()),
    }
}

fn validate_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| "URL has no host".to_string())?;

    // Allow localhost in dev; block private and link-local ranges
    match host {
        "localhost" | "127.0.0.1" | "::1" => return Ok(()),
        _ => {}
    }

    // If the host is an IP literal, check ranges
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(ipv4) => {
                if ipv4.is_private() || ipv4.is_link_local() {
                    return Err("Internal/private IP not allowed".to_string());
                }
            }
            std::net::IpAddr::V6(ipv6) => {
                if ipv6.is_unique_local() || ipv6.is_unicast_link_local() {
                    return Err("Internal/private IP not allowed".to_string());
                }
            }
        }
    }

    Ok(())
}
