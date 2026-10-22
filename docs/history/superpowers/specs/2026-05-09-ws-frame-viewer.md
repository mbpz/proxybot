# WebSocket Frame Viewer v0.9.0 设计方案

## Status: Draft

## 1. Overview

实现GUI WebSocket帧实时查看器，显示帧方向、opcode、payload。

**当前问题：**
- TUI仅基础WS支持
- 无可视化帧列表
- 无16进制查看

**目标：**
- 实时帧列表
- 帧详情面板
- 16进制查看

---

## 2. 竞品分析

| 竞品 | WS帧查看器 |
|------|-----------|
| Proxyman | 完整帧列表，支持二进制查看 |
| mitmproxy | mitmweb提供WS消息流 |
| Charles | 支持文本/二进制切换 |

---

## 3. 数据结构

### 3.1 Rust端

```rust
struct WsFrame {
    id: String,
    request_id: String,      // 关联的HTTP请求
    direction: FrameDirection, // Incoming/Outgoing
    opcode: u8,              // 0x01=text, 0x02=binary, 0x08=close
    payload: Vec<u8>,
    payload_text: Option<String>, // 如果是文本
    timestamp: i64,
}

enum FrameDirection {
    Incoming,
    Outgoing,
}
```

### 3.2 IPC命令

```rust
#[tauri::command]
fn get_ws_frames(request_id: String) -> Result<Vec<WsFrame>, String>;

#[tauri::command]
fn subscribe_ws_frames() -> Result<Channel<WsFrame>, String>;
```

---

## 4. 组件设计

### 4.1 WsFramesView.tsx

```tsx
interface WsFrame {
  id: string;
  direction: 'incoming' | 'outgoing';
  opcode: number;
  payload: string;
  timestamp: number;
}

function WsFramesView({ requestId }: { requestId: string }) {
  const [frames, setFrames] = useState<WsFrame[]>([]);
  const [selectedFrame, setSelectedFrame] = useState<WsFrame | null>(null);

  // Subscribe to real-time frames
  useEffect(() => {
    const unsubscribe = subscribeWsFrames(requestId, (frame) => {
      setFrames(prev => [...prev, frame]);
    });
    return () => unsubscribe();
  }, [requestId]);

  return (
    <div className="flex h-full">
      {/* Frame List */}
      <div className="w-1/2 border-r overflow-auto">
        {frames.map(frame => (
          <div
            key={frame.id}
            onClick={() => setSelectedFrame(frame)}
            className={`flex items-center px-3 py-2 border-b cursor-pointer ${
              frame.direction === 'incoming' ? 'text-green-600' : 'text-blue-600'
            }`}
          >
            <span className="w-4">{frame.direction === 'incoming' ? '←' : '→'}</span>
            <span className="w-8 font-mono text-xs">{getOpcodeName(frame.opcode)}</span>
            <span className="flex-1 truncate text-sm">{truncate(frame.payload, 30)}</span>
            <span className="text-xs text-gray-500">{formatTime(frame.timestamp)}</span>
          </div>
        ))}
      </div>

      {/* Frame Detail */}
      <div className="w-1/2 p-4 overflow-auto">
        {selectedFrame ? (
          <FrameDetail frame={selectedFrame} />
        ) : (
          <div className="text-gray-500">Select a frame to view details</div>
        )}
      </div>
    </div>
  );
}
```

### 4.2 FrameDetail.tsx

```tsx
function FrameDetail({ frame }: { frame: WsFrame }) {
  const [viewMode, setViewMode] = useState<'text' | 'hex'>('text');

  return (
    <div className="space-y-4">
      {/* Metadata */}
      <div className="grid grid-cols-2 gap-2 text-sm">
        <div><span className="text-gray-500">Direction:</span> {frame.direction}</div>
        <div><span className="text-gray-500">Opcode:</span> {frame.opcode} ({getOpcodeName(frame.opcode)})</div>
        <div><span className="text-gray-500">Size:</span> {frame.payload.length} bytes</div>
        <div><span className="text-gray-500">Time:</span> {formatTime(frame.timestamp)}</div>
      </div>

      {/* View Mode Toggle */}
      <div className="flex gap-2">
        <button onClick={() => setViewMode('text')} className={viewMode === 'text' ? 'bg-blue-500 text-white' : ''}>
          Text
        </button>
        <button onClick={() => setViewMode('hex')} className={viewMode === 'hex' ? 'bg-blue-500 text-white' : ''}>
          Hex
        </button>
      </div>

      {/* Payload */}
      <div className="bg-gray-100 rounded p-3 font-mono text-xs overflow-auto" style={{ maxHeight: '400px' }}>
        {viewMode === 'text' ? (
          <pre className="whitespace-pre-wrap">{frame.payload}</pre>
        ) : (
          <HexDump bytes={stringToBytes(frame.payload)} />
        )}
      </div>
    </div>
  );
}
```

### 4.3 HexDump Component

```tsx
function HexDump({ bytes }: { bytes: number[] }) {
  const lines = [];
  for (let i = 0; i < bytes.length; i += 16) {
    const chunk = bytes.slice(i, i + 16);
    const hex = chunk.map(b => b.toString(16).padStart(2, '0')).join(' ');
    const ascii = chunk.map(b => b >= 32 && b < 127 ? String.fromCharCode(b) : '.').join('');
    lines.push(`${i.toString(16).padStart(8, '0')}  ${hex.padEnd(48)}  ${ascii}`);
  }
  return <pre>{lines.join('\n')}</pre>;
}
```

---

## 5. 依赖

无需额外依赖，使用内置功能实现。

---

## 6. 验证

```bash
# 连接WS服务器
# 发送WS消息
# 验证帧列表实时更新
# 测试Text/Hex切换
```
