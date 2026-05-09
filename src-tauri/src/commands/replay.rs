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
pub fn get_replay_targets() -> Result<Vec<ReplayTarget>, String> {
    // 从配置加载保存的目标
    Ok(vec![])
}

#[tauri::command]
pub fn save_replay_target(target: ReplayTarget) -> Result<(), String> {
    // 保存到配置
    Ok(())
}

#[tauri::command]
pub fn delete_replay_target(id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn toggle_replay_target(id: String, enabled: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn execute_replay(targets: Vec<ReplayTarget>) -> Result<Vec<ReplayResult>, String> {
    let mut results = Vec::new();

    for target in targets {
        let result = crate::replay::engine::execute_target(&target).await;
        results.push(result);
    }

    Ok(results)
}