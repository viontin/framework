use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait SessionDriver: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn read(&self, id: &str) -> HashMap<String, String>;
    fn write(&self, id: &str, data: &HashMap<String, String>);
    fn destroy(&self, id: &str);
    fn gc(&self, max_lifetime: u64);
}

pub struct FileSession { dir: PathBuf, }
impl std::fmt::Debug for FileSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("FileSession").field("dir", &self.dir).finish() }
}
impl FileSession {
    pub fn new(dir: impl Into<PathBuf>) -> Self { let dir = dir.into(); std::fs::create_dir_all(&dir).ok(); FileSession { dir } }
    fn pf(&self, id: &str) -> PathBuf { self.dir.join(format!("session_{}", id)) }
}
impl SessionDriver for FileSession {
    fn name(&self) -> &str { "file" }
    fn read(&self, id: &str) -> HashMap<String, String> {
        let p = self.pf(id); if !p.exists() { return HashMap::new(); }
        if let Ok(c) = std::fs::read_to_string(&p) {
            let mut lines = c.lines(); let mut data = HashMap::new();
            if let Some(exp_line) = lines.next()
                && let Ok(exp) = exp_line.trim().parse::<u64>() && SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() > exp { std::fs::remove_file(&p).ok(); return HashMap::new(); }
            for line in lines { if let Some(eq) = line.find('=') { data.insert(line[..eq].to_string(), line[eq+1..].to_string()); } }
            return data;
        }
        HashMap::new()
    }
    fn write(&self, id: &str, data: &HashMap<String, String>) {
        let p = self.pf(id); std::fs::create_dir_all(&self.dir).ok();
        let exp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() + 86400*2;
        let mut c = format!("{}\n", exp);
        for (k, v) in data { if !k.starts_with('_') { c.push_str(&format!("{}={}\n", k, v)); } }
        std::fs::write(&p, c).ok();
    }
    fn destroy(&self, id: &str) { let _ = std::fs::remove_file(self.pf(id)); }
    fn gc(&self, max_lifetime: u64) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        if let Ok(e) = std::fs::read_dir(&self.dir) {
            for f in e.flatten() { if let Ok(m) = f.metadata() && let Ok(t) = m.modified()
                && let Ok(d) = t.duration_since(UNIX_EPOCH) && now > d.as_secs() + max_lifetime { let _ = std::fs::remove_file(f.path()); }}
        }
    }
}

// ── MemorySession ──

struct SessionData { data: HashMap<String, String> }
pub struct MemorySession { sessions: Mutex<HashMap<String, SessionData>> }
impl std::fmt::Debug for MemorySession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("MemorySession").finish() }
}
impl MemorySession { pub fn new() -> Self { MemorySession { sessions: Mutex::new(HashMap::new()) } } }
impl Default for MemorySession { fn default() -> Self { Self::new() } }
impl SessionDriver for MemorySession {
    fn name(&self) -> &str { "memory" }
    fn read(&self, id: &str) -> HashMap<String, String> { self.sessions.lock().ok().and_then(|s| s.get(id).map(|d| d.data.clone())).unwrap_or_default() }
    fn write(&self, id: &str, data: &HashMap<String, String>) { if let Ok(mut s) = self.sessions.lock() { s.insert(id.into(), SessionData { data: data.clone() }); } }
    fn destroy(&self, id: &str) { if let Ok(mut s) = self.sessions.lock() { s.remove(id); } }
    fn gc(&self, _: u64) {}
}

// ── Session Facade ──

pub struct Session { driver: Box<dyn SessionDriver>, id: String, data: HashMap<String, String>, flashed: Vec<String>, }
impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").field("id", &self.id).field("data_len", &self.data.len()).finish()
    }
}
impl Session {
    pub fn with_driver(d: impl SessionDriver + 'static) -> Self {
        let id = format!("sess_{:x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos());
        let mut s = Session { driver: Box::new(d), id: id.clone(), data: HashMap::new(), flashed: Vec::new() };
        s.data = s.driver.read(&id); s
    }
    pub fn get(&mut self, key: &str) -> Option<String> { if key.starts_with('_') && self.data.contains_key(key) { self.flashed.push(key.into()); } self.data.get(key).cloned() }
    pub fn peek(&self, key: &str) -> Option<&str> { self.data.get(key).map(|s| s.as_str()) }
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) { self.data.insert(key.into(), value.into()); }
    pub fn has(&self, key: &str) -> bool { self.data.contains_key(key) }
    pub fn remove(&mut self, key: &str) -> Option<String> { self.data.remove(key) }
    pub fn flash(&mut self, key: impl Into<String>, value: impl Into<String>) { self.data.insert(format!("_{}", key.into()), value.into()); }
    pub fn save(&self) { let mut data = self.data.clone(); for k in &self.flashed { data.remove(k); } self.driver.write(&self.id, &data); }
    pub fn destroy(&mut self) { self.driver.destroy(&self.id); self.data.clear(); }
}
impl Drop for Session { fn drop(&mut self) { self.save(); } }
