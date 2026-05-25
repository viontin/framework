use std::path::Path;
use std::fs;

pub fn read(path: impl AsRef<Path>) -> Result<String, String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))
}

pub fn read_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>, String> {
    let path = path.as_ref();
    fs::read(path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))
}

pub fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() { super::dir::ensure_dir(parent)?; }
    fs::write(path, content).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

pub fn append(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> Result<(), String> {
    let path = path.as_ref();
    use std::io::Write;
    let mut file = fs::OpenOptions::new().create(true).append(true).open(path)
        .map_err(|e| format!("Cannot open {}: {}", path.display(), e))?;
    file.write_all(content.as_ref()).map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

pub fn delete(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    fs::remove_file(path).map_err(|e| format!("Cannot delete {}: {}", path.display(), e))
}

pub fn exists(path: impl AsRef<Path>) -> bool { path.as_ref().exists() }

pub fn size(path: impl AsRef<Path>) -> Result<u64, String> {
    let path = path.as_ref();
    fs::metadata(path).map(|m| m.len()).map_err(|e| format!("Cannot stat {}: {}", path.display(), e))
}

pub fn last_modified(path: impl AsRef<Path>) -> Result<std::time::SystemTime, String> {
    let path = path.as_ref();
    fs::metadata(path).and_then(|m| m.modified()).map_err(|e| format!("Cannot stat {}: {}", path.display(), e))
}

pub fn extension(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref().extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase())
}

pub fn stem(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref().file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
}


