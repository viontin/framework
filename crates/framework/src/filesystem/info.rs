use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub extension: String,
    pub size: u64,
    pub modified: Option<std::time::SystemTime>,
    pub is_dir: bool,
}

pub fn info(path: impl AsRef<Path>) -> Result<FileInfo, String> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|e| format!("Cannot read metadata: {}", e))?;
    Ok(FileInfo {
        path: path.to_path_buf(),
        name: super::file::stem(path).unwrap_or_default(),
        extension: super::file::extension(path).unwrap_or_default(),
        size: metadata.len(),
        modified: metadata.modified().ok(),
        is_dir: metadata.is_dir(),
    })
}

pub fn hash(path: impl AsRef<Path>) -> Result<String, String> {
    let content = super::file::read_bytes(&path)?;
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    Ok(format!("{:x}", hasher.finish()))
}
