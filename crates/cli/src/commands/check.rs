use std::path::PathBuf;
use viontin_tui::{Command, Input, Output, ExitCode};
use crate::project;

pub struct CheckCommand;

impl Command for CheckCommand {
    fn signature(&self) -> &str { "check {--arch}" }
    fn description(&self) -> &str { "Type-check the project, or verify architecture boundaries with --arch" }

    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let current_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
        };

        if !project::is_cargo_project(&current_dir) {
            output.error("No Cargo.toml found in current directory");
            return ExitCode::Failure;
        }

        if input.flag("arch") {
            return check_arch(&current_dir, output);
        }

        output.title("Check");
        output.line("");

        let status = std::process::Command::new("cargo")
            .arg("check")
            .current_dir(&current_dir)
            .status();

        match status {
            Ok(s) if s.success() => {
                output.success("No errors");
                ExitCode::Success
            }
            Ok(s) => {
                output.error(&format!("Check failed (exit: {:?})", s.code()));
                ExitCode::Failure
            }
            Err(e) => {
                output.error(&format!("Failed to run cargo: {}", e));
                ExitCode::Failure
            }
        }
    }
}

fn check_arch(project_dir: &PathBuf, output: &Output) -> ExitCode {
    output.title("Architecture Check");
    output.line("");

    let domains_dir = project_dir.join("src").join("domain");
    if !domains_dir.is_dir() {
        output.info("No domains detected (src/domain/ not found)");
        output.line("");
        output.info("Create a domain with: \x1b[33mviontin make:domain <name>\x1b[0m");
        return ExitCode::Success;
    }

    let domain_dirs = match std::fs::read_dir(&domains_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect::<Vec<_>>(),
        Err(e) => {
            output.error(&format!("Failed to read domains directory: {}", e));
            return ExitCode::Failure;
        }
    };

    if domain_dirs.is_empty() {
        output.info("No domains found in src/domain/");
        return ExitCode::Success;
    }

    output.info(&format!("Found {} domain(s):", domain_dirs.len()));
    let mut domain_names: Vec<String> = Vec::new();
    for entry in &domain_dirs {
        let name = entry.file_name().to_string_lossy().to_string();
        let domain_file = entry.path().join("domain.rs");
        let has_def = domain_file.exists();
        let marker = if has_def { "\x1b[32m✓\x1b[0m" } else { "\x1b[33m⚠\x1b[0m" };
        output.line(&format!("  {} {}", marker, name));
        if !has_def {
            output.line(&format!("    \x1b[33mMissing domain.rs — run: viontin make:domain {}\x1b[0m", name));
        }
        domain_names.push(name);
    }
    output.line("");

    let mut violations = Vec::new();

    for entry in &domain_dirs {
        let domain_name = entry.file_name().to_string_lossy().to_string();
        let allowed = parse_domain_allows(&entry.path());

        scan_domain_imports(
            &entry.path(),
            &domain_name,
            &allowed,
            &domain_names,
            &mut violations,
        );
    }

    if violations.is_empty() {
        output.success("✓ All domain boundaries respected");
        ExitCode::Success
    } else {
        let errors = violations.iter().filter(|v| !v.is_warning).count();
        let warnings = violations.len() - errors;

        for v in &violations {
            if v.is_warning {
                output.line(&format!(
                    "\x1b[33m⚠\x1b[0m \x1b[33m{}\x1b[0m → \x1b[33m{}\x1b[0m (allowed but not recommended)",
                    v.from_domain, v.to_domain
                ));
            } else {
                output.line(&format!(
                    "\x1b[31m✘\x1b[0m \x1b[31m{}\x1b[0m → \x1b[31m{}\x1b[0m (NOT allowed)",
                    v.from_domain, v.to_domain
                ));
            }
            output.line(&format!("    in: {}", v.source_file));
            output.line(&format!("    import: {}", v.imported_path));
        }

        output.line("");
        if errors > 0 {
            output.error(&format!("{} error(s), {} warning(s)", errors, warnings));
            ExitCode::Failure
        } else {
            output.info(&format!("0 errors, {} warning(s)", warnings));
            ExitCode::Success
        }
    }
}

#[allow(dead_code)]
struct ParsedDomain {
    allows: Vec<String>,
}

struct Violation {
    from_domain: String,
    to_domain: String,
    source_file: String,
    imported_path: String,
    is_warning: bool,
}

fn parse_domain_allows(domain_path: &std::path::Path) -> Vec<String> {
    let domain_file = domain_path.join("domain.rs");
    let content = match std::fs::read_to_string(&domain_file) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut allows = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(".allows(&[") || trimmed.starts_with(".allows(&") {
            let start = trimmed.find('[').unwrap_or(trimmed.len());
            let end = trimmed.rfind(']').unwrap_or(0);
            if start < end {
                let slice = &trimmed[start + 1..end];
                for item in slice.split(',') {
                    let item = item.trim().trim_matches('"').trim();
                    if !item.is_empty() {
                        allows.push(item.to_string());
                    }
                }
            }
        }
    }
    allows
}

fn scan_domain_imports(
    domain_path: &std::path::Path,
    domain_name: &str,
    allowed: &[String],
    all_domains: &[String],
    violations: &mut Vec<Violation>,
) {
    walk_rs_files(domain_path, &mut |file_path, content| {
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("use ") { continue; }

            let import_path = parse_use_path(trimmed);
            if let Some(ref path) = import_path {
                for other in all_domains {
                    if other == domain_name { continue; }
                    if path.contains(other.as_str()) {
                        let is_warning = allowed.contains(other);
                        violations.push(Violation {
                            from_domain: domain_name.to_string(),
                            to_domain: other.clone(),
                            source_file: file_path.to_string_lossy().to_string(),
                            imported_path: path.clone(),
                            is_warning,
                        });
                    }
                }
            }
        }
    });
}

fn walk_rs_files(dir: &std::path::Path, callback: &mut dyn FnMut(&std::path::Path, &str)) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_rs_files(&path, callback);
            } else if path.extension().is_some_and(|e| e == "rs")
                && let Ok(content) = std::fs::read_to_string(&path) {
                    callback(&path, &content);
                }
        }
    }
}

fn parse_use_path(line: &str) -> Option<String> {
    let line = line.trim();
    let line = line.strip_prefix("use ")?;
    let line = line.trim_end_matches(';');
    let line = line.trim();

    if line.starts_with('{') || line.starts_with("super") || line.starts_with("crate") || line.starts_with("self") {
        return None;
    }

    let first_segment = line.split("::").next().unwrap_or("");
    if first_segment.is_empty() { None }
    else { Some(line.to_string()) }
}