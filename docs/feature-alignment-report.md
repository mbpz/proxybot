# ProxyBot TUI 功能对齐报告

## 对比结论

**README 描述的功能与 TUI 实现基本对齐**，9 个 tab 全部实现，核心功能覆盖完整。以下是详细差异。

---

## ✅ 已对齐的功能

### Traffic tab ✅
- 方法/主机/状态/app_tag 过滤 ✅ (filters: method, host_pattern, status_class, app_tag)
- 正则搜索 ✅ (search_input + search_regex)
- 分栏面板 (60/40) ✅ (request list + detail panel)
- pf/DNS 控制 ✅ (TogglePf, ToggleDns)

### Rules tab ✅
- 规则表格 (DIRECT/PROXY/REJECT) ✅
- 模态编辑器 ✅
- 热重载状态 ✅ (watcher_active)
- a=add, e=edit, d=delete, s=save ✅

### Devices tab ✅
- 设备表格 (MAC/last_seen/bytes up-down) ✅
- per-device rule override ✅ — `[e]` 进入编辑模式，输入规则动作，Enter 确认，Esc 取消

### Certs tab ✅
- CA 指纹/过期/序列号显示 ✅
- 重新生成 CA ✅ (RegenerateCert)
- 导出 PEM ✅ (ExportCert)

### DNS tab ✅
- DoH / Plain UDP 上游选择 ✅
- blocklist 切换 ✅ (ToggleBlocklist)
- hosts 条目 ✅ (hosts lock)
- 实时查询日志 ✅ (recent_entries)

### Alerts tab ✅
- 严重级别徽章 (SEV1/2/3) ✅
- ACK/clear 控制 ✅
- 基准统计 ✅ (baseline_info)

### Replay tab ✅
- Replay 目标表格 ✅ (targets_list)
- start/stop replay ✅
- HAR 导出 ✅ (ExportHar)
- diff 视图 ✅ (ShowDiff)

### Graph tab ✅
- ASCII DAG 可视化 ✅ (DAG view)
- Auth 状态机检测 ✅ (AuthStateMachine view)

### Gen tab ✅
- Mock API 生成 ✅ (GenMockApi, m)
- Frontend scaffold ✅ (GenFrontend, f)
- Docker bundle ✅ (GenDocker, d)
- 打开输出目录 ✅ (OpenOutput, o)

---

## ⚠️ 不对齐 / 缺失的功能

### 1. README 键盘快捷方式 vs 实际实现

README 列出的快捷方式有几处不一致：

| README 描述 | 实际实现 | 状态 |
|---|---|---|
| `s` = stop proxy | `s` on Rules tab = SaveRule | ⚠️ 冲突 |
| `n` = toggle DNS server | `n` on Traffic tab = ToggleDns | ✅ 一致 |
| `/` = focus search | ✅ | ✅ |
| `x` = clear filters | ✅ (ClearSearch) | ✅ |
| `Enter` = load request detail | ✅ | ✅ |
| `m` = method filter | ✅ 已实现 (FilterMethod) | ✅ |
| `f` = host filter | ✅ 已实现 (FilterHost) | ✅ |
| `o` = status filter | ✅ 已实现 (FilterStatus) | ✅ |
| `a` = app_tag filter | ✅ 已实现 (FilterAppTag) | ✅ |
| `d` = toggle blocklist (DNS) | ✅ on DNS tab | ✅ |
| `u` = cycle upstream (DNS) | ✅ | ✅ |

### 2. 端口配置不一致

README Installation Step 4 说 port = `8080`：
```
4. Set **Port** to `8080`
```

但 `config.rs` 中 `proxy_port = 8088`。这是**配置漂移**。

### 3. Devices tab — per-device rule override

README 说：
> **Devices** tab: Device table with MAC/last_seen/bytes up-down, **per-device rule override**

`devices.rs` 渲染器**已实现 rule override 选择器** — 按 `[e]` 进入编辑模式，显示当前值，输入新值后 Enter 确认，Esc 取消。数据库 `rule_override` 字段与 UI 编辑器已连接。

### 4. Gen tab README 描述不完整

README 只说：
> **Gen** | Mock API / frontend scaffold / Docker bundle generation

实际 TUI 显示了详细的热键提示（`[m]` Generate Mock API, `[f]` Generate Frontend, `[d]` Generate Docker, `[o]` Open output），比 README 更详细。README 缺少 Gen tab 的描述。

### 5. Graph tab README 描述简化

README 说：
> **Graph** tab: ASCII DAG visualization, auth state machine detection

实际 TUI 还支持 `g` / `a` 切换两种视图，`r` 刷新。

### 6. Replay tab README 描述简化

README 说：
> **Replay** tab: Replay targets table, start/stop replay, HAR export, diff view

实际 TUI 还有 `d` = ShowDiff。但 README 没提 `d`。

---

## 📋 操作问题

### Devices tab 热键缺失

README 键盘部分没有列出 Devices tab 的快捷键。实际 TUI 中该 tab 没有任何定义的快捷键（没有 `handle_key_event` 处理）。

### Graph tab 热键在 README 中缺失

README 键盘部分只列了 Traffic/Rules/Certs/DNS tab 的热键，Graph 和 Gen 的热键没在表格中列出，但在 TUI 中存在。

---

## 配置漂移

| 项目 | README | config.rs | 实际行为 |
|---|---|---|---|
| Proxy port | 8080 | 8088 | 8088 (config.rs) |
| DNS port | 未明确 | 5300 | 5300 (config.rs) |

---

## 总结

### 需要修复
1. **端口不一致**: README 说 8080，config.rs 是 8088 — ✅ 已统一（README 改为 8088）
2. **过滤快捷键**: README 说 `m/f/o/a` 是过滤键，TUI 中未实现 — ✅ 已实现
3. **Devices rule override**: 数据库有字段，UI 无选择器 — ✅ 已实现

### README 需要补充
（已全部补充完整）
1. Gen tab 热键 `m/f/d/o`
2. Graph tab 热键 `g/a/r`
3. Replay tab `d` = ShowDiff
4. Rules/Graph/Gen 的完整热键
5. `s` 在 Traffic/其他 tab 是 StartProxy，不是 StopProxy（README 有误导）

### 可选增强
1. Devices tab 添加 rule override 选择器 UI
2. 9 个 tab 都应该有热键（目前 Devices 没有）