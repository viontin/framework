use std::time::Instant;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::log::{LogChannel, LogEntry, Level};

/// Styled terminal output — inspired by Laravel's OutputStyle.
#[derive(Debug)]
pub struct Output {
    supports_ansi: bool,
}

impl Output {
    pub fn new() -> Self {
        let supports_ansi = supports_ansi();
        Output { supports_ansi }
    }

    /// Create an Output configured from the environment.
    /// Uses `APP_ENV` and `APP_DEBUG` env vars to determine output style.
    pub fn from_env() -> Self {
        let env = crate::env::Environment::detect();
        let supports_ansi = supports_ansi();

        // In testing or CI, disable ANSI even if terminal supports it
        let force_plain = std::env::var("CI").is_ok()
            || std::env::var("NO_COLOR").is_ok()
            || env.is_testing();

        Output {
            supports_ansi: supports_ansi && !force_plain,
        }
    }

    fn style(&self, text: &str, code: &str) -> String {
        if self.supports_ansi {
            format!("\x1b[{}m{}\x1b[0m", code, text)
        } else {
            text.to_string()
        }
    }

    pub fn line(&self, text: &str) {
        println!("{}", text);
    }

    pub fn info(&self, text: &str) {
        println!("  {} {}", self.style("ℹ", "34"), text);
    }

    pub fn success(&self, text: &str) {
        println!("  {} {}", self.style("✔", "32"), text);
    }

    pub fn warn(&self, text: &str) {
        println!("  {} {}", self.style("⚠", "33"), text);
    }

    pub fn error(&self, text: &str) {
        eprintln!("  {} {}", self.style("✘", "31"), text);
    }

    pub fn comment(&self, text: &str) {
        println!("  {}", self.style(text, "90"));
    }

    pub fn title(&self, text: &str) {
        let line = "─".repeat(text.len() + 4);
        println!("\n {} ", self.style(&line, "36"));
        println!("  {}  ", self.style(text, "36;1"));
        println!(" {} ", self.style(&line, "36"));
    }

    pub fn table(&self, rows: Vec<Vec<String>>) {
        if rows.is_empty() {
            return;
        }

        let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if col_count == 0 {
            return;
        }

        // Calculate column widths
        let mut widths = vec![0usize; col_count];
        for row in &rows {
            for (i, cell) in row.iter().enumerate() {
                widths[i] = widths[i].max(cell.len());
            }
        }

        let _total_width: usize = widths.iter().sum::<usize>() + (col_count - 1) * 3 + 4;
        let sep = format!("+{}+", widths.iter().map(|w| "─".repeat(w + 2)).collect::<Vec<_>>().join("+"));

        println!("{}", self.style(&sep, "90"));

        for (row_idx, row) in rows.iter().enumerate() {
            let mut line = String::from(" ");
            for (i, cell) in row.iter().enumerate() {
                let padded = format!(" {:<width$} ", cell, width = widths[i]);
                line.push_str(&self.style(&padded, "90"));
                if i < col_count - 1 {
                    line.push_str(&self.style("│", "90"));
                }
            }
            line.push(' ');
            println!("{}", line);

            if row_idx == 0 {
                println!("{}", self.style(&sep, "90"));
            }
        }

        println!("{}", self.style(&sep, "90"));
    }

    pub fn listing(&self, items: Vec<&str>) {
        for item in items {
            println!("  {} {}", self.style("•", "33"), item);
        }
    }

    pub fn task<F: FnOnce() -> Result<String, String>>(&self, description: &str, f: F) {
        print!("  {} {} ... ", self.style("➤", "36"), description);
        std::io::Write::flush(&mut std::io::stdout()).ok();

        match f() {
            Ok(msg) => {
                println!("{} {}", self.style("✔", "32"), self.style(&msg, "90"));
            }
            Err(e) => {
                println!("{} {}", self.style("✘", "31"), e);
            }
        }
    }

    pub fn spinner(&self) -> Spinner {
        Spinner::new(self.supports_ansi)
    }

    pub fn progress_bar(&self, total: u64) -> ProgressBar {
        ProgressBar::new(total, self.supports_ansi)
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new()
    }
}

// ── Spinner ──

pub struct Spinner {
    frames: &'static [&'static str],
    message: String,
    done: Arc<AtomicBool>,
    supports_ansi: bool,
}

impl Spinner {
    fn new(supports_ansi: bool) -> Self {
        Spinner {
            frames: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            message: String::new(),
            done: Arc::new(AtomicBool::new(false)),
            supports_ansi,
        }
    }

    pub fn start(&mut self, message: &str) {
        self.message = message.to_string();
        if !self.supports_ansi {
            print!("  {} ... ", message);
            std::io::Write::flush(&mut std::io::stdout()).ok();
            return;
        }

        let done = self.done.clone();
        let msg = self.message.clone();
        let frames = self.frames;

        std::thread::spawn(move || {
            let mut i = 0;
            while !done.load(Ordering::Relaxed) {
                let frame = frames[i % frames.len()];
                print!("\r  {} {}", frame, msg);
                std::io::Write::flush(&mut std::io::stdout()).ok();
                std::thread::sleep(std::time::Duration::from_millis(80));
                i += 1;
            }
        });
    }

    pub fn ok(&self, message: &str) {
        if !self.supports_ansi {
            println!("{}", message);
            return;
        }
        self.done.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(100));
        println!("\r  \x1b[32m✔\x1b[0m {} {}", self.message, "\x1b[90m".to_string() + message + "\x1b[0m");
    }

    pub fn fail(&self, message: &str) {
        if !self.supports_ansi {
            eprintln!("FAILED: {}", message);
            return;
        }
        self.done.store(true, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(100));
        eprintln!("\r  \x1b[31m✘\x1b[0m {} {}", self.message, "\x1b[90m".to_string() + message + "\x1b[0m");
    }
}

// ── Progress Bar ──

pub struct ProgressBar {
    total: u64,
    current: u64,
    start: Instant,
    supports_ansi: bool,
}

impl ProgressBar {
    fn new(total: u64, supports_ansi: bool) -> Self {
        ProgressBar {
            total,
            current: 0,
            start: Instant::now(),
            supports_ansi,
        }
    }

    pub fn advance(&mut self, n: u64) {
        self.current += n;
        self.render();
    }

    pub fn set(&mut self, n: u64) {
        self.current = n;
        self.render();
    }

    fn render(&self) {
        if !self.supports_ansi {
            return;
        }

        let pct = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as u32
        } else {
            0
        };

        let bar_width: usize = 30;
        let filled = (pct as f64 / 100.0 * bar_width as f64) as usize;
        let empty = bar_width.saturating_sub(filled);
        let elapsed = self.start.elapsed().as_secs();

        let bar = format!(
            "\r  {} [{}{}] {}% ({}/{})",
            "\x1b[34m▐\x1b[0m",
            "\x1b[34m█\x1b[0m".repeat(filled),
            "\x1b[90m░\x1b[0m".repeat(empty),
            pct,
            self.current,
            self.total
        );

        print!("{}{}", bar, format!(" {:>5}s", elapsed));
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        if self.supports_ansi {
            println!();
        }
    }
}

// ── Log — routes framework log entries through styled Output ──

/// A `LogChannel` that routes log entries through Viontin's styled `Output`.
///
/// Use this when you want framework log messages (from config, env, debug, etc.)
/// to appear with proper CLI styling (colors, icons) instead of raw stdout.
///
/// # Example
///
/// ```rust
/// use viontin_tui::output::{Output, Log};
/// use crate::log::Logger;
///
/// let out = Output::new();
/// let logger = Logger::new()
///     .add_channel(Log::new(&out, viontin_framework::log::Level::Info));
/// ```
#[derive(Debug)]
pub struct Log<'a> {
    output: &'a Output,
    min_level: Level,
}

impl<'a> Log<'a> {
    pub fn new(output: &'a Output, min_level: Level) -> Self {
        Log { output, min_level }
    }
}

impl LogChannel for Log<'_> {
    fn name(&self) -> &str { "output" }

    fn is_enabled_for(&self, level: Level) -> bool {
        level <= self.min_level
    }

    fn write(&self, entry: &LogEntry) {
        if !self.is_enabled_for(entry.level) {
            return;
        }

        match entry.level {
            Level::Emergency | Level::Alert | Level::Critical | Level::Error => {
                self.output.error(&entry.message);
            }
            Level::Warning => {
                self.output.warn(&entry.message);
            }
            Level::Notice | Level::Info => {
                self.output.info(&entry.message);
            }
            Level::Debug => {
                self.output.comment(&entry.message);
            }
        }
    }
}

fn supports_ansi() -> bool {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return false;
    }
    std::env::var("TERM").map_or(true, |t| t != "dumb")
}
