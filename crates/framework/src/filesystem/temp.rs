use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug)]
pub struct TempDir { path: PathBuf }

impl TempDir {
    pub fn new(prefix: &str) -> Result<Self, String> {
        let mut path = std::env::temp_dir().join(format!("{}_{}", prefix, std::process::id()));
        let mut n = 0;
        while path.exists() {
            n += 1;
            path = std::env::temp_dir().join(format!("{}_{}_{}", prefix, std::process::id(), n));
        }
        super::dir::ensure_dir(&path)?;
        Ok(TempDir { path })
    }
    pub fn path(&self) -> &Path { &self.path }
}

impl Drop for TempDir {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.path); }
}
