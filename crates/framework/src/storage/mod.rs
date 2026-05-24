use std::fmt;

pub trait Driver: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn get(&self, path: &str) -> Result<Vec<u8>, String>;
    fn put(&self, path: &str, content: &[u8]) -> Result<(), String>;
    fn exists(&self, path: &str) -> bool;
    fn delete(&self, path: &str) -> Result<(), String>;
    fn files(&self, directory: &str) -> Result<Vec<String>, String>;
    fn url(&self, path: &str) -> String;
    fn size(&self, path: &str) -> Result<u64, String>;
    fn copy(&self, from: &str, to: &str) -> Result<(), String>;
    fn move_file(&self, from: &str, to: &str) -> Result<(), String>;
}

use std::path::PathBuf;
use std::sync::Mutex;

pub struct LocalStorage { root: PathBuf, base_url: String, }
impl std::fmt::Debug for LocalStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalStorage").field("root", &self.root).finish()
    }
}
impl LocalStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self { LocalStorage { root: root.into(), base_url: "/storage".into() } }
    fn full_path(&self, path: &str) -> PathBuf { self.root.join(path.trim_start_matches('/')) }
}

impl Driver for LocalStorage {
    fn name(&self) -> &str { "local" }
    fn get(&self, path: &str) -> Result<Vec<u8>, String> { std::fs::read(self.full_path(path)).map_err(|e| e.to_string()) }
    fn put(&self, path: &str, content: &[u8]) -> Result<(), String> {
        let p = self.full_path(path);
        if let Some(parent) = p.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
        std::fs::write(&p, content).map_err(|e| e.to_string())
    }
    fn exists(&self, path: &str) -> bool { self.full_path(path).exists() }
    fn delete(&self, path: &str) -> Result<(), String> { let p = self.full_path(path); if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) }.map_err(|e| e.to_string()) }
    fn files(&self, directory: &str) -> Result<Vec<String>, String> {
        let dir = self.full_path(directory); let mut r = Vec::new();
        if dir.is_dir() {
            for e in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
                if e.path().is_file() { if let Ok(rel) = e.path().strip_prefix(&self.root) { r.push(rel.to_string_lossy().to_string()); } }
            }
        }
        Ok(r)
    }
    fn url(&self, path: &str) -> String { format!("{}/{}", self.base_url, path.trim_start_matches('/')) }
    fn size(&self, path: &str) -> Result<u64, String> { std::fs::metadata(self.full_path(path)).map(|m| m.len()).map_err(|e| e.to_string()) }
    fn copy(&self, from: &str, to: &str) -> Result<(), String> {
        let fp = self.full_path(from); let tp = self.full_path(to);
        if let Some(parent) = tp.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
        std::fs::copy(&fp, &tp).map_err(|e| e.to_string())?; Ok(())
    }
    fn move_file(&self, from: &str, to: &str) -> Result<(), String> {
        std::fs::rename(self.full_path(from), self.full_path(to)).map_err(|e| e.to_string())
    }
}

// ── MemoryStorage ──

pub struct MemoryStorage { files: Mutex<std::collections::HashMap<String, Vec<u8>>> }
impl std::fmt::Debug for MemoryStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("MemoryStorage").finish() }
}
impl MemoryStorage { pub fn new() -> Self { MemoryStorage { files: Mutex::new(std::collections::HashMap::new()) } } }
impl Default for MemoryStorage { fn default() -> Self { Self::new() } }
impl Driver for MemoryStorage {
    fn name(&self) -> &str { "memory" }
    fn get(&self, path: &str) -> Result<Vec<u8>, String> { self.files.lock().map_err(|e| e.to_string())?.get(path).cloned().ok_or_else(|| "not found".into()) }
    fn put(&self, path: &str, content: &[u8]) -> Result<(), String> { self.files.lock().map_err(|e| e.to_string())?.insert(path.into(), content.to_vec()); Ok(()) }
    fn exists(&self, path: &str) -> bool { self.files.lock().map(|m| m.contains_key(path)).unwrap_or(false) }
    fn delete(&self, path: &str) -> Result<(), String> { self.files.lock().map_err(|e| e.to_string())?.remove(path); Ok(()) }
    fn files(&self, _directory: &str) -> Result<Vec<String>, String> { Ok(self.files.lock().map_err(|e| e.to_string())?.keys().cloned().collect()) }
    fn url(&self, path: &str) -> String { format!("/memory/{}", path) }
    fn size(&self, path: &str) -> Result<u64, String> { self.files.lock().map_err(|e| e.to_string())?.get(path).map(|c| c.len() as u64).ok_or_else(|| "not found".into()) }
    fn copy(&self, from: &str, _to: &str) -> Result<(), String> { if self.files.lock().map_err(|e| e.to_string())?.contains_key(from) { Ok(()) } else { Err("not found".into()) } }
    fn move_file(&self, from: &str, _to: &str) -> Result<(), String> { if self.files.lock().map_err(|e| e.to_string())?.remove(from).is_some() { Ok(()) } else { Err("not found".into()) } }
}

// ── Storage Facade ──

#[derive(Debug)]
pub struct Storage { disks: std::collections::HashMap<String, Box<dyn Driver>>, default: String, }
impl Storage {
    pub fn new() -> Self {
        let mut s = Storage { disks: std::collections::HashMap::new(), default: "local".into() };
        s.add("local", LocalStorage::new("./storage")); s
    }
    pub fn add(&mut self, name: impl Into<String>, d: impl Driver + 'static) { self.disks.insert(name.into(), Box::new(d)); }
    pub fn disk(&self, name: &str) -> Option<&dyn Driver> { self.disks.get(name).map(|b| b.as_ref()) }
    fn default_driver(&self) -> &dyn Driver { self.disks.get(&self.default).expect("default storage not found").as_ref() }
    pub fn get(&self, path: &str) -> Result<Vec<u8>, String> { self.default_driver().get(path) }
    pub fn put(&self, path: &str, content: &[u8]) -> Result<(), String> { self.default_driver().put(path, content) }
    pub fn exists(&self, path: &str) -> bool { self.default_driver().exists(path) }
}
impl Default for Storage { fn default() -> Self { Self::new() } }
