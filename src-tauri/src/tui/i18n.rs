use std::sync::Mutex;

/// Language enum mirrors the existing GUI i18n Language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    En,
    Zh,
}

impl Language {
    pub fn from_env() -> Self {
        std::env::var("PROXYBOT_LANG")
            .ok()
            .and_then(|v| match v.as_str() {
                "zh" | "ZH" | "zh-CN" | "zh_TW" => Some(Language::Zh),
                _ => Some(Language::En),
            })
            .unwrap_or(Language::En)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Zh => "zh",
        }
    }

    pub fn set_locale(&self) {
        rust_i18n::set_locale(self.as_str());
    }
}

/// Thread-safe language state for TuiApp.
pub struct LocaleState(pub Mutex<Language>);

impl LocaleState {
    pub fn new(lang: Language) -> Self {
        lang.set_locale();
        Self(Mutex::new(lang))
    }

    pub fn get(&self) -> Language {
        *self.0.lock().unwrap()
    }

    pub fn set(&self, lang: Language) {
        lang.set_locale();
        *self.0.lock().unwrap() = lang;
    }

    pub fn toggle(&self) {
        let mut guard = self.0.lock().unwrap();
        let new = match *guard {
            Language::En => Language::Zh,
            Language::Zh => Language::En,
        };
        new.set_locale();
        *guard = new;
    }
}

// Re-export t! macro for convenience
pub use rust_i18n::t;