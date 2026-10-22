# ProxyBot PC Client Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply cyberpunk tech theme to ProxyBot UI with neon cyan/purple accents, left sidebar navigation, and redesigned Traffic/Devices/Rules pages.

**Architecture:** CSS variables define the design system; React components use Tailwind utility classes with custom color tokens. Sidebar uses existing Layout/Sidebar pattern. Traffic/Devices/Rules pages get updated component trees.

**Tech Stack:** React 18, TypeScript, Tailwind CSS, React Router v6

---

## File Structure

```
src/
├── index.css                    # Design tokens + base styles
├── components/
│   ├── layout/
│   │   └── Sidebar.tsx         # Main navigation
│   ├── traffic/
│   │   ├── TrafficPage.tsx     # Main traffic view
│   │   ├── RequestTable.tsx    # Connection list
│   │   └── RequestDetail.tsx   # Request detail panel
│   ├── devices/
│   │   └── DevicesPage.tsx     # Device cards
│   └── rules/
│       └── RulesPage.tsx        # Rule list + toggles
```

---

## Task 1: Update CSS Design Tokens (Tech Theme)

**Files:**
- Modify: `src/index.css:32-106` (CSS variables section)

- [ ] **Step 1: Update background and surface colors**

Replace lines 32-37 with:
```css
/* Colors — Cyberpunk Dark */
--bg-primary: #0a0a0f;
--bg-secondary: #12121a;
--bg-tertiary: #1a1a2e;
--bg-elevated: #16162a;
```

- [ ] **Step 2: Update text colors**

Replace lines 38-41 with:
```css
--text-primary: #ffffff;
--text-secondary: #8888aa;
--text-muted: #666688;
```

- [ ] **Step 3: Update border colors**

Replace lines 43-44 with:
```css
--border: #1e1e2e;
--border-light: #2a2a4a;
```

- [ ] **Step 4: Update accent colors**

Replace lines 46-51 with:
```css
/* Accents — Neon */
--accent-blue: #00d4ff;
--accent-green: #3ecf8e;
--accent-yellow: #f4d35e;
--accent-red: #ff4d4d;
--accent-purple: #a855f7;
```

- [ ] **Step 5: Add glow effects CSS**

Add before line 108 (after `--content-max-h: 600px;`):
```css
/* Glow effects for tech aesthetic */
--glow-cyan: 0 0 8px rgba(0, 212, 255, 0.5);
--glow-purple: 0 0 8px rgba(168, 85, 247, 0.5);
--glow-green: 0 0 8px rgba(62, 207, 142, 0.5);
--glow-red: 0 0 8px rgba(255, 77, 77, 0.5);
```

- [ ] **Step 6: Commit**

```bash
git add src/index.css
git commit -m "style: apply cyberpunk theme CSS variables"
```

---

## Task 2: Update Sidebar Component (Tech Nav Style)

**Files:**
- Modify: `src/components/layout/Sidebar.tsx:64-78` (nav item styles)
- Modify: `src/index.css` (focus/active state colors if needed)

- [ ] **Step 1: Update active nav item styles**

Replace lines 64-78 in Sidebar.tsx with:
```tsx
<Link
  key={item.path}
  to={item.path}
  title={collapsed ? item.label : undefined}
  className={`flex items-center gap-3 mx-2 px-4 py-2.5 rounded-lg transition-all duration-200 ${
    isActive
      ? "bg-[rgba(0,212,255,0.08)] text-accent-blue border-l-2 border-accent-blue"
      : "border-l-2 border-transparent text-text-secondary hover:bg-surface-secondary hover:text-text-primary"
  }`}
>
  <span className={isActive ? "text-accent-blue" : ""}>{item.icon}</span>
  {!collapsed && <span>{item.label}</span>}
</Link>
```

- [ ] **Step 2: Update sidebar header styles**

Replace lines 49-56 with:
```tsx
<div className="flex items-center justify-between p-4 border-b border-border">
  {!collapsed && (
    <span className="font-bold text-accent-blue tracking-wider">
      PROXYBOT
    </span>
  )}
  <button
    onClick={() => setCollapsed(!collapsed)}
    className="p-1 hover:bg-surface-tertiary rounded text-text-secondary hover:text-text-primary transition-colors"
  >
    {collapsed ? <Menu size={20} /> : <X size={20} />}
  </button>
</div>
```

- [ ] **Step 3: Update Settings link style**

Replace lines 83-96 with:
```tsx
<div className="p-3 border-t border-border">
  <Link
    to="/settings"
    title={collapsed ? "Settings" : undefined}
    className={`flex items-center gap-3 px-4 py-2.5 rounded-lg transition-all duration-200 ${
      location.pathname === "/settings"
        ? "bg-[rgba(0,212,255,0.08)] text-accent-blue border-l-2 border-accent-blue"
        : "border-l-2 border-transparent text-text-secondary hover:bg-surface-secondary hover:text-text-primary"
    }`}
  >
    <span className={location.pathname === "/settings" ? "text-accent-blue" : ""}>
      <Settings size={20} />
    </span>
    {!collapsed && <span>Settings</span>}
  </Link>
</div>
```

- [ ] **Step 4: Commit**

```bash
git add src/components/layout/Sidebar.tsx
git commit -m "style: apply tech theme to sidebar navigation"
```

---

## Task 3: Update TrafficPage Component

**Files:**
- Modify: `src/components/traffic/TrafficPage.tsx`
- Modify: `src/components/traffic/RequestTable.tsx`
- Modify: `src/components/traffic/RequestDetail.tsx`

- [ ] **Step 1: Update TrafficPage layout structure**

Read current `src/components/traffic/TrafficPage.tsx`, then update the main container structure:
- Change main container to use full viewport height layout
- Add top toolbar with Start/Stop buttons
- Make connection list and detail panel side-by-side

```tsx
// New structure for TrafficPage
<div className="flex flex-col h-full">
  {/* Top Toolbar */}
  <div className="h-14 bg-surface-secondary border-b border-border flex items-center justify-between px-5">
    <div className="flex items-center gap-3">
      <button className="btn btn-sm bg-accent-green text-black">Start Proxy</button>
      <button className="btn btn-sm bg-accent-red text-white">Stop</button>
      <div className="w-px h-6 bg-border mx-2"></div>
      <input
        className="w-60 bg-bg-primary border-border text-text-primary text-sm"
        placeholder="Filter by domain..."
      />
    </div>
    <div className="flex items-center gap-4 text-sm">
      <span className="text-accent-blue">● Proxy Running</span>
      <span className="text-text-muted">12:34:56</span>
    </div>
  </div>

  {/* Main content */}
  <div className="flex-1 flex overflow-hidden">
    {/* Connection list - 320px */}
    <div className="w-80 border-r border-border overflow-y-auto">
      <RequestTable />
    </div>
    {/* Request detail - flex */}
    <div className="flex-1 overflow-y-auto">
      <RequestDetail />
    </div>
  </div>
</div>
```

- [ ] **Step 2: Update RequestTable row styling**

Read `RequestTable.tsx`, update each connection row:
- Add colored status dot with glow effect
- Style app tag with accent color
- Add left border highlight on selected item

```tsx
// Status dot with glow
<div
  className="w-2 h-2 rounded-full"
  style={{
    background: appColor,
    boxShadow: `0 0 8px ${appColor}`
  }}
/>
// Selected row
<div className={`border-l-2 ${isSelected ? 'border-accent-blue bg-[rgba(0,212,255,0.08)]' : 'border-transparent'}`}>
```

- [ ] **Step 3: Update RequestDetail tabs**

Read `RequestDetail.tsx`, update tab styles:
```tsx
// Tab styling
<span className={`text-sm pb-2 px-1 ${
  isActive
    ? 'text-accent-blue border-b-2 border-accent-blue'
    : 'text-text-secondary'
}`}>
  {label}
</span>
```

- [ ] **Step 4: Commit**

```bash
git add src/components/traffic/TrafficPage.tsx src/components/traffic/RequestTable.tsx src/components/traffic/RequestDetail.tsx
git commit -m "style: apply tech theme to Traffic page"
```

---

## Task 4: Update DevicesPage Component

**Files:**
- Modify: `src/components/devices/DevicesPage.tsx`

- [ ] **Step 1: Read current DevicesPage structure**

Read `src/components/devices/DevicesPage.tsx` to understand current layout.

- [ ] **Step 2: Update device cards with tech styling**

Update device card component:
```tsx
<div
  className="bg-gradient-to-br from-surface-elevated to-surface-secondary
             border border-border rounded-lg p-4
             hover:border-accent-cyan/40 transition-all duration-200"
>
  {/* Status dot with glow */}
  <div
    className="w-2.5 h-2.5 rounded-full"
    style={{
      background: statusColor,
      boxShadow: `0 0 8px ${statusColor}`
    }}
  />
  {/* Device name in white, IP in muted */}
  <div className="text-text-primary font-medium">{name}</div>
  <div className="text-text-muted text-xs">{ip} • {os}</div>
</div>
```

- [ ] **Step 3: Add gradient to card backgrounds**

Update CSS for device cards:
```css
.device-card {
  background: linear-gradient(135deg, #1a1a2e 0%, #12121a 100%);
}
```

- [ ] **Step 4: Commit**

```bash
git add src/components/devices/DevicesPage.tsx
git commit -m "style: apply tech theme to Devices page"
```

---

## Task 5: Update RulesPage Component

**Files:**
- Modify: `src/components/rules/RulesPage.tsx`

- [ ] **Step 1: Read current RulesPage structure**

Read `src/components/rules/RulesPage.tsx` to understand current layout.

- [ ] **Step 2: Update rule item with toggle switch styling**

```tsx
<div className="flex items-center gap-3 p-3 bg-surface-secondary rounded-lg
                hover:bg-surface-tertiary transition-colors">
  {/* Toggle switch - ON state */}
  <div className={`w-9 h-5.5 rounded-full flex items-center justify-center transition-all ${
    isEnabled
      ? 'bg-accent-red/40 border border-accent-red/50'
      : 'bg-surface-tertiary border border-border'
  }`}>
    <span className={`text-xs font-medium ${isEnabled ? 'text-accent-red' : 'text-text-muted'}`}>
      {isEnabled ? 'ON' : 'OFF'}
    </span>
  </div>

  {/* Domain and rule type */}
  <div className="flex-1">
    <div className="text-text-primary font-mono text-sm">{domain}</div>
    <div className="text-text-muted text-xs">{ruleType} • {category}</div>
  </div>

  {/* Blocked count with danger color */}
  <span className="text-accent-red text-xs">{blocked} blocked</span>
</div>
```

- [ ] **Step 3: Style the New Rule button**

```tsx
<button className="btn btn-sm bg-accent-blue text-black hover:bg-accent-blue/80">
  + New Rule
</button>
```

- [ ] **Step 4: Commit**

```bash
git add src/components/rules/RulesPage.tsx
git commit -m "style: apply tech theme to Rules page"
```

---

## Task 6: Final Verification

- [ ] **Step 1: Run dev server and verify all pages render**

```bash
cd /Users/doug/orca/workspaces/proxybot/onboarding && pnpm dev
```

Open browser and check:
- [ ] Sidebar shows PROXYBOT in cyan with correct nav highlighting
- [ ] Traffic page shows connection list + detail panel
- [ ] Devices page shows device cards with status glow
- [ ] Rules page shows toggle switches with ON/OFF styling

- [ ] **Step 2: Test start/stop proxy buttons**

- [ ] **Step 3: Test navigation between all pages**

- [ ] **Step 4: Commit remaining changes if any**

```bash
git status
git add -A
git commit -m "feat: complete ProxyBot tech theme redesign"
```

---

## Self-Review Checklist

- [ ] All CSS variables updated with cyberpunk colors
- [ ] Sidebar active state shows cyan border + glow
- [ ] Traffic connection rows show colored status dots with glow
- [ ] Device cards have gradient backgrounds + hover glow
- [ ] Rule toggles show ON (red glow) / OFF (muted) states
- [ ] No placeholder text remaining
- [ ] All components consistent with design spec

---

## Execution Options

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?