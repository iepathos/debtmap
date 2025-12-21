//! Unified error type for debtmap operations.
//!
//! This module consolidates the three error hierarchies in debtmap:
//! - Domain errors (`src/error.rs`): CliError, ConfigError, AnalysisError, AppError
//! - Unified errors (`src/errors/mod.rs`): AnalysisError enum
//! - Core errors (`src/core/errors.rs`): Error enum
//!
//! The `DebtmapError` type provides:
//! - Clear categorization via variants (Io, Parse, Config, Analysis, Cli)
//! - Structured error codes for programmatic handling (e.g., E001, E010)
//! - Error classification methods (is_retryable, is_user_fixable)
//! - Context trails for debugging
//! - Serde serialization for structured logging
//!
//! # Error Codes
//!
//! Error codes are assigned by category:
//! - E001-E009: I/O and filesystem errors
//! - E010-E019: Parse errors
//! - E020-E029: Configuration errors
//! - E030-E039: Analysis errors
//! - E040-E049: CLI errors
//! - E050-E059: Validation errors
//!
//! # Migration
//!
//! This module provides `From` implementations for gradual migration from
//! the old error types. Use `DebtmapError` for new code; existing code
//! using old error types will continue to work.
//!
//! # Example
//!
//! ```rust
//! use debtmap::debtmap_error::{DebtmapError, ErrorCode};
//!
//! // Create typed errors
//! let io_err = DebtmapError::io("File not found", Some("/path/to/file".into()));
//! let parse_err = DebtmapError::parse("Invalid syntax", "/path/to/file", Some(42), None);
//!
//! // Check error classification
//! assert!(!io_err.is_user_fixable());
//! assert!(parse_err.is_user_fixable()); // User can fix syntax errors
//!
//! // Get error code
//! println!("Error code: {}", io_err.code());
//! ```

use crate::observability::AnalysisPhase;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;

/// Structured error code for documentation and programmatic handling.
///
/// Error codes follow a category-based scheme:
/// - E001-E009: I/O and filesystem errors
/// - E010-E019: Parse errors
/// - E020-E029: Configuration errors
/// - E030-E039: Analysis errors
/// - E040-E049: CLI errors
/// - E050-E059: Validation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct ErrorCode(&'static str);

impl ErrorCode {
    /// I/O error - file not found
    pub const IO_FILE_NOT_FOUND: ErrorCode = ErrorCode("E001");
    /// I/O error - permission denied
    pub const IO_PERMISSION_DENIED: ErrorCode = ErrorCode("E002");
    /// I/O error - resource busy
    pub const IO_RESOURCE_BUSY: ErrorCode = ErrorCode("E003");
    /// I/O error - generic
    pub const IO_GENERIC: ErrorCode = ErrorCode("E009");

    /// Parse error - syntax error
    pub const PARSE_SYNTAX: ErrorCode = ErrorCode("E010");
    /// Parse error - unsupported language
    pub const PARSE_UNSUPPORTED: ErrorCode = ErrorCode("E011");
    /// Parse error - invalid encoding
    pub const PARSE_ENCODING: ErrorCode = ErrorCode("E012");
    /// Parse error - generic
    pub const PARSE_GENERIC: ErrorCode = ErrorCode("E019");

    /// Config error - invalid value
    pub const CONFIG_INVALID: ErrorCode = ErrorCode("E020");
    /// Config error - missing required field
    pub const CONFIG_MISSING: ErrorCode = ErrorCode("E021");
    /// Config error - file not found
    pub const CONFIG_FILE_NOT_FOUND: ErrorCode = ErrorCode("E022");
    /// Config error - generic
    pub const CONFIG_GENERIC: ErrorCode = ErrorCode("E029");

    /// Analysis error - complexity calculation failed
    pub const ANALYSIS_COMPLEXITY: ErrorCode = ErrorCode("E030");
    /// Analysis error - coverage loading failed
    pub const ANALYSIS_COVERAGE: ErrorCode = ErrorCode("E031");
    /// Analysis error - debt scoring failed
    pub const ANALYSIS_SCORING: ErrorCode = ErrorCode("E032");
    /// Analysis error - generic
    pub const ANALYSIS_GENERIC: ErrorCode = ErrorCode("E039");

    /// CLI error - invalid command
    pub const CLI_INVALID_COMMAND: ErrorCode = ErrorCode("E040");
    /// CLI error - missing argument
    pub const CLI_MISSING_ARG: ErrorCode = ErrorCode("E041");
    /// CLI error - invalid argument
    pub const CLI_INVALID_ARG: ErrorCode = ErrorCode("E042");
    /// CLI error - generic
    pub const CLI_GENERIC: ErrorCode = ErrorCode("E049");

    /// Validation error - generic
    pub const VALIDATION_GENERIC: ErrorCode = ErrorCode("E050");
    /// Validation error - threshold exceeded
    pub const VALIDATION_THRESHOLD: ErrorCode = ErrorCode("E051");
    /// Validation error - constraint violated
    pub const VALIDATION_CONSTRAINT: ErrorCode = ErrorCode("E052");

    /// Get the error code string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unified error type for debtmap operations.
///
/// This enum consolidates all error types across the codebase into a single,
/// well-structured type with error codes, classification methods, and context.
#[derive(Debug, Clone)]
pub enum DebtmapError {
    /// I/O and filesystem errors.
    Io {
        /// Error code for documentation lookup.
        code: ErrorCode,
        /// Human-readable error message.
        message: String,
        /// Associated file path, if any.
        path: Option<PathBuf>,
        /// Source error for debugging.
        source: Option<Arc<std::io::Error>>,
    },

    /// Source code parsing errors.
    Parse {
        /// Error code for documentation lookup.
        code: ErrorCode,
        /// Human-readable error message.
        message: String,
        /// File being parsed.
        path: PathBuf,
        /// Line number where error occurred.
        line: Option<usize>,
        /// Column number where error occurred.
        column: Option<usize>,
    },

    /// Configuration errors.
    Config {
        /// Error code for documentation lookup.
        code: ErrorCode,
        /// Human-readable error message.
        message: String,
        /// Configuration field name, if applicable.
        field: Option<String>,
        /// Configuration file path, if applicable.
        path: Option<PathBuf>,
    },

    /// Analysis execution errors.
    Analysis {
        /// Error code for documentation lookup.
        code: ErrorCode,
        /// Human-readable error message.
        message: String,
        /// Analysis phase where error occurred.
        phase: Option<AnalysisPhase>,
    },

    /// CLI argument errors.
    Cli {
        /// Error code for documentation lookup.
        code: ErrorCode,
        /// Human-readable error message.
        message: String,
        /// Argument name, if applicable.
        arg: Option<String>,
    },

    /// Validation errors (may contain multiple issues).
    Validation {
        /// Error code for documentation lookup.
        code: ErrorCode,
        /// Number of validation errors.
        count: usize,
        /// Individual error messages.
        errors: Vec<String>,
    },
}

impl DebtmapError {
    // ==========================================================================
    // Constructor Methods
    // ==========================================================================

    /// Create an I/O error with a message and optional path.
    #[must_use]
    pub fn io(message: impl Into<String>, path: Option<PathBuf>) -> Self {
        Self::Io {
            code: ErrorCode::IO_GENERIC,
            message: message.into(),
            path,
            source: None,
        }
    }

    /// Create an I/O error from a std::io::Error.
    #[must_use]
    pub fn from_io_error(err: std::io::Error, path: Option<PathBuf>) -> Self {
        let code = match err.kind() {
            std::io::ErrorKind::NotFound => ErrorCode::IO_FILE_NOT_FOUND,
            std::io::ErrorKind::PermissionDenied => ErrorCode::IO_PERMISSION_DENIED,
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                ErrorCode::IO_RESOURCE_BUSY
            }
            _ => ErrorCode::IO_GENERIC,
        };
        Self::Io {
            code,
            message: err.to_string(),
            path,
            source: Some(Arc::new(err)),
        }
    }

    /// Create a parse error with context.
    #[must_use]
    pub fn parse(
        message: impl Into<String>,
        path: impl Into<PathBuf>,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Self {
        Self::Parse {
            code: ErrorCode::PARSE_GENERIC,
            message: message.into(),
            path: path.into(),
            line,
            column,
        }
    }

    /// Create a parse error with syntax error code.
    #[must_use]
    pub fn parse_syntax(
        message: impl Into<String>,
        path: impl Into<PathBuf>,
        line: Option<usize>,
        column: Option<usize>,
    ) -> Self {
        Self::Parse {
            code: ErrorCode::PARSE_SYNTAX,
            message: message.into(),
            path: path.into(),
            line,
            column,
        }
    }

    /// Create a configuration error.
    #[must_use]
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            code: ErrorCode::CONFIG_GENERIC,
            message: message.into(),
            field: None,
            path: None,
        }
    }

    /// Create a configuration error with field context.
    #[must_use]
    pub fn config_with_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        Self::Config {
            code: ErrorCode::CONFIG_INVALID,
            message: message.into(),
            field: Some(field.into()),
            path: None,
        }
    }

    /// Create a configuration error with path context.
    #[must_use]
    pub fn config_with_path(message: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::Config {
            code: ErrorCode::CONFIG_FILE_NOT_FOUND,
            message: message.into(),
            field: None,
            path: Some(path.into()),
        }
    }

    /// Create an analysis error.
    #[must_use]
    pub fn analysis(message: impl Into<String>) -> Self {
        Self::Analysis {
            code: ErrorCode::ANALYSIS_GENERIC,
            message: message.into(),
            phase: None,
        }
    }

    /// Create an analysis error with phase context.
    #[must_use]
    pub fn analysis_with_phase(message: impl Into<String>, phase: AnalysisPhase) -> Self {
        let code = match phase {
            AnalysisPhase::CoverageLoading => ErrorCode::ANALYSIS_COVERAGE,
            AnalysisPhase::DebtScoring => ErrorCode::ANALYSIS_SCORING,
            _ => ErrorCode::ANALYSIS_GENERIC,
        };
        Self::Analysis {
            code,
            message: message.into(),
            phase: Some(phase),
        }
    }

    /// Create a CLI error.
    #[must_use]
    pub fn cli(message: impl Into<String>) -> Self {
        Self::Cli {
            code: ErrorCode::CLI_GENERIC,
            message: message.into(),
            arg: None,
        }
    }

    /// Create a CLI error for invalid command.
    #[must_use]
    pub fn cli_invalid_command(message: impl Into<String>) -> Self {
        Self::Cli {
            code: ErrorCode::CLI_INVALID_COMMAND,
            message: message.into(),
            arg: None,
        }
    }

    /// Create a CLI error for missing argument.
    #[must_use]
    pub fn cli_missing_arg(arg: impl Into<String>) -> Self {
        let arg_str = arg.into();
        Self::Cli {
            code: ErrorCode::CLI_MISSING_ARG,
            message: format!("Missing required argument: {}", arg_str),
            arg: Some(arg_str),
        }
    }

    /// Create a CLI error for invalid argument.
    #[must_use]
    pub fn cli_invalid_arg(arg: impl Into<String>, reason: impl Into<String>) -> Self {
        let arg_str = arg.into();
        Self::Cli {
            code: ErrorCode::CLI_INVALID_ARG,
            message: format!("Invalid argument '{}': {}", arg_str, reason.into()),
            arg: Some(arg_str),
        }
    }

    /// Create a validation error with a single message.
    #[must_use]
    pub fn validation(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self::Validation {
            code: ErrorCode::VALIDATION_GENERIC,
            count: 1,
            errors: vec![msg],
        }
    }

    /// Create a validation error with multiple messages.
    #[must_use]
    pub fn validations(errors: Vec<String>) -> Self {
        Self::Validation {
            code: ErrorCode::VALIDATION_GENERIC,
            count: errors.len(),
            errors,
        }
    }

    // ==========================================================================
    // Accessor Methods
    // ==========================================================================

    /// Get the error code.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Io { code, .. } => *code,
            Self::Parse { code, .. } => *code,
            Self::Config { code, .. } => *code,
            Self::Analysis { code, .. } => *code,
            Self::Cli { code, .. } => *code,
            Self::Validation { code, .. } => *code,
        }
    }

    /// Get the error category name.
    #[must_use]
    pub fn category(&self) -> &'static str {
        match self {
            Self::Io { .. } => "I/O",
            Self::Parse { .. } => "Parse",
            Self::Config { .. } => "Config",
            Self::Analysis { .. } => "Analysis",
            Self::Cli { .. } => "CLI",
            Self::Validation { .. } => "Validation",
        }
    }

    /// Get the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Io { message, .. } => message,
            Self::Parse { message, .. } => message,
            Self::Config { message, .. } => message,
            Self::Analysis { message, .. } => message,
            Self::Cli { message, .. } => message,
            Self::Validation { errors, .. } => errors.first().map_or("Validation failed", |s| s),
        }
    }

    /// Get the associated path, if any.
    #[must_use]
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            Self::Io { path, .. } => path.as_ref(),
            Self::Parse { path, .. } => Some(path),
            Self::Config { path, .. } => path.as_ref(),
            _ => None,
        }
    }

    // ==========================================================================
    // Classification Methods
    // ==========================================================================

    /// Check if this error is potentially transient and retryable.
    ///
    /// Retryable errors are those that might succeed on a subsequent attempt:
    /// - Resource busy / file locks
    /// - Network timeouts
    /// - Coverage loading (external tool issues)
    ///
    /// Non-retryable errors include:
    /// - Parse/syntax errors
    /// - Configuration errors
    /// - Validation errors
    /// - File not found (permanent)
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Io { code, message, .. } => {
                // Resource busy is retryable
                if *code == ErrorCode::IO_RESOURCE_BUSY {
                    return true;
                }
                // Check message for transient patterns
                let msg_lower = message.to_lowercase();
                msg_lower.contains("resource busy")
                    || msg_lower.contains("would block")
                    || msg_lower.contains("timed out")
                    || msg_lower.contains("timeout")
                    || msg_lower.contains("interrupted")
                    || msg_lower.contains("temporarily unavailable")
                    || msg_lower.contains("connection reset")
            }
            Self::Analysis { phase, message, .. } => {
                // Coverage loading errors may be transient
                if *phase == Some(AnalysisPhase::CoverageLoading) {
                    let msg_lower = message.to_lowercase();
                    return msg_lower.contains("connection")
                        || msg_lower.contains("timeout")
                        || msg_lower.contains("unavailable");
                }
                false
            }
            // Parse, Config, CLI, Validation errors are never retryable
            Self::Parse { .. }
            | Self::Config { .. }
            | Self::Cli { .. }
            | Self::Validation { .. } => false,
        }
    }

    /// Check if this error is something the user can fix.
    ///
    /// User-fixable errors include:
    /// - Configuration errors (fix config file)
    /// - CLI errors (fix command arguments)
    /// - Validation errors (fix input)
    /// - Parse errors (fix source code)
    ///
    /// Non-user-fixable errors include:
    /// - I/O errors (system issues)
    /// - Analysis errors (internal algorithm issues)
    #[must_use]
    pub fn is_user_fixable(&self) -> bool {
        matches!(
            self,
            Self::Config { .. } | Self::Cli { .. } | Self::Validation { .. } | Self::Parse { .. }
        )
    }

    /// Get the suggested exit code for this error.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Cli { .. } => 2,        // Invalid usage
            Self::Config { .. } => 3,     // Configuration error
            Self::Validation { .. } => 4, // Validation error
            Self::Parse { .. } => 5,      // Parse error
            Self::Analysis { .. } => 1,   // Analysis failed
            Self::Io { .. } => 1,         // I/O error
        }
    }
}

impl std::fmt::Display for DebtmapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io {
                code,
                message,
                path,
                ..
            } => {
                write!(f, "[{}] I/O error: {}", code, message)?;
                if let Some(p) = path {
                    write!(f, " (path: {})", p.display())?;
                }
                Ok(())
            }
            Self::Parse {
                code,
                message,
                path,
                line,
                column,
            } => {
                write!(
                    f,
                    "[{}] Parse error in {}: {}",
                    code,
                    path.display(),
                    message
                )?;
                if let Some(l) = line {
                    write!(f, " at line {}", l)?;
                    if let Some(c) = column {
                        write!(f, ", column {}", c)?;
                    }
                }
                Ok(())
            }
            Self::Config {
                code,
                message,
                field,
                path,
            } => {
                write!(f, "[{}] Configuration error: {}", code, message)?;
                if let Some(fld) = field {
                    write!(f, " (field: {})", fld)?;
                }
                if let Some(p) = path {
                    write!(f, " (file: {})", p.display())?;
                }
                Ok(())
            }
            Self::Analysis {
                code,
                message,
                phase,
            } => {
                write!(f, "[{}] Analysis error: {}", code, message)?;
                if let Some(ph) = phase {
                    write!(f, " (phase: {})", ph)?;
                }
                Ok(())
            }
            Self::Cli { code, message, arg } => {
                write!(f, "[{}] CLI error: {}", code, message)?;
                if let Some(a) = arg {
                    write!(f, " (argument: {})", a)?;
                }
                Ok(())
            }
            Self::Validation {
                code,
                count,
                errors,
            } => {
                write!(f, "[{}] Validation failed with {} error(s)", code, count)?;
                if *count <= 3 {
                    for (i, err) in errors.iter().enumerate() {
                        write!(f, "\n  {}. {}", i + 1, err)?;
                    }
                } else {
                    for (i, err) in errors.iter().take(2).enumerate() {
                        write!(f, "\n  {}. {}", i + 1, err)?;
                    }
                    write!(f, "\n  ... and {} more", count - 2)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for DebtmapError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => source
                .as_ref()
                .map(|s| s.as_ref() as &(dyn std::error::Error + 'static)),
            _ => None,
        }
    }
}

// =============================================================================
// Serde Serialization for Structured Logging
// =============================================================================

impl Serialize for DebtmapError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("DebtmapError", 5)?;
        state.serialize_field("code", &self.code().as_str())?;
        state.serialize_field("category", &self.category())?;
        state.serialize_field("message", &self.to_string())?;
        state.serialize_field("retryable", &self.is_retryable())?;
        state.serialize_field("user_fixable", &self.is_user_fixable())?;
        state.end()
    }
}

// =============================================================================
// Migration: From Old Error Types
// =============================================================================

// From src/error.rs CliError
impl From<crate::error::CliError> for DebtmapError {
    fn from(err: crate::error::CliError) -> Self {
        match err {
            crate::error::CliError::InvalidCommand(msg) => Self::cli_invalid_command(msg),
            crate::error::CliError::MissingArgument(arg) => Self::cli_missing_arg(arg),
            crate::error::CliError::InvalidArgument(msg) => Self::Cli {
                code: ErrorCode::CLI_INVALID_ARG,
                message: msg,
                arg: None,
            },
            crate::error::CliError::Config(config_err) => config_err.into(),
        }
    }
}

// From src/error.rs ConfigError
impl From<crate::error::ConfigError> for DebtmapError {
    fn from(err: crate::error::ConfigError) -> Self {
        match err {
            crate::error::ConfigError::InvalidThreshold(msg) => Self::Config {
                code: ErrorCode::CONFIG_INVALID,
                message: msg,
                field: Some("threshold".to_string()),
                path: None,
            },
            crate::error::ConfigError::PathNotFound(path) => {
                Self::config_with_path(format!("Path not found: {}", path.display()), path)
            }
            crate::error::ConfigError::InvalidConfigFile(msg) => Self::Config {
                code: ErrorCode::CONFIG_INVALID,
                message: msg,
                field: None,
                path: None,
            },
            crate::error::ConfigError::ValidationFailed(msg) => Self::validation(msg),
            crate::error::ConfigError::Io(io_err) => Self::from_io_error(io_err, None),
        }
    }
}

// From src/error.rs AnalysisError
impl From<crate::error::AnalysisError> for DebtmapError {
    fn from(err: crate::error::AnalysisError) -> Self {
        match err {
            crate::error::AnalysisError::ParseError { path, source } => {
                Self::parse(source.to_string(), path, None, None)
            }
            crate::error::AnalysisError::AnalysisFailed(msg) => Self::analysis(msg),
            crate::error::AnalysisError::Io(io_err) => Self::from_io_error(io_err, None),
        }
    }
}

// From src/error.rs AppError
impl From<crate::error::AppError> for DebtmapError {
    fn from(err: crate::error::AppError) -> Self {
        match err {
            crate::error::AppError::Cli(cli_err) => cli_err.into(),
            crate::error::AppError::Analysis(analysis_err) => analysis_err.into(),
        }
    }
}

// From src/errors/mod.rs AnalysisError (the unified one)
impl From<crate::errors::AnalysisError> for DebtmapError {
    fn from(err: crate::errors::AnalysisError) -> Self {
        match err {
            crate::errors::AnalysisError::IoError { message, path } => Self::io(message, path),
            crate::errors::AnalysisError::ParseError {
                message,
                path,
                line,
            } => Self::Parse {
                code: ErrorCode::PARSE_GENERIC,
                message,
                path: path.unwrap_or_else(|| PathBuf::from("<unknown>")),
                line,
                column: None,
            },
            crate::errors::AnalysisError::ValidationError { message } => Self::validation(message),
            crate::errors::AnalysisError::ConfigError { message, path } => Self::Config {
                code: ErrorCode::CONFIG_GENERIC,
                message,
                field: None,
                path,
            },
            crate::errors::AnalysisError::CoverageError { message, path: _ } => Self::Analysis {
                code: ErrorCode::ANALYSIS_COVERAGE,
                message: format!("Coverage error: {}", message),
                phase: Some(AnalysisPhase::CoverageLoading),
            },
            crate::errors::AnalysisError::AnalysisFailure { message } => Self::analysis(message),
            crate::errors::AnalysisError::Other(message) => Self::analysis(message),
        }
    }
}

// From src/core/errors.rs Error
impl From<crate::core::errors::Error> for DebtmapError {
    fn from(err: crate::core::errors::Error) -> Self {
        match err {
            crate::core::errors::Error::FileSystem {
                message,
                path,
                source,
            } => {
                if let Some(io_err) = source {
                    let mut result = Self::from_io_error(io_err, path);
                    // Override message if we have a better one
                    if !message.is_empty() {
                        if let Self::Io {
                            message: ref mut msg,
                            ..
                        } = result
                        {
                            *msg = message;
                        }
                    }
                    result
                } else {
                    Self::io(message, path)
                }
            }
            crate::core::errors::Error::Parse {
                file,
                line,
                column,
                message,
            } => Self::parse(message, file, Some(line), Some(column)),
            crate::core::errors::Error::Analysis(message) => Self::analysis(message),
            crate::core::errors::Error::Configuration(message) => Self::config(message),
            crate::core::errors::Error::Unsupported(message) => Self::Parse {
                code: ErrorCode::PARSE_UNSUPPORTED,
                message,
                path: PathBuf::from("<unsupported>"),
                line: None,
                column: None,
            },
            crate::core::errors::Error::Validation(message) => Self::validation(message),
            crate::core::errors::Error::Dependency(message) => {
                Self::analysis(format!("Dependency error: {}", message))
            }
            crate::core::errors::Error::Concurrency(message) => {
                Self::analysis(format!("Concurrency error: {}", message))
            }
            crate::core::errors::Error::WithContext { context, message } => {
                Self::analysis(format!("{}: {}", context, message))
            }
            crate::core::errors::Error::External(anyhow_err) => {
                Self::analysis(anyhow_err.to_string())
            }
            crate::core::errors::Error::Io(io_err) => Self::from_io_error(io_err, None),
            crate::core::errors::Error::Json(json_err) => {
                Self::parse(format!("JSON error: {}", json_err), "<json>", None, None)
            }
            crate::core::errors::Error::Pattern(pattern_err) => {
                Self::config(format!("Pattern error: {}", pattern_err))
            }
        }
    }
}

// From std::io::Error
impl From<std::io::Error> for DebtmapError {
    fn from(err: std::io::Error) -> Self {
        Self::from_io_error(err, None)
    }
}

// From anyhow::Error for backwards compatibility
impl From<anyhow::Error> for DebtmapError {
    fn from(err: anyhow::Error) -> Self {
        let error_string = err.to_string();

        // Try to categorize based on common patterns
        if error_string.contains("I/O error") || error_string.contains("No such file") {
            Self::io(error_string, None)
        } else if error_string.contains("Parse error") || error_string.contains("syntax") {
            Self::parse(error_string, "<unknown>", None, None)
        } else if error_string.contains("Config") || error_string.contains("configuration") {
            Self::config(error_string)
        } else if error_string.contains("Validation") || error_string.contains("invalid") {
            Self::validation(error_string)
        } else {
            Self::analysis(error_string)
        }
    }
}

// Note: anyhow::Error has a blanket impl From<E: std::error::Error>, so DebtmapError
// automatically converts to anyhow::Error via `.into()` since it implements std::error::Error.
// No explicit impl needed.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_creation() {
        let err = DebtmapError::io("File not found", Some(PathBuf::from("/path/to/file")));
        assert_eq!(err.code(), ErrorCode::IO_GENERIC);
        assert_eq!(err.category(), "I/O");
        assert_eq!(err.path(), Some(&PathBuf::from("/path/to/file")));
        assert!(!err.is_user_fixable());
    }

    #[test]
    fn test_io_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = DebtmapError::from_io_error(io_err, Some(PathBuf::from("/test")));
        assert_eq!(err.code(), ErrorCode::IO_FILE_NOT_FOUND);
    }

    #[test]
    fn test_parse_error_creation() {
        let err = DebtmapError::parse("Unexpected token", "/path/to/file.rs", Some(42), Some(10));
        assert_eq!(err.code(), ErrorCode::PARSE_GENERIC);
        assert_eq!(err.category(), "Parse");
        assert!(err.is_user_fixable());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_config_error_with_field() {
        let err = DebtmapError::config_with_field("Invalid value", "threshold");
        assert_eq!(err.code(), ErrorCode::CONFIG_INVALID);
        assert!(err.is_user_fixable());
    }

    #[test]
    fn test_cli_error_missing_arg() {
        let err = DebtmapError::cli_missing_arg("--output");
        assert_eq!(err.code(), ErrorCode::CLI_MISSING_ARG);
        assert!(err.is_user_fixable());
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn test_validation_error() {
        let err = DebtmapError::validations(vec![
            "Error 1".to_string(),
            "Error 2".to_string(),
            "Error 3".to_string(),
        ]);
        assert_eq!(err.code(), ErrorCode::VALIDATION_GENERIC);
        assert!(err.is_user_fixable());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_is_retryable_resource_busy() {
        let err = DebtmapError::Io {
            code: ErrorCode::IO_RESOURCE_BUSY,
            message: "Resource busy".to_string(),
            path: None,
            source: None,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_retryable_timeout_in_message() {
        let err = DebtmapError::io("Connection timed out", None);
        assert!(err.is_retryable());
    }

    #[test]
    fn test_is_not_retryable_file_not_found() {
        let err = DebtmapError::Io {
            code: ErrorCode::IO_FILE_NOT_FOUND,
            message: "File not found".to_string(),
            path: None,
            source: None,
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = DebtmapError::parse("Unexpected token", "/src/main.rs", Some(42), Some(10));
        let display = format!("{}", err);
        assert!(display.contains("E019")); // PARSE_GENERIC
        assert!(display.contains("Parse error"));
        assert!(display.contains("/src/main.rs"));
        assert!(display.contains("line 42"));
        assert!(display.contains("column 10"));
    }

    #[test]
    fn test_validation_display_truncates() {
        let err = DebtmapError::validations(vec![
            "Error 1".to_string(),
            "Error 2".to_string(),
            "Error 3".to_string(),
            "Error 4".to_string(),
        ]);
        let display = format!("{}", err);
        assert!(display.contains("4 error(s)"));
        assert!(display.contains("Error 1"));
        assert!(display.contains("Error 2"));
        assert!(display.contains("and 2 more"));
    }

    #[test]
    fn test_error_serialization() {
        let err = DebtmapError::io("Test error", None);
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"code\":\"E009\""));
        assert!(json.contains("\"category\":\"I/O\""));
        assert!(json.contains("\"retryable\":false"));
    }

    #[test]
    fn test_from_errors_analysis_error() {
        let old_err = crate::errors::AnalysisError::io("Old style error");
        let new_err: DebtmapError = old_err.into();
        assert_eq!(new_err.category(), "I/O");
    }

    #[test]
    fn test_from_error_config_error() {
        let old_err = crate::error::ConfigError::InvalidThreshold("must be > 0".to_string());
        let new_err: DebtmapError = old_err.into();
        assert_eq!(new_err.category(), "Config");
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(DebtmapError::cli("test").exit_code(), 2);
        assert_eq!(DebtmapError::config("test").exit_code(), 3);
        assert_eq!(DebtmapError::validation("test").exit_code(), 4);
        assert_eq!(
            DebtmapError::parse("test", "file", None, None).exit_code(),
            5
        );
        assert_eq!(DebtmapError::analysis("test").exit_code(), 1);
        assert_eq!(DebtmapError::io("test", None).exit_code(), 1);
    }

    #[test]
    fn test_into_anyhow() {
        let err = DebtmapError::io("Test error", None);
        let anyhow_err: anyhow::Error = err.into();
        assert!(anyhow_err.to_string().contains("I/O error"));
    }
}
