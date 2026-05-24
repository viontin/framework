use std::collections::{HashMap, HashSet};
use crate::cli::command::Signature;

#[derive(Debug)]
pub struct Input {
    pub command_name: String,
    pub raw_args: Vec<String>,
    pub signature: Signature,
    parsed_options: HashMap<String, String>,
    parsed_flags: HashSet<String>,
}

impl Input {
    pub fn new(args: Vec<String>, signature: Signature) -> Self {
        let command_name = signature.name.clone();
        Input {
            command_name,
            raw_args: args,
            signature,
            parsed_options: HashMap::new(),
            parsed_flags: HashSet::new(),
        }
    }

    /// Parse raw CLI args against the signature.
    ///
    /// Handles both `--option=value` and `--option value` forms,
    /// plus standalone `--flag` boolean flags.
    pub fn parse(raw: &[String], signature: &Signature) -> Self {
        let mut positional = Vec::new();
        let mut options = HashMap::new();
        let mut flags = HashSet::new();

        let mut i = 0;
        while i < raw.len() {
            let arg = &raw[i];

            if arg.starts_with("--") {
                let opt = arg.trim_start_matches("--");

                if let Some(eq_pos) = opt.find('=') {
                    let key = opt[..eq_pos].to_string();
                    let val = opt[eq_pos + 1..].to_string();
                    options.insert(key, val);
                } else {
                    let is_known_flag = signature.options.iter().any(|o| o.name == opt && !o.has_value);
                    let is_known_opt = signature.options.iter().any(|o| o.name == opt && o.has_value);

                    if is_known_flag || (!is_known_opt) {
                        flags.insert(opt.to_string());
                    } else if is_known_opt {
                        if i + 1 < raw.len() && !raw[i + 1].starts_with('-') {
                            i += 1;
                            options.insert(opt.to_string(), raw[i].clone());
                        } else if let Some(default) = signature.options.iter()
                            .find(|o| o.name == opt)
                            .and_then(|o| o.default.clone())
                        {
                            options.insert(opt.to_string(), default);
                        }
                    }
                }
            } else {
                positional.push(arg.clone());
            }
            i += 1;
        }

        let mut full: Vec<String> = Vec::new();
        full.push(signature.name.clone());
        full.extend(positional);

        Input {
            command_name: signature.name.clone(),
            raw_args: full,
            signature: Signature::clone(signature),
            parsed_options: options,
            parsed_flags: flags,
        }
    }

    /// Get a positional argument by name.
    pub fn argument<T: std::str::FromStr>(&self, name: &str) -> Result<T, String> {
        let sig_idx = self
            .signature
            .arguments
            .iter()
            .position(|a| a.name == name)
            .ok_or_else(|| format!("Unknown argument: {}", name))?;

        let pos = sig_idx + 1;
        let raw = self
            .raw_args
            .get(pos)
            .ok_or_else(|| format!("Missing required argument: {}", name))?;

        raw.parse::<T>()
            .map_err(|_| format!("Failed to parse argument '{}'", name))
    }

    /// Check if a boolean flag was set.
    pub fn flag(&self, name: &str) -> bool {
        if self.parsed_flags.contains(name) {
            return true;
        }
        self.raw_args.iter().any(|a| a == &format!("--{}", name))
    }

    /// Alias for `flag`.
    pub fn has_flag(&self, name: &str) -> bool {
        self.flag(name)
    }

    /// Get an option value by name. Returns `None` if not provided.
    /// Returns `Some(Err(...))` if parsing fails.
    pub fn option<T: std::str::FromStr>(&self, name: &str) -> Option<Result<T, String>> {
        // Check parsed options first (from `--name=value` or `--name value`)
        if let Some(val) = self.parsed_options.get(name) {
            return Some(
                val.parse::<T>()
                    .map_err(|_| format!("Failed to parse option '{}'", name)),
            );
        }

        // Check raw args for `--name value` form
        for opt in &self.signature.options {
            if opt.name == name {
                let mut found: Option<String> = None;
                let mut i = 0;
                while i < self.raw_args.len() {
                    if self.raw_args[i] == format!("--{}", name) {
                        if let Some(default) = &opt.default {
                            found = Some(default.clone());
                        } else if i + 1 < self.raw_args.len() {
                            found = Some(self.raw_args[i + 1].clone());
                        }
                    }
                    i += 1;
                }

                let val = found.or_else(|| opt.default.clone());
                return val.map(|v| v.parse::<T>()
                            .map_err(|_| format!("Failed to parse option '{}'", name)));
            }
        }

        None
    }
}
