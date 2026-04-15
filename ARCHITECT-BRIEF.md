# Architect Brief — ProxyBot

## Step 12 — WSS 详情面板 ✅ (完成)

---

## Step 13 — 导出 JSON

目标：将请求列表导出为 JSON 文件，方便程序化分析或导入其他工具。

### UI 实现

在「导出 HAR」按钮旁边加「导出 JSON」按钮：

```tsx
<button className="btn-export" onClick={exportJson}>
  📄 JSON
</button>
```

```tsx
const exportJson = async () => {
  const data = JSON.stringify(filteredRequests, null, 2);
  const blob = new Blob([data], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `proxybot-requests-${new Date().toISOString().slice(0,10)}.json`;
  a.click();
  URL.revokeObjectURL(url);
};
```

纯前端实现，无需 Rust 修改。

### 验收标准

1. 点「导出 JSON」→ 下载 `.json` 文件
2. 文件内容是有效的 JSON，包含完整请求字段
