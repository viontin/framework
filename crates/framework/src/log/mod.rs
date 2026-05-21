use std::fmt;
use std::sync::OnceLock;
use crate::env::Environment;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Emergency, Alert, Critical, Error, Warning, Notice, Info, Debug,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self { Level::Emergency => "EMERGENCY", Level::Alert => "ALERT",
            Level::Critical => "CRITICAL", Level::Error => "ERROR",
            Level::Warning => "WARNING", Level::Notice => "NOTICE",
            Level::Info => "INFO", Level::Debug => "DEBUG", }
    }
}

impl fmt::Display for Level { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.as_str()) } }

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: Level, pub message: String, pub channel: String,
    pub context: Vec<(String, String)>, pub timestamp: String,
}

pub trait LogChannel: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn write(&self, entry: &LogEntry);
    fn is_enabled_for(&self, _level: Level) -> bool { true }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat { Structured, Simple }

#[derive(Debug)]
pub struct StdoutLog { min_level: Level, format: LogFormat }

impl StdoutLog {
    pub fn new(min_level: Level) -> Self { StdoutLog { min_level, format: LogFormat::Structured } }
    pub fn with_format(mut self, format: LogFormat) -> Self { self.format = format; self }
}

impl LogChannel for StdoutLog {
    fn name(&self) -> &str { "stdout" }
    fn is_enabled_for(&self, level: Level) -> bool { level <= self.min_level }
    fn write(&self, entry: &LogEntry) {
        if !self.is_enabled_for(entry.level) { return; }
        match self.format {
            LogFormat::Structured => {
                let ctx = if entry.context.is_empty() { String::new() } else {
                    let pairs: Vec<String> = entry.context.iter().map(|(k, v)| format!("\"{}\": \"{}\"", k, v)).collect();
                    format!(" {{{}}}", pairs.join(", "))
                };
                println!("[{}] {}.{}: {}{}", entry.timestamp, entry.channel, entry.level, entry.message, ctx);
            }
            LogFormat::Simple => eprintln!("[{}] {}", entry.level, entry.message),
        }
    }
}

#[derive(Debug)]
pub struct Logger { channels: Vec<Box<dyn LogChannel>>, env: Environment, }

impl Logger {
    pub fn new() -> Self { Logger { channels: Vec::new(), env: Environment::detect() } }
    pub fn with_env(mut self, env: Environment) -> Self { self.env = env; self }
    pub fn add_channel(mut self, ch: impl LogChannel + 'static) -> Self { self.channels.push(Box::new(ch)); self }
    pub fn log(&self, entry: LogEntry) { for ch in &self.channels { ch.write(&entry); } }
    pub fn info(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Info, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: String::new() }); }
    pub fn error(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Error, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: String::new() }); }
    pub fn warning(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Warning, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: String::new() }); }
    pub fn debug(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Debug, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: String::new() }); }
}

impl Default for Logger { fn default() -> Self { Self::new() } }

static GLOBAL_LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init_logger(logger: Logger) {
    GLOBAL_LOGGER.set(logger).unwrap_or_else(|_| panic!("Global logger already initialized"));
}

pub fn log_info(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.info(msg); } }
pub fn log_error(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.error(msg); } }
pub fn log_warning(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.warning(msg); } }
pub fn log_debug(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.debug(msg); } }

pub fn default_logger() -> Logger {
    let env = Environment::detect();
    let min_level = if env.is_local() { Level::Debug } else { Level::Warning };
    Logger::new().with_env(env).add_channel(StdoutLog::new(min_level))
}
