use std::net::UdpSocket;
use std::path::PathBuf;

pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("PROXYBOT_DATA_DIR") {
        return PathBuf::from(dir);
    }
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("AppData/Local")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ProxyBot");
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support/ProxyBot")
    }
}

pub fn lan_ip() -> String {
    // UDP connect chooses the active route without sending a packet.
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|socket| {
            socket.connect("192.0.2.1:9")?;
            socket.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

pub fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).status();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .status();
}

#[cfg(test)]
mod tests {
    #[test]
    fn lan_ip_is_ipv4() {
        let ip = super::lan_ip();
        assert!(ip.parse::<std::net::Ipv4Addr>().is_ok());
    }
}
