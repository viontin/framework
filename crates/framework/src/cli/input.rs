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

    /// Return all positional argument values in order.
    pub fn arguments(&self) -> Vec<&str> {
        self.raw_args.iter().skip(1).map(|s| s.as_str()).collect()
    }

    /// Check if an option was explicitly provided (regardless of value).
    pub fn has_option(&self, name: &str) -> bool {
        if self.parsed_options.contains_key(name) {
            return true;
        }
        self.raw_args.iter().any(|a| a == &format!("--{}", name) || a.starts_with(&format!("--{}=", name)))
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
    pub fn has_flag(&self, name: &str) -> bool {
        if self.parsed_flags.contains(name) {
            return true;
        }
        self.raw_args.iter().any(|a| a == &format!("--{}", name))
    }

    /// Get an option value by name. Returns `Ok(None)` if not provided,
    /// `Ok(Some(value))` on successful parse, or `Err(msg)` on parse failure.
    pub fn option<T: std::str::FromStr>(&self, name: &str) -> Result<Option<T>, String> {
        // Check parsed options first (from `--name=value` or `--name value`)
        if let Some(val) = self.parsed_options.get(name) {
            return val.parse::<T>()
                .map(Some)
                .map_err(|_| format!("Failed to parse option '{}'", name));
        }

        // Check raw args for `--name value` form
        if let Some(opt) = self.signature.options.iter().find(|o| o.name == name) {
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
            return match val {
                Some(v) => v.parse::<T>()
                    .map(Some)
                    .map_err(|_| format!("Failed to parse option '{}'", name)),
                None => Ok(None),
            };
        }

        Ok(None)
    }

    /// Get an option value with a default fallback. Never returns Err on parse failure —
    /// falls back to `default` instead.
    pub fn option_or<T: std::str::FromStr>(&self, name: &str, default: T) -> T {
        self.option::<T>(name).ok().flatten().unwrap_or(default)
    }

    /// Get an option value, using `default` if not provided. Returns Err on parse failure.
    pub fn option_or_else<T: std::str::FromStr>(&self, name: &str, default: T) -> Result<T, String> {
        self.option::<T>(name).map(|v| v.unwrap_or(default))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::command::Signature;

    fn parse(args: &[&str], sig: &str) -> Input {
        let raw: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        Input::parse(&raw, &Signature::parse(sig))
    }

    #[test]
    fn test_arguments() {
        let input = parse(&["Alice", "Bob"], "greet {name} {other}");
        let args = input.arguments();
        assert_eq!(args, vec!["Alice", "Bob"]);
    }

    #[test]
    fn test_arguments_empty() {
        let input = parse(&[], "cmd");
        let args: Vec<&str> = input.arguments();
        assert!(args.is_empty());
    }

    #[test]
    fn test_has_option_true() {
        let input = parse(&["cmd", "--name=value"], "cmd {--name=}");
        assert!(input.has_option("name"));
    }

    #[test]
    fn test_has_option_false() {
        let input = parse(&["cmd"], "cmd {--name=}");
        assert!(!input.has_option("name"));
    }
}
