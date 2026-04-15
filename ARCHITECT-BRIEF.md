# Architect Brief — ProxyBot

## Step 7 — Production Build ✅ (完成)

---

## Step 9 — 导出 HAR 文件

目标：将请求列表导出为 HAR（HTTP Archive）格式，可在 Chrome DevTools / Charles Proxy / Fiddler 中打开。

### HAR 格式概述

HAR 1.2 spec: https://w3c.github.io/web-performance/specs/HAR/Overview.html

```json
{
  "log": {
    "version": "1.2",
    "creator": { "name": "ProxyBot", "version": "0.1.0" },
    "entries": [
      {
        "startedDateTime": "2026-04-15T10:23:45.123Z",
        "time": 1281,
        "request": {
          "method": "GET",
          "url": "https://httpbin.org/get",
          "httpVersion": "HTTP/1.1",
          "headers": [{ "name": "Host", "value": "httpbin.org" }],
          "queryString": [],
          "cookies": [],
          "headersSize": -1,
          "bodySize": 0
        },
        "response": {
          "status": 200,
          "statusText": "OK",
          "httpVersion": "HTTP/1.1",
          "headers": [...],
          "content": { "size": 123, "mimeType": "application/json", "text": "..." },
          "redirectURL": "",
          "headersSize": -1,
          "bodySize": 123
        },
        "timings": { "send": -1, "wait": 1281, "receive": -1 }
      }
    ]
  }
}
```

### Rust 实现

**1. 新文件 `src-tauri/src/har.rs`**

```rust
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
struct HarLog {
    version: &'static str,
    creator: HarCreator,
    entries: Vec<HarEntry>,
}

#[derive(Serialize)]
struct HarCreator {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct HarEntry {
    started_date_time: String,  // ISO 8601
    time: i64,                  // latency ms
    request: HarRequest,
    response: HarResponse,
    #[serde(rename = "timings")]
    timings_obj: HarTimings,
}

#[derive(Serialize)]
struct HarRequest {
    method: String,
    url: String,
    http_version: String,
    headers: Vec<HarHeader>,
    query_string: Vec<HarQueryParam>,
    cookies: Vec<()>,
    headers_size: i64,
    body_size: i64,
}

#[derive(Serialize)]
struct HarResponse {
    status: u16,
    status_text: String,
    http_version: String,
    headers: Vec<HarHeader>,
    content: HarContent,
    redirect_url: String,
    headers_size: i64,
    body_size: i64,
}
```

**2. Tauri Command**

```rust
#[tauri::command]
pub fn export_har(requests: Vec<InterceptedRequest>) -> Result<String, String> {
    let har = build_har(requests);
    serde_json::to_string_pretty(&har).map_err(|e| e.to_string())
}
```

**3. 转换逻辑**

- `InterceptedRequest` → `HarEntry`：时间戳用 `timestamp_ms` 转换 ISO 8601
- URL 构造：`https://{host}{path}`
- Headers 解析：`request_headers` 是 `"Name: value\r\n..."` 格式，按 `\r\n` 分割，再按 `": "` 分割 name/value
- Response body：用已缓存的 `response_body`（10KB 内）
- `timings.wait` = `latency_ms`

### UI 实现

**1. 导出按钮**

在请求列表 Tab 栏旁边加「导出 HAR」按钮：

```tsx
<button onClick={exportHar}>Export HAR</button>
```

**2. 导出逻辑**

```tsx
const exportHar = async () => {
  const har = await invoke<string>("export_har", { requests: requests });
  const blob = new Blob([har], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `proxybot-${new Date().toISOString().slice(0,10)}.har`;
  a.click();
  URL.revokeObjectURL(url);
};
```

### 验收标准

1. 点「导出 HAR」→ 下载 `.har` 文件
2. 用 Chrome DevTools → Network → Import 导入 → 显示所有请求
3. 用 Charles Proxy → File → Import → 显示所有请求
