//! Structural regression tests for the desktop composition root.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

#[test]
fn frontend_literal_invocations_are_registered() {
    let frontend_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src");
    let invocation = Regex::new(
        r#"(?:safeInvokeOr|safeInvoke|invoke|desktop\s*\.\s*call)(?:\s*<[^;\n()]*>)?\s*\(\s*["']([A-Za-z0-9_]+)["']"#,
    )
    .unwrap();
    let mut used_by_command: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();

    visit_source_files(&frontend_root, &["ts", "tsx"], &mut |path, source| {
        for capture in invocation.captures_iter(source) {
            used_by_command
                .entry(capture[1].to_string())
                .or_default()
                .insert(path.strip_prefix(&frontend_root).unwrap().to_path_buf());
        }
    });
    assert!(
        used_by_command.len() >= 90,
        "IPC scanner found suspiciously few commands: {}",
        used_by_command.len()
    );

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
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let manifest = include_str!("../Cargo.toml");
    let main = include_str!("../src/main.rs");
    let main_function = Regex::new(r"(?m)^\s*fn\s+main\s*\(").unwrap();
    let mut mains = Vec::new();
    let mut builders = Vec::new();
    let mut handlers = Vec::new();
    let mut contexts = Vec::new();

    visit_source_files(&source_root, &["rs"], &mut |path, source| {
        let relative = path.strip_prefix(&source_root).unwrap().to_path_buf();
        if main_function.is_match(source) {
            mains.push(relative.clone());
        }
        builders.extend(
            source
                .match_indices("tauri::Builder::default()")
                .map(|_| relative.clone()),
        );
        handlers.extend(
            source
                .match_indices("tauri::generate_handler!")
                .map(|_| relative.clone()),
        );
        contexts.extend(
            source
                .match_indices("tauri::generate_context!")
                .map(|_| relative.clone()),
        );
    });

    assert!(main.contains("proxybot_lib::run()"));
    assert!(!main.contains("generate_handler!"));
    assert!(!manifest.contains("[[bin]]"));
    assert_eq!(mains, [PathBuf::from("main.rs")]);
    assert_eq!(builders, [PathBuf::from("bootstrap.rs")]);
    assert_eq!(handlers, [PathBuf::from("bootstrap.rs")]);
    assert_eq!(contexts, [PathBuf::from("bootstrap.rs")]);
}

#[test]
fn generated_contract_matches_rust_and_registered_commands() {
    let manifest_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/generated/desktop-contract.ts");
    let generated = fs::read_to_string(&manifest_path).unwrap();
    assert_eq!(
        generated,
        proxybot_lib::desktop_contract::render_typescript()
    );

    let registered: BTreeSet<&str> = proxybot_lib::DESKTOP_COMMANDS
        .iter()
        .map(|path| path.rsplit("::").next().unwrap().trim())
        .collect();
    let missing: Vec<_> = proxybot_lib::desktop_contract::CAPTURE_SESSION_COMMANDS
        .iter()
        .chain(proxybot_lib::desktop_contract::TRAFFIC_COMMANDS)
        .chain(proxybot_lib::desktop_contract::RULE_COMMANDS)
        .chain(proxybot_lib::desktop_contract::ALERT_COMMANDS)
        .filter(|command| !registered.contains(**command))
        .collect();
    assert!(
        missing.is_empty(),
        "generated commands absent from the Tauri handler: {missing:?}"
    );
}

#[test]
fn migrated_slices_only_use_the_desktop_adapter() {
    let frontend_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src");
    let mut bypasses = Vec::new();

    for directory in [
        "components/traffic",
        "components/ws-frames",
        "components/rules",
        "features/capture-session",
    ] {
        let root = frontend_root.join(directory);
        visit_source_files(&root, &["ts", "tsx"], &mut |path, source| {
            if source.contains("@tauri-apps/api/core")
                || source.contains("@tauri-apps/api/event")
                || source.contains("safeInvoke")
            {
                bypasses.push(path.strip_prefix(&frontend_root).unwrap().to_path_buf());
            }
        });
    }

    assert!(
        bypasses.is_empty(),
        "migrated frontend files bypass the desktop Adapter: {bypasses:?}"
    );
}

fn visit_source_files(root: &Path, extensions: &[&str], visitor: &mut impl FnMut(&Path, &str)) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            visit_source_files(&path, extensions, visitor);
            continue;
        }

        let extension = path.extension().and_then(|value| value.to_str());
        if extension.is_some_and(|extension| extensions.contains(&extension)) {
            let source = fs::read_to_string(&path).unwrap();
            visitor(&path, &source);
        }
    }
}
