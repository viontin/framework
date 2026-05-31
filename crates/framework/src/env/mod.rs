use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment { Local, Development, Staging, Production, Testing, Custom(String) }

impl Environment {
    pub fn detect() -> Self {
        Self::detect_from(&std::env::args().collect::<Vec<_>>())
    }

    pub fn detect_from(args: &[String]) -> Self {
        // 1. Check --env CLI argument
        for (i, arg) in args.iter().enumerate() {
            if arg == "--env" && i + 1 < args.len() {
                return Self::from_str(&args[i + 1]);
            }
            if let Some(val) = arg.strip_prefix("--env=") {
                return Self::from_str(val);
            }
        }
        // 2. Check APP_ENV environment variable
        if let Ok(val) = std::env::var("APP_ENV") {
            return Self::from_str(&val);
        }
        // 3. Compile-time default
        if cfg!(debug_assertions) {
            Environment::Local
        } else {
            Environment::Production
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "local" | "development" => Environment::Local,
            "staging" => Environment::Staging,
            "production" => Environment::Production,
            "testing" => Environment::Testing,
            other => Environment::Custom(other.to_string()),
        }
    }

    pub fn is_local(&self) -> bool { matches!(self, Environment::Local | Environment::Development) }
    pub fn is_production(&self) -> bool { matches!(self, Environment::Production) }
    pub fn is_prod(&self) -> bool { self.is_production() }
    pub fn is_testing(&self) -> bool { matches!(self, Environment::Testing) }
    pub fn is_dev(&self) -> bool { matches!(self, Environment::Local | Environment::Development) }

    pub fn as_str(&self) -> &str {
        match self {
            Environment::Local => "local", Environment::Development => "development",
            Environment::Staging => "staging", Environment::Production => "production",
            Environment::Testing => "testing", Environment::Custom(s) => s,
        }
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
                // SAFETY: set_var is used to propagate .env values to process environment.
                // This is the standard pattern used by dotenvy/dotenv crate. We only set vars
                // that are not already present, preventing override of existing env vars.
                unsafe { std::env::set_var(&key, &value); }
                vars.insert(key, value);
            }
        }
    }
    LOADED_ENV.set(vars).map_err(|_| "Env already loaded".to_string())?;
    Ok(())
}

fn load_env_val(key: &str) -> Option<String> {
    if let Ok(val) = std::env::var(key) {
        return Some(val);
    }
    LOADED_ENV.get().and_then(|vars| vars.get(key).cloned())
}

// ── Env Facade ──

/// Environment variable facade backed by process env + .env file cache.
pub struct Env;

impl Env {
    /// Load `.env` from a specific path.
    pub fn load(path: &Path) -> Result<(), String> { load_env_at(path) }

    /// Auto-discover and load `.env` by walking up directories.
    pub fn load_auto() -> Result<(), String> {
        let mut cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        loop {
            let env = cwd.join(".env");
            if env.exists() { return load_env_at(&env); }
            if !cwd.pop() { break; }
        }
        Err(".env not found".to_string())
    }

    /// Get a value. Returns `default` if key is missing.
    pub fn get(key: &str, default: &str) -> String {
        load_env_val(key).unwrap_or_else(|| default.to_string())
    }

    /// Get a value, returning None if missing.
    pub fn get_opt(key: &str) -> Option<String> { load_env_val(key) }

    /// Get an integer value. Returns `default` if missing or not parseable.
    pub fn get_int(key: &str, default: i64) -> i64 {
        load_env_val(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    /// Get a boolean value. Returns `default` if missing.
    pub fn get_bool(key: &str, default: bool) -> bool {
        load_env_val(key).map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on")).unwrap_or(default)
    }

    /// Check if a key exists.
    pub fn has(key: &str) -> bool { load_env_val(key).is_some() }
}

