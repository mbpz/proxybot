//! Gen (Scaffold/Generate) tab renderer.
//!
//! Shows scaffold generation and API mocking controls.

use ratatui::style::Stylize;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::i18n::{t as tr, I18nKey as K};
use crate::tui::{GenMode, TuiApp};

/// Render the Gen tab.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    let mut lines = Vec::new();

    // Header
    let generator_label = tr(K::GenGenerator);
    let mode_label = tr(K::GenMode);
    lines.push(format!(
        "┌─ {} ─────────────────────────────────────────────────┐",
        generator_label
    ));
    lines.push("│                                                         │".to_string());

    // Mode selector
    lines.push(format!(
        "│  {}                                                  │",
        mode_label
    ));

    let modes = [
        (GenMode::Mock, tr(K::GenMockApi)),
        (GenMode::Frontend, tr(K::GenFrontendScaffold)),
        (GenMode::Docker, tr(K::GenDockerBundle)),
    ];

    for (mode, label) in modes.iter() {
        let current = if app.gen.gen_mode == *mode {
            "[*]"
        } else {
            "[ ]"
        };
        let color_label = match mode {
            GenMode::Mock => label.clone().green(),
            GenMode::Frontend => label.clone().cyan(),
            GenMode::Docker => label.clone().yellow(),
        };
        lines.push(format!(
            "│    {} {} {}",
            current,
            color_label,
            " ".repeat(40 - label.len() - 4)
        ));
    }

    lines.push("│                                                         │".to_string());
    lines.push(format!(
        "│  {}                                              │",
        tr(K::GenActions)
    ));
    lines.push(format!("│    {}", tr(K::GenGenerateMockApi)));
    lines.push(format!("│    {}", tr(K::GenGenerateFrontend)));
    lines.push(format!("│    {}", tr(K::GenGenerateDocker)));
    lines.push(format!("│    {}", tr(K::GenOpenOutputFolder)));
    lines.push("│                                                         │".to_string());

    // Progress/Output section
    lines.push(format!(
        "│  {}                                               │",
        tr(K::GenOutput)
    ));

    if app.gen.is_generating {
        lines.push("│  ┌─────────────────────────────────────────────────┐  │".to_string());
        lines.push(format!(
            "│  │ {}                       │  │",
            tr(K::GenGenerating)
        ));
        lines.push("│  └─────────────────────────────────────────────────┘  │".to_string());
    } else if app.gen.progress_output.is_empty() {
        lines.push("│  ┌─────────────────────────────────────────────────┐  │".to_string());
        lines.push(format!("│  │ {}     │  │", tr(K::GenNoGenerationYet)));
        lines.push(format!(
            "│  │ {}             │  │",
            tr(K::GenSelectModeAndPress)
        ));
        lines.push("│  └─────────────────────────────────────────────────┘  │".to_string());
    } else {
        // Show progress lines (truncated to fit)
        for line in app.gen.progress_output.iter().take(8) {
            let truncated = if line.len() > 48 {
                format!("{}..", &line[..46])
            } else {
                line.clone()
            };
            lines.push(format!("│  │ {} │", truncated));
        }
    }

    lines.push("│                                                         │".to_string());

    // Output path
    if let Some(ref path) = app.gen.output_path {
        lines.push(format!(
            "│  {}                                          │",
            tr(K::GenOutputLast)
        ));
        let truncated = if path.len() > 46 {
            format!("{}..", &path[..44])
        } else {
            path.clone()
        };
        lines.push(format!("│  └─ {} ─│", truncated));
    }

    lines.push("│                                                         │".to_string());
    lines.push(format!("│  {}    │", tr(K::GenNoteRequiresInferred)));
    lines.push(format!(
        "│        {}                  │",
        tr(K::GenRunInference)
    ));
    lines.push("└─────────────────────────────────────────────────────────┘".to_string());

    let content = lines.join("\n");
    let gen_title = tr(K::GenTitle);
    let para = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("{} │ {}", gen_title, tr(K::GenMode))),
    );

    f.render_widget(para, area);
}
