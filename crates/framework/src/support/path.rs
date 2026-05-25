use std::path::PathBuf;

fn find_root() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = Some(cwd.as_path());
        while let Some(d) = dir {
            if d.join("Cargo.toml").exists() { return d.to_path_buf(); }
            dir = d.parent();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn base_path(path: &str) -> PathBuf {
    let root = find_root();
    if path.is_empty() { root } else { root.join(path) }
}

pub fn base_path_glob(pattern: &str) -> Result<Vec<PathBuf>, String> {
    let full = find_root().join(pattern);
    let full_str = full.to_string_lossy().to_string();
    let entries = glob::glob(&full_str).map_err(|e| format!("Invalid glob pattern: {}", e))?;
    let mut results = Vec::new();
    for entry in entries { match entry { Ok(p) => results.push(p), Err(e) => return Err(format!("Glob error: {}", e)), } }
    results.sort();
    Ok(results)
}

pub fn url(path: &str) -> String {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() { return "/".to_string(); }
    format!("/{}", trimmed)
}
