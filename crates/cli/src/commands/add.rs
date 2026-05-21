use std::path::{Path, PathBuf};
use viontin_tui::{Command, Input, Output, ExitCode};
use crate::project;

pub struct AddCommand;

#[derive(Debug)]
struct Pkg {
    name: &'static str,
    dir: &'static str,
    features: &'static [&'static str],
    desc: &'static str,
}

static PACKAGES: &[Pkg] = &[
    Pkg { name: "viontin",       dir: "crates/viontin",  features: &[],      desc: "Unified facade" },
    Pkg { name: "viontin-framework", dir: "crates/framework", features: &[], desc: "Runtime implementations with types & traits" },
    Pkg { name: "viontin-tui",   dir: "crates/tui",      features: &["prompts"], desc: "CLI toolkit" },
    Pkg { name: "viontin-orm",   dir: "../orm/crates/viontin-orm", features: &[], desc: "Multi-driver ORM" },
    Pkg { name: "viontin-gems",  dir: "../gems/crates/viontin-gems", features: &[], desc: "Plugin system" },
];

impl Command for AddCommand {
    fn signature(&self) -> &str { "add {package} {--path} {--git}" }
    fn description(&self) -> &str { "Add a Rust dependency to your project" }

    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let raw = match input.argument::<String>("package") {
            Ok(p) => p,
            Err(e) => { output.error(&e); return ExitCode::InvalidArgs; }
        };

        let (package, version) = match raw.split_once('@') {
            Some((name, ver)) => (name.to_string(), Some(ver.to_string())),
            None => (raw, None),
        };

        if package.is_empty() {
            output.error("Package name required");
            return ExitCode::InvalidArgs;
        }

        let current_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
        };

        if !project::is_cargo_project(&current_dir) {
            output.error("No Cargo.toml found in current directory");
            return ExitCode::Failure;
        }

        let cargo_toml = current_dir.join("Cargo.toml");
        let explicit_path = input.option::<String>("path").and_then(|r| r.ok());
        let explicit_git = input.option::<String>("git").and_then(|r| r.ok());

        // Handle `all` — add all viontin packages
        if package == "all" {
            let root = find_framework_root();
            if let Some(root) = root {
                for pkg in PACKAGES.iter() {
                    if let Err(e) = add_dep(&cargo_toml, &root, pkg, output) {
                        output.error(&e);
                    }
                }
                output.success("All viontin packages added");
            } else {
                output.error("Not in viontin monorepo. Use --path, --git, or @version for each package.");
            }
            return ExitCode::Success;
        }

        // Build the dependency line
        let dep_line = if let Some(path) = &explicit_path {
            format!("{} = {{ path = \"{}\" }}", package, path)
        } else if let Some(url) = &explicit_git {
            format!("{} = {{ git = \"{}\" }}", package, url)
        } else if let Some(ver) = &version {
            format!("{} = \"^{}\"", package, ver)
        } else if let Some(pkg) = PACKAGES.iter().find(|p| p.name == package) {
            // Known viontin package — add as path dependency
            if let Some(root) = find_framework_root() {
                let crate_path = match std::fs::canonicalize(root.join(pkg.dir)) {
                    Ok(p) => p,
                    Err(e) => {
                        output.error(&format!("Cannot resolve path for {}: {}", package, e));
                        return ExitCode::Failure;
                    }
                };
                let rel = relative_path(cargo_toml.parent().unwrap(), &crate_path);
                if pkg.features.is_empty() {
                    format!("{} = {{ path = \"{}\" }}", package, rel.to_string_lossy())
                } else {
                    let feats: Vec<String> = pkg.features.iter().map(|f| format!("\"{}\"", f)).collect();
                    format!("{} = {{ path = \"{}\", features = [{}] }}", package, rel.to_string_lossy(), feats.join(", "))
                }
            } else {
                format!("{} = \"*\"", package)
            }
        } else {
            // Unknown package — fallback to crates.io
            format!("{} = \"*\"", package)
        };

        match write_dep(&cargo_toml, &dep_line, &package, output) {
            Ok(_) => {
                output.success(&format!("Added {}", package));
                output.line("");
                output.info("Next: run \x1b[33mviontin build\x1b[0m to verify");
                ExitCode::Success
            }
            Err(e) => {
                output.error(&e);
                ExitCode::Failure
            }
        }
    }
}

fn find_framework_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?;
    for _ in 0..4 { dir = dir.parent()?; }
    if dir.join("crates").join("framework").join("Cargo.toml").exists() {
        Some(dir.to_path_buf())
    } else {
        None
    }
}

fn add_dep(cargo_toml: &Path, framework_root: &Path, pkg: &Pkg, output: &Output) -> Result<(), String> {
    let crate_path = std::fs::canonicalize(framework_root.join(pkg.dir))
        .map_err(|e| format!("Cannot resolve path for {}: {}", pkg.name, e))?;
    let rel = relative_path(cargo_toml.parent().unwrap(), &crate_path);
    let dep_line = if pkg.features.is_empty() {
        format!("{} = {{ path = \"{}\" }}", pkg.name, rel.to_string_lossy())
    } else {
        let feats: Vec<String> = pkg.features.iter().map(|f| format!("\"{}\"", f)).collect();
        format!("{} = {{ path = \"{}\", features = [{}] }}", pkg.name, rel.to_string_lossy(), feats.join(", "))
    };
    write_dep(cargo_toml, &dep_line, pkg.name, output)
}

fn write_dep(cargo_toml: &Path, dep_line: &str, name: &str, output: &Output) -> Result<(), String> {
    let content = std::fs::read_to_string(cargo_toml).map_err(|e| format!("Read error: {}", e))?;
    let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let exists = lines.iter().any(|l| l.trim_start().starts_with(&format!("{} =", name)));

    if exists {
        let mut new_lines = Vec::new();
        for line in &lines {
            if line.trim_start().starts_with(&format!("{} =", name)) {
                new_lines.push(dep_line.to_string());
            } else {
                new_lines.push(line.clone());
            }
        }
        std::fs::write(cargo_toml, new_lines.join("\n")).map_err(|e| format!("Write error: {}", e))?;
        output.info(&format!("Updated {}", name));
    } else {
        let new_lines = insert_after_deps(&lines, dep_line);
        std::fs::write(cargo_toml, new_lines.join("\n")).map_err(|e| format!("Write error: {}", e))?;
        output.info(&format!("  added {}", name));
    }
    Ok(())
}

fn insert_after_deps(lines: &[String], dep_line: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut in_deps = false;
    let mut inserted = false;

    for line in lines {
        result.push(line.to_string());
        if line.trim() == "[dependencies]" {
            in_deps = true;
        } else if in_deps && !inserted {
            if line.trim().starts_with('[') || line.trim().is_empty() {
                result.pop();
                result.push(dep_line.to_string());
                result.push(String::new());
                result.push(line.to_string());
                inserted = true;
                in_deps = false;
            }
        }
    }

    if !inserted {
        result.push(dep_line.to_string());
        result.push(String::new());
    }

    result
}

fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let mut from_components = from.components().peekable();
    let mut to_components = to.components().peekable();
    while let (Some(a), Some(b)) = (from_components.peek(), to_components.peek()) {
        if a == b { from_components.next(); to_components.next(); }
        else { break; }
    }
    let mut result = PathBuf::new();
    for _ in from_components { result.push(".."); }
    for component in to_components { result.push(component); }
    result
}
