# Step 4 Review Request — App Rules Classification

**Ready for Review: YES**

## Summary

Built app classification rule library for traffic filtering. Added `app_rules.rs` module with WeChat (💬), Douyin (🎵), and Alipay (💳) classification rules. Modified proxy to attach app info to each intercepted request event. Added tab filtering and app columns to the requests table in the UI.

## New File

**src-tauri/src/app_rules.rs** (lines 1-64)
- `AppRule` struct with `name: &'static str`, `icon: &'static str`, `domains: &'static [&'static str]`
- `APP_RULES` static slice with three app rules
- `classify_host(host: &str) -> Option<(&str, &str)>` — exact match or ends_with match
- Unit tests for exact match, subdomain match, and unknown hosts

## Modified Files

**src-tauri/src/lib.rs** (lines 3, 7)
- Added `mod app_rules` declaration

**src-tauri/src/proxy.rs** (lines 1-5, 31-42, 518-532, 611-626)
- Added `use crate::app_rules` import
- Extended `InterceptedRequest` with `app_name: Option<String>` and `app_icon: Option<String>`
- In HTTPS CONNECT handler and HTTP handler: called `classify_host()` and attached result to event payload

**src/App.tsx** (lines 6-20, 37-38, 227-276)
- Added `app_name?: string` and `app_icon?: string` to `InterceptedRequest` interface
- Added `AppTab` type and `selectedTab` state for tab filtering
- Added tab filter buttons: All / WeChat 💬 / Douyin 🎵 / Alipay 💳 / Unknown
- Added App column to requests table showing emoji + app name
- Client-side filtering: requests filtered by selected tab before display

## Build Verification

- `cargo check` in src-tauri/: **0 errors**

## Open Questions

None.
