//! Translation — JSON-based lang files with locale fallback and pluralization.

pub mod json;

pub use json::JsonTranslator;

use std::sync::{OnceLock, Mutex};

pub trait Translator: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn locale(&self) -> &str;
    fn set_locale(&mut self, locale: &str);
    fn trans(&self, key: &str, replacements: &[(&str, &str)]) -> String;
    fn choice(&self, key: &str, count: u64, replacements: &[(&str, &str)]) -> String;
}

// ── Global Facade ──

static GLOBAL_TRANSLATOR: OnceLock<Mutex<Box<dyn Translator>>> = OnceLock::new();

pub fn init(translator: impl Translator + 'static) {
    GLOBAL_TRANSLATOR.set(Mutex::new(Box::new(translator)))
        .unwrap_or_else(|_| panic!("Global translator already initialized"));
}

pub fn trans(key: &str, replacements: &[(&str, &str)]) -> String {
    GLOBAL_TRANSLATOR.get()
        .and_then(|m| m.lock().ok())
        .map(|t| t.trans(key, replacements))
        .unwrap_or_else(|| key.to_string())
}

pub fn choice(key: &str, count: u64, replacements: &[(&str, &str)]) -> String {
    GLOBAL_TRANSLATOR.get()
        .and_then(|m| m.lock().ok())
        .map(|t| t.choice(key, count, replacements))
        .unwrap_or_else(|| format!("{} (count: {})", key, count))
}

pub fn locale() -> String {
    GLOBAL_TRANSLATOR.get()
        .and_then(|m| m.lock().ok())
        .map(|t| t.locale().to_string())
        .unwrap_or_else(|| "en".to_string())
}
