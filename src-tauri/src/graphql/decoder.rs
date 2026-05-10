use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a parsed GraphQL operation extracted from a request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLOperation {
    /// "query", "mutation", or "subscription"
    pub operation_type: String,
    /// The operation name, if the client provided one
    pub operation_name: Option<String>,
    /// Top-level field names in the selection set
    pub root_fields: Vec<String>,
    /// Number of top-level fields
    pub field_count: usize,
    /// Whether the request included variables
    pub has_variables: bool,
}

/// Stateless decoder for GraphQL request bodies and headers.
pub struct GraphQLDecoder;

impl GraphQLDecoder {
    /// Detect if a request body is a GraphQL query.
    /// Checks for the presence of "query" or "mutation" keys in the JSON payload.
    pub fn is_graphql_body(body: &str) -> bool {
        if let Ok(val) = serde_json::from_str::<Value>(body) {
            val.get("query").is_some() || val.get("mutation").is_some()
        } else {
            false
        }
    }

    /// Detect if the request headers indicate a JSON content-type that could
    /// carry a GraphQL payload.
    pub fn is_graphql_content_type(headers: &[(String, String)]) -> bool {
        headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("content-type") && v.contains("application/json"))
    }

    /// Parse a GraphQL request body into a `GraphQLOperation`.
    ///
    /// The body must be a JSON object with a `query` field containing the
    /// GraphQL operation document string.
    pub fn parse_request(body: &str) -> Result<GraphQLOperation, String> {
        let val: Value = serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;

        let query_str = val.get("query").and_then(|q| q.as_str()).unwrap_or("");

        let operation_type = if query_str.trim_start().starts_with("mutation") {
            "mutation"
        } else if query_str.trim_start().starts_with("subscription") {
            "subscription"
        } else {
            "query"
        };

        let operation_name = val
            .get("operationName")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from);

        let has_variables = val.get("variables").map(|v| !v.is_null()).unwrap_or(false);

        let root_fields = Self::extract_root_fields(query_str);
        let field_count = root_fields.len();

        Ok(GraphQLOperation {
            operation_type: operation_type.to_string(),
            operation_name,
            root_fields,
            field_count,
            has_variables,
        })
    }

    /// Extract top-level field names from a GraphQL query string.
    ///
    /// Handles:
    /// - Simple fields: `user`, `posts`
    /// - Fields with sub-selections: `user { id name }` -> `["user"]`
    /// - Fields with arguments: `user(id: 1)` -> `["user"]`
    /// - Fields with aliases: `myUser: user` -> `["user"]`
    /// - Comma or whitespace-separated sibling fields
    fn extract_root_fields(query: &str) -> Vec<String> {
        let start = match query.find('{') {
            Some(pos) => pos,
            None => return Vec::new(),
        };

        let mut fields: Vec<String> = Vec::new();
        let mut depth: i32 = 0;
        let mut paren_depth: i32 = 0; // track () nesting within a root field
        let mut current_field = String::new();
        let mut in_field = false;
        let mut saw_separator = false; // true when whitespace seen between two tokens

        for ch in query[start..].chars() {
            match ch {
                '{' => {
                    depth += 1;
                    if depth == 1 {
                        // Opening the outermost selection set — reset state.
                        in_field = false;
                        paren_depth = 0;
                        current_field.clear();
                        saw_separator = false;
                    } else if depth == 2 && in_field {
                        // Entering a sub-selection of the current root field.
                        // Finalize the field name we've accumulated so far.
                        let field = current_field.trim().to_string();
                        if !field.is_empty() {
                            fields.push(extract_field_name(&field));
                        }
                        current_field.clear();
                        in_field = false;
                        paren_depth = 0;
                        saw_separator = false;
                    }
                }
                '}' => {
                    if depth == 1 && in_field && !current_field.is_empty() {
                        // Closing the outermost selection set — finalize the
                        // last root field.
                        let field = current_field.trim().to_string();
                        if !field.is_empty() {
                            fields.push(extract_field_name(&field));
                        }
                        current_field.clear();
                    }
                    depth -= 1;
                    if depth < 0 {
                        depth = 0; // malformed query guard
                    }
                    in_field = false;
                    paren_depth = 0;
                    saw_separator = false;
                }
                ',' if depth == 1 => {
                    // Comma separates sibling root fields.
                    if in_field && !current_field.is_empty() {
                        let field = current_field.trim().to_string();
                        if !field.is_empty() {
                            fields.push(extract_field_name(&field));
                        }
                        current_field.clear();
                    }
                    in_field = false;
                    paren_depth = 0;
                    saw_separator = false;
                }
                '(' if depth == 1 && in_field => {
                    paren_depth += 1;
                    saw_separator = false; // opening paren means args, not new field
                    current_field.push(ch);
                }
                ')' if depth == 1 && in_field => {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        paren_depth = 0;
                    }
                    current_field.push(ch);
                }
                _ if depth == 1 && ch.is_whitespace() && in_field && paren_depth == 0 => {
                    // Whitespace outside of parentheses — could be between
                    // sibling fields or before arguments/sub-selection.
                    saw_separator = true;
                }
                _ if depth == 1 && ch.is_whitespace() && in_field => {
                    // Whitespace inside parentheses — part of the arguments,
                    // push it into the field buffer.
                    current_field.push(ch);
                }
                _ if depth == 1 && ch.is_whitespace() => {
                    // Whitespace before any field — skip.
                }
                _ if depth == 1 => {
                    // A non-whitespace character at the root selection level.
                    if saw_separator && !current_field.is_empty() {
                        // Whitespace separated two field tokens. Only finalize
                        // the previous field if this is NOT an opening paren
                        // or the continuation of an alias (field ends with ':').
                        if ch != '(' && !current_field.trim().ends_with(':') {
                            let field = current_field.trim().to_string();
                            if !field.is_empty() {
                                fields.push(extract_field_name(&field));
                            }
                            current_field.clear();
                        }
                    }
                    saw_separator = false;
                    in_field = true;
                    current_field.push(ch);
                }
                _ => {
                    // Characters inside nested selections — ignore.
                }
            }
        }

        fields
    }

    /// Detect the `graphql-ws` WebSocket sub-protocol from request headers.
    pub fn is_graphql_ws(headers: &[(String, String)]) -> bool {
        headers.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("sec-websocket-protocol") && v.contains("graphql-ws")
        })
    }
}

/// Extract the canonical field name from a raw field token.
///
/// Handles:
/// - `user(id: 1)`            -> `user`
/// - `myUser: user`           -> `user`
/// - `myUser: user(id: 1)`    -> `user`
fn extract_field_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return raw.to_string();
    }

    // Strip arguments in parentheses (find the first unmatched '(')
    let without_args = if let Some(paren_pos) = find_top_level_paren(trimmed) {
        &trimmed[..paren_pos]
    } else {
        trimmed
    };

    // Handle alias: everything after the last ':' is the field name.
    // (Aliases cannot contain ':', so the last ':' is the separator.)
    if let Some(colon_pos) = without_args.rfind(':') {
        let after = without_args[colon_pos + 1..].trim();
        if after.is_empty() {
            without_args.to_string()
        } else {
            after.to_string()
        }
    } else {
        without_args.trim().to_string()
    }
}

/// Find the position of the first top-level '(' that is not inside nested
/// parentheses (used for locating the argument list start).
fn find_top_level_paren(s: &str) -> Option<usize> {
    let mut depth: i32 = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' if depth == 0 => return Some(i),
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

impl Default for GraphQLDecoder {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── detection ──────────────────────────────────────────────────────

    #[test]
    fn test_is_graphql_body() {
        assert!(GraphQLDecoder::is_graphql_body(r#"{"query": "..."}"#));
        assert!(GraphQLDecoder::is_graphql_body(
            r#"{"query": "query { user { id } }", "variables": {}}"#
        ));
        assert!(!GraphQLDecoder::is_graphql_body(r#"{"not_graphql": true}"#));
        assert!(!GraphQLDecoder::is_graphql_body("not json"));
    }

    #[test]
    fn test_detect_graphql_content_type() {
        let headers = vec![("Content-Type".into(), "application/json".into())];
        assert!(GraphQLDecoder::is_graphql_content_type(&headers));

        let headers = vec![(
            "content-type".into(),
            "application/json; charset=utf-8".into(),
        )];
        assert!(GraphQLDecoder::is_graphql_content_type(&headers));

        let headers = vec![("Content-Type".into(), "text/html".into())];
        assert!(!GraphQLDecoder::is_graphql_content_type(&headers));
    }

    #[test]
    fn test_detect_graphql_ws() {
        let headers = vec![("Sec-WebSocket-Protocol".into(), "graphql-ws".into())];
        assert!(GraphQLDecoder::is_graphql_ws(&headers));

        let bad = vec![("Sec-WebSocket-Protocol".into(), "chat".into())];
        assert!(!GraphQLDecoder::is_graphql_ws(&bad));
    }

    // ── parsing ────────────────────────────────────────────────────────

    #[test]
    fn test_parse_simple_query() {
        let body = r#"{"query": "{ user { name email } }"}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.operation_type, "query");
        assert_eq!(op.root_fields, vec!["user"]);
        assert_eq!(op.field_count, 1);
        assert!(!op.has_variables);
        assert!(op.operation_name.is_none());
    }

    #[test]
    fn test_parse_mutation() {
        let body = r#"{"query": "mutation CreateUser($name: String!) { createUser(name: $name) { id name } }", "variables": {"name": "test"}}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.operation_type, "mutation");
        assert!(op.has_variables);
        assert_eq!(op.root_fields, vec!["createUser"]);
    }

    #[test]
    fn test_parse_subscription() {
        let body = r#"{"query": "subscription { messageAdded { id text } }"}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.operation_type, "subscription");
    }

    #[test]
    fn test_parse_with_operation_name() {
        let body =
            r#"{"query": "query GetUser { user(id: 1) { id } }", "operationName": "GetUser"}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.operation_name.as_deref(), Some("GetUser"));
        assert_eq!(op.operation_type, "query");
    }

    #[test]
    fn test_parse_multiple_root_fields() {
        let body = r#"{"query": "{ user { id } posts { title } comments { text } }"}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.field_count, 3);
        assert_eq!(op.root_fields, vec!["user", "posts", "comments"]);
    }

    #[test]
    fn test_parse_fields_with_args() {
        let body = r#"{"query": "{ user(id: 42) { name } post(slug: \"hello\") { title } }"}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.root_fields, vec!["user", "post"]);
    }

    #[test]
    fn test_parse_field_with_alias() {
        let body = r#"{"query": "{ myUser: user(id: 1) { name } }"}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.root_fields, vec!["user"]);
    }

    #[test]
    fn test_parse_comma_separated_fields() {
        let body = r#"{"query": "{ user, posts, comments }"}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.root_fields, vec!["user", "posts", "comments"]);
    }

    #[test]
    fn test_parse_inline_fields_no_subselection() {
        // Sibling fields without sub-selection, separated by newlines
        let body = r#"{"query": "{\n  me\n  viewer\n}"}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.root_fields, vec!["me", "viewer"]);
    }

    #[test]
    fn test_parse_field_with_space_before_args() {
        // Space between field name and opening paren should not split
        let body = r#"{"query": "{ user (id: 1) { name } }"}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.root_fields, vec!["user"]);
    }

    #[test]
    fn test_parse_sibling_fields_space_separated() {
        // Space-separated fields on the same line
        let body = r#"{"query": "{ user posts comments }"}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.root_fields, vec!["user", "posts", "comments"]);
    }

    #[test]
    fn test_parse_empty_query_string() {
        let body = r#"{"query": ""}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert_eq!(op.field_count, 0);
        assert!(op.root_fields.is_empty());
    }

    #[test]
    fn test_parse_null_variables() {
        let body = r#"{"query": "{ a }", "variables": null}"#;
        let op = GraphQLDecoder::parse_request(body).unwrap();
        assert!(!op.has_variables);
    }
}
