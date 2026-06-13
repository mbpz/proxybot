//! User custom bypass scripts loader.
//!
//! Reads `.js` files from `~/.proxybot/bypass-scripts/` and returns
//! them as `BypassScript` entries with `is_builtin: false`.
//!
//! This is a stub. Full implementation lands in Task 5.

use crate::ssl_bypass::bypass_scripts::BypassScript;

/// Stub: returns no scripts. Real implementation in Task 5.
pub fn load_custom_scripts() -> Vec<BypassScript> {
    Vec::new()
}
