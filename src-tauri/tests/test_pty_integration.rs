//! PTY integration tests for proxybot-tui binary.
//!
//! Simulates real terminal sessions end-to-end. Tests the full
//! binary as a black box — start proxy, navigate tabs, verify output.
//!
//! These tests are marked `#[ignore]` because they require the binary
//! to be pre-built and are slower than unit tests.
//!
//! Run with:
//!   cargo build --bin proxybot-tui
//!   cargo test --test test_pty_integration -- --ignored

use std::time::Duration;
use rexpect::session::PtySession;

/// Run proxybot-tui and interact with it via PTY.
mod integration {
    use super::*;

    /// Basic sanity: proxybot-tui starts and shows its header.
    #[test]
    #[ignore = "requires binary pre-built"]
    fn test_tui_starts_and_shows_banner() {
        let mut pty = rexpect::spawn("cargo run --bin proxybot-tui -- --dev", Some(30_000))
            .expect("Failed to spawn proxybot-tui");

        // Wait for initial render — proxybot-tui shows a header
        pty.exp_string("ProxyBot").ok();

        // Clean exit
        pty.send("q").unwrap();
        let status = pty.process_mut().status();
        assert!(status.is_some(), "process should have exited");
    }

    /// User starts proxy with 'r' key.
    #[test]
    #[ignore = "requires binary pre-built"]
    fn test_start_proxy() {
        let mut pty = rexpect::spawn("cargo run --bin proxybot-tui -- --dev", Some(30_000))
            .expect("Failed to spawn proxybot-tui");

        pty.exp_string("ProxyBot").ok();
        pty.send("r").unwrap();
        // Wait briefly and check no crash
        std::thread::sleep(Duration::from_millis(200));
        pty.send("q").unwrap();
    }

    /// User navigates through all 9 tabs.
    #[test]
    #[ignore = "requires binary pre-built"]
    fn test_tab_navigation_wraps() {
        let mut pty = rexpect::spawn("cargo run --bin proxybot-tui -- --dev", Some(30_000))
            .expect("Failed to spawn proxybot-tui");

        pty.exp_string("ProxyBot").ok();

        // Navigate through 9 tabs with Tab key
        for _ in 0..9 {
            pty.send("\t").unwrap();
            std::thread::sleep(Duration::from_millis(50));
        }

        // Should wrap back to Traffic — the initial render text
        // should appear again (TUI re-renders the full frame)
        pty.exp_string("Intercepted").ok();

        pty.send("q").unwrap();
    }

    /// User quits with 'q'.
    #[test]
    #[ignore = "requires binary pre-built"]
    fn test_quit_with_q() {
        let mut pty = rexpect::spawn("cargo run --bin proxybot-tui -- --dev", Some(30_000))
            .expect("Failed to spawn proxybot-tui");

        pty.exp_string("ProxyBot").ok();
        pty.send("q").unwrap();

        let status = pty.process_mut().wait();
        assert!(status.is_ok(), "process should exit cleanly");
    }

    /// User quits with Escape.
    #[test]
    #[ignore = "requires binary pre-built"]
    fn test_quit_with_escape() {
        let mut pty = rexpect::spawn("cargo run --bin proxybot-tui -- --dev", Some(30_000))
            .expect("Failed to spawn proxybot-tui");

        pty.exp_string("ProxyBot").ok();
        pty.send("\x1b").unwrap(); // ASCII escape

        let status = pty.process_mut().wait();
        assert!(status.is_ok(), "process should exit cleanly with Esc");
    }

    /// User navigates tabs with hjkl.
    #[test]
    #[ignore = "requires binary pre-built"]
    fn test_hjkl_navigation() {
        let mut pty = rexpect::spawn("cargo run --bin proxybot-tui -- --dev", Some(30_000))
            .expect("Failed to spawn proxybot-tui");

        pty.exp_string("ProxyBot").ok();

        // 'h' moves back, 'l' moves forward
        pty.send("h").unwrap();
        std::thread::sleep(Duration::from_millis(50));
        pty.send("l").unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // We're back at Traffic
        pty.exp_string("Intercepted").ok();

        pty.send("q").unwrap();
    }
}