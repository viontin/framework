use std::path::{Path, PathBuf};
use std::fs;

pub fn ensure_dir(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path).map_err(|e| format!("Cannot create {}: {}", path.display(), e))?;
    }
    Ok(())
}

pub fn create_dir(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    fs::create_dir(path).map_err(|e| format!("Cannot create {}: {}", path.display(), e))
}

pub fn remove_dir(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    fs::remove_dir(path).map_err(|e| format!("Cannot remove {}: {}", path.display(), e))
}

pub fn remove_all(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    fs::remove_dir_all(path).map_err(|e| format!("Cannot remove {}: {}", path.display(), e))
}

pub fn list(path: impl AsRef<Path>) -> Result<Vec<PathBuf>, String> {
    let path = path.as_ref();
    let mut entries: Vec<PathBuf> = fs::read_dir(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?
        .filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    Ok(entries)
}

pub fn list_files(path: impl AsRef<Path>, ext: &str) -> Result<Vec<PathBuf>, String> {
    Ok(list(path)?.into_iter().filter(|p| p.is_file() && p.extension().is_some_and(|e| e == ext)).collect())
}

pub fn copy(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<u64, String> {
    let src = src.as_ref(); let dst = dst.as_ref();
    if let Some(parent) = dst.parent() { ensure_dir(parent)?; }
    fs::copy(src, dst).map_err(|e| format!("Cannot copy {} to {}: {}", src.display(), dst.display(), e))
}

pub fn copy_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), String> {
    let src = src.as_ref(); let dst = dst.as_ref();
    if !src.is_dir() { return copy(src, dst).map(|_| ()); }
    ensure_dir(dst)?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let target = dst.join(entry.file_name());
        if entry.path().is_dir() { copy_all(&entry.path(), &target)?; }
        else { copy(&entry.path(), &target)?; }
    }
    Ok(())
}

pub fn find_files(dir: impl AsRef<Path>, ext: &str) -> Result<Vec<PathBuf>, String> {
    let mut results = Vec::new();
    let dir = dir.as_ref();
    if !dir.is_dir() { return Ok(results); }
    for entry in walk(dir)? {
        if entry.is_file() && entry.extension().is_some_and(|e| e == ext) {
            results.push(entry);
        }
    }
    Ok(results)
}

fn walk(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() { files.extend(walk(&path)?); }
            else { files.push(path); }
        }
    }
    Ok(files)
}
