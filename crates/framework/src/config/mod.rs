use std::collections::HashMap;
use std::path::Path;
use std::sync::{OnceLock, Mutex};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    String(String), Int(i64), Float(f64), Bool(bool),
    Array(Vec<ConfigValue>), Table(HashMap<String, ConfigValue>), Null,
}

pub trait ConfigGet: Sized {
    fn from_raw(val: &str) -> Option<Self>;
}

impl ConfigGet for String { fn from_raw(val: &str) -> Option<Self> { Some(val.to_string()) } }
impl ConfigGet for i64 { fn from_raw(val: &str) -> Option<Self> { val.parse().ok() } }
impl ConfigGet for f64 { fn from_raw(val: &str) -> Option<Self> { val.parse().ok() } }
impl ConfigGet for bool {
    fn from_raw(val: &str) -> Option<Self> {
        match val.to_lowercase().as_str() { "true"|"1"|"yes" => Some(true), "false"|"0"|"no" => Some(false), _ => None }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigRepository { items: HashMap<String, ConfigValue> }

impl ConfigRepository {
    pub fn new() -> Self { ConfigRepository { items: HashMap::new() } }
    pub fn set(&mut self, key: &str, value: ConfigValue) { self.items.insert(key.into(), value); }
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        if let Some(v) = self.items.get(key) { return Some(v); }
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = self.items.get(parts[0])?;
        for part in &parts[1..] { match current { ConfigValue::Table(t) => { current = t.get(*part)?; } _ => return None } }
        Some(current)
    }
    pub fn load_json(&mut self, name: &str, json: &str) -> Result<(), String> {
        let mut parsed: HashMap<String, ConfigValue> = serde_json::from_str(json).map_err(|e| format!("Config: {}", e))?;
        for v in parsed.values_mut() { resolve_env(v); }
        self.items.insert(name.into(), ConfigValue::Table(parsed));
        Ok(())
    }
}
impl Default for ConfigRepository { fn default() -> Self { Self::new() } }

fn resolve_env(val: &mut ConfigValue) {
    match val {
        ConfigValue::Array(arr) if arr.len() == 2 => {
            if let ConfigValue::String(s) = &arr[0]
                && let Some(key) = s.strip_prefix("env:") {
                    *val = match std::env::var(key) { Ok(v) => auto_type(&v), Err(_) => arr[1].clone() };
                    return;
                }
            for item in arr.iter_mut() { resolve_env(item); }
        }
        ConfigValue::Table(t) => { for v in t.values_mut() { resolve_env(v); } }
        ConfigValue::Array(arr) => { for item in arr.iter_mut() { resolve_env(item); } }
        _ => {}
    }
}

fn auto_type(s: &str) -> ConfigValue {
    if let Ok(i) = s.parse::<i64>() { return ConfigValue::Int(i); }
    if let Ok(f) = s.parse::<f64>() { return ConfigValue::Float(f); }
    match s.to_lowercase().as_str() { "true"|"yes"|"on" => return ConfigValue::Bool(true), "false"|"no"|"off" => return ConfigValue::Bool(false), _ => {} }
    ConfigValue::String(s.to_string())
}

pub struct ConfigLoader { repository: ConfigRepository, env: String, config_dir: Option<String> }
impl ConfigLoader {
    pub fn new(env: impl Into<String>) -> Self { ConfigLoader { repository: ConfigRepository::new(), env: env.into(), config_dir: None } }
    pub fn config_dir(mut self, dir: impl Into<String>) -> Self { self.config_dir = Some(dir.into()); self }
    pub fn load(&mut self) -> Result<(), String> {
        if let Some(dir) = &self.config_dir {
            let d = Path::new(dir);
            if d.exists() {
                for e in std::fs::read_dir(d).map_err(|e| e.to_string())?.flatten() {
                    let p = e.path();
                    if p.extension().is_none_or(|e| e != "json") { continue; }
                    let name = p.file_stem().and_then(|s| s.to_str()).ok_or_else(|| "Invalid filename".to_string())?;
                    let content = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
                    self.repository.load_json(name, &content)?;
                    let env_path = d.join(format!("{}.{}.json", name, self.env));
                    if env_path.exists() {
                        self.repository.load_json(name, &std::fs::read_to_string(&env_path).map_err(|e| e.to_string())?)?;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn repository(&self) -> &ConfigRepository { &self.repository }
}

static GLOBAL: OnceLock<Mutex<ConfigRepository>> = OnceLock::new();
fn global() -> &'static Mutex<ConfigRepository> { GLOBAL.get_or_init(|| Mutex::new(ConfigRepository::new())) }

pub fn init(config: ConfigRepository) {
    if let Ok(mut g) = global().lock() { g.items = config.items; }
}

pub fn config<T: ConfigGet>(key: &str, default: T) -> T {
    global().lock().ok()
        .and_then(|g| g.get(key).cloned())
        .and_then(|v| match v { ConfigValue::String(s) => Some(s), ConfigValue::Int(i) => Some(i.to_string()),
            ConfigValue::Float(f) => Some(f.to_string()), ConfigValue::Bool(b) => Some(b.to_string()), _ => None })
        .and_then(|s| T::from_raw(&s))
        .unwrap_or(default)
}

pub fn config_set(key: &str, value: ConfigValue) {
    if let Ok(mut g) = global().lock() { g.set(key, value); }
}
