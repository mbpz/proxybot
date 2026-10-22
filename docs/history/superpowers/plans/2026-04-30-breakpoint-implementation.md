# Breakpoint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 TUI Breakpoint 功能，允许用户在 Traffic tab 暂停、检查和修改 HTTP 请求/响应。

**Architecture:** 断点功能通过新增 `BreakpointState` 在 TUI 层管理状态，代理层在匹配到 BREAKPOINT 规则时暂停请求，通过 broadcast channel 通知 TUI，TUI 编辑完成后通过专用 channel 发回修改后的请求。

**Tech Stack:** Rust, ratatui, crossterm, tokio broadcast channel

---

## 文件结构

```
src-tauri/src/
├── tui/
│   ├── mod.rs           # BreakpointState, BreakpointType, BreakpointMode 新增
│   ├── input.rs        # InputAction::ToggleBreakpoint 等新增
│   └── render/
│       └── traffic.rs   # render_breakpoint_editor() 新增
├── rules.rs            # RuleAction::Breakpoint 新增
└── proxy.rs            # 代理层断点集成
```

---

## Task 1: 添加 Breakpoint 类型和状态

**Files:**
- Modify: `src-tauri/src/tui/mod.rs`

- [ ] **Step 1: Write failing test - 验证 BreakpointState 默认值**

```rust
// src-tauri/src/tui/mod.rs tests 模块新增

#[test]
fn test_breakpoint_state_default() {
    let state = BreakpointState::default();
    assert_eq!(state.mode, BreakpointMode::None);
    assert!(state.queue.is_empty());
    assert!(state.current_edit.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib -- breakpoint --nocapture`
Expected: FAIL — `BreakpointState` not defined

- [ ] **Step 3: 为 InterceptedRequest 添加 Default derive**

在 `src-tauri/src/proxy.rs` 中：

```rust
#[derive(Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct InterceptedRequest {
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --lib -- breakpoint --nocapture`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/proxy.rs src-tauri/src/tui/mod.rs
git commit -m "feat(breakpoint): add BreakpointState types and InterceptedRequest Default"
```

---

## Task 2: 添加 InputAction 断点相关动作

**Files:**
- Modify: `src-tauri/src/tui/input.rs`

- [ ] **Step 1: 添加 InputAction 枚举变体**

在 `InputAction` 枚举中添加：

```rust
/// Toggle breakpoint on selected request (Traffic tab).
ToggleBreakpoint,
/// Continue sending paused request (GO).
BreakpointGo,
/// Cancel the paused request.
BreakpointCancel,
/// Switch to editing mode for current breakpoint.
BreakpointEdit,
```

- [ ] **Step 2: 添加键盘绑定**

在 `handle_key_event` 函数中添加（Traffic tab）：

```rust
KeyCode::Char('b') if current_tab == Tab::Traffic => InputAction::ToggleBreakpoint,
KeyCode::Char('g') if current_tab == Tab::Traffic => InputAction::BreakpointGo,
KeyCode::Char('c') if current_tab == Tab::Traffic && app.traffic.breakpoint.mode != BreakpointMode::None => InputAction::BreakpointCancel,
KeyCode::Char('e') if current_tab == Tab::Traffic && app.traffic.breakpoint.mode != BreakpointMode::None => InputAction::BreakpointEdit,
```

- [ ] **Step 3: Build to verify**

Run: `cargo build --lib`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/tui/input.rs
git commit -m "feat(breakpoint): add ToggleBreakpoint/BreakpointGo/Cancel/Edit actions"
```

---

## Task 3: 修改 rules.rs 添加 RuleAction::Breakpoint

**Files:**
- Modify: `src-tauri/src/rules.rs`

- [ ] **Step 1: 扩展 RuleAction 枚举**

在 `RuleAction` 枚举中添加：

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE", tag = "type", content = "target")]
pub enum RuleAction {
    Direct,
    Proxy,
    Reject,
    MapRemote(String),
    MapLocal(String),
    #[serde(rename = "BREAKPOINT")]
    Breakpoint(BreakpointTarget),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BreakpointTarget {
    Request,
    Response,
    Both,
}

impl std::fmt::Display for RuleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuleAction::Direct => write!(f, "DIRECT"),
            RuleAction::Proxy => write!(f, "PROXY"),
            RuleAction::Reject => write!(f, "REJECT"),
            RuleAction::MapRemote(ref t) => write!(f, "MAPREMOTE:{}", t),
            RuleAction::MapLocal(ref t) => write!(f, "MAPLOCAL:{}", t),
            RuleAction::Breakpoint(ref t) => write!(f, "BREAKPOINT:{:?}", t),
        }
    }
}
```

- [ ] **Step 2: 更新 RuleEntry to_rule 解析**

在 `RuleEntry::to_rule()` 方法中添加：

```rust
"BREAKPOINT" => {
    let target = match self.target.as_deref() {
        Some("REQUEST") => BreakpointTarget::Request,
        Some("RESPONSE") => BreakpointTarget::Response,
        Some("BOTH") | None => BreakpointTarget::Both,
        _ => BreakpointTarget::Both,
    };
    RuleAction::Breakpoint(target)
}
```

- [ ] **Step 3: 更新 RuleEntry 序列化**

在 `RulesEngine::rule_to_entry()` 中添加：

```rust
target: match &r.action {
    RuleAction::MapRemote(t) => Some(t.clone()),
    RuleAction::MapLocal(t) => Some(t.clone()),
    RuleAction::Breakpoint(t) => Some(format!("{:?}", t)),
    _ => None,
},
```

- [ ] **Step 4: Build to verify**

Run: `cargo build --lib`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/rules.rs
git commit -m "feat(breakpoint): add RuleAction::Breakpoint variant"
```

---

## Task 4: Traffic tab Detail Panel 断点编辑器渲染

**Files:**
- Modify: `src-tauri/src/tui/render/traffic.rs`

- [ ] **Step 1: 添加 render_breakpoint_editor 函数**

在 `render_traffic` 函数中，当 `app.traffic.breakpoint.mode != BreakpointMode::None` 时，调用 `render_breakpoint_editor`：

```rust
fn render_breakpoint_editor(f: &mut Frame, area: Rect, app: &mut TuiApp) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::{Block, Borders, Paragraph};
    use ratatui::style::Color;

    let bp = &app.traffic.breakpoint;
    let req = match &bp.current_edit {
        Some(r) => r,
        None => return,
    };

    let modal_width = 60.min(area.width.saturating_sub(4));
    let modal_height = 20.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    let mode_label = match bp.mode {
        BreakpointMode::RequestPaused => "REQUEST BREAKPOINT",
        BreakpointMode::ResponsePaused => "RESPONSE BREAKPOINT",
        _ => return,
    };

    let lines = vec![
        format!("  {} — press [g] to GO, [c] to CANCEL, [e] to EDIT", mode_label),
        format!(""),
        format!("  Method:  {}", req.method),
        format!("  URL:    {}://{}{}", req.scheme, req.host, req.path),
        format!("  Status: {:?}", req.status),
        format!("  Headers: ({} pairs)", req.req_headers.len()),
        for (k, v) in req.req_headers.iter().take(3) {
            format!("    {}: {}", k, v);
        }
        if req.req_headers.len() > 3 {
            format!("    ... and {} more", req.req_headers.len() - 3);
        }
        format!(""),
        format!("  Body: {}", req.req_body.as_ref().map(|s| s.chars().take(100).collect::<String>().unwrap_or_default()).unwrap_or_else(|| "(empty)".to_string())),
    ];

    let content = Paragraph::new(lines.join("\n"))
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", mode_label))
            .border_style(ratatui::style::Style::new().fg(Color::Cyan)))
        .alignment(Alignment::Left);

    f.render_widget(content, modal_area);
}
```

- [ ] **Step 2: 修改 render 函数入口**

在 `render_content` 函数中，当 breakpoint mode != None 时，调用 `render_breakpoint_editor` 而非正常的 detail panel：

```rust
fn render_content(f: &mut Frame, area: Rect, app: &mut TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(area);

    render_request_list(f, chunks[0], app);

    // If breakpoint is active, show breakpoint editor instead of normal detail
    if app.traffic.breakpoint.mode != BreakpointMode::None {
        render_breakpoint_editor(f, chunks[1], app);
    } else {
        render_detail_panel(f, chunks[1], app);
    }
}
```

- [ ] **Step 3: Build to verify**

Run: `cargo build --bin proxybot-tui 2>&1 | head -30`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/tui/render/traffic.rs
git commit -m "feat(breakpoint): add breakpoint editor rendering in detail panel"
```

---

## Task 5: TUI 主循环处理 Breakpoint 动作

**Files:**
- Modify: `src-tauri/src/bin/proxybot-tui.rs`

- [ ] **Step 1: 添加 breakpoint 状态到 TuiApp**

由于 TuiApp 已有 `traffic: TrafficState`，BreakpointState 是其字段。确保 `TrafficState` 包含 `breakpoint: BreakpointState`。

检查 `src-tauri/src/tui/mod.rs` 中的 `TrafficState` 结构体：

```rust
pub struct TrafficState {
    pub requests: Vec<RecentRequest>,
    pub selected: usize,
    pub last_id: i64,
    // Filters
    pub filters: TrafficFilters,
    // ... existing fields ...
    // 新增
    pub breakpoint: BreakpointState,
}
```

- [ ] **Step 2: 在 main 事件循环中处理 ToggleBreakpoint**

在 `handle_key_event` 的 `InputAction` 分支中添加：

```rust
InputAction::ToggleBreakpoint => {
    if app.current_tab == Tab::Traffic {
        let filtered = app.traffic.filtered_requests();
        if !filtered.is_empty() {
            let selected = app.traffic.selected.min(filtered.len().saturating_sub(1));
            let req = filtered[selected];
            // 克隆请求加入断点队列
            let intercepted = InterceptedRequest {
                id: req.id.to_string(),
                timestamp: req.timestamp.clone(),
                method: req.method.clone(),
                host: req.host.clone(),
                path: req.path.clone(),
                scheme: req.scheme.clone(),
                ..Default::default()
            };
            app.traffic.breakpoint.queue.push(intercepted);
            if app.traffic.breakpoint.current_edit.is_none() {
                app.traffic.breakpoint.current_edit = app.traffic.breakpoint.queue.first().cloned();
                app.traffic.breakpoint.mode = BreakpointMode::RequestPaused;
            }
        }
    }
}
```

- [ ] **Step 3: 处理 BreakpointGo**

```rust
InputAction::BreakpointGo => {
    // 从队列中移除当前请求，继续处理
    if !app.traffic.breakpoint.queue.is_empty() {
        app.traffic.breakpoint.queue.remove(0);
    }
    if let Some(next) = app.traffic.breakpoint.queue.first() {
        app.traffic.breakpoint.current_edit = Some(next.clone());
        app.traffic.breakpoint.mode = BreakpointMode::RequestPaused;
    } else {
        app.traffic.breakpoint.current_edit = None;
        app.traffic.breakpoint.mode = BreakpointMode::None;
    }
}
```

- [ ] **Step 4: 处理 BreakpointCancel**

```rust
InputAction::BreakpointCancel => {
    // 取消所有队列中的请求
    app.traffic.breakpoint.queue.clear();
    app.traffic.breakpoint.current_edit = None;
    app.traffic.breakpoint.mode = BreakpointMode::None;
}
```

- [ ] **Step 5: Build to verify**

Run: `cargo build --bin proxybot-tui 2>&1 | head -30`
Expected: SUCCESS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/bin/proxybot-tui.rs
git commit -m "feat(breakpoint): handle ToggleBreakpoint/Go/Cancel in main loop"
```

---

## Task 6: 测试 Breakpoint 状态转换

**Files:**
- Modify: `src-tauri/src/tui/mod.rs`

- [ ] **Step 1: 添加状态转换测试**

在 `src-tauri/src/tui/mod.rs` 的 tests 模块中添加：

```rust
#[test]
fn test_breakpoint_toggle_adds_to_queue() {
    let mut state = TrafficState::default();
    let req = RecentRequest {
        id: 1,
        timestamp: "1".to_string(),
        method: "GET".to_string(),
        scheme: "https".to_string(),
        host: "example.com".to_string(),
        path: "/".to_string(),
        status: Some(200),
        duration_ms: Some(100),
        app_tag: None,
    };
    state.requests.push(req);

    // Simulate adding to breakpoint queue
    let intercepted = InterceptedRequest {
        id: "1".to_string(),
        timestamp: "1".to_string(),
        method: "GET".to_string(),
        host: "example.com".to_string(),
        path: "/".to_string(),
        scheme: "https".to_string(),
        ..Default::default()
    };
    state.breakpoint.queue.push(intercepted);
    state.breakpoint.current_edit = state.breakpoint.queue.first().cloned();
    state.breakpoint.mode = BreakpointMode::RequestPaused;

    assert_eq!(state.breakpoint.queue.len(), 1);
    assert_eq!(state.breakpoint.mode, BreakpointMode::RequestPaused);
}

#[test]
fn test_breakpoint_go_clears_current() {
    let mut state = BreakpointState::default();
    let req = InterceptedRequest {
        id: "1".to_string(),
        timestamp: "1".to_string(),
        method: "GET".to_string(),
        host: "example.com".to_string(),
        path: "/".to_string(),
        scheme: "https".to_string(),
        ..Default::default()
    };
    state.queue.push(req.clone());
    state.current_edit = Some(req);
    state.mode = BreakpointMode::RequestPaused;

    // Simulate GO: remove first item
    if !state.queue.is_empty() {
        state.queue.remove(0);
    }
    state.current_edit = state.queue.first().cloned();
    if state.current_edit.is_none() {
        state.mode = BreakpointMode::None;
    }

    assert!(state.queue.is_empty());
    assert!(state.current_edit.is_none());
    assert_eq!(state.mode, BreakpointMode::None);
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --lib -- breakpoint`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/tui/mod.rs
git commit -m "test(breakpoint): add state transition tests"
```

---

## Task 7: 集成测试 - 手动验证

**Files:**
- None (manual test)

- [ ] **Step 1: 构建并运行 TUI**

```bash
cd src-tauri
cargo build --bin proxybot-tui --release
./target/release/proxybot-tui
```

- [ ] **Step 2: 启动代理**

按 `r` 启动代理

- [ ] **Step 3: 添加一条 BREAKPOINT 规则**

在 Rules tab，按 `a` 添加规则：
- Pattern: DOMAIN-SUFFIX
- Value: example.com
- Action: BREAKPOINT

- [ ] **Step 4: 发送请求触发断点**

用手机访问 example.com，观察是否进入断点模式

- [ ] **Step 5: 测试 Go/Cancel**

按 `g` 继续，按 `c` 取消

---

## 实现检查清单

| 任务 | 状态 |
|------|------|
| Task 1: BreakpointState 类型 | ⬜ |
| Task 2: InputAction 断点动作 | ⬜ |
| Task 3: RuleAction::Breakpoint | ⬜ |
| Task 4: Detail Panel 渲染 | ⬜ |
| Task 5: 主循环处理 | ⬜ |
| Task 6: 单元测试 | ⬜ |
| Task 7: 手动验证 | ⬜ |
