//! Captured Request query semantics.
//!
//! This Module compiles Filter DSL once, combines structured Traffic filters,
//! applies stable ordering, computes the filtered count, and only then pages
//! the result. Persistence and Capture Event Adapters both supply the same
//! `InterceptedRequest` subject, so matching rules cannot drift by source.

use std::cmp::Ordering;

use proxybot_core::InterceptedRequest;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::dsl;
use super::evaluator::Evaluator;
use super::expr::{FilterExpr, FilterOp};

const MAX_PAGE_SIZE: i64 = 500;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrafficOrder {
    #[default]
    Newest,
    Oldest,
    Slowest,
    Largest,
}

impl proxybot_core::desktop_contract::WireType for TrafficOrder {
    fn type_script_type() -> String {
        "\"newest\" | \"oldest\" | \"slowest\" | \"largest\"".to_owned()
    }
}

proxybot_core::desktop_contract_type! {
    /// One complete query over Captured Requests.
    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct TrafficQuery {
        pub expression: String,
        pub method: Option<String>,
        pub host: Option<String>,
        pub status: Option<u16>,
        pub application: Option<String>,
        pub search: Option<String>,
        pub order: TrafficOrder,
        pub page: i64,
        pub page_size: i64,
    }
}

#[derive(Clone, Debug)]
pub struct QueryPage {
    pub records: Vec<InterceptedRequest>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub has_more: bool,
}

/// A query whose DSL and expensive matcher syntax have already been checked.
pub struct CompiledTrafficQuery {
    expression: Option<FilterExpr>,
    method: Option<String>,
    host: Option<String>,
    status: Option<u16>,
    application: Option<String>,
    search: Option<FilterExpr>,
    order: TrafficOrder,
    page: i64,
    page_size: i64,
}

impl CompiledTrafficQuery {
    pub fn compile(query: &TrafficQuery) -> Result<Self, String> {
        let expression = if query.expression.trim().is_empty() {
            None
        } else {
            let expression = dsl::parse(query.expression.trim())?;
            validate_expression(&expression)?;
            Some(expression)
        };
        let search = query
            .search
            .as_ref()
            .filter(|search| !search.is_empty())
            .map(|search| FilterExpr::Text(search.clone()));
        Ok(Self {
            expression,
            method: query.method.clone().filter(|value| !value.is_empty()),
            host: query.host.clone().filter(|value| !value.is_empty()),
            status: query.status,
            application: query.application.clone().filter(|value| !value.is_empty()),
            search,
            order: query.order,
            page: query.page.max(0),
            page_size: query.page_size.clamp(0, MAX_PAGE_SIZE),
        })
    }

    pub fn execute(&self, records: impl IntoIterator<Item = InterceptedRequest>) -> QueryPage {
        let mut matching: Vec<_> = records
            .into_iter()
            .filter(|request| self.matches(request))
            .collect();
        matching.sort_by(|left, right| self.compare(left, right));

        let total = matching.len() as i64;
        let start = self.page.saturating_mul(self.page_size) as usize;
        let end = start
            .saturating_add(self.page_size as usize)
            .min(matching.len());
        let records = if self.page_size == 0 || start >= matching.len() {
            Vec::new()
        } else {
            matching.drain(start..end).collect()
        };
        QueryPage {
            records,
            total,
            page: self.page,
            page_size: self.page_size,
            has_more: self.page_size > 0 && end < total as usize,
        }
    }

    pub fn matches(&self, request: &InterceptedRequest) -> bool {
        self.method
            .as_ref()
            .is_none_or(|method| request.method == *method)
            && self
                .host
                .as_ref()
                .is_none_or(|host| structured_host_matches(host, &request.host))
            && self
                .status
                .is_none_or(|status| request.status == Some(status))
            && self
                .application
                .as_ref()
                .is_none_or(|application| request.app_name.as_ref() == Some(application))
            && self
                .search
                .as_ref()
                .is_none_or(|search| Evaluator::evaluate(search, request))
            && self
                .expression
                .as_ref()
                .is_none_or(|expression| Evaluator::evaluate(expression, request))
    }

    fn compare(&self, left: &InterceptedRequest, right: &InterceptedRequest) -> Ordering {
        match self.order {
            TrafficOrder::Newest => compare_ids(right, left),
            TrafficOrder::Oldest => compare_ids(left, right),
            TrafficOrder::Slowest => right
                .latency_ms
                .unwrap_or_default()
                .cmp(&left.latency_ms.unwrap_or_default())
                .then_with(|| compare_ids(right, left)),
            TrafficOrder::Largest => response_size(right)
                .cmp(&response_size(left))
                .then_with(|| compare_ids(right, left)),
        }
    }
}

fn structured_host_matches(pattern: &str, host: &str) -> bool {
    if pattern.contains('*') || pattern.contains('?') {
        Evaluator::evaluate(
            &FilterExpr::Field {
                field: "host".to_owned(),
                op: FilterOp::Glob,
                value: pattern.to_owned(),
            },
            &InterceptedRequest {
                host: host.to_owned(),
                ..Default::default()
            },
        )
    } else {
        host.contains(pattern)
    }
}

fn compare_ids(left: &InterceptedRequest, right: &InterceptedRequest) -> Ordering {
    match (left.id.parse::<i64>(), right.id.parse::<i64>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.id.cmp(&right.id),
    }
}

fn response_size(request: &InterceptedRequest) -> usize {
    request
        .resp_size
        .or_else(|| request.resp_body.as_ref().map(String::len))
        .unwrap_or_default()
}

fn validate_expression(expression: &FilterExpr) -> Result<(), String> {
    match expression {
        FilterExpr::Field { op, value, .. }
        | FilterExpr::HeaderName { op, value, .. }
        | FilterExpr::BodyText { op, value } => validate_matcher(*op, value),
        FilterExpr::And(left, right) | FilterExpr::Or(left, right) => {
            validate_expression(left)?;
            validate_expression(right)
        }
        FilterExpr::Not(expression) | FilterExpr::Group(expression) => {
            validate_expression(expression)
        }
        FilterExpr::Text(_) => Ok(()),
    }
}

fn validate_matcher(operator: FilterOp, value: &str) -> Result<(), String> {
    match operator {
        FilterOp::Regex => Regex::new(value)
            .map(|_| ())
            .map_err(|error| format!("Invalid regular expression: {error}")),
        FilterOp::Gt | FilterOp::Lt | FilterOp::Gte | FilterOp::Lte => value
            .parse::<u64>()
            .map(|_| ())
            .map_err(|_| format!("Expected a non-negative integer, got: {value}")),
        FilterOp::Eq | FilterOp::Glob => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::CapturedRequestRecord;

    fn request(id: i64, method: &str, host: &str, status: u16) -> InterceptedRequest {
        InterceptedRequest {
            id: id.to_string(),
            timestamp: format!("170406720{id}.000"),
            method: method.to_owned(),
            host: host.to_owned(),
            path: format!("/items/{id}"),
            status: Some(status),
            latency_ms: Some(id as u64 * 10),
            scheme: "https".to_owned(),
            resp_size: Some(id as usize * 100),
            app_name: Some("Example".to_owned()),
            client_ip: Some("10.0.0.2".to_owned()),
            ..Default::default()
        }
    }

    #[test]
    fn filters_before_counting_and_pagination() {
        let query = TrafficQuery {
            expression: "status:>=400".to_owned(),
            order: TrafficOrder::Newest,
            page_size: 2,
            ..Default::default()
        };
        let page = CompiledTrafficQuery::compile(&query).unwrap().execute([
            request(1, "GET", "api.example.com", 200),
            request(2, "POST", "api.example.com", 500),
            request(3, "GET", "cdn.example.com", 404),
            request(4, "GET", "api.example.com", 201),
            request(5, "POST", "api.example.com", 503),
        ]);

        assert_eq!(page.total, 3);
        assert_eq!(
            page.records
                .iter()
                .map(|record| &record.id)
                .collect::<Vec<_>>(),
            ["5", "3"]
        );
        assert!(page.has_more);
    }

    #[test]
    fn combines_structured_filters_with_dsl_and_search() {
        let query = TrafficQuery {
            expression: "status:<500".to_owned(),
            method: Some("GET".to_owned()),
            host: Some("*.example.com".to_owned()),
            application: Some("Example".to_owned()),
            search: Some("items/3".to_owned()),
            page_size: 50,
            ..Default::default()
        };
        let page = CompiledTrafficQuery::compile(&query).unwrap().execute([
            request(2, "POST", "api.example.com", 400),
            request(3, "GET", "cdn.example.com", 404),
        ]);
        assert_eq!(page.records.len(), 1);
        assert_eq!(page.records[0].id, "3");
    }

    #[test]
    fn sorting_and_pagination_boundaries_are_stable() {
        let records = [
            request(1, "GET", "one.example.com", 200),
            request(2, "GET", "two.example.com", 200),
            request(3, "GET", "three.example.com", 200),
            request(4, "GET", "four.example.com", 200),
        ];
        let query = TrafficQuery {
            order: TrafficOrder::Oldest,
            page: 1,
            page_size: 2,
            ..Default::default()
        };
        let page = CompiledTrafficQuery::compile(&query)
            .unwrap()
            .execute(records.clone());
        assert_eq!(
            page.records
                .iter()
                .map(|record| &record.id)
                .collect::<Vec<_>>(),
            ["3", "4"]
        );
        assert!(!page.has_more);

        let zero = CompiledTrafficQuery::compile(&TrafficQuery {
            page_size: 0,
            ..Default::default()
        })
        .unwrap()
        .execute(records);
        assert_eq!(zero.total, 4);
        assert!(zero.records.is_empty());
        assert!(!zero.has_more);
    }

    #[test]
    fn malformed_expressions_fail_the_query() {
        for expression in [
            "method:GET trailing:",
            "status:>=many",
            "path:~[unterminated",
            "(method:GET",
        ] {
            let error = CompiledTrafficQuery::compile(&TrafficQuery {
                expression: expression.to_owned(),
                ..Default::default()
            })
            .err()
            .unwrap();
            assert!(!error.is_empty(), "{expression} should fail");
        }
    }

    #[test]
    fn live_and_persisted_adapters_match_identically() {
        let live = request(7, "POST", "api.example.com", 201);
        let persisted = CapturedRequestRecord {
            id: 7,
            timestamp: live.timestamp.clone(),
            method: live.method.clone(),
            scheme: live.scheme.clone(),
            host: live.host.clone(),
            path: live.path.clone(),
            request_headers: live.req_headers.clone(),
            request_body: live.req_body.as_ref().map(|body| body.as_bytes().to_vec()),
            response_status: live.status,
            response_headers: live.resp_headers.clone(),
            response_body: live.resp_body.as_ref().map(|body| body.as_bytes().to_vec()),
            duration_ms: live.latency_ms.map(|value| value as i64),
            device_id: live.device_id,
            app_tag: live.app_name.clone(),
            response_size: live.resp_size.map(|value| value as i64),
            is_websocket: live.is_websocket,
            session_id: None,
            client_ip: live.client_ip.clone(),
            upstream_ip: live.upstream_ip.clone(),
        }
        .as_intercepted();
        let query = CompiledTrafficQuery::compile(&TrafficQuery {
            expression: "method:POST AND client_ip:10.0.0.2".to_owned(),
            page_size: 50,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(query.matches(&live), query.matches(&persisted));
        assert!(query.matches(&live));
    }
}
