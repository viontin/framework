//! Configuration system — JSON-based, environment-aware, dot-notation access.
//!
//! Loading chain:
//!   1. config/default.json  → base values
//!   2. config/{env}.json    → environment-specific overrides
//!   3. config/local.json    → local overrides (gitignored)
//!   4. Environment vars     → APP_* overrides (highest priority)

use std::collections::HashMap;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

/// Config value type — mirror of viontin_core::Value for JSON interop.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Array(Vec<ConfigValue>),
    Table(HashMap<String, ConfigValue>),
}

// ── ConfigRepository ──

#[derive(Debug, Clone)]
pub struct ConfigRepository {
    items: HashMap<String, ConfigValue>,
    frozen: bool,
}

impl ConfigRepository {
    pub fn new() -> Self { ConfigRepository { items: HashMap::new(), frozen: false } }

    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        if let Some(v) = self.items.get(key) { return Some(v); }
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = self.items.get(parts[0])?;
        for part in &parts[1..] {
            match current {
                ConfigValue::Table(t) => { current = t.get(*part)?; }
                _ => return None,
            }
        }
        Some(current)
    }

    pub fn string(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            ConfigValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn int(&self, key: &str) -> Option<i64> {
        match self.get(key)? {
            ConfigValue::I64(i) => Some(*i),
            ConfigValue::String(s) => s.parse().ok(),
            ConfigValue::F64(f) => Some(*f as i64),
            _ => None,
        }
    }

    pub fn float(&self, key: &str) -> Option<f64> {
        match self.get(key)? {
            ConfigValue::F64(f) => Some(*f),
            ConfigValue::I64(i) => Some(*i as f64),
            ConfigValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        match self.get(key)? {
            ConfigValue::Bool(b) => Some(*b),
            ConfigValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn array(&self, key: &str) -> Option<&Vec<ConfigValue>> {
        match self.get(key)? {
            ConfigValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn object(&self, key: &str) -> Option<&HashMap<String, ConfigValue>> {
        match self.get(key)? {
            ConfigValue::Table(t) => Some(t),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: ConfigValue) {
        if self.frozen { return; }
        self.items.insert(key.to_string(), value);
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn all(&self) -> &HashMap<String, ConfigValue> {
        &self.items
    }

    /// Freeze config, preventing further mutations.
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// Is the config frozen?
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Apply environment variable overlay.
    /// Converts APP_DATABASE_HOST to key "database.host" and overrides matching config values.
    pub fn apply_env_overlay(&mut self, prefix: &str) {
        for (key, value) in std::env::vars() {
            if !key.starts_with(prefix) { continue; }
            let config_key = key
                .strip_prefix(prefix)
                .unwrap_or(&key)
                .to_lowercase()
                .replace('_', ".");
            if self.has(&config_key) {
                self.set(&config_key, auto_type(&value));
            }
        }
    }

    /// Load and merge a JSON file.
    pub fn load_json(&mut self, namespace: &str, json: &str) -> Result<(), String> {
        let parsed: HashMap<String, ConfigValue> = serde_json::from_str(json)
            .map_err(|e| format!("Config parse error: {}", e))?;
        let mut with_env = parsed;
        for v in with_env.values_mut() { resolve_env(v); }
        // Merge into namespace
        let ns = self.items.entry(namespace.to_string()).or_insert_with(|| ConfigValue::Table(HashMap::new()));
        if let ConfigValue::Table(existing) = ns {
            for (k, v) in with_env { existing.insert(k, v); }
        }
        Ok(())
    }
}

impl Default for ConfigRepository {
    fn default() -> Self { Self::new() }
}

// ── ConfigLoader ──

pub struct ConfigLoader {
    repository: ConfigRepository,
    env: String,
    config_dir: Option<String>,
}

impl ConfigLoader {
    pub fn new(env: impl Into<String>) -> Self {
        ConfigLoader { repository: ConfigRepository::new(), env: env.into(), config_dir: None }
    }

    pub fn config_dir(mut self, dir: impl Into<String>) -> Self {
        self.config_dir = Some(dir.into());
        self
    }

    pub fn load(&mut self) -> Result<(), String> {
        if let Some(dir) = &self.config_dir {
            let d = Path::new(dir);
            if !d.exists() { return Ok(()); }
            for entry in std::fs::read_dir(d).map_err(|e| e.to_string())?.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) != Some("json") { continue; }
                let name = p.file_stem().and_then(|s| s.to_str()).ok_or("Invalid filename")?.to_string();
                // Load default
                let content = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
                self.repository.load_json(&name, &content)?;
                // Load environment-specific override
                let env_path = d.join(format!("{}.{}.json", name, self.env));
                if env_path.exists() {
                    self.repository.load_json(&name, &std::fs::read_to_string(&env_path).map_err(|e| e.to_string())?)?;
                }
            }
            // Load local overrides (gitignored)
            let local_path = d.join("local.json");
            if local_path.exists() {
                self.repository.load_json("local", &std::fs::read_to_string(&local_path).map_err(|e| e.to_string())?)?;
            }
        }
        Ok(())
    }

    pub fn repository(&self) -> &ConfigRepository { &self.repository }
}

// ── Environment Variable Resolution ──

fn resolve_env(val: &mut ConfigValue) {
    match val {
        ConfigValue::Array(arr) if arr.len() == 2 => {
            if let ConfigValue::String(s) = &arr[0] {
                if let Some(key) = s.strip_prefix("env:") {
                    *val = match std::env::var(key) {
                        Ok(v) => auto_type(&v),
                        Err(_) => arr[1].clone(),
                    };
                    return;
                }
            }
            for item in arr.iter_mut() { resolve_env(item); }
        }
        ConfigValue::Table(t) => { for v in t.values_mut() { resolve_env(v); } }
        ConfigValue::Array(arr) => { for item in arr.iter_mut() { resolve_env(item); } }
        _ => {}
    }
}

fn auto_type(s: &str) -> ConfigValue {
    if let Ok(i) = s.parse::<i64>() { return ConfigValue::I64(i); }
    if let Ok(f) = s.parse::<f64>() { return ConfigValue::F64(f); }
    match s.to_lowercase().as_str() {
        "true" | "yes" | "on" => return ConfigValue::Bool(true),
        "false" | "no" | "off" => return ConfigValue::Bool(false),
        _ => {}
    }
    ConfigValue::String(s.to_string())
}

// ── Config Facade ──

static GLOBAL: OnceLock<RwLock<ConfigRepository>> = OnceLock::new();

fn global() -> &'static RwLock<ConfigRepository> {
    GLOBAL.get_or_init(|| RwLock::new(ConfigRepository::new()))
}

/// Configuration facade with static methods backed by a global singleton.
pub struct Config;

impl Config {
    /// Initialize from a loaded repository.
    pub fn init(repo: ConfigRepository) {
        if let Ok(mut g) = global().write() { *g = repo; }
    }

    /// Get a string value. Returns `default` if key is missing.
    pub fn get(key: &str, default: &str) -> String {
        global().read().ok().and_then(|g| g.get(key).cloned()).and_then(|v| match v {
            ConfigValue::String(s) => Some(s),
            ConfigValue::I64(i) => Some(i.to_string()),
            ConfigValue::F64(f) => Some(f.to_string()),
            ConfigValue::Bool(b) => Some(b.to_string()),
            _ => None,
        }).unwrap_or_else(|| default.to_string())
    }

    /// Get a string value, returning None if missing.
    pub fn get_opt(key: &str) -> Option<String> {
        global().read().ok().and_then(|g| g.string(key).map(|s| s.to_string()))
    }

    /// Get an integer value. Returns `default` if missing or not numeric.
    pub fn get_int(key: &str, default: i64) -> i64 {
        global().read().ok().and_then(|g| g.int(key)).unwrap_or(default)
    }

    /// Get a boolean value. Returns `default` if missing or not boolean.
    pub fn get_bool(key: &str, default: bool) -> bool {
        global().read().ok().and_then(|g| g.bool(key)).unwrap_or(default)
    }

    /// Get a float value, returning None if missing.
    pub fn get_float(key: &str) -> Option<f64> {
        global().read().ok().and_then(|g| g.float(key))
    }

    /// Check if a key exists.
    pub fn has(key: &str) -> bool {
        global().read().ok().map_or(false, |g| g.has(key))
    }

    /// Set a value (no-op if frozen).
    pub fn set(key: &str, value: ConfigValue) {
        if let Ok(mut g) = global().write() { g.set(key, value); }
    }

    /// Freeze the config, preventing further mutations.
    pub fn freeze() {
        if let Ok(mut g) = global().write() { g.freeze(); }
    }

    /// Apply APP_ env var overlay (APP_DATABASE_HOST → database.host).
    pub fn apply_env_overlay() {
        if let Ok(mut g) = global().write() { g.apply_env_overlay("APP_"); }
    }
}
