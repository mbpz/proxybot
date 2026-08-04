//! State machine visualizer module for auth flows.
//!
//! Extracts login → token → resource lifecycle from DAG.
//! Outputs Mermaid markdown diagrams and flags anomalous transitions.

use crate::alerts::{AlertSeverity, AlertType, NewAlert};
use crate::analysis::CapturedRequestAnalysis;
use crate::db::{CapturedRequestOrder, CapturedRequestQuery, DbState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

/// A DAG edge: (from_node_id, to_node_id, token_value).
type DagEdge = (i64, i64, String);

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
    token_states: HashMap<String, AuthState>,
    // Request ID to path mapping
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
        nodes: &[CapturedRequestAnalysis],
        edges: &[(i64, i64, String)],
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
        sorted_nodes.sort_by(|a, b| {
            a.1.captured_at
                .cmp(&b.1.captured_at)
                .then_with(|| a.1.id.cmp(&b.1.id))
        });

        for (idx, request) in sorted_nodes {
            let full_path = format!("{}://{}{}", request.scheme, request.host, request.path);

            // Classify the request
            let (state_type, _token_type) = self.classify_request(&request.method, &request.path);

            match state_type {
                AuthStateType::Login => {
                    if login_state.is_none() {
                        let ls = AuthState {
                            id: format!("login_{}", request.id),
                            label: format!("Login ({})", request.method),
                            state_type: AuthStateType::Login,
                        };
                        login_state = Some(ls.clone());
                        states.push(ls.clone());

                        // Transition from initial to login
                        transitions.push(AuthTransition {
                            from_state: initial_state.id.clone(),
                            to_state: ls.id.clone(),
                            request_id: request.id,
                            method: request.method.clone(),
                            path: full_path.clone(),
                            token_type: None,
                            is_anomalous: false,
                            anomaly_reason: None,
                        });
                    }

                    // Check for response tokens (access_token, sessionId, etc.)
                    for edge in edges.iter().filter(|edge| edge.0 == request.id) {
                        let token_value = &edge.2;
                        if Self::is_auth_token(token_value) {
                            let as_id = format!("auth_{}_{}", request.id, edge.1);
                            let as_state = AuthState {
                                id: as_id.clone(),
                                label: format!(
                                    "Auth ({}...)",
                                    &token_value[..token_value.len().min(8)]
                                ),
                                state_type: AuthStateType::Authenticated,
                            };
                            authenticated_state = Some(as_state.clone());
                            states.push(as_state.clone());
                            token_to_authenticated.insert(token_value.clone(), as_state.id.clone());

                            transitions.push(AuthTransition {
                                from_state: login_state.as_ref().unwrap().id.clone(),
                                to_state: as_state.id.clone(),
                                request_id: request.id,
                                method: request.method.clone(),
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
                    let requires_auth = self.requires_auth_token(&request.path);
                    if requires_auth {
                        if authenticated_state.is_none() {
                            // Anomaly: resource accessed before login
                            let anomaly = Anomaly {
                                request_id: request.id,
                                anomaly_type: "AUTH_ANOMALY".to_string(),
                                description: format!(
                                    "Request to {} {} appears to require auth but no login was detected",
                                    request.method, request.path
                                ),
                                severity: AlertSeverity::Warning,
                            };
                            anomalies.push(anomaly);

                            // Still create the state but mark as anomalous transition
                            let rs = AuthState {
                                id: format!("resource_{}_{}", request.id, idx),
                                label: format!("Resource ({})", request.method),
                                state_type: AuthStateType::Resource,
                            };
                            states.push(rs.clone());

                            let from_state = login_state
                                .as_ref()
                                .map(|s| s.id.clone())
                                .unwrap_or_else(|| initial_state.id.clone());
                            transitions.push(AuthTransition {
                                from_state,
                                to_state: rs.id.clone(),
                                request_id: request.id,
                                method: request.method.clone(),
                                path: full_path.clone(),
                                token_type: None,
                                is_anomalous: true,
                                anomaly_reason: Some(
                                    "Resource accessed before authentication".to_string(),
                                ),
                            });
                        } else {
                            // Normal authenticated resource access
                            let rs = AuthState {
                                id: format!("resource_{}_{}", request.id, idx),
                                label: format!("Resource ({})", request.method),
                                state_type: AuthStateType::Resource,
                            };
                            states.push(rs.clone());

                            transitions.push(AuthTransition {
                                #[allow(clippy::unnecessary_unwrap)]
                                from_state: authenticated_state.as_ref().unwrap().id.clone(),
                                to_state: rs.id.clone(),
                                request_id: request.id,
                                method: request.method.clone(),
                                path: full_path.clone(),
                                token_type: None,
                                is_anomalous: false,
                                anomaly_reason: None,
                            });
                        }
                    } else {
                        // Public resource
                        let rs = AuthState {
                            id: format!("resource_{}_{}", request.id, idx),
                            label: format!("Resource ({})", request.method),
                            state_type: AuthStateType::Resource,
                        };
                        states.push(rs.clone());

                        transitions.push(AuthTransition {
                            from_state: initial_state.id.clone(),
                            to_state: rs.id.clone(),
                            request_id: request.id,
                            method: request.method.clone(),
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

        // Populate internal caches for downstream use
        for (token, state_id) in &token_to_authenticated {
            if let Some(state) = states.iter().find(|s| s.id == *state_id) {
                self.token_states.insert(token.clone(), state.clone());
            }
        }
        for request in nodes {
            self.request_paths
                .insert(request.id, (request.method.clone(), request.path.clone()));
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
            || path_lower.contains("session")
        {
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
            || path_lower.contains("payment")
        {
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

/// Build auth state machine from DAG data.
pub fn build_auth_state_machine(
    nodes: &[CapturedRequestAnalysis],
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
    db_state: &DbState,
    device_id: Option<i64>,
) -> Result<(Vec<CapturedRequestAnalysis>, Vec<DagEdge>), String> {
    let nodes = db_state.analysis_requests(&CapturedRequestQuery {
        device_id,
        order: CapturedRequestOrder::TimestampAscending,
        ..Default::default()
    })?;

    // Get edges
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
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
    let (nodes, edges) = get_dag_data_for_device(&db_state, device_id)?;

    let machine = build_auth_state_machine(&nodes, &edges, device_id);

    publish_auth_alerts(&db_state, &machine)?;

    Ok(machine)
}

fn publish_auth_alerts(db_state: &DbState, machine: &AuthStateMachine) -> Result<(), String> {
    for anomaly in &machine.anomalies {
        db_state.publish_alert(NewAlert {
            device_id: machine.device_id,
            severity: anomaly.severity,
            alert_type: AlertType::AuthAnomaly,
            details: anomaly.description.clone(),
            occurrence_key: Some(format!(
                "auth:{:?}:{}:{}",
                machine.device_id, anomaly.request_id, anomaly.anomaly_type
            )),
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Pure function tests — generate_mermaid_md
    // ------------------------------------------------------------------

    #[test]
    fn test_generate_mermaid_md_with_empty_states() {
        let md = generate_mermaid_md(&[], &[]);
        assert_eq!(md, "stateDiagram-v2\n\n");
    }

    #[test]
    fn test_generate_mermaid_md_with_initial_state() {
        let states = vec![AuthState {
            id: "initial".to_string(),
            label: "Initial".to_string(),
            state_type: AuthStateType::Initial,
        }];
        let md = generate_mermaid_md(&states, &[]);
        // Format string is `format!("    {} {}\n", "[*] --> ", "Initial")`
        // which emits two spaces between `>` and `Initial`.
        assert!(
            md.contains("[*] -->  Initial"),
            "Initial state should render as `[*] -->  Initial` (two spaces), got: {:?}",
            md
        );
    }

    #[test]
    fn test_generate_mermaid_md_with_authenticated_state() {
        let states = vec![AuthState {
            id: "auth_42".to_string(),
            label: "Auth (abc12345...)".to_string(),
            state_type: AuthStateType::Authenticated,
        }];
        let md = generate_mermaid_md(&states, &[]);
        // Authenticated has empty style, so it uses the `<id> : <label>` branch.
        assert!(
            md.contains("auth_42 : Auth (abc12345...)"),
            "Authenticated state should render as `id : label`, got: {:?}",
            md
        );
    }

    #[test]
    fn test_generate_mermaid_md_with_transition_marks_anomalous() {
        let transitions = vec![AuthTransition {
            from_state: "initial".to_string(),
            to_state: "resource_1".to_string(),
            request_id: 1,
            method: "GET".to_string(),
            path: "/profile".to_string(),
            token_type: None,
            is_anomalous: true,
            anomaly_reason: Some("Resource before auth".to_string()),
        }];
        let md = generate_mermaid_md(&[], &transitions);
        assert!(
            md.contains("ANOMALY"),
            "Anomalous transition should produce ANOMALY marker, got: {:?}",
            md
        );
    }

    #[test]
    fn test_generate_mermaid_md_with_transition_no_anomaly() {
        let transitions = vec![AuthTransition {
            from_state: "initial".to_string(),
            to_state: "resource_1".to_string(),
            request_id: 1,
            method: "GET".to_string(),
            path: "/profile".to_string(),
            token_type: None,
            is_anomalous: false,
            anomaly_reason: None,
        }];
        let md = generate_mermaid_md(&[], &transitions);
        assert!(
            !md.contains("ANOMALY"),
            "Non-anomalous transition should not produce ANOMALY marker, got: {:?}",
            md
        );
    }

    // ------------------------------------------------------------------
    // Pure function tests — build_auth_state_machine / AuthFlowExtractor
    // ------------------------------------------------------------------

    #[test]
    fn test_build_auth_state_machine_empty_nodes() {
        let machine = build_auth_state_machine(&[], &[], Some(1));
        // extract_auth_flow always seeds an "initial" state, so states == 1.
        assert_eq!(
            machine.states.len(),
            1,
            "empty input should yield just the initial state"
        );
        assert!(
            machine.transitions.is_empty(),
            "empty input should yield no transitions"
        );
        assert!(
            machine.anomalies.is_empty(),
            "empty input should yield no anomalies"
        );
        assert!(
            !machine.mermaid_md.is_empty(),
            "mermaid_md should still have the diagram header"
        );
    }

    #[test]
    fn test_build_auth_state_machine_preserves_device_id() {
        let machine = build_auth_state_machine(&[], &[], Some(42));
        assert_eq!(machine.device_id, Some(42));
    }

    #[test]
    fn test_auth_flow_extractor_new_creates_empty_extractor() {
        let mut extractor = AuthFlowExtractor::new();
        let (states, transitions, anomalies) = extractor.extract_auth_flow(&[], &[]);
        // extract_auth_flow seeds the "initial" state.
        assert_eq!(states.len(), 1);
        assert!(transitions.is_empty());
        assert!(anomalies.is_empty());
    }

    #[test]
    fn shared_analysis_fixture_supplies_auth_order_and_request_facts() {
        let fixture = crate::analysis::fixed_analysis_fixture();
        let edges = vec![(1, 2, "abc123token456def789".to_owned())];
        let machine = build_auth_state_machine(&fixture, &edges, Some(7));
        assert_eq!(machine.device_id, Some(7));
        assert!(machine
            .transitions
            .iter()
            .any(|transition| transition.request_id == 1 && transition.method == "POST"));
        assert!(machine
            .transitions
            .iter()
            .any(|transition| transition.request_id == 2
                && transition.path == "https://api.example.com/profile"));
        assert!(machine.anomalies.is_empty());
    }

    #[test]
    fn auth_alert_publication_is_idempotent_per_request() {
        let db = DbState::new_in_memory(std::sync::Mutex::new(())).unwrap();
        let machine = AuthStateMachine {
            device_id: Some(9),
            states: Vec::new(),
            transitions: Vec::new(),
            mermaid_md: String::new(),
            anomalies: vec![Anomaly {
                request_id: 42,
                anomaly_type: "AUTH_ANOMALY".to_owned(),
                description: "Resource accessed before authentication".to_owned(),
                severity: AlertSeverity::Warning,
            }],
        };

        publish_auth_alerts(&db, &machine).unwrap();
        publish_auth_alerts(&db, &machine).unwrap();
        assert_eq!(
            db.alerts(&crate::alerts::AlertQuery::default())
                .unwrap()
                .len(),
            1
        );
    }
}
