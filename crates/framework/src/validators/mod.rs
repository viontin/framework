use std::collections::HashMap;
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

// ──────────────────────────────────────────────
//  RULE ENGINE — Parsing + Validation
// ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Rule {
    Required,
    String,
    Numeric,
    Alpha,
    AlphaNum,
    AlphaDash,
    Email,
    Min(usize),
    Max(usize),
    Between(usize, usize),
    In(Vec<String>),
    NotIn(Vec<String>),
    Regex(String),
    Confirmed,
    Accepted,
    DateFormat(String),
    StartsWith(String),
    EndsWith(String),
}

impl Rule {
    pub fn name(&self) -> &'static str {
        match self {
            Rule::Required => "required",
            Rule::String => "string",
            Rule::Numeric => "numeric",
            Rule::Alpha => "alpha",
            Rule::AlphaNum => "alpha_num",
            Rule::AlphaDash => "alpha_dash",
            Rule::Email => "email",
            Rule::Min(_) => "min",
            Rule::Max(_) => "max",
            Rule::Between(_, _) => "between",
            Rule::In(_) => "in",
            Rule::NotIn(_) => "not_in",
            Rule::Regex(_) => "regex",
            Rule::Confirmed => "confirmed",
            Rule::Accepted => "accepted",
            Rule::DateFormat(_) => "date",
            Rule::StartsWith(_) => "starts_with",
            Rule::EndsWith(_) => "ends_with",
        }
    }
}

pub fn parse_rules(input: &str) -> Vec<Rule> {
    let mut rules = Vec::new();
    for part in input.split('|') {
        let part = part.trim();
        if part.is_empty() { continue; }
        match parse_single_rule(part) {
            Some(rule) => rules.push(rule),
            None => {/* unknown rule skipped */},
        }
    }
    rules
}

fn parse_single_rule(s: &str) -> Option<Rule> {
    if let Some(args) = s.strip_prefix("min:") {
        return args.parse::<usize>().ok().map(Rule::Min);
    }
    if let Some(args) = s.strip_prefix("max:") {
        return args.parse::<usize>().ok().map(Rule::Max);
    }
    if let Some(args) = s.strip_prefix("between:") {
        let parts: Vec<&str> = args.split(',').collect();
        if parts.len() == 2 {
            let lo = parts[0].parse::<usize>().ok()?;
            let hi = parts[1].parse::<usize>().ok()?;
            return Some(Rule::Between(lo, hi));
        }
        return None;
    }
    if let Some(args) = s.strip_prefix("in:") {
        let vals: Vec<String> = args.split(',').map(|s| s.trim().to_string()).collect();
        return Some(Rule::In(vals));
    }
    if let Some(args) = s.strip_prefix("not_in:") {
        let vals: Vec<String> = args.split(',').map(|s| s.trim().to_string()).collect();
        return Some(Rule::NotIn(vals));
    }
    if let Some(args) = s.strip_prefix("regex:") {
        return Some(Rule::Regex(args.to_string()));
    }
    if let Some(args) = s.strip_prefix("date:") {
        return Some(Rule::DateFormat(args.to_string()));
    }
    if let Some(args) = s.strip_prefix("starts_with:") {
        return Some(Rule::StartsWith(args.to_string()));
    }
    if let Some(args) = s.strip_prefix("ends_with:") {
        return Some(Rule::EndsWith(args.to_string()));
    }
    match s {
        "required" => Some(Rule::Required),
        "string" => Some(Rule::String),
        "numeric" => Some(Rule::Numeric),
        "alpha" => Some(Rule::Alpha),
        "alpha_num" => Some(Rule::AlphaNum),
        "alpha_dash" => Some(Rule::AlphaDash),
        "email" => Some(Rule::Email),
        "confirmed" => Some(Rule::Confirmed),
        "accepted" => Some(Rule::Accepted),
        _ => None,
    }
}

pub fn validate_rule(rule: &Rule, value: Option<&str>, all_values: &HashMap<String, String>, field: &str) -> Option<String> {
    let val = value.unwrap_or("");

    match rule {
        Rule::Required if val.is_empty() => {
            Some(format!("{field} is required"))
        }
        Rule::String if !val.chars().all(|c| c.is_alphabetic() || c.is_whitespace() || c == '-' || c == '_') => {
            Some(format!("{field} must be a string"))
        }
        Rule::Numeric if val.parse::<f64>().is_err() => {
            Some(format!("{field} must be numeric"))
        }
        Rule::Alpha if !val.chars().all(|c| c.is_alphabetic()) => {
            Some(format!("{field} must contain only letters"))
        }
        Rule::AlphaNum if !val.chars().all(|c| c.is_alphanumeric()) => {
            Some(format!("{field} must contain only letters and numbers"))
        }
        Rule::AlphaDash if !val.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') => {
            Some(format!("{field} must contain only letters, numbers, dashes, and underscores"))
        }
        Rule::Email if !is_valid_email(val) => {
            Some(format!("{field} must be a valid email address"))
        }
        Rule::Min(n) if val.len() < *n => {
            Some(format!("{field} must be at least {n} characters"))
        }
        Rule::Max(n) if val.len() > *n => {
            Some(format!("{field} must not exceed {n} characters"))
        }
        Rule::Between(lo, hi) if val.len() < *lo || val.len() > *hi => {
            Some(format!("{field} must be between {lo} and {hi} characters"))
        }
        Rule::In(allowed) if !allowed.contains(&val.to_string()) => {
            Some(format!("{field} must be one of: {allowed}", allowed = allowed.join(", ")))
        }
        Rule::NotIn(disallowed) if disallowed.contains(&val.to_string()) => {
            Some(format!("{field} must not be one of: {disallowed}", disallowed = disallowed.join(", ")))
        }
        Rule::Regex(pattern) => {
            if !simple_pattern_match(val, pattern) {
                return Some(format!("{field} format is invalid"));
            }
            None
        }
        Rule::Confirmed => {
            let confirm_key = format!("{field}_confirmation");
            let confirmation = all_values.get(&confirm_key).map(|s| s.as_str()).unwrap_or("");
            if val != confirmation {
                return Some(format!("{field} confirmation does not match"));
            }
            None
        }
        Rule::Accepted if !matches!(val, "yes" | "on" | "1" | "true") => {
            Some(format!("{field} must be accepted"))
        }
        Rule::DateFormat(fmt) => {
            if !is_valid_date(val, fmt) {
                return Some(format!("{field} must match date format {fmt}"));
            }
            None
        }
        Rule::StartsWith(prefix) if !val.starts_with(prefix.as_str()) => {
            Some(format!("{field} must start with {prefix}"))
        }
        Rule::EndsWith(suffix) if !val.ends_with(suffix.as_str()) => {
            Some(format!("{field} must end with {suffix}"))
        }
        _ => None,
    }
}

fn is_valid_email(s: &str) -> bool {
    if let Some(at) = s.find('@') {
        let local = &s[..at];
        let domain = &s[at + 1..];
        !local.is_empty() && !domain.is_empty() && domain.contains('.') && !domain.starts_with('.')
    } else {
        false
    }
}

fn is_valid_date(s: &str, _fmt: &str) -> bool {
    !s.is_empty()
}

fn simple_pattern_match(s: &str, pattern: &str) -> bool {
    let mut si = 0;
    let mut pi = 0;
    let chars: Vec<char> = s.chars().collect();
    let pats: Vec<char> = pattern.chars().collect();
    let mut star_idx = None;
    let mut match_idx = 0;

    while si < chars.len() {
        if pi < pats.len() && (pats[pi] == chars[si] || pats[pi] == '.') {
            si += 1;
            pi += 1;
        } else if pi < pats.len() && pats[pi] == '*' {
            star_idx = Some(pi);
            match_idx = si;
            pi += 1;
        } else if star_idx.is_some() {
            pi = star_idx.unwrap() + 1;
            match_idx += 1;
            si = match_idx;
        } else {
            return false;
        }
    }

    while pi < pats.len() && pats[pi] == '*' {
        pi += 1;
    }
    pi == pats.len()
}

pub fn validate_rules(rules: &[Rule], data: &HashMap<String, String>) -> Outcome {
    let mut outcome = Outcome::new();
    for (field, raw_value) in data {
        for rule in rules.iter().filter(|r| matches!(r, Rule::Confirmed)) {
            if field.ends_with("_confirmation") {
                continue;
            }
            if let Some(err) = validate_rule(rule, Some(raw_value), data, field) {
                outcome.error("validation", err);
            }
        }
        for rule in rules.iter().filter(|r| !matches!(r, Rule::Confirmed)) {
            if let Some(err) = validate_rule(rule, Some(raw_value), data, field) {
                outcome.error("validation", err);
            }
        }
    }
    outcome
}

// ──────────────────────────────────────────────
//  RULE-BASED VALIDATOR
// ──────────────────────────────────────────────

pub struct RuleValidator {
    name: String,
    pub rules: HashMap<String, Vec<Rule>>,
}

impl fmt::Debug for RuleValidator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuleValidator").field("name", &self.name).field("fields", &self.rules.len()).finish()
    }
}

impl RuleValidator {
    pub fn new(name: &str) -> Self {
        RuleValidator { name: name.into(), rules: HashMap::new() }
    }

    pub fn add_field(mut self, field: &str, rules: &str) -> Self {
        self.rules.insert(field.to_string(), parse_rules(rules));
        self
    }

    pub fn validate_data(&self, data: &HashMap<String, String>) -> Outcome {
        let mut outcome = Outcome::new();
        for (field, rules) in &self.rules {
            let value = data.get(field).map(|s| s.as_str());
            for rule in rules {
                if let Some(err) = validate_rule(rule, value, data, field) {
                    outcome.error("validation", err);
                }
            }
        }
        outcome
    }
}

impl Validator for RuleValidator {
    fn name(&self) -> &str { &self.name }
    fn validate(&self, _ctx: &Context) -> Outcome {
        Outcome::new()
    }
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
