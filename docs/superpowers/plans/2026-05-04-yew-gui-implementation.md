# ProxyBot Yew GUI 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 使用 Yew (Rust WASM) 重写 GUI，移除 Node.js 依赖，实现全部 9 个功能模块

**Architecture:** Tauri v2 后端 + Yew 前端编译为 WASM，通过 Tauri invoke/emit 与 Rust 后端通信

**Tech Stack:** Yew 0.21, wasm-pack, stylist, Tauri v2

---

## 文件结构

```
src-tauri/src/gui/
├── lib.rs                 # Yew 应用根组件
├── main.rs                # WASM 入口
├── app.rs                 # 主应用（App 结构）
├── components/
│   ├── mod.rs
│   ├── traffic/
│   │   ├── mod.rs
│   │   ├── component.rs   # Traffic 主组件
│   │   └── types.rs       # Traffic 相关类型
│   ├── rules/
│   ├── devices/
│   ├── certs/
│   ├── dns/
│   ├── alerts/
│   ├── replay/
│   ├── graph/
│   └── gen/
├── hooks/                 # 共享 hooks
│   ├── mod.rs
│   ├── tauri.rs           # invoke 调用封装
│   └── state.rs           # 状态管理
├── i18n/
│   ├── mod.rs
│   ├── en.rs              # 英文翻译
│   └── zh.rs              # 中文翻译
└── styles/
    └── mod.rs             # 共享样式
```

---

## Task 1: 项目基础设置

**Files:**
- Create: `src-tauri/src/gui/lib.rs`
- Create: `src-tauri/src/gui/main.rs`
- Create: `src-tauri/src/gui/app.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add Yew dependencies to Cargo.toml**

在 `[dependencies]` 后添加：

```toml
yew = "0.21"
wasm-bindgen = "0.2"
 stylist = "0.2"
serde_json = "1"
serde = { version = "1", features = ["derive"] }

[target.wasm32-unknown-unknown]
rustflags = "-C target-feature=+atomics,+bulk-memory"
```

- [ ] **Step 2: Create lib.rs - Yew 应用根**

```rust
use yew::prelude::*;

mod components;
mod hooks;
mod i18n;
mod styles;

use app::App;

#[wasm_bindgen::start]
pub fn main() {
    yew::Renderer::<App>::new().mount();
}
```

- [ ] **Step 3: Create main.rs - WASM 入口**

```rust
use yew::prelude::*;
use stylist::yew::styled_component;
use crate::app::App;

#[styled_component(App)]
pub fn app() -> Html {
    html! {
        <div class="app-container">
            <App />
        </div>
    }
}
```

- [ ] **Step 4: Create app.rs - 主应用组件**

```rust
use yew::prelude::*;
use crate::components::traffic::TrafficTab;
use crate::components::rules::RulesTab;
use crate::components::devices::DevicesTab;
use crate::hooks::tauri::use_invoke;

#[derive(Clone, PartialEq)]
enum Tab {
    Traffic, Rules, Devices, Certs, Dns, Alerts, Replay, Graph, Gen,
}

#[function_component(App)]
pub fn app() -> Html {
    let selected_tab = use_state(|| Tab::Traffic);

    let tab_labels = {
        let mut m = std::collections::HashMap::new();
        m.insert(Tab::Traffic, "Traffic");
        m.insert(Tab::Rules, "Rules");
        m.insert(Tab::Devices, "Devices");
        m.insert(Tab::Certs, "Certs");
        m.insert(Tab::Dns, "DNS");
        m.insert(Tab::Alerts, "Alerts");
        m.insert(Tab::Replay, "Replay");
        m.insert(Tab::Graph, "Graph");
        m.insert(Tab::Gen, "Gen");
        m
    };

    html! {
        <div class="app">
            <nav class="tab-nav">
                { for tab_labels.iter().map(|(tab, label)| {
                    let active = *selected_tab == *tab;
                    let tab = tab.clone();
                    html! {
                        <button
                            class={if active { "tab active" } else { "tab" }}
                            onclick={Callback::from(move |_| selected_tab.set(tab.clone()))}
                        >
                            { label }
                        </button>
                    }
                })}
            </nav>
            <main class="content">
                { match *selected_tab {
                    Tab::Traffic => html! { <TrafficTab /> },
                    Tab::Rules => html! { <RulesTab /> },
                    Tab::Devices => html! { <DevicesTab /> },
                    _ => html! { <div> { format!("{:?} tab", *selected_tab) } </div> },
                }}
            </main>
        </div>
    }
}
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(gui-yew): add Yew project skeleton"
```

---

## Task 2: Traffic 组件

**Files:**
- Create: `src-tauri/src/gui/components/mod.rs`
- Create: `src-tauri/src/gui/components/traffic/mod.rs`
- Create: `src-tauri/src/gui/components/traffic/component.rs`
- Create: `src-tauri/src/gui/components/traffic/types.rs`
- Create: `src-tauri/src/gui/hooks/mod.rs`
- Create: `src-tauri/src/gui/hooks/tauri.rs`

- [ ] **Step 1: Create components/mod.rs**

```rust
pub mod traffic;
pub mod rules;
pub mod devices;
pub mod certs;
pub mod dns;
pub mod alerts;
pub mod replay;
pub mod graph;
pub mod gen;
```

- [ ] **Step 2: Create hooks/mod.rs and hooks/tauri.rs**

```rust
// hooks/mod.rs
pub mod tauri;
pub mod state;
```

```rust
// hooks/tauri.rs
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

pub async fn invoke<T: Serialize, R: for<'de> Deserialize<'de>>(cmd: &str, args: Option<T>) -> Result<R, String> {
    let args = args.map(|a| serde_json::to_value(a).unwrap_or(JsValue::NULL)).unwrap_or(JsValue::NULL);
    let result = invoke(cmd, args).await.map_err(|e| format!("{:?}", e))?;
    serde_json::from_value(result).map_err(|e| format!("{:?}", e))
}
```

- [ ] **Step 3: Create traffic/types.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: u64,
    pub size_bytes: u64,
    pub app: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficStats {
    pub total_requests: u64,
    pub bytes_up: u64,
    pub bytes_down: u64,
}
```

- [ ] **Step 4: Create traffic/component.rs**

```rust
use yew::prelude::*;
use crate::hooks::tauri::invoke;
use super::types::{Request, TrafficStats};

#[function_component(TrafficTab)]
pub fn traffic_tab() -> Html {
    let requests = use_state(Vec::<Request>::new);
    let filter_text = use_state(String::new);
    let selected_request = use_state(Option::<Request>::new);

    // Load requests on mount
    use_effect_with_deps(move |_| {
        // TODO: invoke("get_requests", None::<()>) to load traffic
        ()
    }, ());

    let filtered_requests = {
        let requests = requests.clone();
        let filter = (*filter_text).clone();
        move || {
            if filter.is_empty() {
                requests.clone()
            } else {
                requests.clone().into_iter().filter(|r| {
                    r.host.contains(&filter) || r.path.contains(&filter)
                }).collect()
            }
        }
    };

    html! {
        <div class="traffic-tab">
            <div class="filter-bar">
                <input
                    type="text"
                    placeholder="Filter by host or path..."
                    value={(*filter_text).clone()}
                    oninput={Callback::from(move |e: InputEvent| {
                        let target = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap();
                        filter_text.set(target.value());
                    })}
                />
                <button onclick={Callback::from(move |_| filter_text.set(String::new()))}>
                    {"Clear"}
                </button>
            </div>
            <div class="request-list">
                { for filtered_requests().iter().map(|req| {
                    let req_clone = req.clone();
                    let selected = selected_request.clone();
                    let req_id = req.id.clone();
                    html! {
                        <div
                            class="request-item"
                            onclick={Callback::from(move |_| selected.set(Some(req_clone.clone())))}
                        >
                            <span class="method">{ req.method }</span>
                            <span class="host">{ req.host }</span>
                            <span class="path">{ req.path }</span>
                            <span class="status">{ req.status }</span>
                        </div>
                    }
                })}
            </div>
            if let Some(ref req) = *selected_request {
                <div class="request-detail">
                    <h3>{ format!("{} {} {}", req.method, req.host, req.path) }</h3>
                    <pre>{ serde_json::to_string_pretty(req).unwrap_or_default() }</pre>
                </div>
            }
        </div>
    }
}
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(gui-yew): add Traffic tab component"
```

---

## Task 3: Rules 组件

**Files:**
- Create: `src-tauri/src/gui/components/rules/mod.rs`
- Create: `src-tauri/src/gui/components/rules/component.rs`

- [ ] **Step 1: Create rules/types.rs** (or inline in component.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub filter: String,
    pub action: RuleAction,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    Direct,
    Proxy,
    Reject,
    MapRemote(String),
    MapLocal(String),
}
```

- [ ] **Step 2: Create rules/component.rs**

```rust
use yew::prelude::*;

#[function_component(RulesTab)]
pub fn rules_tab() -> Html {
    html! {
        <div class="rules-tab">
            <h2>{"Rules"}</h2>
            <p>{"Rules management panel"}</p>
        </div>
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(gui-yew): add Rules tab component"
```

---

## Task 4-9: Devices, Certs, DNS, Alerts, Replay, Graph, Gen 组件

类似 Task 2-3，每个模块创建 `components/<module>/mod.rs` 和 `component.rs`

**各组件占位实现：**

```rust
#[function_component(<Module>Tab)]
pub fn <module>_tab() -> Html {
    html! {
        <div class="{<module>}-tab">
            <h2>{stringify!(<Module>)}</h2>
            <p>{format!("{:?} tab placeholder", stringify!(<Module>))}</p>
        </div>
    }
}
```

- [ ] **Task 4:** Devices tab
- [ ] **Task 5:** Certs tab
- [ ] **Task 6:** DNS tab
- [ ] **Task 7:** Alerts tab
- [ ] **Task 8:** Replay tab
- [ ] **Task 9:** Graph tab
- [ ] **Task 10:** Gen tab

每个 Task 完成后单独 commit。

---

## Task 11: 更新 App 组件路由所有 Tab

**Files:**
- Modify: `src-tauri/src/gui/app.rs`

- [ ] **Step 1: Update match to route all 9 tabs**

```rust
match *selected_tab {
    Tab::Traffic => html! { <TrafficTab /> },
    Tab::Rules => html! { <RulesTab /> },
    Tab::Devices => html! { <DevicesTab /> },
    Tab::Certs => html! { <CertsTab /> },
    Tab::Dns => html! { <DnsTab /> },
    Tab::Alerts => html! { <AlertsTab /> },
    Tab::Replay => html! { <ReplayTab /> },
    Tab::Graph => html! { <GraphTab /> },
    Tab::Gen => html! { <GenTab /> },
}
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "feat(gui-yew): route all 9 tabs in App component"
```

---

## Task 12: 样式基础

**Files:**
- Create: `src-tauri/src/gui/styles/mod.rs`
- Modify: `src-tauri/src/gui/app.rs` (添加基础 CSS)

- [ ] **Step 1: Create styles/mod.rs with stylist**

```rust
use stylist::style;

pub fn app_container_style() -> style {
    style!("width: 100%; height: 100vh; display: flex; flex-direction: column;")
        .unwrap()
}

pub fn tab_nav_style() -> style {
    style!("display: flex; gap: 4px; padding: 8px; background: #1a1a1a;")
        .unwrap()
}

pub fn tab_style(active: bool) -> style {
    let base = "padding: 8px 16px; border: none; cursor: pointer;";
    let active_style = if active { "background: #333; color: white;" } else { "background: #222; color: #888;" };
    style!(format!("{}{}", base, active_style)).unwrap()
}
```

- [ ] **Step 2: Apply styles in app.rs**

```rust
use crate::styles::{app_container_style, tab_nav_style, tab_style};

#[styled_component(App)]
pub fn app() -> Html {
    // ... use styled_component decorator and apply styles
}
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(gui-yew): add base styles with stylist"
```

---

## Task 13: 更新 proxybot-gui.rs 入口

**Files:**
- Modify: `src-tauri/src/bin/proxybot-gui.rs`

- [ ] **Step 1: Update to load Yew WASM instead of React**

```rust
// 移除 frontendDist 配置，或指向编译后的 WASM 输出
// tauri.conf.json build.frontedDist 改为 wasm 输出目录
```

- [ ] **Step 2: 确保 invoke handlers 正确注册**

```rust
// 保持现有 invoke handlers 不变
.invoke_handler(tauri::generate_handler![
    proxybot_lib::proxy::start_proxy,
    proxybot_lib::proxy::stop_proxy,
    // ... all existing handlers
])
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(gui-yew): update proxybot-gui entry for Yew WASM"
```

---

## Task 14: wasm-pack 构建配置

**Files:**
- Create: `src-tauri/pkg/` (wasm-pack 输出)
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Update tauri.conf.json**

```json
{
  "build": {
    "devUrl": "http://localhost:8080",
    "frontendDist": "../pkg",
    "devtools": true
  }
}
```

- [ ] **Step 2: 添加构建脚本到 package.json 或 Makefile**

```makefile
build-gui:
    cd src-tauri && wasm-pack build --target web --out-dir pkg
    cd src-tauri && cargo build --bin proxybot-gui --release
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(gui-yew): add wasm-pack build config"
```

---

## Task 15: 集成测试和 CI

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: 添加 wasm-pack 安装到 CI**

```yaml
- name: Install wasm-pack
  run: |
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

- name: Build GUI
  run: |
    cd src-tauri
    wasm-pack build --target web --out-dir pkg
    cargo build --bin proxybot-gui --release
```

- [ ] **Step 2: Commit**

```bash
git add -A
git commit -m "ci: add Yew GUI build to CI"
```

---

## 实现检查清单

| Task | 模块 | 状态 |
|------|------|------|
| Task 1 | 项目基础设置 | ⬜ |
| Task 2 | Traffic | ⬜ |
| Task 3 | Rules | ⬜ |
| Task 4 | Devices | ⬜ |
| Task 5 | Certs | ⬜ |
| Task 6 | DNS | ⬜ |
| Task 7 | Alerts | ⬜ |
| Task 8 | Replay | ⬜ |
| Task 9 | Graph | ⬜ |
| Task 10 | Gen | ⬜ |
| Task 11 | App 路由 | ⬜ |
| Task 12 | 样式 | ⬜ |
| Task 13 | proxybot-gui.rs | ⬜ |
| Task 14 | wasm-pack 配置 | ⬜ |
| Task 15 | CI 集成 | ⬜ |