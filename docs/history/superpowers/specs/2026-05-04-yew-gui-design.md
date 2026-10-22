# ProxyBot Yew GUI 实现设计

> **状态:** 已批准
> **日期:** 2026-05-04

## Context

当前 ProxyBot GUI 使用 Tauri v2 + React + TypeScript + shadcn/ui 技术栈，需要 Node.js 环境。用户希望移除 Node.js 依赖，统一为纯 Rust 技术栈。

## 目标

使用 Yew (Rust WASM) 重写 GUI，移除 npm/node 依赖，构建全 Rust 技术栈的桌面应用。

## 架构

```
┌─────────────────────────────────────────────────────┐
│  proxybot-gui (Rust binary)                        │
│                                                     │
│  ┌──────────────────┐    ┌──────────────────────┐ │
│  │  Tauri Backend   │    │   Yew Frontend       │ │
│  │  (Rust)          │    │   (Rust → WASM)      │ │
│  │                  │    │                      │ │
│  │  - invoke handlers│◄──►│  Components (9 tabs) │ │
│  │  - tray/menu     │    │                      │ │
│  │  - state mgmt     │    └──────────────────────┘ │
│  └────────┬─────────┘                             │
│           │              WebView (WASM)            │
│  ┌────────▼─────────┐                             │
│  │  proxybot_lib    │                             │
│  │  (shared)        │                             │
│  └──────────────────┘                             │
└─────────────────────────────────────────────────────┘
```

## 构建流程（无 Node）

```
cargo build --bin proxybot-gui
  → wasm-pack build --target web (编译 Yew → WASM)
  → Tauri bundler 打包
  → 输出 .dmg / Homebrew package
```

## 目录结构

```
src-tauri/
├── src/
│   ├── bin/
│   │   ├── proxybot-tui.rs    (TUI, unchanged)
│   │   └── proxybot-gui.rs    (Tauri + Yew 入口)
│   └── gui/
│       ├── lib.rs             (Yew 应用根)
│       ├── main.rs            (WASM 入口)
│       ├── app.rs             (主应用组件)
│       ├── components/        (9 个功能模块)
│       │   ├── traffic/
│       │   ├── rules/
│       │   ├── devices/
│       │   ├── certs/
│       │   ├── dns/
│       │   ├── alerts/
│       │   ├── replay/
│       │   ├── graph/
│       │   └── gen/
│       ├── hooks/             (共享 Yew hooks)
│       └── i18n/              (国际化)
└── tauri.conf.json
```

## 功能范围

所有 9 个 TUI 功能模块：

| 模块 | 功能 |
|------|------|
| **Traffic** | 实时请求列表、方法/主机/状态/应用过滤、详情面板 |
| **Rules** | 规则表、内联编辑器、5 种动作类型 |
| **Devices** | 设备表、MAC/最后出现/字节统计、设备规则覆盖 |
| **Certs** | CA 证书导出、指纹/过期/序列号显示、重新生成 |
| **DNS** | 上游解析器选择、拦截列表、查询日志 |
| **Alerts** | SEV1/2/3 告警、确认/清除控制 |
| **Replay** | 回放目标表、HAR 导出、diff 对比 |
| **Graph** | ASCII DAG 可视化、认证状态机 |
| **Gen** | Mock API 生成、脚手架生成、Docker bundle |

## 安装包

- **macOS:** `.dmg` + Homebrew (`brew install mbpz/proxybot/proxybot-gui`)
- **TUI 关系:** 保持独立二元 `proxybot-tui` 和 `proxybot-gui`，TUI 保留给 CLI 用户

## Tauri Invoke 接口

现有 IPC 接口保持不变：

| Handler | 功能 |
|---------|------|
| `start_proxy` | 启动代理 |
| `stop_proxy` | 停止代理 |
| `get_proxy_status` | 获取状态 |
| `get_devices` | 设备列表 |
| `get_rules` | 规则列表 |
| `save_rule` | 保存规则 |
| `delete_rule` | 删除规则 |
| `get_db_stats` | 数据库统计 |
| `set_device_rule_override` | 设备规则覆盖 |
| `get_ca_cert_path` | CA 证书路径 |
| `regenerate_ca` | 重新生成 CA |

## 依赖

```toml
# Tauri 后端
tauri = { version = "2", features = ["image-png", "tray-icon"] }
tauri-plugin-opener = "2"
tauri-plugin-notification = "2"

# Yew 前端 (编译为 WASM)
yew = "0.21"
wasm-bindgen = "0.2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 样式
stylist = "0.2"
```

## 技术要点

1. **wasm-pack:** 编译 Yew → WASM，`wasm-pack build --target web`
2. **trunk:** 开发时热重载（可选）
3. **stylist:** CSS-in-Rust，类型安全、性能好
4. **i18n:** Yew 国际化可用 `yew-i18n` 或自定义实现
5. **state:** Yew 信号 (use_state, use_reducer) + Tauri 状态同步

## 测试策略

- Rust 单元测试 (lib)
- WASM 集成测试 (wasm-bindgen-test)
- Tauri 端到端测试

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| Yew 学习曲线 | 类似 React，文档完善 |
| WASM 调试困难 | 使用 `console_error_panic_hook` |
| 构建速度 | CI 缓存 wasm-pack 产物 |