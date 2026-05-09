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
            const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10MB
            let body_bytes = resp.bytes().await.unwrap_or_default();
            let resp_body = if body_bytes.len() > MAX_BODY_SIZE {
                format!(
                    "[Response truncated: {} bytes, showing first 10MB]\n\n{}",
                    body_bytes.len(),
                    String::from_utf8_lossy(&body_bytes[..MAX_BODY_SIZE])
                )
            } else {
                String::from_utf8_lossy(&body_bytes).into_owned()
            };
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
    let port = parsed
        .port()
        .unwrap_or(if parsed.scheme() == "https" { 443 } else { 80 });

    // Allow localhost in dev
    match host {
        "localhost" | "127.0.0.1" | "::1" => return Ok(()),
        _ => {}
    }

    // Resolve hostname and check all resolved IPs
    let addr_str = format!("{}:{}", host, port);
    match std::net::ToSocketAddrs::to_socket_addrs(&addr_str) {
        Ok(addrs) => {
            for addr in addrs {
                let ip = addr.ip();
                let blocked = match ip {
                    std::net::IpAddr::V4(v4) => {
                        v4.is_private() || v4.is_loopback() || v4.is_link_local()
                    }
                    std::net::IpAddr::V6(v6) => {
                        v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local()
                    }
                };
                if blocked {
                    return Err(format!("Internal/private IP not allowed: {}", ip));
                }
            }
        }
        Err(_) => {
            // DNS resolution failed - allow the request to proceed
            // (the HTTP client will also fail with a better error)
        }
    }

    Ok(())
}
