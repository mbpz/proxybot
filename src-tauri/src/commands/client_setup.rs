use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub id: String,
    pub name: String,
    pub client_type: ClientType,
    pub installed: bool,
    pub proxy_configured: bool,
    pub config_instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientType {
    Browser,
    Runtime,
    Other,
}

#[tauri::command]
// Keeping a comment above each client makes this user-facing compatibility
// list easier to audit than one large `vec!` expression.
#[allow(clippy::vec_init_then_push)]
pub fn detect_clients(
    config: State<'_, Arc<proxybot_core::AppConfig>>,
) -> Result<Vec<ClientInfo>, String> {
    let mut clients = Vec::new();
    let proxy = format!("127.0.0.1:{}", config.proxy_port);
    let ca = config.ca_cert_path.to_string_lossy();

    // Chrome
    clients.push(ClientInfo {
        id: "chrome".into(),
        name: "Google Chrome".into(),
        client_type: ClientType::Browser,
        installed: app_exists("Google Chrome"),
        proxy_configured: false,
        config_instructions: format!(
            "Settings → System → Open proxy settings → Set HTTP proxy to {proxy}"
        ),
    });

    // Firefox
    clients.push(ClientInfo {
        id: "firefox".into(),
        name: "Firefox".into(),
        client_type: ClientType::Browser,
        installed: app_exists("Firefox"),
        proxy_configured: false,
        config_instructions: format!(
            "Settings → Network Settings → Manual proxy → HTTP Proxy: 127.0.0.1, Port: {}",
            config.proxy_port
        ),
    });

    // Safari
    clients.push(ClientInfo {
        id: "safari".into(),
        name: "Safari".into(),
        client_type: ClientType::Browser,
        installed: true, // macOS always has Safari
        proxy_configured: false,
        config_instructions: "Uses system proxy settings (System Preferences → Network → Proxies)"
            .into(),
    });

    // Brave
    clients.push(ClientInfo {
        id: "brave".into(),
        name: "Brave Browser".into(),
        client_type: ClientType::Browser,
        installed: app_exists("Brave Browser"),
        proxy_configured: false,
        config_instructions: format!(
            "Settings → System → Open proxy settings → Set HTTP proxy to {proxy}"
        ),
    });

    // Edge
    clients.push(ClientInfo {
        id: "edge".into(),
        name: "Microsoft Edge".into(),
        client_type: ClientType::Browser,
        installed: app_exists("Microsoft Edge"),
        proxy_configured: false,
        config_instructions: format!(
            "Settings → System → Open proxy settings → Set HTTP proxy to {proxy}"
        ),
    });

    // Arc
    clients.push(ClientInfo {
        id: "arc".into(),
        name: "Arc".into(),
        client_type: ClientType::Browser,
        installed: app_exists("Arc"),
        proxy_configured: false,
        config_instructions: format!(
            "Settings → System → Open proxy settings → Set HTTP proxy to {proxy}"
        ),
    });

    // Node.js
    clients.push(ClientInfo {
        id: "nodejs".into(),
        name: "Node.js".into(),
        client_type: ClientType::Runtime,
        installed: command_exists("node"),
        proxy_configured: false,
        config_instructions: format!(
            "export HTTP_PROXY=http://{proxy} HTTPS_PROXY=http://{proxy} NODE_EXTRA_CA_CERTS={ca}"
        ),
    });

    // Python
    clients.push(ClientInfo {
        id: "python".into(),
        name: "Python".into(),
        client_type: ClientType::Runtime,
        installed: command_exists("python3") || command_exists("python"),
        proxy_configured: false,
        config_instructions: format!(
            "export HTTP_PROXY=http://{proxy} HTTPS_PROXY=http://{proxy} REQUESTS_CA_BUNDLE={ca}"
        ),
    });

    Ok(clients)
}

#[tauri::command]
pub fn get_proxy_config_command(
    client_id: String,
    config: State<'_, Arc<proxybot_core::AppConfig>>,
) -> Result<String, String> {
    let proxy = format!("127.0.0.1:{}", config.proxy_port);
    let ca = config.ca_cert_path.to_string_lossy();
    match client_id.as_str() {
        "chrome" => Ok(format!("open -a 'Google Chrome' --args --proxy-server='http://{proxy}' --ignore-certificate-errors-spki-list=''")),
        "nodejs" => Ok(format!("export HTTP_PROXY=http://{proxy} HTTPS_PROXY=http://{proxy} NODE_EXTRA_CA_CERTS={ca}")),
        "python" => Ok(format!("export HTTP_PROXY=http://{proxy} HTTPS_PROXY=http://{proxy} REQUESTS_CA_BUNDLE={ca}")),
        _ => Ok(format!("Configure your client to use HTTP proxy at {proxy}")),
    }
}

fn app_exists(name: &str) -> bool {
    // 1. Check standard locations
    let paths = [
        format!("/Applications/{}.app", name),
        format!("/Applications/{} Canary.app", name), // Chrome Canary etc
        format!(
            "{}/Applications/{}.app",
            std::env::var("HOME").unwrap_or_default(),
            name
        ),
    ];
    for path in &paths {
        if std::path::Path::new(path).exists() {
            return true;
        }
    }

    // 2. Use mdfind for broader search (macOS Spotlight)
    if let Ok(output) = Command::new("mdfind")
        .args([
            "kMDItemKind == 'Application'",
            &format!("kMDItemDisplayName == '*{}*'", name),
        ])
        .output()
    {
        if output.status.success() && !output.stdout.is_empty() {
            return true;
        }
    }

    false
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
