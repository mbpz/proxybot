//! AST types for the filter DSL.
//!
//! Shared by the parser (`dsl`), evaluator, preset storage, and
//! Tauri command surface so all layers see the same
//! serde-compatible representation.

use serde::{Deserialize, Serialize};

/// AST node for a parsed filter expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterExpr {
    /// `field:op value` predicate, e.g. `method:GET`, `host:*.example.com`,
    /// `status:>=200`.
    Field { field: String, op: FilterOp, value: String },
    /// Logical AND.
    And(Box<FilterExpr>, Box<FilterExpr>),
    /// Logical OR.
    Or(Box<FilterExpr>, Box<FilterExpr>),
    /// Logical NOT.
    Not(Box<FilterExpr>),
    /// Parenthesized grouping.
    Group(Box<FilterExpr>),
    /// Plain text search across multiple fields (host, path, body).
    Text(String),
}

/// Comparison / matching operators recognised by the DSL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOp {
    /// `:` — exact equality.
    Eq,
    /// `:*` — glob pattern (`*` and `?` wildcards).
    Glob,
    /// `:~` — regex pattern.
    Regex,
    /// `>` — numeric greater-than.
    Gt,
    /// `<` — numeric less-than.
    Lt,
    /// `>=` — numeric greater-than-or-equal.
    Gte,
    /// `<=` — numeric less-than-or-equal.
    Lte,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_expr_constructs_and_serializes() {
        let expr = FilterExpr::Field {
            field: "method".into(),
            op: FilterOp::Eq,
            value: "GET".into(),
        };
        let json = serde_json::to_string(&expr).unwrap();
        assert!(json.contains("\"Field\""), "got: {json}");
        assert!(json.contains("\"Eq\""), "got: {json}");
    }

    #[test]
    fn op_variants_match_spec() {
        // Lock in all 7 op variants and their distinctness.
        assert_ne!(FilterOp::Eq, FilterOp::Glob);
        assert_ne!(FilterOp::Regex, FilterOp::Gt);
        assert_ne!(FilterOp::Lt, FilterOp::Gte);
        assert_ne!(FilterOp::Lte, FilterOp::Eq);
        assert_eq!(FilterOp::Eq, FilterOp::Eq);
    }
}