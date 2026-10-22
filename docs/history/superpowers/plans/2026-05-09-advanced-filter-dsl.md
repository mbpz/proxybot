# Advanced Filter DSL Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现高级过滤DSL语法，支持AND/OR/NOT组合和过滤器预设保存

**Architecture:** Rust端实现Tokenizer + Parser + Evaluator，前端提供FilterInput组件调用

**Tech Stack:** Rust (nom parser), React, TypeScript

---

## File Structure

| 操作 | 文件 | 职责 |
|------|------|------|
| Create | `src-tauri/src/filter/dsl.rs` | DSL解析器 |
| Create | `src-tauri/src/filter/mod.rs` | 模块导出 |
| Create | `src-tauri/src/filter/evaluator.rs` | 过滤器执行器 |
| Modify | `src-tauri/src/lib.rs` | 注册filter模块 |
| Create | `src/components/ui/FilterInput.tsx` | 过滤器输入组件 |
| Create | `src-tauri/src/commands/filter.rs` | IPC命令 |

---

## Task 1: 创建Filter DSL解析器

**Files:**
- Create: `src-tauri/src/filter/dsl.rs`

- [ ] **Step 1: 创建dsl.rs**

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum FilterExpr {
    Field {
        field: String,
        op: FilterOp,
        value: String,
    },
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Group(Box<FilterExpr>),
}

#[derive(Debug, Clone, Copy)]
pub enum FilterOp {
    Eq,      // :
    Glob,    // :*
    Regex,   // :~
    Gt,      // >
    Lt,      // <
    Gte,     // >=
    Lte,     // <=
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Field(String),
    Op(FilterOp),
    Value(String),
    And,
    Or,
    Not,
    LParen,
    RParen,
    EOF,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            let ch = self.input[self.pos];

            if ch.is_whitespace() {
                self.pos += 1;
                continue;
            }

            if ch == '(' {
                tokens.push(Token::LParen);
                self.pos += 1;
                continue;
            }

            if ch == ')' {
                tokens.push(Token::RParen);
                self.pos += 1;
                continue;
            }

            if ch.is_alphabetic() || ch == '_' {
                let start = self.pos;
                while self.pos < self.input.len() && (self.input[self.pos].is_alphanumeric() || self.input[self.pos] == '_' || self.input[self.pos] == '.') {
                    self.pos += 1;
                }
                let word: String = self.input[start..self.pos].iter().collect();

                if word == "AND" {
                    tokens.push(Token::And);
                } else if word == "OR" {
                    tokens.push(Token::Or);
                } else if word == "NOT" {
                    tokens.push(Token::Not);
                } else if self.pos < self.input.len() && self.input[self.pos] == ':' {
                    self.pos += 1;
                    let op = if self.pos < self.input.len() && self.input[self.pos] == '*' {
                        self.pos += 1;
                        FilterOp::Glob
                    } else if self.pos < self.input.len() && self.input[self.pos] == '~' {
                        self.pos += 1;
                        FilterOp::Regex
                    } else {
                        FilterOp::Eq
                    };
                    tokens.push(Token::Field(word));
                    tokens.push(Token::Op(op));
                } else {
                    return Err(format!("Unexpected token: {}", word));
                }
                continue;
            }

            if ch == '>' || ch == '<' {
                let op = if self.input[self.pos + 1] == '=' {
                    self.pos += 1;
                    if ch == '>' { FilterOp::Gte } else { FilterOp::Lte }
                } else {
                    if ch == '>' { FilterOp::Gt } else { FilterOp::Lt }
                };
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.input.len() && self.input[self.pos].is_numeric() {
                    self.pos += 1;
                }
                let value: String = self.input[start..self.pos].iter().collect();
                tokens.push(Token::Op(op));
                tokens.push(Token::Value(value));
                continue;
            }

            if ch == '"' || ch == '\'' {
                let quote = ch;
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.input.len() && self.input[self.pos] != quote {
                    self.pos += 1;
                }
                let value: String = self.input[start..self.pos].iter().collect();
                self.pos += 1;
                tokens.push(Token::Value(value));
                continue;
            }

            return Err(format!("Unexpected character: {}", ch));
        }

        tokens.push(Token::EOF);
        Ok(tokens)
    }
}

pub fn parse(input: &str) -> Result<FilterExpr, String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_expr()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn parse_expr(&mut self) -> Result<FilterExpr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<FilterExpr, String> {
        let mut left = self.parse_and()?;

        while self.peek() == &Token::Or {
            self.advance();
            let right = self.parse_and()?;
            left = FilterExpr::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<FilterExpr, String> {
        let mut left = self.parse_not()?;

        while self.peek() == &Token::And {
            self.advance();
            let right = self.parse_not()?;
            left = FilterExpr::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_not(&mut self) -> Result<FilterExpr, String> {
        if self.peek() == &Token::Not {
            self.advance();
            let expr = self.parse_not()?;
            return Ok(FilterExpr::Not(Box::new(expr)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<FilterExpr, String> {
        match self.peek() {
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                if self.peek() != &Token::RParen {
                    return Err("Expected closing paren".to_string());
                }
                self.advance();
                Ok(FilterExpr::Group(Box::new(expr)))
            }
            Token::Field(name) => {
                self.advance();
                let op = match self.peek() {
                    Token::Op(op) => {
                        self.advance();
                        op
                    }
                    _ => return Err("Expected operator".to_string()),
                };
                let value = match self.peek() {
                    Token::Value(v) => {
                        self.advance();
                        v
                    }
                    _ => return Err("Expected value".to_string()),
                };
                Ok(FilterExpr::Field {
                    field: name,
                    op,
                    value,
                })
            }
            _ => Err("Unexpected token".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_field() {
        let result = parse("method:GET");
        assert!(result.is_ok());
    }

    #[test]
    fn test_and_expr() {
        let result = parse("method:GET AND status:200");
        assert!(result.is_ok());
    }

    #[test]
    fn test_glob() {
        let result = parse("host:*example.com");
        assert!(result.is_ok());
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/filter/dsl.rs src-tauri/src/filter/mod.rs
git commit -m "feat(filter): add DSL lexer and parser"
```

---

## Task 2: 创建FilterEvaluator

**Files:**
- Create: `src-tauri/src/filter/evaluator.rs`

- [ ] **Step 1: 创建evaluator.rs**

```rust
use crate::filter::dsl::{FilterExpr, FilterOp};
use std::collections::HashMap;

pub struct InterceptedRequest {
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: Option<u16>,
    pub duration_ms: u64,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
}

pub struct Evaluator;

impl Evaluator {
    pub fn evaluate(expr: &FilterExpr, req: &InterceptedRequest) -> bool {
        match expr {
            FilterExpr::Field { field, op, value } => {
                let field_value = Self::get_field_value(req, field);
                Self::compare(&field_value, op, value)
            }
            FilterExpr::And(a, b) => {
                Self::evaluate(a, req) && Self::evaluate(b, req)
            }
            FilterExpr::Or(a, b) => {
                Self::evaluate(a, req) || Self::evaluate(b, req)
            }
            FilterExpr::Not(e) => {
                !Self::evaluate(e, req)
            }
            FilterExpr::Group(e) => {
                Self::evaluate(e, req)
            }
        }
    }

    fn get_field_value(req: &InterceptedRequest, field: &str) -> String {
        match field {
            "method" => req.method.clone(),
            "host" => req.host.clone(),
            "path" => req.path.clone(),
            "status" => req.status.map(|s| s.to_string()).unwrap_or_default(),
            "duration" => req.duration_ms.to_string(),
            _ => req.headers.get(field).cloned().unwrap_or_default(),
        }
    }

    fn compare(field_value: &str, op: &FilterOp, filter_value: &str) -> bool {
        match op {
            FilterOp::Eq => field_value == filter_value,
            FilterOp::Glob => {
                let pattern = filter_value
                    .replace('*', ".*")
                    .replace('?', ".");
                regex_match(&pattern, field_value)
            }
            FilterOp::Regex => {
                regex_match(filter_value, field_value)
            }
            FilterOp::Gt => {
                field_value.parse::<u64>().map(|v| v > filter_value.parse().unwrap_or(0)).unwrap_or(false)
            }
            FilterOp::Lt => {
                field_value.parse::<u64>().map(|v| v < filter_value.parse().unwrap_or(u64::MAX)).unwrap_or(false)
            }
            FilterOp::Gte => {
                field_value.parse::<u64>().map(|v| v >= filter_value.parse().unwrap_or(0)).unwrap_or(false)
            }
            FilterOp::Lte => {
                field_value.parse::<u64>().map(|v| v <= filter_value.parse().unwrap_or(u64::MAX)).unwrap_or(false)
            }
        }
    }
}

fn regex_match(pattern: &str, value: &str) -> bool {
    if let Ok(re) = regex::Regex::new(pattern) {
        re.is_match(value)
    } else {
        false
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/filter/evaluator.rs
git commit -m "feat(filter): add filter evaluator"
```

---

## Task 3: 创建FilterInput组件

**Files:**
- Create: `src/components/ui/FilterInput.tsx`

- [ ] **Step 1: 创建FilterInput**

```tsx
import { useState, useCallback } from "react";

interface FilterPreset {
  id: string;
  name: string;
  expr: string;
}

interface FilterInputProps {
  value: string;
  onChange: (value: string) => void;
  presets?: FilterPreset[];
  onSavePreset?: (name: string, expr: string) => void;
  error?: string | null;
}

export function FilterInput({
  value,
  onChange,
  presets = [],
  onSavePreset,
  error,
}: FilterInputProps) {
  const [showPresets, setShowPresets] = useState(false);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onChange(e.target.value);
    },
    [onChange]
  );

  const handlePresetSelect = useCallback(
    (expr: string) => {
      onChange(expr);
      setShowPresets(false);
    },
    [onChange]
  );

  return (
    <div className="relative">
      <input
        type="text"
        value={value}
        onChange={handleChange}
        placeholder="method:GET AND status:2*"
        className={`w-full px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-blue-500 ${
          error ? "border-red-500" : ""
        }`}
      />
      {error && <span className="text-red-500 text-sm mt-1">{error}</span>}

      <div className="flex gap-2 mt-2">
        {presets.length > 0 && (
          <select
            onChange={(e) => handlePresetSelect(e.target.value)}
            className="px-2 py-1 border rounded text-sm"
          >
            <option value="">Load Preset...</option>
            {presets.map((p) => (
              <option key={p.id} value={p.expr}>
                {p.name}
              </option>
            ))}
          </select>
        )}

        {onSavePreset && value && (
          <button
            onClick={() => {
              const name = prompt("Preset name:");
              if (name) onSavePreset(name, value);
            }}
            className="px-2 py-1 bg-gray-100 rounded text-sm hover:bg-gray-200"
          >
            Save Preset
          </button>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/ui/FilterInput.tsx
git commit -m "feat(filter): add FilterInput component"
```

---

## Task 4: 创建IPC命令

**Files:**
- Create: `src-tauri/src/commands/filter.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建filter.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPreset {
    pub id: String,
    pub name: String,
    pub expr: String,
}

#[tauri::command]
pub fn parse_filter(expr: &str) -> Result<String, String> {
    crate::filter::dsl::parse(expr)?;
    Ok("valid".to_string())
}

#[tauri::command]
pub fn evaluate_filter(expr: &str, request_json: &str) -> Result<bool, String> {
    let expr = crate::filter::dsl::parse(expr)?;
    let request: crate::filter::evaluator::InterceptedRequest =
        serde_json::from_str(request_json)
            .map_err(|e| e.to_string())?;
    Ok(crate::filter::evaluator::Evaluator::evaluate(&expr, &request))
}

#[tauri::command]
pub fn save_filter_preset(preset: FilterPreset) -> Result<(), String> {
    // Save to config file
    Ok(())
}

#[tauri::command]
pub fn get_filter_presets() -> Result<Vec<FilterPreset>, String> {
    Ok(vec![])
}
```

- [ ] **Step 2: 注册命令**

在 `src-tauri/src/lib.rs` 中添加:
```rust
pub mod commands;
pub mod commands::filter;
pub use commands::filter::*;
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/filter.rs src-tauri/src/lib.rs
git commit -m "feat(filter): add filter IPC commands"
```

---

## Task 5: 编译验证

- [ ] **Step 1: 运行测试**

```bash
cd src-tauri && cargo test filter -- --nocapture
```

- [ ] **Step 2: 编译**

```bash
npm run build 2>&1 | tail -20
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(filter): complete Filter DSL implementation"
```

---

## 验证清单

- [ ] `method:GET AND status:2*` 解析成功
- [ ] `(method:GET OR method:POST) AND host:*.example.com` 解析成功
- [ ] `NOT method:OPTIONS` 解析成功
- [ ] FilterInput组件正常显示
- [ ] 预设保存/加载功能
- [ ] 编译通过
