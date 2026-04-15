# Architect Brief — ProxyBot

## Step 10 — CA 安装指引 UI ✅ (完成)

---

## Step 11 — 持久化历史

目标：请求记录保存到本地文件，重启 App 后历史记录仍在。

### 方案选择

直接用 `serde_json` 读写 JSON 文件，不引入额外数据库依赖。

### 存储位置

`~/.proxybot/history.json`

格式：
```json
{
  "version": 1,
  "last_updated": "2026-04-15T10:23:00Z",
  "requests": [
    {
      "id": "req_001",
      "timestamp": "2026-04-15T10:23:00.123Z",
      "method": "GET",
      "host": "httpbin.org",
      "path": "/get",
      "status": 200,
      "latency_ms": 1281,
      "app_name": null,
      "app_icon": null,
      "request_headers": "...",
      "response_headers": "...",
      "response_body": "...",
      "request_body": null
    }
  ]
}
```

### Rust 实现

**1. 新文件 `src-tauri/src/history.rs`**

```rust
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

const HISTORY_FILE: &str = "history.json";
const MAX_STORED: usize = 1000;  // 最多保留 1000 条

pub struct HistoryStore {
    path: PathBuf,
}

impl HistoryStore {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(home).join(".proxybot");
        std::fs::create_dir_all(&dir).ok();
        Self { path: dir.join(HISTORY_FILE) }
    }

    pub fn load(&self) -> Vec<InterceptedRequest> {
        let file = File::open(&self.path).ok()?;
        let reader = BufReader::new(file);
        let data: serde_json::Value = serde_json::from_reader(reader).ok()?;
        data.get("requests")?.as_array()?
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect()
    }

    pub fn save(&self, requests: &[InterceptedRequest]) -> Result<(), String> {
        let data = serde_json::json!({
            "version": 1,
            "last_updated": chrono_now(),
            "requests": &requests[..requests.len().min(MAX_STORED)]
        });
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &data).map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())
    }
}
```

**2. Tauri Command**

```rust
#[tauri::command]
pub fn load_history() -> Vec<InterceptedRequest> {
    HistoryStore::new().load()
}

#[tauri::command]
pub fn save_history(requests: Vec<InterceptedRequest>) -> Result<(), String> {
    HistoryStore::new().save(&requests)
}
```

**3. 自动保存**

- 每次新请求到来时（emit event 前）：异步保存到文件
- 关闭 App 时（tauri window `on_window_event`）：同步保存
- 最多保留 1000 条，超出截断旧记录

### UI 修改

**1. 启动时加载**

```tsx
useEffect(() => {
  invoke<Vec<InterceptedRequest>>("load_history").then(hist => {
    if (hist.length > 0) setRequests(hist);
  });
}, []);
```

**2. 新增按钮**

过滤栏旁加「保存历史」+「清空历史」按钮：
```tsx
<button onClick={saveHistory}>💾 保存</button>
<button onClick={clearHistory}>🗑️ 清空</button>
```

**3. 清空历史**

```tsx
const clearHistory = async () => {
  if (!confirm("确定清空所有历史记录？")) return;
  await invoke("save_history", { requests: [] });
  setRequests([]);
};
```

### 不做

- 分页加载（1000 条以内全量加载）
- 按时间/App 过滤历史
- 历史记录搜索

### 验收标准

1. 重启 App → 历史请求记录自动恢复
2. 点「💾 保存」→ 手动触发一次保存
3. 点「🗑️ 清空」→ 确认弹框 → 清空所有记录，文件同步清空
