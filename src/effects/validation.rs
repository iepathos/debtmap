//! Expanded validation patterns using stillwater's predicate combinators.
//!
//! This module provides validation patterns for:
//! - Analysis results validation with error accumulation
//! - Predicate-based debt detection rules
//! - File processing validation with partial success semantics
//! - Field context for structured error reporting (Spec 003)
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
//! # Field Context (Spec 003)
//!
//! The module provides types for attaching field context to validation errors:
//!
//! ```rust,ignore
//! use debtmap::effects::validation::{FieldPath, ValidationError};
//!
//! // Create a nested field path
//! let path = FieldPath::root()
//!     .push("config")
//!     .push("thresholds")
//!     .push("cyclomatic");
//!
//! // Create error with field context
//! let error = ValidationError::at_field(&path, "must be greater than zero")
//!     .with_context("positive integer", "-5");
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
//!
//! # ValidatedFileSet (Spec 003)
//!
//! An alternative to ValidatedFileResults that separates valid files from errors
//! while supporting partial success with file-specific error context:
//!
//! ```rust,ignore
//! use debtmap::effects::validation::ValidatedFileSet;
//!
//! let file_set = parse_all_files(&file_paths);
//! if file_set.is_partial_success() {
//!     println!("Processed {} files with {} errors",
//!         file_set.valid.len(), file_set.errors.len());
//! }
//! ```

use crate::core::FileMetrics;
use crate::effects::{validation_success, AnalysisValidation};
use crate::errors::AnalysisError;
use serde::Serialize;
use std::path::PathBuf;
use stillwater::predicate::Predicate;
use stillwater::refined::{FieldError, ValidationFieldExt};
use stillwater::{NonEmptyVec, Validation};

// =============================================================================
// Field Context Types (Spec 003)
// =============================================================================

/// Nested field path for error context.
///
/// Tracks the path from root to a specific field in a configuration or data
/// structure, enabling precise error messages like "config.thresholds.cyclomatic".
///
/// # Example
///
/// ```rust
/// use debtmap::effects::validation::FieldPath;
///
/// let path = FieldPath::root()
///     .push("config")
///     .push("thresholds")
///     .push("cyclomatic");
///
/// assert_eq!(path.as_string(), "config.thresholds.cyclomatic");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct FieldPath(Vec<String>);

impl FieldPath {
    /// Create an empty root path.
    pub fn root() -> Self {
        Self(Vec::new())
    }

    /// Create a path with a single field.
    pub fn new(field: impl Into<String>) -> Self {
        Self(vec![field.into()])
    }

    /// Add a field to the path, returning a new path.
    pub fn push(&self, field: impl Into<String>) -> Self {
        let mut path = self.0.clone();
        path.push(field.into());
        Self(path)
    }

    /// Get the path as a dot-separated string.
    pub fn as_string(&self) -> String {
        self.0.join(".")
    }

    /// Check if this is the root path (no fields).
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the number of segments in the path.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the path is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the last segment of the path, if any.
    pub fn last(&self) -> Option<&str> {
        self.0.last().map(|s| s.as_str())
    }

    /// Get the segments of the path.
    pub fn segments(&self) -> &[String] {
        &self.0
    }
}

impl std::fmt::Display for FieldPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

impl From<&str> for FieldPath {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for FieldPath {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Validation error with full field context.
///
/// Provides structured error information including:
/// - The field path where the error occurred
/// - A human-readable error message
/// - Optional expected and actual values for debugging
///
/// This type is JSON-serializable for IDE integration and tooling.
///
/// # Example
///
/// ```rust
/// use debtmap::effects::validation::{FieldPath, ValidationError};
///
/// let error = ValidationError::at_field(
///     &FieldPath::new("threshold"),
///     "must be greater than zero"
/// ).with_context("positive integer", "-5");
///
/// assert_eq!(error.field.as_string(), "threshold");
/// assert_eq!(error.expected, Some("positive integer".to_string()));
/// assert_eq!(error.actual, Some("-5".to_string()));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ValidationError {
    /// The field path where the error occurred.
    pub field: FieldPath,
    /// Human-readable error message.
    pub message: String,
    /// Expected value or constraint (for debugging).
    pub expected: Option<String>,
    /// Actual value that failed validation (for debugging).
    pub actual: Option<String>,
}

impl ValidationError {
    /// Create a validation error at a specific field.
    pub fn at_field(field: &FieldPath, message: impl Into<String>) -> Self {
        Self {
            field: field.clone(),
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    /// Create a validation error with a simple field name.
    pub fn for_field(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: FieldPath::new(field),
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    /// Add expected and actual context to the error.
    pub fn with_context(mut self, expected: impl Into<String>, actual: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self.actual = Some(actual.into());
        self
    }

    /// Add expected value context.
    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self
    }

    /// Add actual value context.
    pub fn with_actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = Some(actual.into());
        self
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.field.is_root() {
            write!(f, "{}", self.message)?;
        } else {
            write!(f, "{}: {}", self.field, self.message)?;
        }

        if let (Some(expected), Some(actual)) = (&self.expected, &self.actual) {
            write!(f, " (expected: {}, got: {})", expected, actual)?;
        } else if let Some(expected) = &self.expected {
            write!(f, " (expected: {})", expected)?;
        } else if let Some(actual) = &self.actual {
            write!(f, " (got: {})", actual)?;
        }

        Ok(())
    }
}

impl std::error::Error for ValidationError {}

/// Error for a specific file with location context.
///
/// Tracks file path and optional line/column information for errors
/// that occur during file processing (parsing, analysis, etc.).
///
/// # Example
///
/// ```rust
/// use debtmap::effects::validation::FileError;
/// use std::path::PathBuf;
///
/// let error = FileError::new(
///     PathBuf::from("src/main.rs"),
///     "unexpected token 'foo'"
/// ).at_location(42, 15);
///
/// assert_eq!(error.path, PathBuf::from("src/main.rs"));
/// assert_eq!(error.line, Some(42));
/// assert_eq!(error.column, Some(15));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileError {
    /// Path to the file where the error occurred.
    pub path: PathBuf,
    /// Line number where the error occurred (1-indexed).
    pub line: Option<u32>,
    /// Column number where the error occurred (1-indexed).
    pub column: Option<u32>,
    /// Human-readable error message.
    pub message: String,
    /// Optional error code for programmatic handling.
    pub error_code: Option<String>,
}

impl FileError {
    /// Create a new file error with path and message.
    pub fn new(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line: None,
            column: None,
            message: message.into(),
            error_code: None,
        }
    }

    /// Add line and column location information.
    pub fn at_location(mut self, line: u32, column: u32) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    /// Add just line location information.
    pub fn at_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    /// Add an error code for programmatic handling.
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.error_code = Some(code.into());
        self
    }

    /// Convert from a parse error with context.
    pub fn from_parse_error(path: impl Into<PathBuf>, error: impl std::fmt::Display) -> Self {
        Self::new(path, error.to_string()).with_code("E010")
    }

    /// Convert from an AnalysisError with path context.
    pub fn from_analysis_error(path: impl Into<PathBuf>, error: &AnalysisError) -> Self {
        let path = path.into();
        let message = error.to_string();

        // Extract line number from the error if it's a parse error
        let line = if let AnalysisError::ParseError { line, .. } = error {
            *line
        } else {
            None
        };

        let mut file_error = Self::new(path, message);
        if let Some(l) = line {
            file_error.line = Some(l as u32);
        }

        file_error
    }
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())?;
        if let Some(line) = self.line {
            write!(f, ":{}", line)?;
            if let Some(column) = self.column {
                write!(f, ":{}", column)?;
            }
        }
        write!(f, ": {}", self.message)?;
        if let Some(code) = &self.error_code {
            write!(f, " [{}]", code)?;
        }
        Ok(())
    }
}

impl std::error::Error for FileError {}

impl From<FileError> for AnalysisError {
    fn from(err: FileError) -> Self {
        AnalysisError::parse_with_path(&err.message, &err.path)
    }
}

/// Result of validating multiple files with partial success semantics.
///
/// Unlike `ValidatedFileResults`, this type uses generic file data and
/// provides richer error information with `FileError` instead of `AnalysisError`.
///
/// # Example
///
/// ```rust
/// use debtmap::effects::validation::{ValidatedFileSet, FileError};
/// use std::path::PathBuf;
///
/// // Create a partial success result
/// let file_set = ValidatedFileSet {
///     valid: vec!["file1 content".to_string(), "file2 content".to_string()],
///     errors: vec![FileError::new(PathBuf::from("bad.rs"), "parse error")],
/// };
///
/// assert!(file_set.is_partial_success());
/// assert_eq!(file_set.valid.len(), 2);
/// assert_eq!(file_set.errors.len(), 1);
/// ```
#[derive(Clone, Debug, Default)]
pub struct ValidatedFileSet<T> {
    /// Successfully processed files.
    pub valid: Vec<T>,
    /// Files that failed to process with their errors.
    pub errors: Vec<FileError>,
}

impl<T> ValidatedFileSet<T> {
    /// Create an empty file set.
    pub fn empty() -> Self {
        Self {
            valid: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Create a file set with only valid files.
    pub fn all_valid(valid: Vec<T>) -> Self {
        Self {
            valid,
            errors: Vec::new(),
        }
    }

    /// Create a file set with only errors.
    pub fn all_errors(errors: Vec<FileError>) -> Self {
        Self {
            valid: Vec::new(),
            errors,
        }
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if there are any valid files.
    pub fn has_valid(&self) -> bool {
        !self.valid.is_empty()
    }

    /// Check if this is a partial success (some valid, some errors).
    pub fn is_partial_success(&self) -> bool {
        self.has_valid() && self.has_errors()
    }

    /// Check if all files succeeded.
    pub fn is_all_success(&self) -> bool {
        self.has_valid() && !self.has_errors()
    }

    /// Check if all files failed.
    pub fn is_all_failed(&self) -> bool {
        !self.has_valid() && self.has_errors()
    }

    /// Get the number of successfully processed files.
    pub fn valid_count(&self) -> usize {
        self.valid.len()
    }

    /// Get the number of errors.
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Convert to a strict Result (any error = failure).
    pub fn into_strict_result(self) -> Result<Vec<T>, Vec<FileError>> {
        if self.errors.is_empty() {
            Ok(self.valid)
        } else {
            Err(self.errors)
        }
    }

    /// Convert to a lenient Result (only fail if all files failed).
    pub fn into_lenient_result(self) -> Result<Vec<T>, Vec<FileError>> {
        if self.valid.is_empty() && !self.errors.is_empty() {
            Err(self.errors)
        } else {
            Ok(self.valid)
        }
    }

    /// Add a valid file to the set.
    pub fn add_valid(&mut self, item: T) {
        self.valid.push(item);
    }

    /// Add an error to the set.
    pub fn add_error(&mut self, error: FileError) {
        self.errors.push(error);
    }

    /// Merge another file set into this one.
    pub fn merge(&mut self, other: ValidatedFileSet<T>) {
        self.valid.extend(other.valid);
        self.errors.extend(other.errors);
    }
}

impl<T: Serialize> Serialize for ValidatedFileSet<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("ValidatedFileSet", 4)?;
        state.serialize_field("valid_count", &self.valid.len())?;
        state.serialize_field("error_count", &self.errors.len())?;
        state.serialize_field("valid", &self.valid)?;
        state.serialize_field("errors", &self.errors)?;
        state.end()
    }
}

// =============================================================================
// Field Context Extension Traits
// =============================================================================

/// Extension trait for adding field context to validations.
///
/// This trait extends stillwater's `Validation` type with methods for
/// attaching field paths to errors.
pub trait FieldContextExt<T, E> {
    /// Attach a field path to validation errors.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let validated = validate_threshold(config.threshold)
    ///     .with_field_path(&FieldPath::new("config.threshold"));
    /// ```
    fn with_field_path(self, path: &FieldPath) -> Validation<T, NonEmptyVec<ValidationError>>
    where
        E: std::fmt::Display;

    /// Attach a simple field name to validation errors.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let validated = validate_threshold(value)
    ///     .with_field_name("threshold");
    /// ```
    fn with_field_name(self, field: &str) -> Validation<T, NonEmptyVec<ValidationError>>
    where
        E: std::fmt::Display;
}

impl<T, E> FieldContextExt<T, E> for Validation<T, NonEmptyVec<E>> {
    fn with_field_path(self, path: &FieldPath) -> Validation<T, NonEmptyVec<ValidationError>>
    where
        E: std::fmt::Display,
    {
        match self {
            Validation::Success(value) => Validation::Success(value),
            Validation::Failure(errors) => {
                let field_errors: Vec<ValidationError> = errors
                    .into_iter()
                    .map(|e| ValidationError::at_field(path, e.to_string()))
                    .collect();
                Validation::Failure(
                    NonEmptyVec::from_vec(field_errors).expect("errors came from non-empty vec"),
                )
            }
        }
    }

    fn with_field_name(self, field: &str) -> Validation<T, NonEmptyVec<ValidationError>>
    where
        E: std::fmt::Display,
    {
        self.with_field_path(&FieldPath::new(field))
    }
}

// Re-export stillwater's field types for convenience
pub use stillwater::refined::FieldError as StillwaterFieldError;

/// Create a validation that wraps errors with field context using stillwater's FieldError.
pub fn validate_field<T, E>(
    field: &'static str,
    validation: Validation<T, E>,
) -> Validation<T, FieldError<E>> {
    validation.with_field(field)
}

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
    // Field Context Tests (Spec 003)
    // =========================================================================

    #[test]
    fn test_field_path_root() {
        let path = FieldPath::root();
        assert!(path.is_root());
        assert!(path.is_empty());
        assert_eq!(path.len(), 0);
        assert_eq!(path.as_string(), "");
    }

    #[test]
    fn test_field_path_single() {
        let path = FieldPath::new("config");
        assert!(!path.is_root());
        assert_eq!(path.len(), 1);
        assert_eq!(path.as_string(), "config");
        assert_eq!(path.last(), Some("config"));
    }

    #[test]
    fn test_field_path_nested() {
        let path = FieldPath::root()
            .push("config")
            .push("thresholds")
            .push("cyclomatic");
        assert_eq!(path.len(), 3);
        assert_eq!(path.as_string(), "config.thresholds.cyclomatic");
        assert_eq!(path.last(), Some("cyclomatic"));
        assert_eq!(path.segments(), &["config", "thresholds", "cyclomatic"]);
    }

    #[test]
    fn test_field_path_display() {
        let path = FieldPath::new("config").push("value");
        assert_eq!(format!("{}", path), "config.value");
    }

    #[test]
    fn test_field_path_from_str() {
        let path: FieldPath = "config".into();
        assert_eq!(path.as_string(), "config");
    }

    #[test]
    fn test_validation_error_at_field() {
        let path = FieldPath::new("threshold");
        let error = ValidationError::at_field(&path, "must be positive");
        assert_eq!(error.field.as_string(), "threshold");
        assert_eq!(error.message, "must be positive");
        assert!(error.expected.is_none());
        assert!(error.actual.is_none());
    }

    #[test]
    fn test_validation_error_for_field() {
        let error = ValidationError::for_field("coverage", "out of range");
        assert_eq!(error.field.as_string(), "coverage");
        assert_eq!(error.message, "out of range");
    }

    #[test]
    fn test_validation_error_with_context() {
        let error = ValidationError::for_field("threshold", "invalid value")
            .with_context("positive integer", "-5");
        assert_eq!(error.expected, Some("positive integer".to_string()));
        assert_eq!(error.actual, Some("-5".to_string()));
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::for_field("config.threshold", "must be positive")
            .with_context("positive", "negative");
        let display = format!("{}", error);
        assert!(display.contains("config.threshold"));
        assert!(display.contains("must be positive"));
        assert!(display.contains("expected: positive"));
        assert!(display.contains("got: negative"));
    }

    #[test]
    fn test_validation_error_display_no_context() {
        let error = ValidationError::for_field("name", "required");
        assert_eq!(format!("{}", error), "name: required");
    }

    #[test]
    fn test_validation_error_display_root_path() {
        let error = ValidationError::at_field(&FieldPath::root(), "general error");
        assert_eq!(format!("{}", error), "general error");
    }

    #[test]
    fn test_validation_error_serialization() {
        let error = ValidationError::for_field("threshold", "invalid").with_context(">=0", "-1");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"field\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"expected\""));
        assert!(json.contains("\"actual\""));
    }

    #[test]
    fn test_file_error_new() {
        let error = FileError::new(PathBuf::from("src/main.rs"), "parse error");
        assert_eq!(error.path, PathBuf::from("src/main.rs"));
        assert_eq!(error.message, "parse error");
        assert!(error.line.is_none());
        assert!(error.column.is_none());
        assert!(error.error_code.is_none());
    }

    #[test]
    fn test_file_error_at_location() {
        let error =
            FileError::new(PathBuf::from("test.rs"), "unexpected token").at_location(42, 15);
        assert_eq!(error.line, Some(42));
        assert_eq!(error.column, Some(15));
    }

    #[test]
    fn test_file_error_at_line() {
        let error = FileError::new(PathBuf::from("test.rs"), "missing semicolon").at_line(10);
        assert_eq!(error.line, Some(10));
        assert!(error.column.is_none());
    }

    #[test]
    fn test_file_error_with_code() {
        let error = FileError::new(PathBuf::from("test.rs"), "syntax error").with_code("E010");
        assert_eq!(error.error_code, Some("E010".to_string()));
    }

    #[test]
    fn test_file_error_display() {
        let error = FileError::new(PathBuf::from("src/lib.rs"), "unexpected eof")
            .at_location(100, 25)
            .with_code("E010");
        let display = format!("{}", error);
        assert!(display.contains("src/lib.rs"));
        assert!(display.contains(":100:25"));
        assert!(display.contains("unexpected eof"));
        assert!(display.contains("[E010]"));
    }

    #[test]
    fn test_file_error_display_no_location() {
        let error = FileError::new(PathBuf::from("test.rs"), "general error");
        let display = format!("{}", error);
        assert_eq!(display, "test.rs: general error");
    }

    #[test]
    fn test_file_error_serialization() {
        let error = FileError::new(PathBuf::from("test.rs"), "error")
            .at_location(10, 5)
            .with_code("E001");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"path\""));
        assert!(json.contains("\"line\""));
        assert!(json.contains("\"column\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"error_code\""));
    }

    #[test]
    fn test_validated_file_set_empty() {
        let set: ValidatedFileSet<String> = ValidatedFileSet::empty();
        assert!(!set.has_valid());
        assert!(!set.has_errors());
        assert!(!set.is_partial_success());
        assert!(!set.is_all_success());
        assert!(!set.is_all_failed());
    }

    #[test]
    fn test_validated_file_set_all_valid() {
        let set = ValidatedFileSet::all_valid(vec!["file1".to_string(), "file2".to_string()]);
        assert!(set.has_valid());
        assert!(!set.has_errors());
        assert!(set.is_all_success());
        assert!(!set.is_partial_success());
        assert!(!set.is_all_failed());
        assert_eq!(set.valid_count(), 2);
        assert_eq!(set.error_count(), 0);
    }

    #[test]
    fn test_validated_file_set_all_errors() {
        let set: ValidatedFileSet<String> = ValidatedFileSet::all_errors(vec![
            FileError::new("a.rs", "error1"),
            FileError::new("b.rs", "error2"),
        ]);
        assert!(!set.has_valid());
        assert!(set.has_errors());
        assert!(set.is_all_failed());
        assert!(!set.is_partial_success());
        assert!(!set.is_all_success());
        assert_eq!(set.valid_count(), 0);
        assert_eq!(set.error_count(), 2);
    }

    #[test]
    fn test_validated_file_set_partial_success() {
        let set = ValidatedFileSet {
            valid: vec!["good.rs".to_string()],
            errors: vec![FileError::new("bad.rs", "parse error")],
        };
        assert!(set.has_valid());
        assert!(set.has_errors());
        assert!(set.is_partial_success());
        assert!(!set.is_all_success());
        assert!(!set.is_all_failed());
    }

    #[test]
    fn test_validated_file_set_into_strict_result() {
        let success_set = ValidatedFileSet::all_valid(vec!["ok".to_string()]);
        assert!(success_set.into_strict_result().is_ok());

        let partial_set = ValidatedFileSet {
            valid: vec!["ok".to_string()],
            errors: vec![FileError::new("bad.rs", "error")],
        };
        assert!(partial_set.into_strict_result().is_err());
    }

    #[test]
    fn test_validated_file_set_into_lenient_result() {
        let partial_set = ValidatedFileSet {
            valid: vec!["ok".to_string()],
            errors: vec![FileError::new("bad.rs", "error")],
        };
        assert!(partial_set.into_lenient_result().is_ok());

        let all_failed: ValidatedFileSet<String> =
            ValidatedFileSet::all_errors(vec![FileError::new("bad.rs", "error")]);
        assert!(all_failed.into_lenient_result().is_err());
    }

    #[test]
    fn test_validated_file_set_add_operations() {
        let mut set: ValidatedFileSet<String> = ValidatedFileSet::empty();
        set.add_valid("file1".to_string());
        set.add_error(FileError::new("bad.rs", "error"));
        assert!(set.is_partial_success());
        assert_eq!(set.valid_count(), 1);
        assert_eq!(set.error_count(), 1);
    }

    #[test]
    fn test_validated_file_set_merge() {
        let mut set1: ValidatedFileSet<String> = ValidatedFileSet::all_valid(vec!["a".to_string()]);
        let set2 = ValidatedFileSet {
            valid: vec!["b".to_string()],
            errors: vec![FileError::new("c.rs", "error")],
        };
        set1.merge(set2);
        assert_eq!(set1.valid_count(), 2);
        assert_eq!(set1.error_count(), 1);
    }

    #[test]
    fn test_validated_file_set_serialization() {
        let set = ValidatedFileSet {
            valid: vec!["file1".to_string()],
            errors: vec![FileError::new("bad.rs", "error")],
        };
        let json = serde_json::to_string(&set).unwrap();
        assert!(json.contains("\"valid_count\":1"));
        assert!(json.contains("\"error_count\":1"));
        assert!(json.contains("\"valid\""));
        assert!(json.contains("\"errors\""));
    }

    #[test]
    fn test_field_context_ext_with_field_path() {
        let validation: Validation<u32, NonEmptyVec<String>> =
            Validation::Failure(NonEmptyVec::new("error message".to_string(), vec![]));

        let path = FieldPath::new("config").push("threshold");
        let result = validation.with_field_path(&path);

        match result {
            Validation::Failure(errors) => {
                let err = errors.head();
                assert_eq!(err.field.as_string(), "config.threshold");
                assert!(err.message.contains("error message"));
            }
            _ => panic!("Expected failure"),
        }
    }

    #[test]
    fn test_field_context_ext_with_field_name() {
        let validation: Validation<u32, NonEmptyVec<String>> =
            Validation::Failure(NonEmptyVec::new("too large".to_string(), vec![]));

        let result = validation.with_field_name("complexity");

        match result {
            Validation::Failure(errors) => {
                let err = errors.head();
                assert_eq!(err.field.as_string(), "complexity");
            }
            _ => panic!("Expected failure"),
        }
    }

    #[test]
    fn test_field_context_ext_success_passthrough() {
        let validation: Validation<u32, NonEmptyVec<String>> = Validation::Success(42);
        let result = validation.with_field_name("value");

        match result {
            Validation::Success(v) => assert_eq!(v, 42),
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_validate_field_with_stillwater() {
        // Test that we can use stillwater's with_field via our wrapper
        let validation: Validation<u32, String> = Validation::Failure("test error".to_string());
        let result = validate_field("my_field", validation);

        match result {
            Validation::Failure(field_error) => {
                assert_eq!(field_error.field, "my_field");
                assert_eq!(field_error.error, "test error");
            }
            _ => panic!("Expected failure"),
        }
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
