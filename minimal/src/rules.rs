//! App classification rules loaded from YAML.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const DEFAULT_RULES_YAML: &str = include_str!("../rules/apps.yaml");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRule {
    pub app: String,
    #[serde(default)]
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuleFile {
    #[serde(default)]
    apps: Vec<AppRule>,
}

#[derive(Debug, Clone, Default)]
pub struct Rules {
    apps: Vec<AppRule>,
}

impl Rules {
    pub fn parse(yaml: &str) -> Result<Self> {
        let file: RuleFile = serde_yaml::from_str(yaml).context("parse rules YAML")?;
        Ok(Self { apps: file.apps })
    }

    /// Load user rules from `path`; fall back to the embedded defaults if the
    /// file is missing or invalid.
    pub fn load_or_default(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(text) => match Self::parse(&text) {
                Ok(rules) => rules,
                Err(error) => {
                    tracing::warn!(%error, path = %path.display(), "invalid rules file; using built-in rules");
                    Self::parse(DEFAULT_RULES_YAML).unwrap_or_default()
                }
            },
            Err(_) => Self::parse(DEFAULT_RULES_YAML).unwrap_or_default(),
        }
    }

    pub fn ensure_file(path: &Path) -> Result<()> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, DEFAULT_RULES_YAML)?;
        }
        Ok(())
    }

    pub fn to_yaml(&self) -> String {
        serde_yaml::to_string(&RuleFile {
            apps: self.apps.clone(),
        })
        .unwrap_or_else(|_| DEFAULT_RULES_YAML.to_string())
    }

    /// Longest domain suffix wins. Returns `(app, source)`.
    pub fn classify(&self, host: &str) -> Option<(String, &'static str)> {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        let mut best: Option<(usize, &str)> = None;
        for rule in &self.apps {
            for domain in &rule.domains {
                let domain = domain.trim_start_matches("*.").to_ascii_lowercase();
                let matches = host == domain || host.ends_with(&format!(".{domain}"));
                if matches {
                    let score = domain.len();
                    if best.map(|(len, _)| score > len).unwrap_or(true) {
                        best = Some((score, rule.app.as_str()));
                    }
                }
            }
        }
        best.map(|(_, app)| (app.to_string(), "rule"))
    }
}
