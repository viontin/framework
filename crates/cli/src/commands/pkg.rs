use viontin_tui::{Command, Input, Output, ExitCode};
use crate::project;

macro_rules! cargo_cmd {
    ($name:ident, $sig:expr, $desc:expr, $args:expr) => {
        pub struct $name;
        impl Command for $name {
            fn signature(&self) -> &str { $sig }
            fn description(&self) -> &str { $desc }
            fn handle(&self, _input: &Input, output: &Output) -> ExitCode {
                if !project::is_cargo_project(&std::env::current_dir().unwrap_or_default()) {
                    output.error("No Cargo.toml found");
                    return ExitCode::Failure;
                }
                project::exec_cargo($args, output)
            }
        }
    };
    ($name:ident, $sig:expr, $desc:expr, $args:expr, needs_project) => {
        pub struct $name;
        impl Command for $name {
            fn signature(&self) -> &str { $sig }
            fn description(&self) -> &str { $desc }
            fn handle(&self, _input: &Input, output: &Output) -> ExitCode {
                if !project::is_cargo_project(&std::env::current_dir().unwrap_or_default()) {
                    output.error("No Cargo.toml found");
                    return ExitCode::Failure;
                }
                project::exec_cargo($args, output)
            }
        }
    };
}

// ── Commands that require a project ──

cargo_cmd!(CleanCommand, "clean", "Remove build artifacts", &["clean"]);
cargo_cmd!(DocCommand, "doc {--open}", "Build and open documentation", &["doc", "--open"]);
cargo_cmd!(FixCommand, "fix", "Automatically fix compiler warnings", &["fix"]);
cargo_cmd!(BenchCommand, "bench", "Run benchmarks", &["bench"]);
cargo_cmd!(TreeCommand, "tree", "Display dependency tree", &["tree"]);
cargo_cmd!(PackageCommand, "package", "Package as distributable crate", &["package"]);
cargo_cmd!(MetadataCommand, "metadata", "Output the resolved dependencies", &["metadata"]);

// ── Commands that don't require a project ──

cargo_cmd!(PublishCommand, "publish", "Publish to crates.io", &["publish"], needs_project);
cargo_cmd!(UpdateCommand, "update", "Update dependencies", &["update"], needs_project);
cargo_cmd!(FmtCommand, "fmt", "Format Rust code", &["fmt"], needs_project);
cargo_cmd!(ClippyCommand, "clippy", "Lint Rust code", &["clippy"], needs_project);

// ── Project-less commands ──

pub struct InitCommand;
impl Command for InitCommand {
    fn signature(&self) -> &str { "init {--lib} {--name}" }
    fn description(&self) -> &str { "Initialize a Rust project in the current directory" }
    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let is_lib = input.flag("lib");
        let opt_name = input.option::<String>("name").and_then(|r| r.ok());
        let mut raw = vec!["init".to_string()];
        if is_lib { raw.push("--lib".to_string()); }
        if let Some(n) = &opt_name { raw.push("--name".to_string()); raw.push(n.clone()); }
        let refs: Vec<&str> = raw.iter().map(|s| s.as_str()).collect();
        project::exec_cargo(&refs, output)
    }
}

pub struct InstallCommand;
impl Command for InstallCommand {
    fn signature(&self) -> &str { "install {crate}" }
    fn description(&self) -> &str { "Install a Rust binary crate" }
    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let name = match input.argument::<String>("crate") {
            Ok(n) => n,
            Err(e) => { output.error(&e); return ExitCode::InvalidArgs; }
        };
        project::exec_cargo(&["install", &name], output)
    }
}

pub struct SearchCommand;
impl Command for SearchCommand {
    fn signature(&self) -> &str { "search {query}" }
    fn description(&self) -> &str { "Search crates.io" }
    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let query = match input.argument::<String>("query") {
            Ok(q) => q,
            Err(e) => { output.error(&e); return ExitCode::InvalidArgs; }
        };
        project::exec_cargo(&["search", &query], output)
    }
}

pub struct UninstallCommand;
impl Command for UninstallCommand {
    fn signature(&self) -> &str { "uninstall {crate}" }
    fn description(&self) -> &str { "Uninstall a Rust binary crate" }
    fn handle(&self, input: &Input, output: &Output) -> ExitCode {
        let name = match input.argument::<String>("crate") {
            Ok(n) => n,
            Err(e) => { output.error(&e); return ExitCode::InvalidArgs; }
        };
        project::exec_cargo(&["uninstall", &name], output)
    }
}
