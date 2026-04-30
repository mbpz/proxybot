//! Common test utilities.

use proxybot_lib::db::RecentRequest;

/// Create a test request with the given fields.
pub fn make_req(
    id: i64,
    method: &str,
    host: &str,
    path: &str,
    status: Option<u16>,
) -> RecentRequest {
    RecentRequest {
        id,
        timestamp: id.to_string(),
        method: method.into(),
        scheme: "https".into(),
        host: host.into(),
        path: path.into(),
        status,
        duration_ms: Some(100),
        app_tag: None,
    }
}
