//! Stable Captured Request input for independent analysis Implementations.
//!
//! Persistence timestamps accepted by [`CapturedRequestRecord`] are converted
//! to UTC once. Invalid legacy timestamps map to Unix epoch. Analysis windows
//! use epoch milliseconds and include both boundaries; an invalid timestamp is
//! therefore excluded from normal recent windows but remains visible to an
//! unbounded/session window beginning at zero.

use crate::db::{parse_captured_timestamp, CapturedRequestQuery, CapturedRequestRecord, DbState};
use chrono::{DateTime, Utc};

/// Immutable facts shared by DAG, Graph, Topology, Auth and anomaly analysis.
#[derive(Clone, Debug, PartialEq)]
pub struct CapturedRequestAnalysis {
    pub id: i64,
    pub timestamp: String,
    pub captured_at: DateTime<Utc>,
    pub method: String,
    pub scheme: String,
    pub host: String,
    pub path: String,
    pub request_headers: Vec<(String, String)>,
    pub request_body: Option<String>,
    pub response_status: Option<u16>,
    pub response_headers: Vec<(String, String)>,
    pub response_body: Option<String>,
    pub duration_ms: u64,
    pub device_id: Option<i64>,
    pub app_tag: Option<String>,
    pub response_size: usize,
    pub upstream_ip: Option<String>,
}

impl CapturedRequestAnalysis {
    pub fn captured_at_millis(&self) -> i64 {
        self.captured_at.timestamp_millis()
    }

    pub fn captured_at_seconds(&self) -> i64 {
        self.captured_at.timestamp()
    }

    pub fn is_error(&self) -> bool {
        self.response_status.is_some_and(|status| status >= 400)
    }

    pub fn response_bytes(&self) -> usize {
        self.response_size
    }

    /// Build the same projection for a live Adapter input that has not been
    /// persisted yet. This keeps anomaly analysis on the shared Interface.
    pub fn live_anomaly_input(
        device_id: Option<i64>,
        host: &str,
        upstream_ip: Option<&str>,
        request_body: Option<&str>,
        response_body: Option<&str>,
    ) -> Self {
        let captured_at = Utc::now();
        Self {
            id: 0,
            timestamp: captured_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            captured_at,
            method: String::new(),
            scheme: String::new(),
            host: host.to_owned(),
            path: String::new(),
            request_headers: Vec::new(),
            request_body: request_body.map(str::to_owned),
            response_status: None,
            response_headers: Vec::new(),
            response_body: response_body.map(str::to_owned),
            duration_ms: 0,
            device_id,
            app_tag: None,
            response_size: response_body.map_or(0, str::len),
            upstream_ip: upstream_ip.map(str::to_owned),
        }
    }
}

impl From<CapturedRequestRecord> for CapturedRequestAnalysis {
    fn from(record: CapturedRequestRecord) -> Self {
        let captured_at = analysis_timestamp(&record.timestamp);
        let request_body = record
            .request_body
            .as_deref()
            .map(|body| String::from_utf8_lossy(body).into_owned());
        let response_body = record
            .response_body
            .as_deref()
            .map(|body| String::from_utf8_lossy(body).into_owned());
        let response_size = record
            .response_size
            .and_then(|value| usize::try_from(value).ok())
            .or_else(|| record.response_body.as_ref().map(Vec::len))
            .unwrap_or(0);
        Self {
            id: record.id,
            timestamp: record.timestamp,
            captured_at,
            method: record.method,
            scheme: record.scheme,
            host: record.host,
            path: record.path,
            request_headers: record.request_headers,
            request_body,
            response_status: record.response_status,
            response_headers: record.response_headers,
            response_body,
            duration_ms: record
                .duration_ms
                .and_then(|value| u64::try_from(value).ok())
                .unwrap_or(0),
            device_id: record.device_id,
            app_tag: record.app_tag,
            response_size,
            upstream_ip: record.upstream_ip,
        }
    }
}

/// Canonical timestamp conversion for Captured Request analysis.
pub fn analysis_timestamp(timestamp: &str) -> DateTime<Utc> {
    parse_captured_timestamp(timestamp).unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
}

pub fn analysis_timestamp_millis(timestamp: &str) -> i64 {
    analysis_timestamp(timestamp).timestamp_millis()
}

impl DbState {
    /// Query Captured Requests and project them once for analysis consumers.
    pub fn analysis_requests(
        &self,
        query: &CapturedRequestQuery,
    ) -> Result<Vec<CapturedRequestAnalysis>, String> {
        Ok(self
            .captured_requests(query)?
            .into_iter()
            .map(CapturedRequestAnalysis::from)
            .collect())
    }
}

/// Inclusive epoch-millisecond window shared by analysis algorithms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnalysisWindow {
    pub start_ms: i64,
    pub end_ms: i64,
}

impl AnalysisWindow {
    pub const fn inclusive(start_ms: i64, end_ms: i64) -> Self {
        Self { start_ms, end_ms }
    }

    pub fn contains(&self, request: &CapturedRequestAnalysis) -> bool {
        let timestamp = request.captured_at_millis();
        timestamp >= self.start_ms && timestamp <= self.end_ms
    }
}

#[cfg(test)]
pub(crate) fn fixed_analysis_fixture() -> Vec<CapturedRequestAnalysis> {
    let first_at = DateTime::parse_from_rfc3339("2026-08-04T10:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let second_at = DateTime::parse_from_rfc3339("2026-08-04T10:00:01Z")
        .unwrap()
        .with_timezone(&Utc);
    vec![
        CapturedRequestAnalysis {
            id: 1,
            timestamp: "2026-08-04T10:00:00Z".to_owned(),
            captured_at: first_at,
            method: "POST".to_owned(),
            scheme: "https".to_owned(),
            host: "api.example.com".to_owned(),
            path: "/login".to_owned(),
            request_headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            request_body: Some(r#"{"phone":"+8613812345678"}"#.to_owned()),
            response_status: Some(200),
            response_headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            response_body: Some(r#"{"access_token":"abc123token456def789"}"#.to_owned()),
            duration_ms: 12,
            device_id: Some(7),
            app_tag: Some("sample-app".to_owned()),
            response_size: 44,
            upstream_ip: Some("203.0.113.10".to_owned()),
        },
        CapturedRequestAnalysis {
            id: 2,
            timestamp: "2026-08-04T10:00:01Z".to_owned(),
            captured_at: second_at,
            method: "GET".to_owned(),
            scheme: "https".to_owned(),
            host: "api.example.com".to_owned(),
            path: "/profile".to_owned(),
            request_headers: vec![
                (
                    "Referer".to_owned(),
                    "https://api.example.com/login".to_owned(),
                ),
                (
                    "Cookie".to_owned(),
                    "access_token=abc123token456def789".to_owned(),
                ),
            ],
            request_body: None,
            response_status: Some(401),
            response_headers: Vec::new(),
            response_body: None,
            duration_ms: 18,
            device_id: Some(7),
            app_tag: Some("sample-app".to_owned()),
            response_size: 0,
            upstream_ip: Some("203.0.113.10".to_owned()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NewCapturedRequest;

    #[test]
    fn persistence_projection_normalizes_analysis_facts_once() {
        let db = DbState::new_in_memory(std::sync::Mutex::new(())).unwrap();
        db.record_captured_request(NewCapturedRequest {
            timestamp: "2026-08-04 10:00:00",
            method: "GET",
            scheme: "https",
            host: "api.example.com",
            path: "/profile",
            request_headers: &[],
            request_body: None,
            response_status: Some(503),
            response_headers: &[],
            response_body: Some("error"),
            duration_ms: None,
            device_id: Some(7),
            app_tag: Some("sample-app"),
            response_size: None,
            session_id: None,
            client_ip: None,
            upstream_ip: Some("203.0.113.10"),
        })
        .unwrap();

        let facts = db
            .analysis_requests(&CapturedRequestQuery::default())
            .unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].captured_at_millis(), 1_785_837_600_000);
        assert_eq!(facts[0].duration_ms, 0);
        assert!(facts[0].is_error());
        assert_eq!(facts[0].response_size, 5);
        assert_eq!(facts[0].device_id, Some(7));
    }

    #[test]
    fn time_windows_are_inclusive_and_invalid_timestamps_use_epoch() {
        let fixture = fixed_analysis_fixture();
        let timestamp = fixture[0].captured_at_millis();
        let window = AnalysisWindow::inclusive(timestamp, timestamp);
        assert!(window.contains(&fixture[0]));
        assert!(!window.contains(&fixture[1]));

        let mut invalid = fixture[0].clone();
        invalid.captured_at = DateTime::<Utc>::UNIX_EPOCH;
        assert!(AnalysisWindow::inclusive(0, timestamp).contains(&invalid));
        assert!(!AnalysisWindow::inclusive(1, timestamp).contains(&invalid));
        assert_eq!(analysis_timestamp_millis("not-a-timestamp"), 0);
    }
}
