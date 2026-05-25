//! Query logging — listen to all database queries and their duration.
//!
//! Register a callback that receives every SQL query and execution time.
//! Useful for debugging, profiling, and monitoring.
//!
//! # Example
//!
//! ```rust,ignore
//! use viontin_framework::db::query_log::set_query_logger;
//!
//! set_query_logger(|sql, duration_ms| {
//!     println!("[query] {} ({:.1}ms)", sql, duration_ms);
//! });
//! ```

use std::sync::OnceLock;

type QueryLogger = Box<dyn Fn(&str, f64) + Send + Sync + 'static>;

static LOGGER: OnceLock<QueryLogger> = OnceLock::new();

/// Set the global query logger callback.
///
/// The callback receives the SQL string and execution duration in milliseconds.
pub fn set_query_logger(logger: impl Fn(&str, f64) + Send + Sync + 'static) {
    LOGGER.set(Box::new(logger)).unwrap_or_else(|_| panic!("Query logger already initialized"));
}

/// Called by database drivers after each query execution.
///
/// This is a public API for driver implementors. Application code should
/// use `set_query_logger` instead.
pub fn log_query(sql: &str, duration_ms: f64) {
    if let Some(logger) = LOGGER.get() {
        logger(sql, duration_ms);
    }
}

/// Wraps a query execution with timing and logging.
///
/// ```rust,ignore
/// use viontin_framework::db::query_log::timed_query;
///
/// let result = timed_query("SELECT * FROM users", || {
///     conn.query("SELECT * FROM users", &[])
/// });
/// ```
pub fn timed_query<T>(sql: &str, f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let start = std::time::Instant::now();
    let result = f();
    let duration = start.elapsed().as_secs_f64() * 1000.0;
    log_query(sql, duration);
    result
}
