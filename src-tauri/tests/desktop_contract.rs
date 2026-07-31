//! Structural regression tests for the desktop composition root.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

#[test]
fn frontend_literal_invocations_are_registered() {
    let frontend_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src");
    let invocation =
        Regex::new(r#"(?:safeInvoke|invoke)(?:\s*<[^;\n()]*>)?\s*\(\s*["']([A-Za-z0-9_]+)["']"#)
            .unwrap();
    let mut used_by_command: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();

    visit_source_files(&frontend_root, &mut |path, source| {
        for capture in invocation.captures_iter(source) {
            used_by_command
                .entry(capture[1].to_string())
                .or_default()
                .insert(path.strip_prefix(&frontend_root).unwrap().to_path_buf());
        }
    });

    let registered: BTreeSet<&str> = proxybot_lib::DESKTOP_COMMANDS
        .iter()
        .map(|path| path.rsplit("::").next().unwrap().trim())
        .collect();
    let missing: Vec<_> = used_by_command
        .iter()
        .filter(|(command, _)| !registered.contains(command.as_str()))
        .collect();

    assert!(
        missing.is_empty(),
        "frontend invokes commands absent from the desktop bootstrap: {missing:#?}"
    );
}

#[test]
fn executable_is_a_thin_bootstrap_adapter() {
    let manifest = include_str!("../Cargo.toml");
    let main = include_str!("../src/main.rs");

    assert!(main.contains("proxybot_lib::run()"));
    assert!(!main.contains("generate_handler!"));
    assert!(!manifest.contains("src/bin/proxybot-gui.rs"));
}

fn visit_source_files(root: &Path, visitor: &mut impl FnMut(&Path, &str)) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            visit_source_files(&path, visitor);
            continue;
        }

        let extension = path.extension().and_then(|value| value.to_str());
        if matches!(extension, Some("ts" | "tsx")) {
            let source = fs::read_to_string(&path).unwrap();
            visitor(&path, &source);
        }
    }
}
