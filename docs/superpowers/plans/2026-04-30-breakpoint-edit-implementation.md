# Breakpoint 编辑功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现 Breakpoint 编辑功能，支持用户编辑 method、url、headers、body。

**Architecture:** 扩展 BreakpointState，新增 BreakpointEditMode 和 BreakpointField 枚举，支持方向键导航和弹出式编辑框。

**Tech Stack:** Rust, ratatui, crossterm

---

## 文件结构

```
src-tauri/src/
├── tui/
│   └── mod.rs           # BreakpointEditMode, BreakpointField, 编辑状态
└── tui/render/
    └── traffic.rs       # render_breakpoint_editor 修改，支持编辑模式渲染
└── bin/
    └── proxybot-tui.rs  # 主循环处理 BreakpointEdit 动作
```

---

## Task 1: 添加编辑模式类型

**Files:**
- Modify: `src-tauri/src/tui/mod.rs`

- [ ] **Step 1: 添加 BreakpointEditMode 和 BreakpointField 枚举**

在 `BreakpointMode` 后添加：

```rust
/// Breakpoint 编辑模式
#[derive(Clone, PartialEq, Eq)]
pub enum BreakpointEditMode {
    None,           // 非编辑模式（正常查看）
    Editing(usize), // 编辑模式（usize = 选中字段索引）
}

/// 可编辑的字段类型
#[derive(Clone, PartialEq, Eq)]
pub enum BreakpointField {
    Method,   // 索引 0
    Url,      // 索引 1
    Headers,  // 索引 2
    Body,     // 索引 3
}
```

- [ ] **Step 2: 扩展 BreakpointState 添加编辑相关字段**

在 `BreakpointState` 中添加：

```rust
pub struct BreakpointState {
    pub mode: BreakpointMode,
    pub edit_mode: BreakpointEditMode,              // 新增
    pub selected_field: BreakpointField,              // 新增
    pub editing_header_index: Option<usize>,          // 新增：正在编辑的 header 行索引
    pub header_input: String,                        // 新增：header 编辑输入缓冲
    pub body_input: String,                          // 新增：body 编辑输入缓冲
    pub url_input: String,                           // 新增：url 编辑输入缓冲
    pub method_input: String,                        // 新增：method 编辑输入缓冲
    // ... 现有字段
    pub queue: Vec<InterceptedRequest>,
    pub current_edit: Option<InterceptedRequest>,
}
```

- [ ] **Step 3: 初始化默认值**

在 `BreakpointState::default()` 中确保新字段有默认值：

```rust
impl Default for BreakpointState {
    fn default() -> Self {
        Self {
            mode: BreakpointMode::None,
            edit_mode: BreakpointEditMode::None,  // 新增
            selected_field: BreakpointField::Method,  // 新增
            editing_header_index: None,  // 新增
            header_input: String::new(),  // 新增
            body_input: String::new(),  // 新增
            url_input: String::new(),  // 新增
            method_input: String::new(),  // 新增
            // ... 现有
            queue: Vec::new(),
            current_edit: None,
        }
    }
}
```

- [ ] **Step 4: Build 验证**

Run: `cargo build --lib`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/tui/mod.rs
git commit -m "feat(breakpoint): add BreakpointEditMode and BreakpointField types"
```

---

## Task 2: 修改 proxybot-tui.rs 处理 BreakpointEdit

**Files:**
- Modify: `src-tauri/src/bin/proxybot-tui.rs`

- [ ] **Step 1: 更新 BreakpointEdit 处理**

找到现有的 `InputAction::BreakpointEdit` 分支（约 line 776），替换为：

```rust
InputAction::BreakpointEdit => {
    use proxybot_lib::tui::{BreakpointEditMode, BreakpointField};
    if app.traffic.breakpoint.mode != proxybot_lib::tui::BreakpointMode::None {
        // 进入编辑模式
        app.traffic.breakpoint.edit_mode = BreakpointEditMode::Editing(0);
        app.traffic.breakpoint.selected_field = BreakpointField::Method;
        // 初始化输入缓冲
        if let Some(ref req) = app.traffic.breakpoint.current_edit {
            app.traffic.breakpoint.method_input = req.method.clone();
            app.traffic.breakpoint.url_input = format!("{}://{}{}", req.scheme, req.host, req.path);
            app.traffic.breakpoint.body_input = req.req_body.clone().unwrap_or_default();
        }
    }
}
```

- [ ] **Step 2: 添加方向键导航处理**

在 `handle_key_event` 中，BreakpointEdit 模式下方向键需要单独处理。因为 `handle_key_event` 不访问 `app`，需要在主循环中添加对 `edit_mode` 的检测：

在主循环中 `InputAction::BreakpointEdit` 处理后添加：

```rust
// Breakpoint 编辑模式下的方向键导航
if app.traffic.breakpoint.edit_mode != BreakpointEditMode::None {
    use proxybot_lib::tui::BreakpointField;
    match key.code {
        crossterm::event::KeyCode::Up => {
            let current = app.traffic.breakpoint.selected_field.clone();
            let next = match current {
                BreakpointField::Method => BreakpointField::Body,
                BreakpointField::Url => BreakpointField::Method,
                BreakpointField::Headers => BreakpointField::Url,
                BreakpointField::Body => BreakpointField::Headers,
            };
            app.traffic.breakpoint.selected_field = next;
        }
        crossterm::event::KeyCode::Down => {
            let current = app.traffic.breakpoint.selected_field.clone();
            let next = match current {
                BreakpointField::Method => BreakpointField::Url,
                BreakpointField::Url => BreakpointField::Headers,
                BreakpointField::Headers => BreakpointField::Body,
                BreakpointField::Body => BreakpointField::Method,
            };
            app.traffic.breakpoint.selected_field = next;
        }
        _ => {}
    }
}
```

注意：这个方向键导航代码需要在 `handle_key_event` 调用的外面，因为 `handle_key_event` 只返回 Action，不处理状态。

- [ ] **Step 3: 添加 Enter 处理编辑确认**

在方向键处理后添加 Enter 处理：

```rust
crossterm::event::KeyCode::Enter => {
    if app.traffic.breakpoint.edit_mode != BreakpointEditMode::None {
        use proxybot_lib::tui::BreakpointField;
        match app.traffic.breakpoint.selected_field {
            BreakpointField::Method => {
                // Method 编辑 - 弹出方法选择器（GET/POST/PUT/DELETE/PATCH/HEAD/OPTIONS）
                // 简单起见：循环切换
                let methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
                let current = &app.traffic.breakpoint.method_input;
                if let Some(idx) = methods.iter().position(|m| m == current) {
                    let next = methods[(idx + 1) % methods.len()];
                    app.traffic.breakpoint.method_input = next.to_string();
                    if let Some(ref mut req) = app.traffic.breakpoint.current_edit {
                        req.method = next.to_string();
                    }
                }
            }
            BreakpointField::Url => {
                // URL 编辑 - 激活 URL 输入模式（类似 filter_input）
                // 先简单处理：直接进入 URL 编辑
            }
            BreakpointField::Headers => {
                // Headers 编辑
                if let Some(idx) = app.traffic.breakpoint.editing_header_index {
                    // 确认当前 header 编辑
                    if let Some(ref mut req) = app.traffic.breakpoint.current_edit {
                        if idx < req.req_headers.len() {
                            let parts: Vec<&str> = app.traffic.breakpoint.header_input.splitn(2, ": ").collect();
                            if parts.len() == 2 {
                                req.req_headers[idx].0 = parts[0].to_string();
                                req.req_headers[idx].1 = parts[1].to_string();
                            }
                        }
                    }
                    app.traffic.breakpoint.editing_header_index = None;
                    app.traffic.breakpoint.header_input.clear();
                } else {
                    // 开始编辑第一个 header
                    app.traffic.breakpoint.editing_header_index = Some(0);
                    if let Some(ref req) = app.traffic.breakpoint.current_edit {
                        if !req.req_headers.is_empty() {
                            let (k, v) = &req.req_headers[0];
                            app.traffic.breakpoint.header_input = format!("{}: {}", k, v);
                        }
                    }
                }
            }
            BreakpointField::Body => {
                // Body 编辑
                if let Some(ref mut req) = app.traffic.breakpoint.current_edit {
                    req.req_body = Some(app.traffic.breakpoint.body_input.clone());
                }
            }
        }
    }
}
```

- [ ] **Step 4: 添加字符输入支持**

在 Enter 处理后添加字符输入（用于 URL 和 header 编辑）：

```rust
crossterm::event::KeyCode::Char(c) => {
    if app.traffic.breakpoint.edit_mode != BreakpointEditMode::None {
        use proxybot_lib::tui::BreakpointField;
        // 只有 Url 和 Headers 模式下才捕获字符
        match app.traffic.breakpoint.selected_field {
            BreakpointField::Url => {
                app.traffic.breakpoint.url_input.push(c);
                if let Some(ref mut req) = app.traffic.breakpoint.current_edit {
                    // 解析 URL 更新 scheme/host/path
                    // 简化：不做解析，直接存 url_input
                }
            }
            BreakpointField::Headers => {
                if app.traffic.breakpoint.editing_header_index.is_some() {
                    app.traffic.breakpoint.header_input.push(c);
                }
            }
            BreakpointField::Body => {
                app.traffic.breakpoint.body_input.push(c);
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 5: 添加 Backspace 支持**

```rust
crossterm::event::KeyCode::Backspace => {
    if app.traffic.breakpoint.edit_mode != BreakpointEditMode::None {
        use proxybot_lib::tui::BreakpointField;
        match app.traffic.breakpoint.selected_field {
            BreakpointField::Url => { app.traffic.breakpoint.url_input.pop(); }
            BreakpointField::Headers => { app.traffic.breakpoint.header_input.pop(); }
            BreakpointField::Body => { app.traffic.breakpoint.body_input.pop(); }
            _ => {}
        }
    }
}
```

- [ ] **Step 6: 修改 BreakpointGo 发送当前编辑**

更新 `InputAction::BreakpointGo` 确保发送的是 current_edit：

```rust
InputAction::BreakpointGo => {
    use proxybot_lib::tui::BreakpointMode;
    // 编辑模式下先退出编辑
    app.traffic.breakpoint.edit_mode = BreakpointEditMode::None;
    // ... 现有逻辑
}
```

- [ ] **Step 7: Build 验证**

Run: `cargo build --bin proxybot-tui 2>&1 | head -30`
Expected: SUCCESS

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/bin/proxybot-tui.rs
git commit -m "feat(breakpoint): handle BreakpointEdit with direction key navigation"
```

---

## Task 3: 修改渲染器支持编辑模式

**Files:**
- Modify: `src-tauri/src/tui/render/traffic.rs`

- [ ] **Step 1: 更新 render_breakpoint_editor 支持编辑模式**

替换现有的 `render_breakpoint_editor` 函数：

```rust
fn render_breakpoint_editor(f: &mut Frame, area: Rect, app: &TuiApp) {
    use ratatui::layout::Alignment;
    use ratatui::widgets::{Block, Borders, Paragraph};
    use ratatui::style::Color;
    use crate::tui::{BreakpointMode, BreakpointEditMode, BreakpointField};

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

    // 检查是否是编辑模式
    let is_editing = !matches!(bp.edit_mode, BreakpointEditMode::None);

    let mode_label = match bp.mode {
        BreakpointMode::RequestPaused => "REQUEST BREAKPOINT",
        BreakpointMode::ResponsePaused => "RESPONSE BREAKPOINT",
        _ => return,
    };

    let edit_indicator = if is_editing { "[EDIT]" } else { "" };
    let help_text = if is_editing {
        "[↑/↓] field  [Enter] edit  [g] send  [Esc] cancel"
    } else {
        "[e] edit  [g] send  [c] cancel"
    };

    let mut lines: Vec<String> = vec![
        format!("  {} {} — {}", mode_label, edit_indicator, help_text),
        String::new(),
    ];

    // Method 行
    let method_str = if is_editing && matches!(bp.selected_field, BreakpointField::Method) {
        format!("> Method:   [{}]", bp.method_input)
    } else {
        format!("  Method:   {}", req.method)
    };
    lines.push(method_str);

    // URL 行
    let url_display = if is_editing && matches!(bp.selected_field, BreakpointField::Url) {
        format!("> URL:    [{}]", bp.url_input)
    } else {
        format!("  URL:    {}://{}{}", req.scheme, req.host, req.path)
    };
    lines.push(url_display);

    // Headers
    lines.push(String::new());
    lines.push(format!("  Headers: ({})", req.req_headers.len()));
    for (i, (k, v)) in req.req_headers.iter().enumerate() {
        let is_selected = is_editing && matches!(bp.selected_field, BreakpointField::Headers);
        let is_editing_this = is_editing && bp.editing_header_index == Some(i);
        let prefix = if is_selected { "> " } else { "  " };
        let editing_indicator = if is_editing_this { "[EDITING]" } else { "" };
        let line = format!("{}{}{}: {}", prefix, editing_indicator, k, v);
        lines.push(line);
    }

    // Body
    lines.push(String::new());
    let body_preview = req.req_body.as_ref()
        .map(|s| s.chars().take(60).collect::<String>())
        .unwrap_or_else(|| "(empty)".to_string());
    let body_str = if is_editing && matches!(bp.selected_field, BreakpointField::Body) {
        format!("> Body:   [{}...]", bp.body_input.chars().take(30).collect::<String>())
    } else {
        format!("  Body:   {}", body_preview)
    };
    lines.push(body_str);

    // Header 编辑弹出框
    if let Some(idx) = bp.editing_header_index {
        let header_line = format!("\n  [Editing header {}: {}]  [Enter] confirm  [Esc] cancel", idx, bp.header_input);
        lines.push(header_line);
    }

    let content = Paragraph::new(lines.join("\n"))
        .block(Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", mode_label))
            .border_style(Style::new().fg(Color::Cyan)))
        .alignment(Alignment::Left);

    f.render_widget(content, modal_area);
}
```

- [ ] **Step 2: 处理 Esc 退出编辑**

在主循环的 key 处理中，在 BreakpointEdit 后添加：

```rust
crossterm::event::KeyCode::Esc => {
    if app.traffic.breakpoint.edit_mode != BreakpointEditMode::None {
        // 退出编辑模式
        app.traffic.breakpoint.edit_mode = BreakpointEditMode::None;
        app.traffic.breakpoint.editing_header_index = None;
        app.traffic.breakpoint.header_input.clear();
    }
}
```

- [ ] **Step 3: Build 验证**

Run: `cargo build --bin proxybot-tui 2>&1 | head -30`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/tui/render/traffic.rs
git commit -m "feat(breakpoint): render editor with direction key navigation and edit mode"
```

---

## Task 4: 集成测试

- [ ] **Step 1: 构建并运行**

```bash
cd src-tauri
cargo build --bin proxybot-tui --release
./target/release/proxybot-tui
```

- [ ] **Step 2: 测试流程**

1. 启动代理 (r)
2. 看到请求后选中
3. 按 b 进入断点
4. 按 e 进入编辑模式
5. 方向键 ↑/↓ 选择字段
6. Enter 编辑 method（循环切换）
7. 方向键选择 Headers，按 Enter 开始编辑 header
8. 输入新值，Enter 确认
9. g 发送修改后的请求

---

## 实现检查清单

| 任务 | 状态 |
|------|------|
| Task 1: 编辑模式类型 | ⬜ |
| Task 2: 主循环处理 | ⬜ |
| Task 3: 渲染器编辑支持 | ⬜ |
| Task 4: 集成测试 | ⬜ |
