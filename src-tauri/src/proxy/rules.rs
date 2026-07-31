//! Rule application (MapLocal / MapRemote / Reject / Breakpoint) and the
//! helpers used to build synthetic HTTP responses for those paths.

use super::protocol::{
    expand_user_path, header_value, http_reason, infer_content_type, set_header,
};
use super::requests::generate_request_id;
use super::ProxyContext;
use super::{BreakpointDecision, BreakpointRequest, InterceptedRequest};
use crate::rules::RuleAction;
use std::net::SocketAddr;
use std::path::Path;

// ---------------------------------------------------------------------------
// Rule response / target types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(super) struct RuleResponse {
    pub(super) status: u16,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

#[derive(Clone, Debug)]
pub(super) struct RemoteTarget {
    pub(super) scheme: String,
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) path_prefix: String,
}

pub(super) enum RuleApplication {
    Continue {
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
    Respond {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
    MapRemote {
        target: RemoteTarget,
        method: String,
        path: String,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
}

// ---------------------------------------------------------------------------
// Mock-template rendering
// ---------------------------------------------------------------------------

pub(super) fn render_mock_template(template: &str, req: &InterceptedRequest) -> String {
    use super::requests::timestamp_now;
    let timestamp = timestamp_now();
    let request_id = generate_request_id();
    template
        .replace("{{request.method}}", &req.method)
        .replace("{{request.host}}", &req.host)
        .replace("{{request.path}}", &req.path)
        .replace("{{request.body}}", req.req_body.as_deref().unwrap_or(""))
        .replace("{{timestamp}}", &timestamp)
        .replace("{{request.id}}", &request_id)
}

// ---------------------------------------------------------------------------
// MapLocal support
// ---------------------------------------------------------------------------

pub(super) fn build_map_local_response(
    target: &str,
    req: &InterceptedRequest,
) -> Result<RuleResponse, String> {
    if target.trim().is_empty() {
        return Err("MAPLOCAL target is empty".to_string());
    }

    let path = expand_user_path(target);
    let raw = std::fs::read(&path)
        .map_err(|e| format!("Failed to read MAPLOCAL target {}: {}", path.display(), e))?;

    let raw_text = String::from_utf8(raw.clone()).ok();
    if let Some(text) = raw_text.as_deref() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            let structured = json
                .as_object()
                .map(|obj| {
                    obj.contains_key("status")
                        || obj.contains_key("headers")
                        || obj.contains_key("body")
                })
                .unwrap_or(false);

            if structured {
                let status = json
                    .get("status")
                    .and_then(|v| v.as_u64())
                    .and_then(|v| u16::try_from(v).ok())
                    .unwrap_or(200);
                let mut headers = json
                    .get("headers")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| {
                                let value = v
                                    .as_str()
                                    .map(str::to_string)
                                    .unwrap_or_else(|| v.to_string());
                                (k.clone(), value)
                            })
                            .collect()
                    })
                    .unwrap_or_else(Vec::new);
                let body = match json.get("body") {
                    Some(v) if v.is_string() => {
                        render_mock_template(v.as_str().unwrap_or(""), req).into_bytes()
                    }
                    Some(v) => render_mock_template(&v.to_string(), req).into_bytes(),
                    None => Vec::new(),
                };
                if header_value(&headers, "content-type").is_none() {
                    headers.push((
                        "Content-Type".to_string(),
                        "application/json; charset=utf-8".to_string(),
                    ));
                }
                return Ok(RuleResponse {
                    status,
                    headers,
                    body,
                });
            }
        }
    }

    let body = if let Some(text) = raw_text {
        render_mock_template(&text, req).into_bytes()
    } else {
        raw
    };
    Ok(RuleResponse {
        status: 200,
        headers: vec![(
            "Content-Type".to_string(),
            infer_content_type(Path::new(&path)).to_string(),
        )],
        body,
    })
}

pub(super) fn build_http_response(response: &RuleResponse) -> Vec<u8> {
    let mut headers = response.headers.clone();
    set_header(
        &mut headers,
        "Content-Length",
        response.body.len().to_string(),
    );
    if header_value(&headers, "Connection").is_none() {
        headers.push(("Connection".to_string(), "close".to_string()));
    }

    let mut out = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status,
        http_reason(response.status)
    )
    .into_bytes();
    for (name, value) in headers {
        out.extend_from_slice(format!("{}: {}\r\n", name, value).as_bytes());
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(&response.body);
    out
}

// ---------------------------------------------------------------------------
// MapRemote support
// ---------------------------------------------------------------------------

pub(super) fn parse_remote_target(target: &str) -> Result<RemoteTarget, String> {
    use super::protocol::parse_host_port;
    let (scheme, rest) = target
        .split_once("://")
        .ok_or_else(|| "MAPREMOTE target must start with http:// or https://".to_string())?;
    if scheme != "http" && scheme != "https" {
        return Err("MAPREMOTE target only supports http and https".to_string());
    }

    let (authority, path_prefix) = rest
        .split_once('/')
        .map(|(a, p)| (a, format!("/{}", p.trim_end_matches('/'))))
        .unwrap_or((rest, String::new()));
    let (host, port) = if let Some((h, p)) = parse_host_port(authority) {
        (h.to_string(), p)
    } else {
        (
            authority.to_string(),
            if scheme == "https" { 443 } else { 80 },
        )
    };
    if host.is_empty() {
        return Err("MAPREMOTE target host is empty".to_string());
    }

    Ok(RemoteTarget {
        scheme: scheme.to_string(),
        host,
        port,
        path_prefix,
    })
}

pub(super) fn combine_remote_path(prefix: &str, path: &str) -> String {
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    if prefix.is_empty() {
        return path;
    }
    format!("{}{}", prefix.trim_end_matches('/'), path)
}

// ---------------------------------------------------------------------------
// Main rule-application entry point
// ---------------------------------------------------------------------------

// Arguments mirror the request at the rule-engine boundary.
#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_request_rule(
    ctx: &ProxyContext,
    client_addr: SocketAddr,
    scheme: &str,
    host: &str,
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> Result<RuleApplication, String> {
    use super::requests::build_request_context;

    let Some(action) = ctx.rules_engine.match_host(host, Some(client_addr.ip())) else {
        // No rule matched. If reverse-proxy mode is on, route the
        // unmatched request to the configured local backend instead
        // of going to DNS. Lets frontend devs point ProxyBot at a
        // local server without writing MapRemote rules. The target
        // is read from the global AppConfig (env var or future
        // Tauri command override), so no ProxyContext plumbing.
        if let Some(target) = proxybot_core::config::reverse_target() {
            let remote = parse_remote_target(&target)?;
            let mapped_path = combine_remote_path(&remote.path_prefix, path);
            return Ok(RuleApplication::MapRemote {
                target: remote,
                method: method.to_string(),
                path: mapped_path,
                headers: headers.to_vec(),
                body: body.to_vec(),
            });
        }
        return Ok(RuleApplication::Continue {
            method: method.to_string(),
            path: path.to_string(),
            headers: headers.to_vec(),
            body: body.to_vec(),
        });
    };

    match action {
        RuleAction::Direct | RuleAction::Proxy => Ok(RuleApplication::Continue {
            method: method.to_string(),
            path: path.to_string(),
            headers: headers.to_vec(),
            body: body.to_vec(),
        }),
        RuleAction::Reject => Ok(RuleApplication::Respond {
            status: 403,
            headers: vec![(
                "Content-Type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            )],
            body: b"ProxyBot rule rejected this request\n".to_vec(),
        }),
        RuleAction::MapLocal(target) => {
            let req = build_request_context(method, scheme, host, path, headers, body, client_addr);
            let response = build_map_local_response(&target, &req)?;
            Ok(RuleApplication::Respond {
                status: response.status,
                headers: response.headers,
                body: response.body,
            })
        }
        RuleAction::MapRemote(target) => {
            let remote = parse_remote_target(&target)?;
            Ok(RuleApplication::MapRemote {
                target: remote,
                method: method.to_string(),
                path: path.to_string(),
                headers: headers.to_vec(),
                body: body.to_vec(),
            })
        }
        RuleAction::Breakpoint(target) => {
            log::info!(
                "Breakpoint triggered for host: {} (target: {:?})",
                host,
                target
            );

            let req = build_request_context(method, scheme, host, path, headers, body, client_addr);

            let (decision_tx, decision_rx) = tokio::sync::oneshot::channel();
            if ctx
                .breakpoint_tx
                .try_send(BreakpointRequest {
                    request: req,
                    target,
                    decision_tx,
                })
                .is_err()
            {
                log::warn!("Breakpoint receiver buffer full, proceeding with request");
                return Ok(RuleApplication::Continue {
                    method: method.to_string(),
                    path: path.to_string(),
                    headers: headers.to_vec(),
                    body: body.to_vec(),
                });
            }

            match decision_rx.await {
                Ok(BreakpointDecision::Drop) => Ok(RuleApplication::Respond {
                    status: 403,
                    headers: vec![(
                        "Content-Type".to_string(),
                        "text/plain; charset=utf-8".to_string(),
                    )],
                    body: b"ProxyBot breakpoint dropped this request\n".to_vec(),
                }),
                Ok(BreakpointDecision::Modify(m)) => Ok(RuleApplication::Continue {
                    method: m.method,
                    path: m.path,
                    headers: m.req_headers,
                    body: m.req_body.unwrap_or_default().into_bytes(),
                }),
                Ok(BreakpointDecision::Proceed) | Err(_) => Ok(RuleApplication::Continue {
                    method: method.to_string(),
                    path: path.to_string(),
                    headers: headers.to_vec(),
                    body: body.to_vec(),
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_localhost_target() {
        // The most common reverse-mode form: local dev server on a
        // custom port. Must round-trip the port, default to 80 for
        // plain http, and strip the path prefix.
        let r = parse_remote_target("http://localhost:3000").unwrap();
        assert_eq!(r.scheme, "http");
        assert_eq!(r.host, "localhost");
        assert_eq!(r.port, 3000);
        assert_eq!(r.path_prefix, "");
    }

    #[test]
    fn parse_target_with_path_prefix() {
        // Path prefix gets re-applied to every request via
        // combine_remote_path so the local backend sees the
        // expected mount point.
        let r = parse_remote_target("http://localhost:8080/api").unwrap();
        assert_eq!(r.path_prefix, "/api");
    }

    #[test]
    fn parse_target_default_ports() {
        let http = parse_remote_target("http://example.com").unwrap();
        assert_eq!(http.port, 80);
        let https = parse_remote_target("https://example.com").unwrap();
        assert_eq!(https.port, 443);
    }

    #[test]
    fn parse_target_rejects_garbage() {
        assert!(parse_remote_target("not a url").is_err());
        assert!(parse_remote_target("ftp://example.com").is_err());
        assert!(parse_remote_target("http://").is_err());
    }

    #[test]
    fn combine_remote_path_handles_trailing_slash() {
        // Trailing slash on the prefix should not produce // in the
        // joined path.
        assert_eq!(combine_remote_path("/api/", "/users"), "/api/users");
    }

    #[test]
    fn combine_remote_path_empty_prefix_passthrough() {
        assert_eq!(combine_remote_path("", "/users"), "/users");
        // Even a non-slash path is normalized because the proxied
        // request always arrives with a leading slash.
        assert_eq!(combine_remote_path("", "users"), "/users");
    }
}
