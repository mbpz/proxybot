## Status: Draft

## 1. Architecture Overview

Tauri GUI reuses Rust core, TUI stays for server/headless use cases.

```
┌─────────────────────────────────────────────┐
│           Tauri WebView (React)              │
│   src/ - App.tsx (traffic, rules, devices)  │
└────────────────────┬────────────────────────┘
                     │ IPC (invoke)
┌────────────────────┴────────────────────────┐
│              Rust Core (src-tauri/)          │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌────────────┐ │
│  │proxy │ │ dns  │ │ cert │ │ rules engine│ │
│  └──────┘ └──────┘ └──────┘ └────────────┘ │
│  ┌──────┐ ┌────────────────────────────────┐│
│  │  db  │ │ app classifier (DNS+SNI+rule)   ││
│  └──────┘ └────────────────────────────────┘│
│                                               │
│  ┌─────────────────────────────────────────┐│
│  │ TUI (ratatui) - standalone binary        ││
│  └─────────────────────────────────────────┘│
└───────────────────────────────────────────────┘
```

## 2. TUI vs GUI Role Separation

| Feature | TUI | GUI |
|---|---|---|
| Traffic monitoring | ✅ | ✅ |
| Rule editing | ✅ | ✅ |
| Breakpoint edit | keyboard-driven | form-based |
| Device management | ✅ | ✅ |
| Certificate wizard | text UI | graphical step-by-step |
| Mock upload | CLI | drag-drop file upload |
| Screenshot export | ❌ | ✅ |
| Remote/server use | ✅ | ❌ |
| Mobile (phone) users | limited | ✅ |

**Principle**: TUI is for developers on remote servers. GUI is for non-technical users and mobile app debugging.

## 3. Component Boundaries

### IPC Commands (from Rust to React)
List the key Tauri commands that React will call:
- `get_traffic(filter)` → paginated request list
- `get_request_detail(id)` → full headers/body/ws
- `get_rules()` / `save_rule()` / `delete_rule()`
- `get_devices()` / `set_device_rule_override()`
- `get_cert_info()` / `export_cert()` / `regenerate_ca()`
- `trigger_breakpoint(decision)` - new for breakpoint
- `get_mock_targets()` / `upload_mock()`

### State Management (React side)
- TanStack Table for traffic list with virtual scrolling
- React Query for data fetching/caching
- Zustand for UI state (selected tab, filters, modal visibility)

### Event Channel (Rust to React)
- `InterceptedRequest` broadcast for real-time traffic updates
- React subscribes via `listen()` event handler
- TUI and GUI share the same event source (proxy broadcast)

## 4. UI Layout

### Main Layout
```
┌────────────────────────────────────────────────────────┐
│ [Logo] ProxyBot    [CA] [Rules] [Devices]    [?Help]  │ <- Header
├────────────────────────────────────────────────────────┤
│ Traffic | DNS | Rules | Devices | AI        [Search] │ <- Tab bar
├────────────────────────────────────────────────────────┤
│                                                        │
│                   Main Content Area                     │
│                                                        │
│  ┌──────────────────────────────────────────────────┐ │
│  │                                                    │ │
│  │              (Tab-specific content)                │ │
│  │                                                    │ │
│  └──────────────────────────────────────────────────┘ │
│                                                        │
├────────────────────────────────────────────────────────┤
│ Status: Proxy running | 1,234 requests | v0.4.2     │ <- Status bar
└────────────────────────────────────────────────────────┘
```

### Traffic Tab Layout (primary)
- Left: Filter bar (method/host/status/app) + request list (virtual scroll)
- Right: Detail panel (Headers/Body/WS sub-tabs)
- Breakpoint overlay: modal with method/url/headers/body editable fields

## 5. Dark Theme
Use shadcn/ui dark theme as base. Primary color: Cyan (#22d3ee). Accent: Emerald (#34d399).

## 6. Implementation Phases

### Phase 1: Traffic UI (v0.6.0-alpha)
1. Traffic list with virtual scroll (TanStack Table)
2. Request detail panel (headers/body/ws tabs)
3. Real-time updates via event channel

### Phase 2: Management UI
4. Rules CRUD with modal editor
5. Devices table with rule override
6. Certificate info and export

### Phase 3: Advanced
7. Breakpoint overlay with edit form
8. Mock file upload with preview
9. Screenshot annotation

## 7. Testing Strategy
- TDD with Vitest for React components
- Integration tests with mocked Rust IPC
- E2E with Playwright for critical flows

## 8. Open Questions
1. Should GUI share the same broadcast channel as TUI? (Yes, both subscribe to `InterceptedRequest`)
2. Breakpoint decisions via IPC or shared state? (IPC: `invoke('breakpoint_decision', {decision})`)
3. Rule hot-reload in GUI? (Yes, same file watcher as TUI)
