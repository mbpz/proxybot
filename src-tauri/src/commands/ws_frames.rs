use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::DbState;

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

fn parse_timestamp(ts: &str) -> i64 {
    if let Ok(f) = ts.parse::<f64>() {
        return f as i64;
    }
    chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%d %H:%M:%S")
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0)
}

#[tauri::command]
pub fn get_ws_frames(
    db_state: State<'_, Arc<DbState>>,
    request_id: String,
) -> Result<Vec<WsFrame>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"SELECT id, request_id, direction, opcode, payload, size, timestamp
               FROM ws_frames
               WHERE request_id = ?1
               ORDER BY id ASC"#,
        )
        .map_err(|e| e.to_string())?;

    let frames = stmt
        .query_map(rusqlite::params![request_id], |row| {
            let id: i64 = row.get(0)?;
            let req_id: String = row.get(1)?;
            let direction_str: String = row.get(2)?;
            let opcode: u8 = row.get(3)?;
            let payload: String = row.get(4)?;
            let _size: i64 = row.get(5)?;
            let timestamp_str: String = row.get(6)?;

            let direction = match direction_str.as_str() {
                "incoming" => FrameDirection::Incoming,
                _ => FrameDirection::Outgoing,
            };

            Ok(WsFrame {
                id: id.to_string(),
                request_id: req_id,
                direction,
                opcode,
                payload_text: Some(payload.clone()),
                payload,
                timestamp: parse_timestamp(&timestamp_str),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(frames)
}
