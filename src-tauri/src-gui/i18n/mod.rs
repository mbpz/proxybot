pub mod en;
pub mod zh;

use serde::{Deserialize, Serialize};
use std::cell::RefCell;

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

thread_local! {
    static CURRENT_LANG: RefCell<Language> = RefCell::new(Language::En);
}

pub fn set_lang(lang: Language) {
    CURRENT_LANG.with(|cell| {
        *cell.borrow_mut() = lang;
    });
}

pub fn get_lang() -> Language {
    CURRENT_LANG.with(|cell| *cell.borrow())
}

pub fn t(key: &str) -> String {
    let lang = get_lang();
    match lang {
        Language::En => en::translations().get(key).unwrap_or(&key).to_string(),
        Language::Zh => zh::translations().get(key).unwrap_or(&key).to_string(),
    }
}
