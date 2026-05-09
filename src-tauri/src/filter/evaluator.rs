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
