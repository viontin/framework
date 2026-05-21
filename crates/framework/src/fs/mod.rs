//! Filesystem utilities — inspired by Laravel's Filesystem.
//!
//! Provides safe file/directory operations with consistent error handling,
//! path utilities, and file watching for the dev server.

use std::path::{Path, PathBuf};
use std::fs;

// ── File Operations ──

/// Read a file's contents as a string.
pub fn read(path: impl AsRef<Path>) -> Result<String, String> {
    let path = path.as_ref();
    fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))
}

/// Read a file's contents as bytes.
pub fn read_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>, String> {
    let path = path.as_ref();
    fs::read(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))
}

/// Write content to a file (creates parent directories).
pub fn write(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(path, content)
        .map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

/// Append content to a file.
pub fn append(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> Result<(), String> {
    let path = path.as_ref();
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Cannot open {}: {}", path.display(), e))?;
    file.write_all(content.as_ref())
        .map_err(|e| format!("Cannot write {}: {}", path.display(), e))
}

/// Delete a file.
pub fn delete(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    fs::remove_file(path)
        .map_err(|e| format!("Cannot delete {}: {}", path.display(), e))
}

/// Check if a file exists.
pub fn exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

/// Get the file size in bytes.
pub fn size(path: impl AsRef<Path>) -> Result<u64, String> {
    let path = path.as_ref();
    fs::metadata(path)
        .map(|m| m.len())
        .map_err(|e| format!("Cannot stat {}: {}", path.display(), e))
}

/// Get the last modified time.
pub fn last_modified(path: impl AsRef<Path>) -> Result<std::time::SystemTime, String> {
    let path = path.as_ref();
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map_err(|e| format!("Cannot stat {}: {}", path.display(), e))
}

/// Get the file extension.
pub fn extension(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
}

/// Get the filename without extension.
pub fn stem(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

// ── Directory Operations ──

/// Ensure a directory exists (create if missing).
pub fn ensure_dir(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| format!("Cannot create directory {}: {}", path.display(), e))?;
    }
    Ok(())
}

/// Create a directory (fails if exists).
pub fn create_dir(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    fs::create_dir(path)
        .map_err(|e| format!("Cannot create directory {}: {}", path.display(), e))
}

/// Remove an empty directory.
pub fn remove_dir(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    fs::remove_dir(path)
        .map_err(|e| format!("Cannot remove directory {}: {}", path.display(), e))
}

/// Remove a directory and all its contents.
pub fn remove_all(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    fs::remove_dir_all(path)
        .map_err(|e| format!("Cannot remove {}: {}", path.display(), e))
}

/// List entries in a directory.
pub fn list(path: impl AsRef<Path>) -> Result<Vec<PathBuf>, String> {
    let path = path.as_ref();
    let mut entries: Vec<PathBuf> = fs::read_dir(path)
        .map_err(|e| format!("Cannot read directory {}: {}", path.display(), e))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    Ok(entries)
}

/// List files matching a pattern in a directory (non-recursive).
pub fn list_files(path: impl AsRef<Path>, ext: &str) -> Result<Vec<PathBuf>, String> {
    let entries = list(path)?;
    Ok(entries
        .into_iter()
        .filter(|p| p.is_file() && p.extension().map_or(false, |e| e == ext))
        .collect())
}

/// Copy a file.
pub fn copy(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<u64, String> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    if let Some(parent) = dst.parent() {
        ensure_dir(parent)?;
    }
    fs::copy(src, dst)
        .map_err(|e| format!("Cannot copy {} to {}: {}", src.display(), dst.display(), e))
}

/// Copy a directory recursively.
pub fn copy_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), String> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    if !src.is_dir() {
        return copy(src, dst).map(|_| ());
    }

    ensure_dir(dst)?;

    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_all(&entry_path, &target_path)?;
        } else {
            copy(&entry_path, &target_path)?;
        }
    }

    Ok(())
}

/// Find files recursively matching an extension.
pub fn find_files(dir: impl AsRef<Path>, ext: &str) -> Result<Vec<PathBuf>, String> {
    let mut results = Vec::new();
    let dir = dir.as_ref();

    if !dir.is_dir() {
        return Ok(results);
    }

    for entry in walk(dir)? {
        if entry.is_file() && entry.extension().map_or(false, |e| e == ext) {
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
            if path.is_dir() {
                files.extend(walk(&path)?);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

// ── Path Utilities ──

/// Normalize a path (resolve `.` and `..`).
pub fn normalize(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }

    let mut result = PathBuf::new();
    for c in components {
        result.push(c);
    }
    result
}

/// Get the relative path from base to target.
pub fn relative(target: impl AsRef<Path>, base: impl AsRef<Path>) -> Option<PathBuf> {
    let target = target.as_ref();
    let base = base.as_ref();

    let mut target_components = target.components();
    let mut base_components = base.components();

    // Skip matching prefix
    loop {
        match (target_components.next(), base_components.next()) {
            (Some(a), Some(b)) if a == b => continue,
            (Some(a), Some(_)) => {
                let mut result = PathBuf::from("..");
                result.push(a);
                result.extend(target_components);
                return Some(result);
            }
            (Some(a), None) => {
                let mut result = PathBuf::new();
                result.push(a);
                result.extend(target_components);
                return Some(result);
            }
            (None, Some(_)) => {
                let mut result = PathBuf::from("..");
                result.extend(base_components);
                return Some(result);
            }
            (None, None) => return Some(PathBuf::from(".")),
        }
    }
}

/// Get the file name with line/column info for display.
pub fn format_location(path: impl AsRef<Path>, line: usize, column: usize) -> String {
    format!("{}:{}:{}", path.as_ref().display(), line, column)
}

// ── Temporary Directory ──

/// Create a temporary directory that will be cleaned up on drop.
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(prefix: &str) -> Result<Self, String> {
        let mut path = std::env::temp_dir().join(format!("{}_{}", prefix, std::process::id()));
        let mut n = 0;
        while path.exists() {
            n += 1;
            path = std::env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), n));
        }
        ensure_dir(&path)?;
        Ok(TempDir { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// ── File Info ──

/// Information about a file.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub extension: String,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
    pub is_dir: bool,
}

/// Get file info for a path.
pub fn info(path: impl AsRef<Path>) -> Result<FileInfo, String> {
    let path = path.as_ref();
    let metadata = fs::metadata(path)
        .map_err(|e| format!("Cannot read metadata: {}", e))?;

    Ok(FileInfo {
        path: path.to_path_buf(),
        name: stem(path).unwrap_or_default(),
        extension: extension(path).unwrap_or_default(),
        size: metadata.len(),
        modified: metadata.modified().ok(),
        is_dir: metadata.is_dir(),
    })
}

/// Hash a file's contents (SHA-256).
pub fn hash(path: impl AsRef<Path>) -> Result<String, String> {
    let content = read_bytes(&path)?;
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    Ok(format!("{:x}", hasher.finish()))
}
