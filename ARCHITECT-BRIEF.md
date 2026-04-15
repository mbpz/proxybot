# Architect Brief — ProxyBot

## Step 7 — Production Build ✅ (完成)

---

## Step 8 — 请求搜索/过滤

目标：在请求列表上方增加搜索框和过滤器，快速定位特定请求。

### UI 实现

**1. 搜索栏布局**

```
[🔍 Search host, path, method... ] [Method ▼] [Status ▼] [App ▼] [Clear]
```

- 搜索框：实时过滤，支持 host / path / method 模糊匹配
- Method 下拉：ALL / GET / POST / PUT / DELETE / WebSocket
- Status 下拉：ALL / 2xx / 3xx / 4xx / 5xx
- App 下拉：ALL / WeChat / Douyin / Alipay / Unknown
- Clear 按钮：清空所有过滤条件

**2. 过滤逻辑（纯前端）**

```typescript
const filtered = requests.filter(req => {
  const matchSearch = !search
    || req.host.includes(search)
    || req.path.includes(search)
    || req.method.includes(search.toUpperCase())

  const matchMethod = !method || method === 'ALL' || req.method === method
  const matchStatus = !status || status === 'ALL' || matchesStatusGroup(req.status, status)
  const matchApp = !app || app === 'ALL' || req.app_name === app

  return matchSearch && matchMethod && matchStatus && matchApp
})
```

**3. 状态分组匹配**

```typescript
function matchesStatusGroup(status: number, group: string): boolean {
  if (group === '2xx') return status >= 200 && status < 300
  if (group === '3xx') return status >= 300 && status < 400
  if (group === '4xx') return status >= 400 && status < 500
  if (group === '5xx') return status >= 500
  return status === parseInt(group)
}
```

### 不做

- 服务端过滤（全部在前端过滤，数据量大了再说）
- 正则搜索
- 搜索历史

### 验收标准

1. 输入 "baidu" → 只显示 host/path 含 baidu 的请求
2. 选择 Method = POST → 只显示 POST 请求
3. 选择 Status = 4xx → 只显示 4xx 请求
4. 组合搜索：Method=GET + App=WeChat → 显示微信的 GET 请求
5. Clear 按钮 → 所有过滤条件清空，完整列表恢复
