//! Logging framework — structured log output via channels.
//!
//! Uses the viontin_core::Logger trait. Built-in StdoutLog writes
//! to stderr with optional structured JSON formatting.

use std::sync::Mutex;

/// Log channel trait for routing log entries through styled output.
pub use viontin_core::{Level, LogEntry};

pub use viontin_core::Logger;

pub trait LogChannel: Send + Sync {
    fn name(&self) -> &str;
    fn is_enabled_for(&self, level: Level) -> bool;
    fn write(&self, entry: &LogEntry);
}

// ── StdoutLog ──

#[derive(Debug)]
pub struct StdoutLog {
    format: LogFormat,
    min_level: Level,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Human,
    Structured,
}

impl StdoutLog {
    pub fn new() -> Self {
        StdoutLog { format: LogFormat::Human, min_level: Level::Debug }
    }

    pub fn format(mut self, format: LogFormat) -> Self {
        self.format = format;
        self
    }

    pub fn min_level(mut self, level: Level) -> Self {
        self.min_level = level;
        self
    }

    fn fmt_entry(&self, entry: &LogEntry) -> String {
        match self.format {
            LogFormat::Human => {
                format!("[{}] {}",
                    entry.level,
                    entry.message)
            }
            LogFormat::Structured => {
                format!("{{\"level\":\"{}\",\"msg\":\"{}\",\"ts\":{}}}",
                    entry.level.as_str(),
                    entry.message.replace('"', "\\\""),
                    entry.timestamp.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
            }
        }
    }
}

impl Default for StdoutLog { fn default() -> Self { Self::new() } }

impl Logger for StdoutLog {
    fn log(&self, entry: &LogEntry) {
        if entry.level as u8 <= self.min_level as u8 {
            eprintln!("{}", self.fmt_entry(entry));
        }
    }

    fn flush(&self) {}
}

// ── MultiLogger — dispatches to multiple loggers ──

pub struct MultiLogger {
    loggers: Vec<Box<dyn Logger + Send + Sync>>,
}

impl std::fmt::Debug for MultiLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiLogger").field("count", &self.loggers.len()).finish()
    }
}

impl MultiLogger {
    pub fn new() -> Self { MultiLogger { loggers: Vec::new() } }

    pub fn add(&mut self, logger: Box<dyn Logger + Send + Sync>) {
        self.loggers.push(logger);
    }
}

impl Default for MultiLogger { fn default() -> Self { Self::new() } }

impl Logger for MultiLogger {
    fn log(&self, entry: &LogEntry) {
        for logger in &self.loggers {
            logger.log(entry);
        }
    }

    fn flush(&self) {
        for logger in &self.loggers {
            logger.flush();
        }
    }
}

// ── Global Logger ──

use std::sync::OnceLock;

struct GlobalLogger(Mutex<Box<dyn Logger + Send + Sync>>);

impl std::fmt::Debug for GlobalLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalLogger").finish()
    }
}

static GLOBAL_LOGGER: OnceLock<GlobalLogger> = OnceLock::new();

fn global() -> &'static Mutex<Box<dyn Logger + Send + Sync>> {
    &GLOBAL_LOGGER.get_or_init(|| GlobalLogger(Mutex::new(Box::new(StdoutLog::new())))).0
}

pub fn default_logger() -> Box<dyn Logger + Send + Sync> {
    Box::new(StdoutLog::new())
}

pub fn init_logger(logger: Box<dyn Logger + Send + Sync>) {
    if let Ok(mut g) = global().lock() { *g = logger; }
}

pub fn set_log_level(level: Level) {
    // Replace global logger with a reconfigured StdoutLog at the given level
    init_logger(Box::new(StdoutLog::new().min_level(level)));
}

// ── Convenience Functions ──

pub fn log_debug(msg: impl Into<String>) {
    let entry = LogEntry::new(Level::Debug, msg);
    if let Ok(g) = global().lock() { g.log(&entry); }
}

pub fn log_info(msg: impl Into<String>) {
    let entry = LogEntry::new(Level::Info, msg);
    if let Ok(g) = global().lock() { g.log(&entry); }
}

pub fn log_warning(msg: impl Into<String>) {
    let entry = LogEntry::new(Level::Warning, msg);
    if let Ok(g) = global().lock() { g.log(&entry); }
}

pub fn log_error(msg: impl Into<String>) {
    let entry = LogEntry::new(Level::Error, msg);
    if let Ok(g) = global().lock() { g.log(&entry); }
}

pub fn log_critical(msg: impl Into<String>) {
    let entry = LogEntry::new(Level::Critical, msg);
    if let Ok(g) = global().lock() { g.log(&entry); }
}
