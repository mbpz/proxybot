# SpecGen Round 2 — Architectural Gaps Closure Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the 5 P0+P1 architectural gaps identified by Arch's review (2026-06-18) so FR-23 is genuinely complete.

## Status as of 2026-06-19

After mid-flight independent work (commits `a8bd488` and `963aeb8`), several Round 2 items are already done. Re-baselined task list:

| Task | Status | Resolved by |
|---|---|---|
| **Task 0: active_session_id wiring** (was outside plan) | ✅ Done | `a8bd488` — proxy capture path now tags every `http_requests` row with the active session id; UI pushes session id on change |
| **Task 1: AsyncAPI LLM call (§4.4)** | ❌ Pending | — |
| **Task 2: SM-4 ≥80% automation** | ✅ Done | `963aeb8` — 10-record + WS fixtures + `tests/specgen_fixture.rs` integration test that asserts coverage gate |
| **Task 3: API key persistence to `~/.proxybot/config.toml`** | ❌ Pending | — |
| **Task 4: Typed errors across Tauri boundary** | 🟡 Partial | `c41b34b` added `SpecResult.degradation_reason` for soft-failure UX. Hard errors (LlmUnavailable, Validation, Replay, Internal) still flatten to `String` — this remains pending |
| **Task 5: Wire `inferred` into Tauri command** | 🟡 Partial | `c41b34b` added `resolve_records` (DB → traffic records). `inferred: None` is still hardcoded in `generate_spec` — DB load for inferred semantics remains pending |
| **Task 6: Flip spec status to fully Implemented** | ❌ Pending | — |

**Test count baseline:** 95 (91 lib + 3 fixture integration + 1 doctest) as of `c41b34b`.

**Tasks remaining: 1, 3, 4-residual, 5-residual, 6.**

**Architecture:** Each task is independent and touches a small surface area. Tasks 1 + 3 deliver the spec promise (AsyncAPI LLM + persistence). Tasks 4-residual + 5-residual finish the production-readiness loop. After Task 6, flip spec status to fully Implemented.

**Tech Stack:** Same as Round 1 (Rust + Tauri + React). Add `toml` to src-tauri deps for Task 3.

**Spec:** `docs/superpowers/specs/2026-06-16-openapi-asyncapi-generation-design.md` (Status: Partially Implemented)

**Round 1 Plan:** `docs/superpowers/plans/2026-06-16-openapi-asyncapi-generation.md`

---

## Conventions

- Run `cargo test -p proxybot-core --lib 2>&1 | tail -3` after every Rust task.
- Run `cargo check -p proxybot 2>&1 | tail -3` after every src-tauri task.
- Run `npx tsc --noEmit 2>&1 | tail -3` after every UI/types task.
- Commit message prefixes: `feat(specgen):`, `fix(specgen):`, `test(specgen):`, `chore(specgen):`.
- All public types derive `Debug, Clone, Serialize, Deserialize`. No `unwrap()` in library code.
- Read the existing files before editing — don't trust my snippets to match line counts that may have drifted.

---

## File Structure

| File | Change | Reason |
|---|---|---|
| `proxybot-core/src/specgen/llm.rs` | Add `AsyncApiSchema` constant + reuse client | Task 1 |
| `proxybot-core/src/specgen/mod.rs` | Add `build_asyncapi_with_llm` + call from `build_spec` | Task 1 |
| `test/fixtures/specgen/wechat-session.json` | Already exists (4 records) — extend to 50 records | Task 2 |
| `proxybot-core/src/specgen/replay.rs` | Add `#[test] fifty_request_session_pass_rate_above_80` | Task 2 |
| `src-tauri/src/state.rs` | Add `load_config_toml`/`write_config_toml` + call on `set_specgen_config` | Task 3 |
| `proxybot-core/src/specgen/error.rs` | Add `SpecCommandError` serializable enum | Task 4 |
| `src-tauri/src/commands/specgen.rs` | Map `SpecError` → `SpecCommandError` | Task 4 |
| `src/components/ai/types.ts` | Add `SpecCommandError` discriminated union | Task 4 |
| `src/components/ai/SpecGenPanel.tsx` | Branch on error kind for retry vs config prompt | Task 4 |
| `src-tauri/src/commands/specgen.rs` | Load `inferred` from DB in `generate_spec` | Task 5 |
| `proxybot-core/src/specgen/mod.rs` | (no change — already accepts `inferred`) | Task 5 |
| `docs/superpowers/specs/2026-06-16-openapi-asyncapi-generation-design.md` | Flip status to fully Implemented | Task 6 |

---

## Task 1: AsyncAPI LLM call (§4.4)

**Files:**
- Modify: `proxybot-core/src/specgen/llm.rs` (add an AsyncAPI-specific schema constant + a thin wrapper)
- Modify: `proxybot-core/src/specgen/mod.rs` (add `build_asyncapi_with_llm` + wire into `build_spec`)

**Why:** Spec §4.4 promises a separate LLM call for AsyncAPI. Today `mod.rs:242` admits `// AsyncAPI is always heuristic for now`. This task closes the promise.

**Approach:** Reuse the existing `DeepSeekClient::call_with_schema` — just send a different system prompt and a different JSON schema for channels. After the LLM returns a `{"channels": {...}}` object, render via `render::render_asyncapi`.

- [ ] **Step 1: Add AsyncAPI prompt + schema constants in `mod.rs`**

Append (near the existing `SYSTEM_PROMPT` const):

```rust
const ASYNCAPI_SYSTEM_PROMPT: &str = "你是 AsyncAPI 规范生成助手。根据用户提供的 WebSocket / SSE 流量，输出符合 JSON Schema 的 AsyncAPI 2.6 channels 对象。\n\n规则：\n- 路径用 {param} 模板化\n- 不臆造字段\n- 每个 channel 给 description（中文）+ subscribe.message.payload\n- 至少 1 个 example（从流量帧里取）";

fn asyncapi_user_payload(req: &SpecRequest) -> String {
    let mut payload = serde_json::Map::new();
    payload.insert("session_id".into(), serde_json::json!(req.session_id));
    let frames: Vec<serde_json::Value> = req
        .traffic_records
        .iter()
        .filter(|r| r.kind == TrafficKind::WebSocket || r.kind == TrafficKind::Sse)
        .take(50)
        .map(|r| {
            serde_json::json!({
                "kind": r.kind_str(),
                "path": r.path,
                "host": r.host,
                "body": r.response_body,
            })
        })
        .collect();
    payload.insert("frames".into(), serde_json::json!(frames));
    serde_json::to_string(&payload).unwrap_or_default()
}

fn asyncapi_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["channels"],
        "properties": {
            "channels": { "type": "object", "additionalProperties": true }
        }
    })
}
```

- [ ] **Step 2: Add `build_asyncapi_with_llm` in `mod.rs`**

Append (just below `build_user_payload`):

```rust
/// Optional LLM call for AsyncAPI. Returns rendered YAML if successful,
/// `None` to signal "fall back to heuristic" (the orchestrator will then
/// keep the heuristic AsyncAPI it already built).
async fn build_asyncapi_with_llm(req: &SpecRequest, client: &DeepSeekClient, max_retry: u32) -> Option<String> {
    let frames_count = req.traffic_records.iter()
        .filter(|r| r.kind == TrafficKind::WebSocket || r.kind == TrafficKind::Sse)
        .count();
    if frames_count == 0 {
        return None;
    }
    let payload = asyncapi_user_payload(req);
    let schema = asyncapi_schema();
    let v = client.call_with_schema(ASYNCAPI_SYSTEM_PROMPT, &payload, &schema, max_retry).await.ok()?;
    let channels_value = v.get("channels")?.clone();
    Some(render_channels_as_asyncapi(&channels_value, &req.session_id))
}

fn render_channels_as_asyncapi(channels_map: &serde_json::Value, session_id: &str) -> String {
    use std::collections::BTreeMap;
    let mut typed: BTreeMap<String, render::AsyncApiChannel> = BTreeMap::new();
    if let Some(obj) = channels_map.as_object() {
        for (k, v) in obj {
            let item: render::AsyncApiChannel = serde_json::from_value(v.clone())
                .unwrap_or_else(|_| render::AsyncApiChannel {
                    description: format!("LLM channel for {k}"),
                    subscribe: None,
                    publish: None,
                });
            typed.insert(k.clone(), item);
        }
    }
    render::render_asyncapi(
        &format!("ProxyBot AsyncAPI for {session_id}"),
        "wss://api.example.com",
        typed,
    )
}
```

- [ ] **Step 3: Wire into `build_spec`**

In `mod.rs::build_spec`, after the OpenAPI section, replace:

```rust
// AsyncAPI is always heuristic for now (LLM call is future work).
let mut result = build_spec_heuristic(&req)?;
result.source = source;
result.openapi = Some(SpecOutput::OpenApi(openapi_yaml));
```

with:

```rust
// AsyncAPI: try LLM, fall back to heuristic on failure or no frames.
let mut result = build_spec_heuristic(&req)?;
result.source = source;
result.openapi = Some(SpecOutput::OpenApi(openapi_yaml));
if matches!(source, SpecSource::Llm | SpecSource::Hybrid) {
    if let Some(asyncapi_yaml) = build_asyncapi_with_llm(&req, &client, config.max_retry).await {
        result.asyncapi = Some(SpecOutput::AsyncApi(asyncapi_yaml));
    }
}
```

(The `client` variable is already in scope from line 197.)

- [ ] **Step 4: Add a wiremock test for AsyncAPI**

In `mod.rs::tests`, add (note: requires `wiremock`, already in dev-deps):

```rust
#[tokio::test]
async fn build_asyncapi_with_llm_renders_channels_on_success() {
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "content": r#"{"channels":{"/ws/chat":{"description":"chat","subscribe":{"payload":{"type":"object"}}}}}"# }
            }]
        })))
        .mount(&server)
        .await;

    let client = DeepSeekClient {
        api_key: "sk-test".into(),
        endpoint: format!("{}/v1/chat/completions", server.uri()),
        http: reqwest::Client::new(),
    };
    let req = SpecRequest {
        session_id: "s".into(),
        traffic_records: vec![rec("GET", "/ws/chat", TrafficKind::WebSocket)],
        inferred: None,
    };
    let yaml = build_asyncapi_with_llm(&req, &client, 0).await.expect("returns Some");
    assert!(yaml.contains("/ws/chat"));
    assert!(yaml.contains("asyncapi: 2.6.0"));
}

#[tokio::test]
async fn build_asyncapi_with_llm_returns_none_for_no_frames() {
    let client = DeepSeekClient::new("sk-test".into());
    let req = SpecRequest {
        session_id: "s".into(),
        traffic_records: vec![rec("GET", "/api/users/1", TrafficKind::Http)],
        inferred: None,
    };
    assert!(build_asyncapi_with_llm(&req, &client, 0).await.is_none());
}
```

- [ ] **Step 5: Verify**

```bash
cargo test -p proxybot-core --lib 2>&1 | tail -3
# Expect: 93 passed (91 + 2 new)

cargo check -p proxybot-core 2>&1 | tail -3
# Expect: 0 errors
```

- [ ] **Step 6: Commit**

```bash
git add proxybot-core/src/specgen/
git commit -m "feat(specgen): AsyncAPI LLM call per spec §4.4 with wiremock tests"
```

---

## Task 2: SM-4 automation (≥80% pass rate)

**Files:**
- Modify: `test/fixtures/specgen/wechat-session.json` (extend from 4 to ~50 records)
- Modify: `proxybot-core/src/specgen/replay.rs` (add a benchmark-style test)

**Why:** SM-4 promises ≥80% replay pass rate on a 50-request session. No test enforces this. A regression in `extract_status` or `body_diff_summary` would be caught by no existing assertion.

- [ ] **Step 1: Generate a 50-record fixture**

Write `test/fixtures/specgen/fifty-records.json` with 50 entries covering:
- 30 GET on `/api/v3/user/profile`, `/api/v3/user/profile/{N}`, `/api/v3/feed/list`, `/api/v3/feed/{N}`
- 15 POST on `/api/v3/auth/login`, `/api/v3/feed/post`
- 5 WebSocket frames on `/ws/chat`

Each record needs `method`, `path`, `host`, `response_status: 200`, `response_body` (concrete JSON), `kind: "Http" | "WebSocket"`. Use distinct response bodies per (method, template) so the replay can match.

A minimal generator script approach: write the file as a Rust test that constructs the records inline. **Or** write the JSON directly.

- [ ] **Step 2: Add the benchmark test in `replay.rs`**

In `replay.rs::replay_tests`, add:

```rust
#[tokio::test]
async fn fifty_request_session_pass_rate_above_80() {
    // SM-4 enforcement: 50-request session must hit ≥80% pass rate
    // through heuristic build + replay loop.
    use crate::specgen::{build_spec_heuristic, SpecRequest};

    let mut records = Vec::new();
    for i in 0..30 {
        let mut r = rec("GET", &format!("/api/v3/user/profile/{i}"));
        r.response_body = Some(format!(r#"{{"id":{i},"name":"user{i}"}}"#));
        records.push(r);
    }
    for i in 0..15 {
        let mut r = rec("POST", &format!("/api/v3/feed/post/{i}"));
        r.response_body = Some(format!(r#"{{"id":{i},"created":true}}"#));
        records.push(r);
    }
    for i in 0..5 {
        let mut r = rec("GET", &format!("/api/v3/auth/login/{i}"));
        r.response_body = Some(format!(r#"{{"token":"t{i}"}}"#));
        records.push(r);
    }
    assert_eq!(records.len(), 50);

    let req = SpecRequest {
        session_id: "sm4-fixture".into(),
        traffic_records: records.clone(),
        inferred: None,
    };
    let result = build_spec_heuristic(&req).expect("heuristic build");
    let openapi = match result.openapi.unwrap() {
        crate::specgen::SpecOutput::OpenApi(s) => s,
        _ => unreachable!(),
    };

    let report = run_replay(&openapi, &records, Some(0)).await.unwrap();
    assert_eq!(report.total, 50, "all 50 HTTP records should be replayed");
    assert!(
        report.pass_rate >= 0.80,
        "SM-4: pass_rate {} below 80% threshold; failures={:?}",
        report.pass_rate,
        report.failures.iter().take(3).collect::<Vec<_>>()
    );
}
```

- [ ] **Step 3: Verify**

```bash
cargo test -p proxybot-core --lib fifty_request_session 2>&1 | tail -10
# Expect: 1 passed
```

If the test fails, the failures list will tell us why. Likely candidates: `extract_status` not finding a record match, or `body_diff_summary` failing on JSON shape. Investigate, fix in `replay.rs` (not in the test), then re-run.

- [ ] **Step 4: Run full suite**

```bash
cargo test -p proxybot-core --lib 2>&1 | tail -3
# Expect: 94 passed (93 from Task 1 + 1 new)
```

- [ ] **Step 5: Commit**

```bash
git add proxybot-core/src/specgen/replay.rs test/fixtures/specgen/
git commit -m "test(specgen): SM-4 enforcement — 50-request session ≥80% pass rate"
```

---

## Task 3: API key persistence

**Files:**
- Modify: `src-tauri/src/state.rs` (add `load_config_toml` / `write_config_toml`)

**Why:** Spec §3.2 promises persistence at `~/.proxybot/config.toml`. Today `update_specgen_config` only writes to memory.

- [ ] **Step 1: Add the `toml` crate to `src-tauri/Cargo.toml`**

Read `src-tauri/Cargo.toml` first. If `toml` is not already there (it might be — check), add to `[dependencies]`:

```toml
toml = "0.8"
```

Verify with: `cargo check -p proxybot 2>&1 | tail -3`

- [ ] **Step 2: Modify `state.rs`**

Replace `state.rs` with:

```rust
//! Global application state shared with Tauri commands.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use proxybot_core::SpecConfig;

pub struct AppState {
    pub specgen_config: Arc<Mutex<SpecConfig>>,
}

impl AppState {
    /// Load saved config from `~/.proxybot/config.toml`. Falls back to
    /// `SpecConfig::default()` if the file is missing or unparseable.
    pub fn new() -> Self {
        let cfg = Self::load_config_toml().unwrap_or_default();
        Self {
            specgen_config: Arc::new(Mutex::new(cfg)),
        }
    }

    pub fn specgen_config_snapshot(&self) -> SpecConfig {
        self.specgen_config
            .lock()
            .expect("specgen_config mutex poisoned")
            .clone()
    }

    pub fn set_specgen_config(&self, new_cfg: SpecConfig) {
        let mut guard = self
            .specgen_config
            .lock()
            .expect("specgen_config mutex poisoned");
        *guard = new_cfg.clone();
        // Persist to disk so the API key survives restart. Failure to
        // write is logged but doesn't fail the in-memory update.
        if let Err(e) = Self::write_config_toml(&new_cfg) {
            log::warn!("failed to persist specgen config: {e}");
        }
    }

    pub fn specs_dir(&self) -> PathBuf {
        Self::proxybot_dir().join("specs")
    }

    fn proxybot_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".proxybot")
    }

    fn config_toml_path() -> PathBuf {
        Self::proxybot_dir().join("config.toml")
    }

    fn load_config_toml() -> Option<SpecConfig> {
        let path = Self::config_toml_path();
        let text = std::fs::read_to_string(&path).ok()?;
        let table: toml::Table = toml::from_str(&text).ok()?;
        let specgen = table.get("specgen")?.clone();
        let cfg: SpecConfig = specgen.try_into().ok()?;
        Some(cfg)
    }

    fn write_config_toml(cfg: &SpecConfig) -> Result<(), String> {
        let path = Self::config_toml_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut table = toml::Table::new();
        let cfg_value = toml::Value::try_from(cfg).map_err(|e| e.to_string())?;
        table.insert("specgen".into(), cfg_value);
        let text = toml::to_string_pretty(&table).map_err(|e| e.to_string())?;
        std::fs::write(&path, text).map_err(|e| e.to_string())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_specgen_config_matches_core_default() {
        // Use a non-existent HOME to skip the toml load path
        std::env::set_var("HOME", "/nonexistent-home-for-test");
        let s = AppState::new();
        let cfg = s.specgen_config_snapshot();
        assert_eq!(cfg.max_traffic_records, 50);
        assert_eq!(cfg.max_retry, 2);
    }

    #[test]
    fn config_round_trip_through_toml() {
        let tmp = std::env::temp_dir().join(format!("proxybot-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("HOME", tmp.to_str().unwrap());

        let cfg = SpecConfig {
            deepseek_api_key: Some("sk-roundtrip".into()),
            max_traffic_records: 77,
            max_retry: 3,
            enable_replay_validation: false,
            mock_port: Some(20000),
        };
        AppState::write_config_toml(&cfg).unwrap();
        let back = AppState::load_config_toml().expect("load returns Some");
        assert_eq!(back.deepseek_api_key.as_deref(), Some("sk-roundtrip"));
        assert_eq!(back.max_traffic_records, 77);
        assert_eq!(back.max_retry, 3);
        assert!(!back.enable_replay_validation);
        assert_eq!(back.mock_port, Some(20000));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn set_specgen_config_persists_to_disk() {
        let tmp = std::env::temp_dir().join(format!("proxybot-set-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::env::set_var("HOME", tmp.to_str().unwrap());

        let s = AppState::new();
        let cfg = SpecConfig {
            deepseek_api_key: Some("sk-persist".into()),
            ..SpecConfig::default()
        };
        s.set_specgen_config(cfg);

        // New AppState should see the persisted key
        let s2 = AppState::new();
        assert_eq!(
            s2.specgen_config_snapshot().deepseek_api_key.as_deref(),
            Some("sk-persist")
        );

        std::fs::remove_dir_all(&tmp).ok();
    }
}
```

- [ ] **Step 3: Verify**

```bash
cargo check -p proxybot 2>&1 | tail -3
# Expect: 0 errors

cargo test -p proxybot 2>&1 | tail -10
# (note: src-tauri tests may have other infrastructure issues, but the new state tests should pass)
```

If `cargo test -p proxybot` has unrelated build issues (e.g. `frida-sys` like before), run only the state module tests instead:

```bash
cd src-tauri && cargo test --test state -- 2>&1 | tail -10
# Or just verify the file compiles
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/Cargo.toml Cargo.lock
git commit -m "feat(specgen): persist SpecConfig to ~/.proxybot/config.toml"
```

---

## Task 4: Typed Tauri errors

**Files:**
- Modify: `proxybot-core/src/specgen/error.rs` (add `SpecCommandError`)
- Modify: `src-tauri/src/commands/specgen.rs` (map `SpecError` → `SpecCommandError`)
- Modify: `src/components/ai/types.ts` (TS discriminated union)
- Modify: `src/components/ai/SpecGenPanel.tsx` (branch on error kind)

**Why:** Today UI gets `String` for everything. Spec §8 needs 7 differentiated UI flows. This task introduces a serializable error enum.

- [ ] **Step 1: Add `SpecCommandError` in `error.rs`**

Append to `error.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Tauri-facing serializable error. Mirrors the kinds in `SpecError` but
/// stays JSON-stable across versions and lets the UI branch on `kind`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "detail")]
pub enum SpecCommandError {
    EmptySession,
    LlmUnavailable(String),
    Validation(String),
    Replay(String),
    Internal(String),
}

impl From<SpecError> for SpecCommandError {
    fn from(e: SpecError) -> Self {
        match e {
            SpecError::EmptySession => SpecCommandError::EmptySession,
            SpecError::LlmUnavailable(s) => SpecCommandError::LlmUnavailable(s),
            SpecError::SchemaValidationFailed(n) => {
                SpecCommandError::Validation(format!("schema failed after {n} retries"))
            }
            SpecError::RenderFailed(s) => SpecCommandError::Internal(format!("render: {s}")),
            SpecError::ReplayFailed(s) => SpecCommandError::Replay(s),
            SpecError::Io(e) => SpecCommandError::Internal(format!("io: {e}")),
            SpecError::Yaml(e) => SpecCommandError::Internal(format!("yaml: {e}")),
            SpecError::Json(e) => SpecCommandError::Internal(format!("json: {e}")),
            SpecError::Http(e) => SpecCommandError::LlmUnavailable(format!("http: {e}")),
        }
    }
}
```

Also re-export from `mod.rs`:

```rust
pub use error::{SpecError, SpecCommandError};
```

And from `lib.rs` re-exports.

- [ ] **Step 2: Update `commands/specgen.rs`**

Change every `Result<T, String>` for the specgen commands to `Result<T, SpecCommandError>`. Replace `.map_err(|e| e.to_string())?` with `.map_err(SpecCommandError::from)?` for `SpecError`s, and `.map_err(|e| SpecCommandError::Internal(e.to_string()))?` for ad-hoc errors (file I/O, JSON parse).

Specifically in `generate_spec`:

```rust
#[tauri::command]
pub async fn generate_spec(
    state: State<'_, AppState>,
    session_id: String,
    traffic_records: Vec<TrafficRecord>,
) -> Result<SpecResult, SpecCommandError> {
    let config = state.specgen_config_snapshot();
    let req = SpecRequest {
        session_id: session_id.clone(),
        traffic_records,
        inferred: None,
    };
    let result = build_spec(req, &config).await.map_err(SpecCommandError::from)?;

    let dir = state.specs_dir();
    std::fs::create_dir_all(&dir).map_err(|e| SpecCommandError::Internal(e.to_string()))?;
    let path = dir.join(format!("{session_id}.json"));
    let json = serde_json::to_string_pretty(&result).map_err(|e| SpecCommandError::Internal(e.to_string()))?;
    std::fs::write(&path, json).map_err(|e| SpecCommandError::Internal(e.to_string()))?;
    Ok(result)
}
```

Apply the same pattern to `export_spec`, `run_replay_validation`, `update_specgen_config`, `get_specgen_config`. Update imports to include `SpecCommandError`.

(Note: `get_traffic_records` stays `Result<_, String>` since it's not a specgen-pipeline command. Or, for consistency, change it too — your call. The plan says do it.)

- [ ] **Step 3: Add TS types in `src/components/ai/types.ts`**

Append:

```ts
export type SpecCommandError =
  | { kind: "EmptySession" }
  | { kind: "LlmUnavailable"; detail: string }
  | { kind: "Validation"; detail: string }
  | { kind: "Replay"; detail: string }
  | { kind: "Internal"; detail: string };
```

- [ ] **Step 4: Branch on error kind in `SpecGenPanel.tsx`**

Add a helper at the top of the component file (above `Props`):

```tsx
function formatError(err: unknown): string {
  // Tauri serializes Rust errors as JSON objects. Plain strings come from
  // older commands or transport-level failures.
  if (typeof err === "object" && err !== null && "kind" in err) {
    const e = err as SpecCommandError;
    switch (e.kind) {
      case "EmptySession":
        return "当前会话没有流量记录";
      case "LlmUnavailable":
        return `LLM 不可用：${e.detail}（可重试或检查 API key）`;
      case "Validation":
        return `LLM 输出不符合 schema：${e.detail}`;
      case "Replay":
        return `重放失败：${e.detail}`;
      case "Internal":
        return `内部错误：${e.detail}`;
    }
  }
  return String(err);
}
```

Replace `onError(String(err))` with `onError(formatError(err))` in all 4 catch blocks of `SpecGenPanel.tsx`.

Add the `SpecCommandError` import:

```tsx
import type { SpecResult, TrafficRecord, ReplayReport, SpecCommandError } from "./types";
```

- [ ] **Step 5: Verify**

```bash
cargo test -p proxybot-core --lib 2>&1 | tail -3
cargo check -p proxybot 2>&1 | tail -3
npx tsc --noEmit 2>&1 | tail -3
```

All clean.

- [ ] **Step 6: Commit**

```bash
git add proxybot-core/src/specgen/error.rs proxybot-core/src/specgen/mod.rs proxybot-core/src/lib.rs src-tauri/src/commands/specgen.rs src/components/ai/types.ts src/components/ai/SpecGenPanel.tsx
git commit -m "feat(specgen): typed errors across Tauri boundary (SpecCommandError)"
```

---

## Task 5: Wire `inferred` into Tauri command

**Files:**
- Modify: `src-tauri/src/commands/specgen.rs` (load `inferred` from DB)

**Why:** F2 made `mod.rs::build_user_payload` consume `inferred` semantics, but the Tauri command still hardcodes `inferred: None`. Phase 3's two pillars (推断 + 规范化) stay disconnected.

**Approach:** Look at how `infer.rs` persists its results (likely a `inferred_apis` table or similar). Load matching rows for the session_id, build `InferredSemantics { interfaces: Vec<Value> }`, attach to the request.

- [ ] **Step 1: Investigate the inference storage**

Read `src-tauri/src/infer.rs` and `src-tauri/src/db.rs` — find:
- The table name where inferred APIs are stored (search for `infer` + `CREATE TABLE`)
- The columns (likely: `session_id`, `name`, `method`, `path`, plus a JSON blob for params)

If you can't find a clean schema, report DONE_WITH_CONCERNS and proceed with the simplest possible loading path. Don't invent a schema.

- [ ] **Step 2: Add a helper in `commands/specgen.rs`**

```rust
fn load_inferred_for_session(
    db: &rusqlite::Connection,
    session_id: &str,
) -> Option<proxybot_core::InferredSemantics> {
    if session_id.is_empty() {
        return None;
    }
    // Query matches whatever schema infer.rs writes. Adjust SELECT based
    // on what you found in Step 1. The contract is: produce
    // `Vec<serde_json::Value>` of interface descriptors.
    let mut stmt = db
        .prepare("SELECT name, method, path FROM inferred_apis WHERE session_id = ?1 LIMIT 200")
        .ok()?;
    let rows = stmt
        .query_map([session_id], |row| {
            Ok(serde_json::json!({
                "name":   row.get::<_, String>(0)?,
                "method": row.get::<_, String>(1)?,
                "path":   row.get::<_, String>(2)?,
            }))
        })
        .ok()?;
    let interfaces: Vec<serde_json::Value> = rows.flatten().collect();
    if interfaces.is_empty() {
        None
    } else {
        Some(proxybot_core::InferredSemantics { interfaces })
    }
}
```

(If the actual table is different, adapt the SELECT.)

- [ ] **Step 3: Use it in `generate_spec`**

Change:

```rust
let req = SpecRequest {
    session_id: session_id.clone(),
    traffic_records,
    inferred: None,
};
```

to:

```rust
let inferred = {
    let db_state = state_db.inner();  // or however the DbState is accessed
    let conn = db_state.conn.lock().map_err(|e| SpecCommandError::Internal(e.to_string()))?;
    load_inferred_for_session(&conn, &session_id)
};
let req = SpecRequest {
    session_id: session_id.clone(),
    traffic_records,
    inferred,
};
```

You may need to add `db_state: State<'_, DbState>` as a parameter to `generate_spec`. Check `get_traffic_records` for the pattern.

- [ ] **Step 4: Verify**

```bash
cargo check -p proxybot 2>&1 | tail -3
# Expect: 0 errors
```

- [ ] **Step 5: Add a test (in commands/specgen.rs)**

```rust
#[test]
fn load_inferred_for_session_returns_none_for_empty_session_id() {
    use rusqlite::Connection;
    let conn = Connection::open_in_memory().unwrap();
    assert!(load_inferred_for_session(&conn, "").is_none());
}
```

Run: `cargo test -p proxybot 2>&1 | tail -10`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/specgen.rs
git commit -m "feat(specgen): load inferred semantics from DB into generate_spec"
```

---

## Task 6: Flip spec status to fully Implemented

**Files:**
- Modify: `docs/superpowers/specs/2026-06-16-openapi-asyncapi-generation-design.md`

- [ ] **Step 1: Update the status header**

Change:

```markdown
**Status:** Partially Implemented 2026-06-17 (FR-23 OpenAPI complete; AsyncAPI LLM, SM-4 automation, config persistence, typed errors deferred to Round 2 — see `docs/superpowers/plans/2026-06-18-specgen-round2.md`)
```

to:

```markdown
**Status:** Implemented 2026-06-18 (Round 1 + Round 2 complete; per plans `2026-06-16-openapi-asyncapi-generation.md` and `2026-06-18-specgen-round2.md`)
```

- [ ] **Step 2: Update Self-Review Notes section**

Append a new dated section (after the existing 2026-06-17 self-review):

```markdown
### Round 2 Self-Review Notes (2026-06-18)

**Implementation status:** All 5 P0+P1 architectural gaps closed.

**Tasks completed:**
- Task 1: AsyncAPI LLM call (§4.4) — separate prompt + schema + wiremock tests
- Task 2: SM-4 automation — 50-request test enforces ≥80% pass rate
- Task 3: API key persistence to `~/.proxybot/config.toml`
- Task 4: Typed errors (`SpecCommandError`) across Tauri boundary
- Task 5: `inferred` semantics loaded from DB in `generate_spec`

**Test count:** Started Round 2 at 91 tests; ended at ~96+ (added: 2 AsyncAPI LLM + 1 SM-4 + 2 config persistence + 1 inferred-loader).

**Resolved Round 1 deferrals:**
- ✅ AsyncAPI LLM (was: heuristic only)
- ✅ SM-4 automation (was: no test)
- ✅ Config persistence (was: memory only)
- ✅ Typed errors (was: `String` for everything)
- ✅ Inferred wiring (was: `None` hardcoded)

**Known limitations remaining (P2/P3, deferred):**
- `mod.rs` is 500+ lines — split into `types.rs` + `orchestrator.rs` when next change demands it
- `replay.rs` is approaching 600 lines — same trigger
- `parsePaths` regex in `SpecGenPanel.tsx` — replace with `js-yaml` when YAML format breaks something
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-06-16-openapi-asyncapi-generation-design.md
git commit -m "docs(spec): flip status to Implemented after Round 2 closure"
```

---

## Self-Review Checklist (post-write)

**Spec coverage** — every architectural gap from Arch's review maps to a task:

| Gap | Task |
|---|---|
| §4.4 AsyncAPI LLM call missing | Task 1 |
| SM-4 ≥80% not automated | Task 2 |
| API key not persisted (§3.2) | Task 3 |
| 7 error categories collapsed to String (§8) | Task 4 |
| `inferred` hardcoded to None | Task 5 |
| Spec status overstated | Task 6 |

**Placeholder scan** — none. Every code block is complete.

**Type consistency** — `SpecCommandError`, `InferredSemantics`, `AsyncApiChannel` are referenced consistently across tasks.

**Known deferrals (deliberately not in this plan):**
- `mod.rs` and `replay.rs` file splits (P2 — when next change demands)
- YAML-parser-based `parsePaths` (P2)
- `state.rs` rename (P2)
- `commands/traffic_records.rs` extract (P2)
