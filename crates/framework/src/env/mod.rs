use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment { Local, Development, Staging, Production, Testing, Custom(String) }

impl Environment {
    pub fn detect() -> Self {
        match std::env::var("APP_ENV").as_deref() {
            Ok("local") | Ok("development") => Environment::Local,
            Ok("staging") => Environment::Staging,
            Ok("production") => Environment::Production,
            Ok("testing") => Environment::Testing,
            Ok(other) => Environment::Custom(other.into()),
            Err(_) => Environment::Local,
        }
    }
    pub fn is_local(&self) -> bool { matches!(self, Environment::Local) }
    pub fn is_production(&self) -> bool { matches!(self, Environment::Production) }
    pub fn is_testing(&self) -> bool { matches!(self, Environment::Testing) }
    pub fn as_str(&self) -> &str {
        match self { Environment::Local => "local", Environment::Development => "development",
            Environment::Staging => "staging", Environment::Production => "production",
            Environment::Testing => "testing", Environment::Custom(s) => s, }
    }
}

static LOADED_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();

fn load_env_at(path: &Path) -> Result<(), String> {
    if !path.exists() { return Err(format!(".env not found: {}", path.display())); }
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut vars = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_string();
            let mut value = line[eq + 1..].trim().to_string();
            if (value.starts_with('"') && value.ends_with('"')) || (value.starts_with('\'') && value.ends_with('\'')) {
                value = value[1..value.len()-1].to_string();
            }
            if std::env::var(&key).is_err() {
                unsafe { std::env::set_var(&key, &value); }
                vars.insert(key, value);
            }
        }
    }
    LOADED_ENV.set(vars).map_err(|_| "Env already loaded".to_string())?;
    Ok(())
}

pub fn load_env(path: &Path) -> Result<(), String> { load_env_at(path) }

pub fn load_env_auto() -> Result<(), String> {
    let mut cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    loop {
        let env = cwd.join(".env");
        if env.exists() { return load_env_at(&env); }
        if !cwd.pop() { break; }
    }
    Err(".env not found".to_string())
}

pub fn env(key: &str, default: &str) -> String { std::env::var(key).unwrap_or_else(|_| default.to_string()) }
pub fn env_int(key: &str, default: i64) -> i64 { std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default) }
pub fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key).ok().map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on")).unwrap_or(default)
}
pub fn has_env(key: &str) -> bool { std::env::var(key).is_ok() }
