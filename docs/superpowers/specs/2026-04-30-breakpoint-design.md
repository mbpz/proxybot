# Breakpoint 设计文档

## Status: Approved

## 概述

Breakpoint 允许用户在请求/响应生命周期中暂停、检查和修改流量。这是 MITM 代理的核心功能，参考 mitmproxy 的断点交互设计。

## 设计决策

| 维度 | 决策 |
|------|------|
| 触发方式 | 混合：手动 `[b]` 单次 + Rules `BREAKPOINT` 规则 |
| 可编辑字段 | method, url, headers, body（完整编辑） |
| 处理方式 | 同步阻塞：请求停在代理层，等待确认 |
| 交互界面 | Detail panel 覆盖：复用 Traffic tab 40% 区域 |
| 多请求处理 | 队列模式：先到先服务，顺序处理 |
| 状态存储 | 内存：TUI 生命周期内有效 |

## 架构

### 新增类型

```rust
// src/tui/mod.rs

#[derive(Clone, PartialEq)]
pub enum BreakpointType {
    Request,  // 请求断点（在发送前断下）
    Response, // 响应断点（在接收后断下）
}

#[derive(Clone, PartialEq)]
pub enum BreakpointState2 {
    None,
    Paused(BreakpointType),        // 请求已暂停，等待处理
    Editing(usize),                // 用户正在编辑（索引）
}

pub struct BreakpointState {
    pub queue: Vec<InterceptedRequest>,  // 等待处理的断点队列
    pub current_edit: Option<InterceptedRequest>,  // 当前编辑的请求
    pub original_request: InterceptedRequest,  // 原始请求（用于取消恢复）
    pub breakpoint_type: BreakpointType,
}

impl Default for BreakpointState {
    fn default() -> Self {
        Self {
            queue: Vec::new(),
            current_edit: None,
            original_request: InterceptedRequest::default(),
            breakpoint_type: BreakpointType::Request,
        }
    }
}
```

### 新增 Action

```rust
// src/tui/input.rs

pub enum InputAction {
    // ... 现有
    ToggleBreakpoint,   // 对选中请求开启/关闭断点
    BreakpointGo,      // 继续发送（GO）
    BreakpointCancel,  // 取消请求（CANCEL）
    BreakpointEdit,    // 进入编辑模式
    BreakpointNext,    // 处理队列中的下一个
}
```

### 快捷键

| 快捷键 | 动作 |
|--------|------|
| `b` | 对选中请求开启/关闭断点 |
| `g` | 继续发送（GO） |
| `c` | 取消请求（CANCEL） |
| `e` | 编辑请求（EDIT） |
| `n` | 处理队列中的下一个 |
| `Tab` | 切换编辑字段 |
| `Enter` | 发送修改后的请求 |

### Detail Panel 编辑状态渲染

```rust
// src/tui/render/traffic.rs

fn render_breakpoint_editor(f: &mut Frame, area: Rect, app: &mut TuiApp) {
    // 当 breakpoint_state != None 时，detail panel 显示断点编辑界面
    // 显示字段：method, url, headers, body
    // 每个字段可编辑，使用方向键导航
    // 底部显示 [g]o [c]ancel [e]dit
}
```

### 规则引擎扩展

```rust
// src/rules.rs

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleAction {
    Direct,
    Proxy,
    Reject,
    MapRemote(String),
    MapLocal(String),
    // 新增
    Breakpoint(BreakpointType),  // 断点（请求/响应）
}
```

### 代理层断点集成

```rust
// src/proxy.rs

// 当 RulesEngine.match_host 返回 Breakpoint 时
// 代理层需要：
// 1. 暂停当前请求/响应
// 2. 发送 InterceptedRequest 到 TUI via broadcast channel
// 3. 等待 TUI 发送修改后的请求/响应
// 4. 继续处理

// 需要新增 channel：
// static BREAKPOINT_TX: ... 用于 TUI -> Proxy 发送修改后的请求
```

## 数据流

```
┌─────────────────────────────────────────────────────────────────┐
│                        请求生命周期                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 请求进入代理层                                               │
│          │                                                      │
│          ▼                                                      │
│  2. RulesEngine.match_host() 返回 Breakpoint(Request)           │
│          │                                                      │
│          ▼                                                      │
│  3. 代理层通过 broadcast 发送 InterceptedRequest 到 TUI          │
│          │                                                      │
│          ▼                                                      │
│  4. TUI 进入断点编辑模式，detail panel 显示编辑界面              │
│          │                                                      │
│          ▼                                                      │
│  5. 用户编辑字段，按 [g] 确认发送                                │
│          │                                                      │
│          ▼                                                      │
│  6. TUI 通过 BREAKPOINT_TX 发送修改后的请求                      │
│          │                                                      │
│          ▼                                                      │
│  7. 代理层用修改后的请求继续处理                                 │
│          │                                                      │
│          ▼                                                      │
│  8. 响应阶段同样可能触发 Breakpoint(Response)                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 错误处理

- **取消（Cancel）**: 直接丢弃该请求，返回空响应或连接关闭
- **超时**: 如果 TUI 30 秒无响应，自动放行（类似 mitmproxy）
- **编辑无效**: 如果用户编辑导致请求构造失败，显示错误提示，不放行

## 测试策略

- 单元测试：BreakpointState 状态转换
- 集成测试：TUI 输入 → 状态更新 → 渲染正确
- 手动测试：用 curl 触发断点，验证同步阻塞

## 里程碑

| 版本 | 内容 |
|------|------|
| v0.3.0 | 基础断点功能：手动触发 + Request 断点 + Detail Panel 编辑 |
| v0.3.1 | Response 断点 + 队列处理 |
| v0.3.2 | 规则触发断点 |
