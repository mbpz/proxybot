use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestNode {
    pub id: String,
    pub host: String,
    pub path: String,
    pub method: String,
    pub status: Option<u16>,
    #[serde(rename = "duration_ms")]
    pub duration_ms: u64,
    pub timestamp: i64,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub requests: Vec<RequestNode>,
    pub edges: Vec<Edge>,
}

#[tauri::command]
pub fn get_graph_data(max_requests: usize) -> Result<GraphData, String> {
    // 从 traffic state 获取请求
    // 构建节点和边
    // 返回 GraphData
    Ok(GraphData {
        requests: vec![],
        edges: vec![],
    })
}
