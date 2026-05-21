use std::process::Command as CargoCmd;
use viontin_tui::{Command, Input, Output, ExitCode};
use crate::project;

pub struct RunCommand;

impl Command for RunCommand {
    fn signature(&self) -> &str { "run {command} {args*}" }
    fn description(&self) -> &str { "Run a project command" }

    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let cmd = match input.argument::<String>("command") {
            Ok(c) => c,
            Err(e) => { output.error(&e); return ExitCode::InvalidArgs; }
        };

        let current_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
        };

        if !project::is_cargo_project(&current_dir) {
            output.error("No Cargo.toml found in current directory");
            return ExitCode::Failure;
        }

        // Pass everything after "viontin run <command>" to cargo
        let all: Vec<String> = std::env::args().collect();
        let rest: Vec<&str> = all.iter().skip(3).map(|s| s.as_str()).collect();

        let status = CargoCmd::new("cargo")
            .arg("run")
            .arg("--")
            .arg(&cmd)
            .args(&rest)
            .current_dir(&current_dir)
            .status();

        match status {
            Ok(s) if s.success() => ExitCode::Success,
            Ok(s) => {
                output.error(&format!("Command failed (exit: {:?})", s.code()));
                ExitCode::Failure
            }
            Err(e) => {
                output.error(&format!("Failed to run cargo: {}", e));
                ExitCode::Failure
            }
        }
    }
}
