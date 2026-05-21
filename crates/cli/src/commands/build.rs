use std::process::Command as CargoCmd;
use viontin_tui::{Command, Input, Output, ExitCode};
use crate::project;

pub struct BuildCommand;

impl Command for BuildCommand {
    fn signature(&self) -> &str { "build {--release}" }
    fn description(&self) -> &str { "Build the project with cargo" }

    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let release = input.flag("release");

        let current_dir = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => { output.error(&e.to_string()); return ExitCode::Failure; }
        };

        if !project::is_cargo_project(&current_dir) {
            output.error("No Cargo.toml found in current directory");
            return ExitCode::Failure;
        }

        output.title("Build");
        if release { output.info("Mode: release"); }
        else { output.info("Mode: debug"); }
        output.line("");

        let mut cmd = CargoCmd::new("cargo");
        cmd.arg("build").current_dir(&current_dir);
        if release { cmd.arg("--release"); }

        let status = cmd.status();

        match status {
            Ok(s) if s.success() => {
                output.success("Build succeeded");
                ExitCode::Success
            }
            Ok(s) => {
                output.error(&format!("Build failed (exit: {:?})", s.code()));
                ExitCode::Failure
            }
            Err(e) => {
                output.error(&format!("Failed to run cargo: {}", e));
                ExitCode::Failure
            }
        }
    }
}
