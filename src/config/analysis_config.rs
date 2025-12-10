//! Analysis configuration with premortem validation.
//!
//! This module provides the `AnalysisConfig` struct with comprehensive validation
//! using premortem's error accumulation and source tracking capabilities.
//!
//! # Design Philosophy
//!
//! - **Error Accumulation**: Show ALL configuration errors at once, not just the first
//! - **Source Tracking**: Know exactly where each config value came from
//! - **Cross-Field Validation**: Validate mutual exclusions and dependencies
//! - **Path Validation**: Verify paths exist before analysis begins
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::config::analysis_config::{AnalysisConfig, AnalysisConfigBuilder};
//! use std::path::PathBuf;
//!
//! let config = AnalysisConfigBuilder::new(PathBuf::from("src"))
//!     .parallel(true)
//!     .jobs(4)
//!     .coverage_file(Some(PathBuf::from("coverage.lcov")))
//!     .build()?;
//!
//! // config is now validated - safe to use
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::effects::{
    combine_validations, validation_failure, validation_failures, validation_success,
    AnalysisValidation,
};
use crate::errors::AnalysisError;

use super::multi_source::ConfigSource;

/// Analysis configuration with declarative validation.
///
/// This struct holds all configuration options for code analysis,
/// with validation rules enforced at construction time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// Project root directory to analyze.
    pub project_path: PathBuf,

    /// Enable parallel analysis.
    #[serde(default)]
    pub parallel: bool,

    /// Number of parallel jobs (1-256 when parallel is enabled).
    #[serde(default = "default_jobs")]
    pub jobs: usize,

    /// Output only aggregate results (mutually exclusive with no_aggregation).
    #[serde(default)]
    pub aggregate_only: bool,

    /// Disable aggregation entirely (mutually exclusive with aggregate_only).
    #[serde(default)]
    pub no_aggregation: bool,

    /// LCOV coverage file path (must exist if provided).
    #[serde(default)]
    pub coverage_file: Option<PathBuf>,

    /// Enable context-aware analysis.
    #[serde(default)]
    pub enable_context: bool,

    /// Enable multi-pass analysis (requires enable_context).
    #[serde(default)]
    pub multi_pass: bool,

    /// Complexity threshold for recommendations (1-1000).
    #[serde(default = "default_complexity_threshold")]
    pub complexity_threshold: u32,

    /// File patterns to exclude from analysis.
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// Show where config values came from (debug mode).
    #[serde(default)]
    pub show_config_sources: bool,
}

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
}

fn default_complexity_threshold() -> u32 {
    50
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            project_path: PathBuf::from("."),
            parallel: false,
            jobs: default_jobs(),
            aggregate_only: false,
            no_aggregation: false,
            coverage_file: None,
            enable_context: false,
            multi_pass: false,
            complexity_threshold: default_complexity_threshold(),
            exclude_patterns: Vec::new(),
            show_config_sources: false,
        }
    }
}

/// Traced configuration value with source information.
#[derive(Debug, Clone)]
pub struct TracedAnalysisValue<T> {
    /// The actual value
    pub value: T,
    /// Where this value came from
    pub source: ConfigSource,
}

impl<T> TracedAnalysisValue<T> {
    pub fn new(value: T, source: ConfigSource) -> Self {
        Self { value, source }
    }
}

/// A validation error with source location context.
#[derive(Debug, Clone)]
pub struct ConfigValidationError {
    /// The field path (e.g., "aggregate_only", "jobs")
    pub path: String,
    /// Source location where the value came from
    pub source: Option<ConfigSource>,
    /// The invalid value (as string for display)
    pub value: Option<String>,
    /// Human-readable error message
    pub message: String,
    /// Optional suggestion for fixing the error
    pub suggestion: Option<String>,
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref source) = self.source {
            write!(f, "[{}] ", source)?;
        }
        write!(f, "{}: {}", self.path, self.message)?;
        if let Some(ref value) = self.value {
            write!(f, " (value: {})", value)?;
        }
        if let Some(ref suggestion) = self.suggestion {
            write!(f, "\n    suggestion: {}", suggestion)?;
        }
        Ok(())
    }
}

impl AnalysisConfig {
    /// Validate the configuration, accumulating ALL errors.
    ///
    /// Returns `AnalysisValidation` which is either:
    /// - `Success(())` if all validation passes
    /// - `Failure(errors)` with ALL accumulated errors
    pub fn validate(&self) -> AnalysisValidation<()> {
        let validations = vec![
            self.validate_project_path(),
            self.validate_mutual_exclusions(),
            self.validate_parallel_jobs(),
            self.validate_context_dependencies(),
            self.validate_paths(),
            self.validate_complexity_threshold(),
            self.validate_exclude_patterns(),
        ];

        combine_validations(validations).map(|_| ())
    }

    /// Validates that project_path is not empty.
    fn validate_project_path(&self) -> AnalysisValidation<()> {
        if self.project_path.as_os_str().is_empty() {
            validation_failure(AnalysisError::config(
                "project_path cannot be empty".to_string(),
            ))
        } else {
            validation_success(())
        }
    }

    /// Validates mutually exclusive options.
    fn validate_mutual_exclusions(&self) -> AnalysisValidation<()> {
        if self.aggregate_only && self.no_aggregation {
            validation_failure(AnalysisError::config(
                "aggregate_only and no_aggregation are mutually exclusive".to_string(),
            ))
        } else {
            validation_success(())
        }
    }

    /// Validates parallel job configuration.
    fn validate_parallel_jobs(&self) -> AnalysisValidation<()> {
        if self.parallel && self.jobs == 0 {
            return validation_failure(AnalysisError::config(
                "jobs must be > 0 when parallel is enabled".to_string(),
            ));
        }

        if self.jobs > 256 {
            return validation_failure(AnalysisError::config(format!(
                "jobs must be <= 256, got {}",
                self.jobs
            )));
        }

        validation_success(())
    }

    /// Validates context-dependent options.
    fn validate_context_dependencies(&self) -> AnalysisValidation<()> {
        if self.multi_pass && !self.enable_context {
            validation_failure(AnalysisError::config(
                "multi_pass requires enable_context to be true".to_string(),
            ))
        } else {
            validation_success(())
        }
    }

    /// Validates file paths exist.
    fn validate_paths(&self) -> AnalysisValidation<()> {
        let mut errors = Vec::new();

        // Validate project_path is a directory
        if !self.project_path.is_dir() {
            if !self.project_path.exists() {
                errors.push(AnalysisError::config(format!(
                    "project_path directory does not exist: {}",
                    self.project_path.display()
                )));
            } else {
                errors.push(AnalysisError::config(format!(
                    "project_path is not a directory: {}",
                    self.project_path.display()
                )));
            }
        }

        // Validate coverage_file exists if provided
        if let Some(ref coverage) = self.coverage_file {
            if !coverage.exists() {
                errors.push(AnalysisError::config(format!(
                    "coverage_file does not exist: {}",
                    coverage.display()
                )));
            }
        }

        if errors.is_empty() {
            validation_success(())
        } else {
            validation_failures(errors)
        }
    }

    /// Validates complexity threshold range.
    fn validate_complexity_threshold(&self) -> AnalysisValidation<()> {
        if self.complexity_threshold == 0 {
            return validation_failure(AnalysisError::config(
                "complexity_threshold must be > 0".to_string(),
            ));
        }

        if self.complexity_threshold > 1000 {
            return validation_failure(AnalysisError::config(format!(
                "complexity_threshold must be <= 1000, got {}",
                self.complexity_threshold
            )));
        }

        validation_success(())
    }

    /// Validates exclude patterns are valid globs.
    fn validate_exclude_patterns(&self) -> AnalysisValidation<()> {
        let mut errors = Vec::new();

        for (i, pattern) in self.exclude_patterns.iter().enumerate() {
            if let Err(e) = glob::Pattern::new(pattern) {
                errors.push(AnalysisError::config(format!(
                    "invalid exclude pattern #{}: '{}' - {}",
                    i + 1,
                    pattern,
                    e
                )));
            }
        }

        if errors.is_empty() {
            validation_success(())
        } else {
            validation_failures(errors)
        }
    }
}

/// Builder for AnalysisConfig with validation.
///
/// This builder collects all configuration values and validates them
/// at build time, reporting ALL errors at once.
#[derive(Debug)]
pub struct AnalysisConfigBuilder {
    config: AnalysisConfig,
    /// Track sources for each field
    sources: std::collections::HashMap<String, ConfigSource>,
}

impl AnalysisConfigBuilder {
    /// Create a new builder with required project path.
    pub fn new(project_path: PathBuf) -> Self {
        let mut sources = std::collections::HashMap::new();
        sources.insert("project_path".to_string(), ConfigSource::Default);

        Self {
            config: AnalysisConfig {
                project_path,
                ..Default::default()
            },
            sources,
        }
    }

    /// Set parallel analysis mode.
    pub fn parallel(mut self, parallel: bool) -> Self {
        self.config.parallel = parallel;
        self
    }

    /// Set parallel analysis with source tracking.
    pub fn parallel_from(mut self, parallel: bool, source: ConfigSource) -> Self {
        self.config.parallel = parallel;
        self.sources.insert("parallel".to_string(), source);
        self
    }

    /// Set number of parallel jobs.
    pub fn jobs(mut self, jobs: usize) -> Self {
        self.config.jobs = jobs;
        self
    }

    /// Set jobs with source tracking.
    pub fn jobs_from(mut self, jobs: usize, source: ConfigSource) -> Self {
        self.config.jobs = jobs;
        self.sources.insert("jobs".to_string(), source);
        self
    }

    /// Set aggregate_only mode.
    pub fn aggregate_only(mut self, aggregate_only: bool) -> Self {
        self.config.aggregate_only = aggregate_only;
        self
    }

    /// Set no_aggregation mode.
    pub fn no_aggregation(mut self, no_aggregation: bool) -> Self {
        self.config.no_aggregation = no_aggregation;
        self
    }

    /// Set coverage file path.
    pub fn coverage_file(mut self, coverage_file: Option<PathBuf>) -> Self {
        self.config.coverage_file = coverage_file;
        self
    }

    /// Set coverage file with source tracking.
    pub fn coverage_file_from(
        mut self,
        coverage_file: Option<PathBuf>,
        source: ConfigSource,
    ) -> Self {
        self.config.coverage_file = coverage_file;
        self.sources.insert("coverage_file".to_string(), source);
        self
    }

    /// Set enable_context mode.
    pub fn enable_context(mut self, enable_context: bool) -> Self {
        self.config.enable_context = enable_context;
        self
    }

    /// Set multi_pass mode.
    pub fn multi_pass(mut self, multi_pass: bool) -> Self {
        self.config.multi_pass = multi_pass;
        self
    }

    /// Set complexity threshold.
    pub fn complexity_threshold(mut self, threshold: u32) -> Self {
        self.config.complexity_threshold = threshold;
        self
    }

    /// Set exclude patterns.
    pub fn exclude_patterns(mut self, patterns: Vec<String>) -> Self {
        self.config.exclude_patterns = patterns;
        self
    }

    /// Set show_config_sources.
    pub fn show_config_sources(mut self, show: bool) -> Self {
        self.config.show_config_sources = show;
        self
    }

    /// Build and validate the configuration.
    ///
    /// Returns `Ok(AnalysisConfig)` if valid, or `Err` with ALL validation errors.
    pub fn build(self) -> Result<AnalysisConfig, Vec<AnalysisError>> {
        match self.config.validate() {
            stillwater::Validation::Success(_) => Ok(self.config),
            stillwater::Validation::Failure(errors) => Err(errors.into_iter().collect()),
        }
    }

    /// Build with validation using AnalysisValidation for error accumulation.
    pub fn build_validated(self) -> AnalysisValidation<AnalysisConfig> {
        match self.config.validate() {
            stillwater::Validation::Success(_) => validation_success(self.config),
            stillwater::Validation::Failure(errors) => stillwater::Validation::Failure(errors),
        }
    }

    /// Get tracked sources for debugging.
    pub fn sources(&self) -> &std::collections::HashMap<String, ConfigSource> {
        &self.sources
    }
}

/// Format validation errors with source locations for user display.
///
/// # Example Output
///
/// ```text
/// Configuration errors (3):
///
///   [debtmap.toml:8] aggregate_only: aggregate_only and no_aggregation are mutually exclusive
///     value: true
///     suggestion: Remove one of these options
///
///   [env:DEBTMAP_JOBS] jobs: value 0 is not in range 1..=256
///     value: 0
///     suggestion: Set DEBTMAP_JOBS to a value between 1 and 256
/// ```
pub fn format_config_errors(errors: &[AnalysisError]) -> String {
    let mut output = format!("Configuration errors ({}):\n", errors.len());

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
    fn test_valid_config_builds_successfully() {
        let temp_dir = TempDir::new().unwrap();

        let config = AnalysisConfigBuilder::new(temp_dir.path().to_path_buf())
            .parallel(true)
            .jobs(4)
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.parallel);
        assert_eq!(config.jobs, 4);
    }

    #[test]
    fn test_mutual_exclusion_error() {
        let temp_dir = TempDir::new().unwrap();

        let result = AnalysisConfigBuilder::new(temp_dir.path().to_path_buf())
            .aggregate_only(true)
            .no_aggregation(true)
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.to_string().contains("mutually exclusive")));
    }

    #[test]
    fn test_parallel_jobs_validation() {
        let temp_dir = TempDir::new().unwrap();

        // jobs=0 with parallel=true should fail
        let result = AnalysisConfigBuilder::new(temp_dir.path().to_path_buf())
            .parallel(true)
            .jobs(0)
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.to_string().contains("jobs")));
    }

    #[test]
    fn test_jobs_too_high() {
        let temp_dir = TempDir::new().unwrap();

        let result = AnalysisConfigBuilder::new(temp_dir.path().to_path_buf())
            .jobs(500)
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.to_string().contains("256")));
    }

    #[test]
    fn test_context_dependency_validation() {
        let temp_dir = TempDir::new().unwrap();

        // multi_pass without enable_context should fail
        let result = AnalysisConfigBuilder::new(temp_dir.path().to_path_buf())
            .multi_pass(true)
            .enable_context(false)
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.to_string().contains("multi_pass requires enable_context")));
    }

    #[test]
    fn test_coverage_file_not_found() {
        let temp_dir = TempDir::new().unwrap();

        let result = AnalysisConfigBuilder::new(temp_dir.path().to_path_buf())
            .coverage_file(Some(PathBuf::from("/nonexistent/coverage.lcov")))
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.to_string().contains("does not exist")));
    }

    #[test]
    fn test_project_path_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        std::fs::write(&file_path, "test").unwrap();

        let result = AnalysisConfigBuilder::new(file_path).build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.to_string().contains("not a directory")));
    }

    #[test]
    fn test_multiple_errors_accumulated() {
        // All errors should be reported together
        let result = AnalysisConfigBuilder::new(PathBuf::from("/nonexistent/path"))
            .parallel(true)
            .jobs(0) // Error: jobs must be > 0
            .aggregate_only(true)
            .no_aggregation(true) // Error: mutually exclusive
            .multi_pass(true) // Error: requires enable_context
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Should have at least 3 errors (path, mutual exclusion, multi_pass)
        assert!(
            errors.len() >= 3,
            "Expected at least 3 errors, got {}: {:?}",
            errors.len(),
            errors
        );
    }

    #[test]
    fn test_invalid_exclude_pattern() {
        let temp_dir = TempDir::new().unwrap();

        let result = AnalysisConfigBuilder::new(temp_dir.path().to_path_buf())
            .exclude_patterns(vec!["[invalid".to_string()])
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.to_string().contains("invalid exclude pattern")));
    }

    #[test]
    fn test_complexity_threshold_validation() {
        let temp_dir = TempDir::new().unwrap();

        // Zero threshold should fail
        let result = AnalysisConfigBuilder::new(temp_dir.path().to_path_buf())
            .complexity_threshold(0)
            .build();

        assert!(result.is_err());

        // Threshold too high should fail
        let result = AnalysisConfigBuilder::new(temp_dir.path().to_path_buf())
            .complexity_threshold(2000)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_default_config() {
        let config = AnalysisConfig::default();
        assert!(!config.parallel);
        assert!(!config.aggregate_only);
        assert!(!config.no_aggregation);
        assert!(!config.enable_context);
        assert!(!config.multi_pass);
        assert_eq!(config.complexity_threshold, 50);
    }

    #[test]
    fn test_format_config_errors() {
        let errors = vec![
            AnalysisError::config("error 1".to_string()),
            AnalysisError::config("error 2".to_string()),
        ];

        let output = format_config_errors(&errors);
        assert!(output.contains("Configuration errors (2)"));
        assert!(output.contains("error 1"));
        assert!(output.contains("error 2"));
    }

    #[test]
    fn test_source_tracking() {
        let temp_dir = TempDir::new().unwrap();

        let builder = AnalysisConfigBuilder::new(temp_dir.path().to_path_buf())
            .jobs_from(8, ConfigSource::Environment("DEBTMAP_JOBS".to_string()));

        let sources = builder.sources();
        assert!(sources.contains_key("jobs"));
        assert!(matches!(
            sources.get("jobs"),
            Some(ConfigSource::Environment(_))
        ));
    }
}
