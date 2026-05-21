use super::Domain;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct DomainViolation {
    pub from_domain: String,
    pub to_domain: String,
    pub source_file: String,
    pub imported_path: String,
}

impl DomainViolation {
    pub fn is_error(&self, domains: &[Domain]) -> bool {
        match domains.iter().find(|d| d.name == self.from_domain) {
            Some(d) => !d.allows.contains(&self.to_domain.as_str()),
            None => false,
        }
    }

    pub fn is_warning(&self, domains: &[Domain]) -> bool {
        !self.is_error(domains)
    }
}

pub struct DomainBoundary;

impl DomainBoundary {
    pub fn scan_imports(src_dir: &std::path::Path, domains: &[Domain]) -> Vec<DomainViolation> {
        let mut violations = Vec::new();
        let domain_dir = src_dir.join("domain");

        if !domain_dir.is_dir() {
            return violations;
        }

        let domain_names: HashSet<&str> = domains.iter().map(|d| d.name).collect();

        if let Ok(entries) = std::fs::read_dir(&domain_dir) {
            for entry in entries.flatten() {
                let domain_path = entry.path();
                if !domain_path.is_dir() { continue; }

                let domain_name = match domain_path.file_name() {
                    Some(n) => n.to_string_lossy().to_string(),
                    None => continue,
                };

                if !domain_names.contains(domain_name.as_str()) { continue; }

                Self::scan_domain_files(&domain_path, &domain_name, domains, &mut violations);
            }
        }

        violations
    }

    fn scan_domain_files(
        domain_path: &std::path::Path,
        domain_name: &str,
        domains: &[Domain],
        violations: &mut Vec<DomainViolation>,
    ) {
        let domain_names: HashSet<&str> = domains.iter().map(|d| d.name).collect();

        Self::walk_rs_files(domain_path, &mut |file_path, content| {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.starts_with("use ") { continue; }

                let imported = Self::parse_import_path(trimmed);
                if let Some(ref path) = imported {
                    for other_domain in &domain_names {
                        if *other_domain == domain_name { continue; }
                        if path.contains(other_domain) {
                            violations.push(DomainViolation {
                                from_domain: domain_name.to_string(),
                                to_domain: other_domain.to_string(),
                                source_file: file_path.to_string_lossy().to_string(),
                                imported_path: path.clone(),
                            });
                        }
                    }
                }
            }
        });
    }

    fn walk_rs_files(
        dir: &std::path::Path,
        callback: &mut dyn FnMut(&std::path::Path, &str),
    ) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::walk_rs_files(&path, callback);
                } else if path.extension().map_or(false, |e| e == "rs") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        callback(&path, &content);
                    }
                }
            }
        }
    }

    fn parse_import_path(line: &str) -> Option<String> {
        let line = line.trim();
        let line = line.strip_prefix("use ")?;
        let line = line.trim_end_matches(';');
        let line = line.trim();

        let path = if line.starts_with('{') || line.starts_with("super") || line.starts_with("crate") {
            return None;
        } else {
            line.split("::").next().unwrap_or("")
        };

        if path.is_empty() { None }
        else { Some(path.to_string()) }
    }
}

pub fn check_all_boundaries(domains: &[Domain]) -> Vec<DomainViolation> {
    let current_dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let src_dir = current_dir.join("src");
    if !src_dir.is_dir() {
        return Vec::new();
    }
    DomainBoundary::scan_imports(&src_dir, domains)
}