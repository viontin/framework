use regex::Regex;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone)]
pub struct Project {
    #[allow(dead_code)]
    pub root: PathBuf,
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone)]
pub struct Module {
    #[allow(dead_code)]
    pub dir: PathBuf,
    pub name: String,
    pub files: Vec<ModuleFile>,
}

#[derive(Debug, Clone)]
pub struct ModuleFile {
    #[allow(dead_code)]
    pub path: PathBuf,
    pub name: String,
    pub exports: Vec<Export>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportKind {
    Struct,
    Function,
    Enum,
    Trait,
    Command,
}

#[derive(Debug, Clone)]
pub struct Export {
    #[allow(dead_code)]
    pub kind: ExportKind,
    pub name: String,
    #[allow(dead_code)]
    pub _line: usize,
}

pub fn scan(root: &Path) -> Project {
    let src = root.join("src");
    let mut modules = Vec::new();

    if !src.is_dir() {
        return Project { root: root.to_path_buf(), modules };
    }

    let mut entries: Vec<fs::DirEntry> = match fs::read_dir(&src) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return Project { root: root.to_path_buf(), modules },
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let files = scan_dir(&path);
            if !files.is_empty() || !has_ignore_flag(&path) {
                modules.push(Module { dir: path, name, files });
            }
        }
    }

    Project { root: root.to_path_buf(), modules }
}

fn scan_dir(dir: &Path) -> Vec<ModuleFile> {
    let mut files = Vec::new();
    let mut entries: Vec<fs::DirEntry> = match fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return files,
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") {
            let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let exports = extract_exports(&path);
            files.push(ModuleFile { path, name, exports });
        }
    }

    files
}

fn extract_exports(path: &Path) -> Vec<Export> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut exports = Vec::new();

    let re_struct = Regex::new(r"pub\s+struct\s+(\w+)").unwrap();
    let re_fn    = Regex::new(r"pub\s+fn\s+(\w+)").unwrap();
    let re_enum  = Regex::new(r"pub\s+enum\s+(\w+)").unwrap();
    let re_trait = Regex::new(r"pub\s+trait\s+(\w+)").unwrap();
    let re_cmd   = Regex::new(r"impl\s+Command\s+for\s+(\w+)").unwrap();

    for line in content.lines() {
        if let Some(cap) = re_struct.captures(line) {
            exports.push(Export { kind: ExportKind::Struct, name: cap[1].to_string(), _line: 0 });
        } else if let Some(cap) = re_fn.captures(line) {
            exports.push(Export { kind: ExportKind::Function, name: cap[1].to_string(), _line: 0 });
        } else if let Some(cap) = re_enum.captures(line) {
            exports.push(Export { kind: ExportKind::Enum, name: cap[1].to_string(), _line: 0 });
        } else if let Some(cap) = re_trait.captures(line) {
            exports.push(Export { kind: ExportKind::Trait, name: cap[1].to_string(), _line: 0 });
        } else if let Some(cap) = re_cmd.captures(line) {
            exports.push(Export { kind: ExportKind::Command, name: cap[1].to_string(), _line: 0 });
        }
    }

    exports
}

fn has_ignore_flag(dir: &Path) -> bool {
    dir.join(".viontin-ignore").exists()
}

pub fn is_cargo_project(dir: &Path) -> bool {
    dir.join("Cargo.toml").exists()
}

use std::process::Command as CargoCmd;
use viontin_tui::{Output, ExitCode};

pub fn exec_cargo(args: &[&str], output: &Output) -> ExitCode {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
    };
    let status = match CargoCmd::new("cargo").args(args).current_dir(&dir).status() {
        Ok(s) => s,
        Err(e) => { output.error(&format!("Failed to run cargo: {}", e)); return ExitCode::Failure; }
    };
    if status.success() { ExitCode::Success }
    else { output.error(&format!("cargo {} failed (exit: {:?})", args.join(" "), status.code())); ExitCode::Failure }
}

#[allow(dead_code)]
pub fn exec_cargo_allow_fail(args: &[&str], output: &Output) -> ExitCode {
    let dir = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
    };
    match CargoCmd::new("cargo").args(args).current_dir(&dir).status() {
        Ok(s) => ExitCode::from(s.code().unwrap_or(1)),
        Err(_) => ExitCode::Failure,
    }
}



