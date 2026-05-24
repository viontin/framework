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

fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    // ISO 8601-ish format: YYYY-MM-DDTHH:MM:SSZ
    let secs = d % 86400; let days = d / 86400;
    let y = days as i64 - 719468 ;
    let era = if y >= 0 { y } else { y - 146096 } / 146097;
    let doe = y - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + 400 * era;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp as usize + 3 } else { mp as usize - 9 };
    let year = if month <= 2 { y as usize + 1 } else { y as usize };
    let h = secs / 3600; let m = (secs % 3600) / 60; let s_val = secs % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, h, m, s_val)
}

impl Logger {
    pub fn new() -> Self { Logger { channels: Vec::new(), env: Environment::detect() } }
    pub fn with_env(mut self, env: Environment) -> Self { self.env = env; self }
    pub fn add_channel(mut self, ch: impl LogChannel + 'static) -> Self { self.channels.push(Box::new(ch)); self }
    pub fn log(&self, entry: LogEntry) { for ch in &self.channels { ch.write(&entry); } }
    pub fn emergency(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Emergency, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: timestamp() }); }
    pub fn alert(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Alert, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: timestamp() }); }
    pub fn critical(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Critical, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: timestamp() }); }
    pub fn error(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Error, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: timestamp() }); }
    pub fn warning(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Warning, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: timestamp() }); }
    pub fn notice(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Notice, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: timestamp() }); }
    pub fn info(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Info, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: timestamp() }); }
    pub fn debug(&self, msg: impl Into<String>) { self.log(LogEntry { level: Level::Debug, message: msg.into(), channel: "app".into(), context: Vec::new(), timestamp: timestamp() }); }
}

impl Default for Logger { fn default() -> Self { Self::new() } }

static GLOBAL_LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init_logger(logger: Logger) {
    GLOBAL_LOGGER.set(logger).unwrap_or_else(|_| panic!("Global logger already initialized"));
}

pub fn log_emergency(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.emergency(msg); } }
pub fn log_alert(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.alert(msg); } }
pub fn log_critical(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.critical(msg); } }
pub fn log_error(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.error(msg); } }
pub fn log_warning(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.warning(msg); } }
pub fn log_notice(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.notice(msg); } }
pub fn log_info(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.info(msg); } }
pub fn log_debug(msg: impl Into<String>) { if let Some(l) = GLOBAL_LOGGER.get() { l.debug(msg); } }

pub fn default_logger() -> Logger {
    let env = Environment::detect();
    let min_level = if env.is_local() { Level::Debug } else { Level::Warning };
    Logger::new().with_env(env).add_channel(StdoutLog::new(min_level))
}
