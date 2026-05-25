use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity { Error, Warning, Info, }

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self { Severity::Error => "error", Severity::Warning => "warning", Severity::Info => "info", }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.as_str()) }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: Severity, pub code: &'static str,
    pub message: String, pub location: Option<String>,
}

#[derive(Debug, Default)]
pub struct Outcome { pub findings: Vec<Finding> }

impl Outcome {
    pub fn new() -> Self { Outcome { findings: Vec::new() } }
    pub fn add(&mut self, f: Finding) { self.findings.push(f); }
    pub fn error(&mut self, code: &'static str, msg: impl Into<String>) {
        self.findings.push(Finding { severity: Severity::Error, code, message: msg.into(), location: None });
    }
    pub fn warning(&mut self, code: &'static str, msg: impl Into<String>) {
        self.findings.push(Finding { severity: Severity::Warning, code, message: msg.into(), location: None });
    }
    pub fn info(&mut self, code: &'static str, msg: impl Into<String>) {
        self.findings.push(Finding { severity: Severity::Info, code, message: msg.into(), location: None });
    }
    pub fn has_errors(&self) -> bool { self.findings.iter().any(|f| matches!(f.severity, Severity::Error)) }
    pub fn errors(&self) -> Vec<&Finding> { self.findings.iter().filter(|f| f.severity == Severity::Error).collect() }
    pub fn is_empty(&self) -> bool { self.findings.is_empty() }
    pub fn merge(&mut self, other: Outcome) { self.findings.extend(other.findings); }
}

pub trait Validator: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn validate(&self, ctx: &Context) -> Outcome;
}

#[derive(Debug)]
#[derive(Default)]
pub struct Context {
    pub project_root: Option<String>,
    pub source_files: Vec<String>,
    pub config: Option<String>,
}


pub struct ValidatorGroup { validators: Vec<Box<dyn Validator>> }
impl std::fmt::Debug for ValidatorGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "ValidatorGroup({})", self.validators.len()) }
}
impl ValidatorGroup {
    pub fn new() -> Self { ValidatorGroup { validators: Vec::new() } }
    pub fn add(mut self, v: impl Validator + 'static) -> Self { self.validators.push(Box::new(v)); self }
    pub fn validate_all(&self, ctx: &Context) -> Outcome { let mut r = Outcome::new(); for v in &self.validators { r.merge(v.validate(ctx)); } r }
}
impl Default for ValidatorGroup { fn default() -> Self { Self::new() } }
