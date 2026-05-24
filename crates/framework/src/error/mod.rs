use std::fmt;

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: Option<std::path::PathBuf>,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.file {
            Some(path) => write!(f, "{}:{}:{}", path.display(), self.line, self.column),
            None => write!(f, "{}:{}", self.line, self.column),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum FrameworkError {
    #[error("{0}")]
    Internal(String),
}

impl From<String> for FrameworkError {
    fn from(s: String) -> Self { FrameworkError::Internal(s) }
}

impl From<&str> for FrameworkError {
    fn from(s: &str) -> Self { FrameworkError::Internal(s.to_string()) }
}

pub type Result<T> = std::result::Result<T, FrameworkError>;
