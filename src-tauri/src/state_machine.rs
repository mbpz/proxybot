//! State machine visualizer module for auth flows.
//!
//! Extracts login → token → resource lifecycle from DAG.
//! Outputs Mermaid markdown diagrams and flags anomalous transitions.

use crate::db::{chrono_lite_timestamp, DbState};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

/// A DAG node: (id, timestamp_ms, method, host, path).
type DagNode = (i64, String, String, String, String);
/// A DAG edge: (from_node_id, to_node_id, token_value).
type DagEdge = (i64, i64, String);

/// Alert severity levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl AlertSeverity {
    fn as_str(&self) -> &str {
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

/// State in the auth flow state machine.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuthState {
    pub id: String,
    pub label: String,
    pub state_type: AuthStateType,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum AuthStateType {
    Initial,
    Login,
    Authenticated,
    Resource,
    Logout,
    Error,
}

/// Transition in the auth flow state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTransition {
    pub from_state: String,
    pub to_state: String,
    pub request_id: i64,
    pub method: String,
    pub path: String,
    pub token_type: Option<String>,
    pub is_anomalous: bool,
    pub anomaly_reason: Option<String>,
}

/// Complete auth state machine for a device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStateMachine {
    pub device_id: Option<i64>,
    pub states: Vec<AuthState>,
    pub transitions: Vec<AuthTransition>,
    pub mermaid_md: String,
    pub anomalies: Vec<Anomaly>,
}

/// Detected anomaly in the auth flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub request_id: i64,
    pub anomaly_type: String,
    pub description: String,
    pub severity: AlertSeverity,
}

/// Auth flow extractor from DAG data.
pub struct AuthFlowExtractor {
    // Token to state mapping
    #[allow(dead_code)]
    token_states: HashMap<String, AuthState>,
    // Request ID to path mapping
    #[allow(dead_code)]
    request_paths: HashMap<i64, (String, String)>, // (method, path)
}

impl AuthFlowExtractor {
    pub fn new() -> Self {
        Self {
            token_states: HashMap::new(),
            request_paths: HashMap::new(),
        }
    }

    /// Extract auth states and transitions from DAG nodes and edges.
    pub fn extract_auth_flow(
        &mut self,
        nodes: &[(i64, String, String, String, String)], // id, timestamp, method, host, path
        edges: &[(i64, i64, String)],                   // from_node_id, to_node_id, token_value
    ) -> (Vec<AuthState>, Vec<AuthTransition>, Vec<Anomaly>) {
        let mut states: Vec<AuthState> = Vec::new();
        let mut transitions: Vec<AuthTransition> = Vec::new();
        let mut anomalies: Vec<Anomaly> = Vec::new();

        // Initial state
        let initial_state = AuthState {
            id: "initial".to_string(),
            label: "Initial".to_string(),
            state_type: AuthStateType::Initial,
        };
        states.push(initial_state.clone());

        // Track login state
        let mut login_state: Option<AuthState> = None;
        let mut authenticated_state: Option<AuthState> = None;
        let mut token_to_authenticated: HashMap<String, String> = HashMap::new();

        // Sort nodes by timestamp
        let mut sorted_nodes: Vec<_> = nodes.iter().enumerate().collect();
        sorted_nodes.sort_by(|a, b| a.1.1.cmp(&b.1.1));

        for (idx, (id, _timestamp, method, host, path)) in sorted_nodes {
            let full_path = format!("{}://{}{}", "https", host, path);

            // Classify the request
            let (state_type, _token_type) = self.classify_request(method, path);

            match state_type {
                AuthStateType::Login => {
                    if login_state.is_none() {
                        let ls = AuthState {
                            id: format!("login_{}", id),
                            label: format!("Login ({})", method),
                            state_type: AuthStateType::Login,
                        };
                        login_state = Some(ls.clone());
                        states.push(ls.clone());

                        // Transition from initial to login
                        transitions.push(AuthTransition {
                            from_state: initial_state.id.clone(),
                            to_state: ls.id.clone(),
                            request_id: *id,
                            method: method.clone(),
                            path: full_path.clone(),
                            token_type: None,
                            is_anomalous: false,
                            anomaly_reason: None,
                        });
                    }

                    // Check for response tokens (access_token, sessionId, etc.)
                    for edge in edges.iter().filter(|e| e.0 == *id) {
                        let token_value = &edge.2;
                        if Self::is_auth_token(token_value) {
                            let as_id = format!("auth_{}_{}", id, edge.1);
                            let as_state = AuthState {
                                id: as_id.clone(),
                                label: format!("Auth ({}...)", &token_value[..token_value.len().min(8)]),
                                state_type: AuthStateType::Authenticated,
                            };
                            authenticated_state = Some(as_state.clone());
                            states.push(as_state.clone());
                            token_to_authenticated.insert(token_value.clone(), as_state.id.clone());

                            transitions.push(AuthTransition {
                                from_state: login_state.as_ref().unwrap().id.clone(),
                                to_state: as_state.id.clone(),
                                request_id: *id,
                                method: method.clone(),
                                path: full_path.clone(),
                                token_type: Some(token_value.clone()),
                                is_anomalous: false,
                                anomaly_reason: None,
                            });
                        }
                    }
                }
                AuthStateType::Resource => {
                    // Check if we have authenticated state
                    let requires_auth = self.requires_auth_token(path);
                    if requires_auth {
                        if authenticated_state.is_none() {
                            // Anomaly: resource accessed before login
                            let anomaly = Anomaly {
                                request_id: *id,
                                anomaly_type: "AUTH_ANOMALY".to_string(),
                                description: format!(
                                    "Request to {} {} appears to require auth but no login was detected",
                                    method, path
                                ),
                                severity: AlertSeverity::Warning,
                            };
                            anomalies.push(anomaly);

                            // Still create the state but mark as anomalous transition
                            let rs = AuthState {
                                id: format!("resource_{}_{}", id, idx),
                                label: format!("Resource ({})", method),
                                state_type: AuthStateType::Resource,
                            };
                            states.push(rs.clone());

                            let from_state = login_state.as_ref().map(|s| s.id.clone())
                                .unwrap_or_else(|| initial_state.id.clone());
                            transitions.push(AuthTransition {
                                from_state,
                                to_state: rs.id.clone(),
                                request_id: *id,
                                method: method.clone(),
                                path: full_path.clone(),
                                token_type: None,
                                is_anomalous: true,
                                anomaly_reason: Some("Resource accessed before authentication".to_string()),
                            });
                        } else {
                            // Normal authenticated resource access
                            let rs = AuthState {
                                id: format!("resource_{}_{}", id, idx),
                                label: format!("Resource ({})", method),
                                state_type: AuthStateType::Resource,
                            };
                            states.push(rs.clone());

                            transitions.push(AuthTransition {
                                #[allow(clippy::unnecessary_unwrap)]
                                from_state: authenticated_state.as_ref().unwrap().id.clone(),
                                to_state: rs.id.clone(),
                                request_id: *id,
                                method: method.clone(),
                                path: full_path.clone(),
                                token_type: None,
                                is_anomalous: false,
                                anomaly_reason: None,
                            });
                        }
                    } else {
                        // Public resource
                        let rs = AuthState {
                            id: format!("resource_{}_{}", id, idx),
                            label: format!("Resource ({})", method),
                            state_type: AuthStateType::Resource,
                        };
                        states.push(rs.clone());

                        transitions.push(AuthTransition {
                            from_state: initial_state.id.clone(),
                            to_state: rs.id.clone(),
                            request_id: *id,
                            method: method.clone(),
                            path: full_path.clone(),
                            token_type: None,
                            is_anomalous: false,
                            anomaly_reason: None,
                        });
                    }
                }
                _ => {}
            }
        }

        (states, transitions, anomalies)
    }

    /// Classify a request as login, resource, etc.
    fn classify_request(&self, method: &str, path: &str) -> (AuthStateType, Option<String>) {
        let path_lower = path.to_lowercase();
        let _method_upper = method.to_uppercase();

        // Login patterns
        if path_lower.contains("login")
            || path_lower.contains("signin")
            || path_lower.contains("auth")
            || path_lower.contains("token")
            || path_lower.contains("session") {
            return (AuthStateType::Login, Some("session token".to_string()));
        }

        // Logout patterns
        if path_lower.contains("logout") || path_lower.contains("signout") {
            return (AuthStateType::Logout, None);
        }

        // Authenticated endpoints
        if path_lower.contains("profile")
            || path_lower.contains("user")
            || path_lower.contains("account")
            || path_lower.contains("me")
            || path_lower.contains("order")
            || path_lower.contains("payment") {
            return (AuthStateType::Resource, Some("access_token".to_string()));
        }

        (AuthStateType::Resource, None)
    }

    /// Check if a request path typically requires auth.
    fn requires_auth_token(&self, path: &str) -> bool {
        let path_lower = path.to_lowercase();
        path_lower.contains("profile")
            || path_lower.contains("user")
            || path_lower.contains("account")
            || path_lower.contains("me")
            || path_lower.contains("order")
            || path_lower.contains("payment")
            || path_lower.contains("api")
            || path_lower.contains("v2")
            || path_lower.contains("v3")
    }

    /// Check if a token value looks like an auth token.
    fn is_auth_token(token: &str) -> bool {
        token.len() >= 16
    }
}

impl Default for AuthFlowExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate Mermaid state diagram markdown.
pub fn generate_mermaid_md(states: &[AuthState], transitions: &[AuthTransition]) -> String {
    let mut md = String::from("stateDiagram-v2\n");

    // Add states
    for state in states {
        let style = match state.state_type {
            AuthStateType::Initial => "[*] --> ",
            AuthStateType::Login => "[*] --> ",
            AuthStateType::Authenticated => "",
            AuthStateType::Resource => "",
            AuthStateType::Logout => "",
            AuthStateType::Error => "",
        };

        if !style.is_empty() {
            md.push_str(&format!("    {} {}\n", style, state.label));
        } else {
            md.push_str(&format!("    {} : {}\n", state.id, state.label));
        }
    }

    md.push('\n');

    // Add transitions
    for trans in transitions {
        let anomaly_note = if trans.is_anomalous {
            " --> \"**ANOMALY**\""
        } else {
            ""
        };
        md.push_str(&format!(
            "    {} --> {}{}\n",
            trans.from_state, trans.to_state, anomaly_note
        ));
    }

    md
}

// ============================================================================
// Database Operations
// ============================================================================

/// Store alert in database.
pub fn store_alert(
    db_state: &DbState,
    device_id: Option<i64>,
    severity: &AlertSeverity,
    alert_type: &AlertType,
    details: &str,
) -> Result<i64, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let now = chrono_lite_timestamp();

    conn.execute(
        "INSERT INTO alerts (device_id, severity, alert_type, details, created_at, acknowledged)
         VALUES (?1, ?2, ?3, ?4, ?5, 0)",
        params![
            device_id,
            severity.as_str(),
            serde_json::to_string(alert_type).unwrap_or_default(),
            details,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

/// Get alerts from database.
pub fn get_alerts(
    db_state: &DbState,
    device_id: Option<i64>,
    severity_filter: Option<&str>,
    limit: i64,
) -> Result<Vec<Alert>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    let query = if device_id.is_some() && severity_filter.is_some() {
        "SELECT id, device_id, severity, alert_type, details, created_at, acknowledged
         FROM alerts WHERE device_id = ?1 AND severity = ?2 ORDER BY created_at DESC LIMIT ?3"
    } else if device_id.is_some() {
        "SELECT id, device_id, severity, alert_type, details, created_at, acknowledged
         FROM alerts WHERE device_id = ?1 ORDER BY created_at DESC LIMIT ?3"
    } else if severity_filter.is_some() {
        "SELECT id, device_id, severity, alert_type, details, created_at, acknowledged
         FROM alerts WHERE severity = ?2 ORDER BY created_at DESC LIMIT ?3"
    } else {
        "SELECT id, device_id, severity, alert_type, details, created_at, acknowledged
         FROM alerts ORDER BY created_at DESC LIMIT ?3"
    };

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;

    let alerts: Vec<Alert> = if let (Some(did), Some(sev)) = (device_id, severity_filter) {
        stmt.query_map(params![did, sev, limit], row_mapper)
    } else if let Some(did) = device_id {
        stmt.query_map(params![did, limit], row_mapper)
    } else if let Some(sev) = severity_filter {
        stmt.query_map(params![sev, limit], row_mapper)
    } else {
        stmt.query_map(params![limit], row_mapper)
    }
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;

    Ok(alerts)
}

fn row_mapper(row: &rusqlite::Row) -> Result<Alert, rusqlite::Error> {
    let severity_str: String = row.get(2)?;
    let alert_type_str: String = row.get(3)?;

    Ok(Alert {
        id: row.get(0)?,
        device_id: row.get(1)?,
        severity: match severity_str.as_str() {
            "info" => AlertSeverity::Info,
            "warning" => AlertSeverity::Warning,
            "critical" => AlertSeverity::Critical,
            _ => AlertSeverity::Info,
        },
        alert_type: serde_json::from_str(&alert_type_str).unwrap_or(AlertType::AuthAnomaly),
        details: row.get(4)?,
        created_at: row.get(5)?,
        acknowledged: row.get::<_, i64>(6)? != 0,
    })
}

/// Acknowledge an alert.
pub fn acknowledge_alert(db_state: &DbState, alert_id: i64) -> Result<(), String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE alerts SET acknowledged = 1 WHERE id = ?1",
        params![alert_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Get unacknowledged alert count.
#[allow(dead_code)]
pub fn get_unacknowledged_alert_count(db_state: &DbState) -> Result<i64, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM alerts WHERE acknowledged = 0",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count)
}

/// Build auth state machine from DAG data.
pub fn build_auth_state_machine(
    nodes: &[(i64, String, String, String, String)],
    edges: &[(i64, i64, String)],
    device_id: Option<i64>,
) -> AuthStateMachine {
    let mut extractor = AuthFlowExtractor::new();
    let (states, transitions, anomalies) = extractor.extract_auth_flow(nodes, edges);

    let mermaid_md = generate_mermaid_md(&states, &transitions);

    AuthStateMachine {
        device_id,
        states,
        transitions,
        mermaid_md,
        anomalies,
    }
}

/// Get DAG data for state machine building.
fn get_dag_data_for_device(
    conn: &rusqlite::Connection,
    device_id: Option<i64>,
) -> Result<(Vec<DagNode>, Vec<DagEdge>), String> {
    // Get nodes - collect inside each branch to avoid lifetime issues
    let nodes: Vec<DagNode> = if let Some(did) = device_id {
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, method, host, path FROM http_requests WHERE device_id = ?1 ORDER BY timestamp",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![did], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    } else {
        let mut stmt = conn
            .prepare("SELECT id, timestamp, method, host, path FROM http_requests ORDER BY timestamp")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    // Get edges
    let mut stmt = conn
        .prepare("SELECT from_node_id, to_node_id, token_value FROM dag_edges")
        .map_err(|e| e.to_string())?;
    let edges: Vec<(i64, i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok((nodes, edges))
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Get auth state machine for a device or all traffic.
#[tauri::command]
pub fn get_auth_state_machine(
    db_state: State<'_, Arc<DbState>>,
    device_id: Option<i64>,
) -> Result<AuthStateMachine, String> {
    let (nodes, edges) = {
        let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
        get_dag_data_for_device(&conn, device_id)?
    };

    let machine = build_auth_state_machine(&nodes, &edges, device_id);

    // Store any anomalies as alerts
    for anomaly in &machine.anomalies {
        let severity = if anomaly.severity == AlertSeverity::Warning {
            AlertSeverity::Warning
        } else {
            AlertSeverity::Info
        };
        let _ = store_alert(
            &db_state,
            device_id,
            &severity,
            &AlertType::AuthAnomaly,
            &anomaly.description,
        );
    }

    Ok(machine)
}

/// Get all alerts.
#[tauri::command]
pub fn get_alerts_cmd(
    db_state: State<'_, Arc<DbState>>,
    device_id: Option<i64>,
    severity: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<Alert>, String> {
    get_alerts(&db_state, device_id, severity.as_deref(), limit.unwrap_or(100))
}

/// Acknowledge an alert.
#[tauri::command]
pub fn acknowledge_alert_cmd(
    db_state: State<'_, Arc<DbState>>,
    alert_id: i64,
) -> Result<(), String> {
    acknowledge_alert(&db_state, alert_id)
}

/// Get unacknowledged alert count.
#[tauri::command]
#[allow(dead_code)]
pub fn get_alert_count_state_machine(db_state: State<'_, Arc<DbState>>) -> Result<i64, String> {
    get_unacknowledged_alert_count(&db_state)
}