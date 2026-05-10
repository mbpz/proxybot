use crate::filter::dsl::{FilterExpr, FilterOp};
use serde::Deserialize;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
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
            FilterExpr::And(a, b) => Self::evaluate(a, req) && Self::evaluate(b, req),
            FilterExpr::Or(a, b) => Self::evaluate(a, req) || Self::evaluate(b, req),
            FilterExpr::Not(e) => !Self::evaluate(e, req),
            FilterExpr::Group(e) => Self::evaluate(e, req),
        }
    }

    fn get_field_value<'a>(req: &'a InterceptedRequest, field: &str) -> Cow<'a, str> {
        match field {
            "method" => Cow::Borrowed(&req.method),
            "host" => Cow::Borrowed(&req.host),
            "path" => Cow::Borrowed(&req.path),
            "status" => Cow::Owned(req.status.map(|s| s.to_string()).unwrap_or_default()),
            "duration" => Cow::Owned(req.duration_ms.to_string()),
            _ => Cow::Owned(req.headers.get(field).cloned().unwrap_or_default()),
        }
    }

    fn compare(field_value: &str, op: &FilterOp, filter_value: &str) -> bool {
        match op {
            FilterOp::Eq => field_value == filter_value,
            FilterOp::Glob => glob_match(filter_value, field_value),
            FilterOp::Regex => regex_match(filter_value, field_value),
            FilterOp::Gt => match (field_value.parse::<u64>(), filter_value.parse::<u64>()) {
                (Ok(v), Ok(fv)) => v > fv,
                _ => false,
            },
            FilterOp::Lt => match (field_value.parse::<u64>(), filter_value.parse::<u64>()) {
                (Ok(v), Ok(fv)) => v < fv,
                _ => false,
            },
            FilterOp::Gte => match (field_value.parse::<u64>(), filter_value.parse::<u64>()) {
                (Ok(v), Ok(fv)) => v >= fv,
                _ => false,
            },
            FilterOp::Lte => match (field_value.parse::<u64>(), filter_value.parse::<u64>()) {
                (Ok(v), Ok(fv)) => v <= fv,
                _ => false,
            },
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

fn glob_match(pattern: &str, value: &str) -> bool {
    let mut re_pattern = String::with_capacity(pattern.len() + 2);
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
    regex::Regex::new(&re_pattern)
        .map(|re| re.is_match(value))
        .unwrap_or(false)
}
