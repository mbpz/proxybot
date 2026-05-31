# ProxyBot PC Client Redesign Design

**Date:** 2026-05-31
**Author:** Claude

---

## 1. Concept & Vision

A developer-focused HTTPS proxy tool with a cyberpunk tech aesthetic. The interface should feel like a professional debugging suite — information-dense but visually organized, with neon accents that make status and data instantly scannable. Think VS Code meets Wireshark in a neon-lit server room.

---

## 2. Design Language

### Aesthetic Direction
**Cyberpunk Terminal** — Dark backgrounds with glowing neon accents, subtle grid patterns, and high-contrast data visualization.

### Color Palette
| Role | Color | Hex |
|------|-------|-----|
| Background | Near Black | `#0a0a0f` |
| Surface | Dark Purple-Black | `#12121a` |
| Surface Elevated | Deep Purple | `#1a1a2e` |
| Border | Subtle Purple | `#1e1e2e` |
| Primary Accent | Neon Cyan | `#00d4ff` |
| Secondary Accent | Electric Purple | `#a855f7` |
| Success | Neon Green | `#22c55e` |
| Danger | Alert Red | `#ff4d4d` |
| Text Primary | White | `#ffffff` |
| Text Secondary | Muted Purple | `#8888aa` |
| Text Tertiary | Dark Purple | `#666688` |

### Typography
- **Primary Font:** Inter (clean, technical)
- **Monospace:** JetBrains Mono (for code/data)
- **Fallbacks:** -apple-system, BlinkMacSystemFont, sans-serif

### Visual Effects
- **Glow Effects:** `box-shadow` with accent colors (e.g., `0 0 8px #00d4ff`) for status indicators
- **Gradients:** Subtle `linear-gradient(135deg, #1a1a2e 0%, #12121a 100%)` on cards
- **Borders:** 1px solid `#1e1e2e` for panel separation
- **Status Dots:** Small circles (8-10px) with matching glow for connection status

---

## 3. Layout & Structure

### Overall Layout
- **Left Sidebar** (200px fixed): Navigation + Logo
- **Main Content** (flexible): Feature-specific views
- **No top bar**: Maximizes vertical content space

### Navigation Structure
```
┌─────────────────────────────────────────┐
│ PROXYBOT                          [≡]   │
├──────────┬──────────────────────────────┤
│          │                              │
│ Traffic  │     Main Content Area         │
│ Rules    │                              │
│ Certs    │                              │
│ Devices  │                              │
│ DNS      │                              │
│ Alerts   │                              │
│ Replay   │                              │
│ Composer │                              │
│ Graph    │                              │
│ Settings │                              │
│          │                              │
└──────────┴──────────────────────────────┘
```

### Responsive Strategy
- Minimum window: 900x600
- Sidebar collapses to icons at < 1000px width (future enhancement)

---

## 4. Features & Interactions

### 4.1 Traffic Monitor (Priority 1)

**Layout:**
- Left panel (320px): Connection list with domain/app/size columns
- Right panel (flex): Request detail with tabs

**Connection List:**
- Each row: status dot (colored by app) + domain + app tag + size
- Selected row: left border `2px solid #00d4ff`, background `rgba(0,212,255,0.08)`
- Filter input at top for domain search

**Request Detail Tabs:**
- Request, Response, Headers
- Active tab: bottom border + accent color

**Top Toolbar:**
- Start Proxy (green), Stop (red) buttons
- Filter by domain input
- Status indicator with timestamp

### 4.2 Devices (Priority 2)

**Layout:**
- Grid of device cards (2 columns on normal width)
- Add Device button in header

**Device Card:**
- Device name + IP + OS version
- Status dot (green = active, purple = idle)
- Stats: request count, data transferred
- Border glow on hover

### 4.3 Rules (Priority 3)

**Layout:**
- List of rule items with toggle switches
- New Rule button in header

**Rule Item:**
- Toggle switch (ON=red glow, OFF=muted)
- Domain pattern + rule type tag
- Blocked count

**Interactions:**
- Toggle click: immediate visual feedback + API call
- Hover: subtle background highlight

---

## 5. Component Inventory

### Navigation Item
- Default: `#8888aa` text, transparent background
- Hover: `#ffffff` text, `#1a1a2e` background
- Active: `#00d4ff` text, `rgba(0,212,255,0.08)` background, `2px` left border in accent

### Action Button
- Primary (Start): Green background `#22c55e`, dark text
- Danger (Stop): Red background `#ff4d4d`, white text
- Outline: Transparent with accent border

### Status Indicator
- 8-10px circle with matching glow
- Green: Active/Connected
- Purple: Idle/Specific app
- Cyan: Selected/Highlighted
- Red: Blocked/Error

### Card
- Background: subtle gradient
- Border: 1px solid `#2a2a4a`
- Border-radius: 8px
- Hover: border brightens to accent

### Toggle Switch
- 36x22px rounded rectangle
- ON: red background with glow
- OFF: muted background

### Input Field
- Dark background `#0a0a0f`
- Border: 1px solid `#2a2a4a`
- Focus: border `#00d4ff` with subtle glow

---

## 6. Technical Approach

### Frontend Stack
- **Framework:** React 18 + TypeScript
- **Routing:** React Router v6
- **UI Library:** shadcn/ui (base components)
- **Styling:** CSS with CSS custom properties for theming
- **Build Tool:** Vite

### Key Files to Modify
- `src/index.css` — Add CSS variables for tech theme
- `src/components/layout/Layout.tsx` — Sidebar navigation
- `src/components/traffic/TrafficPage.tsx` — Traffic UI
- `src/components/devices/DevicesPage.tsx` — Devices UI
- `src/components/rules/RulesPage.tsx` — Rules UI

### Theme Implementation
```css
:root {
  --bg-primary: #0a0a0f;
  --bg-surface: #12121a;
  --bg-elevated: #1a1a2e;
  --border: #1e1e2e;
  --accent-cyan: #00d4ff;
  --accent-purple: #a855f7;
  --success: #22c55e;
  --danger: #ff4d4d;
  --text-primary: #ffffff;
  --text-secondary: #8888aa;
  --text-tertiary: #666688;
}
```

---

## 7. Scope for Phase 1

**In Scope:**
- Tech theme CSS variables + base styles
- Sidebar navigation component
- Traffic page redesign
- Devices page redesign
- Rules page redesign

**Out of Scope (Phase 2+):**
- Certs page redesign
- DNS page redesign
- Alerts/Replay/Composer/Graph pages
- Dark/light mode toggle
- Mobile responsive sidebar

---

## 8. Approval

This design is ready for implementation. Proceed to writing the implementation plan.