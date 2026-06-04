# Deploy Panel Design

**Date:** 2026-06-04
**Author:** Claude
**PRD Story:** US-009 (Redesign deploy panel)

---

## 1. Concept & Vision

A focused deployment bundle preview & export tool. Developers who have captured traffic and inferred APIs/Mocks via the AI pipeline need a one-click way to preview the generated Docker Compose stack, then write it to disk and (optionally) initialize a git repo — all without leaving the ProxyBot desktop app.

The Deploy page should feel like a **build inspector**: clear input → live preview of the three generated artifacts (compose, README, CI) → explicit write + git-init actions with persistent state across reloads.

---

## 2. Why This Exists

US-009 has been skipped in the redesign because "Deploy panel uses existing deploy module, not part of core UI redesign." Reality is different: deploy logic is currently wedged into `GenPage.tsx` as a tab, with no dedicated route, no sidebar entry, and no persistence. This design:

- Promotes deploy to a first-class page (mirrors Alerts / Replay / Graph status)
- Splits the `Write to Disk` and `Initialize Git Repo` actions into separate, explicit buttons
- Persists the last deployment record in SQLite so reload doesn't lose context
- Cleans up `GenPage.tsx` by removing the duplicated deploy tab

---

## 3. Layout & Structure

### Page Layout (within main content area)

```
┌────────────────────────────────────────────────────────┐
│  Deploy                              [ Refresh ]       │  ← panel-header
├────────────────────────────────────────────────────────┤
│  ┌─ Inputs ──────────────────────────────────────────┐  │
│  │  Session ID:    [______________________]          │  │
│  │  Project Name:  [______________________]          │  │
│  │  Output path:   ~/.proxybot/deployments/{name}    │  │  ← read-only
│  │  [✓] Initialize git repo on write                 │  │
│  │                              [ Generate Preview ] │  │
│  └───────────────────────────────────────────────────┘  │
│                                                        │
│  ┌─ Preview ────────────────────────────────────────┐  │
│  │  [ docker-compose.yml ] [ README.md ] [ e2e.yml ]│  │  ← Tabs
│  │  ┌──────────────────────────────────────────────┐ │  │
│  │  │  <CodeViewer>                                │ │  │
│  │  │  version: '3.8'                              │ │  │
│  │  │  services:                                   │ │  │
│  │  │    mock-api: ...                             │ │  │
│  │  │  </CodeViewer>                               │ │  │
│  │  └──────────────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────────────┘  │
│                                                        │
│  ┌─ Actions ─────────────────────────────────────────┐  │
│  │  Bundle path: ~/.proxybot/deployments/proxybot_x  │  │
│  │  [ Write to Disk ]   [ Re-init Git ]              │  │
│  └───────────────────────────────────────────────────┘  │
│                                                        │
│  ┌─ Result / Error ─────────────────────────────────┐  │
│  │  ✓ Deployment bundle created at ...               │  │  ← success banner
│  │  To run:                                          │  │
│  │    cd /path/...                                   │  │
│  │    docker compose up --build                      │  │
│  └───────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

### Sidebar Entry

Add a new item between **Gen** and **AI**:

```ts
{ path: "/deploy", label: "Deploy", icon: <Package size={20} /> }
```

### Tech Theme Compliance

- Use existing CSS variables (`--accent-blue`, `--surface-primary`, `--border`, etc.)
- Code preview via `CodeViewer` component (already in `shared/`)
- `panel` / `panel-header` / `panel-title` classes for consistency
- `error-banner` for errors, custom success banner for `DeploymentResult`

---

## 4. Components

### File Structure

```
src/components/deploy/
  DeployPage.tsx          (~250 lines) — container, state machine, handlers
  DeployForm.tsx          (~80 lines)  — sessionId / projectName inputs + Generate
  DeployPreview.tsx       (~80 lines)  — Tabs(Compose | README | CI) + CodeViewer
  DeployActions.tsx       (~100 lines) — bundle path display + Write + Re-init buttons
  DeployResult.tsx        (~50 lines)  — success / error banner
  DeployPage.css          (~80 lines)  — page-local styles
```

### State

```ts
type Phase = "idle" | "generating" | "preview" | "writing" | "done" | "error";

interface DeployState {
  // inputs (defaults: sessionId="", projectName="proxybot_deployment", initGit=true)
  sessionId: string;
  projectName: string;
  initGit: boolean;

  // bundle
  bundle: DeploymentBundle | null;
  activeTab: "compose" | "readme" | "ci";

  // persistence
  bundlePath: string;          // hydrated from get_last_deployment
  lastGitInitAt: string | null;

  // ui
  phase: Phase;
  error: string | null;
  generatingLoading: boolean;
  writingLoading: boolean;
  writeResult: DeploymentResult | null;
}
```

### Button Enablement

| Button | Enabled when |
|--------|--------------|
| Generate Preview | sessionId.trim() ≠ "" && !generating && !writing |
| Write to Disk | bundle ≠ null && !writing |
| Re-init Git | bundlePath ≠ "" && !writing |

### Handlers (in DeployPage)

```ts
async function handleGenerate()    // → bundle = invoke("generate_deployment_bundle", {sessionId, projectName})
async function handleWrite()       // → writeResult = invoke("write_deployment_bundle", {sessionId, projectName, initGit})
async function handleReinitGit()   // → writeResult = invoke("git_init_deployment", {path: bundlePath})
async function loadPersisted()     // on mount → invoke("get_last_deployment", {sessionId, projectName})
```

### Error & Loading Patterns

- **Error:** `error-banner` block at top, with `Retry` button calling the last failed handler
- **Loading (generate):** `<SkeletonCard />` inside preview area
- **Loading (write/re-init):** button shows spinner + disabled, other actions disabled
- **Empty (idle):** centered empty state with package icon and instructions

---

## 5. Backend Changes

### 5.1 New SQLite Table (`src-tauri/src/db.rs`)

```sql
CREATE TABLE IF NOT EXISTS deployments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    project_name TEXT NOT NULL,
    bundle_path TEXT NOT NULL,
    last_git_init_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(session_id, project_name)
);
```

Add `pub fn upsert_deployment(...)` and `pub fn get_deployment(session_id, project_name) -> Option<DeploymentRecord>` to `db.rs`.

### 5.2 Tauri Command Changes (`src-tauri/src/deploy.rs`)

**Modify `write_deployment_bundle`:**

```rust
pub fn write_deployment_bundle(
    db: State<'_, Arc<DbState>>,
    session_id: String,
    project_name: Option<String>,
    output_dir: Option<String>,
    init_git: bool,                          // ← new
) -> Result<DeploymentResult, String> {
    // ... existing write logic ...

    if init_git {
        if let Err(e) = init_git_repo(&base_path) {
            log::warn!("Git init failed (non-fatal): {}", e);
        }
    }

    // Persist record
    crate::db::upsert_deployment(
        &db.conn.lock().map_err(|e| e.to_string())?,
        &session_id,
        &name,
        &base,
        if init_git { Some(chrono_lite_now()) } else { None },
    )?;

    Ok(DeploymentResult { ... })
}
```

**Make `init_git_repo` public:**

```rust
pub fn init_git_repo(base_path: &PathBuf) -> Result<(), String> { ... }
```

**Add `git_init_deployment` command:**

```rust
#[tauri::command]
pub fn git_init_deployment(
    db: State<'_, Arc<DbState>>,
    path: String,
) -> Result<DeploymentResult, String> {
    let base_path = PathBuf::from(&path);
    if !base_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    // Note: init_git_repo also creates .github/workflows/ and .gitignore
    // (idempotent — re-running with existing files overwrites them)
    init_git_repo(&base_path)?;

    // Update last_git_init_at
    // (query existing row by path, update timestamp)

    Ok(DeploymentResult {
        success: true,
        bundle_path: path.clone(),
        message: format!("Git repository re-initialized at {}", path),
    })
}
```

**Add `get_last_deployment` command:**

```rust
#[tauri::command]
pub fn get_last_deployment(
    db: State<'_, Arc<DbState>>,
    session_id: String,
    project_name: String,
) -> Result<Option<DeploymentRecord>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::get_deployment(&conn, &session_id, &project_name)
}
```

### 5.3 Register in `src-tauri/src/lib.rs`

```rust
// invoke_handler array:
{generate_deployment_bundle, write_deployment_bundle, git_init_deployment, get_last_deployment}
```

### 5.4 New `DeploymentRecord` Struct (`deploy.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecord {
    pub id: i64,
    pub session_id: String,
    pub project_name: String,
    pub bundle_path: String,
    pub last_git_init_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

---

## 6. Frontend TypeScript Types

```ts
interface DeploymentBundle {
  name: string;
  base_path: string;
  mock_api_path: string;
  frontend_path: string;
  docker_compose_content: string;
  readme_content: string;
  ci_template_content: string;
}

interface DeploymentResult {
  success: boolean;
  bundle_path: string;
  message: string;
}

interface DeploymentRecord {
  id: number;
  session_id: string;
  project_name: string;
  bundle_path: string;
  last_git_init_at: string | null;
  created_at: string;
  updated_at: string;
}
```

---

## 7. Wiring

### Route Registration (`src/main.tsx`)

```tsx
import { DeployPage } from "./components/deploy/DeployPage";
// ...
<Route path="deploy" element={<DeployPage />} />
```

### Sidebar Entry (`src/components/layout/Sidebar.tsx`)

```tsx
import { Package } from "lucide-react";
// add to navItems:
{ path: "/deploy", label: "Deploy", icon: <Package size={20} /> },
```

### `GenPage.tsx` Cleanup

Remove the deploy tab block (everything from `// Deploy state` declarations through the deploy tab content). The `DeploymentBundle` interface can also be removed if no other code references it.

---

## 8. Acceptance Criteria Mapping

| US-009 Requirement | Implementation |
|--------------------|----------------|
| Project name input + Generate button | `DeployForm` |
| Preview area for generated docker-compose.yml | `DeployPreview` with Compose tab (default) |
| Write to disk + Git init buttons | `DeployActions` exposes both |
| Success/error status messages | `DeployResult` + `error-banner` |
| Loading state during generation | `SkeletonCard` while `generatingLoading` |
| Typecheck passes | All TypeScript types declared, no `any` |
| Verify in browser using dev-browser skill | E2E test + manual smoke test |

Additional benefits beyond US-009:
- Tabs to preview all three artifacts (compose, README, CI)
- Persistent bundle path via SQLite hydration
- Cleaner `GenPage.tsx`

---

## 9. Testing

### Backend (Rust)

- `db.rs` — `upsert_deployment` / `get_deployment` unit tests
- `deploy.rs::git_init_deployment` — error path (path does not exist) test

### E2E (`e2e/deploy.spec.ts`)

Following the pattern in `e2e/all-pages.spec.ts`:
1. Navigate to `/deploy`
2. Verify inputs present (sessionId, projectName, Generate button)
3. Enter values, click Generate, verify preview area shows docker-compose content
4. Switch tabs, verify content changes
5. Click Write to Disk, verify success message
6. Click Re-init Git, verify success message

### Type Check

- `npm run typecheck` — no errors
- `cargo check` — no errors

### Manual Browser Verification

- `npm run dev` to start Vite
- Navigate to `/deploy`
- Fill session/project, generate, switch tabs, write to disk
- Verify file system output at expected path
- Reload page, verify `bundlePath` is hydrated

---

## 10. Out of Scope

- Listing all historical deployments (only the most-recent for the current session/project is hydrated)
- Custom output_dir (always uses default `~/.proxybot/deployments/{name}`)
- Editing docker-compose.yml in the browser
- Triggering `docker compose up` from the UI
- Git push to remote (only local `git init` + initial commit)

---

## 11. Files Changed Summary

| File | Action | Lines |
|------|--------|-------|
| `src-tauri/src/deploy.rs` | Modify + add commands | +90 / -10 |
| `src-tauri/src/db.rs` | Add table + CRUD | +60 |
| `src-tauri/src/lib.rs` | Register 2 new commands | +1 / -1 |
| `src/components/deploy/DeployPage.tsx` | New | ~250 |
| `src/components/deploy/DeployForm.tsx` | New | ~80 |
| `src/components/deploy/DeployPreview.tsx` | New | ~80 |
| `src/components/deploy/DeployActions.tsx` | New | ~100 |
| `src/components/deploy/DeployResult.tsx` | New | ~50 |
| `src/components/deploy/DeployPage.css` | New | ~80 |
| `src/components/gen/GenPage.tsx` | Remove deploy tab | -150 |
| `src/components/layout/Sidebar.tsx` | Add Deploy entry | +2 / -1 |
| `src/main.tsx` | Add route | +1 / 0 |
| `e2e/deploy.spec.ts` | New | ~80 |

**Total:** ~640 lines new, ~160 lines removed.

---

## 12. Approval

This design is ready for implementation. Proceed to writing the implementation plan.
