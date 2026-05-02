//! Version update checker for ProxyBot TUI.
//!
//! Fetches the latest release from GitHub API and compares with current version.

use ureq::get;

/// Check GitHub releases for a newer version.
/// Returns Some(tag_name) if update available, None otherwise.
pub fn check_for_updates(current_version: &str) -> Option<String> {
    let url = "https://api.github.com/repos/mbpz/proxybot/releases/latest";
    let resp = match get(url).call() {
        Ok(r) => r,
        Err(e) => {
            log::debug!("Update check failed: {}", e);
            return None;
        }
    };
    let json: serde_json::Value = match serde_json::from_reader(resp.into_reader()) {
        Ok(j) => j,
        Err(e) => {
            log::debug!("Failed to parse release JSON: {}", e);
            return None;
        }
    };
    let tag_name = json.get("tag_name")?.as_str()?.to_string();
    // tag_name format: "tui-v0.4.1" - strip prefix to get version
    let latest = tag_name.trim_start_matches("tui-");
    if latest != current_version {
        Some(tag_name)
    } else {
        None
    }
}