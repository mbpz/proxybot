# Step 7 Review Request — Production Build (macOS .app Packaging)

**Ready for Review: YES**

## Summary

Configured ProxyBot for macOS distribution. Created entitlements plist, updated tauri.conf.json for production build, added build:tauri npm script, replaced default README with user-facing documentation.

## Files Changed

### src-tauri/entitlements.plist (NEW)

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.network.server</key>
    <true/>
    <key>com.apple.security.files.user-selected.read-write</key>
    <true/>
</dict>
</plist>
```

Required for pf transparent proxy and file access.

### src-tauri/tauri.conf.json (UPDATED)

- `productName`: "proxybot" → "ProxyBot"
- `app.windows[0]`: title → "ProxyBot", width → 1100, height → 750, minWidth → 900, minHeight → 600, center → true, resizable → true
- `bundle.targets`: "all" → `["dmg", "app"]`
- `bundle.category`: "Developer Tools"
- `bundle.shortDescription` + `longDescription` added
- `bundle.icon`: removed `icon.ico` (Windows, not needed for macOS)

Note: `devtools: true` was removed from `build` — field is not supported in the current Tauri v2 version. Devtools are enabled automatically in `tauri dev` mode.

### package.json (UPDATED)

Added `"build:tauri": "tauri build"` script.

### README.md (REPLACED)

Replaced default Tauri template README with:
- Project description and feature list
- Installation steps (download .dmg, mount, drag to Applications)
- iOS CA certificate install guide (step-by-step text instructions)
- Phone setup steps (gateway/DNS = PC IP)
- FAQ covering WeChat certificate pinning, no-traffic troubleshooting, classification pipeline

## Icons

All icons already exist at `src-tauri/icons/`. No icon generation needed.

## Build Verification

- `cargo check` in `src-tauri/`: **0 errors, 0 warnings**

## Notes for User

- Do NOT run `npm run tauri build` in this environment — it requires codesigning and will fail without a Developer ID.
- The user should run `npm run build:tauri` themselves on a machine with a valid Developer ID for codesigning.
- If codesigning is not available, the user can still build and run locally with `xattr -cr /Applications/ProxyBot.app` to bypass gatekeeper.
