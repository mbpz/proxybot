use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayTarget {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<String>,
    #[serde(rename = "expected_status")]
    pub expected_status: Option<u16>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    #[serde(rename = "target_id")]
    pub target_id: String,
    pub status: u16,
    #[serde(rename = "duration_ms")]
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub fn save_replay_target(target: ReplayTarget) -> Result<(), String> {
    // 保存到配置
    Ok(())
}

#[tauri::command]
pub fn delete_replay_target(_id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn toggle_replay_target(_id: String, _enabled: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn execute_replay(targets: Vec<ReplayTarget>) -> Result<Vec<ReplayResult>, String> {
    let mut results = Vec::new();
    let client = reqwest::Client::new();
    for target in targets {
        let start = std::time::Instant::now();
        let result = match execute_one(&client, &target).await {
            Ok(response) => {
                let status = response.status().as_u16();
                ReplayResult {
                    target_id: target.id.clone(),
                    status,
                    duration_ms: start.elapsed().as_millis() as u64,
                    success: target.expected_status.map(|e| status == e).unwrap_or(status < 400),
                    error: None,
                }
            }
            Err(e) => ReplayResult {
                target_id: target.id.clone(),
                status: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                success: false,
                error: Some(e.to_string()),
            },
        };
        results.push(result);
    }
    Ok(results)
}

async fn execute_one(
    client: &reqwest::Client,
    target: &ReplayTarget,
) -> Result<reqwest::Response, reqwest::Error> {
    let method = reqwest::Method::from_bytes(target.method.as_bytes())
        .unwrap_or(reqwest::Method::GET);
    let mut req = client.request(method, &target.url);
    for (k, v) in &target.headers {
        req = req.header(k.as_str(), v.as_str());
    }
    if let Some(body) = &target.body {
        req = req.body(body.clone());
    }
    req.send().await
}