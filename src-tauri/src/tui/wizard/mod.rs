//! Certificate installation wizard for mobile platforms.
//!
//! Guides users through CA certificate installation on iOS and Android.

use ratatui::{Frame, widgets::{Block, Borders, Paragraph, Wrap}, style::Color};
use ratatui::text::{Line, Span};
use ratatui::layout::{Constraint, Direction, Layout};

/// Platform for certificate installation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Platform {
    IOS,
    Android,
    Unknown,
}

impl Platform {
    /// Detect platform from user agent string.
    pub fn detect_from_ua(ua: &str) -> Self {
        let ua = ua.to_lowercase();
        if ua.contains("iphone") || ua.contains("ipad") {
            Platform::IOS
        } else if ua.contains("android") {
            Platform::Android
        } else {
            Platform::Unknown
        }
    }
}

/// WizardStep represents a single step in the certificate installation wizard.
#[derive(Debug, Clone)]
pub struct WizardStep {
    /// Step title.
    pub title: String,
    /// Instruction text for the user.
    pub instruction: String,
    /// Optional deep link to open system settings (None for copy steps).
    pub deep_link: Option<String>,
}

/// The certificate installation wizard state.
pub struct CertWizard {
    /// Target platform (iOS or Android).
    pub platform: Platform,
    /// Current step index (0-based).
    pub current_step: usize,
    /// All wizard steps for the platform.
    pub steps: Vec<WizardStep>,
    /// Whether the wizard has been completed.
    pub completed: bool,
}

impl CertWizard {
    /// Create a new wizard for the specified platform.
    pub fn new(platform: Platform) -> Self {
        let steps = match platform {
            Platform::IOS => vec![
                WizardStep {
                    title: "Step 1: Transfer Certificate".to_string(),
                    instruction: "Use AirDrop to send ca.crt to your iPhone/iPad.\n\
                                 Or download the certificate file to your device.".to_string(),
                    deep_link: None,
                },
                WizardStep {
                    title: "Step 2: Install Profile".to_string(),
                    instruction: "Open Settings > Profile Downloaded.\n\
                                 Tap \"Install\" and enter your passcode.".to_string(),
                    deep_link: None,
                },
                WizardStep {
                    title: "Step 3: Approve Installation".to_string(),
                    instruction: "When prompted, tap \"Install\" again to confirm.\n\
                                 Enter your passcode to authorize.".to_string(),
                    deep_link: None,
                },
                WizardStep {
                    title: "Step 4: Enable Trust".to_string(),
                    instruction: "Go to Settings > General > About >\n\
                                 Certificate Trust Settings.\n\
                                 Enable full trust for \"ProxyBot CA\".".to_string(),
                    deep_link: Some("App-prefs:root=General&path=About/CERTIFICATE_TRUST_SETTINGS".to_string()),
                },
                WizardStep {
                    title: "Step 5: Verify".to_string(),
                    instruction: "Visit https://mitm.it in Safari.\n\
                                 You should see a green \"Congratulations\" page.\n\
                                 If not, try restarting Safari.".to_string(),
                    deep_link: Some("https://mitm.it".to_string()),
                },
            ],
            Platform::Android => vec![
                WizardStep {
                    title: "Step 1: Transfer Certificate".to_string(),
                    instruction: "Connect your device via USB and copy ca.crt\n\
                                 to your Downloads folder.\n\
                                 Or email the file to yourself.".to_string(),
                    deep_link: None,
                },
                WizardStep {
                    title: "Step 2: Open Certificate".to_string(),
                    instruction: "Open the ca.crt file in Files app.\n\
                                 You may need to locate it in Downloads.".to_string(),
                    deep_link: None,
                },
                WizardStep {
                    title: "Step 3: Install Certificate".to_string(),
                    instruction: "When prompted, name it \"ProxyBot CA\"\n\
                                 and select \"VPN and apps\" for credentials use.\n\
                                 Tap OK to confirm.".to_string(),
                    deep_link: None,
                },
                WizardStep {
                    title: "Step 4: Verify".to_string(),
                    instruction: "Open Chrome and visit https://mitm.it\n\
                                 You should see a green \"Congratulations\" page.".to_string(),
                    deep_link: Some("https://mitm.it".to_string()),
                },
                WizardStep {
                    title: "Important Note".to_string(),
                    instruction: "On Android 7+, user-added certificates don't\n\
                                 work for all apps (apps that ignore\n\
                                 system certs). Consider using ProxyBot's\n\
                                 VPN mode for full traffic capture.".to_string(),
                    deep_link: None,
                },
            ],
            Platform::Unknown => vec![],
        };

        CertWizard {
            platform,
            current_step: 0,
            steps,
            completed: false,
        }
    }

    /// Advance to the next step.
    pub fn next(&mut self) {
        if self.current_step < self.steps.len().saturating_sub(1) {
            self.current_step += 1;
        } else {
            self.completed = true;
        }
    }

    /// Go back to the previous step.
    pub fn prev(&mut self) {
        if self.current_step > 0 {
            self.current_step -= 1;
        }
    }

    /// Check if on the last step.
    pub fn is_last(&self) -> bool {
        self.current_step >= self.steps.len().saturating_sub(1)
    }
}

/// Render the wizard overlay on top of the current view.
pub fn render_wizard(f: &mut Frame, area: ratatui::layout::Rect, wizard: &CertWizard) {
    let platform_name = match wizard.platform {
        Platform::IOS => "iOS",
        Platform::Android => "Android",
        Platform::Unknown => "Unknown",
    };

    // Create wizard popup
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Certificate Install Wizard - {}", platform_name))
        .title_style(Color::Cyan);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Layout: title area, instruction area, progress area, key hints
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Platform indicator
            Constraint::Min(3),     // Step title
            Constraint::Min(4),     // Instructions
            Constraint::Length(1),  // Progress (e.g., "Step 2/5")
            Constraint::Length(1),  // Key hints
        ])
        .split(inner_area);

    // Platform indicator
    let platform_indicator = Paragraph::new(format!("Platform: {}", platform_name))
        .style(Color::Yellow);
    f.render_widget(platform_indicator, chunks[0]);

    // Current step info
    if wizard.current_step < wizard.steps.len() {
        let step = &wizard.steps[wizard.current_step];

        let step_title = Paragraph::new(step.title.clone())
            .style(Color::Green);
        f.render_widget(step_title, chunks[1]);

        let instruction_lines: Vec<Line> = step.instruction
            .lines()
            .map(|l| Line::from(Span::raw(l)))
            .collect();
        let instruction = Paragraph::new(instruction_lines)
            .wrap(Wrap { trim: true });
        f.render_widget(instruction, chunks[2]);

        // Deep link if available
        if let Some(ref link) = step.deep_link {
            let link_text = format!("Link: {}", link);
            let link_para = Paragraph::new(link_text)
                .style(Color::Blue);
            f.render_widget(link_para, chunks[2]);
        }
    }

    // Progress indicator
    let progress = format!(
        "Step {}/{}",
        wizard.current_step + 1,
        wizard.steps.len()
    );
    let progress_para = Paragraph::new(progress)
        .style(Color::White);
    f.render_widget(progress_para, chunks[3]);

    // Key hints
    let hints = if wizard.completed {
        "[w] Close wizard"
    } else if wizard.current_step == 0 {
        "[w] Close | [Space/Enter] Next"
    } else {
        "[w] Close | [Space/Enter] Next | [Backspace] Previous"
    };
    let hints_para = Paragraph::new(hints)
        .style(Color::Gray);
    f.render_widget(hints_para, chunks[4]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection_ios() {
        assert_eq!(Platform::detect_from_ua("Mozilla/5.0 (iPhone; CPU iPhone OS 15_0)"), Platform::IOS);
        assert_eq!(Platform::detect_from_ua("Mozilla/5.0 (iPad; CPU OS 14_0)"), Platform::IOS);
    }

    #[test]
    fn test_platform_detection_android() {
        assert_eq!(Platform::detect_from_ua("Mozilla/5.0 (Linux; Android 11)"), Platform::Android);
    }

    #[test]
    fn test_platform_detection_unknown() {
        assert_eq!(Platform::detect_from_ua("Mozilla/5.0 (Windows NT 10.0)"), Platform::Unknown);
    }

    #[test]
    fn test_wizard_ios_steps() {
        let wizard = CertWizard::new(Platform::IOS);
        assert_eq!(wizard.steps.len(), 5);
        assert_eq!(wizard.platform, Platform::IOS);
        assert!(!wizard.completed);
    }

    #[test]
    fn test_wizard_android_steps() {
        let wizard = CertWizard::new(Platform::Android);
        assert_eq!(wizard.steps.len(), 5);
        assert_eq!(wizard.platform, Platform::Android);
    }

    #[test]
    fn test_wizard_navigation() {
        let mut wizard = CertWizard::new(Platform::IOS);
        assert_eq!(wizard.current_step, 0);

        wizard.next();
        assert_eq!(wizard.current_step, 1);

        wizard.prev();
        assert_eq!(wizard.current_step, 0);
    }

    #[test]
    fn test_wizard_completion() {
        let mut wizard = CertWizard::new(Platform::Android);
        for _ in 0..5 {
            wizard.next();
        }
        assert!(wizard.completed);
    }

    #[test]
    fn test_wizard_is_last() {
        let mut wizard = CertWizard::new(Platform::Android);
        assert!(!wizard.is_last());

        for _ in 0..4 {
            wizard.next();
        }
        assert!(wizard.is_last());
    }
}