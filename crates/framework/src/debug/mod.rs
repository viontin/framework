//! Debug utilities — inspired by Laravel's debugging helpers.
//!
//! Provides `dump()`, `dd()`, profiling, and debugging tools
//! to improve developer experience.

use std::fmt;
use std::time::{Instant, Duration};
use crate::env::Environment;

// ── Dump & Die ──

/// Dump a value to stderr with formatting.
///
/// ```rust
/// let count = 42;
/// dump(&count); // prints "int(42)" to stderr
/// ```
pub fn dump(value: &dyn fmt::Debug) {
    eprintln!("  {}", value_fmt(value));
}

/// Dump and die — prints the value then terminates.
///
/// ```rust
/// let items = vec![1, 2, 3];
/// dd(&items); // prints + exits
/// ```
pub fn dd(value: &dyn fmt::Debug) -> ! {
    eprintln!("\n  {}", value_fmt(value));
    eprintln!("\n  [dd] Execution halted.\n");
    std::process::exit(0);
}

/// Dump multiple values at once.
pub fn dump_many(values: &[&dyn fmt::Debug]) {
    for (i, v) in values.iter().enumerate() {
        eprintln!("  [{}] {}", i, value_fmt(v));
    }
}

/// Dump and die with multiple values.
pub fn dd_many(values: &[&dyn fmt::Debug]) -> ! {
    dump_many(values);
    eprintln!("\n  [dd] Execution halted.\n");
    std::process::exit(0);
}

fn value_fmt(value: &dyn fmt::Debug) -> String {
    let s = format!("{:?}", value);
    let type_name = std::any::type_name_of_val(value);
    // Simplify type names
    let short_name = type_name.rsplit("::").next().unwrap_or(type_name);
    format!("{}({})", short_name, s)
}

// ── Profiling ──

/// Simple profiler for measuring code execution.
#[derive(Debug)]
pub struct Profiler {
    start: Instant,
    markers: Vec<(String, Duration)>,
    last: Instant,
}

impl Profiler {
    pub fn new() -> Self {
        let now = Instant::now();
        Profiler {
            start: now,
            markers: Vec::new(),
            last: now,
        }
    }

    /// Add a timing marker.
    pub fn mark(&mut self, name: impl Into<String>) {
        let now = Instant::now();
        let _elapsed_since_start = now.duration_since(self.start);
        let elapsed_since_last = now.duration_since(self.last);
        self.markers.push((name.into(), elapsed_since_last));
        self.last = now;
    }

    /// Print the profiler report.
    pub fn report(&self) {
        eprintln!("\n  ┌─────────────────────────────────────────────┐");
        eprintln!("  │              Profiler Report                │");
        eprintln!("  ├─────────────────────────────────────────────┤");
        for (name, dur) in &self.markers {
            let us = dur.as_micros();
            let label = if us > 1000 {
                format!("{:.3} ms", us as f64 / 1000.0)
            } else {
                format!("{} μs", us)
            };
            eprintln!("  │ {:<20}  {:>20} │", name, label);
        }
        let total = self.last.duration_since(self.start);
        let total_us = total.as_micros();
        let total_label = if total_us > 1000 {
            format!("{:.3} ms", total_us as f64 / 1000.0)
        } else {
            format!("{} μs", total_us)
        };
        eprintln!("  ├─────────────────────────────────────────────┤");
        eprintln!("  │ {:<20}  {:>20} │", "Total", total_label);
        eprintln!("  └─────────────────────────────────────────────┘\n");
    }
}

impl Default for Profiler {
    fn default() -> Self { Self::new() }
}

// ── Benchmark ──

/// Benchmark a function, printing execution time.
pub fn benchmark<F: FnOnce()>(name: &str, f: F) {
    let start = Instant::now();
    f();
    let dur = start.elapsed();
    let us = dur.as_micros();
    if us > 1000 {
        eprintln!("  [bench] {}: {:.3} ms", name, us as f64 / 1000.0);
    } else {
        eprintln!("  [bench] {}: {} μs", name, us);
    }
}

/// Benchmark a function and return its result.
pub fn benchmark_with<F: FnOnce() -> T, T>(name: &str, f: F) -> T {
    let start = Instant::now();
    let result = f();
    let dur = start.elapsed();
    let us = dur.as_micros();
    if us > 1000 {
        eprintln!("  [bench] {}: {:.3} ms", name, us as f64 / 1000.0);
    } else {
        eprintln!("  [bench] {}: {} μs", name, us);
    }
    result
}

// ── Memory Usage ──

/// Get current memory usage (platform-dependent).
pub fn memory_usage() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    return line.trim().to_string();
                }
            }
        }
    }
    "N/A".to_string()
}

// ── Debug Mode ──

/// Check if debug mode is enabled (based on environment).
pub fn is_debug_mode() -> bool {
    let env = Environment::detect();
    env.is_local() || env.is_testing()
}

/// Only execute the closure in debug mode.
pub fn debug_only<F: FnOnce()>(f: F) {
    if is_debug_mode() {
        f();
    }
}

/// Only execute in non-production environments.
pub fn when_local<F: FnOnce()>(f: F) {
    if Environment::detect().is_local() {
        f();
    }
}
