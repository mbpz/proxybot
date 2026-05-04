pub mod en;
pub mod zh;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    En,
    Zh,
}

impl Language {
    pub fn label(&self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Zh => "中文",
        }
    }
}

pub static mut CURRENT_LANG: Language = Language::En;

pub fn t(key: &str) -> String {
    let lang = unsafe { CURRENT_LANG };
    match lang {
        Language::En => en::translations().get(key).unwrap_or(&key).to_string(),
        Language::Zh => zh::translations().get(key).unwrap_or(&key).to_string(),
    }
}