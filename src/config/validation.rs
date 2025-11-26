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

use std::path::{Path, PathBuf};

use crate::effects::{
    combine_validations, run_validation, validation_failure, validation_failures,
    validation_success, AnalysisValidation,
};
use crate::errors::AnalysisError;

use super::scoring::ScoringWeights;
use super::thresholds::{ThresholdsConfig, ValidationThresholds};
use super::DebtmapConfig;

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
}
