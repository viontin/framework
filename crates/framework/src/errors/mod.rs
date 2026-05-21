//! Error handling — inspired by Laravel's error handling system.
//!
//! Provides HTTP error types, error reporting, and exception handler
//! infrastructure.

use std::fmt;
use crate::SourceLocation;

// ── HTTP Error Codes ──

/// HTTP error with status code and message.
#[derive(Debug, Clone)]
pub struct HttpError {
    pub status: u16,
    pub message: String,
    pub code: Option<String>,
}

impl HttpError {
    pub fn new(status: u16, message: impl Into<String>) -> Self {
        HttpError { status, message: message.into(), code: None }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    // Common HTTP errors
    pub fn not_found(message: impl Into<String>) -> Self {
        HttpError::new(404, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        HttpError::new(401, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        HttpError::new(403, message)
    }

    pub fn validation_error(_errors: Vec<String>) -> Self {
        HttpError {
            status: 422,
            message: "Validation failed".to_string(),
            code: Some("VALIDATION_ERROR".to_string()),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        HttpError::new(500, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        HttpError::new(400, message)
    }

    pub fn too_many_requests(message: impl Into<String>) -> Self {
        HttpError::new(429, message)
    }

    pub fn status_text(&self) -> &str {
        match self.status {
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            422 => "Unprocessable Entity",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Error",
        }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.status, self.status_text(), self.message)
    }
}

// ── Error Report ──

/// A structured error report for logging and debugging.
#[derive(Debug, Clone)]
pub struct ErrorReport {
    pub message: String,
    pub kind: String,
    pub stack: Vec<String>,
    pub location: Option<SourceLocation>,
    pub context: Vec<(String, String)>,
}

impl ErrorReport {
    pub fn new(message: impl Into<String>, kind: impl Into<String>) -> Self {
        ErrorReport {
            message: message.into(),
            kind: kind.into(),
            stack: Vec::new(),
            location: None,
            context: Vec::new(),
        }
    }

    pub fn with_location(mut self, loc: SourceLocation) -> Self {
        self.location = Some(loc);
        self
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.push((key.into(), value.into()));
        self
    }

    pub fn with_stack(mut self, stack: Vec<String>) -> Self {
        self.stack = stack;
        self
    }
}

impl fmt::Display for ErrorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[{}] {}", self.kind, self.message)?;
        if let Some(loc) = &self.location {
            writeln!(f, "  at {}", loc)?;
        }
        for (key, value) in &self.context {
            writeln!(f, "  {}: {}", key, value)?;
        }
        if !self.stack.is_empty() {
            writeln!(f, "  Stack:")?;
            for line in &self.stack {
                writeln!(f, "    {}", line)?;
            }
        }
        Ok(())
    }
}

// ── Exception Handler ──

/// Callback type for handling errors.
pub type ErrorHandlerFn = Box<dyn Fn(&ErrorReport) -> Result<(), String> + Send + Sync>;

/// Global exception handler registry.
static HANDLER: std::sync::OnceLock<ErrorHandlerFn> = std::sync::OnceLock::new();

/// Register a global error handler.
pub fn register_error_handler(handler: ErrorHandlerFn) -> Result<(), String> {
    HANDLER.set(handler)
        .map_err(|_| "Error handler already registered".to_string())
}

/// Report an error through the global handler (if registered).
pub fn report_error(report: &ErrorReport) {
    if let Some(handler) = HANDLER.get() {
        let _ = handler(report);
    }
}

/// Convert any `Result` into a reportable error.
pub fn report_result<T, E: fmt::Display>(result: Result<T, E>, kind: &str) -> Option<T> {
    match result {
        Ok(val) => Some(val),
        Err(e) => {
            let report = ErrorReport::new(e.to_string(), kind);
            report_error(&report);
            None
        }
    }
}
