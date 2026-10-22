# Request Replay Enhancement v0.9.0 设计方案

## Status: Draft

## 1. Overview

增强Replay功能，支持修改请求参数后重放，保存重放场景为模板。

**当前问题：**
- TUI仅有HAR导出
- 基础重放功能
- 无模板保存

**目标：**
- 修改参数后重放
- 保存场景模板
- 对比原始vs重放响应

---

## 2. 竞品分析

| 竞品 | 重放功能 |
|------|---------|
| Charles | Repeat(重放N次), Compose(修改后发送) |
| Proxyman | 完整重放 + 修改后重放 |
| mitmproxy | 无官方GUI增强，仅命令行 |

---

## 3. 功能设计

### 3.1 核心功能

**F1: 快速重放**
- 选中请求 → 一键重放
- 记录重放历史

**F2: 修改后重放**
- 修改任意字段 (method, url, headers, body)
- 发送修改后的请求
- 对比响应差异

**F3: 重放模板**
- 保存当前请求为模板
- 模板包含: method, url, headers, body, expected_response
- 支持变量: `{{timestamp}}`, `{{uuid}}`, `{{random}}`

**F4: 场景重放**
- 导入HAR文件
- 按顺序/并发重放
- 导出结果报告

---

## 4. 组件设计

### 4.1 ReplayPage.tsx

```tsx
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

function ReplayPage() {
  const [targets, setTargets] = useState<ReplayTarget[]>([]);
  const [selectedTarget, setSelectedTarget] = useState<ReplayTarget | null>(null);
  const [replayResults, setReplayResults] = useState<ReplayResult[]>([]);

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">Replay</h1>
        <div className="flex gap-2">
          <button onClick={handleImportHar}>Import HAR</button>
          <button onClick={handleStartReplay}>Start Replay</button>
        </div>
      </div>

      {/* Targets Table */}
      <div className="bg-white rounded-lg shadow">
        <table className="w-full">
          <thead>
            <tr>
              <th>Name</th>
              <th>Method</th>
              <th>URL</th>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {targets.map(target => (
              <tr key={target.id}>
                <td>{target.name}</td>
                <td><span className="method-badge">{target.method}</span></td>
                <td className="truncate">{target.url}</td>
                <td>{target.expected_status || '-'}</td>
                <td>
                  <button onClick={() => handleReplay(target)}>Replay</button>
                  <button onClick={() => handleEdit(target)}>Edit</button>
                  <button onClick={() => handleDelete(target.id)}>Delete</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Replay Results */}
      {replayResults.length > 0 && <ReplayResults results={replayResults} />}
    </div>
  );
}
```

### 4.2 ReplayModal.tsx (修改后重放)

```tsx
interface ReplayRequest {
  method: string;
  url: string;
  headers: Record<string, string>;
  body?: string;
}

function ReplayModal({ request, onReplay, onClose }: ReplayModalProps) {
  const [form, setForm] = useState<ReplayRequest>({
    method: request.method,
    url: `${request.host}${request.path}`,
    headers: { ...request.headers },
    body: request.body,
  });

  const [response, setResponse] = useState<Response | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleReplay() {
    setLoading(true);
    try {
      const res = await fetch(form.url, {
        method: form.method,
        headers: form.headers,
        body: form.body,
      });
      setResponse(await res.text());
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-2xl p-6">
        <h2 className="text-xl font-bold mb-4">Replay Request</h2>

        <form className="space-y-4">
          {/* Method + URL */}
          <div className="flex gap-2">
            <select
              value={form.method}
              onChange={e => setForm({ ...form, method: e.target.value })}
              className="w-24"
            >
              <option value="GET">GET</option>
              <option value="POST">POST</option>
              <option value="PUT">PUT</option>
              <option value="DELETE">DELETE</option>
            </select>
            <input
              type="text"
              value={form.url}
              onChange={e => setForm({ ...form, url: e.target.value })}
              className="flex-1"
            />
          </div>

          {/* Headers */}
          <div>
            <label className="block text-sm font-medium mb-1">Headers</label>
            <textarea
              value={Object.entries(form.headers).map(([k,v]) => `${k}: ${v}`).join('\n')}
              onChange={e => {
                const headers: Record<string, string> = {};
                e.target.value.split('\n').forEach(line => {
                  const [key, ...val] = line.split(':');
                  if (key && val.length) headers[key.trim()] = val.join(':').trim();
                });
                setForm({ ...form, headers });
              }}
              className="w-full h-32 font-mono text-sm"
            />
          </div>

          {/* Body */}
          <div>
            <label className="block text-sm font-medium mb-1">Body</label>
            <textarea
              value={form.body || ''}
              onChange={e => setForm({ ...form, body: e.target.value })}
              className="w-full h-32 font-mono text-sm"
            />
          </div>
        </form>

        {/* Response */}
        {response && (
          <div className="mt-4 p-4 bg-gray-100 rounded">
            <h3 className="font-semibold mb-2">Response</h3>
            <pre className="text-sm overflow-auto max-h-64">{response}</pre>
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-3 mt-6">
          <button onClick={onClose}>Cancel</button>
          <button onClick={handleReplay} disabled={loading}>
            {loading ? 'Replaying...' : 'Replay'}
          </button>
        </div>
      </div>
    </div>
  );
}
```

### 4.3 Template Variables

```tsx
const templateVars = {
  '{{timestamp}}': () => Date.now().toString(),
  '{{uuid}}': () => crypto.randomUUID(),
  '{{random}}': () => Math.random().toString(36).slice(2),
  '{{randomInt}}': () => Math.floor(Math.random() * 1000).toString(),
  '{{date}}': () => new Date().toISOString().split('T')[0],
};

function renderTemplate(template: string): string {
  return template.replace(/\{\{(\w+)\}\}/g, (_, key) => {
    if (key in templateVars) {
      return templateVars[key as keyof typeof templateVars]();
    }
    return `{{${key}}}`;
  });
}
```

---

## 5. Rust IPC

```rust
#[tauri::command]
fn save_replay_template(template: ReplayTemplate) -> Result<(), String>;

#[tauri::command]
fn get_replay_templates() -> Result<Vec<ReplayTemplate>, String>;

#[tauri::command]
fn execute_replay(targets: Vec<ReplayTarget>) -> Result<Vec<ReplayResult>, String>;
```

---

## 6. 验证

```bash
# 从Traffic选中请求 → Replay
# 修改参数 → 发送
# 对比响应差异
# 保存模板 → 加载
```
