//! Expanded validation patterns using stillwater's predicate combinators.
//!
//! This module provides validation patterns for:
//! - Analysis results validation with error accumulation
//! - Predicate-based debt detection rules
//! - File processing validation with partial success semantics
//!
//! # Predicate Combinators
//!
//! Use stillwater's predicate module for composable validation rules:
//!
//! ```rust,ignore
//! use stillwater::predicate::*;
//! use debtmap::effects::validation::*;
//!
//! // Define complexity thresholds as composable predicates
//! let warning_zone = gt(20_u32).and(lt(100_u32));
//! let critical_zone = ge(100_u32);
//!
//! // Apply to values using ensure extension
//! let validated = complexity_value.ensure(lt(100_u32), "Complexity too high");
//! ```
//!
//! # EnsureExt Trait
//!
//! The `EnsureExt` trait provides a fluent API for applying predicates:
//!
//! ```rust,ignore
//! use debtmap::effects::validation::EnsureExt;
//! use debtmap::errors::AnalysisError;
//!
//! let validated = file_length
//!     .ensure(le(500_usize), AnalysisError::validation("File too long"));
//! ```
//!
//! # ValidatedFileResults
//!
//! Represents the result of validating multiple files with partial success:
//!
//! ```rust,ignore
//! match validate_files(paths)? {
//!     ValidatedFileResults::AllSucceeded(metrics) => {
//!         println!("All {} files parsed successfully", metrics.len());
//!     }
//!     ValidatedFileResults::PartialSuccess { succeeded, failures } => {
//!         println!("{} succeeded, {} failed", succeeded.len(), failures.len());
//!     }
//! }
//! ```

use crate::core::FileMetrics;
use crate::effects::{validation_success, AnalysisValidation};
use crate::errors::AnalysisError;
use stillwater::predicate::Predicate;
use stillwater::{NonEmptyVec, Validation};

// =============================================================================
// EnsureExt Trait
// =============================================================================

/// Extension trait for applying predicates to values with validation.
///
/// This trait provides a fluent API for validating values against predicates,
/// returning `Validation` results that can accumulate errors.
///
/// # Example
///
/// ```rust
/// use debtmap::effects::validation::EnsureExt;
/// use debtmap::errors::AnalysisError;
/// use stillwater::predicate::lt;
///
/// let complexity: u32 = 50;
/// let validated = complexity.ensure(lt(100_u32), AnalysisError::validation("Complexity too high"));
/// assert!(validated.is_success());
///
/// let high_complexity: u32 = 150;
/// let validated = high_complexity.ensure(lt(100_u32), AnalysisError::validation("Complexity too high"));
/// assert!(validated.is_failure());
/// ```
pub trait EnsureExt<T> {
    /// Validate that this value satisfies the given predicate.
    ///
    /// Returns `Validation::Success(self)` if the predicate passes,
    /// or `Validation::Failure` with the provided error if it fails.
    fn ensure<P, E>(self, predicate: P, error: E) -> Validation<T, NonEmptyVec<E>>
    where
        P: Predicate<T>;

    /// Validate with an error-generating function.
    ///
    /// The function receives a reference to the value and generates
    /// an appropriate error message.
    fn ensure_with<P, E, F>(self, predicate: P, error_fn: F) -> Validation<T, NonEmptyVec<E>>
    where
        P: Predicate<T>,
        F: FnOnce(&T) -> E;
}

impl<T> EnsureExt<T> for T {
    fn ensure<P, E>(self, predicate: P, error: E) -> Validation<T, NonEmptyVec<E>>
    where
        P: Predicate<T>,
    {
        if predicate.check(&self) {
            Validation::Success(self)
        } else {
            Validation::Failure(NonEmptyVec::new(error, Vec::new()))
        }
    }

    fn ensure_with<P, E, F>(self, predicate: P, error_fn: F) -> Validation<T, NonEmptyVec<E>>
    where
        P: Predicate<T>,
        F: FnOnce(&T) -> E,
    {
        if predicate.check(&self) {
            Validation::Success(self)
        } else {
            let error = error_fn(&self);
            Validation::Failure(NonEmptyVec::new(error, Vec::new()))
        }
    }
}

// =============================================================================
// ValidatedFileResults
// =============================================================================

/// Result of validating multiple files with partial success semantics.
///
/// This type allows analysis to continue even when some files fail to parse,
/// collecting both successful results and accumulated errors.
#[derive(Debug, Clone)]
pub enum ValidatedFileResults {
    /// All files parsed and analyzed successfully.
    AllSucceeded(Vec<FileMetrics>),

    /// Some files succeeded, some failed.
    /// Analysis can continue with partial results while reporting errors.
    PartialSuccess {
        /// Successfully parsed and analyzed files.
        succeeded: Vec<FileMetrics>,
        /// Errors from files that failed to parse or analyze.
        failures: NonEmptyVec<AnalysisError>,
    },

    /// All files failed to parse or analyze.
    AllFailed(NonEmptyVec<AnalysisError>),
}

impl ValidatedFileResults {
    /// Create a new ValidatedFileResults from a collection of individual validations.
    pub fn from_validations(validations: Vec<AnalysisValidation<FileMetrics>>) -> Self {
        let mut succeeded = Vec::new();
        let mut failures: Vec<AnalysisError> = Vec::new();

        for validation in validations {
            match validation {
                Validation::Success(metrics) => succeeded.push(metrics),
                Validation::Failure(errors) => {
                    failures.extend(errors);
                }
            }
        }

        match (succeeded.is_empty(), failures.is_empty()) {
            (false, true) => ValidatedFileResults::AllSucceeded(succeeded),
            (false, false) => ValidatedFileResults::PartialSuccess {
                succeeded,
                failures: NonEmptyVec::from_vec(failures)
                    .expect("failures cannot be empty when not all succeeded"),
            },
            (true, false) => ValidatedFileResults::AllFailed(
                NonEmptyVec::from_vec(failures).expect("failures cannot be empty when all failed"),
            ),
            (true, true) => ValidatedFileResults::AllSucceeded(Vec::new()),
        }
    }

    /// Get the successfully parsed files (if any).
    pub fn succeeded(&self) -> &[FileMetrics] {
        match self {
            ValidatedFileResults::AllSucceeded(metrics) => metrics,
            ValidatedFileResults::PartialSuccess { succeeded, .. } => succeeded,
            ValidatedFileResults::AllFailed(_) => &[],
        }
    }

    /// Get the failures (if any).
    pub fn failures(&self) -> Option<&NonEmptyVec<AnalysisError>> {
        match self {
            ValidatedFileResults::AllSucceeded(_) => None,
            ValidatedFileResults::PartialSuccess { failures, .. } => Some(failures),
            ValidatedFileResults::AllFailed(failures) => Some(failures),
        }
    }

    /// Check if all files succeeded.
    pub fn is_all_success(&self) -> bool {
        matches!(self, ValidatedFileResults::AllSucceeded(_))
    }

    /// Check if there are any failures.
    pub fn has_failures(&self) -> bool {
        !matches!(self, ValidatedFileResults::AllSucceeded(_))
    }

    /// Convert to a standard Validation, treating any failure as overall failure.
    pub fn into_validation(self) -> AnalysisValidation<Vec<FileMetrics>> {
        match self {
            ValidatedFileResults::AllSucceeded(metrics) => validation_success(metrics),
            ValidatedFileResults::PartialSuccess { failures, .. } => Validation::Failure(failures),
            ValidatedFileResults::AllFailed(failures) => Validation::Failure(failures),
        }
    }

    /// Convert to Result, including partial successes (lenient mode).
    ///
    /// In lenient mode, if some files succeed, return Ok with those files.
    /// Only fail if ALL files failed.
    pub fn into_lenient_result(self) -> Result<Vec<FileMetrics>, NonEmptyVec<AnalysisError>> {
        match self {
            ValidatedFileResults::AllSucceeded(metrics) => Ok(metrics),
            ValidatedFileResults::PartialSuccess { succeeded, .. } => Ok(succeeded),
            ValidatedFileResults::AllFailed(failures) => Err(failures),
        }
    }
}

// =============================================================================
// Predicate Builders for Debt Detection
// =============================================================================

/// Predicate builder functions for common validation patterns.
///
/// These functions create composable predicates for debt detection rules.
pub mod predicates {
    use stillwater::predicate::*;

    /// Create a predicate that checks if complexity is in the warning zone (high but not critical).
    ///
    /// Default: complexity between 21 and 99 (inclusive on lower bound, exclusive on upper).
    pub fn high_complexity(warning_threshold: u32, critical_threshold: u32) -> impl Predicate<u32> {
        ge(warning_threshold).and(lt(critical_threshold))
    }

    /// Create a predicate that checks if complexity is critical.
    ///
    /// Default: complexity >= 100.
    pub fn critical_complexity(threshold: u32) -> impl Predicate<u32> {
        ge(threshold)
    }

    /// Create a predicate that checks if a value is within acceptable bounds.
    pub fn within_bounds(min: u32, max: u32) -> impl Predicate<u32> {
        ge(min).and(le(max))
    }

    /// Create a predicate that checks if file length is acceptable.
    pub fn acceptable_file_length(max_length: usize) -> impl Predicate<usize> {
        le(max_length)
    }

    /// Create a predicate that checks if nesting depth is acceptable.
    pub fn acceptable_nesting(max_depth: u32) -> impl Predicate<u32> {
        le(max_depth)
    }

    /// Create a predicate that checks if function length is acceptable.
    pub fn acceptable_function_length(max_lines: usize) -> impl Predicate<usize> {
        le(max_lines)
    }

    /// Create a predicate that checks if a string is not empty.
    pub fn not_empty_string() -> impl Predicate<String> {
        not_empty()
    }

    /// Create a predicate that checks if a string length is within bounds.
    pub fn valid_name_length(min: usize, max: usize) -> impl Predicate<String> {
        len_between(min, max)
    }
}

// =============================================================================
// ValidationRuleSet
// =============================================================================

/// Configurable rule set for validation thresholds.
///
/// This struct allows dynamic configuration of validation rules,
/// enabling different strictness levels for different contexts.
#[derive(Debug, Clone)]
pub struct ValidationRuleSet {
    /// Complexity threshold for warnings (default: 21).
    pub complexity_warning: u32,
    /// Complexity threshold for critical issues (default: 100).
    pub complexity_critical: u32,
    /// Maximum function length in lines (default: 50).
    pub max_function_length: usize,
    /// Maximum nesting depth (default: 4).
    pub max_nesting_depth: u32,
    /// Maximum file length in lines (default: 1000).
    pub max_file_length: usize,
    /// Minimum name length for identifiers (default: 2).
    pub min_name_length: usize,
    /// Maximum name length for identifiers (default: 50).
    pub max_name_length: usize,
}

impl Default for ValidationRuleSet {
    fn default() -> Self {
        Self {
            complexity_warning: 21,
            complexity_critical: 100,
            max_function_length: 50,
            max_nesting_depth: 4,
            max_file_length: 1000,
            min_name_length: 2,
            max_name_length: 50,
        }
    }
}

impl ValidationRuleSet {
    /// Create a strict rule set with lower thresholds.
    pub fn strict() -> Self {
        Self {
            complexity_warning: 10,
            complexity_critical: 50,
            max_function_length: 20,
            max_nesting_depth: 2,
            max_file_length: 500,
            min_name_length: 3,
            max_name_length: 30,
        }
    }

    /// Create a lenient rule set with higher thresholds.
    pub fn lenient() -> Self {
        Self {
            complexity_warning: 30,
            complexity_critical: 150,
            max_function_length: 100,
            max_nesting_depth: 6,
            max_file_length: 2000,
            min_name_length: 1,
            max_name_length: 100,
        }
    }

    /// Check if complexity is in the warning zone.
    pub fn is_warning_complexity(&self, complexity: u32) -> bool {
        complexity >= self.complexity_warning && complexity < self.complexity_critical
    }

    /// Check if complexity is critical.
    pub fn is_critical_complexity(&self, complexity: u32) -> bool {
        complexity >= self.complexity_critical
    }

    /// Check if function length is acceptable.
    pub fn is_acceptable_function_length(&self, length: usize) -> bool {
        length <= self.max_function_length
    }

    /// Check if nesting depth is acceptable.
    pub fn is_acceptable_nesting(&self, depth: u32) -> bool {
        depth <= self.max_nesting_depth
    }

    /// Check if file length is acceptable.
    pub fn is_acceptable_file_length(&self, length: usize) -> bool {
        length <= self.max_file_length
    }

    /// Create a complexity predicate for this rule set.
    pub fn complexity_predicate(&self) -> impl Predicate<u32> + '_ {
        use stillwater::predicate::lt;
        lt(self.complexity_critical)
    }

    /// Create a function length predicate for this rule set.
    pub fn function_length_predicate(&self) -> impl Predicate<usize> + '_ {
        use stillwater::predicate::le;
        le(self.max_function_length)
    }

    /// Create a nesting depth predicate for this rule set.
    pub fn nesting_predicate(&self) -> impl Predicate<u32> + '_ {
        use stillwater::predicate::le;
        le(self.max_nesting_depth)
    }

    /// Create a file length predicate for this rule set.
    pub fn file_length_predicate(&self) -> impl Predicate<usize> + '_ {
        use stillwater::predicate::le;
        le(self.max_file_length)
    }
}

// =============================================================================
// Validation Helpers
// =============================================================================

/// Validate a function's complexity using the given rule set.
pub fn validate_function_complexity(
    function_name: &str,
    complexity: u32,
    rules: &ValidationRuleSet,
) -> AnalysisValidation<u32> {
    use stillwater::predicate::lt;

    complexity.ensure(
        lt(rules.complexity_critical),
        AnalysisError::validation(format!(
            "Function '{}' has critical complexity: {} (threshold: {})",
            function_name, complexity, rules.complexity_critical
        )),
    )
}

/// Validate a function's length using the given rule set.
pub fn validate_function_length(
    function_name: &str,
    length: usize,
    rules: &ValidationRuleSet,
) -> AnalysisValidation<usize> {
    use stillwater::predicate::le;

    length.ensure(
        le(rules.max_function_length),
        AnalysisError::validation(format!(
            "Function '{}' is too long: {} lines (max: {})",
            function_name, length, rules.max_function_length
        )),
    )
}

/// Validate a function's nesting depth using the given rule set.
pub fn validate_nesting_depth(
    function_name: &str,
    depth: u32,
    rules: &ValidationRuleSet,
) -> AnalysisValidation<u32> {
    use stillwater::predicate::le;

    depth.ensure(
        le(rules.max_nesting_depth),
        AnalysisError::validation(format!(
            "Function '{}' has excessive nesting: {} levels (max: {})",
            function_name, depth, rules.max_nesting_depth
        )),
    )
}

/// Validate a file's length using the given rule set.
pub fn validate_file_length(
    file_path: &std::path::Path,
    length: usize,
    rules: &ValidationRuleSet,
) -> AnalysisValidation<usize> {
    use stillwater::predicate::le;

    length.ensure(
        le(rules.max_file_length),
        AnalysisError::validation(format!(
            "File '{}' is too long: {} lines (max: {})",
            file_path.display(),
            length,
            rules.max_file_length
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::validation_failure;
    use stillwater::predicate::*;

    // =========================================================================
    // EnsureExt Tests
    // =========================================================================

    #[test]
    fn test_ensure_success() {
        let value: u32 = 50;
        let result = value.ensure(lt(100_u32), AnalysisError::validation("Too high"));
        assert!(result.is_success());
        match result {
            Validation::Success(v) => assert_eq!(v, 50),
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_ensure_failure() {
        let value: u32 = 150;
        let result = value.ensure(lt(100_u32), AnalysisError::validation("Too high"));
        assert!(result.is_failure());
    }

    #[test]
    fn test_ensure_with_error_fn() {
        let value: u32 = 150;
        let result = value.ensure_with(lt(100_u32), |v| {
            AnalysisError::validation(format!("Value {} exceeds limit", v))
        });
        assert!(result.is_failure());
        match result {
            Validation::Failure(errors) => {
                let msg = errors.head().to_string();
                assert!(msg.contains("150"));
            }
            _ => panic!("Expected failure"),
        }
    }

    // =========================================================================
    // Predicate Combinator Tests
    // =========================================================================

    #[test]
    fn test_high_complexity_predicate() {
        let pred = predicates::high_complexity(21, 100);
        assert!(pred.check(&50)); // In warning zone
        assert!(pred.check(&21)); // At lower boundary
        assert!(pred.check(&99)); // Just below critical
        assert!(!pred.check(&20)); // Below warning
        assert!(!pred.check(&100)); // At critical (excluded)
    }

    #[test]
    fn test_critical_complexity_predicate() {
        let pred = predicates::critical_complexity(100);
        assert!(pred.check(&100));
        assert!(pred.check(&150));
        assert!(!pred.check(&99));
    }

    #[test]
    fn test_within_bounds_predicate() {
        let pred = predicates::within_bounds(10, 50);
        assert!(pred.check(&30));
        assert!(pred.check(&10)); // Lower bound inclusive
        assert!(pred.check(&50)); // Upper bound inclusive
        assert!(!pred.check(&9));
        assert!(!pred.check(&51));
    }

    #[test]
    fn test_acceptable_file_length_predicate() {
        let pred = predicates::acceptable_file_length(1000);
        assert!(pred.check(&500));
        assert!(pred.check(&1000));
        assert!(!pred.check(&1001));
    }

    #[test]
    fn test_predicate_composition() {
        // Test AND composition
        let and_pred = ge(10_u32).and(le(20_u32));
        assert!(and_pred.check(&15));
        assert!(!and_pred.check(&5));
        assert!(!and_pred.check(&25));

        // Test OR composition
        let or_pred = lt(10_u32).or(gt(90_u32));
        assert!(or_pred.check(&5));
        assert!(or_pred.check(&95));
        assert!(!or_pred.check(&50));

        // Test NOT composition
        let not_pred = ge(50_u32).not();
        assert!(not_pred.check(&25));
        assert!(!not_pred.check(&75));
    }

    // =========================================================================
    // ValidatedFileResults Tests
    // =========================================================================

    #[test]
    fn test_validated_file_results_all_succeeded() {
        let metrics = create_test_file_metrics();
        let validations = vec![
            validation_success(metrics.clone()),
            validation_success(metrics.clone()),
        ];

        let result = ValidatedFileResults::from_validations(validations);

        assert!(result.is_all_success());
        assert!(!result.has_failures());
        assert_eq!(result.succeeded().len(), 2);
        assert!(result.failures().is_none());
    }

    #[test]
    fn test_validated_file_results_partial_success() {
        let metrics = create_test_file_metrics();
        let validations = vec![
            validation_success(metrics.clone()),
            validation_failure(AnalysisError::parse("Parse error")),
            validation_success(metrics.clone()),
        ];

        let result = ValidatedFileResults::from_validations(validations);

        assert!(!result.is_all_success());
        assert!(result.has_failures());
        assert_eq!(result.succeeded().len(), 2);
        assert!(result.failures().is_some());
        assert_eq!(result.failures().unwrap().len(), 1);
    }

    #[test]
    fn test_validated_file_results_all_failed() {
        let validations: Vec<AnalysisValidation<FileMetrics>> = vec![
            validation_failure(AnalysisError::parse("Error 1")),
            validation_failure(AnalysisError::parse("Error 2")),
        ];

        let result = ValidatedFileResults::from_validations(validations);

        assert!(!result.is_all_success());
        assert!(result.has_failures());
        assert!(result.succeeded().is_empty());
        assert!(result.failures().is_some());
        assert_eq!(result.failures().unwrap().len(), 2);
    }

    #[test]
    fn test_validated_file_results_into_validation() {
        let metrics = create_test_file_metrics();

        // All success
        let all_success = ValidatedFileResults::AllSucceeded(vec![metrics.clone()]);
        assert!(all_success.into_validation().is_success());

        // Partial success becomes failure
        let partial = ValidatedFileResults::PartialSuccess {
            succeeded: vec![metrics.clone()],
            failures: NonEmptyVec::new(AnalysisError::parse("Error"), Vec::new()),
        };
        assert!(partial.into_validation().is_failure());
    }

    #[test]
    fn test_validated_file_results_into_lenient_result() {
        let metrics = create_test_file_metrics();

        // Partial success becomes Ok in lenient mode
        let partial = ValidatedFileResults::PartialSuccess {
            succeeded: vec![metrics.clone()],
            failures: NonEmptyVec::new(AnalysisError::parse("Error"), Vec::new()),
        };
        let result = partial.into_lenient_result();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);

        // All failed becomes Err even in lenient mode
        let all_failed = ValidatedFileResults::AllFailed(NonEmptyVec::new(
            AnalysisError::parse("Error"),
            vec![],
        ));
        assert!(all_failed.into_lenient_result().is_err());
    }

    // =========================================================================
    // ValidationRuleSet Tests
    // =========================================================================

    #[test]
    fn test_validation_rule_set_default() {
        let rules = ValidationRuleSet::default();
        assert_eq!(rules.complexity_warning, 21);
        assert_eq!(rules.complexity_critical, 100);
        assert_eq!(rules.max_function_length, 50);
    }

    #[test]
    fn test_validation_rule_set_strict() {
        let rules = ValidationRuleSet::strict();
        assert!(rules.complexity_warning < ValidationRuleSet::default().complexity_warning);
        assert!(rules.max_function_length < ValidationRuleSet::default().max_function_length);
    }

    #[test]
    fn test_validation_rule_set_lenient() {
        let rules = ValidationRuleSet::lenient();
        assert!(rules.complexity_warning > ValidationRuleSet::default().complexity_warning);
        assert!(rules.max_function_length > ValidationRuleSet::default().max_function_length);
    }

    #[test]
    fn test_validation_rule_set_checks() {
        let rules = ValidationRuleSet::default();

        // Complexity checks
        assert!(!rules.is_warning_complexity(20));
        assert!(rules.is_warning_complexity(50));
        assert!(!rules.is_warning_complexity(100));
        assert!(!rules.is_critical_complexity(99));
        assert!(rules.is_critical_complexity(100));

        // Length checks
        assert!(rules.is_acceptable_function_length(50));
        assert!(!rules.is_acceptable_function_length(51));
        assert!(rules.is_acceptable_nesting(4));
        assert!(!rules.is_acceptable_nesting(5));
    }

    #[test]
    fn test_validation_rule_set_predicates() {
        let rules = ValidationRuleSet::default();

        // Complexity predicate
        let complexity_pred = rules.complexity_predicate();
        assert!(complexity_pred.check(&50));
        assert!(!complexity_pred.check(&100));

        // Function length predicate
        let length_pred = rules.function_length_predicate();
        assert!(length_pred.check(&50));
        assert!(!length_pred.check(&51));
    }

    // =========================================================================
    // Validation Helper Tests
    // =========================================================================

    #[test]
    fn test_validate_function_complexity() {
        let rules = ValidationRuleSet::default();

        let valid = validate_function_complexity("test_fn", 50, &rules);
        assert!(valid.is_success());

        let invalid = validate_function_complexity("complex_fn", 150, &rules);
        assert!(invalid.is_failure());
    }

    #[test]
    fn test_validate_function_length() {
        let rules = ValidationRuleSet::default();

        let valid = validate_function_length("short_fn", 30, &rules);
        assert!(valid.is_success());

        let invalid = validate_function_length("long_fn", 100, &rules);
        assert!(invalid.is_failure());
    }

    #[test]
    fn test_validate_nesting_depth() {
        let rules = ValidationRuleSet::default();

        let valid = validate_nesting_depth("shallow_fn", 2, &rules);
        assert!(valid.is_success());

        let invalid = validate_nesting_depth("deep_fn", 10, &rules);
        assert!(invalid.is_failure());
    }

    #[test]
    fn test_validate_file_length() {
        let rules = ValidationRuleSet::default();
        let path = std::path::Path::new("test.rs");

        let valid = validate_file_length(path, 500, &rules);
        assert!(valid.is_success());

        let invalid = validate_file_length(path, 2000, &rules);
        assert!(invalid.is_failure());
    }

    // =========================================================================
    // Helper Functions
    // =========================================================================

    fn create_test_file_metrics() -> FileMetrics {
        use crate::core::{ComplexityMetrics, Language};
        FileMetrics {
            path: std::path::PathBuf::from("test.rs"),
            language: Language::Rust,
            complexity: ComplexityMetrics::default(),
            debt_items: Vec::new(),
            dependencies: Vec::new(),
            duplications: Vec::new(),
            total_lines: 100,
            module_scope: None,
            classes: None,
        }
    }
}
