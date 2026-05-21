use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait CacheDriver: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str, ttl_seconds: Option<u64>);
    fn delete(&self, key: &str);
    fn clear(&self);
    fn has(&self, key: &str) -> bool { self.get(key).is_some() }
    fn increment(&self, key: &str, amount: i64) -> i64;
    fn decrement(&self, key: &str, amount: i64) -> i64 { self.increment(key, -amount) }
}

// ── FileCache ──

pub struct FileCache { dir: PathBuf, }
impl std::fmt::Debug for FileCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("FileCache").field("dir", &self.dir).finish() }
}
impl FileCache {
    pub fn new(dir: impl Into<PathBuf>) -> Self { let dir = dir.into(); std::fs::create_dir_all(&dir).ok(); FileCache { dir } }
    fn path_for(&self, key: &str) -> PathBuf { self.dir.join(format!("{}.cache", key.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_"))) }
    fn is_expired(path: &PathBuf) -> bool {
        if let Ok(c) = std::fs::read_to_string(path) {
            if let Some(exp) = c.lines().next().and_then(|l| l.trim().parse::<u64>().ok()) {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                return now > exp;
            }
        }
        false
    }
}
impl CacheDriver for FileCache {
    fn name(&self) -> &str { "file" }
    fn get(&self, key: &str) -> Option<String> {
        let p = self.path_for(key); if !p.exists() || Self::is_expired(&p) { std::fs::remove_file(&p).ok(); return None; }
        std::fs::read_to_string(&p).ok().and_then(|c| c.lines().nth(1).map(|s| s.to_string()))
    }
    fn set(&self, key: &str, value: &str, ttl: Option<u64>) {
        let p = self.path_for(key); std::fs::create_dir_all(&self.dir).ok();
        let exp = ttl.map(|t| SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() + t).unwrap_or(0);
        std::fs::write(&p, format!("{}\n{}", exp, value)).ok();
    }
    fn delete(&self, key: &str) { let _ = std::fs::remove_file(self.path_for(key)); }
    fn clear(&self) { if let Ok(e) = std::fs::read_dir(&self.dir) { for f in e.flatten() { if f.path().extension().map_or(false, |e| e == "cache") { let _ = std::fs::remove_file(f.path()); } } } }
    fn has(&self, key: &str) -> bool { let p = self.path_for(key); p.exists() && !Self::is_expired(&p) }
    fn increment(&self, key: &str, amount: i64) -> i64 {
        let val = self.get(key).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0) + amount;
        self.set(key, &val.to_string(), None); val
    }
    fn decrement(&self, key: &str, amount: i64) -> i64 { self.increment(key, -amount) }
}

// ── MemoryCache ──

struct CacheEntry { value: String, expires_at: u64 }
pub struct MemoryCache { data: Mutex<HashMap<String, CacheEntry>> }
impl std::fmt::Debug for MemoryCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("MemoryCache").finish() }
}
impl MemoryCache { pub fn new() -> Self { MemoryCache { data: Mutex::new(HashMap::new()) } } }
impl Default for MemoryCache { fn default() -> Self { Self::new() } }
impl CacheDriver for MemoryCache {
    fn name(&self) -> &str { "memory" }
    fn get(&self, key: &str) -> Option<String> {
        let d = self.data.lock().ok()?; let e = d.get(key)?;
        if e.expires_at > 0 && SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() > e.expires_at { None } else { Some(e.value.clone()) }
    }
    fn set(&self, key: &str, value: &str, ttl: Option<u64>) {
        if let Ok(mut d) = self.data.lock() { d.insert(key.into(), CacheEntry { value: value.into(), expires_at: ttl.map(|t| SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() + t).unwrap_or(0) }); }
    }
    fn delete(&self, key: &str) { if let Ok(mut d) = self.data.lock() { d.remove(key); } }
    fn clear(&self) { if let Ok(mut d) = self.data.lock() { d.clear(); } }
    fn has(&self, key: &str) -> bool { self.get(key).is_some() }
    fn increment(&self, key: &str, amount: i64) -> i64 {
        let val = self.get(key).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0) + amount;
        self.set(key, &val.to_string(), None); val
    }
}

// ── NullCache ──

pub struct NullCache;
impl std::fmt::Debug for NullCache { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "NullCache") } }
impl CacheDriver for NullCache {
    fn name(&self) -> &str { "null" }
    fn get(&self, _: &str) -> Option<String> { None } fn set(&self, _: &str, _: &str, _: Option<u64>) {}
    fn delete(&self, _: &str) {} fn clear(&self) {} fn has(&self, _: &str) -> bool { false }
    fn increment(&self, _: &str, amount: i64) -> i64 { amount } fn decrement(&self, _: &str, amount: i64) -> i64 { amount }
}

// ── Cache Facade ──

pub struct Cache { driver: Box<dyn CacheDriver>, prefix: String, }
impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("Cache").field("driver", &self.driver.name()).finish() }
}
impl Cache {
    pub fn with_driver(d: impl CacheDriver + 'static) -> Self { Cache { driver: Box::new(d), prefix: String::new() } }
    pub fn with_prefix(mut self, p: impl Into<String>) -> Self { self.prefix = p.into(); self }
    fn p(&self, k: &str) -> String { if self.prefix.is_empty() { k.to_string() } else { format!("{}:{}", self.prefix, k) } }
    pub fn get(&self, key: &str) -> Option<String> { self.driver.get(&self.p(key)) }
    pub fn set(&self, key: &str, value: &str, ttl: Option<u64>) { self.driver.set(&self.p(key), value, ttl); }
    pub fn delete(&self, key: &str) { self.driver.delete(&self.p(key)); }
    pub fn has(&self, key: &str) -> bool { self.get(key).is_some() }
    pub fn clear(&self) { self.driver.clear(); }
    pub fn remember(&self, key: &str, ttl: u64, f: impl FnOnce() -> String) -> String {
        let k = self.p(key);
        if let Some(v) = self.driver.get(&k) { return v; }
        let v = f(); self.driver.set(&k, &v, Some(ttl)); v
    }
    pub fn pull(&self, key: &str) -> Option<String> { let v = self.get(key); if v.is_some() { self.delete(key); } v }
    pub fn increment(&self, key: &str, amount: i64) -> i64 { self.driver.increment(&self.p(key), amount) }
}
