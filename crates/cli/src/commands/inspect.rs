use viontin_tui::{Command, Input, Output, ExitCode};
use crate::project;

pub struct InspectCommand;

impl Command for InspectCommand {
    fn signature(&self) -> &str { "inspect {--models} {--routes} {--commands} {--events} {--jobs} {--mail} {--notifications} {--queries} {--domains}" }
    fn description(&self) -> &str { "Show project structure and exports" }

    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let current_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
        };

        if !project::is_cargo_project(&current_dir) {
            output.error("No Cargo.toml found in current directory");
            return ExitCode::Failure;
        }

        if input.flag("domains") {
            return inspect_domains(&current_dir, output);
        }

        let proj = project::scan(&current_dir);
        let filter = filter_for(input);
        let filtered: Vec<&project::Module> = proj.modules.iter()
            .filter(|m| filter.matches(&m.name))
            .collect();

        output.title("Structure");

        let root_name = current_dir.file_name().unwrap_or_default().to_string_lossy();
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!(" {}", root_name));
        lines.push(" └── src".to_string());

        if filtered.is_empty() {
            let main_rs = current_dir.join("src").join("main.rs");
            if main_rs.exists() {
                lines.push("     └── main.rs".to_string());
            } else {
                lines.push("     └── (empty)".to_string());
            }
        }

        for (mi, module) in filtered.iter().enumerate() {
            let last_mod = mi == filtered.len() - 1;
            let branch = if last_mod { "└──" } else { "├──" };
            let child_prefix = if last_mod { "    " } else { "│   " };

            let count = if !module.files.is_empty() {
                format!(" ({} file{})", module.files.len(), if module.files.len() == 1 { "" } else { "s" })
            } else {
                String::new()
            };
            lines.push(format!(" {} {} {}{}", " │", branch, module.name, count));

            for (fi, file) in module.files.iter().enumerate() {
                let last_file = fi == module.files.len() - 1;
                let fb = if last_file { "└──" } else { "├──" };

                let exports = if file.exports.is_empty() {
                    String::new()
                } else {
                    let names: Vec<&str> = file.exports.iter().map(|e| e.name.as_str()).collect();
                    format!(" ({})", names.join(", "))
                };
                lines.push(format!(" {} {} {} {}{}", " │", child_prefix, fb, file.name, exports));
            }
        }

        for line in &lines {
            output.line(line);
        }

        output.line("");

        if filtered.is_empty() && proj.modules.is_empty() {
            output.info("No modules. Use \x1b[33mviontin make:*\x1b[0m to scaffold one.");
        }

        ExitCode::Success
    }
}

fn inspect_domains(current_dir: &std::path::PathBuf, output: &Output) -> ExitCode {
    let domains_dir = current_dir.join("src").join("domain");

    if !domains_dir.is_dir() {
        output.info("No domains detected (src/domain/ not found)");
        output.line("");
        output.info("Create a domain with: \x1b[33mviontin make:domain <name>\x1b[0m");
        return ExitCode::Success;
    }

    let entries: Vec<_> = match std::fs::read_dir(&domains_dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).collect(),
        Err(e) => {
            output.error(&format!("Failed to read domains: {}", e));
            return ExitCode::Failure;
        }
    };

    if entries.is_empty() {
        output.info("No domains found in src/domain/");
        return ExitCode::Success;
    }

    output.title("Domains");

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let domain_file = path.join("domain.rs");
        let port_file = path.join("port.rs");

        let allows = if domain_file.exists() {
            parse_allows(&domain_file)
        } else {
            Vec::new()
        };

        let allows_str = if allows.is_empty() {
            "\x1b[2m(no dependencies)\x1b[0m".to_string()
        } else {
            allows.iter().map(|a| format!("\x1b[36m{}\x1b[0m", a)).collect::<Vec<_>>().join(", ")
        };

        let has_def = domain_file.exists();
        let has_port = port_file.exists();
        let def_marker = if has_def { "\x1b[32m✓\x1b[0m" } else { "\x1b[33m✗\x1b[0m" };
        let port_marker = if has_port { "\x1b[32m✓\x1b[0m" } else { "\x1b[33m✗\x1b[0m" };

        output.line(&format!("  \x1b[1m{}\x1b[0m", name));
        output.line(&format!("    domain.rs  {}   port.rs  {}", def_marker, port_marker));
        output.line(&format!("    allows: {}", allows_str));

        let sub_files = scan_sub_files(&path);
        if !sub_files.is_empty() {
            output.line(&format!("    files: {}", sub_files.join(", ")));
        }
        output.line("");
    }

    ExitCode::Success
}

fn parse_allows(domain_file: &std::path::Path) -> Vec<String> {
    let content = match std::fs::read_to_string(domain_file) {
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

fn scan_sub_files(dir: &std::path::Path) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if name == "domain.rs" || name == "port.rs" || name == "mod.rs" { continue; }
            if path.is_dir() {
                names.push(format!("{}/", name));
            } else if path.extension().is_some_and(|e| e == "rs") {
                names.push(name);
            }
        }
    }
    names.sort();
    names
}

fn no_filter(input: &Input) -> bool {
    !input.flag("models")
        && !input.flag("routes")
        && !input.flag("commands")
        && !input.flag("events")
        && !input.flag("jobs")
        && !input.flag("mail")
        && !input.flag("notifications")
        && !input.flag("queries")
}

struct Filter(Vec<String>);

impl Filter {
    fn matches(&self, name: &str) -> bool {
        self.0.is_empty() || self.0.iter().any(|f| name.contains(f) || f == name)
    }
}

fn filter_for(input: &Input) -> Filter {
    if no_filter(input) {
        return Filter(Vec::new());
    }
    let mut names = Vec::new();
    if input.flag("models") { names.push("models".to_string()); }
    if input.flag("routes") { names.push("routes".to_string()); }
    if input.flag("commands") { names.push("commands".to_string()); }
    if input.flag("events") { names.push("events".to_string()); }
    if input.flag("jobs") { names.push("jobs".to_string()); }
    if input.flag("mail") { names.push("mail".to_string()); }
    if input.flag("notifications") { names.push("notifications".to_string()); }
    if input.flag("queries") { names.push("queries".to_string()); }
    Filter(names)
}