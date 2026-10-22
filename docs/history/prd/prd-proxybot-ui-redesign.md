# PRD: ProxyBot UI Redesign

## Introduction

Redesign ProxyBot's UI to match Proxyman/Charles quality — a professional, dark-themed developer tool for macOS that captures and classifies network traffic from mobile devices. The redesign establishes a coherent visual design language, fixes incomplete state handling, and organizes information architecture around the traffic dashboard as the primary use case.

## Goals

- **Professional developer aesthetic**: Dark theme with monospace data display, clear visual hierarchy
- **Traffic dashboard as the core experience**: Fast filtering, detail inspection, real-time updates
- **Complete state coverage**: Every async operation has loading skeleton, error boundary, and empty state
- **Unified design system**: Consistent colors, typography, spacing, and component library
- **Full-panel delivery**: All existing panels (Traffic, DNS, Rules, Devices, Replay, Deploy) redesigned in one pass

---

## Design Direction

**Aesthetic**: Proxyman/Charles — dark background, high-contrast text, monospace for data values, subtle borders

**Color Palette (CSS variables)**:
```
--bg-primary: #1a1a2e        /* Deep navy background */
--bg-secondary: #16213e      /* Panel backgrounds */
--bg-tertiary: #0f3460       /* Elevated cards/headers */
--text-primary: #e8e8e8      /* High contrast text */
--text-secondary: #a0a0a0    /* Labels, metadata */
--text-muted: #606060        /* Disabled, placeholder */
--border: #2a2a4a            /* Subtle borders */
--accent-blue: #4d9de0        /* Interactive elements */
--accent-green: #3ecf8e      /* Success, CONNECT tunnel */
--accent-yellow: #f4d35e     /* Warning, DNS queries */
--accent-red: #e76f51         /* Error, blocked traffic */
--accent-purple: #9b5de5     /* WebSocket frames */
--method-get: #3ecf8e         /* GET badge */
--method-post: #4d9de0       /* POST badge */
--method-put: #f4d35e        /* PUT badge */
--method-delete: #e76f51     /* DELETE badge */
```

**Typography**:
- Headings: `SF Pro Display` / system-ui, medium weight
- Body: `SF Pro Text` / system-ui, regular
- Data/code: `JetBrains Mono` / `SF Mono`, monospace

**Spacing System**: 4px base unit (4, 8, 12, 16, 24, 32, 48)

**Border Radius**: 6px for cards, 4px for inputs, 2px for badges

---

## User Stories

### US-001: Establish design system foundation
**Description:** As a developer, I need a shared CSS variable system and base component library so all panels look consistent.

**Acceptance Criteria:**
- [ ] CSS variables for all colors, typography, spacing defined in index.css
- [ ] Base component file exports: Button, Badge, Card, Input, Select, Tabs, Skeleton
- [ ] Dark theme applied to root element
- [ ] Typecheck passes

### US-002: Redesign traffic request list
**Description:** As a developer, I want a fast-scrolling request list with real-time updates, color-coded method badges, and app tag filters.

**Acceptance Criteria:**
- [ ] Request list shows: method badge (color-coded), full URL, status code, latency, size, timestamp, app tag
- [ ] Method badges: GET=green, POST=blue, PUT=yellow, DELETE=red
- [ ] Host filter dropdown (groups by host, "all" default)
- [ ] App filter: All | WeChat | Douyin | Alipay | Unknown
- [ ] Keyword search filters by host+path
- [ ] Virtual scrolling for performance (100+ items)
- [ ] New requests appear at top without scroll disruption
- [ ] Click row to expand request detail
- [ ] Typecheck passes
- [ ] Verify in browser using dev-browser skill

### US-003: Request detail panel
**Description:** As a developer, I want to inspect full request/response details — headers, query params, body with formatter.

**Acceptance Criteria:**
- [ ] Tabs: Headers | Params | Body | WS Frames
- [ ] Headers tab: two-column layout (Name | Value) for req headers and resp headers
- [ ] Params tab: key-value table with syntax highlighting
- [ ] Body tab: JSON pretty-print with syntax coloring, or raw text fallback
- [ ] WS Frames tab (if WebSocket): bidirectional frame list with direction indicator and timestamp
- [ ] Copy button for headers/body
- [ ] Typecheck passes
- [ ] Verify in browser using dev-browser skill

### US-004: Add loading/error/empty states to traffic panel
**Description:** As a developer, I want to know when data is loading, when an error occurs, and when there's no traffic — not blank panels.

**Acceptance Criteria:**
- [ ] Loading: skeleton rows (3-5 animated placeholder rows) while fetching initial data
- [ ] Error: red-tinted banner with error message and retry button
- [ ] Empty: centered icon + "No requests captured yet" + helper text
- [ ] Error boundary wraps each panel independently
- [ ] Typecheck passes
- [ ] Verify in browser using dev-browser skill

### US-005: Redesign DNS log panel
**Description:** As a developer, I want to see DNS queries with app correlation, upstream info, and resolution status.

**Acceptance Criteria:**
- [ ] Table columns: Timestamp | Domain | Type | Response IPs | App tag
- [ ] Color-coded by app tag
- [ ] Upstream indicator (DoH/DoT/UDP) in header
- [ ] Loading skeleton, error boundary, empty state
- [ ] Typecheck passes
- [ ] Verify in browser using dev-browser skill

### US-006: Redesign rules editor panel
**Description:** As a developer, I want a visual rule editor that shows rule actions and patterns clearly.

**Acceptance Criteria:**
- [ ] Rule list: pattern badge + value + action badge (DIRECT=green, PROXY=blue, REJECT=red)
- [ ] Add/Edit rule modal with pattern selector, value input, action dropdown
- [ ] Delete rule with confirmation
- [ ] Reorder rules via drag handle
- [ ] YAML source toggle (view raw YAML)
- [ ] Loading skeleton, error boundary, empty state
- [ ] Typecheck passes
- [ ] Verify in browser using dev-browser skill

### US-007: Redesign device management panel
**Description:** As a developer, I want to see connected devices with traffic stats and per-device rule overrides.

**Acceptance Criteria:**
- [ ] Device cards: name, MAC/IP, last seen, upload/download bytes
- [ ] Simple topology diagram (dots connected by lines, not full graph library)
- [ ] Rule override dropdown per device
- [ ] Edit device name inline
- [ ] Loading skeleton, error boundary, empty state
- [ ] Typecheck passes
- [ ] Verify in browser using dev-browser skill

### US-008: Redesign replay panel
**Description:** As a developer, I want to replay recorded traffic against a mock server with diff view.

**Acceptance Criteria:**
- [ ] Host selector dropdown + delay input + replay button
- [ ] Results table: Method | URL | Status | Diff indicator
- [ ] Diff detail: side-by-side headers and body with highlighted changes
- [ ] Progress indicator during replay
- [ ] Loading skeleton, error boundary, empty state
- [ ] Typecheck passes
- [ ] Verify in browser using dev-browser skill

### US-009: Redesign deploy panel
**Description:** As a developer, I want to generate and preview a Docker deployment bundle.

**Acceptance Criteria:**
- [ ] Project name input + Generate button
- [ ] Preview area for generated docker-compose.yml
- [ ] Write to disk + Git init buttons
- [ ] Success/error status messages
- [ ] Loading state during generation
- [ ] Typecheck passes
- [ ] Verify in browser using dev-browser skill

### US-010: Unified app header and navigation
**Description:** As a developer, I want a consistent header across all panels with proxy status, CA cert actions, and navigation.

**Acceptance Criteria:**
- [ ] Header: ProxyBot logo/name left, status indicator center, CA cert actions right
- [ ] Status: green dot = running, gray = stopped, with label
- [ ] Tab bar below header: Traffic | DNS | Rules | Devices | Replay | Deploy | AI
- [ ] Active tab indicator (bottom border accent color)
- [ ] Alert badge on AI tab when alerts > 0
- [ ] Typecheck passes
- [ ] Verify in browser using dev-browser skill

### US-011: AI panel — auth state machine and alerts
**Description:** As a developer, I want to see inferred auth flows as Mermaid diagrams and anomaly alerts.

**Acceptance Criteria:**
- [ ] Two tabs: Auth Flow | Alerts
- [ ] Auth Flow tab: rendered Mermaid diagram from state machine data
- [ ] Alerts tab: severity-filtered list with acknowledge button
- [ ] Loading skeleton, error boundary, empty state
- [ ] Typecheck passes
- [ ] Verify in browser using dev-browser skill

---

## Functional Requirements

### FR-1: Global
- FR-1.1: All CSS custom properties defined in :root
- FR-1.2: No inline styles; all styling via Tailwind classes or CSS variables
- FR-1.3: Error boundaries wrap every async panel independently
- FR-1.4: Responsive to window resize (panels collapse gracefully)

### FR-2: Traffic Dashboard
- FR-2.1: Real-time WebSocket event listener adds requests to top of list
- FR-2.2: List capped at 500 items (older items dropped)
- FR-2.3: Filter state (host, app, keyword) persisted in component state (not URL)
- FR-2.4: Selected request highlighted with accent border

### FR-3: Request Detail
- FR-3.1: JSON body auto-detected and pretty-printed
- FR-3.2: Binary body shows size + mime type (no attempted decoding)
- FR-3.3: WebSocket frames sorted by timestamp ascending

### FR-4: Design System
- FR-4.1: No new color values outside CSS variables
- FR-4.2: All shadows use CSS variable `--shadow-sm`, `--shadow-md`
- FR-4.3: All spacing uses 4px-based tokens

---

## Non-Goals

- No mobile/responsive layout for phone screens (macOS desktop only)
- No drag-and-drop layout customization
- No dark/light theme toggle (dark only)
- No multi-window support
- No internationalization (English only)

---

## Technical Considerations

### Stack
- **Framework**: Tauri v2 + React 19 + TypeScript
- **Styling**: Tailwind CSS + shadcn/ui components
- **Icons**: Lucide React (already in use)
- **Fonts**: JetBrains Mono via @fontsource or system fallback

### Key Implementation Notes
- Reuse existing shadcn/ui components where possible (Button, Card, Tabs, Input, Select, Dialog)
- Create `components/ui` wrapper for design tokens
- Use `react-virtual` or `@tanstack/react-virtual` for request list virtualization
- Mermaid rendering via `mermaid` npm package for auth flow diagrams
- Error boundary: simple class component with fallback UI

### File Structure
```
src/
  components/
    ui/                    # Base design system components
      button.tsx
      badge.tsx
      card.tsx
      input.tsx
      select.tsx
      tabs.tsx
      skeleton.tsx
      error-boundary.tsx
    traffic/
      request-list.tsx
      request-detail.tsx
      filters.tsx
    dns/
      dns-log.tsx
    rules/
      rule-list.tsx
      rule-editor.tsx
    devices/
      device-card.tsx
      topology.tsx
    replay/
      replay-panel.tsx
      diff-view.tsx
    deploy/
      deploy-panel.tsx
    ai/
      auth-flow.tsx
      alerts.tsx
    layout/
      header.tsx
      tab-bar.tsx
  hooks/
    use-traffic.ts
    use-dns.ts
    use-rules.ts
    use-devices.ts
    use-replay.ts
  lib/
    design-system.ts      # CSS variable exports
  App.tsx
  index.css
```

---

## Success Metrics

- All 11 user stories complete and visually verified
- TypeScript `tsc --noEmit` passes
- `cargo clippy` errors reduced to 0 (or documented allow-list)
- All panels have loading skeleton + error boundary + empty state
- No regression in proxy functionality

---

## Open Questions

1. Should request list support export to HAR/JSON?
2. Should device topology use a graph library (d3, react-force-graph) or stay as simple SVG dots?
3. Should we add keyboard shortcuts for filtering and navigation?
4. Vision screenshot upload — keep in AI tab or move to separate panel?
