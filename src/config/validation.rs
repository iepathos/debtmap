//! Validation with error accumulation for configuration.
//!
//! This module provides validation functions that use stillwater's `Validation` type
//! to accumulate ALL errors instead of failing at the first one. This enables users
//! to see all configuration issues in a single run.
//!
//! # Design Philosophy
//!
//! - **Error Accumulation**: Collect ALL validation errors before reporting
//! - **Pure Functions**: All validation functions are pure and testable
//! - **Context Preservation**: Errors include file paths, line numbers, and field names
//! - **Backwards Compatible**: Wrappers convert to `anyhow::Result` for existing code
//! - **Field Context** (Spec 003): Nested field paths for precise error reporting
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::config::validation::{validate_config, validate_config_result};
//! use debtmap::config::DebtmapConfig;
//!
//! // Get validation with ALL errors accumulated
//! let validation = validate_config(&raw_config);
//!
//! // Or use backwards-compatible Result API
//! let result = validate_config_result(&raw_config);
//! ```
//!
//! # Field-Aware Validation (Spec 003)
//!
//! Use `validate_config_with_context` for structured errors with field paths:
//!
//! ```rust,ignore
//! use debtmap::config::validation::validate_config_with_context;
//!
//! let result = validate_config_with_context(&config);
//! if let Validation::Failure(errors) = result {
//!     for error in errors {
//!         println!("{}: {}", error.field, error.message);
//!         // e.g., "scoring.coverage: weight out of range (expected: 0.0-1.0, got: -0.5)"
//!     }
//! }
//! ```

use std::path::{Path, PathBuf};

use crate::effects::validation::{FieldPath, ValidationError};
use crate::effects::{
    combine_validations, run_validation, validation_failure, validation_failures,
    validation_success, AnalysisValidation,
};
use crate::errors::AnalysisError;
use stillwater::{NonEmptyVec, Validation};

use super::scoring::ScoringWeights;
use super::thresholds::{ThresholdsConfig, ValidationThresholds};
use super::DebtmapConfig;

/// Validation result with field context for structured error reporting.
pub type FieldValidation<T> = Validation<T, NonEmptyVec<ValidationError>>;

/// Validate entire config, accumulating ALL errors.
///
/// This is the primary validation function that collects all configuration
/// errors instead of failing at the first one.
///
/// # Example
///
/// ```rust
/// use debtmap::config::validation::validate_config;
/// use debtmap::config::DebtmapConfig;
///
/// let config = DebtmapConfig::default();
/// let validation = validate_config(&config);
/// assert!(validation.is_success());
/// ```
pub fn validate_config(config: &DebtmapConfig) -> AnalysisValidation<()> {
    let validations = vec![
        validate_scoring_weights(config.scoring.as_ref()),
        validate_thresholds_config(config.thresholds.as_ref()),
        validate_ignore_patterns(config.ignore.as_ref()),
    ];

    combine_validations(validations).map(|_| ())
}

/// Validate config with backwards-compatible Result API.
///
/// This wraps `validate_config` to return `anyhow::Result` for use with
/// existing code that expects fail-fast error handling.
pub fn validate_config_result(config: &DebtmapConfig) -> anyhow::Result<()> {
    run_validation(validate_config(config))
}

/// Validate scoring weights, accumulating all weight errors.
fn validate_scoring_weights(scoring: Option<&ScoringWeights>) -> AnalysisValidation<()> {
    let Some(weights) = scoring else {
        return validation_success(());
    };

    let mut errors = Vec::new();

    // Validate individual weight ranges
    if !ScoringWeights::is_valid_weight(weights.coverage) {
        errors.push(AnalysisError::config(format!(
            "Coverage weight out of range: {} (must be 0.0-1.0)",
            weights.coverage
        )));
    }

    if !ScoringWeights::is_valid_weight(weights.complexity) {
        errors.push(AnalysisError::config(format!(
            "Complexity weight out of range: {} (must be 0.0-1.0)",
            weights.complexity
        )));
    }

    if !ScoringWeights::is_valid_weight(weights.semantic) {
        errors.push(AnalysisError::config(format!(
            "Semantic weight out of range: {} (must be 0.0-1.0)",
            weights.semantic
        )));
    }

    if !ScoringWeights::is_valid_weight(weights.dependency) {
        errors.push(AnalysisError::config(format!(
            "Dependency weight out of range: {} (must be 0.0-1.0)",
            weights.dependency
        )));
    }

    if !ScoringWeights::is_valid_weight(weights.security) {
        errors.push(AnalysisError::config(format!(
            "Security weight out of range: {} (must be 0.0-1.0)",
            weights.security
        )));
    }

    if !ScoringWeights::is_valid_weight(weights.organization) {
        errors.push(AnalysisError::config(format!(
            "Organization weight out of range: {} (must be 0.0-1.0)",
            weights.organization
        )));
    }

    // Validate active weights sum to 1.0
    let active_sum = weights.coverage + weights.complexity + weights.dependency;
    if (active_sum - 1.0).abs() > 0.001 {
        errors.push(AnalysisError::config(format!(
            "Active weights (coverage, complexity, dependency) must sum to 1.0, got {:.3}",
            active_sum
        )));
    }

    if errors.is_empty() {
        validation_success(())
    } else {
        validation_failures(errors)
    }
}

/// Validate thresholds configuration.
fn validate_thresholds_config(thresholds: Option<&ThresholdsConfig>) -> AnalysisValidation<()> {
    let Some(thresholds) = thresholds else {
        return validation_success(());
    };

    let mut errors = Vec::new();

    // Validate complexity threshold
    if let Some(complexity) = thresholds.complexity {
        if complexity == 0 {
            errors.push(AnalysisError::config(
                "Complexity threshold cannot be zero".to_string(),
            ));
        }
    }

    // Validate max_file_length
    if let Some(max_len) = thresholds.max_file_length {
        if max_len == 0 {
            errors.push(AnalysisError::config(
                "Max file length cannot be zero".to_string(),
            ));
        }
    }

    // Validate validation thresholds if present
    if let Some(ref validation) = thresholds.validation {
        errors.extend(validate_validation_thresholds(validation));
    }

    if errors.is_empty() {
        validation_success(())
    } else {
        validation_failures(errors)
    }
}

/// Validate validation thresholds, returning errors.
fn validate_validation_thresholds(thresholds: &ValidationThresholds) -> Vec<AnalysisError> {
    let mut errors = Vec::new();

    if thresholds.max_average_complexity < 0.0 {
        errors.push(AnalysisError::config(format!(
            "Max average complexity cannot be negative: {}",
            thresholds.max_average_complexity
        )));
    }

    if thresholds.max_debt_density < 0.0 {
        errors.push(AnalysisError::config(format!(
            "Max debt density cannot be negative: {}",
            thresholds.max_debt_density
        )));
    }

    if thresholds.max_codebase_risk_score < 0.0 {
        errors.push(AnalysisError::config(format!(
            "Max codebase risk score cannot be negative: {}",
            thresholds.max_codebase_risk_score
        )));
    }

    if thresholds.min_coverage_percentage < 0.0 || thresholds.min_coverage_percentage > 100.0 {
        errors.push(AnalysisError::config(format!(
            "Min coverage percentage must be 0-100: {}",
            thresholds.min_coverage_percentage
        )));
    }

    errors
}

/// Validate ignore patterns configuration.
fn validate_ignore_patterns(ignore: Option<&super::core::IgnoreConfig>) -> AnalysisValidation<()> {
    let Some(ignore) = ignore else {
        return validation_success(());
    };

    let mut errors = Vec::new();

    for (i, pattern) in ignore.patterns.iter().enumerate() {
        // Validate glob pattern syntax
        if let Err(e) = glob::Pattern::new(pattern) {
            errors.push(AnalysisError::config(format!(
                "Invalid ignore pattern #{}: '{}' - {}",
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

/// Validate that paths exist, accumulating all path errors.
///
/// This is useful for validating CLI arguments where multiple paths
/// are provided and we want to report all missing paths at once.
pub fn validate_paths_exist(paths: &[PathBuf]) -> AnalysisValidation<Vec<PathBuf>> {
    let validations: Vec<AnalysisValidation<PathBuf>> = paths
        .iter()
        .map(|path| {
            if path.exists() {
                validation_success(path.clone())
            } else {
                validation_failure(AnalysisError::io_with_path(
                    format!("Path not found: {}", path.display()),
                    path.clone(),
                ))
            }
        })
        .collect();

    combine_validations(validations)
}

/// Validate paths with backwards-compatible Result API.
pub fn validate_paths_exist_result(paths: &[PathBuf]) -> anyhow::Result<Vec<PathBuf>> {
    run_validation(validate_paths_exist(paths))
}

/// Validate a config file path exists and is readable.
pub fn validate_config_path(path: &Path) -> AnalysisValidation<PathBuf> {
    if !path.exists() {
        return validation_failure(AnalysisError::config_with_path(
            format!("Config file not found: {}", path.display()),
            path,
        ));
    }

    if !path.is_file() {
        return validation_failure(AnalysisError::config_with_path(
            format!("Config path is not a file: {}", path.display()),
            path,
        ));
    }

    // Try to read to check permissions
    match std::fs::read_to_string(path) {
        Ok(_) => validation_success(path.to_path_buf()),
        Err(e) => validation_failure(AnalysisError::io_with_path(
            format!("Cannot read config file: {}", e),
            path,
        )),
    }
}

/// Validate multiple regex patterns, accumulating all errors.
pub fn validate_regex_patterns(patterns: &[String]) -> AnalysisValidation<Vec<regex::Regex>> {
    let validations: Vec<AnalysisValidation<regex::Regex>> = patterns
        .iter()
        .enumerate()
        .map(|(i, pattern)| match regex::Regex::new(pattern) {
            Ok(regex) => validation_success(regex),
            Err(e) => validation_failure(AnalysisError::config(format!(
                "Invalid regex pattern #{}: '{}' - {}",
                i + 1,
                pattern,
                e
            ))),
        })
        .collect();

    combine_validations(validations)
}

/// Validate regex patterns with backwards-compatible Result API.
pub fn validate_regex_patterns_result(patterns: &[String]) -> anyhow::Result<Vec<regex::Regex>> {
    run_validation(validate_regex_patterns(patterns))
}

// =============================================================================
// Field-Aware Validation Functions (Spec 003)
// =============================================================================

/// Validate entire config with field context, accumulating ALL errors.
///
/// Unlike `validate_config`, this function returns errors with full field paths,
/// enabling precise error reporting for IDE integration and tooling.
///
/// # Example
///
/// ```rust
/// use debtmap::config::validation::validate_config_with_context;
/// use debtmap::config::DebtmapConfig;
///
/// let config = DebtmapConfig::default();
/// let validation = validate_config_with_context(&config);
/// assert!(validation.is_success());
/// ```
pub fn validate_config_with_context(config: &DebtmapConfig) -> FieldValidation<()> {
    let root = FieldPath::root();
    let mut errors: Vec<ValidationError> = Vec::new();

    // Validate scoring weights
    if let Some(ref scoring) = config.scoring {
        errors.extend(validate_scoring_weights_with_context(
            scoring,
            &root.push("scoring"),
        ));
    }

    // Validate thresholds
    if let Some(ref thresholds) = config.thresholds {
        errors.extend(validate_thresholds_with_context(
            thresholds,
            &root.push("thresholds"),
        ));
    }

    // Validate ignore patterns
    if let Some(ref ignore) = config.ignore {
        errors.extend(validate_ignore_patterns_with_context(
            &ignore.patterns,
            &root.push("ignore").push("patterns"),
        ));
    }

    if errors.is_empty() {
        Validation::Success(())
    } else {
        Validation::Failure(NonEmptyVec::from_vec(errors).expect("errors cannot be empty"))
    }
}

/// Validate scoring weights with field context.
fn validate_scoring_weights_with_context(
    weights: &ScoringWeights,
    path: &FieldPath,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Helper to validate a single weight
    let validate_weight = |name: &str, value: f64| -> Option<ValidationError> {
        if !ScoringWeights::is_valid_weight(value) {
            Some(
                ValidationError::at_field(&path.push(name), "weight out of range")
                    .with_context("0.0 to 1.0", format!("{:.3}", value)),
            )
        } else {
            None
        }
    };

    // Validate individual weights
    if let Some(err) = validate_weight("coverage", weights.coverage) {
        errors.push(err);
    }
    if let Some(err) = validate_weight("complexity", weights.complexity) {
        errors.push(err);
    }
    if let Some(err) = validate_weight("semantic", weights.semantic) {
        errors.push(err);
    }
    if let Some(err) = validate_weight("dependency", weights.dependency) {
        errors.push(err);
    }
    if let Some(err) = validate_weight("security", weights.security) {
        errors.push(err);
    }
    if let Some(err) = validate_weight("organization", weights.organization) {
        errors.push(err);
    }

    // Validate active weights sum to 1.0
    let active_sum = weights.coverage + weights.complexity + weights.dependency;
    if (active_sum - 1.0).abs() > 0.001 {
        errors.push(
            ValidationError::at_field(path, "active weights must sum to 1.0")
                .with_context("1.0", format!("{:.3}", active_sum)),
        );
    }

    errors
}

/// Validate thresholds with field context.
fn validate_thresholds_with_context(
    thresholds: &ThresholdsConfig,
    path: &FieldPath,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Validate complexity threshold
    if let Some(complexity) = thresholds.complexity {
        if complexity == 0 {
            errors.push(
                ValidationError::at_field(&path.push("complexity"), "cannot be zero")
                    .with_expected("positive integer"),
            );
        }
    }

    // Validate max_file_length
    if let Some(max_len) = thresholds.max_file_length {
        if max_len == 0 {
            errors.push(
                ValidationError::at_field(&path.push("max_file_length"), "cannot be zero")
                    .with_expected("positive integer"),
            );
        }
    }

    // Validate validation thresholds if present
    if let Some(ref validation) = thresholds.validation {
        let validation_path = path.push("validation");
        errors.extend(validate_validation_thresholds_with_context(
            validation,
            &validation_path,
        ));
    }

    errors
}

/// Validate validation thresholds with field context.
fn validate_validation_thresholds_with_context(
    thresholds: &ValidationThresholds,
    path: &FieldPath,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if thresholds.max_average_complexity < 0.0 {
        errors.push(
            ValidationError::at_field(&path.push("max_average_complexity"), "cannot be negative")
                .with_context(
                    "non-negative number",
                    format!("{:.2}", thresholds.max_average_complexity),
                ),
        );
    }

    if thresholds.max_debt_density < 0.0 {
        errors.push(
            ValidationError::at_field(&path.push("max_debt_density"), "cannot be negative")
                .with_context(
                    "non-negative number",
                    format!("{:.2}", thresholds.max_debt_density),
                ),
        );
    }

    if thresholds.max_codebase_risk_score < 0.0 {
        errors.push(
            ValidationError::at_field(&path.push("max_codebase_risk_score"), "cannot be negative")
                .with_context(
                    "non-negative number",
                    format!("{:.2}", thresholds.max_codebase_risk_score),
                ),
        );
    }

    if thresholds.min_coverage_percentage < 0.0 || thresholds.min_coverage_percentage > 100.0 {
        errors.push(
            ValidationError::at_field(&path.push("min_coverage_percentage"), "out of valid range")
                .with_context(
                    "0 to 100",
                    format!("{:.2}", thresholds.min_coverage_percentage),
                ),
        );
    }

    errors
}

/// Validate ignore patterns with field context.
fn validate_ignore_patterns_with_context(
    patterns: &[String],
    path: &FieldPath,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for (i, pattern) in patterns.iter().enumerate() {
        let pattern_path = path.push(format!("[{}]", i));
        if let Err(e) = glob::Pattern::new(pattern) {
            errors.push(
                ValidationError::at_field(&pattern_path, "invalid glob pattern")
                    .with_context("valid glob pattern", format!("'{}' - {}", pattern, e)),
            );
        }
    }

    errors
}

/// Validate regex patterns with field context.
pub fn validate_regex_patterns_with_context(
    patterns: &[String],
    path: &FieldPath,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    for (i, pattern) in patterns.iter().enumerate() {
        let pattern_path = path.push(format!("[{}]", i));
        if let Err(e) = regex::Regex::new(pattern) {
            errors.push(
                ValidationError::at_field(&pattern_path, "invalid regex pattern")
                    .with_context("valid regex", format!("'{}' - {}", pattern, e)),
            );
        }
    }

    errors
}

/// Combine multiple field validations.
pub fn combine_field_validations<T: Clone>(
    validations: Vec<FieldValidation<T>>,
) -> FieldValidation<Vec<T>> {
    let mut successes = Vec::new();
    let mut failures: Vec<ValidationError> = Vec::new();

    for v in validations {
        match v {
            Validation::Success(value) => successes.push(value),
            Validation::Failure(errors) => {
                for err in errors {
                    failures.push(err);
                }
            }
        }
    }

    if failures.is_empty() {
        Validation::Success(successes)
    } else {
        Validation::Failure(NonEmptyVec::from_vec(failures).expect("failures cannot be empty here"))
    }
}

/// Create a successful field validation.
pub fn field_validation_success<T>(value: T) -> FieldValidation<T> {
    Validation::Success(value)
}

/// Create a failed field validation with a single error.
pub fn field_validation_failure<T>(error: ValidationError) -> FieldValidation<T> {
    Validation::Failure(NonEmptyVec::new(error, Vec::new()))
}

/// Create a failed field validation with multiple errors.
pub fn field_validation_failures<T>(errors: Vec<ValidationError>) -> FieldValidation<T> {
    Validation::Failure(NonEmptyVec::from_vec(errors).expect("errors cannot be empty"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use stillwater::Validation;

    #[test]
    fn test_validate_config_default_succeeds() {
        let config = DebtmapConfig::default();
        let result = validate_config(&config);
        assert!(result.is_success());
    }

    #[test]
    fn test_validate_scoring_weights_accumulates_errors() {
        let weights = ScoringWeights {
            coverage: -0.5,     // Error 1: negative
            complexity: 1.5,    // Error 2: > 1.0
            semantic: 0.0,      // OK
            dependency: 0.2,    // OK
            security: 2.0,      // Error 3: > 1.0
            organization: -1.0, // Error 4: negative
        };

        let result = validate_scoring_weights(Some(&weights));

        match result {
            Validation::Failure(errors) => {
                // Should have at least 4 individual weight errors + sum error
                let error_count = errors.len();
                assert!(
                    error_count >= 4,
                    "Expected at least 4 errors, got {}",
                    error_count
                );

                let error_msgs: Vec<String> = errors.into_iter().map(|e| e.to_string()).collect();
                assert!(error_msgs.iter().any(|e| e.contains("Coverage")));
                assert!(error_msgs.iter().any(|e| e.contains("Complexity")));
                assert!(error_msgs.iter().any(|e| e.contains("Security")));
                assert!(error_msgs.iter().any(|e| e.contains("Organization")));
            }
            Validation::Success(_) => panic!("Expected validation failure"),
        }
    }

    #[test]
    fn test_validate_scoring_weights_sum_error() {
        let weights = ScoringWeights {
            coverage: 0.5,
            complexity: 0.5,
            semantic: 0.0,
            dependency: 0.5, // Sum = 1.5, not 1.0
            security: 0.0,
            organization: 0.0,
        };

        let result = validate_scoring_weights(Some(&weights));

        match result {
            Validation::Failure(errors) => {
                let error_msgs: Vec<String> = errors.into_iter().map(|e| e.to_string()).collect();
                assert!(error_msgs.iter().any(|e| e.contains("must sum to 1.0")));
            }
            Validation::Success(_) => panic!("Expected validation failure"),
        }
    }

    #[test]
    fn test_validate_paths_accumulates_errors() {
        let paths = vec![
            PathBuf::from("/definitely/nonexistent/path1"),
            PathBuf::from("/definitely/nonexistent/path2"),
            PathBuf::from("/definitely/nonexistent/path3"),
        ];

        let result = validate_paths_exist(&paths);

        match result {
            Validation::Failure(errors) => {
                assert_eq!(errors.len(), 3, "Expected 3 path errors");
            }
            Validation::Success(_) => panic!("Expected validation failure for nonexistent paths"),
        }
    }

    #[test]
    fn test_validate_regex_patterns_accumulates_errors() {
        let patterns = vec![
            "[unclosed".to_string(),      // Error 1: unclosed bracket
            "valid.*pattern".to_string(), // OK
            "(?P<".to_string(),           // Error 2: incomplete group
            "[a-z]+".to_string(),         // OK
        ];

        let result = validate_regex_patterns(&patterns);

        match result {
            Validation::Failure(errors) => {
                assert_eq!(errors.len(), 2, "Expected 2 regex errors");
            }
            Validation::Success(_) => panic!("Expected validation failure for invalid patterns"),
        }
    }

    #[test]
    fn test_validate_regex_patterns_all_valid() {
        let patterns = vec![
            ".*\\.rs$".to_string(),
            "[a-zA-Z_]+".to_string(),
            "\\d{3}-\\d{4}".to_string(),
        ];

        let result = validate_regex_patterns(&patterns);
        assert!(result.is_success());
    }

    #[test]
    fn test_validate_thresholds_accumulates_errors() {
        let thresholds = ThresholdsConfig {
            complexity: Some(0),      // Error: zero
            max_file_length: Some(0), // Error: zero
            ..Default::default()
        };

        let result = validate_thresholds_config(Some(&thresholds));

        match result {
            Validation::Failure(errors) => {
                assert_eq!(errors.len(), 2, "Expected 2 threshold errors");
            }
            Validation::Success(_) => panic!("Expected validation failure"),
        }
    }

    #[test]
    fn test_validate_validation_thresholds_accumulates_errors() {
        let thresholds = ValidationThresholds {
            max_average_complexity: -5.0,   // Error: negative
            max_debt_density: -10.0,        // Error: negative
            max_codebase_risk_score: -1.0,  // Error: negative
            min_coverage_percentage: 150.0, // Error: > 100
            ..Default::default()
        };

        let errors = validate_validation_thresholds(&thresholds);
        assert_eq!(errors.len(), 4, "Expected 4 validation threshold errors");
    }

    #[test]
    fn test_validate_ignore_patterns_accumulates_errors() {
        use super::super::core::IgnoreConfig;

        let ignore = IgnoreConfig {
            patterns: vec![
                "[invalid".to_string(),      // Error: unclosed bracket
                "valid/**/*.rs".to_string(), // OK
                "[also-invalid".to_string(), // Error: unclosed bracket
            ],
        };

        let result = validate_ignore_patterns(Some(&ignore));

        match result {
            Validation::Failure(errors) => {
                assert_eq!(errors.len(), 2, "Expected 2 pattern errors");
            }
            Validation::Success(_) => panic!("Expected validation failure"),
        }
    }

    #[test]
    fn test_validate_config_result_backwards_compatible() {
        let config = DebtmapConfig::default();
        let result = validate_config_result(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_result_shows_all_errors() {
        let config = DebtmapConfig {
            scoring: Some(ScoringWeights {
                coverage: -0.5,
                complexity: 1.5,
                semantic: 0.0,
                dependency: 0.2,
                security: 0.0,
                organization: 0.0,
            }),
            ..Default::default()
        };

        let result = validate_config_result(&config);
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        // Should mention multiple errors
        assert!(
            error_msg.contains("Multiple errors") || error_msg.contains("Coverage"),
            "Error should mention the issues: {}",
            error_msg
        );
    }

    // =========================================================================
    // Field-Aware Validation Tests (Spec 003)
    // =========================================================================

    #[test]
    fn test_validate_config_with_context_default_succeeds() {
        let config = DebtmapConfig::default();
        let result = validate_config_with_context(&config);
        assert!(result.is_success());
    }

    #[test]
    fn test_validate_scoring_weights_with_context_field_paths() {
        let weights = ScoringWeights {
            coverage: -0.5,  // Error
            complexity: 1.5, // Error
            semantic: 0.0,
            dependency: 0.2,
            security: 0.0,
            organization: 0.0,
        };

        let path = FieldPath::new("scoring");
        let errors = validate_scoring_weights_with_context(&weights, &path);

        // Should have errors for coverage and complexity, plus sum error
        assert!(
            errors.len() >= 2,
            "Expected at least 2 errors, got {}",
            errors.len()
        );

        // Check field paths are correct
        let coverage_error = errors
            .iter()
            .find(|e| e.field.as_string().contains("coverage"));
        assert!(coverage_error.is_some(), "Expected coverage error");
        assert_eq!(
            coverage_error.unwrap().field.as_string(),
            "scoring.coverage"
        );

        let complexity_error = errors
            .iter()
            .find(|e| e.field.as_string().contains("complexity"));
        assert!(complexity_error.is_some(), "Expected complexity error");
        assert_eq!(
            complexity_error.unwrap().field.as_string(),
            "scoring.complexity"
        );
    }

    #[test]
    fn test_validate_scoring_weights_with_context_has_context() {
        let weights = ScoringWeights {
            coverage: -0.5,
            complexity: 0.8,
            semantic: 0.0,
            dependency: 0.7, // sum will be 1.0, so only coverage error
            security: 0.0,
            organization: 0.0,
        };

        let path = FieldPath::new("scoring");
        let errors = validate_scoring_weights_with_context(&weights, &path);

        // Find the coverage error
        let coverage_error = errors
            .iter()
            .find(|e| e.field.as_string().contains("coverage"));
        assert!(coverage_error.is_some());

        let err = coverage_error.unwrap();
        assert!(err.expected.is_some(), "Expected 'expected' context");
        assert!(err.actual.is_some(), "Expected 'actual' context");
        assert!(err.expected.as_ref().unwrap().contains("0.0"));
        assert!(err.actual.as_ref().unwrap().contains("-0.5"));
    }

    #[test]
    fn test_validate_thresholds_with_context_field_paths() {
        let thresholds = ThresholdsConfig {
            complexity: Some(0),
            max_file_length: Some(0),
            ..Default::default()
        };

        let path = FieldPath::new("thresholds");
        let errors = validate_thresholds_with_context(&thresholds, &path);

        assert_eq!(errors.len(), 2, "Expected 2 errors");

        let complexity_error = errors
            .iter()
            .find(|e| e.field.as_string().contains("complexity"));
        assert!(complexity_error.is_some());
        assert_eq!(
            complexity_error.unwrap().field.as_string(),
            "thresholds.complexity"
        );

        let max_file_length_error = errors
            .iter()
            .find(|e| e.field.as_string().contains("max_file_length"));
        assert!(max_file_length_error.is_some());
        assert_eq!(
            max_file_length_error.unwrap().field.as_string(),
            "thresholds.max_file_length"
        );
    }

    #[test]
    fn test_validate_validation_thresholds_with_context() {
        let thresholds = ValidationThresholds {
            max_average_complexity: -5.0,
            max_debt_density: -10.0,
            max_codebase_risk_score: 0.0, // OK
            min_coverage_percentage: 150.0,
            ..Default::default()
        };

        let path = FieldPath::new("validation");
        let errors = validate_validation_thresholds_with_context(&thresholds, &path);

        assert_eq!(errors.len(), 3, "Expected 3 errors");

        // Verify nested field paths
        assert!(errors
            .iter()
            .any(|e| e.field.as_string() == "validation.max_average_complexity"));
        assert!(errors
            .iter()
            .any(|e| e.field.as_string() == "validation.max_debt_density"));
        assert!(errors
            .iter()
            .any(|e| e.field.as_string() == "validation.min_coverage_percentage"));
    }

    #[test]
    fn test_validate_ignore_patterns_with_context_array_indices() {
        let patterns = vec![
            "[invalid".to_string(),
            "valid/**/*.rs".to_string(),
            "[also-invalid".to_string(),
        ];

        let path = FieldPath::new("ignore").push("patterns");
        let errors = validate_ignore_patterns_with_context(&patterns, &path);

        assert_eq!(errors.len(), 2, "Expected 2 errors");

        // Check array index notation
        assert!(errors
            .iter()
            .any(|e| e.field.as_string() == "ignore.patterns.[0]"));
        assert!(errors
            .iter()
            .any(|e| e.field.as_string() == "ignore.patterns.[2]"));
    }

    #[test]
    fn test_validate_regex_patterns_with_context() {
        let patterns = vec![
            "[unclosed".to_string(),
            "valid.*".to_string(),
            "(?P<".to_string(),
        ];

        let path = FieldPath::new("config").push("patterns");
        let errors = validate_regex_patterns_with_context(&patterns, &path);

        assert_eq!(errors.len(), 2, "Expected 2 errors");

        // Verify paths include array indices
        assert!(errors
            .iter()
            .any(|e| e.field.as_string() == "config.patterns.[0]"));
        assert!(errors
            .iter()
            .any(|e| e.field.as_string() == "config.patterns.[2]"));
    }

    #[test]
    fn test_validate_config_with_context_full_integration() {
        use super::super::core::IgnoreConfig;

        let config = DebtmapConfig {
            scoring: Some(ScoringWeights {
                coverage: -0.5, // Error
                complexity: 0.5,
                semantic: 0.0,
                dependency: 0.5, // Sum OK
                security: 0.0,
                organization: 0.0,
            }),
            thresholds: Some(ThresholdsConfig {
                complexity: Some(0), // Error
                ..Default::default()
            }),
            ignore: Some(IgnoreConfig {
                patterns: vec!["[invalid".to_string()], // Error
            }),
            ..Default::default()
        };

        let result = validate_config_with_context(&config);

        match result {
            Validation::Failure(errors) => {
                // We may get 3 or 4 errors depending on sum validation
                // Key is that each section has at least one error
                assert!(
                    errors.len() >= 3,
                    "Expected at least 3 errors from different sections, got {}",
                    errors.len()
                );

                // Verify each section has an error
                assert!(
                    errors
                        .iter()
                        .any(|e| e.field.as_string().starts_with("scoring")),
                    "Expected scoring error"
                );
                assert!(
                    errors
                        .iter()
                        .any(|e| e.field.as_string().starts_with("thresholds")),
                    "Expected thresholds error"
                );
                assert!(
                    errors
                        .iter()
                        .any(|e| e.field.as_string().starts_with("ignore")),
                    "Expected ignore error"
                );
            }
            Validation::Success(_) => panic!("Expected validation failure"),
        }
    }

    #[test]
    fn test_combine_field_validations() {
        let v1: FieldValidation<i32> = field_validation_success(1);
        let v2: FieldValidation<i32> =
            field_validation_failure(ValidationError::for_field("field1", "error 1"));
        let v3: FieldValidation<i32> = field_validation_success(3);
        let v4: FieldValidation<i32> =
            field_validation_failure(ValidationError::for_field("field2", "error 2"));

        let result = combine_field_validations(vec![v1, v2, v3, v4]);

        match result {
            Validation::Failure(errors) => {
                assert_eq!(errors.len(), 2, "Expected 2 accumulated errors");
                assert!(errors.iter().any(|e| e.field.as_string() == "field1"));
                assert!(errors.iter().any(|e| e.field.as_string() == "field2"));
            }
            Validation::Success(_) => panic!("Expected failure with accumulated errors"),
        }
    }

    #[test]
    fn test_field_validation_helpers() {
        let success: FieldValidation<i32> = field_validation_success(42);
        assert!(success.is_success());

        let failure: FieldValidation<i32> =
            field_validation_failure(ValidationError::for_field("test", "error"));
        assert!(failure.is_failure());

        let failures: FieldValidation<i32> = field_validation_failures(vec![
            ValidationError::for_field("field1", "error 1"),
            ValidationError::for_field("field2", "error 2"),
        ]);
        match failures {
            Validation::Failure(errs) => assert_eq!(errs.len(), 2),
            _ => panic!("Expected failure"),
        }
    }

    #[test]
    fn test_validation_error_display_format() {
        let error = ValidationError::at_field(
            &FieldPath::new("scoring").push("coverage"),
            "weight out of range",
        )
        .with_context("0.0 to 1.0", "-0.5");

        let display = format!("{}", error);
        assert!(display.contains("scoring.coverage"));
        assert!(display.contains("weight out of range"));
        assert!(display.contains("expected: 0.0 to 1.0"));
        assert!(display.contains("got: -0.5"));
    }
}
