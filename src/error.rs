//! Domain-specific error types for debtmap (DEPRECATED).
//!
//! **DEPRECATED**: This module is deprecated in favor of [`crate::debtmap_error::DebtmapError`].
//! The types here are maintained for backwards compatibility during migration.
//! New code should use [`DebtmapError`](crate::debtmap_error::DebtmapError) instead.
//!
//! This module provides a clear hierarchy of error types that correspond to different
//! error domains in the application:
//!
//! - `CliError`: CLI argument parsing and validation errors
//! - `ConfigError`: Configuration building and validation errors
//! - `AnalysisError`: Analysis execution errors
//! - `AppError`: Top-level application error that encompasses all domains
//!
//! # Migration
//!
//! All types in this module implement `Into<DebtmapError>` for gradual migration:
//!
//! ```rust,ignore
//! use debtmap::error::ConfigError;
//! use debtmap::debtmap_error::DebtmapError;
//!
//! let old_error = ConfigError::InvalidThreshold("must be > 0".to_string());
//! let new_error: DebtmapError = old_error.into();
//! ```
//!
//! # Error Boundaries
//!
//! - **CLI Layer**: Use `CliError` for argument parsing and validation
//! - **Config Layer**: Use `ConfigError` for configuration building
//! - **Analysis Layer**: Use `AnalysisError` for execution errors
//!
//! # Examples
//!
//! ```
//! use debtmap::error::{CliError, ConfigError, AppError};
//! use std::path::PathBuf;
//!
//! fn validate_path(path: &PathBuf) -> Result<(), ConfigError> {
//!     if !path.exists() {
//!         return Err(ConfigError::PathNotFound(path.clone()));
//!     }
//!     Ok(())
//! }
//!
//! fn parse_command() -> Result<(), CliError> {
//!     let path = PathBuf::from("/nonexistent");
//!     validate_path(&path).map_err(CliError::Config)?;
//!     Ok(())
//! }
//! ```

use std::path::PathBuf;

/// Errors that occur during CLI argument parsing and validation
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    #[error("Invalid argument value: {0}")]
    InvalidArgument(String),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
}

/// Errors during configuration building and validation
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid threshold value: {0}")]
    InvalidThreshold(String),

    #[error("Path does not exist: {}", .0.display())]
    PathNotFound(PathBuf),

    #[error("Invalid configuration file: {0}")]
    InvalidConfigFile(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors during analysis execution
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("Failed to parse file: {}", .path.display())]
    ParseError {
        path: PathBuf,
        #[source]
        source: anyhow::Error,
    },

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Top-level application error
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("CLI error: {0}")]
    Cli(#[from] CliError),

    #[error("Analysis error: {0}")]
    Analysis(#[from] AnalysisError),
}

impl AppError {
    /// Get exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Cli(_) => 2,      // Invalid usage
            AppError::Analysis(_) => 1, // Analysis failed
        }
    }

    /// Get user-facing error message with recovery suggestions
    pub fn user_message(&self) -> String {
        match self {
            AppError::Cli(CliError::Config(ConfigError::PathNotFound(path))) => {
                format!(
                    "Error: Path '{}' does not exist.\n\n\
                     Suggestion: Check the path and try again, or run:\n\
                     debtmap analyze <path>",
                    path.display()
                )
            }
            AppError::Cli(CliError::Config(ConfigError::InvalidThreshold(msg))) => {
                format!(
                    "Error: {}\n\n\
                     Suggestion: Use --threshold-complexity <n> where n > 0\n\
                     See 'debtmap analyze --help' for more information.",
                    msg
                )
            }
            AppError::Analysis(AnalysisError::ParseError { path, source }) => {
                format!(
                    "Error: Failed to parse '{}':\n  {}\n\n\
                     Suggestion: Check file syntax or exclude with --exclude",
                    path.display(),
                    source
                )
            }
            _ => self.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_error_from_config_error() {
        let config_err = ConfigError::InvalidThreshold("test".to_string());
        let cli_err: CliError = config_err.into();
        assert!(matches!(cli_err, CliError::Config(_)));
    }

    #[test]
    fn test_app_error_from_cli_error() {
        let cli_err = CliError::InvalidCommand("test".to_string());
        let app_err: AppError = cli_err.into();
        assert!(matches!(app_err, AppError::Cli(_)));
        assert_eq!(app_err.exit_code(), 2);
    }

    #[test]
    fn test_app_error_from_analysis_error() {
        let analysis_err = AnalysisError::AnalysisFailed("test".to_string());
        let app_err: AppError = analysis_err.into();
        assert!(matches!(app_err, AppError::Analysis(_)));
        assert_eq!(app_err.exit_code(), 1);
    }

    #[test]
    fn test_path_not_found_user_message() {
        let err = AppError::Cli(CliError::Config(ConfigError::PathNotFound(PathBuf::from(
            "/nonexistent",
        ))));
        let msg = err.user_message();
        assert!(msg.contains("does not exist"));
        assert!(msg.contains("Suggestion"));
    }

    #[test]
    fn test_invalid_threshold_user_message() {
        let err = AppError::Cli(CliError::Config(ConfigError::InvalidThreshold(
            "must be > 0".to_string(),
        )));
        let msg = err.user_message();
        assert!(msg.contains("must be > 0"));
        assert!(msg.contains("--threshold-complexity"));
    }
}
