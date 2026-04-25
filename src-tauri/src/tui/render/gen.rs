//! Gen (Scaffold/Generate) tab renderer.
//!
//! Shows scaffold generation and API mocking controls.

use ratatui::{Frame, layout::Rect, widgets::{Block, Borders, Paragraph}};
use ratatui::style::Stylize;

use crate::tui::{GenMode, GenState, TuiApp};

/// Render the Gen tab.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    let mut lines = Vec::new();

    // Header
    lines.push("┌─ Generator ─────────────────────────────────────────────────┐".to_string());
    lines.push("│                                                         │".to_string());

    // Mode selector
    lines.push("│  Mode:                                                  │".to_string());

    let modes = [
        (GenMode::Mock, "Mock API"),
        (GenMode::Frontend, "Frontend Scaffold"),
        (GenMode::Docker, "Docker Bundle"),
    ];

    for (mode, label) in modes.iter() {
        let current = if app.gen.gen_mode == *mode { "[*]" } else { "[ ]" };
        let color_label = match mode {
            GenMode::Mock => label.green(),
            GenMode::Frontend => label.cyan(),
            GenMode::Docker => label.yellow(),
        };
        lines.push(format!("│    {} {} {}", current, color_label, " ".repeat(40 - label.len() - 4)));
    }

    lines.push("│                                                         │".to_string());
    lines.push("│  Actions:                                              │".to_string());
    lines.push("│    [m] Generate Mock API     - Create FastAPI mock    │".to_string());
    lines.push("│    [f] Generate Frontend     - React scaffold         │".to_string());
    lines.push("│    [d] Generate Docker      - Full deployment bundle │".to_string());
    lines.push("│    [o] Open Output Folder   - Open generated files   │".to_string());
    lines.push("│                                                         │".to_string());

    // Progress/Output section
    lines.push("│  Output:                                               │".to_string());

    if app.gen.is_generating {
        lines.push("│  ┌─────────────────────────────────────────────────┐  │".to_string());
        lines.push("│  │ Generating... please wait                       │  │".to_string());
        lines.push("│  └─────────────────────────────────────────────────┘  │".to_string());
    } else if app.gen.progress_output.is_empty() {
        lines.push("│  ┌─────────────────────────────────────────────────┐  │".to_string());
        lines.push("│  │ No generation yet. Select a mode and press     │  │".to_string());
        lines.push("│  │ the corresponding key to generate.             │  │".to_string());
        lines.push("│  └─────────────────────────────────────────────────┘  │".to_string());
    } else {
        // Show progress lines (truncated to fit)
        for line in app.gen.progress_output.iter().take(8) {
            let truncated = if line.len() > 48 { format!("{}..", &line[..46]) } else { line.clone() };
            lines.push(format!("│  │ {} │", truncated));
        }
    }

    lines.push("│                                                         │".to_string());

    // Output path
    if let Some(ref path) = app.gen.output_path {
        lines.push("│  Last output:                                          │".to_string());
        let truncated = if path.len() > 46 { format!("{}..", &path[..44]) } else { path.clone() };
        lines.push(format!("│  └─ {} ─│", truncated));
    }

    lines.push("│                                                         │".to_string());
    lines.push("│  Note: Requires inferred APIs from captured traffic.    │".to_string());
    lines.push("│        Run inference before generating.                  │".to_string());
    lines.push("└─────────────────────────────────────────────────────────┘".to_string());

    let content = lines.join("\n");
    let para = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Gen │ Mock & Scaffold Generator"));

    f.render_widget(para, area);
}
