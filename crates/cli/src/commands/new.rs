use std::process::Command as CargoCmd;
use viontin_tui::{Command, Input, Output, ExitCode};

pub struct NewCommand;

impl Command for NewCommand {
    fn signature(&self) -> &str { "new {name} {--with} {--force}" }
    fn description(&self) -> &str { "Scaffold a new Rust project" }

    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let name = match input.argument::<String>("name") {
            Ok(n) => n,
            Err(e) => { output.error(&e); return ExitCode::InvalidArgs; }
        };

        let with = input.option::<String>("with").and_then(|r| r.ok()).unwrap_or_default();
        let force = input.flag("force");

        let current_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
        };
        let root = current_dir.join(&name);

        if root.exists() && !force {
            output.error("Directory already exists. Use --force to overwrite.");
            return ExitCode::InvalidArgs;
        }

        output.title("New Project");
        output.info(&format!("Name: {}", name));
        output.line("");

        // Create directory
        if !root.exists() {
            std::fs::create_dir_all(&root).map_err(|e| {
                output.error(&e.to_string()); ExitCode::Failure
            }).ok();
        }

        // Run cargo init
        let status = CargoCmd::new("cargo")
            .arg("init")
            .arg("--name")
            .arg(&name)
            .current_dir(&root)
            .status();

        match status {
            Ok(s) if s.success() => {
                output.success("Project created with cargo init");
            }
            Ok(s) => {
                output.error(&format!("cargo init failed (exit: {:?})", s.code()));
                return ExitCode::Failure;
            }
            Err(e) => {
                output.error(&format!("Failed to run cargo: {}", e));
                return ExitCode::Failure;
            }
        }

        // Parse --with flag (comma-separated list)
        let dirs: Vec<&str> = with.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        for dir in &dirs {
            let dir_path = root.join("src").join(dir);
            std::fs::create_dir_all(&dir_path).ok();
            std::fs::write(dir_path.join(".gitkeep"), "").ok();
            output.success(&format!("Created: src/{}/", dir));
        }

        // Default: create routes/ if --with is empty but not a basic project
        // (actually don't — keep it minimal by default)

        output.line("");
        output.info("Next:");
        output.line(&format!("  cd {}", name));
        if !dirs.is_empty() {
            output.line(&format!("  viontin inspect"));
        } else {
            output.line("  viontin dev");
        }

        ExitCode::Success
    }
}
