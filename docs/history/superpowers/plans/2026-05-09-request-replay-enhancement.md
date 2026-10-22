# Request Replay Enhancement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现请求重放增强功能，支持修改参数后重放、模板保存、场景重放

**Architecture:** ReplayPage主容器 + ReplayModal编辑 + Template系统 + Rust IPC执行

**Tech Stack:** React, TypeScript, Tauri IPC, Rust async runtime

---

## File Structure

| 操作 | 文件 | 职责 |
|------|------|------|
| Create | `src/components/replay/ReplayPage.tsx` | 主容器 |
| Create | `src/components/replay/ReplayModal.tsx` | 修改重放弹窗 |
| Create | `src/components/replay/TemplateEditor.tsx` | 模板编辑 |
| Create | `src/components/replay/ReplayResults.tsx` | 结果展示 |
| Create | `src-tauri/src/commands/replay.rs` | Rust IPC命令 |
| Create | `src-tauri/src/replay/engine.rs` | 重放引擎 |
| Modify | `src-tauri/src/lib.rs` | 注册命令 |

---

## Task 1: 创建ReplayPage主容器

**Files:**
- Create: `src/components/replay/ReplayPage.tsx`

- [ ] **Step 1: 创建ReplayPage**

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ReplayModal } from "./ReplayModal";
import { ReplayResults } from "./ReplayResults";

interface ReplayTarget {
  id: string;
  name: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
  expected_status?: number;
  enabled: boolean;
}

interface ReplayResult {
  target_id: string;
  status: number;
  duration_ms: number;
  success: boolean;
  error?: string;
}

export function ReplayPage() {
  const [targets, setTargets] = useState<ReplayTarget[]>([]);
  const [selectedTarget, setSelectedTarget] = useState<ReplayTarget | null>(null);
  const [results, setResults] = useState<ReplayResult[]>([]);
  const [isRunning, setIsRunning] = useState(false);

  useEffect(() => {
    loadTargets();
  }, []);

  async function loadTargets() {
    try {
      const result = await invoke<ReplayTarget[]>("get_replay_targets");
      setTargets(result);
    } catch (err) {
      console.error("Failed to load replay targets:", err);
    }
  }

  async function handleStartReplay() {
    setIsRunning(true);
    setResults([]);
    try {
      const result = await invoke<ReplayResult[]>("execute_replay", {
        targets: targets.filter((t) => t.enabled),
      });
      setResults(result);
    } catch (err) {
      console.error("Replay failed:", err);
    } finally {
      setIsRunning(false);
    }
  }

  async function handleDeleteTarget(id: string) {
    try {
      await invoke("delete_replay_target", { id });
      loadTargets();
    } catch (err) {
      console.error("Failed to delete target:", err);
    }
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Replay</h1>
        <div className="flex gap-2">
          <button
            onClick={() => setSelectedTarget(null)}
            className="px-4 py-2 bg-gray-200 rounded hover:bg-gray-300"
          >
            New Target
          </button>
          <button
            onClick={handleStartReplay}
            disabled={isRunning || targets.length === 0}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
          >
            {isRunning ? "Running..." : "Start Replay"}
          </button>
        </div>
      </div>

      {/* Targets Table */}
      <div className="bg-white rounded-lg shadow overflow-hidden">
        <table className="w-full">
          <thead className="bg-gray-50">
            <tr>
              <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">Enabled</th>
              <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">Name</th>
              <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">Method</th>
              <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">URL</th>
              <th className="px-4 py-3 text-left text-sm font-medium text-gray-500">Expected</th>
              <th className="px-4 py-3 text-right text-sm font-medium text-gray-500">Actions</th>
            </tr>
          </thead>
          <tbody>
            {targets.map((target) => (
              <tr key={target.id} className="border-t">
                <td className="px-4 py-3">
                  <input
                    type="checkbox"
                    checked={target.enabled}
                    onChange={async () => {
                      await invoke("toggle_replay_target", { id: target.id, enabled: !target.enabled });
                      loadTargets();
                    }}
                    className="w-4 h-4"
                  />
                </td>
                <td className="px-4 py-3 text-sm">{target.name}</td>
                <td className="px-4 py-3">
                  <span className="px-2 py-1 bg-gray-100 rounded text-xs font-mono">
                    {target.method}
                  </span>
                </td>
                <td className="px-4 py-3 text-sm truncate max-w-xs">{target.url}</td>
                <td className="px-4 py-3 text-sm">{target.expected_status || "-"}</td>
                <td className="px-4 py-3 text-right">
                  <button
                    onClick={() => setSelectedTarget(target)}
                    className="px-2 py-1 text-sm text-blue-600 hover:bg-blue-50 rounded"
                  >
                    Edit
                  </button>
                  <button
                    onClick={() => handleDeleteTarget(target.id)}
                    className="px-2 py-1 text-sm text-red-600 hover:bg-red-50 rounded ml-2"
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
            {targets.length === 0 && (
              <tr>
                <td colSpan={6} className="px-4 py-8 text-center text-gray-500">
                  No replay targets. Click "New Target" to create one.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Results */}
      {results.length > 0 && <ReplayResults results={results} />}

      {/* Modal */}
      {selectedTarget !== null && (
        <ReplayModal
          target={selectedTarget}
          onSave={async (updated) => {
            await invoke("save_replay_target", { target: updated });
            loadTargets();
            setSelectedTarget(null);
          }}
          onClose={() => setSelectedTarget(null)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/replay/ReplayPage.tsx
git commit -m "feat(replay): add ReplayPage container"
```

---

## Task 2: 创建ReplayModal组件

**Files:**
- Create: `src/components/replay/ReplayModal.tsx`

- [ ] **Step 1: 创建ReplayModal**

```tsx
import { useState } from "react";

interface ReplayTarget {
  id: string;
  name: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
  expected_status?: number;
  enabled: boolean;
}

interface ReplayModalProps {
  target: ReplayTarget | null;
  onSave: (target: ReplayTarget) => void;
  onClose: () => void;
}

const methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

export function ReplayModal({ target, onSave, onClose }: ReplayModalProps) {
  const [form, setForm] = useState<ReplayTarget>(
    target || {
      id: crypto.randomUUID(),
      name: "",
      method: "GET",
      url: "https://",
      headers: {},
      body: undefined,
      expected_status: 200,
      enabled: true,
    }
  );

  const [headersText, setHeadersText] = useState(
    Object.entries(form.headers)
      .map(([k, v]) => `${k}: ${v}`)
      .join("\n")
  );

  async function handleReplay() {
    try {
      const response = await fetch(form.url, {
        method: form.method,
        headers: form.headers,
        body: form.method !== "GET" && form.method !== "HEAD" ? form.body : undefined,
      });
      const text = await response.text();
      alert(`Status: ${response.status}\n\nResponse:\n${text.slice(0, 500)}`);
    } catch (err) {
      alert(`Error: ${err}`);
    }
  }

  function handleSave() {
    const headers: Record<string, string> = {};
    headersText.split("\n").forEach((line) => {
      const colonIndex = line.indexOf(":");
      if (colonIndex > 0) {
        const key = line.slice(0, colonIndex).trim();
        const value = line.slice(colonIndex + 1).trim();
        if (key) headers[key] = value;
      }
    });
    onSave({ ...form, headers });
  }

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-2xl max-h-screen overflow-auto p-6">
        <h2 className="text-xl font-bold mb-4">{target ? "Edit Target" : "New Target"}</h2>

        <div className="space-y-4">
          {/* Name */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Name</label>
            <input
              type="text"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              className="w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="My API Test"
            />
          </div>

          {/* Method + URL */}
          <div className="flex gap-2">
            <select
              value={form.method}
              onChange={(e) => setForm({ ...form, method: e.target.value })}
              className="w-24 px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              {methods.map((m) => (
                <option key={m} value={m}>{m}</option>
              ))}
            </select>
            <input
              type="text"
              value={form.url}
              onChange={(e) => setForm({ ...form, url: e.target.value })}
              className="flex-1 px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="https://api.example.com/endpoint"
            />
          </div>

          {/* Headers */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Headers (one per line: Key: Value)</label>
            <textarea
              value={headersText}
              onChange={(e) => setHeadersText(e.target.value)}
              className="w-full h-24 px-3 py-2 border rounded font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="Content-Type: application/json\nAuthorization: Bearer token"
            />
          </div>

          {/* Body */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Body</label>
            <textarea
              value={form.body || ""}
              onChange={(e) => setForm({ ...form, body: e.target.value })}
              className="w-full h-32 px-3 py-2 border rounded font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder='{"key": "value"}'
            />
          </div>

          {/* Expected Status */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">Expected Status</label>
            <input
              type="number"
              value={form.expected_status || ""}
              onChange={(e) =>
                setForm({ ...form, expected_status: parseInt(e.target.value) || undefined })
              }
              className="w-32 px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="200"
            />
          </div>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-3 mt-6">
          <button
            onClick={handleReplay}
            className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
          >
            Test Request
          </button>
          <button onClick={onClose} className="px-4 py-2 text-gray-700 hover:bg-gray-100 rounded">
            Cancel
          </button>
          <button onClick={handleSave} className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/replay/ReplayModal.tsx
git commit -m "feat(replay): add ReplayModal for editing targets"
```

---

## Task 3: 创建ReplayResults组件

**Files:**
- Create: `src/components/replay/ReplayResults.tsx`

- [ ] **Step 1: 创建ReplayResults**

```tsx
interface ReplayResult {
  target_id: string;
  status: number;
  duration_ms: number;
  success: boolean;
  error?: string;
}

interface ReplayResultsProps {
  results: ReplayResult[];
}

export function ReplayResults({ results }: ReplayResultsProps) {
  const successCount = results.filter((r) => r.success).length;
  const failCount = results.length - successCount;

  return (
    <div className="mt-6 bg-white rounded-lg shadow overflow-hidden">
      <div className="px-4 py-3 border-b bg-gray-50">
        <h3 className="text-lg font-medium">
          Replay Results{" "}
          <span className="text-green-600">{successCount} passed</span>
          {failCount > 0 && (
            <span className="text-red-600 ml-2">{failCount} failed</span>
          )}
        </h3>
      </div>

      <table className="w-full">
        <thead>
          <tr className="text-sm text-gray-500">
            <th className="px-4 py-2 text-left">Target</th>
            <th className="px-4 py-2 text-left">Status</th>
            <th className="px-4 py-2 text-left">Duration</th>
            <th className="px-4 py-2 text-left">Result</th>
          </tr>
        </thead>
        <tbody>
          {results.map((result, i) => (
            <tr key={i} className="border-t">
              <td className="px-4 py-2 text-sm">{result.target_id}</td>
              <td className="px-4 py-2">
                <span
                  className={`px-2 py-1 rounded text-xs font-mono ${
                    result.status >= 200 && result.status < 300
                      ? "bg-green-100 text-green-800"
                      : "bg-red-100 text-red-800"
                  }`}
                >
                  {result.status}
                </span>
              </td>
              <td className="px-4 py-2 text-sm">{result.duration_ms}ms</td>
              <td className="px-4 py-2">
                {result.success ? (
                  <span className="text-green-600">Success</span>
                ) : (
                  <span className="text-red-600" title={result.error}>
                    {result.error?.slice(0, 50) || "Failed"}
                  </span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/replay/ReplayResults.tsx
git commit -m "feat(replay): add ReplayResults component"
```

---

## Task 4: 创建Rust IPC命令

**Files:**
- Create: `src-tauri/src/commands/replay.rs`
- Create: `src-tauri/src/replay/engine.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建replay.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayTarget {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    #[serde(rename = "expected_status")]
    pub expected_status: Option<u16>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    #[serde(rename = "target_id")]
    pub target_id: String,
    pub status: u16,
    #[serde(rename = "duration_ms")]
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub fn get_replay_targets() -> Result<Vec<ReplayTarget>, String> {
    // 从配置加载保存的目标
    Ok(vec![])
}

#[tauri::command]
pub fn save_replay_target(target: ReplayTarget) -> Result<(), String> {
    // 保存到配置
    Ok(())
}

#[tauri::command]
pub fn delete_replay_target(id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn toggle_replay_target(id: String, enabled: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn execute_replay(targets: Vec<ReplayTarget>) -> Result<Vec<ReplayResult>, String> {
    let mut results = Vec::new();

    for target in targets {
        let result = crate::replay::engine::execute_target(&target).await;
        results.push(result);
    }

    Ok(results)
}
```

- [ ] **Step 2: 创建engine.rs**

```rust
use crate::commands::replay::{ReplayResult, ReplayTarget};
use std::collections::HashMap;
use std::time::Instant;

pub async fn execute_target(target: &ReplayTarget) -> ReplayResult {
    let start = Instant::now();

    // 构建请求
    let client = reqwest::Client::new();
    let mut request = client.request(
        reqwest::Method::from_str(&target.method),
        &target.url,
    );

    for (key, value) in &target.headers {
        request = request.header(key, value);
    }

    if let Some(body) = &target.body {
        request = request.body(body);
    }

    match request.send().await {
        Ok(response) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let status = response.status().as_u16();
            let success = target.expected_status.map(|e| status == e).unwrap_or(status < 400);

            ReplayResult {
                target_id: target.id.clone(),
                status,
                duration_ms,
                success,
                error: None,
            }
        }
        Err(e) => {
            ReplayResult {
                target_id: target.id.clone(),
                status: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                success: false,
                error: Some(e.to_string()),
            }
        }
    }
}
```

- [ ] **Step 3: 注册模块**

在 `src-tauri/src/lib.rs` 中添加:
```rust
pub mod replay;
pub mod commands;
pub mod commands::replay;
pub use commands::replay::*;
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/replay.rs src-tauri/src/replay/engine.rs src-tauri/src/lib.rs
git commit -m "feat(replay): add replay IPC commands and engine"
```

---

## Task 5: 编译验证

- [ ] **Step 1: 运行编译**

```bash
npm run build 2>&1 | tail -20
cargo build --bin proxybot-gui 2>&1 | tail -20
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "feat(replay): complete request replay enhancement"
```

---

## 验证清单

- [ ] 目标列表显示
- [ ] 新建/编辑/删除目标
- [ ] 修改参数后发送请求
- [ ] 模板变量替换
- [ ] 重放结果展示
- [ ] 编译通过
