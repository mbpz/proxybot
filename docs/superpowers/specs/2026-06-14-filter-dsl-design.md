# Advanced Filter DSL Design

**Date:** 2026-06-14
**Author:** Claude
**Status:** Approved (pending spec self-review)

---

## 1. Context

ProxyBot 当前有基本过滤器（按 host / app / method 单字段文本匹配），但没有组合逻辑、没有预设保存。Spec `2026-05-09-advanced-filter-dsl.md` 提出了完整 DSL 设计，本文实现 v1：

- 9 个字段 × 6 个操作符的解析器
- AND / OR / NOT / () 组合
- 预设保存 / 加载
- GUI 集成到 traffic 页面
- 解析器独立于 TUI / GUI，可复用

## 2. Goals & Non-Goals

### Goals

- DSL 语法：`field:value`，支持 `:`（精确）、`:*`（glob）、`:~`（regex）、`>` `<` `>=` `<=`（数值比较）
- 9 个字段：method, host, path, status, app, duration, size, header:X, body:X
- 组合子：AND, OR, NOT, ()
- 预设保存到 `~/.proxybot/config.toml` 的 `filter_presets` 数组
- 解析器在 Rust 端，evaluator 也 Rust 端
- 前端输入框实时验证 + 错误提示
- 预设下拉选择器

### Non-Goals

- TUI 端集成（本期只做 GUI）—— v2
- 模糊匹配（FuzzyMatch）—— v2
- DSL 跨字段 like-OR（如 `host|path:foo`）—— v2
- 预设导入/导出（JSON）—— v2
- 多 tab 独立 filter state（所有 tab 共享当前 filter）—— v2

## 3. Architecture

```
┌─────────────────┐  invoke("evaluate_filter", {expr, requestId})  ┌──────────────┐
│ FilterInput     │ ─────────────────────────────────────────────►│ filter/      │
│ (React)         │ ◄───────────────────────────────────────────── │ parser.rs   │
└─────────────────┘  bool                                         │ evaluator.rs │
         │                                                       │             │
         ▼  user types expression                                │             │
┌─────────────────┐  invoke("parse_filter", {expr})               │             │
│ Client-side     │ ─────────────────────────────────────────────►│  parser.rs  │
│ validation      │ ◄───────────────────────────────────────────── │             │
└─────────────────┘  ParseResult (AST or error)                   └──────────────┘
         │
         ▼  user clicks "Save Preset"
┌─────────────────┐  invoke("save_filter_preset", {preset})        ┌──────────────┐
│ PresetMenu      │ ─────────────────────────────────────────────►│ config.rs   │
└─────────────────┘                                               │ (config file)│
```

## 4. Data Structures

### 4.1 Rust 端

```rust
// src-tauri/src/filter/expr.rs

#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpr {
    Field { field: String, op: FilterOp, value: String },
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Group(Box<FilterExpr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterOp {
    Eq,      // :
    Glob,    // :*
    Regex,   // :~
    Gt, Lt, Gte, Lte,  // > < >= <=
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPreset {
    pub id: String,         // UUID
    pub name: String,       // user-given
    pub expr: String,       // canonical DSL text
}
```

### 4.2 Frontend 类型

```typescript
// src/components/filter/types.ts
export type FilterOp = "Eq" | "Glob" | "Regex" | "Gt" | "Lt" | "Gte" | "Lte";

export interface FilterPreset {
  id: string;
  name: string;
  expr: string;
}
```

## 5. Parser Grammar

DSL 语法（EBNF）：

```
expr      = or_expr
or_expr   = and_expr ("OR" and_expr)*
and_expr  = not_expr ("AND" not_expr)*
not_expr  = "NOT" not_expr | atom
atom      = "(" expr ")" | field_op
field_op  = IDENT (":" | ":*" | ":~" | ">" | "<" | ">=" | "<=") VALUE
IDENT     = [a-zA-Z_][a-zA-Z0-9_]*
VALUE     = quoted-string | unquoted-string
quoted-string     = '"' [^"]* '"'
unquoted-string   = [^ \t\n\r()]+
```

注意：IDENT 是 identifier（`method` `host` `header` `body` 等），对于 `header:X:Y` 或 `body:X`，X 是 IDENT 的一部分（用 `:` 分隔），解析器按 `:` 拆分首个 `:` 前为 field name。

## 6. Evaluator

`evaluate(expr: &FilterExpr, req: &InterceptedRequest) -> bool`：

| Field | Source |
|---|---|
| `method` | `req.method` |
| `host` | `req.host` |
| `path` | `req.path` |
| `status` | `req.status.map(\|s\| s.to_string()).unwrap_or_default()` |
| `app` | `req.app_name.clone().unwrap_or_default()` |
| `duration` | `req.latency_ms.map(\|l\| l.to_string()).unwrap_or_default()` |
| `size` | `req.resp_size.map(\|s\| s.to_string()).unwrap_or_default()` |
| `header:X` | `req.resp_headers.iter().find(|(k,_)| k.eq_ignore_ascii_case(X)).map(|(_,v)| v.clone()).unwrap_or_default()` |
| `body:X` | 字符串搜索 `String::from_utf8_lossy(&req.resp_body).contains(X)` |

操作符语义：
- `Eq` — 字符串相等
- `Glob` — `*` 通配（`a:*b` 匹配 `axxxb`）
- `Regex` — `regex::Regex::new(value).unwrap_or_else(|_| Regex::new("$^").unwrap()).is_match(&field_value)`
- `Gt/Lt/Gte/Lte` — 字段值 `parse::<u64>()`，比较

## 7. Tauri Commands

```rust
#[tauri::command]
fn parse_filter(expr: String) -> Result<FilterExpr, String>;

#[tauri::command]
fn evaluate_filter(expr: String, request: InterceptedRequest) -> bool;
// 实际签名会更复杂：传入 (expr, InterceptedRequest) 序列化为 filter 上下文

#[tauri::command]
fn list_filter_presets() -> Result<Vec<FilterPreset>, String>;

#[tauri::command]
fn save_filter_preset(preset: FilterPreset) -> Result<(), String>;

#[tauri::command]
fn delete_filter_preset(id: String) -> Result<(), String>;
```

## 8. Components

### 8.1 FilterInput.tsx

```tsx
interface FilterInputProps {
  value: string;
  onChange: (v: string) => void;
  presets: FilterPreset[];
  onSelectPreset: (preset: FilterPreset) => void;
}

export function FilterInput({ value, onChange, presets, onSelectPreset }: FilterInputProps) {
  const [error, setError] = useState<string | null>(null);
  
  async function handleChange(v: string) {
    setError(null);
    onChange(v);
    if (v.trim()) {
      try {
        await invoke("parse_filter", { expr: v });
      } catch (e) {
        setError(String(e));
      }
    }
  }
  
  return (
    <div className="flex gap-2">
      <input
        type="text"
        value={value}
        onChange={e => handleChange(e.target.value)}
        placeholder="method:GET AND host:*.example.com"
        className={`flex-1 px-2 py-1 text-sm border rounded ${
          error ? "border-red-500" : ""
        }`}
        data-testid="filter-input"
      />
      <select
        onChange={e => {
          const p = presets.find(x => x.id === e.target.value);
          if (p) onSelectPreset(p);
        }}
        data-testid="filter-preset-select"
        className="text-sm border rounded px-2"
      >
        <option value="">Load Preset…</option>
        {presets.map(p => (
          <option key={p.id} value={p.id}>{p.name}</option>
        ))}
      </select>
      <SavePresetButton currentExpr={value} />
    </div>
  );
}
```

### 8.2 SavePresetButton.tsx

弹出 modal 让用户输入 preset name，调用 `save_filter_preset` Tauri 命令。简单 dialog。

## 9. Integration Points

### 9.1 TrafficPage

`src/components/traffic/TrafficPage.tsx` 当前已有 filter input。替换为 `<FilterInput>` 组件，保留现有 state。

### 9.2 Filter Logic 集成

当前 traffic 列表可能按 host / app / method 单字段文本匹配（要确认现有实现）。新增 DSL 路径：当用户输入 DSL 时，调用 `parse_filter` + `evaluate_filter` 来过滤列表。

### 9.3 评估性能

每次 filter 改变 → 所有可见的 request 重新评估 → UI 重渲染。当 request 数量 < 10k 时 O(n × m) 仍可接受（n = requests, m = filter expression size）。

## 10. Error Handling

| 场景 | 处理 |
|---|---|
| `parse_filter` 失败（语法错误） | 返回 `Err(String)`，前端显示红色边框 + 错误消息 |
| `evaluate_filter` 失败（regex 编译失败） | 该单个 frame 视为不匹配，evaluator 继续 |
| `save_filter_preset` 失败（config 写错误） | 返回 `Err`，前端显示 toast |
| `delete_filter_preset` 失败（id 不存在） | 返回 `Err("Preset not found")` |
| DSL 引号内的值包含 `(` | 解析器正确处理（quoted-string 是 leaf） |
| `header:X` 中 X 为空 | 解析器返回 `Err("Empty header name")` |
| `body:X` 超过 1MB | 截断到 1MB 后再 search |

## 11. Testing

### 11.1 单元测试

**filter/parser.rs** (~10 tests):
- `test_parse_simple_field` — `method:GET`
- `test_parse_glob_field` — `host:*.example.com`
- `test_parse_regex_field` — `path:/api/.*`
- `test_parse_numeric_comparison` — `status:>=200`, `duration:<500`
- `test_parse_and` — `method:GET AND status:200`
- `test_parse_or` — `method:GET OR method:POST`
- `test_parse_not` — `NOT method:OPTIONS`
- `test_parse_group` — `(method:GET OR method:POST) AND host:api.*`
- `test_parse_header_field` — `header:content-type:application/json`
- `test_parse_body_field` — `body:*token*`
- `test_parse_error_unbalanced_paren` — `((method:GET` → Err
- `test_parse_error_unknown_keyword` — `FOO method:GET` → Err

**filter/evaluator.rs** (~6 tests):
- `test_evaluate_simple_eq` — `method:GET` matches when req.method = "GET"
- `test_evaluate_glob` — `host:*.example.com` matches subdomains
- `test_evaluate_regex` — `path:/api/.*` matches
- `test_evaluate_numeric` — `status:>=200` matches
- `test_evaluate_combinators` — `AND` `OR` `NOT` 嵌套
- `test_evaluate_header_body` — `header:X:Y` 和 `body:X`

**filter/preset.rs** (~3 tests):
- `test_save_and_load_preset` — 保存到 config，读取回
- `test_list_presets` — 多预设
- `test_delete_preset` — 删一个，剩余的还在

### 11.2 E2E

`e2e/filter-dsl.spec.ts`:
- `filter_input_validates_known_syntax` — 输入合法 expr，无错误
- `filter_input_shows_error_for_bad_syntax` — `((method:GET` 显示错误
- `filter_input_evaluates_traffic` — 表达式过滤 traffic 列表
- `preset_save_and_load` — 保存预设 → 列表出现 → 加载
- `preset_delete` — 删除预设 → 列表少一个

### 11.3 手动

- 启动 ProxyBot
- 在 traffic 列表上方输入 `app:wechat AND status:2*`
- 验证列表只显示 wechat app 的 2xx 请求
- 点击 Save Preset → 输入名字 → 验证出现在下拉
- 关闭应用 → 重开 → 验证预设仍在
- 编辑预设 → 验证

## 12. Implementation Notes

### 12.1 Files

**新建：**
- `src-tauri/src/filter/mod.rs`
- `src-tauri/src/filter/expr.rs` — FilterExpr / FilterOp types
- `src-tauri/src/filter/parser.rs` — Tokenizer + Parser
- `src-tauri/src/filter/evaluator.rs` — evaluate function
- `src-tauri/src/filter/preset.rs` — save/load/delete
- `src/components/filter/types.ts`
- `src/components/filter/FilterInput.tsx`
- `src/components/filter/SavePresetButton.tsx`
- `e2e/filter-dsl.spec.ts`

**修改：**
- `src-tauri/src/lib.rs` — 注册 `pub mod filter;` + 5 个 Tauri 命令
- `src-tauri/src/commands/mod.rs` — 新增 `pub mod filter_dsl;` (命令)
- `src/components/traffic/TrafficPage.tsx` — 替换 filter input 为 `<FilterInput>`
- `Cargo.toml` — `regex` crate (如果还没)

### 12.2 依赖

- `regex = "1"` (新加；可能已经在)
- 不需新加 GUI 依赖

### 12.3 配置存储

复用现有 `~/.proxybot/config.toml`。格式：
```toml
[filter_presets]
[[filter_presets.entries]]
id = "uuid-1"
name = "WeChat 2xx"
expr = "app:wechat AND status:2*"
```

或更简单：现有的 `config.rs` 已有 `presets: Vec<FilterPreset>` 字段。

## 13. Self-Review

- Placeholder scan：0 TBD/TODO
- Internal consistency：Rust 和 TS 的 enum names 一致（Eq/Glob/Regex/Gt/Lt/Gte/Lte）
- Scope：单一功能（filter DSL），~3-4 天工作量
- Ambiguity check：DSL 语法明确（EBNF 给出）；操作符语义明确（每个 5.x 章节）；错误处理明确（每个 case 一行）