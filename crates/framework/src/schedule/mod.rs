//! Scheduler implementation — runs scheduled jobs.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

pub trait ScheduledJob: fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn handle(&self) -> Result<(), String>;
    fn expression(&self) -> &str;
}

pub fn cron_matches(expression: &str, minute: u32, hour: u32, day: u32, month: u32, weekday: u32) -> bool {
    let parts: Vec<&str> = expression.split_whitespace().collect();
    if parts.len() != 5 { return false; }
    field_matches(parts[0], minute) && field_matches(parts[1], hour)
        && field_matches(parts[2], day) && field_matches(parts[3], month)
        && field_matches(parts[4], weekday)
}

fn field_matches(pattern: &str, value: u32) -> bool {
    if pattern == "*" { return true; }
    if let Some(slash_pos) = pattern.find('/') {
        let step: u32 = pattern[slash_pos + 1..].parse().unwrap_or(1);
        return value.is_multiple_of(step);
    }
    if let Some(dash_pos) = pattern.find('-') {
        let lo: u32 = pattern[..dash_pos].parse().unwrap_or(0);
        let hi: u32 = pattern[dash_pos + 1..].parse().unwrap_or(59);
        return value >= lo && value <= hi;
    }
    pattern.split(',').any(|p| p.trim().parse::<u32>() == Ok(value))
}

/// Scheduler — manages and runs scheduled tasks.
#[derive(Debug, Default)]
pub struct Scheduler {
    jobs: Vec<Box<dyn ScheduledJob>>,
}

impl Scheduler {
    pub fn new() -> Self { Scheduler { jobs: Vec::new() } }
    pub fn add(&mut self, job: impl ScheduledJob + 'static) { self.jobs.push(Box::new(job)); }

    /// Run all jobs whose schedule matches the current time.
    pub fn run_due(&self) -> Result<Vec<String>, String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let total_secs = now.as_secs();
        let minute = (total_secs / 60) % 60;
        let hour = (total_secs / 3600) % 24;
        let day = (total_secs / 86400) % 31 + 1;
        let month = 1; // simplified — full implementation would use chrono
        let weekday = (total_secs / 86400 + 4) % 7; // Thursday = 0

        let mut executed = Vec::new();
        for job in &self.jobs {
            if cron_matches(job.expression(), minute as u32, hour as u32, day as u32, month, weekday as u32) {
                job.handle().map_err(|e| format!("Job '{}' failed: {}", job.name(), e))?;
                executed.push(job.name().to_string());
            }
        }
        Ok(executed)
    }

    pub fn jobs(&self) -> &[Box<dyn ScheduledJob>] { &self.jobs }
}
