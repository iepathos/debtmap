//! Effect type aliases and helpers for debtmap analysis.
//!
//! This module provides type aliases that integrate stillwater's effect system
//! with debtmap's environment and error types. Using these aliases:
//!
//! - Reduces boilerplate in function signatures
//! - Centralizes the environment and error types
//! - Makes it easy to refactor if types change
//!
//! # Effect vs Validation
//!
//! - **Effect**: Represents a computation that may perform I/O and may fail.
//!   Use for operations like reading files or loading coverage data.
//!
//! - **Validation**: Represents a validation check that accumulates ALL errors
//!   instead of failing at the first one. Use for configuration validation,
//!   input checking, and anywhere you want comprehensive error reporting.
//!
//! # Example: Using Effects
//!
//! ```rust,ignore
//! use debtmap::effects::AnalysisEffect;
//! use debtmap::env::AnalysisEnv;
//! use stillwater::Effect;
//!
//! fn read_source(path: PathBuf) -> AnalysisEffect<String> {
//!     Effect::from_fn(move |env: &dyn AnalysisEnv| {
//!         env.file_system()
//!             .read_to_string(&path)
//!             .map_err(Into::into)
//!     })
//! }
//! ```
//!
//! # Example: Using Validation
//!
//! ```rust,ignore
//! use debtmap::effects::{AnalysisValidation, validation_success, validation_failure};
//!
//! fn validate_thresholds(complexity: u32, lines: usize) -> AnalysisValidation<()> {
//!     let v1 = if complexity <= 50 {
//!         validation_success(())
//!     } else {
//!         validation_failure(AnalysisError::validation("Complexity too high"))
//!     };
//!
//!     let v2 = if lines <= 1000 {
//!         validation_success(())
//!     } else {
//!         validation_failure(AnalysisError::validation("File too long"))
//!     };
//!
//!     // Combine validations - collects ALL errors
//!     v1.and(v2)
//! }
//! ```

use crate::config::DebtmapConfig;
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::{errors_to_anyhow, AnalysisError};
use stillwater::effect::prelude::*;
use stillwater::{BoxedEffect, NonEmptyVec, Validation};

/// Error collection type for validation accumulation.
///
/// This type holds multiple errors during validation, enabling comprehensive
/// error reporting instead of failing at the first error.
pub type AnalysisErrors = NonEmptyVec<AnalysisError>;

/// Effect type for debtmap analysis operations.
///
/// This type alias parameterizes stillwater's Effect with:
/// - Success type `T` (the computation result)
/// - Error type `AnalysisError` (our unified error type)
/// - Environment type `RealEnv` (production I/O capabilities)
///
/// # Usage
///
/// ```rust,ignore
/// fn analyze_file(path: PathBuf) -> AnalysisEffect<FileMetrics> {
///     Effect::from_fn(move |env| {
///         let content = env.file_system().read_to_string(&path)?;
///         let metrics = compute_metrics(&content);
///         Ok(metrics)
///     })
/// }
/// ```
pub type AnalysisEffect<T> = BoxedEffect<T, AnalysisError, RealEnv>;

/// Validation type for debtmap validations.
///
/// This type alias uses stillwater's Validation with:
/// - Success type `T` (the validated value)
/// - Error type `NonEmptyVec<AnalysisError>` (accumulated errors)
///
/// Unlike Result, Validation accumulates ALL errors instead of short-circuiting
/// at the first failure. This is useful for:
/// - Configuration validation (report all issues at once)
/// - Input validation (show all problems to the user)
/// - Analysis validation (collect all findings)
///
/// # Usage
///
/// ```rust
/// use debtmap::effects::{AnalysisValidation, validation_success, validation_failure};
/// use debtmap::errors::AnalysisError;
///
/// fn validate_name(name: &str) -> AnalysisValidation<String> {
///     if name.is_empty() {
///         validation_failure(AnalysisError::validation("Name cannot be empty"))
///     } else {
///         validation_success(name.to_string())
///     }
/// }
/// ```
pub type AnalysisValidation<T> = Validation<T, AnalysisErrors>;

/// Create a successful validation result.
///
/// # Example
///
/// ```rust
/// use debtmap::effects::validation_success;
///
/// let v: debtmap::effects::AnalysisValidation<i32> = validation_success(42);
/// assert!(v.is_success());
/// ```
pub fn validation_success<T>(value: T) -> AnalysisValidation<T> {
    Validation::Success(value)
}

/// Create a failed validation result with a single error.
///
/// # Example
///
/// ```rust
/// use debtmap::effects::validation_failure;
/// use debtmap::errors::AnalysisError;
///
/// let v: debtmap::effects::AnalysisValidation<i32> =
///     validation_failure(AnalysisError::validation("Invalid input"));
/// assert!(v.is_failure());
/// ```
pub fn validation_failure<T>(error: AnalysisError) -> AnalysisValidation<T> {
    Validation::Failure(NonEmptyVec::new(error, Vec::new()))
}

/// Create a failed validation result with multiple errors.
///
/// # Panics
///
/// Panics if the errors vector is empty. Use `validation_failure` for
/// single errors or ensure the vector is non-empty.
///
/// # Example
///
/// ```rust
/// use debtmap::effects::validation_failures;
/// use debtmap::errors::AnalysisError;
///
/// let errors = vec![
///     AnalysisError::validation("Error 1"),
///     AnalysisError::validation("Error 2"),
/// ];
/// let v: debtmap::effects::AnalysisValidation<i32> = validation_failures(errors);
/// ```
pub fn validation_failures<T>(errors: Vec<AnalysisError>) -> AnalysisValidation<T> {
    let nev =
        NonEmptyVec::from_vec(errors).expect("validation_failures requires at least one error");
    Validation::Failure(nev)
}

/// Create an effect from a pure value (no I/O).
///
/// This is useful for wrapping pure computations in the effect system.
///
/// # Example
///
/// ```rust,ignore
/// let effect = effect_pure(42);
/// assert_eq!(effect.run(&env).unwrap(), 42);
/// ```
pub fn effect_pure<T: Send + 'static>(value: T) -> AnalysisEffect<T> {
    pure(value).boxed()
}

/// Create an effect from an error.
///
/// This is useful for creating failing effects without needing I/O.
///
/// # Example
///
/// ```rust,ignore
/// let effect: AnalysisEffect<i32> = effect_fail(AnalysisError::validation("bad input"));
/// assert!(effect.run(&env).is_err());
/// ```
pub fn effect_fail<T: Send + 'static>(error: AnalysisError) -> AnalysisEffect<T> {
    fail(error).boxed()
}

/// Create an effect from a synchronous function.
///
/// The function receives the environment and should return a Result.
///
/// # Example
///
/// ```rust,ignore
/// fn read_config() -> AnalysisEffect<Config> {
///     effect_from_fn(|env| {
///         let content = env.file_system().read_to_string(Path::new("config.toml"))?;
///         parse_config(&content).map_err(Into::into)
///     })
/// }
/// ```
pub fn effect_from_fn<T, F>(f: F) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce(&RealEnv) -> Result<T, AnalysisError> + Send + 'static,
{
    from_fn(f).boxed()
}

/// Run an effect and convert the result to anyhow::Result for backwards compatibility.
///
/// This function bridges the new effect system with existing code that uses
/// anyhow::Result. Use this at the boundaries of your code where you need
/// to integrate with existing APIs.
///
/// Note: This uses tokio's block_on to run the async effect synchronously.
/// For better performance in async contexts, use `run_effect_async` instead.
///
/// # Example
///
/// ```rust,ignore
/// // Old code using anyhow
/// fn old_api() -> anyhow::Result<Metrics> {
///     let config = DebtmapConfig::default();
///     run_effect(analyze_effect(), config)
/// }
/// ```
pub fn run_effect<T: Send + 'static>(
    effect: AnalysisEffect<T>,
    config: DebtmapConfig,
) -> anyhow::Result<T> {
    let env = RealEnv::new(config);
    // Use tokio runtime to block on the async effect
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?;
    rt.block_on(effect.run(&env)).map_err(Into::into)
}

/// Run an effect with a custom environment.
///
/// This is useful when you have an existing environment or need custom
/// I/O implementations.
///
/// Note: This uses tokio's block_on to run the async effect synchronously.
pub fn run_effect_with_env<T: Send + 'static, E: AnalysisEnv + Sync + 'static>(
    effect: BoxedEffect<T, AnalysisError, E>,
    env: &E,
) -> anyhow::Result<T> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?;
    rt.block_on(effect.run(env)).map_err(Into::into)
}

/// Run an effect asynchronously.
///
/// This is the preferred method when you're already in an async context.
pub async fn run_effect_async<T: Send + 'static>(
    effect: AnalysisEffect<T>,
    config: DebtmapConfig,
) -> anyhow::Result<T> {
    let env = RealEnv::new(config);
    effect.run(&env).await.map_err(Into::into)
}

/// Convert a Validation result to anyhow::Result for backwards compatibility.
///
/// If the validation failed with multiple errors, they are formatted as a list.
///
/// # Example
///
/// ```rust
/// use debtmap::effects::{run_validation, validation_success, validation_failure};
/// use debtmap::errors::AnalysisError;
///
/// let success = validation_success(42);
/// assert_eq!(run_validation(success).unwrap(), 42);
///
/// let failure: debtmap::effects::AnalysisValidation<i32> =
///     validation_failure(AnalysisError::validation("bad input"));
/// assert!(run_validation(failure).is_err());
/// ```
pub fn run_validation<T>(validation: AnalysisValidation<T>) -> anyhow::Result<T> {
    match validation {
        Validation::Success(value) => Ok(value),
        Validation::Failure(errors) => Err(errors_to_anyhow(errors.into_vec())),
    }
}

/// Combine multiple validations, accumulating all errors.
///
/// This is the core of error accumulation - if any validation fails,
/// all errors are collected. If all succeed, the results are collected.
///
/// # Example
///
/// ```rust
/// use debtmap::effects::{combine_validations, validation_success, validation_failure};
/// use debtmap::errors::AnalysisError;
///
/// let validations = vec![
///     validation_success(1),
///     validation_failure(AnalysisError::validation("error 1")),
///     validation_success(3),
///     validation_failure(AnalysisError::validation("error 2")),
/// ];
///
/// let result = combine_validations(validations);
/// // Result contains BOTH errors, not just the first one
/// ```
pub fn combine_validations<T>(validations: Vec<AnalysisValidation<T>>) -> AnalysisValidation<Vec<T>>
where
    T: Clone,
{
    let mut successes = Vec::new();
    let mut failures: Vec<AnalysisError> = Vec::new();

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

/// Map a function over a validation's success value.
///
/// If the validation is successful, applies the function.
/// If it failed, passes through the errors unchanged.
pub fn validation_map<T, U, F>(validation: AnalysisValidation<T>, f: F) -> AnalysisValidation<U>
where
    F: FnOnce(T) -> U,
{
    match validation {
        Validation::Success(value) => Validation::Success(f(value)),
        Validation::Failure(errors) => Validation::Failure(errors),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_success() {
        let v: AnalysisValidation<i32> = validation_success(42);
        assert!(v.is_success());
        match v {
            Validation::Success(n) => assert_eq!(n, 42),
            Validation::Failure(_) => panic!("Expected success"),
        }
    }

    #[test]
    fn test_validation_failure() {
        let v: AnalysisValidation<i32> =
            validation_failure(AnalysisError::validation("test error"));
        assert!(v.is_failure());
    }

    #[test]
    fn test_validation_failures() {
        let errors = vec![
            AnalysisError::validation("error 1"),
            AnalysisError::validation("error 2"),
        ];
        let v: AnalysisValidation<i32> = validation_failures(errors);
        assert!(v.is_failure());
        match v {
            Validation::Failure(nev) => {
                let vec: Vec<_> = nev.into_iter().collect();
                assert_eq!(vec.len(), 2);
            }
            _ => panic!("Expected failure"),
        }
    }

    #[test]
    fn test_run_validation_success() {
        let v = validation_success(42);
        let result = run_validation(v);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_run_validation_failure() {
        let v: AnalysisValidation<i32> =
            validation_failure(AnalysisError::validation("test error"));
        let result = run_validation(v);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("test error"));
    }

    #[test]
    fn test_combine_validations_all_success() {
        let validations = vec![
            validation_success(1),
            validation_success(2),
            validation_success(3),
        ];
        let result = combine_validations(validations);
        match result {
            Validation::Success(values) => assert_eq!(values, vec![1, 2, 3]),
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_combine_validations_accumulates_errors() {
        let validations = vec![
            validation_success(1),
            validation_failure(AnalysisError::validation("error 1")),
            validation_success(3),
            validation_failure(AnalysisError::validation("error 2")),
        ];
        let result: AnalysisValidation<Vec<i32>> = combine_validations(validations);
        match result {
            Validation::Failure(errors) => {
                let vec: Vec<_> = errors.into_iter().collect();
                assert_eq!(vec.len(), 2);
            }
            _ => panic!("Expected failure with accumulated errors"),
        }
    }

    #[test]
    fn test_validation_map_success() {
        let v = validation_success(21);
        let v2 = validation_map(v, |n| n * 2);
        match v2 {
            Validation::Success(n) => assert_eq!(n, 42),
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_validation_map_failure() {
        let v: AnalysisValidation<i32> = validation_failure(AnalysisError::validation("error"));
        let v2: AnalysisValidation<i32> = validation_map(v, |n| n * 2);
        assert!(v2.is_failure());
    }

    #[test]
    fn test_effect_pure() {
        let effect = effect_pure(42);
        let result = run_effect(effect, DebtmapConfig::default());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_effect_fail() {
        let effect: AnalysisEffect<i32> = effect_fail(AnalysisError::validation("test"));
        let result = run_effect(effect, DebtmapConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_run_effect() {
        let effect = effect_pure(42);
        let result = run_effect(effect, DebtmapConfig::default());
        assert_eq!(result.unwrap(), 42);
    }
}
