use serde::{Deserialize, Serialize};
use std::process::Command;

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
pub fn detect_clients() -> Result<Vec<ClientInfo>, String> {
    let mut clients = Vec::new();

    // Chrome
    clients.push(ClientInfo {
        id: "chrome".into(),
        name: "Google Chrome".into(),
        client_type: ClientType::Browser,
        installed: app_exists("Google Chrome"),
        proxy_configured: false,
        config_instructions:
            "Settings → System → Open proxy settings → Set HTTP proxy to 127.0.0.1:8088".into(),
    });

    // Firefox
    clients.push(ClientInfo {
        id: "firefox".into(),
        name: "Firefox".into(),
        client_type: ClientType::Browser,
        installed: app_exists("Firefox"),
        proxy_configured: false,
        config_instructions:
            "Settings → Network Settings → Manual proxy → HTTP Proxy: 127.0.0.1, Port: 8088".into(),
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
        config_instructions:
            "Settings → System → Open proxy settings → Set HTTP proxy to 127.0.0.1:8088".into(),
    });

    // Edge
    clients.push(ClientInfo {
        id: "edge".into(),
        name: "Microsoft Edge".into(),
        client_type: ClientType::Browser,
        installed: app_exists("Microsoft Edge"),
        proxy_configured: false,
        config_instructions:
            "Settings → System → Open proxy settings → Set HTTP proxy to 127.0.0.1:8088".into(),
    });

    // Arc
    clients.push(ClientInfo {
        id: "arc".into(),
        name: "Arc".into(),
        client_type: ClientType::Browser,
        installed: app_exists("Arc"),
        proxy_configured: false,
        config_instructions:
            "Settings → System → Open proxy settings → Set HTTP proxy to 127.0.0.1:8088".into(),
    });

    // Node.js
    clients.push(ClientInfo {
        id: "nodejs".into(),
        name: "Node.js".into(),
        client_type: ClientType::Runtime,
        installed: command_exists("node"),
        proxy_configured: false,
        config_instructions: "export HTTP_PROXY=http://127.0.0.1:8088 HTTPS_PROXY=http://127.0.0.1:8088 NODE_EXTRA_CA_CERTS=~/.proxybot/ca.crt".into(),
    });

    // Python
    clients.push(ClientInfo {
        id: "python".into(),
        name: "Python".into(),
        client_type: ClientType::Runtime,
        installed: command_exists("python3") || command_exists("python"),
        proxy_configured: false,
        config_instructions: "export HTTP_PROXY=http://127.0.0.1:8088 HTTPS_PROXY=http://127.0.0.1:8088 REQUESTS_CA_BUNDLE=~/.proxybot/ca.crt".into(),
    });

    Ok(clients)
}

#[tauri::command]
pub fn get_proxy_config_command(client_id: String) -> Result<String, String> {
    match client_id.as_str() {
        "chrome" => Ok("open -a 'Google Chrome' --args --proxy-server='http://127.0.0.1:8088' --ignore-certificate-errors-spki-list=''".into()),
        "nodejs" => Ok("export HTTP_PROXY=http://127.0.0.1:8088 HTTPS_PROXY=http://127.0.0.1:8088 NODE_EXTRA_CA_CERTS=~/.proxybot/ca.crt".into()),
        "python" => Ok("export HTTP_PROXY=http://127.0.0.1:8088 HTTPS_PROXY=http://127.0.0.1:8088 REQUESTS_CA_BUNDLE=~/.proxybot/ca.crt".into()),
        _ => Ok("Configure your client to use HTTP proxy at 127.0.0.1:8088".into()),
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
