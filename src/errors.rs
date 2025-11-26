//! Unified error types for debtmap analysis operations.
//!
//! This module provides error types that bridge between anyhow's dynamic errors
//! and stillwater's structured effect system. The design allows gradual migration
//! from anyhow to effect-based error handling while maintaining backwards compatibility.
//!
//! # Design Philosophy
//!
//! - **Backwards Compatible**: All existing code using `anyhow::Result` continues to work
//! - **Structured Errors**: Categorized errors enable better error handling and reporting
//! - **Accumulation Support**: Error types support stillwater's Validation pattern for
//!   collecting ALL errors instead of failing at the first one
//!
//! # Example
//!
//! ```rust
//! use debtmap::errors::AnalysisError;
//!
//! // Create typed errors
//! let io_err = AnalysisError::io("Failed to read file");
//! let parse_err = AnalysisError::parse("Invalid syntax at line 42");
//!
//! // Convert to/from anyhow for backwards compatibility
//! let anyhow_err: anyhow::Error = io_err.clone().into();
//! let back_to_analysis: AnalysisError = anyhow_err.into();
//! ```

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Unified error type for debtmap analysis operations.
///
/// This enum categorizes errors into meaningful types that help with:
/// - Better error messages for users
/// - Programmatic error handling
/// - Error aggregation with stillwater's Validation
///
/// # Categories
///
/// - `IoError`: File system operations (read, write, permissions)
/// - `ParseError`: Source code parsing failures
/// - `ValidationError`: Configuration or input validation failures
/// - `ConfigError`: Configuration file issues
/// - `CoverageError`: Coverage data processing errors
/// - `AnalysisError`: Core analysis algorithm errors
/// - `Other`: Catch-all for unexpected errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisError {
    /// File system I/O errors (read, write, permissions, etc.)
    IoError {
        message: String,
        path: Option<PathBuf>,
    },
    /// Source code parsing errors
    ParseError {
        message: String,
        path: Option<PathBuf>,
        line: Option<usize>,
    },
    /// Validation errors (configuration, input constraints)
    ValidationError { message: String },
    /// Configuration file errors
    ConfigError {
        message: String,
        path: Option<PathBuf>,
    },
    /// Coverage data processing errors
    CoverageError {
        message: String,
        path: Option<PathBuf>,
    },
    /// Core analysis algorithm errors
    AnalysisFailure { message: String },
    /// Catch-all for other errors
    Other(String),
}

impl AnalysisError {
    /// Create an I/O error with a message.
    pub fn io(message: impl Into<String>) -> Self {
        Self::IoError {
            message: message.into(),
            path: None,
        }
    }

    /// Create an I/O error with a message and path context.
    pub fn io_with_path(message: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::IoError {
            message: message.into(),
            path: Some(path.into()),
        }
    }

    /// Create a parse error with a message.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
            path: None,
            line: None,
        }
    }

    /// Create a parse error with full context.
    pub fn parse_with_context(
        message: impl Into<String>,
        path: impl Into<PathBuf>,
        line: usize,
    ) -> Self {
        Self::ParseError {
            message: message.into(),
            path: Some(path.into()),
            line: Some(line),
        }
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::ValidationError {
            message: message.into(),
        }
    }

    /// Create a configuration error.
    pub fn config(message: impl Into<String>) -> Self {
        Self::ConfigError {
            message: message.into(),
            path: None,
        }
    }

    /// Create a configuration error with path context.
    pub fn config_with_path(message: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::ConfigError {
            message: message.into(),
            path: Some(path.into()),
        }
    }

    /// Create a coverage processing error.
    pub fn coverage(message: impl Into<String>) -> Self {
        Self::CoverageError {
            message: message.into(),
            path: None,
        }
    }

    /// Create a coverage error with path context.
    pub fn coverage_with_path(message: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::CoverageError {
            message: message.into(),
            path: Some(path.into()),
        }
    }

    /// Create an analysis failure error.
    pub fn analysis(message: impl Into<String>) -> Self {
        Self::AnalysisFailure {
            message: message.into(),
        }
    }

    /// Create an error from any message.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }

    /// Get the error message without context details.
    pub fn message(&self) -> &str {
        match self {
            Self::IoError { message, .. } => message,
            Self::ParseError { message, .. } => message,
            Self::ValidationError { message } => message,
            Self::ConfigError { message, .. } => message,
            Self::CoverageError { message, .. } => message,
            Self::AnalysisFailure { message } => message,
            Self::Other(message) => message,
        }
    }

    /// Get the associated path, if any.
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::IoError { path, .. } => path.as_ref(),
            Self::ParseError { path, .. } => path.as_ref(),
            Self::ConfigError { path, .. } => path.as_ref(),
            Self::CoverageError { path, .. } => path.as_ref(),
            _ => None,
        }
    }

    /// Get the error category name.
    pub fn category(&self) -> &'static str {
        match self {
            Self::IoError { .. } => "I/O",
            Self::ParseError { .. } => "Parse",
            Self::ValidationError { .. } => "Validation",
            Self::ConfigError { .. } => "Config",
            Self::CoverageError { .. } => "Coverage",
            Self::AnalysisFailure { .. } => "Analysis",
            Self::Other(_) => "Error",
        }
    }
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError { message, path } => {
                write!(f, "I/O error: {}", message)?;
                if let Some(p) = path {
                    write!(f, " (path: {})", p.display())?;
                }
                Ok(())
            }
            Self::ParseError {
                message,
                path,
                line,
            } => {
                write!(f, "Parse error: {}", message)?;
                if let Some(p) = path {
                    write!(f, " in {}", p.display())?;
                }
                if let Some(l) = line {
                    write!(f, " at line {}", l)?;
                }
                Ok(())
            }
            Self::ValidationError { message } => {
                write!(f, "Validation error: {}", message)
            }
            Self::ConfigError { message, path } => {
                write!(f, "Config error: {}", message)?;
                if let Some(p) = path {
                    write!(f, " (file: {})", p.display())?;
                }
                Ok(())
            }
            Self::CoverageError { message, path } => {
                write!(f, "Coverage error: {}", message)?;
                if let Some(p) = path {
                    write!(f, " (file: {})", p.display())?;
                }
                Ok(())
            }
            Self::AnalysisFailure { message } => {
                write!(f, "Analysis error: {}", message)
            }
            Self::Other(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for AnalysisError {}

// Backwards compatibility: Convert from anyhow::Error
impl From<anyhow::Error> for AnalysisError {
    fn from(err: anyhow::Error) -> Self {
        // Try to preserve error type information from the chain
        let error_string = err.to_string();

        // Attempt to categorize based on common patterns
        if error_string.contains("I/O error") || error_string.contains("No such file") {
            Self::io(error_string)
        } else if error_string.contains("Parse error") || error_string.contains("syntax") {
            Self::parse(error_string)
        } else if error_string.contains("Config") || error_string.contains("configuration") {
            Self::config(error_string)
        } else if error_string.contains("Coverage") || error_string.contains("coverage") {
            Self::coverage(error_string)
        } else if error_string.contains("Validation") || error_string.contains("invalid") {
            Self::validation(error_string)
        } else {
            Self::Other(error_string)
        }
    }
}

// Note: We can't impl From<AnalysisError> for anyhow::Error because anyhow has a
// blanket impl From<E: StdError> for anyhow::Error, and AnalysisError implements StdError.
// Instead, use .into() directly or the into_anyhow() method below.

impl AnalysisError {
    /// Convert this error to an anyhow::Error.
    ///
    /// This is a convenience method for backwards compatibility with anyhow-based APIs.
    pub fn into_anyhow(self) -> anyhow::Error {
        anyhow::Error::from(self)
    }
}

// Convert from std::io::Error
impl From<io::Error> for AnalysisError {
    fn from(err: io::Error) -> Self {
        Self::io(err.to_string())
    }
}

/// Format a list of errors for display.
///
/// This is useful for displaying accumulated validation errors.
///
/// # Example
///
/// ```rust
/// use debtmap::errors::{AnalysisError, format_error_list};
///
/// let errors = vec![
///     AnalysisError::io("File not found"),
///     AnalysisError::parse("Invalid syntax"),
/// ];
/// let formatted = format_error_list(&errors);
/// assert!(formatted.contains("1. I/O error"));
/// assert!(formatted.contains("2. Parse error"));
/// ```
pub fn format_error_list(errors: &[AnalysisError]) -> String {
    errors
        .iter()
        .enumerate()
        .map(|(i, e)| format!("  {}. {}", i + 1, e))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert a list of AnalysisErrors to an anyhow::Error with formatted message.
///
/// This is useful when you need to convert accumulated validation errors
/// back to anyhow for backwards compatibility with existing code.
pub fn errors_to_anyhow(errors: Vec<AnalysisError>) -> anyhow::Error {
    if errors.is_empty() {
        anyhow::anyhow!("Unknown error (no errors provided)")
    } else if errors.len() == 1 {
        errors.into_iter().next().unwrap().into()
    } else {
        anyhow::anyhow!("Multiple errors occurred:\n{}", format_error_list(&errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_creation() {
        let err = AnalysisError::io("File not found");
        assert_eq!(err.message(), "File not found");
        assert_eq!(err.category(), "I/O");
        assert!(err.path().is_none());
    }

    #[test]
    fn test_io_error_with_path() {
        let err = AnalysisError::io_with_path("Permission denied", "/etc/passwd");
        assert_eq!(err.message(), "Permission denied");
        assert_eq!(err.path().unwrap(), &PathBuf::from("/etc/passwd"));
    }

    #[test]
    fn test_parse_error_with_context() {
        let err = AnalysisError::parse_with_context("Unexpected token", "src/main.rs", 42);
        assert!(err.to_string().contains("src/main.rs"));
        assert!(err.to_string().contains("line 42"));
    }

    #[test]
    fn test_validation_error() {
        let err = AnalysisError::validation("Weights must sum to 1.0");
        assert_eq!(err.category(), "Validation");
        assert!(err.to_string().contains("Weights must sum"));
    }

    #[test]
    fn test_anyhow_roundtrip() {
        let original = AnalysisError::io("Test error");
        let anyhow_err: anyhow::Error = original.clone().into();
        let back: AnalysisError = anyhow_err.into();

        // The error message should be preserved
        assert!(back.to_string().contains("Test error"));
    }

    #[test]
    fn test_format_error_list() {
        let errors = vec![
            AnalysisError::io("File not found"),
            AnalysisError::parse("Invalid syntax"),
            AnalysisError::validation("Invalid config"),
        ];
        let formatted = format_error_list(&errors);
        assert!(formatted.contains("1."));
        assert!(formatted.contains("2."));
        assert!(formatted.contains("3."));
    }

    #[test]
    fn test_errors_to_anyhow_single() {
        let errors = vec![AnalysisError::io("Single error")];
        let result = errors_to_anyhow(errors);
        assert!(result.to_string().contains("Single error"));
    }

    #[test]
    fn test_errors_to_anyhow_multiple() {
        let errors = vec![
            AnalysisError::io("Error 1"),
            AnalysisError::parse("Error 2"),
        ];
        let result = errors_to_anyhow(errors);
        let msg = result.to_string();
        assert!(msg.contains("Multiple errors"));
        assert!(msg.contains("Error 1"));
        assert!(msg.contains("Error 2"));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file.txt not found");
        let analysis_err: AnalysisError = io_err.into();
        assert_eq!(analysis_err.category(), "I/O");
    }
}
