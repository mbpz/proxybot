use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsFrame {
    pub id: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub direction: FrameDirection,
    pub opcode: u8,
    pub payload: String,
    #[serde(rename = "payloadText")]
    pub payload_text: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameDirection {
    Incoming,
    Outgoing,
}

#[tauri::command]
pub fn get_ws_frames(request_id: String) -> Result<Vec<WsFrame>, String> {
    // 从连接状态获取该请求关联的WS帧
    // 返回帧列表
    Ok(vec![])
}

#[tauri::command]
pub fn subscribe_ws_frames(request_id: String) -> Result<Channel<WsFrame>, String> {
    // 创建channel用于实时推送帧
    Err("Not implemented".to_string())
}