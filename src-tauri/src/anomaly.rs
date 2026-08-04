//! Anomaly detection and privacy scanning module.
//!
//! Provides:
//! - Per-device traffic baseline (7-day rolling domain/IP frequency profile)
//! - New domain/IP detection (triggers info-level alerts)
//! - Privacy scanner (IDFA, phone E.164, GPS coordinates detection)

#![allow(clippy::manual_is_multiple_of)]

use crate::alerts::{AlertSeverity, AlertType, NewAlert};
use crate::db::DbState;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

/// Privacy pattern types for scanning results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum PrivacyPattern {
    IDFA,
    PhoneNumber,
    GpsCoordinates,
}

/// Result of a privacy scan on a body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyScanResult {
    pub pattern: PrivacyPattern,
    pub matched_text: String,
    pub context: String,
}

/// Baseline entry for a domain/IP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineEntry {
    pub value: String,
    pub count: i64,
    pub first_seen: String,
    pub last_seen: String,
}

/// Traffic baseline for a device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficBaseline {
    pub device_id: Option<i64>,
    pub domains: Vec<BaselineEntry>,
    pub ips: Vec<BaselineEntry>,
}

/// Anomaly scan result for a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyScanResult {
    pub new_domains: Vec<String>,
    pub new_ips: Vec<String>,
    pub privacy_findings: Vec<PrivacyScanResult>,
    pub alerts_generated: i32,
}

/// Baseline store for persistent domain/IP baseline storage.
pub struct BaselineStore {
    path: PathBuf,
    baselines: Mutex<BaselineData>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct BaselineData {
    version: u32,
    domains: Vec<DomainEntry>,
    ips: Vec<IpEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct DomainEntry {
    device_id: Option<i64>,
    domain: String,
    count: i64,
    first_seen: String,
    last_seen: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct IpEntry {
    device_id: Option<i64>,
    ip_address: String,
    count: i64,
    first_seen: String,
    last_seen: String,
}

impl Default for BaselineStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BaselineStore {
    pub fn new() -> Self {
        let path = PathBuf::from("baseline.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        Self::with_path(path)
    }

    /// Test-friendly constructor that uses the given path instead of `$HOME/.proxybot/`.
    pub fn with_path(path: PathBuf) -> Self {
        let baselines = Self::load_from_file(&path);
        Self {
            path,
            baselines: Mutex::new(baselines),
        }
    }

    fn load_from_file(path: &PathBuf) -> BaselineData {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return BaselineData::default(),
        };
        let reader = BufReader::new(file);
        serde_json::from_reader::<_, BaselineData>(reader).unwrap_or_default()
    }

    fn save_to_file(&self) {
        let baselines = self.baselines.lock().unwrap();
        let data = baselines.clone();
        drop(baselines);
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
        {
            let mut writer = BufWriter::new(file);
            let _ = serde_json::to_writer(&mut writer, &data);
            let _ = writer.flush();
        }
    }

    pub fn is_new_domain(&self, device_id: Option<i64>, domain: &str) -> bool {
        let baselines = self.baselines.lock().unwrap();
        let seven_days_ago = get_seven_days_ago();
        !baselines
            .domains
            .iter()
            .any(|e| e.device_id == device_id && e.domain == domain && e.last_seen > seven_days_ago)
    }

    pub fn is_new_ip(&self, device_id: Option<i64>, ip: &str) -> bool {
        let baselines = self.baselines.lock().unwrap();
        let seven_days_ago = get_seven_days_ago();
        !baselines
            .ips
            .iter()
            .any(|e| e.device_id == device_id && e.ip_address == ip && e.last_seen > seven_days_ago)
    }

    pub fn add_domain(&self, device_id: Option<i64>, domain: &str) {
        let mut baselines = self.baselines.lock().unwrap();
        let now = chrono_lite_timestamp();
        let seven_days_ago = get_seven_days_ago();

        if let Some(entry) = baselines
            .domains
            .iter_mut()
            .find(|e| e.device_id == device_id && e.domain == domain)
        {
            entry.count += 1;
            entry.last_seen = now.clone();
        } else {
            baselines.domains.push(DomainEntry {
                device_id,
                domain: domain.to_string(),
                count: 1,
                first_seen: now.clone(),
                last_seen: now,
            });
        }

        // Cleanup old entries
        baselines.domains.retain(|e| e.last_seen > seven_days_ago);
        baselines.ips.retain(|e| e.last_seen > seven_days_ago);

        drop(baselines);
        self.save_to_file();
    }

    pub fn add_ip(&self, device_id: Option<i64>, ip: &str) {
        let mut baselines = self.baselines.lock().unwrap();
        let now = chrono_lite_timestamp();

        if let Some(entry) = baselines
            .ips
            .iter_mut()
            .find(|e| e.device_id == device_id && e.ip_address == ip)
        {
            entry.count += 1;
            entry.last_seen = now.clone();
        } else {
            baselines.ips.push(IpEntry {
                device_id,
                ip_address: ip.to_string(),
                count: 1,
                first_seen: now.clone(),
                last_seen: now,
            });
        }

        drop(baselines);
        self.save_to_file();
    }

    pub fn get_baseline(&self, device_id: Option<i64>) -> TrafficBaseline {
        let baselines = self.baselines.lock().unwrap();
        let seven_days_ago = get_seven_days_ago();

        let domains: Vec<BaselineEntry> = baselines
            .domains
            .iter()
            .filter(|e| e.device_id == device_id && e.last_seen > seven_days_ago.clone())
            .map(|e| BaselineEntry {
                value: e.domain.clone(),
                count: e.count,
                first_seen: e.first_seen.clone(),
                last_seen: e.last_seen.clone(),
            })
            .collect();

        let ips: Vec<BaselineEntry> = baselines
            .ips
            .iter()
            .filter(|e| e.device_id == device_id && e.last_seen > seven_days_ago.clone())
            .map(|e| BaselineEntry {
                value: e.ip_address.clone(),
                count: e.count,
                first_seen: e.first_seen.clone(),
                last_seen: e.last_seen.clone(),
            })
            .collect();

        TrafficBaseline {
            device_id,
            domains,
            ips,
        }
    }
}

/// Privacy scanner with compiled regex patterns.
pub struct PrivacyScanner {
    idfa_regex: Regex,
    phone_regex: Regex,
    gps_regex: Regex,
}

impl PrivacyScanner {
    pub fn new() -> Self {
        let idfa_regex = Regex::new(
            r"\b[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}\b",
        )
        .unwrap();

        let phone_regex = Regex::new(r"\+\d{7,15}").unwrap();

        let gps_regex = Regex::new(
            r#"(?x)
            (?:["']?(?:latitude|lat|lng|longitude|long)["']?\s*[:=]\s*)?
            (?:["'])?
            (-?\d{1,3}\.\d{4,10})
            (?:["']?\s*[,;]\s*)
            (-?\d{1,3}\.\d{4,10})
            "#,
        )
        .unwrap();

        Self {
            idfa_regex,
            phone_regex,
            gps_regex,
        }
    }

    pub fn scan(&self, text: &str) -> Vec<PrivacyScanResult> {
        let mut results = Vec::new();

        for m in self.idfa_regex.find_iter(text) {
            let matched = m.as_str().to_uppercase();
            if Self::looks_like_idfa(&matched) {
                results.push(PrivacyScanResult {
                    pattern: PrivacyPattern::IDFA,
                    matched_text: matched,
                    context: Self::extract_context(text, m.start(), m.end()),
                });
            }
        }

        for m in self.phone_regex.find_iter(text) {
            let matched = m.as_str().to_string();
            if Self::looks_like_phone(&matched) {
                results.push(PrivacyScanResult {
                    pattern: PrivacyPattern::PhoneNumber,
                    matched_text: matched,
                    context: Self::extract_context(text, m.start(), m.end()),
                });
            }
        }

        for m in self.gps_regex.find_iter(text) {
            results.push(PrivacyScanResult {
                pattern: PrivacyPattern::GpsCoordinates,
                matched_text: m.as_str().to_string(),
                context: Self::extract_context(text, m.start(), m.end()),
            });
        }

        results
    }

    fn looks_like_idfa(s: &str) -> bool {
        if s.len() != 36 {
            return false;
        }
        s.chars()
            .filter(|c| *c != '-')
            .all(|c| c.is_ascii_hexdigit())
    }

    fn looks_like_phone(s: &str) -> bool {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.len() >= 7 && digits.len() <= 15
    }

    fn extract_context(text: &str, start: usize, end: usize) -> String {
        let context_len = 30;
        let ctx_start = start.saturating_sub(context_len);
        let ctx_end = (end + context_len).min(text.len());
        let prefix = if ctx_start > 0 { "..." } else { "" };
        let suffix = if ctx_end < text.len() { "..." } else { "" };
        format!("{}{}{}", prefix, &text[ctx_start..ctx_end], suffix)
    }
}

impl Default for PrivacyScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Anomaly detector state.
pub struct AnomalyDetector {
    privacy_scanner: PrivacyScanner,
    alerts: Arc<DbState>,
    baseline_store: Arc<BaselineStore>,
    domain_cache: Mutex<HashSet<(Option<i64>, String)>>,
    ip_cache: Mutex<HashSet<(Option<i64>, String)>>,
}

impl AnomalyDetector {
    pub fn new(alerts: Arc<DbState>) -> Self {
        Self {
            privacy_scanner: PrivacyScanner::new(),
            alerts,
            baseline_store: Arc::new(BaselineStore::new()),
            domain_cache: Mutex::new(HashSet::new()),
            ip_cache: Mutex::new(HashSet::new()),
        }
    }

    /// Test-friendly constructor that wires in the given stores instead of creating
    /// fresh `$HOME/.proxybot/`-backed stores.
    pub fn with_stores(alerts: Arc<DbState>, baseline: Arc<BaselineStore>) -> Self {
        Self {
            privacy_scanner: PrivacyScanner::new(),
            alerts,
            baseline_store: baseline,
            domain_cache: Mutex::new(HashSet::new()),
            ip_cache: Mutex::new(HashSet::new()),
        }
    }

    pub fn scan_request(
        &self,
        device_id: Option<i64>,
        host: &str,
        ip: Option<&str>,
        req_body: Option<&str>,
        resp_body: Option<&str>,
    ) -> Result<AnomalyScanResult, String> {
        let mut result = AnomalyScanResult {
            new_domains: Vec::new(),
            new_ips: Vec::new(),
            privacy_findings: Vec::new(),
            alerts_generated: 0,
        };

        // Check if domain is new for this device
        if self.is_new_domain(device_id, host) {
            result.new_domains.push(host.to_string());
            let details = format!("New domain accessed: {} (device: {:?})", host, device_id);
            self.alerts.publish_alert(NewAlert {
                device_id,
                severity: AlertSeverity::Info,
                alert_type: AlertType::NewDomain,
                details,
                occurrence_key: None,
            })?;
            result.alerts_generated += 1;
        }

        if let Some(ip_addr) = ip {
            if self.is_new_ip(device_id, ip_addr) {
                result.new_ips.push(ip_addr.to_string());
                let details = format!("New IP accessed: {} (device: {:?})", ip_addr, device_id);
                self.alerts.publish_alert(NewAlert {
                    device_id,
                    severity: AlertSeverity::Info,
                    alert_type: AlertType::NewIp,
                    details,
                    occurrence_key: None,
                })?;
                result.alerts_generated += 1;
            }
        }

        // Scan request body for privacy data
        if let Some(body) = req_body {
            let findings = self.privacy_scanner.scan(body);
            for finding in &findings {
                let pattern_name = match &finding.pattern {
                    PrivacyPattern::IDFA => "IDFA (advertising identifier)",
                    PrivacyPattern::PhoneNumber => "Phone number (E.164)",
                    PrivacyPattern::GpsCoordinates => "GPS coordinates",
                };
                let details = format!(
                    "Privacy data detected: {} in request body. Matched: '{}'. Context: {}",
                    pattern_name, finding.matched_text, finding.context
                );
                self.alerts.publish_alert(NewAlert {
                    device_id,
                    severity: AlertSeverity::Warning,
                    alert_type: AlertType::PrivacyExfil,
                    details,
                    occurrence_key: None,
                })?;
                result.alerts_generated += 1;
            }
            result.privacy_findings.extend(findings);
        }

        // Scan response body for privacy data
        if let Some(body) = resp_body {
            let findings = self.privacy_scanner.scan(body);
            for finding in &findings {
                let pattern_name = match &finding.pattern {
                    PrivacyPattern::IDFA => "IDFA (advertising identifier)",
                    PrivacyPattern::PhoneNumber => "Phone number (E.164)",
                    PrivacyPattern::GpsCoordinates => "GPS coordinates",
                };
                let details = format!(
                    "Privacy data detected: {} in response body. Matched: '{}'. Context: {}",
                    pattern_name, finding.matched_text, finding.context
                );
                self.alerts.publish_alert(NewAlert {
                    device_id,
                    severity: AlertSeverity::Warning,
                    alert_type: AlertType::PrivacyExfil,
                    details,
                    occurrence_key: None,
                })?;
                result.alerts_generated += 1;
            }
            result.privacy_findings.extend(findings);
        }

        // Update baseline after scanning
        self.baseline_store.add_domain(device_id, host);
        if let Some(ip_addr) = ip {
            self.baseline_store.add_ip(device_id, ip_addr);
        }

        Ok(result)
    }

    fn is_new_domain(&self, device_id: Option<i64>, domain: &str) -> bool {
        {
            let cache = self.domain_cache.lock().unwrap();
            if cache.contains(&(device_id, domain.to_string())) {
                return false;
            }
        }

        let is_new = self.baseline_store.is_new_domain(device_id, domain);

        if !is_new {
            let mut cache = self.domain_cache.lock().unwrap();
            cache.insert((device_id, domain.to_string()));
        }

        is_new
    }

    fn is_new_ip(&self, device_id: Option<i64>, ip: &str) -> bool {
        {
            let cache = self.ip_cache.lock().unwrap();
            if cache.contains(&(device_id, ip.to_string())) {
                return false;
            }
        }

        let is_new = self.baseline_store.is_new_ip(device_id, ip);

        if !is_new {
            let mut cache = self.ip_cache.lock().unwrap();
            cache.insert((device_id, ip.to_string()));
        }

        is_new
    }

    pub fn get_baseline(&self, device_id: Option<i64>) -> TrafficBaseline {
        self.baseline_store.get_baseline(device_id)
    }
}

/// Format timestamp for SQLite-like storage.
pub fn chrono_lite_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    let mut remaining = secs;

    let mut year = 1970;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year as u64 * 86400 {
            break;
        }
        remaining -= days_in_year as u64 * 86400;
        year += 1;
    }

    let days_in_months: &[u64] = if is_leap_year(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for days in days_in_months {
        if remaining < days * 86400 {
            break;
        }
        remaining -= days * 86400;
        month += 1;
    }

    let day = (remaining / 86400) + 1;
    remaining %= 86400;
    let hour = remaining / 3600;
    remaining %= 3600;
    let minute = remaining / 60;
    let second = remaining % 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    )
}

#[allow(clippy::manual_is_multiple_of)]
pub(crate) fn is_leap_year(year: u64) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn get_seven_days_ago() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let seven_days_secs = 7 * 24 * 60 * 60;
    let seven_days_ago = now - std::time::Duration::from_secs(seven_days_secs);

    let secs = seven_days_ago.as_secs();
    let mut remaining = secs;

    let mut year = 1970;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year as u64 * 86400 {
            break;
        }
        remaining -= days_in_year as u64 * 86400;
        year += 1;
    }

    let days_in_months: &[u64] = if is_leap_year(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1;
    for days in days_in_months {
        if remaining < days * 86400 {
            break;
        }
        remaining -= days * 86400;
        month += 1;
    }

    let day = (remaining / 86400) + 1;
    remaining %= 86400;
    let hour = remaining / 3600;
    remaining %= 3600;
    let minute = remaining / 60;
    let second = remaining % 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    )
}

// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
pub fn get_traffic_baseline(
    detector: State<'_, Arc<AnomalyDetector>>,
    device_id: Option<i64>,
) -> TrafficBaseline {
    detector.get_baseline(device_id)
}

#[tauri::command]
pub fn scan_request_anomalies(
    detector: State<'_, Arc<AnomalyDetector>>,
    device_id: Option<i64>,
    host: String,
    ip: Option<String>,
    req_body: Option<String>,
    resp_body: Option<String>,
) -> Result<AnomalyScanResult, String> {
    detector.scan_request(
        device_id,
        &host,
        ip.as_deref(),
        req_body.as_deref(),
        resp_body.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;

    // ------------------------------------------------------------------
    // PrivacyScanner
    // ------------------------------------------------------------------

    #[test]
    fn test_scanner_finds_idfa() {
        let scanner = PrivacyScanner::new();
        let results = scanner.scan("x-idfa: 12345678-1234-5678-9ABC-123456789012");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pattern, PrivacyPattern::IDFA);
        // Matched text is uppercased; digits and already-upper letters unchanged.
        assert_eq!(
            results[0].matched_text,
            "12345678-1234-5678-9ABC-123456789012"
        );
    }

    #[test]
    fn test_scanner_finds_phone_e164() {
        let scanner = PrivacyScanner::new();
        let results = scanner.scan("call +14155551234 now");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pattern, PrivacyPattern::PhoneNumber);
        assert_eq!(results[0].matched_text, "+14155551234");
    }

    #[test]
    fn test_scanner_rejects_short_phone() {
        let scanner = PrivacyScanner::new();
        // Only 3 digits after the leading `+` — regex requires 7-15.
        let results = scanner.scan("+123");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_scanner_finds_gps_coords() {
        let scanner = PrivacyScanner::new();
        let results = scanner.scan("latitude:37.7749,-122.4194");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pattern, PrivacyPattern::GpsCoordinates);
    }

    #[test]
    fn test_scanner_empty_text_returns_no_results() {
        let scanner = PrivacyScanner::new();
        let results = scanner.scan("");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_scanner_finds_multiple_patterns_in_same_text() {
        let scanner = PrivacyScanner::new();
        let text =
            "IDFA: 12345678-1234-5678-9ABC-123456789012 phone: +14155551234 lat:37.7749,-122.4194";
        let results = scanner.scan(text);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_extract_context_truncates_long_text() {
        let scanner = PrivacyScanner::new();
        // 50 chars of padding on each side, match is in the middle — context window
        // (30 chars each side) gets clipped on both ends. Trailing space on the prefix
        // (and leading space on the suffix) ensures the IDFA's `\b` word boundary fires.
        let prefix = format!("{} ", "a".repeat(49));
        let suffix = format!(" {}", "b".repeat(49));
        let text = format!("{}12345678-1234-5678-9ABC-123456789012{}", prefix, suffix);
        let results = scanner.scan(&text);
        assert_eq!(results.len(), 1);
        assert!(
            results[0].context.starts_with("..."),
            "context should have leading ellipsis: {:?}",
            results[0].context
        );
        assert!(
            results[0].context.ends_with("..."),
            "context should have trailing ellipsis: {:?}",
            results[0].context
        );
    }

    #[test]
    fn test_extract_context_no_truncation_for_short_text() {
        let scanner = PrivacyScanner::new();
        // Text is short enough that the 30-char context window fully contains it.
        let text = "a 12345678-1234-5678-9ABC-123456789012 b";
        let results = scanner.scan(text);
        assert_eq!(results.len(), 1);
        assert!(
            !results[0].context.contains("..."),
            "context should not be truncated: {:?}",
            results[0].context
        );
    }

    // ------------------------------------------------------------------
    // BaselineStore
    // ------------------------------------------------------------------

    #[test]
    fn test_baseline_new_domain_for_device() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("baseline.json");
        let store = BaselineStore::with_path(path);
        assert!(store.is_new_domain(Some(1), "example.com"));
    }

    #[test]
    fn test_baseline_existing_domain_not_new() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("baseline.json");
        let store = BaselineStore::with_path(path);
        store.add_domain(Some(1), "example.com");
        assert!(!store.is_new_domain(Some(1), "example.com"));
    }

    #[test]
    fn test_baseline_different_device_treats_as_new() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("baseline.json");
        let store = BaselineStore::with_path(path);
        store.add_domain(Some(1), "a.com");
        assert!(store.is_new_domain(Some(2), "a.com"));
    }

    #[test]
    fn test_baseline_add_increments_count() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("baseline.json");
        let store = BaselineStore::with_path(path);
        store.add_domain(Some(1), "counter.com");
        store.add_domain(Some(1), "counter.com");
        let baseline = store.get_baseline(Some(1));
        let entry = baseline
            .domains
            .iter()
            .find(|e| e.value == "counter.com")
            .expect("counter.com should exist in baseline");
        assert_eq!(entry.count, 2);
    }

    #[test]
    fn test_baseline_persists_across_reload() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("baseline.json");
        {
            let store = BaselineStore::with_path(path.clone());
            store.add_domain(Some(1), "example.com");
        }
        let store = BaselineStore::with_path(path);
        let baseline = store.get_baseline(Some(1));
        assert_eq!(baseline.domains.len(), 1);
        assert_eq!(baseline.domains[0].value, "example.com");
    }

    #[test]
    fn test_baseline_is_new_ip_separate_from_domain() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("baseline.json");
        let store = BaselineStore::with_path(path);
        assert!(store.is_new_ip(Some(1), "1.2.3.4"));
        store.add_ip(Some(1), "1.2.3.4");
        assert!(!store.is_new_ip(Some(1), "1.2.3.4"));
    }

    // ------------------------------------------------------------------
    // AnomalyDetector integration
    // ------------------------------------------------------------------

    #[test]
    fn test_detector_scan_request_generates_alert_for_new_domain() {
        let dir = tempdir().unwrap();
        let baseline_path = dir.path().join("baseline.json");
        let alerts = Arc::new(DbState::new_in_memory(std::sync::Mutex::new(())).unwrap());
        let baseline = Arc::new(BaselineStore::with_path(baseline_path));
        let detector = AnomalyDetector::with_stores(alerts.clone(), baseline);

        let result = detector
            .scan_request(Some(1), "fresh.example.com", None, None, None)
            .unwrap();
        assert!(
            result.alerts_generated >= 1,
            "expected at least one alert for a brand-new domain, got {}",
            result.alerts_generated
        );
        assert!(result
            .new_domains
            .contains(&"fresh.example.com".to_string()));
        let persisted = alerts
            .alerts(&crate::alerts::AlertQuery::default())
            .unwrap();
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].alert_type, AlertType::NewDomain);
    }

    #[test]
    fn test_detector_scan_request_no_alert_for_seen_domain() {
        let dir = tempdir().unwrap();
        let baseline_path = dir.path().join("baseline.json");
        let alerts = Arc::new(DbState::new_in_memory(std::sync::Mutex::new(())).unwrap());
        let baseline = Arc::new(BaselineStore::with_path(baseline_path));
        let detector = AnomalyDetector::with_stores(alerts, baseline);

        // First call populates the baseline.
        detector
            .scan_request(Some(1), "seen.example.com", None, None, None)
            .unwrap();
        // Second call: in-memory cache (and baseline) short-circuit, no new domains.
        let result2 = detector
            .scan_request(Some(1), "seen.example.com", None, None, None)
            .unwrap();
        assert!(
            result2.new_domains.is_empty(),
            "expected no new_domains on second scan, got {:?}",
            result2.new_domains
        );
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    #[test]
    fn test_chrono_lite_timestamp_format() {
        let ts = chrono_lite_timestamp();
        assert_eq!(ts.len(), 19, "unexpected length: {:?}", ts);
        let bytes = ts.as_bytes();
        assert_eq!(bytes[4], b'-', "expected '-' at index 4, got {:?}", ts);
        assert_eq!(bytes[7], b'-', "expected '-' at index 7, got {:?}", ts);
        assert_eq!(bytes[10], b' ', "expected ' ' at index 10, got {:?}", ts);
        assert_eq!(bytes[13], b':', "expected ':' at index 13, got {:?}", ts);
        assert_eq!(bytes[16], b':', "expected ':' at index 16, got {:?}", ts);
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2000), "2000 should be a leap year");
        assert!(!is_leap_year(1900), "1900 should NOT be a leap year");
        assert!(is_leap_year(2024), "2024 should be a leap year");
        assert!(!is_leap_year(2023), "2023 should NOT be a leap year");
    }
}
