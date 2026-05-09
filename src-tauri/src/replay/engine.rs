use crate::commands::replay::{ReplayResult, ReplayTarget};
use std::time::Instant;

pub async fn execute_target(target: &ReplayTarget) -> ReplayResult {
    let start = Instant::now();

    // 构建请求
    let client = reqwest::Client::new();
    let mut request = client.request(
        reqwest::Method::from_str(&target.method),
        &target.url,
    );

    for (key, value) in &target.headers {
        request = request.header(key, value);
    }

    if let Some(body) = &target.body {
        request = request.body(body);
    }

    match request.send().await {
        Ok(response) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let status = response.status().as_u16();
            let success = target.expected_status.map(|e| status == e).unwrap_or(status < 400);

            ReplayResult {
                target_id: target.id.clone(),
                status,
                duration_ms,
                success,
                error: None,
            }
        }
        Err(e) => {
            ReplayResult {
                target_id: target.id.clone(),
                status: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                success: false,
                error: Some(e.to_string()),
            }
        }
    }
}