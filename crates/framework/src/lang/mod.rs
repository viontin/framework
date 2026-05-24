//! Translation implementation — JSON-based lang files.
//!
//! Lang files are stored as JSON in a `lang/` directory:
//!
//! ```json
//! lang/
//! ├── en/
//! │   ├── messages.json  → { "welcome": "Welcome :name", "apples": "{0} None|[1,*] :count apples" }
//! │   └── auth.json      → { "login": "Login", "logout": "Logout" }
//! └── id/
//!     └── messages.json  → { "welcome": "Selamat datang :name" }
//! ```

use std::collections::HashMap;
use std::path::Path;

pub trait Translator: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn locale(&self) -> &str;
    fn set_locale(&mut self, locale: &str);
    fn trans(&self, key: &str, replacements: &[(&str, &str)]) -> String;
    fn choice(&self, key: &str, count: u64, replacements: &[(&str, &str)]) -> String;
}

/// JSON-based translator — loads translation files at construction time.
///
/// Ownership-friendly: all strings are owned (`String`), no lifetimes needed.
#[derive(Debug, Clone)]
pub struct JsonTranslator {
    locale: String,
    fallback: String,
    translations: HashMap<String, HashMap<String, String>>,
}

impl JsonTranslator {
    /// Create an empty translator with a default locale.
    pub fn new(locale: impl Into<String>) -> Self {
        JsonTranslator {
            locale: locale.into(),
            fallback: String::from("en"),
            translations: HashMap::new(),
        }
    }

    /// Set the fallback locale (used when a key is not found in the current locale).
    pub fn fallback(mut self, locale: impl Into<String>) -> Self {
        self.fallback = locale.into();
        self
    }

    /// Load all `.json` files from a directory structure.
    ///
    /// Expected structure:
    ///   lang/<locale>/<file>.json
    ///
    /// Keys are stored as `"file.key"` (dot notation).
    pub fn load_dir(&mut self, dir: &Path) -> Result<(), String> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Cannot read lang dir: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let locale = path.file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| "Invalid locale directory name".to_string())?
                .to_string();

            let files = std::fs::read_dir(&path)
                .map_err(|e| format!("Cannot read locale dir {}: {}", path.display(), e))?;

            for file in files.flatten() {
                let file_path = file.path();
                if file_path.extension().is_none_or(|e| e != "json") {
                    continue;
                }

                let file_stem = file_path.file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| "Invalid filename".to_string())?
                    .to_string();

                let content = std::fs::read_to_string(&file_path)
                    .map_err(|e| format!("Cannot read {}: {}", file_path.display(), e))?;

                let parsed: HashMap<String, serde_json::Value> = serde_json::from_str(&content)
                    .map_err(|e| format!("Invalid JSON in {}: {}", file_path.display(), e))?;

                let locale_map = self.translations.entry(locale.clone())
                    .or_default();

                for (key, value) in parsed {
                    let full_key = format!("{}.{}", file_stem, key);
                    let str_value = match value {
                        serde_json::Value::String(s) => s,
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        other => other.to_string(),
                    };
                    locale_map.insert(full_key, str_value);
                }
            }
        }

        Ok(())
    }

    fn resolve(&self, key: &str) -> Option<&String> {
        // Try current locale first
        if let Some(locale_map) = self.translations.get(&self.locale)
            && let Some(value) = locale_map.get(key) {
                return Some(value);
            }
        // Try fallback locale
        if self.fallback != self.locale
            && let Some(locale_map) = self.translations.get(&self.fallback)
                && let Some(value) = locale_map.get(key) {
                    return Some(value);
                }
        None
    }

    fn replace_params(text: &str, replacements: &[(&str, &str)]) -> String {
        let mut result = text.to_string();
        for (key, value) in replacements {
            result = result.replace(&format!(":{}", key), value);
        }
        result
    }

    fn pluralize(text: &str, count: u64) -> String {
        // Format: "{0} None|{1} One|[2,*] Many"
        for part in text.split('|') {
            let part = part.trim();
            if let Some(brace_end) = part.find('}') {
                let rule = &part[1..brace_end]; // e.g. "0", "1", "2,*", "*"
                let value = &part[brace_end + 1..];

                if rule.contains(',') {
                    // Range: "[2,*]" means 2 or more
                    let range_parts: Vec<&str> = rule.split(',').collect();
                    let min = range_parts[0].trim().parse::<u64>().unwrap_or(0);
                    let max_str = range_parts.get(1).map(|s| s.trim()).unwrap_or("*");
                    let matches = if max_str == "*" {
                        count >= min
                    } else {
                        let max = max_str.parse::<u64>().unwrap_or(u64::MAX);
                        count >= min && count <= max
                    };
                    if matches { return value.to_string(); }
                } else if let Ok(num) = rule.parse::<u64>() {
                    if count == num {
                        return value.to_string();
                    }
                } else if rule == "*" {
                    return value.to_string();
                }
            }
        }
        text.to_string()
    }
}

impl Translator for JsonTranslator {
    fn name(&self) -> &str { "json" }
    fn locale(&self) -> &str { &self.locale }
    fn set_locale(&mut self, locale: &str) { self.locale = locale.to_string(); }

    fn trans(&self, key: &str, replacements: &[(&str, &str)]) -> String {
        match self.resolve(key) {
            Some(template) => Self::replace_params(template, replacements),
            None => key.to_string(),
        }
    }

    fn choice(&self, key: &str, count: u64, replacements: &[(&str, &str)]) -> String {
        let template = self.resolve(key).map(|s| s.as_str()).unwrap_or(key);
        let pluralized = Self::pluralize(template, count);
        let with_count = pluralized.replace(":count", &count.to_string());
        Self::replace_params(&with_count, replacements)
    }
}

// ── Global Lang Facade ──

use std::sync::{OnceLock, Mutex};

static GLOBAL_TRANSLATOR: OnceLock<Mutex<Box<dyn Translator>>> = OnceLock::new();

/// Initialize the global translator. Call once at startup.
pub fn init(translator: impl Translator + 'static) {
    GLOBAL_TRANSLATOR.set(Mutex::new(Box::new(translator)))
        .unwrap_or_else(|_| panic!("Global translator already initialized"));
}

/// Translate a key — global shortcut.
///
/// ```rust
/// trans("messages.welcome", &[("name", "John")]);
/// // Returns "Welcome John" if translation exists, or key if not found.
/// ```
pub fn trans(key: &str, replacements: &[(&str, &str)]) -> String {
    GLOBAL_TRANSLATOR.get()
        .and_then(|m| m.lock().ok())
        .map(|t| t.trans(key, replacements))
        .unwrap_or_else(|| key.to_string())
}

/// Translate with pluralization — global shortcut.
///
/// ```rust
/// choice("messages.apples", 5, &[]);
/// // Uses pluralization rules from the translation file.
/// ```
pub fn choice(key: &str, count: u64, replacements: &[(&str, &str)]) -> String {
    GLOBAL_TRANSLATOR.get()
        .and_then(|m| m.lock().ok())
        .map(|t| t.choice(key, count, replacements))
        .unwrap_or_else(|| format!("{} (count: {})", key, count))
}

/// Get the current locale.
pub fn locale() -> String {
    GLOBAL_TRANSLATOR.get()
        .and_then(|m| m.lock().ok())
        .map(|t| t.locale().to_string())
        .unwrap_or_else(|| "en".to_string())
}
