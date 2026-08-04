//! Authoritative Alert domain and persistence Module.
//!
//! Producers publish [`NewAlert`] values. Desktop and MCP Adapters query and
//! acknowledge the same SQLite-backed [`Alert`] records through [`DbState`].

use crate::db::{chrono_lite_timestamp, DbState};
use proxybot_core::desktop_contract::{DesktopContractType, WireType};
use rusqlite::{params, types::Type, Connection, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tauri::State;

const DEFAULT_ALERT_LIMIT: usize = 100;
const MAX_ALERT_LIMIT: usize = 1_000;
const LEGACY_ALERT_IMPORT: &str = "alerts-json-v1";

/// Alert severity is the urgency assigned by an Alert producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl AlertSeverity {
    fn storage_name(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    fn from_storage(value: &str) -> Result<Self, String> {
        match value {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "critical" => Ok(Self::Critical),
            _ => Err(format!("unknown Alert Severity: {value}")),
        }
    }
}

impl WireType for AlertSeverity {
    fn type_script_type() -> String {
        "AlertSeverity".to_owned()
    }
}

impl DesktopContractType for AlertSeverity {
    const NAME: &'static str = "AlertSeverity";

    fn type_script_declaration() -> String {
        "export type AlertSeverity = \"Info\" | \"Warning\" | \"Critical\";\n".to_owned()
    }
}

/// Alert Type identifies the condition reported by an Alert producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    NewDomain,
    NewIp,
    PrivacyExfil,
    AuthAnomaly,
    UntrustedCert,
}

impl WireType for AlertType {
    fn type_script_type() -> String {
        "AlertType".to_owned()
    }
}

impl DesktopContractType for AlertType {
    const NAME: &'static str = "AlertType";

    fn type_script_declaration() -> String {
        "export type AlertType = \"NewDomain\" | \"NewIp\" | \"PrivacyExfil\" | \"AuthAnomaly\" | \"UntrustedCert\";\n".to_owned()
    }
}

proxybot_core::desktop_contract_type! {
    /// Persisted security or anomaly fact presented by every Adapter.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Alert {
        pub id: i64,
        pub device_id: Option<i64>,
        pub severity: AlertSeverity,
        pub alert_type: AlertType,
        pub details: String,
        pub created_at: String,
        pub acknowledged: bool,
    }
}

/// Producer input. SQLite owns identifiers, timestamps and acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAlert {
    pub device_id: Option<i64>,
    pub severity: AlertSeverity,
    pub alert_type: AlertType,
    pub details: String,
    /// Optional producer-owned identity used for idempotent publication.
    pub occurrence_key: Option<String>,
}

/// Shared query Interface used by Desktop, MCP and internal consumers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlertQuery {
    pub device_id: Option<i64>,
    pub severity: Option<AlertSeverity>,
    pub since: Option<String>,
    pub acknowledged: Option<bool>,
    pub limit: Option<usize>,
}

impl DbState {
    /// Publish one Alert and return the authoritative persisted record.
    pub fn publish_alert(&self, alert: NewAlert) -> Result<Alert, String> {
        let created_at = chrono_lite_timestamp();
        let conn = self.conn.lock().map_err(|error| error.to_string())?;
        insert_alert(&conn, alert, &created_at, false)
    }

    /// Query Alerts in deterministic newest-first order.
    pub fn alerts(&self, query: &AlertQuery) -> Result<Vec<Alert>, String> {
        let conn = self.conn.lock().map_err(|error| error.to_string())?;
        query_alerts(&conn, query)
    }

    /// Acknowledge one Alert and return its updated authoritative record.
    pub fn acknowledge_alert(&self, alert_id: i64) -> Result<Alert, String> {
        let conn = self.conn.lock().map_err(|error| error.to_string())?;
        let changed = conn
            .execute(
                "UPDATE alerts SET acknowledged = 1 WHERE id = ?1",
                params![alert_id],
            )
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err("Alert not found".to_owned());
        }
        find_alert(&conn, alert_id)?.ok_or_else(|| "Alert not found".to_owned())
    }

    /// Count all unacknowledged Alerts.
    pub fn unacknowledged_alert_count(&self) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|error| error.to_string())?;
        conn.query_row(
            "SELECT COUNT(*) FROM alerts WHERE acknowledged = 0",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
    }

    /// Import the retired JSON Alert store once, preserving user-visible facts.
    /// The JSON file is never read again after the transaction records success.
    pub fn import_legacy_alerts(&self, path: &Path) -> Result<usize, String> {
        if !path.is_file() {
            return Ok(0);
        }

        {
            let conn = self.conn.lock().map_err(|error| error.to_string())?;
            if legacy_import_complete(&conn)? {
                return Ok(0);
            }
        }

        let data = std::fs::read(path).map_err(|error| error.to_string())?;
        let legacy: LegacyAlertStore =
            serde_json::from_slice(&data).map_err(|error| error.to_string())?;
        let mut conn = self.conn.lock().map_err(|error| error.to_string())?;
        let transaction = conn.transaction().map_err(|error| error.to_string())?;
        if legacy_import_complete(&transaction)? {
            return Ok(0);
        }

        for alert in &legacy.alerts {
            insert_alert(
                &transaction,
                NewAlert {
                    device_id: alert.device_id,
                    severity: alert.severity,
                    alert_type: alert.alert_type,
                    details: alert.details.clone(),
                    occurrence_key: None,
                },
                &alert.created_at,
                alert.acknowledged,
            )?;
        }
        transaction
            .execute(
                "INSERT INTO legacy_imports (name, imported_at) VALUES (?1, ?2)",
                params![LEGACY_ALERT_IMPORT, chrono_lite_timestamp()],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(legacy.alerts.len())
    }
}

fn legacy_import_complete(conn: &Connection) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM legacy_imports WHERE name = ?1)",
        params![LEGACY_ALERT_IMPORT],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

fn insert_alert(
    conn: &Connection,
    alert: NewAlert,
    created_at: &str,
    acknowledged: bool,
) -> Result<Alert, String> {
    let alert_type = serde_json::to_string(&alert.alert_type).map_err(|error| error.to_string())?;
    let occurrence_key = alert.occurrence_key.clone();
    let changed = conn.execute(
        "INSERT INTO alerts (device_id, severity, alert_type, details, created_at, acknowledged, occurrence_key)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT DO NOTHING",
        params![
            alert.device_id,
            alert.severity.storage_name(),
            alert_type,
            alert.details,
            created_at,
            acknowledged,
            occurrence_key
        ],
    )
    .map_err(|error| error.to_string())?;
    if changed == 0 {
        return find_alert_by_occurrence(
            conn,
            occurrence_key
                .as_deref()
                .ok_or_else(|| "Alert publication made no change".to_owned())?,
        )?
        .ok_or_else(|| "Idempotent Alert publication lost its occurrence".to_owned());
    }
    let id = conn.last_insert_rowid();
    find_alert(conn, id)?.ok_or_else(|| format!("Alert {id} was not persisted"))
}

fn find_alert(conn: &Connection, alert_id: i64) -> Result<Option<Alert>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, device_id, severity, alert_type, details, created_at, acknowledged
             FROM alerts WHERE id = ?1",
        )
        .map_err(|error| error.to_string())?;
    let mut rows = statement
        .query_map(params![alert_id], map_alert)
        .map_err(|error| error.to_string())?;
    rows.next().transpose().map_err(|error| error.to_string())
}

fn find_alert_by_occurrence(
    conn: &Connection,
    occurrence_key: &str,
) -> Result<Option<Alert>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, device_id, severity, alert_type, details, created_at, acknowledged
             FROM alerts WHERE occurrence_key = ?1",
        )
        .map_err(|error| error.to_string())?;
    let mut rows = statement
        .query_map(params![occurrence_key], map_alert)
        .map_err(|error| error.to_string())?;
    rows.next().transpose().map_err(|error| error.to_string())
}

fn query_alerts(conn: &Connection, query: &AlertQuery) -> Result<Vec<Alert>, String> {
    let severity = query.severity.map(AlertSeverity::storage_name);
    let acknowledged = query.acknowledged.map(i64::from);
    let limit = query
        .limit
        .unwrap_or(DEFAULT_ALERT_LIMIT)
        .min(MAX_ALERT_LIMIT) as i64;
    let mut statement = conn
        .prepare(
            "SELECT id, device_id, severity, alert_type, details, created_at, acknowledged
             FROM alerts
             WHERE (?1 IS NULL OR device_id = ?1)
               AND (?2 IS NULL OR severity = ?2)
               AND (?3 IS NULL OR created_at > ?3)
               AND (?4 IS NULL OR acknowledged = ?4)
             ORDER BY created_at DESC, id DESC
             LIMIT ?5",
        )
        .map_err(|error| error.to_string())?;
    let alerts = statement
        .query_map(
            params![query.device_id, severity, query.since, acknowledged, limit],
            map_alert,
        )
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(alerts)
}

fn map_alert(row: &Row<'_>) -> rusqlite::Result<Alert> {
    let severity: String = row.get(2)?;
    let alert_type: String = row.get(3)?;
    Ok(Alert {
        id: row.get(0)?,
        device_id: row.get(1)?,
        severity: AlertSeverity::from_storage(&severity)
            .map_err(|error| invalid_alert_value(2, severity, error))?,
        alert_type: serde_json::from_str(&alert_type)
            .map_err(|error| invalid_alert_value(3, alert_type, error.to_string()))?,
        details: row.get(4)?,
        created_at: row.get(5)?,
        acknowledged: row.get::<_, i64>(6)? != 0,
    })
}

fn invalid_alert_value(index: usize, value: String, error: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{error}; stored value: {value}"),
        )),
    )
}

#[derive(Debug, Deserialize)]
struct LegacyAlertStore {
    #[allow(dead_code)]
    version: u32,
    alerts: Vec<Alert>,
}

/// Desktop Adapter: query Alerts through the shared domain Interface.
#[tauri::command]
pub fn get_alerts(
    db: State<'_, Arc<DbState>>,
    device_id: Option<i64>,
    severity: Option<AlertSeverity>,
    since: Option<String>,
    acknowledged: Option<bool>,
    limit: Option<usize>,
) -> Result<Vec<Alert>, String> {
    db.alerts(&AlertQuery {
        device_id,
        severity,
        since,
        acknowledged,
        limit,
    })
}

/// Desktop Adapter: acknowledge one persisted Alert.
#[tauri::command]
pub fn acknowledge_alert(db: State<'_, Arc<DbState>>, alert_id: i64) -> Result<Alert, String> {
    db.acknowledge_alert(alert_id)
}

/// Desktop Adapter: count all unacknowledged Alerts.
#[tauri::command]
pub fn get_alert_count(db: State<'_, Arc<DbState>>) -> Result<i64, String> {
    db.unacknowledged_alert_count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn database() -> DbState {
        DbState::new_in_memory(std::sync::Mutex::new(())).unwrap()
    }

    fn draft(severity: AlertSeverity, details: &str) -> NewAlert {
        NewAlert {
            device_id: None,
            severity,
            alert_type: AlertType::NewDomain,
            details: details.to_owned(),
            occurrence_key: None,
        }
    }

    #[test]
    fn publish_filter_order_and_acknowledge_share_one_store() {
        let db = database();
        let first = db
            .publish_alert(draft(AlertSeverity::Info, "first"))
            .unwrap();
        let second = db
            .publish_alert(draft(AlertSeverity::Warning, "second"))
            .unwrap();

        let warnings = db
            .alerts(&AlertQuery {
                severity: Some(AlertSeverity::Warning),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(warnings, vec![second.clone()]);
        assert_eq!(db.unacknowledged_alert_count().unwrap(), 2);

        let acknowledged = db.acknowledge_alert(first.id).unwrap();
        assert!(acknowledged.acknowledged);
        assert_eq!(db.unacknowledged_alert_count().unwrap(), 1);

        let all = db.alerts(&AlertQuery::default()).unwrap();
        assert_eq!(
            all.iter().map(|alert| alert.id).collect::<Vec<_>>(),
            vec![second.id, first.id]
        );
    }

    #[test]
    fn acknowledge_rejects_unknown_identifier() {
        let error = database().acknowledge_alert(404).unwrap_err();
        assert!(error.contains("not found"));
    }

    #[test]
    fn corrupt_stored_vocabulary_is_not_silently_relabelled() {
        let db = database();
        db.conn
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO alerts (severity, alert_type, details, created_at)
                 VALUES ('urgent', '\"AuthAnomaly\"', 'corrupt', '2026-01-01 00:00:00')",
                [],
            )
            .unwrap();
        let error = db.alerts(&AlertQuery::default()).unwrap_err();
        assert!(error.contains("unknown Alert Severity"));
    }

    #[test]
    fn repeated_occurrence_is_published_once() {
        let db = database();
        let mut alert = draft(AlertSeverity::Warning, "auth anomaly");
        alert.occurrence_key = Some("auth:none:request-7".to_owned());
        let first = db.publish_alert(alert.clone()).unwrap();
        let repeated = db.publish_alert(alert).unwrap();
        assert_eq!(repeated, first);
        assert_eq!(db.alerts(&AlertQuery::default()).unwrap().len(), 1);
    }

    #[test]
    fn imports_legacy_json_exactly_once() {
        let db = database();
        let directory = tempdir().unwrap();
        let path = directory.path().join("alerts.json");
        std::fs::write(
            &path,
            r#"{"version":1,"alerts":[{"id":8,"device_id":null,"severity":"Critical","alert_type":"UntrustedCert","details":"legacy","created_at":"2026-01-02 03:04:05","acknowledged":true}]}"#,
        )
        .unwrap();

        assert_eq!(db.import_legacy_alerts(&path).unwrap(), 1);
        std::fs::write(&path, b"retired store is no longer read").unwrap();
        assert_eq!(db.import_legacy_alerts(&path).unwrap(), 0);
        let alerts = db.alerts(&AlertQuery::default()).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].details, "legacy");
        assert!(alerts[0].acknowledged);
    }

    #[test]
    fn independent_adapters_observe_the_same_sqlite_alerts() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("proxybot.db");
        let desktop = DbState::open(&path).unwrap();
        let mcp = DbState::open(&path).unwrap();

        let published = desktop
            .publish_alert(draft(AlertSeverity::Critical, "shared database"))
            .unwrap();
        assert_eq!(
            mcp.alerts(&AlertQuery::default()).unwrap(),
            vec![published.clone()]
        );

        mcp.acknowledge_alert(published.id).unwrap();
        assert_eq!(desktop.unacknowledged_alert_count().unwrap(), 0);
        assert!(desktop.alerts(&AlertQuery::default()).unwrap()[0].acknowledged);
    }
}
