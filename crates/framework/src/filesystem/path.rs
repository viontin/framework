use std::path::{Path, PathBuf};

pub fn normalize(path: impl AsRef<Path>) -> PathBuf {
    let mut components = Vec::new();
    for component in path.as_ref().components() {
        match component {
            std::path::Component::ParentDir => { components.pop(); }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    let mut result = PathBuf::new();
    for c in components { result.push(c); }
    result
}

pub fn relative(target: impl AsRef<Path>, base: impl AsRef<Path>) -> Option<PathBuf> {
    let mut target_c = target.as_ref().components();
    let mut base_c = base.as_ref().components();
    loop {
        match (target_c.next(), base_c.next()) {
            (Some(a), Some(b)) if a == b => continue,
            (Some(a), Some(_)) => { let mut r = PathBuf::from(".."); r.push(a); r.extend(target_c); return Some(r); }
            (Some(a), None) => { let mut r = PathBuf::new(); r.push(a); r.extend(target_c); return Some(r); }
            (None, Some(_)) => { let mut r = PathBuf::from(".."); r.extend(base_c); return Some(r); }
            (None, None) => return Some(PathBuf::from(".")),
        }
    }
}

pub fn format_location(path: impl AsRef<Path>, line: usize, column: usize) -> String {
    format!("{}:{}:{}", path.as_ref().display(), line, column)
}
