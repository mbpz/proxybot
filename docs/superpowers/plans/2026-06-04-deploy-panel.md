# Deploy Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a dedicated Deploy page that wraps the existing `deploy.rs` backend, allowing users to preview a Docker Compose deployment bundle, write it to disk, and (separately) initialize a git repo.

**Architecture:** Five small React components in a new `src/components/deploy/` directory orchestrated by `DeployPage`. State held in DeployPage, subcomponents are presentational. Backend gains a SQLite `deployments` table, two new Tauri commands (`git_init_deployment`, `get_last_deployment`), and a new `init_git: bool` parameter on `write_deployment_bundle`. The deploy logic is removed from `GenPage.tsx` and a new `/deploy` route + sidebar entry wire it in.

**Tech Stack:** Tauri 2 + Rust (rusqlite) + React 18 + TypeScript + Vite + Playwright E2E

**Spec:** `docs/superpowers/specs/2026-06-04-deploy-panel-design.md` (commit `c3ecdd8`)

---

## File Structure

**New files:**
- `src-tauri/src/deploy.rs` — new `DeploymentRecord` struct, `git_init_deployment` command, `get_last_deployment` command, `init_git_repo` made public, `init_git: bool` added to `write_deployment_bundle`
- `src/components/deploy/DeployPage.tsx` — container (~250 lines)
- `src/components/deploy/DeployForm.tsx` — inputs + Generate (~80 lines)
- `src/components/deploy/DeployPreview.tsx` — Tabs + CodeViewer (~80 lines)
- `src/components/deploy/DeployActions.tsx` — path + Write + Re-init buttons (~100 lines)
- `src/components/deploy/DeployResult.tsx` — success/error banner (~50 lines)
- `src/components/deploy/DeployPage.css` — page-local styles (~80 lines)
- `e2e/deploy.spec.ts` — Playwright E2E tests

**Modified files:**
- `src-tauri/src/db.rs` — migration 4 adds `deployments` table, two new public functions
- `src-tauri/src/lib.rs` — register `git_init_deployment` and `get_last_deployment`
- `src/components/layout/Sidebar.tsx` — add Deploy entry
- `src/main.tsx` — register `/deploy` route
- `src/components/gen/GenPage.tsx` — remove deploy tab + related state

---

## Task 1: Add `deployments` table migration + db functions (TDD)

**Files:**
- Modify: `src-tauri/src/db.rs:316-348` (add migration 4)
- Modify: `src-tauri/src/db.rs` (add `DeploymentRecord` struct + two functions)
- Test: `src-tauri/src/db.rs:871` (extend `tests` module)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `db.rs`:

```rust
#[test]
fn test_deployments_table_upsert_and_get() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
    DbState::init_schema(&conn).unwrap();

    upsert_deployment(&conn, "sess1", "proj1", "/tmp/proj1", Some("2026-06-04T00:00:00Z")).unwrap();
    let rec = get_deployment(&conn, "sess1", "proj1").unwrap().unwrap();
    assert_eq!(rec.session_id, "sess1");
    assert_eq!(rec.project_name, "proj1");
    assert_eq!(rec.bundle_path, "/tmp/proj1");
    assert_eq!(rec.last_git_init_at, Some("2026-06-04T00:00:00Z".to_string()));

    // Upsert updates the path
    upsert_deployment(&conn, "sess1", "proj1", "/tmp/proj1_v2", None).unwrap();
    let rec = get_deployment(&conn, "sess1", "proj1").unwrap().unwrap();
    assert_eq!(rec.bundle_path, "/tmp/proj1_v2");
    assert_eq!(rec.last_git_init_at, None);

    // Missing returns None
    let none = get_deployment(&conn, "sess1", "missing").unwrap();
    assert!(none.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --lib test_deployments_table_upsert_and_get`
Expected: FAIL with "cannot find function `upsert_deployment`" or similar.

- [ ] **Step 3: Add migration 4 to db.rs**

In `db.rs`, find the `migrations` vec (around line 319) and append a 4th entry:

```rust
(
    4,
    "Add deployments table for Deploy panel persistence",
    r#"
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
    CREATE INDEX IF NOT EXISTS idx_deployments_session_project
        ON deployments(session_id, project_name);
    "#,
),
```

- [ ] **Step 4: Add the struct and two functions to db.rs**

Append after the `pub fn get_db_stats` block (or near the end of the file, before the test module):

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

pub fn upsert_deployment(
    conn: &Connection,
    session_id: &str,
    project_name: &str,
    bundle_path: &str,
    last_git_init_at: Option<&str>,
) -> Result<(), String> {
    let now = chrono_lite_timestamp();
    conn.execute(
        r#"
        INSERT INTO deployments
            (session_id, project_name, bundle_path, last_git_init_at, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?5)
        ON CONFLICT(session_id, project_name) DO UPDATE SET
            bundle_path = excluded.bundle_path,
            last_git_init_at = COALESCE(excluded.last_git_init_at, deployments.last_git_init_at),
            updated_at = excluded.updated_at
        "#,
        rusqlite::params![session_id, project_name, bundle_path, last_git_init_at, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_deployment(
    conn: &Connection,
    session_id: &str,
    project_name: &str,
) -> Result<Option<DeploymentRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, project_name, bundle_path, last_git_init_at, created_at, updated_at
             FROM deployments WHERE session_id = ?1 AND project_name = ?2",
        )
        .map_err(|e| e.to_string())?;
    let mut rows = stmt
        .query_map(rusqlite::params![session_id, project_name], |row| {
            Ok(DeploymentRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                project_name: row.get(2)?,
                bundle_path: row.get(3)?,
                last_git_init_at: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    match rows.next() {
        Some(Ok(rec)) => Ok(Some(rec)),
        Some(Err(e)) => Err(e.to_string()),
        None => Ok(None),
    }
}
```

Add `use serde::{Deserialize, Serialize};` at the top of `db.rs` if not already present.

- [ ] **Step 5: Run test to verify it passes**

Run: `cd src-tauri && cargo test --lib test_deployments_table_upsert_and_get`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): add deployments table for Deploy panel persistence"
```

---

## Task 2: Modify `write_deployment_bundle` to accept `init_git: bool`

**Files:**
- Modify: `src-tauri/src/deploy.rs:567-866` (`write_deployment_bundle` function)
- Modify: `src-tauri/src/deploy.rs:363` (`init_git_repo` — make `pub`)

- [ ] **Step 1: Make `init_git_repo` public**

In `src-tauri/src/deploy.rs`, find:
```rust
fn init_git_repo(base_path: &PathBuf) -> Result<(), String> {
```
Change to:
```rust
pub fn init_git_repo(base_path: &PathBuf) -> Result<(), String> {
```

- [ ] **Step 2: Add `init_git: bool` parameter to `write_deployment_bundle`**

Find the function signature (around line 567):
```rust
pub fn write_deployment_bundle(
    db: State<'_, Arc<DbState>>,
    session_id: String,
    project_name: Option<String>,
    output_dir: Option<String>,
) -> Result<DeploymentResult, String> {
```

Change to:
```rust
pub fn write_deployment_bundle(
    db: State<'_, Arc<DbState>>,
    session_id: String,
    project_name: Option<String>,
    output_dir: Option<String>,
    init_git: bool,
) -> Result<DeploymentResult, String> {
```

- [ ] **Step 3: Wrap the git init call in `if init_git`**

Find (around line 851-854):
```rust
    // Initialize git repo
    if let Err(e) = init_git_repo(&base_path) {
        log::warn!("Git init failed (non-fatal): {}", e);
    }
```

Change to:
```rust
    // Initialize git repo if requested
    if init_git {
        if let Err(e) = init_git_repo(&base_path) {
            log::warn!("Git init failed (non-fatal): {}", e);
        }
    }
```

- [ ] **Step 4: Persist the deployment record**

Immediately after the `if init_git { ... }` block, add:

```rust
    // Persist deployment record
    let last_git_init = if init_git { Some(crate::db::chrono_lite_timestamp()) } else { None };
    if let Err(e) = crate::db::upsert_deployment(
        &conn,
        &session_id,
        &name,
        &base,
        last_git_init.as_deref(),
    ) {
        log::warn!("Failed to persist deployment record: {}", e);
    }
```

- [ ] **Step 5: Verify `cargo check` succeeds**

Run: `cd src-tauri && cargo check`
Expected: success (callers of `write_deployment_bundle` only fail in TypeScript, which is handled later)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/deploy.rs
git commit -m "feat(deploy): add init_git parameter to write_deployment_bundle + persist record"
```

---

## Task 3: Add `git_init_deployment` command (TDD)

**Files:**
- Modify: `src-tauri/src/deploy.rs` (add command)

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/src/deploy.rs` (at the end, outside any existing `mod`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn test_git_init_deployment_errors_on_missing_path() {
        let path = "/tmp/definitely_does_not_exist_xyz_12345";
        // ensure path is gone
        let _ = fs::remove_dir_all(path);
        let result = git_init_deployment_inner(path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_git_init_deployment_succeeds_on_real_dir() {
        // Create a temp dir
        let tmp = env::temp_dir().join(format!("proxybot_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let result = git_init_deployment_inner(tmp.to_str().unwrap());
        assert!(result.is_ok(), "Got error: {:?}", result.err());

        // Verify .git was created
        assert!(tmp.join(".git").exists());
        assert!(tmp.join(".gitignore").exists());

        let _ = fs::remove_dir_all(&tmp);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib test_git_init_deployment`
Expected: FAIL with "cannot find function `git_init_deployment_inner`"

- [ ] **Step 3: Implement `git_init_deployment_inner`**

Add to `src-tauri/src/deploy.rs` (before the `#[tauri::command]` `git_init_deployment` we'll add next):

```rust
/// Inner testable logic for `git_init_deployment`.
/// Splits the Tauri-State wrapper so we can unit-test it.
pub fn git_init_deployment_inner(path: &str) -> Result<DeploymentResult, String> {
    let base_path = PathBuf::from(path);
    if !base_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    init_git_repo(&base_path)?;
    Ok(DeploymentResult {
        success: true,
        bundle_path: path.to_string(),
        message: format!("Git repository re-initialized at {}", path),
    })
}
```

- [ ] **Step 4: Add the Tauri command wrapper**

Right below `git_init_deployment_inner`:

```rust
#[tauri::command]
pub fn git_init_deployment(
    db: State<'_, Arc<DbState>>,
    path: String,
) -> Result<DeploymentResult, String> {
    let result = git_init_deployment_inner(&path)?;

    // Update last_git_init_at in the deployments table
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let now = crate::db::chrono_lite_timestamp();
    let updated = conn.execute(
        "UPDATE deployments SET last_git_init_at = ?1, updated_at = ?1 WHERE bundle_path = ?2",
        rusqlite::params![now, path],
    );
    if let Err(e) = updated {
        log::warn!("Failed to update last_git_init_at: {}", e);
    }

    Ok(result)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib test_git_init_deployment`
Expected: PASS for both tests

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/deploy.rs
git commit -m "feat(deploy): add git_init_deployment command"
```

---

## Task 4: Add `get_last_deployment` command (TDD)

**Files:**
- Modify: `src-tauri/src/deploy.rs` (add command)

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `deploy.rs`:

```rust
    #[test]
    fn test_get_last_deployment_returns_none_when_missing() {
        use crate::db::DbState;
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();
        let result = get_last_deployment_inner(&conn, "nope", "nope");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_get_last_deployment_returns_record() {
        use crate::db::DbState;
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        DbState::init_schema(&conn).unwrap();
        crate::db::upsert_deployment(&conn, "s1", "p1", "/x", None).unwrap();
        let rec = get_last_deployment_inner(&conn, "s1", "p1").unwrap().unwrap();
        assert_eq!(rec.bundle_path, "/x");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib test_get_last_deployment`
Expected: FAIL with "cannot find function `get_last_deployment_inner`"

- [ ] **Step 3: Implement `get_last_deployment_inner`**

Add to `deploy.rs`:

```rust
/// Inner testable logic for `get_last_deployment`.
pub fn get_last_deployment_inner(
    conn: &rusqlite::Connection,
    session_id: &str,
    project_name: &str,
) -> Result<Option<DeploymentRecord>, String> {
    crate::db::get_deployment(conn, session_id, project_name)
}
```

- [ ] **Step 4: Add the Tauri command wrapper**

```rust
#[tauri::command]
pub fn get_last_deployment(
    db: State<'_, Arc<DbState>>,
    session_id: String,
    project_name: String,
) -> Result<Option<DeploymentRecord>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    get_last_deployment_inner(&conn, &session_id, &project_name)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd src-tauri && cargo test --lib test_get_last_deployment`
Expected: PASS for both tests

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/deploy.rs
git commit -m "feat(deploy): add get_last_deployment command"
```

---

## Task 5: Register new commands in `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs:67` (add to invoke_handler array)

- [ ] **Step 1: Update the `invoke_handler` array**

Find (around line 67):
```rust
{generate_deployment_bundle, write_deployment_bundle};
```

Change to:
```rust
{generate_deployment_bundle, write_deployment_bundle, git_init_deployment, get_last_deployment};
```

- [ ] **Step 2: Verify `cargo check` succeeds**

Run: `cd src-tauri && cargo check`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(deploy): register git_init_deployment and get_last_deployment commands"
```

---

## Task 6: Add TypeScript types for the deploy domain

**Files:**
- Create: `src/components/deploy/types.ts`

- [ ] **Step 1: Create the types file**

```ts
export interface DeploymentBundle {
  name: string;
  base_path: string;
  mock_api_path: string;
  frontend_path: string;
  docker_compose_content: string;
  readme_content: string;
  ci_template_content: string;
}

export interface DeploymentResult {
  success: boolean;
  bundle_path: string;
  message: string;
}

export interface DeploymentRecord {
  id: number;
  session_id: string;
  project_name: string;
  bundle_path: string;
  last_git_init_at: string | null;
  created_at: string;
  updated_at: string;
}

export type DeployTab = "compose" | "readme" | "ci";
```

- [ ] **Step 2: Verify typecheck passes**

Run: `npm run typecheck`
Expected: success (no errors related to new file)

- [ ] **Step 3: Commit**

```bash
git add src/components/deploy/types.ts
git commit -m "feat(deploy): add TypeScript types for DeploymentBundle/Record/Result"
```

---

## Task 7: Create `DeployForm` component

**Files:**
- Create: `src/components/deploy/DeployForm.tsx`

- [ ] **Step 1: Create the form component**

```tsx
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";

interface DeployFormProps {
  sessionId: string;
  projectName: string;
  initGit: boolean;
  generating: boolean;
  onSessionIdChange: (v: string) => void;
  onProjectNameChange: (v: string) => void;
  onInitGitChange: (v: boolean) => void;
  onGenerate: () => void;
}

export function DeployForm({
  sessionId,
  projectName,
  initGit,
  generating,
  onSessionIdChange,
  onProjectNameChange,
  onInitGitChange,
  onGenerate,
}: DeployFormProps) {
  return (
    <div className="deploy-form">
      <div className="deploy-form-row">
        <label className="deploy-form-label">Session ID</label>
        <Input
          value={sessionId}
          onChange={(e) => onSessionIdChange(e.target.value)}
          placeholder="e.g. 2026-06-04-001"
          disabled={generating}
        />
      </div>
      <div className="deploy-form-row">
        <label className="deploy-form-label">Project Name</label>
        <Input
          value={projectName}
          onChange={(e) => onProjectNameChange(e.target.value)}
          placeholder="proxybot_deployment"
          disabled={generating}
        />
      </div>
      <div className="deploy-form-row">
        <label className="deploy-form-label">Output Path</label>
        <code className="deploy-form-path">~/.proxybot/deployments/{projectName || "proxybot_deployment"}</code>
      </div>
      <div className="deploy-form-row deploy-form-checkbox-row">
        <label className="deploy-form-checkbox">
          <input
            type="checkbox"
            checked={initGit}
            onChange={(e) => onInitGitChange(e.target.checked)}
            disabled={generating}
          />
          <span>Initialize git repo on write</span>
        </label>
      </div>
      <div className="deploy-form-actions">
        <Button
          variant="primary"
          onClick={onGenerate}
          disabled={generating || !sessionId.trim()}
        >
          {generating ? "Generating..." : "Generate Preview"}
        </Button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify typecheck passes**

Run: `npm run typecheck`
Expected: success (this file is not yet imported anywhere; tsc should still parse it)

- [ ] **Step 3: Commit**

```bash
git add src/components/deploy/DeployForm.tsx
git commit -m "feat(deploy): add DeployForm component"
```

---

## Task 8: Create `DeployPreview` component

**Files:**
- Create: `src/components/deploy/DeployPreview.tsx`

- [ ] **Step 1: Create the preview component**

```tsx
import { Tabs } from "../ui/Tabs";
import { CodeViewer } from "../shared/CodeViewer";
import { SkeletonCard } from "../ui/skeleton";
import { ErrorBoundary } from "../ui/error-boundary";
import type { DeploymentBundle, DeployTab } from "./types";

interface DeployPreviewProps {
  bundle: DeploymentBundle | null;
  activeTab: DeployTab;
  loading: boolean;
  onTabChange: (t: DeployTab) => void;
}

export function DeployPreview({ bundle, activeTab, loading, onTabChange }: DeployPreviewProps) {
  if (loading) {
    return (
      <div className="deploy-preview">
        <SkeletonCard />
      </div>
    );
  }

  if (!bundle) {
    return (
      <div className="deploy-preview deploy-preview-empty">
        <div className="empty-state">
          <div className="empty-state-icon">🐳</div>
          <div className="empty-state-title">No preview yet</div>
          <div className="empty-state-description">
            Fill in a session ID and click <strong>Generate Preview</strong> to see the
            Docker Compose stack that would be produced.
          </div>
        </div>
      </div>
    );
  }

  const tabs = [
    { id: "compose", label: "docker-compose.yml" },
    { id: "readme", label: "README.md" },
    { id: "ci", label: "e2e.yml" },
  ];

  const content =
    activeTab === "compose"
      ? bundle.docker_compose_content
      : activeTab === "readme"
      ? bundle.readme_content
      : bundle.ci_template_content;

  const contentType =
    activeTab === "compose" ? "yaml" : activeTab === "readme" ? "markdown" : "yaml";

  return (
    <div className="deploy-preview">
      <Tabs tabs={tabs} activeTab={activeTab} onTabChange={(t) => onTabChange(t as DeployTab)} />
      <div className="deploy-preview-content">
        <ErrorBoundary>
          <CodeViewer content={content} contentType={contentType} maxHeight="32rem" />
        </ErrorBoundary>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify typecheck passes**

Run: `npm run typecheck`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add src/components/deploy/DeployPreview.tsx
git commit -m "feat(deploy): add DeployPreview component with Tabs + CodeViewer"
```

---

## Task 9: Create `DeployActions` component

**Files:**
- Create: `src/components/deploy/DeployActions.tsx`

- [ ] **Step 1: Create the actions component**

```tsx
import { Button } from "../ui/Button";

interface DeployActionsProps {
  bundlePath: string;
  hasBundle: boolean;
  writing: boolean;
  onWrite: () => void;
  onReinitGit: () => void;
}

export function DeployActions({
  bundlePath,
  hasBundle,
  writing,
  onWrite,
  onReinitGit,
}: DeployActionsProps) {
  return (
    <div className="deploy-actions">
      <div className="deploy-actions-path-row">
        <span className="deploy-actions-label">Bundle path:</span>
        <code className="deploy-actions-path">{bundlePath || "(not yet written)"}</code>
      </div>
      <div className="deploy-actions-buttons">
        <Button
          variant="primary"
          onClick={onWrite}
          disabled={!hasBundle || writing}
        >
          {writing ? "Writing..." : "Write to Disk"}
        </Button>
        <Button
          variant="secondary"
          onClick={onReinitGit}
          disabled={!bundlePath || writing}
        >
          Re-init Git
        </Button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify typecheck passes**

Run: `npm run typecheck`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add src/components/deploy/DeployActions.tsx
git commit -m "feat(deploy): add DeployActions component"
```

---

## Task 10: Create `DeployResult` component

**Files:**
- Create: `src/components/deploy/DeployResult.tsx`

- [ ] **Step 1: Create the result banner component**

```tsx
import { Button } from "../ui/Button";
import type { DeploymentResult } from "./types";

interface DeployResultProps {
  result: DeploymentResult | null;
  error: string | null;
  onRetry?: () => void;
  onDismiss?: () => void;
}

export function DeployResult({ result, error, onRetry, onDismiss }: DeployResultProps) {
  if (error) {
    return (
      <div className="error-banner" style={{ margin: "0 var(--space-4) var(--space-2)" }}>
        <span className="error-banner-message">{error}</span>
        {onRetry && (
          <Button variant="secondary" size="sm" onClick={onRetry}>
            Retry
          </Button>
        )}
      </div>
    );
  }

  if (!result) return null;

  return (
    <div className="deploy-result deploy-result-success">
      <div className="deploy-result-header">
        <span className="deploy-result-icon">✓</span>
        <span className="deploy-result-title">Deployment bundle ready</span>
        {onDismiss && (
          <button
            className="deploy-result-dismiss"
            onClick={onDismiss}
            aria-label="Dismiss"
          >
            ×
          </button>
        )}
      </div>
      <pre className="deploy-result-message">{result.message}</pre>
    </div>
  );
}
```

- [ ] **Step 2: Verify typecheck passes**

Run: `npm run typecheck`
Expected: success

- [ ] **Step 3: Commit**

```bash
git add src/components/deploy/DeployResult.tsx
git commit -m "feat(deploy): add DeployResult component for success/error display"
```

---

## Task 11: Create `DeployPage` orchestrator + CSS

**Files:**
- Create: `src/components/deploy/DeployPage.tsx`
- Create: `src/components/deploy/DeployPage.css`

- [ ] **Step 1: Create the CSS file**

```css
.deploy-page {
  padding: var(--space-4);
}

.deploy-section {
  margin-bottom: var(--space-5);
}

.deploy-section-title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin-bottom: var(--space-3);
}

.deploy-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.deploy-form-row {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.deploy-form-label {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.deploy-form-path {
  font-family: var(--font-mono, "JetBrains Mono", monospace);
  font-size: var(--text-sm);
  color: var(--text-tertiary);
  padding: var(--space-2) var(--space-3);
  background: var(--bg-elevated, #1a1a2e);
  border: 1px solid var(--border, #1e1e2e);
  border-radius: 4px;
}

.deploy-form-checkbox-row {
  flex-direction: row;
  align-items: center;
}

.deploy-form-checkbox {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  cursor: pointer;
  font-size: var(--text-sm);
  color: var(--text-primary);
}

.deploy-form-checkbox input[type="checkbox"] {
  cursor: pointer;
  accent-color: var(--accent-cyan, #00d4ff);
}

.deploy-form-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: var(--space-2);
}

.deploy-preview {
  min-height: 200px;
}

.deploy-preview-content {
  padding: var(--space-2);
}

.deploy-preview-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 200px;
}

.deploy-actions {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.deploy-actions-path-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.deploy-actions-label {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.deploy-actions-path {
  font-family: var(--font-mono, "JetBrains Mono", monospace);
  font-size: var(--text-sm);
  color: var(--accent-cyan, #00d4ff);
  word-break: break-all;
}

.deploy-actions-buttons {
  display: flex;
  gap: var(--space-2);
}

.deploy-result {
  padding: var(--space-3) var(--space-4);
  border-radius: 4px;
  border: 1px solid;
}

.deploy-result-success {
  background: rgba(34, 197, 94, 0.08);
  border-color: var(--success, #22c55e);
  color: var(--text-primary);
}

.deploy-result-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
}

.deploy-result-icon {
  color: var(--success, #22c55e);
  font-size: var(--text-lg);
  font-weight: 700;
}

.deploy-result-title {
  font-weight: 600;
  flex: 1;
}

.deploy-result-dismiss {
  background: transparent;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: var(--text-lg);
  padding: 0 var(--space-2);
}

.deploy-result-dismiss:hover {
  color: var(--text-primary);
}

.deploy-result-message {
  font-family: var(--font-mono, "JetBrains Mono", monospace);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  white-space: pre-wrap;
  margin: 0;
}
```

- [ ] **Step 2: Create the page orchestrator**

```tsx
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { DeployForm } from "./DeployForm";
import { DeployPreview } from "./DeployPreview";
import { DeployActions } from "./DeployActions";
import { DeployResult } from "./DeployResult";
import "./DeployPage.css";
import type {
  DeploymentBundle,
  DeploymentResult as DeploymentResultT,
  DeploymentRecord,
  DeployTab,
} from "./types";

export function DeployPage() {
  // Inputs
  const [sessionId, setSessionId] = useState("");
  const [projectName, setProjectName] = useState("proxybot_deployment");
  const [initGit, setInitGit] = useState(true);

  // Bundle + preview
  const [bundle, setBundle] = useState<DeploymentBundle | null>(null);
  const [activeTab, setActiveTab] = useState<DeployTab>("compose");

  // Persistence
  const [bundlePath, setBundlePath] = useState("");

  // UI
  const [generating, setGenerating] = useState(false);
  const [writing, setWriting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<DeploymentResultT | null>(null);

  // Hydrate last deployment record on mount (when sessionId/projectName set)
  useEffect(() => {
    if (!sessionId.trim()) {
      setBundlePath("");
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const rec = await invoke<DeploymentRecord | null>("get_last_deployment", {
          sessionId,
          projectName,
        });
        if (!cancelled && rec) {
          setBundlePath(rec.bundle_path);
        } else if (!cancelled) {
          setBundlePath("");
        }
      } catch (err) {
        // Non-fatal: just log
        console.error("Failed to load last deployment:", err);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sessionId, projectName]);

  const handleGenerate = useCallback(async () => {
    if (!sessionId.trim()) {
      setError("Session ID is required");
      return;
    }
    setGenerating(true);
    setError(null);
    try {
      const b = await invoke<DeploymentBundle>("generate_deployment_bundle", {
        sessionId,
        projectName: projectName || null,
      });
      setBundle(b);
    } catch (err) {
      setError(String(err));
    } finally {
      setGenerating(false);
    }
  }, [sessionId, projectName]);

  const handleWrite = useCallback(async () => {
    setWriting(true);
    setError(null);
    try {
      const r = await invoke<DeploymentResultT>("write_deployment_bundle", {
        sessionId,
        projectName: projectName || null,
        initGit,
      });
      setResult(r);
      setBundlePath(r.bundle_path);
    } catch (err) {
      setError(String(err));
    } finally {
      setWriting(false);
    }
  }, [sessionId, projectName, initGit]);

  const handleReinitGit = useCallback(async () => {
    if (!bundlePath) return;
    setWriting(true);
    setError(null);
    try {
      const r = await invoke<DeploymentResultT>("git_init_deployment", {
        path: bundlePath,
      });
      setResult(r);
    } catch (err) {
      setError(String(err));
    } finally {
      setWriting(false);
    }
  }, [bundlePath]);

  const lastFailedHandler = generating ? handleGenerate : writing ? handleWrite : null;

  return (
    <div className="deploy-page">
      <div className="panel">
        <div className="panel-header">
          <span className="panel-title">Deploy</span>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => {
              setError(null);
              setResult(null);
            }}
          >
            Reset
          </Button>
        </div>

        <DeployResult
          result={result}
          error={error}
          onRetry={lastFailedHandler ?? undefined}
          onDismiss={() => {
            setError(null);
            setResult(null);
          }}
        />

        <div style={{ padding: "var(--space-4)" }}>
          <div className="deploy-section">
            <div className="deploy-section-title">Inputs</div>
            <DeployForm
              sessionId={sessionId}
              projectName={projectName}
              initGit={initGit}
              generating={generating}
              onSessionIdChange={setSessionId}
              onProjectNameChange={setProjectName}
              onInitGitChange={setInitGit}
              onGenerate={handleGenerate}
            />
          </div>

          <div className="deploy-section">
            <div className="deploy-section-title">Preview</div>
            <DeployPreview
              bundle={bundle}
              activeTab={activeTab}
              loading={generating}
              onTabChange={setActiveTab}
            />
          </div>

          <div className="deploy-section">
            <div className="deploy-section-title">Actions</div>
            <DeployActions
              bundlePath={bundlePath}
              hasBundle={bundle !== null}
              writing={writing}
              onWrite={handleWrite}
              onReinitGit={handleReinitGit}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Verify typecheck passes**

Run: `npm run typecheck`
Expected: success

- [ ] **Step 4: Commit**

```bash
git add src/components/deploy/DeployPage.tsx src/components/deploy/DeployPage.css
git commit -m "feat(deploy): add DeployPage orchestrator + page styles"
```

---

## Task 12: Wire up route + sidebar

**Files:**
- Modify: `src/main.tsx` (add route)
- Modify: `src/components/layout/Sidebar.tsx` (add entry)

- [ ] **Step 1: Add the route in `main.tsx`**

Find (around line 12):
```tsx
import { GenPage } from "./components/gen/GenPage";
```

Add above it:
```tsx
import { DeployPage } from "./components/deploy/DeployPage";
```

Find (around line 41):
```tsx
<Route path="gen" element={<GenPage />} />
```

Add below it:
```tsx
<Route path="deploy" element={<DeployPage />} />
```

- [ ] **Step 2: Add the sidebar entry in `Sidebar.tsx`**

Find (around line 12-23, the imports from `lucide-react`):
```tsx
import {
  Menu,
  X,
  List,
  ...
  Wand2,
  Send,
  Brain,
  Settings,
} from "lucide-react";
```

Add `Package` to the import list:
```tsx
import {
  Menu,
  X,
  List,
  ...
  Wand2,
  Send,
  Brain,
  Package,
  Settings,
} from "lucide-react";
```

Find the `navItems` array (around line 25-36). Add a new entry:
```tsx
{ path: "/gen", label: "Gen", icon: <Wand2 size={20} /> },
{ path: "/deploy", label: "Deploy", icon: <Package size={20} /> },
{ path: "/ai", label: "AI", icon: <Brain size={20} /> },
```

- [ ] **Step 3: Verify typecheck passes**

Run: `npm run typecheck`
Expected: success

- [ ] **Step 4: Verify dev server starts**

Run: `npm run dev` (in background, then kill)
Expected: Vite reports no compile errors.

- [ ] **Step 5: Commit**

```bash
git add src/main.tsx src/components/layout/Sidebar.tsx
git commit -m "feat(deploy): register /deploy route and sidebar entry"
```

---

## Task 13: Clean up `GenPage.tsx` — remove deploy tab

**Files:**
- Modify: `src/components/gen/GenPage.tsx`

- [ ] **Step 1: Identify the deploy tab block**

Open `src/components/gen/GenPage.tsx` and find:
- The `DeploymentBundle` interface declaration (around line 25-32)
- The deploy state declarations: `deployBundle`, `deployLoading`, `deployError`, `deployWriteResult`
- The `generateDeploy()` and `writeDeploy()` functions
- The Tabs array entry for "deploy"
- The `{activeTab === "deploy" && (...)}` block in the JSX

- [ ] **Step 2: Remove the `DeploymentBundle` interface**

Delete the entire interface:
```ts
interface DeploymentBundle {
  name: string;
  base_path: string;
  docker_compose_content: string;
  readme_content: string;
  ci_template_content: string;
}
```

- [ ] **Step 3: Remove deploy state declarations**

Delete the four lines:
```ts
const [deployBundle, setDeployBundle] = useState<DeploymentBundle | null>(null);
const [deployLoading, setDeployLoading] = useState(false);
const [deployError, setDeployError] = useState<string | null>(null);
const [deployWriteResult, setDeployWriteResult] = useState<string | null>(null);
```

- [ ] **Step 4: Remove `generateDeploy` and `writeDeploy` functions**

Delete both function bodies entirely.

- [ ] **Step 5: Remove the "deploy" entry from the Tabs array**

Find the tabs array and remove the entry:
```ts
{ id: "deploy", label: "Deploy" },
```

- [ ] **Step 6: Remove the deploy tab JSX block**

Find and delete:
```tsx
{activeTab === "deploy" && (
  // ... entire deploy tab content ...
)}
```

- [ ] **Step 7: Verify typecheck passes**

Run: `npm run typecheck`
Expected: success

- [ ] **Step 8: Verify E2E for Gen still works (no "Deploy" tab)**

Manual: open `/gen`, confirm only Mock API / Scaffold tabs remain.

- [ ] **Step 9: Commit**

```bash
git add src/components/gen/GenPage.tsx
git commit -m "refactor(gen): remove deploy tab — moved to dedicated /deploy page"
```

---

## Task 14: Add E2E test for the Deploy page

**Files:**
- Create: `e2e/deploy.spec.ts`

- [ ] **Step 1: Create the E2E test file**

```ts
import { test, expect } from "@playwright/test";

// Mirrors the pattern in e2e/all-pages.spec.ts:
// verifies page structure without requiring Tauri IPC mocks.

test.describe("Deploy Page", () => {
  test("page loads at /deploy", async ({ page }) => {
    await page.goto("/deploy");
    await expect(page.getByText("ProxyBot")).toBeVisible({ timeout: 5000 });
    const body = page.locator("body");
    await expect(body).toBeVisible();
  });

  test("shows Deploy panel title", async ({ page }) => {
    await page.goto("/deploy");
    await expect(page.locator(".panel-title", { hasText: "Deploy" })).toBeVisible();
  });

  test("has session ID input", async ({ page }) => {
    await page.goto("/deploy");
    const input = page.locator('input[placeholder="e.g. 2026-06-04-001"]');
    await expect(input).toBeVisible();
  });

  test("has project name input with default", async ({ page }) => {
    await page.goto("/deploy");
    const input = page.locator('input[placeholder="proxybot_deployment"]');
    await expect(input).toBeVisible();
  });

  test("has output path display", async ({ page }) => {
    await page.goto("/deploy");
    await expect(page.locator("code", { hasText: ".proxybot/deployments" })).toBeVisible();
  });

  test("has Initialize git repo checkbox checked by default", async ({ page }) => {
    await page.goto("/deploy");
    const checkbox = page.locator('input[type="checkbox"]');
    await expect(checkbox).toBeChecked();
  });

  test("Generate button is disabled when session ID is empty", async ({ page }) => {
    await page.goto("/deploy");
    const btn = page.getByRole("button", { hasText: "Generate Preview" });
    await expect(btn).toBeDisabled();
  });

  test("Generate button enables when session ID is filled", async ({ page }) => {
    await page.goto("/deploy");
    await page.fill('input[placeholder="e.g. 2026-06-04-001"]', "test-session");
    const btn = page.getByRole("button", { hasText: "Generate Preview" });
    await expect(btn).toBeEnabled();
  });

  test("shows empty state when no bundle generated", async ({ page }) => {
    await page.goto("/deploy");
    await expect(page.getByText("No preview yet")).toBeVisible();
  });

  test("Write to Disk button is disabled when no bundle", async ({ page }) => {
    await page.goto("/deploy");
    const btn = page.getByRole("button", { hasText: "Write to Disk" });
    await expect(btn).toBeDisabled();
  });

  test("Re-init Git button is disabled when no bundle path", async ({ page }) => {
    await page.goto("/deploy");
    const btn = page.getByRole("button", { hasText: "Re-init Git" });
    await expect(btn).toBeDisabled();
  });

  test("sidebar has Deploy entry", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("aside").getByText("Deploy")).toBeVisible();
  });

  test("clicking sidebar Deploy navigates to /deploy", async ({ page }) => {
    await page.goto("/");
    await page.click("aside a[href='/deploy']");
    await expect(page).toHaveURL("/deploy");
  });
});
```

- [ ] **Step 2: Update `e2e/all-pages.spec.ts` to drop the deploy tab assertion**

Open `e2e/all-pages.spec.ts` and find the Gen test:

```ts
test("has all generate tabs", async ({ page }) => {
  await page.goto("/gen");
  const tabs = ["Mock API", "Scaffold", "Deploy"];
  ...
```

Change to:
```ts
test("has all generate tabs", async ({ page }) => {
  await page.goto("/gen");
  const tabs = ["Mock API", "Scaffold"];
  ...
```

Also find the deploy-specific test (around line 176):
```ts
test("deploy tab shows Generate button", async ({ page }) => {
  await page.goto("/gen");
  await page.click("text=Deploy");
  await expect(page.getByText("Generate Deployment Bundle")).toBeVisible();
});
```

Delete this entire test (deploy tab no longer exists in Gen).

- [ ] **Step 3: Update `e2e/all-pages.spec.ts` to add /deploy to the navigation list**

Find the `pages` array (around line 8-20):
```ts
const pages = [
  { name: "Traffic", path: "/" },
  ...
  { name: "AI", path: "/ai" },
];
```

Add the Deploy entry:
```ts
{ name: "AI", path: "/ai" },
{ name: "Deploy", path: "/deploy" },
```

Also update the labels array in the `Sidebar Navigation` test:
```ts
const labels = ["Traffic", "Rules", "Certs", "Devices", "DNS", "Alerts", "Replay", "Graph", "Composer", "Gen", "AI", "Deploy"];
```

- [ ] **Step 4: Run the deploy E2E tests**

Run: `npm run test:e2e -- e2e/deploy.spec.ts`
Expected: all tests PASS (page structure renders even without Tauri IPC working, because `invoke` errors are caught and shown in the error banner)

- [ ] **Step 5: Run the full E2E suite to ensure no regressions**

Run: `npm run test:e2e`
Expected: all tests PASS

- [ ] **Step 6: Commit**

```bash
git add e2e/deploy.spec.ts e2e/all-pages.spec.ts
git commit -m "test: add E2E tests for Deploy page and update existing tests"
```

---

## Task 15: Final verification

**Files:** none (verification only)

- [ ] **Step 1: Type check the whole project**

Run: `npm run typecheck`
Expected: no errors

- [ ] **Step 2: Rust check**

Run: `cd src-tauri && cargo check`
Expected: no errors

- [ ] **Step 3: Run all Rust unit tests**

Run: `cd src-tauri && cargo test --lib`
Expected: all tests pass, including the new ones (`test_deployments_table_upsert_and_get`, `test_git_init_deployment_*`, `test_get_last_deployment_*`)

- [ ] **Step 4: Run all E2E tests**

Run: `npm run test:e2e`
Expected: all tests pass

- [ ] **Step 5: Manual browser smoke test**

Run: `npm run dev`

Then in the browser:
1. Navigate to `/deploy`
2. Enter a session ID (any string, e.g. "test-001") and project name "myproject"
3. Click Generate Preview — expect error banner (Tauri IPC not available in dev server) OR success if running in Tauri
4. Switch tabs — expect UI responsive
5. Click Write to Disk — expect error in dev (expected) or success in Tauri
6. Click sidebar Deploy — expect navigation works

- [ ] **Step 6: Update US-009 in `ralph/prd.json`**

Find the entry:
```json
{
  "id": "US-009",
  "title": "Redesign deploy panel",
  ...
  "passes": false,
  "notes": "Skipped - Deploy panel uses existing deploy module, not part of core UI redesign"
}
```

Change:
```json
{
  "id": "US-009",
  "title": "Redesign deploy panel",
  ...
  "passes": true,
  "notes": "Implemented in 2026-06-04 — dedicated /deploy page with preview, write, re-init git actions, SQLite persistence."
}
```

- [ ] **Step 7: Commit the PRD update**

```bash
git add ralph/prd.json
git commit -m "chore(prd): mark US-009 as passes (Deploy panel complete)"
```

---

## Self-Review Checklist

- [x] **Spec coverage:** Every section of `2026-06-04-deploy-panel-design.md` is covered by a task:
  - § 4 Components → Tasks 7-11 (5 components)
  - § 5.1 Schema + db fns → Task 1
  - § 5.2 init_git param → Task 2
  - § 5.2 git_init_deployment → Task 3
  - § 5.2 get_last_deployment → Task 4
  - § 5.3 lib.rs registration → Task 5
  - § 6 TS types → Task 6
  - § 7 wiring → Task 12
  - § 7 GenPage cleanup → Task 13
  - § 9 testing → Task 14
  - § 8 acceptance criteria → covered by all tasks collectively
  - § 9 typecheck/cargo check → Task 15

- [x] **No placeholders:** Every code step has complete, copy-pasteable code. No "TBD", no "similar to task N", no vague "add error handling".

- [x] **Type consistency:** Used same names throughout: `DeploymentBundle`, `DeploymentResult`, `DeploymentRecord`, `sessionId`, `projectName`, `bundlePath`, `initGit`, `handleGenerate`, `handleWrite`, `handleReinitGit`, `git_init_deployment_inner`, `get_last_deployment_inner`.

- [x] **No out-of-scope work:** No light/dark mode toggle, no custom output_dir, no docker compose up button, no git push.

- [x] **TDD where appropriate:** Backend has 5 unit tests (deployments table, git_init inner, get_last_deployment inner). Frontend covered by E2E + manual smoke (matches existing project pattern).

- [x] **Frequent commits:** 15 tasks → 15 commits, each with a conventional commit message.
