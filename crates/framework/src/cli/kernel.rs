use std::collections::HashMap;

use crate::cli::command::{Command, Signature};
use crate::cli::exit::ExitCode;
use crate::cli::input::Input;
use crate::cli::output::Output;

pub struct Kernel {
    commands: HashMap<String, Box<dyn Command>>,
    name: String,
    version: String,
}

impl Kernel {
    pub fn new() -> Self {
        Kernel {
            commands: HashMap::new(),
            name: String::from("Viontin"),
            version: String::from("0.1.0"),
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    pub fn register<C: Command + 'static>(mut self, command: C) -> Self {
        let sig = Signature::parse(command.signature());
        let name = sig.name.clone();
        self.commands.insert(name, Box::new(command));
        self
    }

    pub fn register_dyn(mut self, command: Box<dyn Command + 'static>) -> Self {
        let sig = Signature::parse(command.signature());
        let name = sig.name.clone();
        self.commands.insert(name, command);
        self
    }

    /// Remove a command by name.
    pub fn remove(mut self, name: &str) -> Self {
        self.commands.remove(name);
        self
    }

    pub fn run(self, args: &[String]) -> ExitCode {
        let output = Output::new();

        let cmd_name = args.get(1).map(|s| s.as_str()).unwrap_or("");

        if cmd_name.is_empty() || cmd_name == "--help" || cmd_name == "-h" {
            self.print_help(&output);
            return ExitCode::Success;
        }

        if cmd_name == "--version" || cmd_name == "-V" {
            output.line(&format!("{} v{}", self.name, self.version));
            return ExitCode::Success;
        }

        if cmd_name == "list" {
            self.print_list(&output);
            return ExitCode::Success;
        }

        match self.commands.get(cmd_name) {
            Some(cmd) => {
                let sig = Signature::parse(cmd.signature());
                let input = Input::parse(&args[2..], &sig);

                if args.iter().any(|a| a == "--help" || a == "-h") {
                    self.print_command_help(cmd.signature(), cmd.description(), &output);
                    return ExitCode::Success;
                }

                cmd.handle(&input, &output)
            }
            None => {
                output.error(&format!("Command not found: {}", cmd_name));
                output.line("");
                output.line("  Did you mean one of these?");
                self.print_available_commands(&output);
                ExitCode::InvalidArgs
            }
        }
    }

    fn print_help(&self, output: &Output) {
        output.line(&format!(" {} v{}", self.name, self.version));
        output.line("");
        output.line("USAGE:");
        output.line("  viontin <command> [options]");
        output.line("");
        output.line("AVAILABLE COMMANDS:");
        self.print_available_commands(output);
        output.line("");
        output.line("For more info:");
        output.line("  viontin <command> --help");
    }

    fn print_list(&self, output: &Output) {
        self.print_available_commands(output);
    }

    fn print_available_commands(&self, output: &Output) {
        let mut rows: Vec<Vec<String>> = Vec::new();
        rows.push(vec!["  list".to_string(), "List commands".to_string()]);

        let mut cmds: Vec<&str> = self.commands.keys().map(|s| s.as_str()).collect();
        cmds.sort_unstable();

        for name in cmds {
            if let Some(cmd) = self.commands.get(name) {
                rows.push(vec![
                    format!("  {}", name),
                    cmd.description().to_string(),
                ]);
            }
        }

        output.table(rows);
    }

    fn print_command_help(&self, signature: &str, description: &str, output: &Output) {
        let sig = Signature::parse(signature);

        output.line("DESCRIPTION:");
        output.line(&format!("  {}", description));
        output.line("");
        output.line("USAGE:");
        output.line(&format!("  viontin {}", signature));

        if !sig.arguments.is_empty() {
            output.line("");
            output.line("ARGUMENTS:");
            for arg in &sig.arguments {
                let req = if arg.required { "(required)" } else { "(optional)" };
                output.line(&format!("  {} {}", arg.name, req));
            }
        }

        if !sig.options.is_empty() {
            output.line("");
            output.line("OPTIONS:");
            for opt in &sig.options {
                let default_str = opt
                    .default
                    .as_ref()
                    .map(|d| format!(" (default: {})", d))
                    .unwrap_or_default();
                let type_str = if opt.has_value { " (value)" } else { " (flag)" };
                output.line(&format!("  --{}{}{}", opt.name, type_str, default_str));
            }
        }

        output.line("");
        output.line("GLOBAL OPTIONS:");
        output.line("  -h, --help    Show help");
        output.line("  -V, --version Show version");
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}
