//! Anomaly detection and privacy scanning module.
//!
//! Provides:
//! - Per-device traffic baseline (7-day rolling domain/IP frequency profile)
//! - New domain/IP detection (triggers info-level alerts)
//! - Privacy scanner (IDFA, phone E.164, GPS coordinates detection)

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

/// Alert severity levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            AlertSeverity::Info => "info",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Critical => "critical",
        }
    }
}

/// Alert type for categorization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    NewDomain,
    NewIp,
    PrivacyExfil,
    AuthAnomaly,
    UntrustedCert,
}

/// Alert record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: i64,
    pub device_id: Option<i64>,
    pub severity: AlertSeverity,
    pub alert_type: AlertType,
    pub details: String,
    pub created_at: String,
    pub acknowledged: bool,
}

/// Privacy pattern types for scanning results.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Alert store for persistent alert storage.
pub struct AlertStore {
    path: PathBuf,
    alerts: Mutex<Vec<Alert>>,
    next_id: Mutex<i64>,
}

impl AlertStore {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(home).join(".proxybot");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("alerts.json");
        let (alerts, next_id) = Self::load_from_file(&path);
        Self {
            path,
            alerts: Mutex::new(alerts),
            next_id: Mutex::new(next_id),
        }
    }

    fn load_from_file(path: &PathBuf) -> (Vec<Alert>, i64) {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return (Vec::new(), 1),
        };
        let reader = BufReader::new(file);
        match serde_json::from_reader::<_, AlertStoreData>(reader) {
            Ok(data) => {
                let next_id = data.alerts.iter().map(|a| a.id).max().unwrap_or(0) + 1;
                (data.alerts, next_id)
            }
            Err(_) => (Vec::new(), 1),
        }
    }

    fn save_to_file(&self) {
        let alerts = self.alerts.lock().unwrap();
        let data = AlertStoreData {
            version: 1,
            alerts: alerts.clone(),
        };
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

    pub fn add_alert(&self, alert: Alert) -> i64 {
        let mut alerts = self.alerts.lock().unwrap();
        let mut next_id = self.next_id.lock().unwrap();
        let mut new_alert = alert;
        new_alert.id = *next_id;
        *next_id += 1;
        alerts.push(new_alert.clone());
        // Keep only last 1000 alerts
        if alerts.len() > 1000 {
            let split_idx = alerts.len() - 1000;
            let old_alerts = std::mem::replace(&mut *alerts, Vec::new());
            *alerts = old_alerts.into_iter().skip(split_idx).collect();
        }
        drop(alerts);
        self.save_to_file();
        new_alert.id
    }

    pub fn get_alerts(&self, severity_filter: Option<&str>, limit: usize) -> Vec<Alert> {
        let alerts = self.alerts.lock().unwrap();
        let mut filtered: Vec<_> = alerts.iter()
            .filter(|a| {
                if let Some(sev) = severity_filter {
                    a.severity.as_str() == sev
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        filtered.sort_by(|a, b| b.id.cmp(&a.id));
        filtered.truncate(limit);
        filtered
    }

    pub fn acknowledge(&self, alert_id: i64) -> bool {
        let mut alerts = self.alerts.lock().unwrap();
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
            drop(alerts);
            self.save_to_file();
            return true;
        }
        false
    }

    pub fn unacknowledged_count(&self) -> i64 {
        let alerts = self.alerts.lock().unwrap();
        alerts.iter().filter(|a| !a.acknowledged).count() as i64
    }
}

#[derive(Serialize, Deserialize)]
struct AlertStoreData {
    version: u32,
    alerts: Vec<Alert>,
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

impl BaselineStore {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(home).join(".proxybot");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("baseline.json");
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
        match serde_json::from_reader::<_, BaselineData>(reader) {
            Ok(data) => data,
            Err(_) => BaselineData::default(),
        }
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
        !baselines.domains.iter().any(|e| {
            e.device_id == device_id &&
            e.domain == domain &&
            e.last_seen > seven_days_ago
        })
    }

    pub fn is_new_ip(&self, device_id: Option<i64>, ip: &str) -> bool {
        let baselines = self.baselines.lock().unwrap();
        let seven_days_ago = get_seven_days_ago();
        !baselines.ips.iter().any(|e| {
            e.device_id == device_id &&
            e.ip_address == ip &&
            e.last_seen > seven_days_ago
        })
    }

    pub fn add_domain(&self, device_id: Option<i64>, domain: &str) {
        let mut baselines = self.baselines.lock().unwrap();
        let now = chrono_lite_timestamp();
        let seven_days_ago = get_seven_days_ago();

        if let Some(entry) = baselines.domains.iter_mut().find(|e| e.device_id == device_id && e.domain == domain) {
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

        if let Some(entry) = baselines.ips.iter_mut().find(|e| e.device_id == device_id && e.ip_address == ip) {
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

        let domains: Vec<BaselineEntry> = baselines.domains.iter()
            .filter(|e| e.device_id == device_id && e.last_seen > seven_days_ago.clone())
            .map(|e| BaselineEntry {
                value: e.domain.clone(),
                count: e.count,
                first_seen: e.first_seen.clone(),
                last_seen: e.last_seen.clone(),
            })
            .collect();

        let ips: Vec<BaselineEntry> = baselines.ips.iter()
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
            r"\b[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}\b"
        ).unwrap();

        let phone_regex = Regex::new(r"\+\d{7,15}").unwrap();

        let gps_regex = Regex::new(
            r#"(?x)
            (?:["']?(?:latitude|lat|lng|longitude|long)["']?\s*[:=]\s*)?
            (?:["'])?
            (-?\d{1,3}\.\d{4,10})
            (?:["']?\s*[,;]\s*)
            (-?\d{1,3}\.\d{4,10})
            "#,
        ).unwrap();

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
        s.chars().filter(|c| *c != '-').all(|c| c.is_ascii_hexdigit())
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
    alert_store: Arc<AlertStore>,
    baseline_store: Arc<BaselineStore>,
    domain_cache: Mutex<HashSet<(Option<i64>, String)>>,
    ip_cache: Mutex<HashSet<(Option<i64>, String)>>,
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self {
            privacy_scanner: PrivacyScanner::new(),
            alert_store: Arc::new(AlertStore::new()),
            baseline_store: Arc::new(BaselineStore::new()),
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
    ) -> AnomalyScanResult {
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
            let _alert_id = self.alert_store.add_alert(Alert {
                id: 0,
                device_id,
                severity: AlertSeverity::Info,
                alert_type: AlertType::NewDomain,
                details,
                created_at: chrono_lite_timestamp(),
                acknowledged: false,
            });
            result.alerts_generated += 1;
        }

        if let Some(ip_addr) = ip {
            if self.is_new_ip(device_id, ip_addr) {
                result.new_ips.push(ip_addr.to_string());
                let details = format!("New IP accessed: {} (device: {:?})", ip_addr, device_id);
                let _alert_id = self.alert_store.add_alert(Alert {
                    id: 0,
                    device_id,
                    severity: AlertSeverity::Info,
                    alert_type: AlertType::NewIp,
                    details,
                    created_at: chrono_lite_timestamp(),
                    acknowledged: false,
                });
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
                    pattern_name,
                    finding.matched_text,
                    finding.context
                );
                let _alert_id = self.alert_store.add_alert(Alert {
                    id: 0,
                    device_id,
                    severity: AlertSeverity::Warning,
                    alert_type: AlertType::PrivacyExfil,
                    details,
                    created_at: chrono_lite_timestamp(),
                    acknowledged: false,
                });
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
                    pattern_name,
                    finding.matched_text,
                    finding.context
                );
                let _alert_id = self.alert_store.add_alert(Alert {
                    id: 0,
                    device_id,
                    severity: AlertSeverity::Warning,
                    alert_type: AlertType::PrivacyExfil,
                    details,
                    created_at: chrono_lite_timestamp(),
                    acknowledged: false,
                });
                result.alerts_generated += 1;
            }
            result.privacy_findings.extend(findings);
        }

        // Update baseline after scanning
        self.baseline_store.add_domain(device_id, host);
        if let Some(ip_addr) = ip {
            self.baseline_store.add_ip(device_id, ip_addr);
        }

        result
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

    pub fn get_alerts(&self, severity_filter: Option<&str>, limit: usize) -> Vec<Alert> {
        self.alert_store.get_alerts(severity_filter, limit)
    }

    pub fn acknowledge_alert(&self, alert_id: i64) -> bool {
        self.alert_store.acknowledge(alert_id)
    }

    pub fn get_unacknowledged_count(&self) -> i64 {
        self.alert_store.unacknowledged_count()
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
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

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
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
) -> AnomalyScanResult {
    detector.scan_request(
        device_id,
        &host,
        ip.as_deref(),
        req_body.as_deref(),
        resp_body.as_deref(),
    )
}

#[tauri::command]
pub fn get_alerts(
    detector: State<'_, Arc<AnomalyDetector>>,
    severity: Option<String>,
    limit: Option<usize>,
) -> Vec<Alert> {
    detector.get_alerts(severity.as_deref(), limit.unwrap_or(100))
}

#[tauri::command]
pub fn acknowledge_alert(
    detector: State<'_, Arc<AnomalyDetector>>,
    alert_id: i64,
) -> Result<(), String> {
    if detector.acknowledge_alert(alert_id) {
        Ok(())
    } else {
        Err("Alert not found".to_string())
    }
}

#[tauri::command]
pub fn get_alert_count(detector: State<'_, Arc<AnomalyDetector>>) -> i64 {
    detector.get_unacknowledged_count()
}
