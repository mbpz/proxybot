use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::proxy::InterceptedRequest;

const HISTORY_FILE: &str = "history.json";
const MAX_STORED: usize = 1000;

pub struct HistoryStore {
    path: PathBuf,
}

impl HistoryStore {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(home).join(".proxybot");
        std::fs::create_dir_all(&dir).ok();
        Self {
            path: dir.join(HISTORY_FILE),
        }
    }

    pub fn load(&self) -> Vec<InterceptedRequest> {
        let file = match File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let reader = BufReader::new(file);
        let data: serde_json::Value = match serde_json::from_reader(reader) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };
        let requests_array = match data.get("requests").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return Vec::new(),
        };
        requests_array
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect()
    }

    pub fn save(&self, requests: &[InterceptedRequest]) -> Result<(), String> {
        let last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|dur| dur.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());

        let data = serde_json::json!({
            "version": 1,
            "last_updated": last_updated,
            "requests": &requests[..requests.len().min(MAX_STORED)]
        });

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;

        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &data).map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())
    }
}
