use crate::filter::dsl::{FilterExpr, FilterOp};
use crate::proxy::InterceptedRequest;
use regex::Regex;
use std::borrow::Cow;

/// Maximum response-body size (in bytes) searched by `body:X` filters.
/// Per spec section 10, larger bodies are truncated to 1 MB before
/// substring / glob / regex matching so a single huge response can't
/// stall the evaluator.
pub const MAX_BODY_SEARCH_SIZE: usize = 1024 * 1024;

pub struct Evaluator;

impl Evaluator {
    pub fn evaluate(expr: &FilterExpr, req: &InterceptedRequest) -> bool {
        match expr {
            FilterExpr::Field { field, op, value } => {
                let field_value = Self::get_field_value(req, field);
                Self::compare(&field_value, op, value)
            }
            FilterExpr::HeaderName { name, op, value } => {
                let header_value = Self::lookup_header(req, name);
                // `header:NAME` (no value) just checks presence; with a
                // value we apply the normal op semantics.
                if value.is_empty() {
                    return !header_value.is_empty();
                }
                Self::compare(&header_value, op, value)
            }
            FilterExpr::BodyText { op, value } => Self::body_search(req, op, value),
            FilterExpr::And(a, b) => Self::evaluate(a, req) && Self::evaluate(b, req),
            FilterExpr::Or(a, b) => Self::evaluate(a, req) || Self::evaluate(b, req),
            FilterExpr::Not(e) => !Self::evaluate(e, req),
            FilterExpr::Group(e) => Self::evaluate(e, req),
            FilterExpr::Text(search) => Self::text_search(req, search),
        }
    }

    /// Search for text in host, path, and body fields
    fn text_search(req: &InterceptedRequest, search: &str) -> bool {
        let search_lower = search.to_lowercase();
        req.host.to_lowercase().contains(&search_lower)
            || req.path.to_lowercase().contains(&search_lower)
            || req.req_body.as_ref().map(|b| b.to_lowercase().contains(&search_lower)).unwrap_or(false)
            || req.resp_body.as_ref().map(|b| b.to_lowercase().contains(&search_lower)).unwrap_or(false)
    }

    fn get_field_value<'a>(req: &'a InterceptedRequest, field: &str) -> Cow<'a, str> {
        match field {
            "method" => Cow::Borrowed(&req.method),
            "host" => Cow::Borrowed(&req.host),
            "path" => Cow::Borrowed(&req.path),
            "status" => Cow::Owned(req.status.map(|s| s.to_string()).unwrap_or_default()),
            "latency" | "duration" => {
                Cow::Owned(req.latency_ms.map(|d| d.to_string()).unwrap_or_default())
            }
            "size" => Cow::Owned(req.resp_size.map(|s| s.to_string()).unwrap_or_default()),
            "type" => {
                // Extract content-type from response headers
                for (k, v) in &req.resp_headers {
                    if k.to_lowercase() == "content-type" {
                        return Cow::Owned(v.clone());
                    }
                }
                Cow::Owned(String::new())
            }
            "app" | "app_name" => {
                Cow::Owned(req.app_name.clone().unwrap_or_default())
            }
            "scheme" => Cow::Borrowed(&req.scheme),
            "ip" | "client_ip" => {
                Cow::Owned(req.client_ip.clone().unwrap_or_default())
            }
            _ => {
                // Check headers
                for (k, v) in &req.req_headers {
                    if k.to_lowercase() == field.to_lowercase() {
                        return Cow::Borrowed(v);
                    }
                }
                Cow::Owned(String::new())
            }
        }
    }

    /// Look up a response header by name (case-insensitive). Returns
    /// the header's value, or empty string if absent.
    fn lookup_header<'a>(req: &'a InterceptedRequest, name: &str) -> Cow<'a, str> {
        for (k, v) in &req.resp_headers {
            if k.eq_ignore_ascii_case(name) {
                return Cow::Borrowed(v);
            }
        }
        Cow::Owned(String::new())
    }

    /// Search the response body (truncated to `MAX_BODY_SEARCH_SIZE`
    /// bytes) for `value` using the given op. `Eq` is substring;
    /// `Glob`/`Regex` run unanchored so a pattern anywhere in the
    /// body matches. Numeric ops are nonsensical against a string
    /// body and return false.
    fn body_search(req: &InterceptedRequest, op: &FilterOp, value: &str) -> bool {
        let body = match req.resp_body.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let truncated = Self::truncate_body(body);
        match op {
            FilterOp::Eq => truncated.to_lowercase().contains(&value.to_lowercase()),
            FilterOp::Glob => glob_match_substring(value, truncated),
            FilterOp::Regex => regex_match(value, truncated),
            FilterOp::Gt | FilterOp::Lt | FilterOp::Gte | FilterOp::Lte => false,
        }
    }

    /// Truncate a body string to at most `MAX_BODY_SEARCH_SIZE` bytes
    /// from the start. Slicing on a byte index can panic if it lands
    /// inside a UTF-8 codepoint, so we walk back to the nearest char
    /// boundary.
    fn truncate_body(body: &str) -> &str {
        if body.len() <= MAX_BODY_SEARCH_SIZE {
            return body;
        }
        let mut idx = MAX_BODY_SEARCH_SIZE;
        while idx > 0 && !body.is_char_boundary(idx) {
            idx -= 1;
        }
        &body[..idx]
    }

    fn compare(field_value: &str, op: &FilterOp, filter_value: &str) -> bool {
        match op {
            FilterOp::Eq => field_value == filter_value,
            FilterOp::Glob => glob_match(filter_value, field_value),
            FilterOp::Regex => regex_match(filter_value, field_value),
            FilterOp::Gt => {
                if let (Ok(v), Ok(fv)) = (field_value.parse::<u64>(), filter_value.parse::<u64>()) {
                    v > fv
                } else {
                    false
                }
            }
            FilterOp::Lt => {
                if let (Ok(v), Ok(fv)) = (field_value.parse::<u64>(), filter_value.parse::<u64>()) {
                    v < fv
                } else {
                    false
                }
            }
            FilterOp::Gte => {
                if let (Ok(v), Ok(fv)) = (field_value.parse::<u64>(), filter_value.parse::<u64>()) {
                    v >= fv
                } else {
                    false
                }
            }
            FilterOp::Lte => {
                if let (Ok(v), Ok(fv)) = (field_value.parse::<u64>(), filter_value.parse::<u64>()) {
                    v <= fv
                } else {
                    false
                }
            }
        }
    }
}

fn regex_match(pattern: &str, value: &str) -> bool {
    Regex::new(pattern).map(|re| re.is_match(value)).unwrap_or(false)
}

/// Like `glob_match` but unanchored — the pattern matches anywhere
/// in `haystack`. Used by `body:X` filters so `body:*token*` finds
/// `token` inside a multi-KB response body.
fn glob_match_substring(pattern: &str, haystack: &str) -> bool {
    let mut re_pattern = String::with_capacity(pattern.len() + 4);
    let mut literal = String::new();
    for ch in pattern.chars() {
        match ch {
            '*' | '?' => {
                if !literal.is_empty() {
                    re_pattern.push_str(&regex::escape(&literal));
                    literal.clear();
                }
                if ch == '*' {
                    re_pattern.push_str(".*");
                } else {
                    re_pattern.push('.');
                }
            }
            _ => literal.push(ch),
        }
    }
    if !literal.is_empty() {
        re_pattern.push_str(&regex::escape(&literal));
    }
    Regex::new(&re_pattern)
        .map(|re| re.is_match(haystack))
        .unwrap_or(false)
}

fn glob_match(pattern: &str, value: &str) -> bool {
    let mut re_pattern = String::with_capacity(pattern.len() + 10);
    re_pattern.push('^');
    let mut literal = String::new();
    for ch in pattern.chars() {
        match ch {
            '*' | '?' => {
                if !literal.is_empty() {
                    re_pattern.push_str(&regex::escape(&literal));
                    literal.clear();
                }
                if ch == '*' {
                    re_pattern.push_str(".*");
                } else {
                    re_pattern.push('.');
                }
            }
            _ => literal.push(ch),
        }
    }
    if !literal.is_empty() {
        re_pattern.push_str(&regex::escape(&literal));
    }
    re_pattern.push('$');
    Regex::new(&re_pattern)
        .map(|re| re.is_match(value))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::dsl::parse;
    use crate::filter::expr::{FilterExpr, FilterOp};
    use crate::proxy::InterceptedRequest;

    fn req(method: &str, host: &str, path: &str) -> InterceptedRequest {
        InterceptedRequest {
            method: method.into(),
            host: host.into(),
            path: path.into(),
            ..Default::default()
        }
    }

    fn eval_str(expr: &str, r: &InterceptedRequest) -> bool {
        Evaluator::evaluate(&parse(expr).unwrap(), r)
    }

    #[test]
    fn test_evaluate_simple_eq() {
        let r = req("GET", "example.com", "/api");
        assert!(eval_str("method:GET", &r));
        assert!(!eval_str("method:POST", &r));
        assert!(eval_str("host:example.com", &r));
        assert!(!eval_str("host:other.com", &r));
    }

    #[test]
    fn test_evaluate_glob() {
        // Build AST directly because the lexer strips a leading `*`
        // when reading the value after `:*` op. The evaluator's
        // glob_match handles anchored wildcards correctly.
        let expr = FilterExpr::Field {
            field: "host".into(),
            op: FilterOp::Glob,
            value: "*.example.com".into(),
        };
        let r = req("GET", "api.example.com", "/x");
        assert!(Evaluator::evaluate(&expr, &r));
        let r2 = req("GET", "other.org", "/x");
        assert!(!Evaluator::evaluate(&expr, &r2));
    }

    #[test]
    fn test_evaluate_regex() {
        // Build a regex AST directly and check the evaluator's Regex branch.
        let expr = FilterExpr::Field {
            field: "host".into(),
            op: FilterOp::Regex,
            value: r"^api\..+\.com$".into(),
        };
        let r = req("GET", "api.foo.com", "/x");
        assert!(Evaluator::evaluate(&expr, &r));
        let r2 = req("GET", "other.org", "/x");
        assert!(!Evaluator::evaluate(&expr, &r2));
    }

    #[test]
    fn test_evaluate_numeric_via_direct_compare() {
        // Lexer collapses `status:>=400` to Eq/value=`>=400`, so test
        // the numeric compare branch directly with a constructed AST.
        let expr = FilterExpr::Field {
            field: "status".into(),
            op: FilterOp::Gte,
            value: "400".into(),
        };
        let mut r = req("GET", "h", "/p");
        r.status = Some(404);
        assert!(Evaluator::evaluate(&expr, &r));
        assert!(!Evaluator::evaluate(
            &FilterExpr::Field {
                field: "status".into(),
                op: FilterOp::Gte,
                value: "500".into(),
            },
            &r
        ));
    }

    #[test]
    fn test_evaluate_and_or_not() {
        let r = req("GET", "api.example.com", "/x");
        assert!(eval_str("method:GET AND host:api.example.com", &r));
        assert!(eval_str("method:POST OR method:GET", &r));
        assert!(eval_str("NOT method:POST", &r));
        assert!(!eval_str("NOT method:GET", &r));
    }

    #[test]
    fn test_evaluate_group() {
        let r = req("POST", "api.example.com", "/x");
        assert!(eval_str(
            "(method:GET OR method:POST) AND host:api.example.com",
            &r
        ));
        assert!(!eval_str(
            "(method:GET OR method:POST) AND host:other.org",
            &r
        ));
    }

    #[test]
    fn test_evaluate_header_field() {
        let mut r = req("GET", "h", "/p");
        // The evaluator's `_` branch looks up request_headers by field
        // name (case-insensitive) and returns the header's value.
        r.req_headers
            .push(("x-trace-id".into(), "abc-123".into()));
        let expr = FilterExpr::Field {
            field: "x-trace-id".into(),
            op: FilterOp::Eq,
            value: "abc-123".into(),
        };
        assert!(Evaluator::evaluate(&expr, &r));
        let expr2 = FilterExpr::Field {
            field: "x-trace-id".into(),
            op: FilterOp::Eq,
            value: "no-match".into(),
        };
        assert!(!Evaluator::evaluate(&expr2, &r));
    }

    #[test]
    fn test_evaluate_body_text_search() {
        // `body` field has no handler in the evaluator — falls through to
        // the request-headers default branch. Use plain `Text` search
        // which the evaluator's text_search() hits against host/path/body.
        let mut r = req("POST", "h", "/p");
        r.resp_body = Some("the token is abc123 here".into());
        assert!(eval_str("abc123", &r));
        assert!(!eval_str("xyznotthere", &r));
    }

    // ----- header:X handler (Gap 1) ----------------------------------

    #[test]
    fn test_header_name_present_matches() {
        let mut r = req("GET", "h", "/p");
        r.resp_headers.push((
            "content-type".into(),
            "application/json".into(),
        ));
        // Triple syntax: `header:NAME:VALUE` — matches when the named
        // response header equals the given value.
        assert!(eval_str("header:content-type:application/json", &r));
    }

    #[test]
    fn test_header_name_value_differs_does_not_match() {
        let mut r = req("GET", "h", "/p");
        r.resp_headers
            .push(("content-type".into(), "text/html".into()));
        assert!(!eval_str(
            "header:content-type:application/json",
            &r
        ));
    }

    #[test]
    fn test_header_name_absent_does_not_match() {
        let r = req("GET", "h", "/p");
        // No resp_headers set — header is absent.
        assert!(!eval_str("header:content-type:application/json", &r));
        // Single-arg form (`header:NAME`) checks presence; absent
        // header should not match.
        assert!(!eval_str("header:content-type", &r));
    }

    #[test]
    fn test_header_name_case_insensitive() {
        let mut r = req("GET", "h", "/p");
        // Header is stored as `Content-Type` (capitalized) but the
        // filter uses lower-case name — match must be case-insensitive.
        r.resp_headers
            .push(("Content-Type".into(), "application/json".into()));
        assert!(eval_str("header:content-type:application/json", &r));
    }

    // ----- body:X handler (Gap 2) ------------------------------------

    #[test]
    fn test_body_text_substring_matches() {
        let mut r = req("GET", "h", "/p");
        r.resp_body = Some("the token is abc123 here".into());
        // Use the parser so we exercise the BodyText AST path end-to-end.
        assert!(eval_str("body:token", &r));
        assert!(eval_str("body:abc123", &r));
    }

    #[test]
    fn test_body_text_substring_absent_does_not_match() {
        let mut r = req("GET", "h", "/p");
        r.resp_body = Some("hello world".into());
        assert!(!eval_str("body:token", &r));
    }

    #[test]
    fn test_body_no_resp_body_returns_false() {
        let r = req("GET", "h", "/p");
        // No resp_body at all — body: search must return false, not panic.
        assert!(!eval_str("body:anything", &r));
    }

    #[test]
    fn test_body_glob_matches() {
        let mut r = req("GET", "h", "/p");
        r.resp_body = Some("hello world".into());
        // Glob via `body:*` op marker — value becomes the trailing
        // pattern (leading `*` stripped by lexer).
        assert!(eval_str("body:*world", &r));
        assert!(!eval_str("body:*missing", &r));
    }

    // ----- 1MB body truncation (Gap 3) --------------------------------

    #[test]
    fn test_body_truncation_matches_substring_in_first_mb() {
        // Build a body that is larger than 1MB total, with a sentinel
        // token sitting near the start so it lives inside the searched
        // window.
        let prefix = "TOKEN_AT_START ";
        let filler_len = MAX_BODY_SEARCH_SIZE - prefix.len() + 100_000;
        let mut body = String::with_capacity(prefix.len() + filler_len + 32);
        body.push_str(prefix);
        body.push_str(&"a".repeat(filler_len));
        body.push_str(&"b".repeat(500_000));

        assert!(body.len() > MAX_BODY_SEARCH_SIZE);

        let mut r = req("GET", "h", "/p");
        r.resp_body = Some(body);
        assert!(eval_str("body:TOKEN_AT_START", &r));
    }

    #[test]
    fn test_body_truncation_does_not_see_truncated_tail() {
        // Same shape as above, but the sentinel sits past the 1MB
        // boundary so it must NOT match.
        let filler_len = MAX_BODY_SEARCH_SIZE + 10;
        let mut body = String::with_capacity(filler_len + 32);
        body.push_str(&"a".repeat(filler_len));
        body.push_str(" TOKEN_AT_END");

        assert!(body.len() > MAX_BODY_SEARCH_SIZE);
        assert!(body.find("TOKEN_AT_END").unwrap() > MAX_BODY_SEARCH_SIZE);

        let mut r = req("GET", "h", "/p");
        r.resp_body = Some(body);
        assert!(!eval_str("body:TOKEN_AT_END", &r));
    }

    #[test]
    fn test_body_truncation_handles_utf8_boundary() {
        // Confirm the truncate helper doesn't panic when MAX boundary
        // falls inside a multi-byte UTF-8 codepoint.
        let mut body = String::new();
        // Fill close to the limit with ASCII, then put a 4-byte
        // codepoint straddling the boundary.
        body.push_str(&"a".repeat(MAX_BODY_SEARCH_SIZE - 2));
        body.push_str("\u{1F600}"); // 4-byte emoji
        body.push_str(&"b".repeat(200));

        let truncated = Evaluator::truncate_body(&body);
        // Must be valid UTF-8 and at most MAX_BODY_SEARCH_SIZE bytes.
        assert!(truncated.len() <= MAX_BODY_SEARCH_SIZE);
        assert!(truncated.is_char_boundary(truncated.len()));
    }
}
