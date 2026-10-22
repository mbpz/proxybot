# Advanced Filter DSL v0.9.0 设计方案

## Status: Draft

## 1. Overview

实现高级过滤DSL，支持AND/OR/NOT组合，保存过滤器预设。

**当前问题：**
- TUI filter仅基础正则
- 无组合逻辑
- 无预设保存

**目标：**
- DSL语法: `method:GET AND host:*.example.com`
- 支持AND/OR/NOT
- 保存预设到本地

---

## 2. 竞品分析

| 竞品 | 过滤语法 |
|------|---------|
| mitmproxy | `~u regex ~m GET ~h foo=bar` |
| Proxyman | GUI勾选 + 正则 |
| HTTP Toolkit | 自然语言过滤 |

---

## 3. DSL 语法设计

### 3.1 支持的字段

| 字段 | 说明 | 示例 |
|------|------|------|
| `method` | HTTP方法 | `method:GET`, `method:POST` |
| `host` | 主机名 | `host:*.example.com`, `host:api.twitter.com` |
| `path` | URL路径 | `path:/api/v1/users` |
| `status` | 状态码 | `status:200`, `status:4*` |
| `app` | App分类 | `app:wechat`, `app:douyin` |
| `duration` | 耗时ms | `duration:>100`, `duration:<500` |
| `size` | 响应大小 | `size:>1000` |
| `header:` | Header字段 | `header:content-type:application/json` |
| `body:` | Body内容 | `body:*token*` |

### 3.2 操作符

| 操作符 | 说明 | 示例 |
|--------|------|------|
| `:` | 精确匹配 | `method:GET` |
| `:*` | 通配符 | `host:*.example.com` |
| `:` | 正则 | `path:/api/.*` |
| `>` `<` `>=` `<=` | 比较 | `duration:>100` |
| `AND` | 逻辑与 | `method:GET AND status:200` |
| `OR` | 逻辑或 | `method:GET OR method:POST` |
| `NOT` | 逻辑非 | `NOT method:OPTIONS` |
| `()` | 分组 | `(method:GET OR method:POST) AND host:api.*` |

### 3.3 示例

```
# 获取微信的200请求
app:wechat AND status:200

# 获取包含token的POST请求
method:POST AND body:*token*

# 排除静态资源
NOT path:*.js AND NOT path:*.css AND NOT path:*.png

# 复杂查询
(method:GET OR method:POST) AND host:api.weixin.qq.com AND status:2*
```

---

## 4. 实现设计

### 4.1 Parser (Rust)

```rust
#[derive(Debug, Clone)]
pub enum FilterExpr {
    Field { field: String, op: FilterOp, value: String },
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Group(Box<FilterExpr>),
}

#[derive(Debug, Clone)]
pub enum FilterOp {
    Eq,           // :
    Glob,         // :*
    Regex,        // :~
    Gt,           // >
    Lt,           // <
    Gte,          // >=
    Lte,          // <=
}
```

解析流程:
```
"method:GET AND status:2*"
  → Tokenizer
  → Parser
  → FilterExpr
  → Evaluator(request) → bool
```

### 4.2 Evaluator

```rust
fn evaluate(expr: &FilterExpr, req: &InterceptedRequest) -> bool {
    match expr {
        FilterExpr::Field { field, op, value } => {
            let field_value = get_field_value(req, field);
            match op {
                FilterOp::Eq => field_value == value,
                FilterOp::Glob => glob_match(value, &field_value),
                FilterOp::Regex => regex_match(value, &field_value),
                FilterOp::Gt => field_value.parse::<u64>().map(|v| v > parse_num(value)).unwrap_or(false),
                // ...
            }
        }
        FilterExpr::And(a, b) => evaluate(a, req) && evaluate(b, req),
        FilterExpr::Or(a, b) => evaluate(a, req) || evaluate(b, req),
        FilterExpr::Not(e) => !evaluate(e, req),
        FilterExpr::Group(e) => evaluate(e, req),
    }
}
```

### 4.3 前端FilterInput

```tsx
interface FilterPreset {
  id: string;
  name: string;
  expr: string;
}

function FilterInput({ value, onChange, presets }: FilterInputProps) {
  const [error, setError] = useState<string | null>(null);

  function handleChange(newValue: string) {
    try {
      // Validate DSL syntax locally
      parseFilter(newValue);
      setError(null);
      onChange(newValue);
    } catch (e) {
      setError(e.message);
    }
  }

  return (
    <div className="relative">
      <input
        type="text"
        value={value}
        onChange={e => handleChange(e.target.value)}
        placeholder="method:GET AND status:2*"
        className={error ? 'border-red-500' : ''}
      />
      {error && <span className="text-red-500 text-sm">{error}</span>}

      {/* Preset Dropdown */}
      <select
        onChange={e => onChange(presets.find(p => p.id === e.target.value)?.expr || '')}
      >
        <option value="">Load Preset...</option>
        {presets.map(p => (
          <option key={p.id} value={p.id}>{p.name}</option>
        ))}
      </select>
    </div>
  );
}
```

### 4.4 Preset Storage

```rust
// 本地存储
struct FilterPreset {
    id: String,
    name: String,
    expr: String,
}

#[tauri::command]
fn save_filter_preset(preset: FilterPreset) -> Result<(), String> {
    let mut config = load_config()?;
    config.filter_presets.push(preset);
    save_config(config)
}

#[tauri::command]
fn get_filter_presets() -> Result<Vec<FilterPreset>, String> {
    let config = load_config()?;
    Ok(config.filter_presets)
}
```

---

## 5. TUI 集成

在 TUI FilterBar 中使用:
```
Filter: [________________] (press / to focus)
                    ↑
              DSL输入 + 预设选择
```

---

## 6. 验证

```bash
# 测试解析
cargo test filter

# 测试DSL
echo "method:GET AND status:2*" | cargo run -- test-filter

# 前端输入验证
# 预设保存/加载
```
