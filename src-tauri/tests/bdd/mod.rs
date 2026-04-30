//! BDD/Cucumber integration tests for proxybot-tui.
//!
//! Run with: cargo test --test test_bdd

mod features;

// Re-export the steps module so cargo test can find it
// The actual test runner is in features/steps.rs as a binary entry point