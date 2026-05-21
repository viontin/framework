//! Console command validator — validates CLI input, signatures, arguments, and flags.

use std::fmt;

/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// A single validation finding.
#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub location: Option<String>,
}

impl Finding {
    pub fn error(code: &'static str, msg: impl Into<String>) -> Self {
        Finding { severity: Severity::Error, code, message: msg.into(), location: None }
    }

    pub fn warning(code: &'static str, msg: impl Into<String>) -> Self {
        Finding { severity: Severity::Warning, code, message: msg.into(), location: None }
    }

    pub fn info(code: &'static str, msg: impl Into<String>) -> Self {
        Finding { severity: Severity::Info, code, message: msg.into(), location: None }
    }

    pub fn at(mut self, loc: impl Into<String>) -> Self {
        self.location = Some(loc.into());
        self
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.code, self.message)?;
        if let Some(loc) = &self.location {
            write!(f, " (at {})", loc)?;
        }
        Ok(())
    }
}

/// Collection of validation findings.
#[derive(Debug, Default, Clone)]
pub struct Outcome {
    pub findings: Vec<Finding>,
}

impl Outcome {
    pub fn new() -> Self { Outcome { findings: Vec::new() } }

    pub fn add(&mut self, f: Finding) { self.findings.push(f); }

    pub fn error(&mut self, code: &'static str, msg: impl Into<String>) {
        self.findings.push(Finding::error(code, msg));
    }

    pub fn warning(&mut self, code: &'static str, msg: impl Into<String>) {
        self.findings.push(Finding::warning(code, msg));
    }

    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == Severity::Error)
    }

    pub fn errors(&self) -> Vec<&Finding> {
        self.findings.iter().filter(|f| f.severity == Severity::Error).collect()
    }

    pub fn is_empty(&self) -> bool { self.findings.is_empty() }
}

/// Validates a command signature string (Laravel-style).
pub fn validate_signature(sig: &str) -> Outcome {
    let mut result = Outcome::new();

    if sig.trim().is_empty() {
        result.error("C001", "Command signature must not be empty");
        return result;
    }

    let parts: Vec<&str> = sig.split_whitespace().collect();

    // First token is the command name
    let cmd_name = parts[0];
    if cmd_name.starts_with('{') {
        result.error("C002", "Command name must not be a token");
    }
    if !cmd_name.chars().all(|c| c.is_ascii_alphanumeric() || c == ':' || c == '-') {
        result.warning("C003", format!("Command name '{}' contains unusual characters", cmd_name));
    }

    // Validate each token
    for part in &parts[1..] {
        if part.starts_with('{') && part.ends_with('}') {
            let inner = part.trim_start_matches('{').trim_end_matches('}');

            // Argument
            if !inner.starts_with("--") {
                let optional = inner.ends_with('?');
                let name = inner.trim_end_matches('?');

                if name.is_empty() {
                    result.error("C004", "Argument name must not be empty");
                }
                if optional && !parts[1..].contains(&part) {
                    // An optional argument after a required one is fine
                }
            }
            // Option or flag
            else {
                let opt_name = inner.trim_start_matches("--").trim_end_matches('=');
                if opt_name.is_empty() {
                    result.error("C005", "Option name must not be empty");
                }
            }
        } else {
            result.error("C006", format!("Invalid token '{}' — must be enclosed in {{ }}", part));
        }
    }

    result
}

/// Validates input arguments and options against a signature.
pub fn validate_input(
    signature: &str,
    provided_args: &[String],
    provided_flags: &[String],
    provided_options: &[(&str, &str)],
) -> Outcome {
    let mut result = Outcome::new();

    // Parse the signature to determine required arguments
    let sig_parts: Vec<&str> = signature.split_whitespace().collect();
    let mut required_count = 0;

    for part in &sig_parts[1..] {
        if part.starts_with('{') && part.ends_with('}') {
            let inner = part.trim_start_matches('{').trim_end_matches('}');
            if !inner.starts_with("--") && !inner.ends_with('?') {
                required_count += 1;
            }
        }
    }

    if (provided_args.len() as i32) < required_count {
        result.error("C010", format!(
            "Missing required arguments: expected {} required, got {}",
            required_count, provided_args.len()
        ));
    }

    for flag in provided_flags {
        if !signature.contains(&format!("{{--{}", flag)) {
            result.warning("C011", format!("Unknown flag '--{}'", flag));
        }
    }

    for (opt, _val) in provided_options {
        if !signature.contains(&format!("{{--{}", opt)) {
            result.warning("C012", format!("Unknown option '--{}'", opt));
        }
    }

    result
}

/// Validates a command name.
pub fn validate_command_name(name: &str) -> Outcome {
    let mut result = Outcome::new();

    if name.is_empty() {
        result.error("C020", "Command name must not be empty");
        return result;
    }

    if name.starts_with('-') {
        result.error("C021", "Command name must not start with '-'");
    }

    if name.len() > 100 {
        result.warning("C022", "Command name is unusually long (>100 chars)");
    }

    result
}
