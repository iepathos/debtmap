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

    /// Create a parse error with path context (no line number).
    pub fn parse_with_path(message: impl Into<String>, path: impl AsRef<std::path::Path>) -> Self {
        Self::ParseError {
            message: message.into(),
            path: Some(path.as_ref().to_path_buf()),
            line: None,
        }
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::ValidationError {
            message: message.into(),
        }
    }

    /// Create a validation error with path context.
    pub fn validation_with_path(
        message: impl Into<String>,
        path: impl AsRef<std::path::Path>,
    ) -> Self {
        Self::ValidationError {
            message: format!("{} (path: {})", message.into(), path.as_ref().display()),
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

    /// Check if this error is potentially transient and retryable.
    ///
    /// Retryable errors are those that might succeed on a subsequent attempt,
    /// such as:
    /// - File locks from concurrent access
    /// - Network timeouts
    /// - Resource temporarily unavailable
    /// - Git index lock contention
    ///
    /// Non-retryable errors include:
    /// - Syntax/parse errors
    /// - Configuration errors
    /// - Validation errors
    /// - File not found (permanent)
    ///
    /// # Example
    ///
    /// ```rust
    /// use debtmap::errors::AnalysisError;
    ///
    /// let io_err = AnalysisError::io("Resource busy");
    /// // Resource busy is typically retryable
    ///
    /// let parse_err = AnalysisError::parse("Syntax error at line 5");
    /// assert!(!parse_err.is_retryable()); // Parse errors are not retryable
    /// ```
    pub fn is_retryable(&self) -> bool {
        match self {
            // I/O errors that might be transient
            Self::IoError { message, .. } => {
                let msg_lower = message.to_lowercase();
                // Check for transient conditions
                msg_lower.contains("resource busy")
                    || msg_lower.contains("would block")
                    || msg_lower.contains("timed out")
                    || msg_lower.contains("timeout")
                    || msg_lower.contains("interrupted")
                    || msg_lower.contains("temporarily unavailable")
                    || msg_lower.contains("connection reset")
                    || msg_lower.contains("broken pipe")
                    || msg_lower.contains("try again")
            }
            // Coverage errors from external tools may be transient
            Self::CoverageError { message, .. } => {
                let msg_lower = message.to_lowercase();
                msg_lower.contains("connection")
                    || msg_lower.contains("timeout")
                    || msg_lower.contains("unavailable")
            }
            // Other errors - check for common transient patterns in message
            Self::Other(message) => {
                let msg_lower = message.to_lowercase();
                // Git lock contention
                msg_lower.contains("index.lock")
                    || msg_lower.contains("lock file")
                    || msg_lower.contains("unable to lock")
                    // Network issues
                    || msg_lower.contains("connection refused")
                    || msg_lower.contains("network unreachable")
            }
            // Parse, Validation, Config, and Analysis errors are never retryable
            Self::ParseError { .. }
            | Self::ValidationError { .. }
            | Self::ConfigError { .. }
            | Self::AnalysisFailure { .. } => false,
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

// Convert from std::convert::Infallible
// This is needed for pipeline stages that can never fail (pure functions)
impl From<std::convert::Infallible> for AnalysisError {
    fn from(infallible: std::convert::Infallible) -> Self {
        // This match is unreachable since Infallible can never be constructed,
        // but it's required by the type system
        match infallible {}
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

// ============================================================================
// CLI Error Reporting (Spec 197)
// ============================================================================

/// Print an error report to stderr with formatting.
///
/// This is the primary function for CLI error output. It formats errors
/// with numbering, colors (when available), and a helpful tip at the end.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::errors::{AnalysisError, print_error_report};
///
/// let errors = vec![
///     AnalysisError::config("Invalid threshold: -5"),
///     AnalysisError::parse_with_context("Unexpected token", "src/bad.rs", 12),
/// ];
///
/// print_error_report(&errors);
/// // Output:
/// // Error: 2 issues found:
/// //
/// //   1. Config error: Invalid threshold: -5
/// //   2. Parse error: Unexpected token in src/bad.rs at line 12
/// //
/// // Tip: Fix the issues above and run again.
/// ```
pub fn print_error_report(errors: &[AnalysisError]) {
    if errors.is_empty() {
        return;
    }

    let issue_count = if errors.len() == 1 {
        "1 issue".to_string()
    } else {
        format!("{} issues", errors.len())
    };

    eprintln!("\nError: {} found:\n", issue_count);

    for (i, error) in errors.iter().enumerate() {
        eprintln!("  {}. {}", i + 1, error);
    }

    eprintln!("\nTip: Fix the issues above and run again.");
}

/// Print an error report with a custom title.
///
/// Similar to `print_error_report` but allows customizing the title text.
pub fn print_error_report_titled(title: &str, errors: &[AnalysisError]) {
    if errors.is_empty() {
        return;
    }

    let issue_count = if errors.len() == 1 {
        "1 issue".to_string()
    } else {
        format!("{} issues", errors.len())
    };

    eprintln!("\n{}: {} found:\n", title, issue_count);

    for (i, error) in errors.iter().enumerate() {
        eprintln!("  {}. {}", i + 1, error);
    }

    eprintln!("\nTip: Fix the issues above and run again.");
}

/// Format error report for return as string (useful for testing).
pub fn format_error_report(errors: &[AnalysisError]) -> String {
    if errors.is_empty() {
        return String::new();
    }

    let issue_count = if errors.len() == 1 {
        "1 issue".to_string()
    } else {
        format!("{} issues", errors.len())
    };

    let mut output = format!("Error: {} found:\n\n", issue_count);
    output.push_str(&format_error_list(errors));
    output.push_str("\n\nTip: Fix the issues above and run again.");

    output
}

/// Group errors by category for structured reporting.
///
/// Returns a map of error category to list of errors in that category.
pub fn group_errors_by_category(
    errors: &[AnalysisError],
) -> std::collections::HashMap<&'static str, Vec<&AnalysisError>> {
    let mut groups: std::collections::HashMap<&'static str, Vec<&AnalysisError>> =
        std::collections::HashMap::new();

    for error in errors {
        groups.entry(error.category()).or_default().push(error);
    }

    groups
}

/// Print a grouped error report organized by category.
///
/// This provides better organization for large error sets by grouping
/// errors into categories.
pub fn print_grouped_error_report(errors: &[AnalysisError]) {
    if errors.is_empty() {
        return;
    }

    let groups = group_errors_by_category(errors);
    let issue_count = if errors.len() == 1 {
        "1 issue".to_string()
    } else {
        format!("{} issues", errors.len())
    };

    eprintln!("\nError: {} found:\n", issue_count);

    for (category, category_errors) in groups {
        eprintln!("[{}] {} error(s):", category, category_errors.len());
        for error in category_errors {
            eprintln!("  - {}", error.message());
            if let Some(path) = error.path() {
                eprintln!("    at {}", path.display());
            }
        }
        eprintln!();
    }

    eprintln!("Tip: Fix the issues above and run again.");
}

/// Error summary for structured output.
#[derive(Debug, Clone)]
pub struct ErrorSummary {
    /// Total number of errors
    pub total_count: usize,
    /// Count by category
    pub by_category: std::collections::HashMap<String, usize>,
    /// Unique file paths mentioned in errors
    pub affected_files: Vec<PathBuf>,
    /// The original errors
    pub errors: Vec<AnalysisError>,
}

impl ErrorSummary {
    /// Create an error summary from a list of errors.
    pub fn from_errors(errors: Vec<AnalysisError>) -> Self {
        let mut by_category: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut affected_files: std::collections::HashSet<PathBuf> =
            std::collections::HashSet::new();

        for error in &errors {
            *by_category.entry(error.category().to_string()).or_insert(0) += 1;
            if let Some(path) = error.path() {
                affected_files.insert(path.clone());
            }
        }

        Self {
            total_count: errors.len(),
            by_category,
            affected_files: affected_files.into_iter().collect(),
            errors,
        }
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.total_count > 0
    }

    /// Format as a one-line summary.
    pub fn one_line_summary(&self) -> String {
        if self.total_count == 0 {
            "No errors".to_string()
        } else {
            let file_count = self.affected_files.len();
            format!("{} error(s) in {} file(s)", self.total_count, file_count)
        }
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

    // Tests for error reporting utilities (Spec 197)

    #[test]
    fn test_format_error_report_single() {
        let errors = vec![AnalysisError::config("Invalid threshold")];
        let report = format_error_report(&errors);

        assert!(report.contains("1 issue"));
        assert!(report.contains("Invalid threshold"));
        assert!(report.contains("Tip:"));
    }

    #[test]
    fn test_format_error_report_multiple() {
        let errors = vec![
            AnalysisError::config("Error 1"),
            AnalysisError::parse("Error 2"),
            AnalysisError::io("Error 3"),
        ];
        let report = format_error_report(&errors);

        assert!(report.contains("3 issues"));
        assert!(report.contains("1."));
        assert!(report.contains("2."));
        assert!(report.contains("3."));
    }

    #[test]
    fn test_format_error_report_empty() {
        let errors: Vec<AnalysisError> = vec![];
        let report = format_error_report(&errors);
        assert!(report.is_empty());
    }

    #[test]
    fn test_group_errors_by_category() {
        let errors = vec![
            AnalysisError::config("Config 1"),
            AnalysisError::config("Config 2"),
            AnalysisError::parse("Parse 1"),
            AnalysisError::io("IO 1"),
        ];

        let groups = group_errors_by_category(&errors);

        assert_eq!(groups.get("Config").map(|v| v.len()), Some(2));
        assert_eq!(groups.get("Parse").map(|v| v.len()), Some(1));
        assert_eq!(groups.get("I/O").map(|v| v.len()), Some(1));
    }

    #[test]
    fn test_error_summary_from_errors() {
        let errors = vec![
            AnalysisError::io_with_path("Not found", "/path/to/file1.rs"),
            AnalysisError::io_with_path("Permission denied", "/path/to/file2.rs"),
            AnalysisError::config("Invalid threshold"),
        ];

        let summary = ErrorSummary::from_errors(errors);

        assert_eq!(summary.total_count, 3);
        assert_eq!(summary.by_category.get("I/O"), Some(&2));
        assert_eq!(summary.by_category.get("Config"), Some(&1));
        assert_eq!(summary.affected_files.len(), 2);
        assert!(summary.has_errors());
    }

    #[test]
    fn test_error_summary_one_line() {
        let errors = vec![
            AnalysisError::io_with_path("Error", "/file1.rs"),
            AnalysisError::io_with_path("Error", "/file2.rs"),
        ];

        let summary = ErrorSummary::from_errors(errors);
        let one_line = summary.one_line_summary();

        assert!(one_line.contains("2 error(s)"));
        assert!(one_line.contains("2 file(s)"));
    }

    #[test]
    fn test_error_summary_empty() {
        let summary = ErrorSummary::from_errors(vec![]);

        assert!(!summary.has_errors());
        assert_eq!(summary.one_line_summary(), "No errors");
    }

    // Tests for is_retryable (Spec 205)

    #[test]
    fn test_is_retryable_io_resource_busy() {
        let err = AnalysisError::io("Resource busy - file locked");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_io_timeout() {
        let err = AnalysisError::io("Operation timed out");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_io_interrupted() {
        let err = AnalysisError::io("System call interrupted");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_io_would_block() {
        let err = AnalysisError::io("Would block on non-blocking read");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_io_not_found() {
        // File not found is NOT retryable - it won't suddenly appear
        let err = AnalysisError::io("No such file or directory");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_io_permission_denied() {
        // Permission denied is NOT retryable
        let err = AnalysisError::io("Permission denied");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_parse_error() {
        let err = AnalysisError::parse("Syntax error at line 5");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_validation_error() {
        let err = AnalysisError::validation("Invalid configuration value");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_config_error() {
        let err = AnalysisError::config("Missing required field");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_analysis_error() {
        let err = AnalysisError::analysis("Algorithm failed");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_coverage_timeout() {
        let err = AnalysisError::coverage("Connection timeout to coverage server");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_coverage_parse_fail() {
        let err = AnalysisError::coverage("Invalid coverage format");
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_git_lock() {
        let err = AnalysisError::other("Unable to create index.lock: File exists");
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_other_not_retryable() {
        let err = AnalysisError::other("Unknown error occurred");
        assert!(!err.is_retryable());
    }
}
