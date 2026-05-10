use crate::filter::dsl::{FilterExpr, FilterOp};
use crate::proxy::InterceptedRequest;
use regex::Regex;
use std::borrow::Cow;

pub struct Evaluator;

impl Evaluator {
    pub fn evaluate(expr: &FilterExpr, req: &InterceptedRequest) -> bool {
        match expr {
            FilterExpr::Field { field, op, value } => {
                let field_value = Self::get_field_value(req, field);
                Self::compare(&field_value, op, value)
            }
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
