# Column-Scoped Filter DSL Implementation Plan

**Date:** 2026-05-10
**Feature:** Column-Scoped Filter DSL
**Priority:** P1
**Estimated Duration:** 2-3 days

---

## 1. Overview

Extend the existing ProxyBot filter DSL to support column-scoped syntax where each field is prefixed with the column name followed by a colon, similar to Proxelar's syntax. This provides more intuitive filtering like `method:POST host:api status:2*`.

## 2. Current Architecture

The existing DSL at `src-tauri/src/filter/dsl.rs` supports:
- Field-based expressions: `method:GET`
- Glob patterns: `host:*example.com`
- Regex operators: `path:~/api/v1/`
- Boolean operators: `AND`, `OR`, `NOT`
- Grouping: `(expr)`

The lexer currently identifies fields as alphabetic words followed by `:`. The extension will add support for more column types and improve the value consumption logic.

## 3. New Syntax Design

### 3.1 Column Scoped Syntax

```
method:POST              # Exact match on method
host:api.*              # Glob match on host
status:2*               # Prefix match on status code (2xx)
path:/api/v1/users      # Exact match on path
app:WeChat               # Exact match on app classification
sni:tls.example.com      # Exact match on SNI
ip:192.168.*            # Glob match on source IP
```

### 3.2 Extended Operators

| Operator | Syntax | Description |
|----------|--------|-------------|
| `:` | `host:api` | Exact match |
| `:*` | `host:*example` | Glob suffix match |
| `:~` | `path:~/v1/` | Regex match |
| `>` | `status:>400` | Greater than |
| `<` | `status:<500` | Less than |
| `>=` | `status:>=200` | Greater or equal |
| `<=` | `status:<=299` | Less or equal |

### 3.3 Value Type Extensions

Currently the lexer consumes alphanumeric, underscore, dot, hyphen, and asterisk characters after the operator. We need to extend this to support:

- Colons in URLs (but not after operators)
- Query parameters (e.g., `?foo=bar`)
- Port numbers (e.g., `host:api.example.com:8080`)

## 4. Implementation Steps

### Day 1: DSL Extension

**File:** `src-tauri/src/filter/dsl.rs`

Modify the lexer to handle column-scoped tokens with extended value characters:

```rust
// Add new column types to the Token enum
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Field(String),        // method, host, status, path, etc.
    Column(String),       // New: explicit column:prefix (e.g., method:POST)
    Op(FilterOp),
    Value(String),
    And,
    Or,
    Not,
    LParen,
    RParen,
    EOF,
}

// New column types enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnType {
    Method,
    Host,
    Path,
    Status,
    App,
    Sni,
    Ip,
    Timestamp,
    Size,
    Unknown,
}

impl ColumnType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "method" => ColumnType::Method,
            "host" => ColumnType::Host,
            "path" => ColumnType::Path,
            "status" => ColumnType::Status,
            "app" => ColumnType::App,
            "sni" => ColumnType::Sni,
            "ip" => ColumnType::Ip,
            "timestamp" | "time" => ColumnType::Timestamp,
            "size" => ColumnType::Size,
            _ => ColumnType::Unknown,
        }
    }
}
```

Modify the lexer tokenization to handle all column types:

```rust
impl Lexer {
    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            let ch = self.input[self.pos];

            if ch.is_whitespace() {
                self.pos += 1;
                continue;
            }

            // ... existing parentheses handling ...

            if ch.is_alphabetic() || ch == '_' {
                let start = self.pos;
                while self.pos < self.input.len()
                    && (self.input[self.pos].is_alphanumeric()
                        || self.input[self.pos] == '_'
                        || self.input[self.pos] == '.')
                {
                    self.pos += 1;
                }
                let word: String = self.input[start..self.pos].iter().collect();

                // Handle boolean keywords
                if word == "AND" {
                    tokens.push(Token::And);
                    continue;
                }
                if word == "OR" {
                    tokens.push(Token::Or);
                    continue;
                }
                if word == "NOT" {
                    tokens.push(Token::Not);
                    continue;
                }

                // Handle column:value syntax
                if self.pos < self.input.len() && self.input[self.pos] == ':' {
                    self.pos += 1; // consume ':'

                    // Determine operator type
                    let op = if self.pos < self.input.len() && self.input[self.pos] == '*' {
                        self.pos += 1;
                        FilterOp::Glob
                    } else if self.pos < self.input.len() && self.input[self.pos] == '~' {
                        self.pos += 1;
                        FilterOp::Regex
                    } else if self.pos < self.input.len() && self.input[self.pos] == '>' {
                        self.pos += 1;
                        if self.pos < self.input.len() && self.input[self.pos] == '=' {
                            self.pos += 1;
                            FilterOp::Gte
                        } else {
                            FilterOp::Gt
                        }
                    } else if self.pos < self.input.len() && self.input[self.pos] == '<' {
                        self.pos += 1;
                        if self.pos < self.input.len() && self.input[self.pos] == '=' {
                            self.pos += 1;
                            FilterOp::Lte
                        } else {
                            FilterOp::Lt
                        }
                    } else {
                        FilterOp::Eq
                    };

                    // Consume value with extended character set
                    let value = self.consume_value();

                    if value.is_empty() {
                        return Err("Expected value after operator".to_string());
                    }

                    tokens.push(Token::Field(word));
                    tokens.push(Token::Op(op));
                    tokens.push(Token::Value(value));
                    continue;
                }

                return Err(format!("Unexpected token: {}", word));
            }

            // ... existing quote handling for values with spaces ...
        }

        tokens.push(Token::EOF);
        Ok(tokens)
    }

    fn consume_value(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            // Extended value characters: alphanumeric, underscore, dot, hyphen,
            // asterisk, slash, question mark, ampersand, equals, colon, percent
            if ch.is_alphanumeric()
                || ch == '_'
                || ch == '.'
                || ch == '-'
                || ch == '*'
                || ch == '/'
                || ch == '?'
                || ch == '&'
                || ch == '='
                || ch == ':'
                || ch == '%'
                || ch == '['
                || ch == ']'
            {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.input[start..self.pos].iter().collect()
    }
}
```

### Day 2: Filter Evaluation Enhancement

**File:** `src-tauri/src/filter/evaluator.rs`

Extend the evaluator to handle the new column types:

```rust
use crate::filter::dsl::{ColumnType, FilterExpr, FilterOp};

pub struct RequestRecord {
    pub method: String,
    pub host: String,
    pub path: String,
    pub status: u16,
    pub app: Option<String>,
    pub sni: Option<String>,
    pub ip: Option<String>,
    pub timestamp: i64,
    pub size: Option<i64>,
}

pub fn evaluate(expr: &FilterExpr, record: &RequestRecord) -> bool {
    match expr {
        FilterExpr::Field { field, op, value } => {
            let column = ColumnType::from_str(field);
            let field_value = match column {
                ColumnType::Method => record.method.clone(),
                ColumnType::Host => record.host.clone(),
                ColumnType::Path => record.path.clone(),
                ColumnType::Status => record.status.to_string(),
                ColumnType::App => record.app.clone().unwrap_or_default(),
                ColumnType::Sni => record.sni.clone().unwrap_or_default(),
                ColumnType::Ip => record.ip.clone().unwrap_or_default(),
                ColumnType::Timestamp => record.timestamp.to_string(),
                ColumnType::Size => record.size.map(|s| s.to_string()).unwrap_or_default(),
                ColumnType::Unknown => return false,
            };

            match op {
                FilterOp::Eq => field_value == *value,
                FilterOp::Glob => matches_glob(&field_value, value),
                FilterOp::Regex => matches_regex(&field_value, value),
                FilterOp::Gt => compare_numeric(&field_value, value) > 0,
                FilterOp::Lt => compare_numeric(&field_value, value) < 0,
                FilterOp::Gte => compare_numeric(&field_value, value) >= 0,
                FilterOp::Lte => compare_numeric(&field_value, value) <= 0,
            }
        }
        // ... existing And, Or, Not, Group handling ...
    }
}

fn matches_glob(value: &str, pattern: &str) -> bool {
    // Simple glob matching: * matches any sequence
    if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        value.ends_with(suffix)
    } else if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        value.starts_with(prefix)
    } else {
        value.contains(pattern)
    }
}
```

### Day 3: UI Integration

**File:** `src-tauri/src/commands/filter.rs`

Add Tauri commands to parse and validate filter expressions:

```rust
use crate::filter::dsl::{parse, FilterExpr};

#[derive(Debug, Serialize, Deserialize)]
pub struct FilterParseResult {
    pub valid: bool,
    pub expr: Option<FilterExpr>,
    pub error: Option<String>,
}

#[tauri::command]
pub fn parse_filter(input: String) -> FilterParseResult {
    match parse(&input) {
        Ok(expr) => FilterParseResult {
            valid: true,
            expr: Some(expr),
            error: None,
        },
        Err(e) => FilterParseResult {
            valid: false,
            expr: None,
            error: Some(e),
        },
    }
}

#[tauri::command]
pub fn evaluate_filter(input: String, record: RequestRecord) -> FilterParseResult {
    match parse(&input) {
        Ok(expr) => {
            let result = evaluate(&expr, &record);
            FilterParseResult {
                valid: result,
                expr: Some(expr),
                error: None,
            }
        }
        Err(e) => FilterParseResult {
            valid: false,
            expr: None,
            error: Some(e),
        },
    }
}
```

**File:** `src/components/TrafficPage.tsx` (or wherever filter input is)

Update the filter input component:

```tsx
import { useState, useCallback } from 'react';

interface FilterInputProps {
  onFilterChange: (filter: string) => void;
  onValidationResult: (valid: boolean, error?: string) => void;
}

export function FilterInput({ onFilterChange, onValidationResult }: FilterInputProps) {
  const [value, setValue] = useState('');

  const handleChange = useCallback(async (e: React.ChangeEvent<HTMLInputElement>) => {
    const input = e.target.value;
    setValue(input);

    if (input.trim() === '') {
      onFilterChange(input);
      onValidationResult(true);
      return;
    }

    try {
      const result = await window.__TAURI__.invoke('parse_filter', { input });
      if (result.valid) {
        onFilterChange(input);
        onValidationResult(true);
      } else {
        onValidationResult(false, result.error);
      }
    } catch (err) {
      onValidationResult(false, String(err));
    }
  }, [onFilterChange, onValidationResult]);

  return (
    <div className="filter-input-container">
      <input
        type="text"
        value={value}
        onChange={handleChange}
        placeholder="method:GET host:api status:2*"
        className="filter-input"
      />
      <div className="filter-help">
        <span>Examples:</span>
        <code>method:POST</code>
        <code>host:*example.com</code>
        <code>status:>400</code>
      </div>
    </div>
  );
}
```

## 5. Key Files to Modify

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/filter/dsl.rs` | MODIFY | Extend lexer for column-scoped syntax |
| `src-tauri/src/filter/evaluator.rs` | MODIFY | Add column type evaluation |
| `src-tauri/src/filter/mod.rs` | MODIFY | Export new types |
| `src-tauri/src/commands/filter.rs` | CREATE | Tauri commands for filter parsing |
| `src-tauri/src/commands/mod.rs` | MODIFY | Register filter commands |
| `frontend/components/FilterInput.tsx` | CREATE/MODIFY | React filter input component |

## 6. Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_filter() {
        let result = parse("method:POST");
        assert!(result.is_ok());
        let expr = result.unwrap();
        match expr {
            FilterExpr::Field { field, op, value } => {
                assert_eq!(field, "method");
                assert_eq!(op, FilterOp::Eq);
                assert_eq!(value, "POST");
            }
            _ => panic!("Expected Field expression"),
        }
    }

    #[test]
    fn test_status_glob() {
        let result = parse("status:2*");
        assert!(result.is_ok());
        let expr = result.unwrap();
        match expr {
            FilterExpr::Field { field, op, value } => {
                assert_eq!(field, "status");
                assert_eq!(op, FilterOp::Glob);
                assert_eq!(value, "2");
            }
            _ => panic!("Expected Field expression"),
        }
    }

    #[test]
    fn test_combined_filter() {
        let result = parse("method:POST AND host:api.* AND status:2*");
        assert!(result.is_ok());
    }

    #[test]
    fn test_numeric_comparison() {
        let result = parse("status:>400");
        assert!(result.is_ok());
        let expr = result.unwrap();
        match expr {
            FilterExpr::Field { field, op, .. } => {
                assert_eq!(field, "status");
                assert_eq!(op, FilterOp::Gt);
            }
            _ => panic!("Expected Field expression"),
        }
    }
}
```

## 7. Backward Compatibility

The new syntax is fully backward compatible:
- Existing `field:value` expressions work unchanged
- Boolean operators `AND`, `OR`, `NOT` continue to work
- Parentheses grouping remains unchanged

Only new value character support and column types are added.

## 8. Performance Considerations

- Filter parsing is O(n) in expression length
- Evaluation is O(1) per request per filter expression
- Consider caching parsed expressions if same filter used repeatedly
- Use database indexes for server-side filtering when filter is pushed to backend

## 9. Timeline

| Day | Task |
|-----|------|
| 1 | Extend DSL lexer/parser for column-scoped syntax |
| 2 | Extend evaluator for new column types and operators |
| 3 | UI integration, testing, documentation |