# Architect Brief — ProxyBot

## Step 11 — 持久化历史 ✅ (完成)

---

## Step 12 — WSS 详情面板

目标：点 WSS 消息条目，弹出详情查看完整帧内容（Text 帧全文）。

### UI 实现

**1. WSS 消息列表已有字段**

WSS Tab 当前显示：Time | Direction | Host | Size | Content Preview

Content Preview 只显示前 20-30 字符。

**2. 点击展开**

```tsx
const [selectedWssMsg, setSelectedWssMsg] = useState<WssMessage | null>(null);
```

WSS 消息行加 `onClick`：

```tsx
<div className="wss-list">
  {filteredWssMsgs.map(msg => (
    <div
      key={msg.id}
      className={`wss-row ${msg.direction}`}
      onClick={() => setSelectedWssMsg(msg)}
    >
      <span>{msg.timestamp}</span>
      <span>{msg.direction === '↑' ? '↑' : '↓'}</span>
      <span>{msg.host}</span>
      <span>{msg.app_icon} {msg.app_name || ''}</span>
      <span className="wss-preview">{msg.content.slice(0, 30)}</span>
    </div>
  ))}
</div>
```

**3. 详情面板**

和 HTTP 请求详情面板相同的右侧 Slide-in 面板（复用样式）：

```
WSS 详情
─────────────────
Direction: ↑ (send)
Host: wss://api.weixin.qq.com
Size: 1024 bytes
App: 💬 WeChat
Time: 22:45:33.123

Content:
[完整 Text 帧内容，可滚动]
```

Binary 帧显示：`[Binary {msg.size} bytes — not displayed as text]`

**4. 关闭**

点 overlay 或 X 关闭。

### 不做

- 二进制帧 hex dump
- WSS 消息搜索（和 HTTP 请求共用搜索栏后再说）
- WSS 消息持久化（Step 11 的历史不包含 WSS）

### 验收标准

1. 点任意 WSS 消息行 → 右侧弹出详情面板
2. Text 帧显示完整内容（可滚动）
3. Binary 帧显示 `[Binary N bytes]` 而非乱码
4. 点 overlay 或 X → 关闭面板
