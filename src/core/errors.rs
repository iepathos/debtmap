//! Shared error types for the application

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for debtmap operations
#[derive(Debug, Error)]
pub enum Error {
    /// File system related errors
    #[error("File system error: {message}")]
    FileSystem {
        message: String,
        path: Option<PathBuf>,
        #[source]
        source: Option<std::io::Error>,
    },

    /// Parsing errors
    #[error("Parse error in {file}:{line}:{column}: {message}")]
    Parse {
        file: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },

    /// Analysis errors
    #[error("Analysis error: {0}")]
    Analysis(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Cache operation errors
    #[error("Cache error: {0}")]
    Cache(String),

    /// Unsupported feature errors
    #[error("Unsupported: {0}")]
    Unsupported(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Dependency resolution errors
    #[error("Dependency error: {0}")]
    Dependency(String),

    /// Concurrency errors
    #[error("Concurrency error: {0}")]
    Concurrency(String),

    /// Generic errors with context
    #[error("{context}: {message}")]
    WithContext { context: String, message: String },

    /// Wrapped external errors
    #[error(transparent)]
    External(#[from] anyhow::Error),

    /// IO errors
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// JSON errors
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Pattern errors
    #[error(transparent)]
    Pattern(#[from] glob::PatternError),
}

impl Error {
    /// Create a file system error with path context
    pub fn file_system(message: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::FileSystem {
            message: message.into(),
            path: Some(path.into()),
            source: None,
        }
    }

    /// Create a parse error with location
    pub fn parse(
        file: impl Into<PathBuf>,
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) -> Self {
        Self::Parse {
            file: file.into(),
            line,
            column,
            message: message.into(),
        }
    }

    /// Add context to an error
    pub fn with_context(self, context: impl Into<String>) -> Self {
        Self::WithContext {
            context: context.into(),
            message: self.to_string(),
        }
    }
}

/// Result type alias using our error type
pub type Result<T> = std::result::Result<T, Error>;

/// Extension trait for adding context to Results
pub trait ResultExt<T> {
    /// Add context to an error
    fn context(self, context: impl Into<String>) -> Result<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| e.with_context(context))
    }
}
