//! Graph tab renderer.
//!
//! Shows traffic DAG / dependency graph visualization as ASCII art.

use std::collections::HashMap;
use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}};
use ratatui::style::Stylize;

use crate::tui::{GraphViewType, TuiApp};

/// Build the DAG lines from recent requests in the DB.
fn build_dag_lines(app: &TuiApp) -> Vec<String> {
    let mut lines = Vec::new();

    // Get recent requests from traffic state
    let requests: Vec<_> = app.traffic.requests.iter().take(30).collect();

    if requests.is_empty() {
        lines.push("No traffic captured yet. Start proxy to see DAG.".to_string());
        return lines;
    }

    // Group requests by (host, path_pattern)
    let mut groups: HashMap<String, Vec<_>> = HashMap::new();
    for req in &requests {
        let path_pattern = normalize_path_pattern(&req.path);
        let key = format!("{}:{}", req.host, path_pattern);
        groups.entry(key).or_default().push(req);
    }

    // Build node list (sorted by first seen timestamp)
    let mut nodes: Vec<(String, String)> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for req in requests.iter().rev() {
        let path_pattern = normalize_path_pattern(&req.path);
        let key = format!("{}:{}", req.host, path_pattern);
        if !seen.contains_key(&key) {
            let idx = nodes.len();
            seen.insert(key.clone(), idx);
            let display = format!("{} {}", req.method, truncate_path(&req.path, 20));
            nodes.push((key, display));
        }
    }

    if nodes.is_empty() {
        lines.push("No request patterns found.".to_string());
        return lines;
    }

    // Header
    lines.push("┌─ Traffic Dependency Graph ──────────────────────────────────┐".to_string());
    lines.push("│                                                         │".to_string());

    // Draw nodes
    for (i, (key, display)) in nodes.iter().enumerate() {
        let conn_info = if let Some(req) = groups.get(key) {
            let count = req.len();
            let status = if let Some(s) = req[0].status {
                if s >= 200 && s < 300 { "OK".to_string() }
                else if s >= 400 { format!("{}", s) }
                else { format!("{}", s) }
            } else {
                "..".to_string()
            };
            format!("[{} reqs, {}]", count, status)
        } else {
            String::new()
        };

        let prefix = if i == nodes.len() - 1 { "└──" } else { "├──" };
        let connector = if i == nodes.len() - 1 { "  " } else { "│ " };

        lines.push(format!("{} {} {} {}", prefix, display, conn_info, connector));
    }

    lines.push("│                                                         │".to_string());

    // Draw edges (temporal ordering - consecutive requests)
    if nodes.len() > 1 {
        lines.push("│  Temporal edges:                                        │".to_string());
        lines.push("│                                                         │".to_string());

        let transitions: Vec<_> = nodes.iter()
            .zip(nodes.iter().skip(1))
            .take(5)
            .map(|((_, a), (_, b))| format!("{} → {}", a, b))
            .collect();

        for t in transitions {
            lines.push(format!("│  ├── {} │", t));
        }
    }

    lines.push("│                                                         │".to_string());
    lines.push("│  Key: [g] DAG  [a] Auth  [r] refresh                   │".to_string());
    lines.push("└─────────────────────────────────────────────────────────┘".to_string());

    lines
}

/// Normalize path to a pattern (group similar paths).
fn normalize_path_pattern(path: &str) -> String {
    let segments: Vec<_> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() <= 2 {
        path.to_string()
    } else {
        segments.iter()
            .enumerate()
            .map(|(i, s)| {
                if s.chars().all(|c| c.is_ascii_digit()) {
                    ":id".to_string()
                } else if i > 0 && s.len() > 10 && s.chars().all(|c| c.is_alphanumeric()) {
                    ":param".to_string()
                } else {
                    s.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// Truncate path for display.
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("{}..", &path[..max_len.saturating_sub(2)])
    }
}

/// Build auth state machine lines from request sequence.
fn build_auth_state_machine_lines(app: &TuiApp) -> Vec<String> {
    let mut lines = Vec::new();

    let auth_indicators = ["login", "auth", "token", "oauth", "signin", "password", "session"];
    let mut auth_states: Vec<String> = Vec::new();
    let mut transitions: Vec<(String, String)> = Vec::new();

    let requests: Vec<_> = app.traffic.requests.iter().take(50).collect();

    if requests.is_empty() {
        lines.push("No traffic captured yet. Start proxy to see auth flow.".to_string());
        return lines;
    }

    let mut prev_was_auth = false;
    let mut prev_state = String::new();

    for req in requests.iter().rev() {
        let path_lower = req.path.to_lowercase();
        let host_lower = req.host.to_lowercase();
        let combined = format!("{} {}", host_lower, path_lower);

        let is_auth = auth_indicators.iter().any(|ind| combined.contains(ind));

        if is_auth {
            let state = if path_lower.contains("login") || path_lower.contains("signin") || (path_lower.contains("auth") && path_lower.contains("password")) {
                "LOGIN".to_string()
            } else if path_lower.contains("token") || path_lower.contains("oauth") || path_lower.contains("access_token") {
                "TOKEN".to_string()
            } else if path_lower.contains("logout") || path_lower.contains("signout") {
                "LOGOUT".to_string()
            } else {
                format!("AUTH({})", truncate_path(&req.path, 10))
            };

            if auth_states.is_empty() || auth_states.last() != Some(&state) {
                auth_states.push(state.clone());

                if !prev_state.is_empty() && prev_was_auth {
                    transitions.push((prev_state.clone(), state.clone()));
                }
                prev_state = state;
            }
            prev_was_auth = true;
        } else if prev_was_auth && !auth_states.is_empty() {
            let api_state = format!("API:{}", truncate_path(&req.path, 12));
            transitions.push((prev_state.clone(), api_state.clone()));
            prev_state = api_state;
            prev_was_auth = false;
        }
    }

    // Header
    lines.push("┌─ Auth State Machine ───────────────────────────────────────┐".to_string());
    lines.push("│                                                          │".to_string());

    if auth_states.is_empty() {
        lines.push("│  No explicit auth flow detected.                          │".to_string());
        lines.push("│  Auth may be embedded in headers or first-party SDK.     │".to_string());
    } else {
        lines.push("│  States:                                                 │".to_string());
        for (i, state) in auth_states.iter().enumerate() {
            let prefix = if i == auth_states.len() - 1 { "└──" } else { "├──" };
            lines.push(format!("{} {} │", prefix, state));
        }

        lines.push("│                                                          │".to_string());
        lines.push("│  Transitions:                                            │".to_string());

        if transitions.is_empty() {
            lines.push("│  └── (no transitions detected)                          │".to_string());
        } else {
            for (i, (from, to)) in transitions.iter().enumerate() {
                let prefix = if i == transitions.len() - 1 { "└──" } else { "├──" };
                lines.push(format!("{} {} → {}", prefix, from, to));
            }
        }
    }

    lines.push("│                                                          │".to_string());
    lines.push("│  Key: [g] DAG  [a] Auth  [r] refresh                      │".to_string());
    lines.push("└──────────────────────────────────────────────────────────┘".to_string());

    lines
}

/// Render the Graph tab.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    // Build DAG and auth state machine lines dynamically
    let dag_lines = build_dag_lines(app);
    let auth_lines = build_auth_state_machine_lines(app);

    // Select view based on view_type
    let (title, content) = match app.graph.view_type {
        GraphViewType::Dag => ("Graph │ DAG View", dag_lines.join("\n")),
        GraphViewType::AuthStateMachine => ("Graph │ Auth State Machine", auth_lines.join("\n")),
    };

    let placeholder = "No data available. Start proxy to capture traffic.";
    let final_content = if content.is_empty() { placeholder.to_string() } else { content };

    let para = Paragraph::new(final_content)
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(para, area);
}
