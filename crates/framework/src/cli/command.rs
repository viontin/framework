use crate::cli::input::Input;
use crate::cli::output::Output;
use crate::cli::exit::ExitCode;

#[derive(Debug, Clone)]
pub struct Signature {
    pub name: String,
    pub arguments: Vec<ArgDef>,
    pub options: Vec<OptDef>,
}

#[derive(Debug, Clone)]
pub struct ArgDef {
    pub name: String,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct OptDef {
    pub name: String,
    pub has_value: bool,
    pub default: Option<String>,
}

impl Signature {
    /// Parse a Laravel-style signature string.
    ///
    /// Examples:
    ///   "make:component {name} {--force} {--type=default}"
    ///   "build {target?} {--release} {--out-dir=dist}"
    pub fn parse(s: &str) -> Self {
        let parts: Vec<&str> = s.split_whitespace().collect();
        let name = parts[0].to_string();

        let mut arguments = Vec::new();
        let mut options = Vec::new();

        for part in &parts[1..] {
            let part = part.trim_start_matches('\\');

            if part.starts_with("{--") {
                // Option or flag
                let inner = part.trim_start_matches("{--").trim_end_matches('}');

                if let Some(eq_pos) = inner.find('=') {
                    let opt_name = inner[..eq_pos].to_string();
                    let default_val = inner[eq_pos + 1..].to_string();
                    options.push(OptDef {
                        name: opt_name,
                        has_value: true,
                        default: if default_val.is_empty() || default_val == "=" {
                            None
                        } else {
                            Some(default_val)
                        },
                    });
                } else {
                    options.push(OptDef {
                        name: inner.to_string(),
                        has_value: false,
                        default: None,
                    });
                }
            } else if part.starts_with('{') {
                let inner = part.trim_start_matches('{').trim_end_matches('}');
                let required = !inner.ends_with('?');
                let arg_name = inner.trim_end_matches('?');
                arguments.push(ArgDef {
                    name: arg_name.to_string(),
                    required,
                });
            }
        }

        Signature {
            name,
            arguments,
            options,
        }
    }
}

pub trait Command: Send + Sync {
    fn signature(&self) -> &str;
    fn description(&self) -> &str {
        ""
    }
    fn handle(&self, input: &Input, output: &Output) -> ExitCode;
}
