//! CLI argument validation with premortem error accumulation.
//!
//! This module provides validation helpers for CLI arguments that integrate
//! with the AnalysisConfig validation system. It ensures all errors are
//! reported together with source locations.
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::config::cli_validation::validate_analyze_args;
//! use std::path::PathBuf;
//!
//! let errors = validate_analyze_args(
//!     &PathBuf::from("./src"),
//!     true,   // aggregate_only
//!     true,   // no_aggregation (conflict!)
//!     4,      // jobs
//!     false,  // no_parallel
//!     Some(PathBuf::from("coverage.lcov")),
//!     true,   // enable_context
//!     false,  // no_multi_pass
//! );
//!
//! if !errors.is_empty() {
//!     for error in &errors {
//!         eprintln!("  [cli] {}", error);
//!     }
//!     std::process::exit(1);
//! }
//! ```

use std::path::{Path, PathBuf};

use super::analysis_config::{AnalysisConfig, AnalysisConfigBuilder};
use super::multi_source::ConfigSource;
use crate::errors::AnalysisError;

/// Validation result for CLI arguments.
pub struct CliValidationResult {
    /// The validated config (if all validations pass)
    pub config: Option<AnalysisConfig>,
    /// All validation errors with source locations
    pub errors: Vec<CliValidationError>,
}

/// A CLI validation error with source context.
#[derive(Debug, Clone)]
pub struct CliValidationError {
    /// The CLI argument that failed validation
    pub argument: String,
    /// Human-readable error message
    pub message: String,
    /// Optional suggestion for fixing
    pub suggestion: Option<String>,
}

impl std::fmt::Display for CliValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[cli:{}] {}", self.argument, self.message)?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, "\n    suggestion: {}", suggestion)?;
        }
        Ok(())
    }
}

impl From<CliValidationError> for AnalysisError {
    fn from(err: CliValidationError) -> Self {
        AnalysisError::config(err.to_string())
    }
}

/// Validate analyze command arguments, accumulating ALL errors.
///
/// Returns an empty Vec if all arguments are valid, otherwise returns
/// all validation errors with their CLI argument context.
#[allow(clippy::too_many_arguments)]
pub fn validate_analyze_args(
    path: &Path,
    aggregate_only: bool,
    no_aggregation: bool,
    jobs: usize,
    no_parallel: bool,
    coverage_file: Option<&Path>,
    enable_context: bool,
    no_multi_pass: bool,
) -> Vec<CliValidationError> {
    let mut errors = Vec::new();

    // Validate path exists and is a directory
    if !path.exists() {
        errors.push(CliValidationError {
            argument: "PATH".to_string(),
            message: format!("directory does not exist: {}", path.display()),
            suggestion: Some("Check that the path is correct and accessible".to_string()),
        });
    } else if !path.is_dir() {
        errors.push(CliValidationError {
            argument: "PATH".to_string(),
            message: format!("path is not a directory: {}", path.display()),
            suggestion: Some("Provide a directory path, not a file".to_string()),
        });
    }

    // Validate mutual exclusions
    if aggregate_only && no_aggregation {
        errors.push(CliValidationError {
            argument: "--aggregate-only".to_string(),
            message: "cannot use with --no-aggregation".to_string(),
            suggestion: Some("Remove one of these options".to_string()),
        });
        errors.push(CliValidationError {
            argument: "--no-aggregation".to_string(),
            message: "cannot use with --aggregate-only".to_string(),
            suggestion: Some("Remove one of these options".to_string()),
        });
    }

    // Validate jobs configuration
    if !no_parallel && jobs == 0 {
        // jobs=0 means "use all cores" which is valid
        // Only warn if it's genuinely invalid
    }

    if jobs > 256 {
        errors.push(CliValidationError {
            argument: "--jobs".to_string(),
            message: format!("value {} is too large (max 256)", jobs),
            suggestion: Some("Use a value between 1 and 256".to_string()),
        });
    }

    // Validate coverage file exists if provided
    if let Some(coverage_path) = coverage_file {
        if !coverage_path.exists() {
            errors.push(CliValidationError {
                argument: "--coverage-file".to_string(),
                message: format!("file does not exist: {}", coverage_path.display()),
                suggestion: Some("Check that the coverage file path is correct".to_string()),
            });
        }
    }

    // Validate context dependencies
    // Note: multi_pass without context is checked in AnalysisConfig validation
    // Here we just flag obvious conflicts
    if !enable_context && !no_multi_pass {
        // multi_pass defaults to enabled when context is enabled
        // This is a soft warning, not an error, so we don't add it
    }

    errors
}

/// Build an AnalysisConfig from CLI arguments with full validation.
///
/// This function creates an AnalysisConfig from CLI arguments, validates
/// all values, and returns either a valid config or all accumulated errors.
#[allow(clippy::too_many_arguments)]
pub fn build_analysis_config_from_cli(
    path: PathBuf,
    aggregate_only: bool,
    no_aggregation: bool,
    jobs: usize,
    no_parallel: bool,
    coverage_file: Option<PathBuf>,
    enable_context: bool,
    no_multi_pass: bool,
    complexity_threshold: u32,
    exclude_patterns: Vec<String>,
    show_config_sources: bool,
) -> Result<AnalysisConfig, Vec<AnalysisError>> {
    // Determine effective values
    let parallel = !no_parallel;
    let multi_pass = enable_context && !no_multi_pass;
    let effective_jobs = if parallel && jobs == 0 {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4)
    } else {
        jobs
    };

    // Build config with source tracking
    let builder = AnalysisConfigBuilder::new(path)
        .parallel_from(
            parallel,
            ConfigSource::Environment("--no-parallel".to_string()),
        )
        .jobs_from(
            effective_jobs,
            ConfigSource::Environment("--jobs".to_string()),
        )
        .aggregate_only(aggregate_only)
        .no_aggregation(no_aggregation)
        .coverage_file_from(
            coverage_file,
            ConfigSource::Environment("--coverage-file".to_string()),
        )
        .enable_context(enable_context)
        .multi_pass(multi_pass)
        .complexity_threshold(complexity_threshold)
        .exclude_patterns(exclude_patterns)
        .show_config_sources(show_config_sources);

    builder.build()
}

/// Format CLI validation errors for display.
///
/// # Example Output
///
/// ```text
/// CLI argument errors (2):
///
///   [cli:--aggregate-only] cannot use with --no-aggregation
///     suggestion: Remove one of these options
///
///   [cli:--coverage-file] file does not exist: ./missing.lcov
///     suggestion: Check that the coverage file path is correct
/// ```
pub fn format_cli_errors(errors: &[CliValidationError]) -> String {
    if errors.is_empty() {
        return String::new();
    }

    let mut output = format!("CLI argument errors ({}):\n", errors.len());

    for error in errors {
        output.push_str(&format!("\n  {}\n", error));
    }

    output.push_str("\nFix all errors and try again.\n");
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_valid_args_no_errors() {
        let temp_dir = TempDir::new().unwrap();

        let errors = validate_analyze_args(
            temp_dir.path(),
            false, // aggregate_only
            false, // no_aggregation
            4,     // jobs
            false, // no_parallel
            None,  // coverage_file
            false, // enable_context
            true,  // no_multi_pass
        );

        assert!(errors.is_empty());
    }

    #[test]
    fn test_mutual_exclusion_error() {
        let temp_dir = TempDir::new().unwrap();

        let errors = validate_analyze_args(
            temp_dir.path(),
            true, // aggregate_only
            true, // no_aggregation (conflict!)
            4,
            false,
            None,
            false,
            true,
        );

        assert_eq!(errors.len(), 2); // Both options flagged
        assert!(errors[0].argument.contains("aggregate"));
        assert!(errors[1].argument.contains("aggregation"));
    }

    #[test]
    fn test_missing_path_error() {
        let errors = validate_analyze_args(
            &PathBuf::from("/nonexistent/path/that/does/not/exist"),
            false,
            false,
            4,
            false,
            None,
            false,
            true,
        );

        assert!(!errors.is_empty());
        assert!(errors[0].argument == "PATH");
        assert!(errors[0].message.contains("does not exist"));
    }

    #[test]
    fn test_missing_coverage_file_error() {
        let temp_dir = TempDir::new().unwrap();
        let missing_file = PathBuf::from("/nonexistent/coverage.lcov");

        let errors = validate_analyze_args(
            temp_dir.path(),
            false,
            false,
            4,
            false,
            Some(missing_file.as_path()),
            false,
            true,
        );

        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.argument == "--coverage-file" && e.message.contains("does not exist")));
    }

    #[test]
    fn test_jobs_too_high_error() {
        let temp_dir = TempDir::new().unwrap();

        let errors = validate_analyze_args(
            temp_dir.path(),
            false,
            false,
            500, // Too many jobs
            false,
            None,
            false,
            true,
        );

        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.argument == "--jobs"));
    }

    #[test]
    fn test_format_cli_errors() {
        let errors = vec![CliValidationError {
            argument: "--test".to_string(),
            message: "test error".to_string(),
            suggestion: Some("fix it".to_string()),
        }];

        let output = format_cli_errors(&errors);
        assert!(output.contains("CLI argument errors (1)"));
        assert!(output.contains("--test"));
        assert!(output.contains("test error"));
        assert!(output.contains("fix it"));
    }

    #[test]
    fn test_build_analysis_config_valid() {
        let temp_dir = TempDir::new().unwrap();

        let result = build_analysis_config_from_cli(
            temp_dir.path().to_path_buf(),
            false,
            false,
            4,
            false,
            None,
            false,
            true,
            50,
            Vec::new(),
            false,
        );

        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.parallel);
        assert_eq!(config.jobs, 4);
    }

    #[test]
    fn test_build_analysis_config_with_errors() {
        let result = build_analysis_config_from_cli(
            PathBuf::from("/nonexistent"),
            true,  // aggregate_only
            true,  // no_aggregation (conflict!)
            0,     // jobs (will be auto-detected)
            false, // parallel enabled
            None,
            true,                         // enable_context
            false,                        // multi_pass enabled (requires context)
            0,                            // complexity_threshold (invalid!)
            vec!["[invalid".to_string()], // invalid glob
            false,
        );

        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Should have multiple errors
        assert!(errors.len() >= 2);
    }

    #[test]
    fn test_auto_detect_jobs() {
        let temp_dir = TempDir::new().unwrap();

        let result = build_analysis_config_from_cli(
            temp_dir.path().to_path_buf(),
            false,
            false,
            0,     // jobs = 0 means auto-detect
            false, // parallel enabled
            None,
            false,
            true,
            50,
            Vec::new(),
            false,
        );

        assert!(result.is_ok());
        let config = result.unwrap();
        // Jobs should have been auto-detected to a sensible value
        assert!(config.jobs > 0);
    }
}
