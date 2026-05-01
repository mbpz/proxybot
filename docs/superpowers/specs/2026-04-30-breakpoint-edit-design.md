# Breakpoint 编辑功能设计文档

## Status: Approved

## 概述

扩展 Breakpoint 功能，支持用户编辑请求/响应的完整内容（method、URL、headers、body）。

## 设计决策

| 维度 | 决策 |
|------|------|
| 编辑方式 | 模态全屏编辑（覆盖 detail panel 区域） |
| 可编辑字段 | method, url, headers, body |
| 导航方式 | 方向键 ↑/↓ 选择字段，←/→ 在 header 行内移动 |
| Headers 编辑 | 方向键选中某行 → Enter 弹出编辑框 → 输入 → Enter 确认 |

## 架构

### 新增状态

```rust
// src/tui/mod.rs

/// Breakpoint 编辑模式
#[derive(Clone, PartialEq, Eq)]
pub enum BreakpointEditMode {
    None,           // 非编辑模式
    Viewing,        // 查看模式（断点触发初始状态）
    Editing(usize), // 编辑模式（usize = 当前选中字段索引）
}

pub enum BreakpointField {
    Method,      // 索引 0
    Url,         // 索引 1
    Headers,     // 索引 2
    Body,        // 索引 3
}

/// 扩展 BreakpointState
pub struct BreakpointState {
    pub mode: BreakpointMode,
    pub edit_mode: BreakpointEditMode,
    pub selected_field: BreakpointField,
    pub editing_header_index: Option<usize>,  // 正在编辑的 header 行索引
    pub header_input: String,                 // header 编辑输入缓冲
    pub body_input: String,                   // body 编辑输入缓冲
    pub queue: Vec<InterceptedRequest>,
    pub current_edit: Option<InterceptedRequest>,
    pub original_request: InterceptedRequest,
}
```

### 快捷键（编辑模式下）

| 快捷键 | 动作 |
|--------|------|
| `e` | 进入编辑模式 |
| `↑/↓` | 选择字段 |
| `←/→` | 在 header 行内导航 |
| `Enter` | 确认当前编辑 / 弹出 header 编辑框 |
| `Esc` | 取消当前编辑，返回 Viewing |
| `g` | 发送修改后的请求（从编辑模式也可） |

### 渲染流程

```
render_breakpoint_editor (查看模式)
    │
    │ 按 [e]
    ▼
render_breakpoint_editor (编辑模式)
    │
    │ 显示可编辑字段 + 选中高亮
    │
    │ ↑/↓ 选择字段
    ▼
    ┌────────────────────────────────────┐
    │ [1] Method:   GET                  │  ← 选中
    │ [2] URL:      https://...          │
    │ [3] Headers:                          │
    │     > Content-Type: application/json│
    │ [4] Body:    {...}                  │
    └────────────────────────────────────┘
    │
    │ Enter 编辑 method
    ▼
    ┌────────────────────────────────────┐
    │ Method: [GET________________]      │  ← 输入框
    │ [Enter] confirm  [Esc] cancel    │
    └────────────────────────────────────┘
```

## 实现要点

### 1. BreakpointEditMode 状态机

```
Viewing ──[e]──► Editing(0)
  ▲                    │
  │                    │
  └──[Esc/完成]────────┘
```

### 2. 字段索引映射

```rust
const FIELD_INDICES: &[BreakpointField] = &[
    BreakpointField::Method,  // 0
    BreakpointField::Url,      // 1
    BreakpointField::Headers,  // 2
    BreakpointField::Body,      // 3
];
```

### 3. Header 编辑特殊处理

- Headers 显示为 `> key: value` 格式
- 方向键 ←/→ 在行内移动
- Enter 弹出编辑框：`key: value` 格式
- 编辑完成后更新 `current_edit.req_headers`

### 4. 数据流

1. 用户按 `e` → `edit_mode = Editing(0)`
2. 用户方向键选择字段 → `selected_field` 更新，UI 高亮变化
3. 用户按 Enter → 根据 `selected_field` 进入相应编辑：
   - Method/Url/Body → 弹出单行输入框
   - Headers → 弹出 header 编辑框
4. 用户输入后按 Enter → 更新 `current_edit` 对应字段
5. 用户按 `g` → 发送修改后的请求（代理层处理）

## 测试要点

- 状态转换：Viewing → Editing → Viewing
- 字段导航：↑/↓ 正确切换 selected_field
- Header 编辑：添加/删除/修改 header
- 数据完整性：编辑后的请求内容正确
