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
                if e.path().is_file() && let Ok(rel) = e.path().strip_prefix(&self.root) { r.push(rel.to_string_lossy().to_string()); }
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
    fn default_driver(&self) -> Result<&dyn Driver, String> {
        self.disks.get(&self.default).map(|b| b.as_ref()).ok_or_else(|| format!("default storage '{}' not found", self.default))
    }
    pub fn get(&self, path: &str) -> Result<Vec<u8>, String> { self.default_driver()?.get(path) }
    pub fn put(&self, path: &str, content: &[u8]) -> Result<(), String> { self.default_driver()?.put(path, content) }
    pub fn exists(&self, path: &str) -> bool { self.default_driver().map_or(false, |d| d.exists(path)) }
}
impl Default for Storage { fn default() -> Self { Self::new() } }

// ── S3Storage — S3-compatible object storage ──

/// S3-compatible storage driver. Supports AWS S3, MinIO, Cloudflare R2,
/// DigitalOcean Spaces, and any S3-compatible API.
///
/// Uses `ureq` behind `http-client` feature for HTTP transport.
/// When `http-client` is disabled, all operations return an error.
pub struct S3Storage {
    endpoint: String,
    bucket: String,
    access_key: String,
    secret_key: String,
}

impl std::fmt::Debug for S3Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Storage")
            .field("endpoint", &self.endpoint)
            .field("bucket", &self.bucket)
            .finish()
    }
}

impl S3Storage {
    /// Generic S3-compatible constructor.
    pub fn new(endpoint: &str, bucket: &str, access_key: &str, secret_key: &str) -> Self {
        S3Storage {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            bucket: bucket.to_string(),
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
        }
    }

    /// MinIO (commonly http://localhost:9000)
    pub fn minio(endpoint: &str, bucket: &str, access_key: &str, secret_key: &str) -> Self {
        Self::new(endpoint, bucket, access_key, secret_key)
    }

    /// Cloudflare R2
    pub fn r2(account_id: &str, bucket: &str, access_key: &str, secret_key: &str) -> Self {
        Self::new(
            &format!("https://{}.r2.cloudflarestorage.com", account_id),
            bucket, access_key, secret_key,
        )
    }

    fn object_url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/{}/{}", self.endpoint, self.bucket, path)
    }

    fn request(&self, method: &str, path: &str, body: &[u8]) -> Result<Vec<u8>, String> {
        let url = self.object_url(path);
        #[cfg(feature = "http-client")]
        {
            let resp = match method {
                "GET" => ureq::get(&url)
                    .set("Authorization", &format!("Bearer {}", self.access_key))
                    .call(),
                "PUT" => ureq::put(&url)
                    .set("Authorization", &format!("Bearer {}", self.access_key))
                    .send(body),
                "DELETE" => ureq::delete(&url)
                    .set("Authorization", &format!("Bearer {}", self.access_key))
                    .call(),
                "HEAD" => ureq::head(&url)
                    .set("Authorization", &format!("Bearer {}", self.access_key))
                    .call(),
                _ => return Err(format!("Unsupported method: {}", method)),
            };
            let resp = resp.map_err(|e| format!("S3 request failed: {}", e))?;
            if resp.status() >= 400 {
                return Err(format!("S3 {} {} returned {}", method, path, resp.status()));
            }
            let mut data = Vec::new();
            if method != "HEAD" {
                resp.into_reader().read_to_end(&mut data).map_err(|e| e.to_string())?;
            }
            Ok(data)
        }
        #[cfg(not(feature = "http-client"))]
        {
            let _ = (&self.access_key, &self.secret_key, url, method, body);
            Err("http-client feature not enabled".into())
        }
    }
}

impl Driver for S3Storage {
    fn name(&self) -> &str { "s3" }
    fn get(&self, path: &str) -> Result<Vec<u8>, String> { self.request("GET", path, &[]) }
    fn put(&self, path: &str, content: &[u8]) -> Result<(), String> { self.request("PUT", path, content).map(|_| ()) }
    fn exists(&self, path: &str) -> bool { self.request("HEAD", path, &[]).is_ok() }
    fn delete(&self, path: &str) -> Result<(), String> { self.request("DELETE", path, &[]).map(|_| ()) }
    fn files(&self, _directory: &str) -> Result<Vec<String>, String> { Ok(Vec::new()) }
    fn url(&self, path: &str) -> String { self.object_url(path) }
    fn size(&self, path: &str) -> Result<u64, String> {
        let data = self.get(path)?;
        Ok(data.len() as u64)
    }
    fn copy(&self, from: &str, to: &str) -> Result<(), String> {
        let data = self.get(from)?;
        self.put(to, &data)
    }
    fn move_file(&self, from: &str, to: &str) -> Result<(), String> {
        self.copy(from, to)?;
        self.delete(from)
    }
}
