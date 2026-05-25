use std::collections::HashMap;
use std::path::Path;
use crate::localization::Translator;

#[derive(Debug, Clone)]
pub struct JsonTranslator {
    locale: String,
    fallback: String,
    translations: HashMap<String, HashMap<String, String>>,
}

impl JsonTranslator {
    pub fn new(locale: impl Into<String>) -> Self {
        JsonTranslator {
            locale: locale.into(),
            fallback: String::from("en"),
            translations: HashMap::new(),
        }
    }

    pub fn fallback(mut self, locale: impl Into<String>) -> Self {
        self.fallback = locale.into(); self
    }

    pub fn load_dir(&mut self, dir: &Path) -> Result<(), String> {
        if !dir.is_dir() { return Ok(()); }

        for entry in std::fs::read_dir(dir).map_err(|e| format!("Cannot read lang dir: {}", e))?.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }

            let locale = path.file_name().and_then(|s| s.to_str())
                .ok_or_else(|| "Invalid locale directory name".to_string())?.to_string();

            for file in std::fs::read_dir(&path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?.flatten() {
                let file_path = file.path();
                if file_path.extension().is_none_or(|e| e != "json") { continue; }

                let file_stem = file_path.file_stem().and_then(|s| s.to_str())
                    .ok_or_else(|| "Invalid filename".to_string())?.to_string();
                let content = std::fs::read_to_string(&file_path)
                    .map_err(|e| format!("Cannot read {}: {}", file_path.display(), e))?;
                let parsed: HashMap<String, serde_json::Value> = serde_json::from_str(&content)
                    .map_err(|e| format!("Invalid JSON in {}: {}", file_path.display(), e))?;

                let locale_map = self.translations.entry(locale.clone()).or_default();
                for (key, value) in parsed {
                    let full_key = format!("{}.{}", file_stem, key);
                    locale_map.insert(full_key, match value {
                        serde_json::Value::String(s) => s,
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        other => other.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn resolve(&self, key: &str) -> Option<&String> {
        if let Some(m) = self.translations.get(&self.locale) && let Some(v) = m.get(key) { return Some(v); }
        if self.fallback != self.locale && let Some(m) = self.translations.get(&self.fallback) && let Some(v) = m.get(key) { return Some(v); }
        None
    }

    fn replace_params(text: &str, replacements: &[(&str, &str)]) -> String {
        let mut result = text.to_string();
        for (key, value) in replacements { result = result.replace(&format!(":{}", key), value); }
        result
    }

    fn pluralize(text: &str, count: u64) -> String {
        for part in text.split('|') {
            let part = part.trim();
            if let Some(brace_end) = part.find('}') {
                let rule = &part[1..brace_end];
                let value = &part[brace_end + 1..];
                if rule.contains(',') {
                    let parts: Vec<&str> = rule.split(',').collect();
                    let min = parts[0].trim().parse::<u64>().unwrap_or(0);
                    let max_str = parts.get(1).map(|s| s.trim()).unwrap_or("*");
                    if (max_str == "*" && count >= min) || (count >= min && count <= max_str.parse::<u64>().unwrap_or(u64::MAX)) {
                        return value.to_string();
                    }
                } else if let Ok(num) = rule.parse::<u64>() { if count == num { return value.to_string(); } }
                else if rule == "*" { return value.to_string(); }
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
