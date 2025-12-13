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
//! # Reader Pattern (Spec 199)
//!
//! This module also provides **Reader pattern** helpers using stillwater 0.11.0's
//! zero-cost `ask`, `asks`, and `local` primitives. The Reader pattern eliminates
//! config parameter threading by making configuration available through the
//! environment.
//!
//! ## Reader Pattern Benefits
//!
//! **Before (parameter threading):**
//! ```rust,ignore
//! fn analyze(ast: &Ast, config: &Config) -> Metrics {
//!     calculate_complexity(ast, &config.thresholds)
//! }
//! ```
//!
//! **After (Reader pattern):**
//! ```rust,ignore
//! use debtmap::effects::asks_config;
//!
//! fn analyze_effect<Env>(ast: Ast) -> impl Effect<...>
//! where Env: AnalysisEnv + Clone + Send + Sync
//! {
//!     asks_config(move |config| calculate_complexity(&ast, &config.thresholds))
//! }
//! ```
//!
//! ## Available Reader Helpers
//!
//! - [`asks_config`]: Access the full config via closure
//! - [`asks_thresholds`]: Access thresholds config section
//! - [`asks_scoring`]: Access scoring weights config section
//! - [`asks_entropy`]: Access entropy config section
//! - [`local_with_config`]: Run effect with modified config (temporary override)
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
//!
//! # Example: Using Reader Pattern
//!
//! ```rust,ignore
//! use debtmap::effects::{asks_config, asks_thresholds, local_with_config};
//! use debtmap::env::AnalysisEnv;
//! use stillwater::Effect;
//!
//! // Query config via closure
//! fn get_complexity_threshold<Env>() -> impl Effect<Output = Option<u32>, Error = AnalysisError, Env = Env>
//! where
//!     Env: AnalysisEnv + Clone + Send + Sync,
//! {
//!     asks_config(|config| config.thresholds.as_ref().and_then(|t| t.complexity))
//! }
//!
//! // Use temporary config override
//! fn analyze_strict<Env>(path: PathBuf) -> impl Effect<...>
//! where
//!     Env: AnalysisEnv + Clone + Send + Sync,
//! {
//!     local_with_config(
//!         |config| {
//!             let mut strict = config.clone();
//!             // Apply stricter thresholds
//!             strict
//!         },
//!         analyze_file_effect(path)
//!     )
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

// =============================================================================
// Reader Pattern Helpers (Spec 199)
// =============================================================================
//
// These helpers use stillwater 0.11.0's zero-cost Reader primitives to provide
// config access without parameter threading.

use crate::config::{EntropyConfig, ScoringWeights, ThresholdsConfig};
use stillwater::Effect;

use std::sync::Arc;

/// A wrapper type that makes a shared function callable via `Fn` traits.
///
/// This wrapper holds an `Arc<F>` and implements `Fn` by cloning the `Arc`
/// on each call, allowing the function to be called multiple times.
#[derive(Clone)]
pub struct SharedFn<F>(Arc<F>);

impl<F> SharedFn<F> {
    fn new(f: F) -> Self {
        Self(Arc::new(f))
    }
}

/// Query config through closure - the core Reader pattern primitive.
///
/// This function creates an effect that queries the environment's config
/// using a provided closure. The closure receives a reference to the
/// [`DebtmapConfig`] and returns any value.
///
/// Uses `Arc` internally to allow the closure to be called multiple times.
///
/// # Type Parameters
///
/// - `U`: The return type of the query
/// - `Env`: The environment type (must implement [`AnalysisEnv`])
/// - `F`: The query function type
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::asks_config;
///
/// // Get ignore patterns from config
/// fn get_ignore_patterns<Env>() -> impl Effect<Output = Vec<String>, Error = AnalysisError, Env = Env>
/// where
///     Env: AnalysisEnv + Clone + Send + Sync,
/// {
///     asks_config(|config| config.get_ignore_patterns())
/// }
///
/// // Get complexity threshold
/// fn get_complexity_threshold<Env>() -> impl Effect<Output = Option<u32>, Error = AnalysisError, Env = Env>
/// where
///     Env: AnalysisEnv + Clone + Send + Sync,
/// {
///     asks_config(|config| config.thresholds.as_ref().and_then(|t| t.complexity))
/// }
/// ```
pub fn asks_config<U, Env, F>(f: F) -> impl Effect<Output = U, Error = AnalysisError, Env = Env>
where
    U: Send + 'static,
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
    F: Fn(&DebtmapConfig) -> U + Send + Sync + 'static,
{
    let shared = SharedFn::new(f);
    stillwater::asks(move |env: &Env| (shared.0)(env.config()))
}

/// Query thresholds config section.
///
/// Convenience helper for accessing the thresholds configuration.
/// Returns `None` if thresholds are not configured.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::asks_thresholds;
///
/// fn get_max_file_length<Env>() -> impl Effect<Output = Option<usize>, Error = AnalysisError, Env = Env>
/// where
///     Env: AnalysisEnv + Clone + Send + Sync,
/// {
///     asks_thresholds(|thresholds| thresholds.and_then(|t| t.max_file_length))
/// }
/// ```
pub fn asks_thresholds<U, Env, F>(f: F) -> impl Effect<Output = U, Error = AnalysisError, Env = Env>
where
    U: Send + 'static,
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
    F: Fn(Option<&ThresholdsConfig>) -> U + Send + Sync + 'static,
{
    let shared = SharedFn::new(f);
    stillwater::asks(move |env: &Env| (shared.0)(env.config().thresholds.as_ref()))
}

/// Query scoring weights config section.
///
/// Convenience helper for accessing the scoring weights configuration.
/// Returns `None` if scoring weights are not configured.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::asks_scoring;
///
/// fn get_coverage_weight<Env>() -> impl Effect<Output = f64, Error = AnalysisError, Env = Env>
/// where
///     Env: AnalysisEnv + Clone + Send + Sync,
/// {
///     asks_scoring(|scoring| scoring.map(|s| s.coverage).unwrap_or(0.5))
/// }
/// ```
pub fn asks_scoring<U, Env, F>(f: F) -> impl Effect<Output = U, Error = AnalysisError, Env = Env>
where
    U: Send + 'static,
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
    F: Fn(Option<&ScoringWeights>) -> U + Send + Sync + 'static,
{
    let shared = SharedFn::new(f);
    stillwater::asks(move |env: &Env| (shared.0)(env.config().scoring.as_ref()))
}

/// Query entropy config section.
///
/// Convenience helper for accessing the entropy configuration.
/// Returns `None` if entropy config is not set.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::asks_entropy;
///
/// fn is_entropy_enabled<Env>() -> impl Effect<Output = bool, Error = AnalysisError, Env = Env>
/// where
///     Env: AnalysisEnv + Clone + Send + Sync,
/// {
///     asks_entropy(|entropy| entropy.map(|e| e.enabled).unwrap_or(true))
/// }
/// ```
pub fn asks_entropy<U, Env, F>(f: F) -> impl Effect<Output = U, Error = AnalysisError, Env = Env>
where
    U: Send + 'static,
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
    F: Fn(Option<&EntropyConfig>) -> U + Send + Sync + 'static,
{
    let shared = SharedFn::new(f);
    stillwater::asks(move |env: &Env| (shared.0)(env.config().entropy.as_ref()))
}

/// Run an effect with a temporarily modified config.
///
/// This is the Reader pattern's `local` operation - it allows running an
/// inner effect with a modified environment. The modification is only
/// visible to the inner effect; after it completes, the original
/// environment is restored.
///
/// This is useful for:
/// - **Strict mode**: Running analysis with stricter thresholds
/// - **Custom thresholds**: Temporarily overriding specific settings
/// - **Feature flags**: Temporarily enabling/disabling features
///
/// # Type Parameters
///
/// - `Inner`: The inner effect type
/// - `F`: The environment transformation function
/// - `Env`: The environment type
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::local_with_config;
///
/// // Run analysis in strict mode (lower complexity threshold)
/// fn analyze_strict<Env>(
///     path: PathBuf,
/// ) -> impl Effect<Output = FileAnalysis, Error = AnalysisError, Env = Env>
/// where
///     Env: AnalysisEnv + Clone + Send + Sync,
/// {
///     local_with_config(
///         |config| {
///             let mut strict = config.clone();
///             if let Some(ref mut thresholds) = strict.thresholds {
///                 // Reduce complexity threshold by half
///                 if let Some(complexity) = thresholds.complexity {
///                     thresholds.complexity = Some(complexity / 2);
///                 }
///             }
///             strict
///         },
///         analyze_file_effect(path)
///     )
/// }
///
/// // Temporarily disable entropy-based scoring
/// fn analyze_without_entropy<Env>(
///     inner: impl Effect<Output = T, Error = AnalysisError, Env = Env>
/// ) -> impl Effect<Output = T, Error = AnalysisError, Env = Env>
/// where
///     Env: AnalysisEnv + Clone + Send + Sync,
/// {
///     local_with_config(
///         |config| {
///             let mut modified = config.clone();
///             if let Some(ref mut entropy) = modified.entropy {
///                 entropy.enabled = false;
///             }
///             modified
///         },
///         inner
///     )
/// }
/// ```
pub fn local_with_config<Inner, F, Env>(
    f: F,
    inner: Inner,
) -> impl Effect<Output = Inner::Output, Error = Inner::Error, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
    F: Fn(&DebtmapConfig) -> DebtmapConfig + Send + Sync + 'static,
    Inner: Effect<Env = Env>,
{
    let shared = SharedFn::new(f);
    stillwater::local(
        move |env: &Env| {
            let new_config = (shared.0)(env.config());
            env.clone().with_config(new_config)
        },
        inner,
    )
}

/// Query the entire environment.
///
/// This is a low-level helper that returns a clone of the entire environment.
/// Prefer using [`asks_config`] or the specific query helpers when you only
/// need config access.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::ask_env;
///
/// fn get_full_env<Env>() -> impl Effect<Output = Env, Error = AnalysisError, Env = Env>
/// where
///     Env: AnalysisEnv + Clone + Send + Sync,
/// {
///     ask_env()
/// }
/// ```
pub fn ask_env<Env>() -> stillwater::effect::reader::Ask<AnalysisError, Env>
where
    Env: Clone + Send + Sync + 'static,
{
    stillwater::ask::<AnalysisError, Env>()
}

// =============================================================================
// Retry Pattern Helpers (Spec 205)
// =============================================================================
//
// These helpers enable automatic retry of transient failures with configurable
// backoff strategies.

use crate::config::RetryConfig;
use log::{error, info, warn};
use std::time::Instant;

/// Wrap an effect with retry logic using the configured policy.
///
/// This combinator automatically retries the effect when it fails with
/// a retryable error, using the configured retry strategy and delays.
///
/// # Arguments
///
/// * `effect_factory` - A function that creates the effect to retry.
///   Called each time a retry is needed.
/// * `retry_config` - Configuration for retry behavior.
///
/// # Retryable Errors
///
/// Only errors where `error.is_retryable()` returns `true` will trigger
/// a retry. Non-retryable errors (parse errors, validation errors, etc.)
/// cause immediate failure.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::{with_retry, AnalysisEffect};
/// use debtmap::config::RetryConfig;
/// use debtmap::io::effects::read_file_effect;
///
/// fn read_file_resilient(path: PathBuf) -> AnalysisEffect<String> {
///     let config = RetryConfig::default();
///     with_retry(
///         move || read_file_effect(path.clone()),
///         config,
///     )
/// }
/// ```
///
/// # Logging
///
/// Retry attempts are logged at WARN level for visibility:
/// ```text
/// WARN Retrying operation (attempt 2/3): I/O error: Resource busy
/// ```
pub fn with_retry<T, F>(effect_factory: F, retry_config: RetryConfig) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: Fn() -> AnalysisEffect<T> + Send + Sync + 'static,
{
    from_async(move |env: &RealEnv| {
        let env = env.clone();
        let config = retry_config.clone();
        let factory = SharedFn::new(effect_factory);

        async move {
            let start = Instant::now();
            let mut attempt = 0u32;
            let mut last_error: Option<AnalysisError> = None;

            loop {
                let effect = (factory.0)();
                match effect.run(&env).await {
                    Ok(value) => {
                        if attempt > 0 {
                            info!("Operation succeeded after {} retry attempt(s)", attempt);
                        }
                        return Ok(value);
                    }
                    Err(e) => {
                        let elapsed = start.elapsed();

                        // Check if error is retryable and we should try again
                        if e.is_retryable() && config.should_retry(attempt, elapsed) {
                            attempt += 1;
                            warn!(
                                "Retrying operation (attempt {}/{}): {}",
                                attempt, config.max_retries, e
                            );

                            // Sleep before retry
                            let delay = config.delay_for_attempt(attempt);
                            tokio::time::sleep(delay).await;

                            let _ = last_error.insert(e);
                        } else {
                            // Not retryable or exhausted retries
                            if attempt > 0 {
                                error!(
                                    "Operation failed after {} retry attempt(s): {}",
                                    attempt, e
                                );
                            }
                            return Err(e);
                        }
                    }
                }
            }
        }
    })
    .boxed()
}

/// Wrap an effect with retry logic, using the retry config from environment.
///
/// This is a convenience function that reads the retry configuration from
/// the environment's config. If no retry config is set, uses defaults.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::with_retry_from_env;
/// use debtmap::io::effects::read_file_effect;
///
/// fn read_file_resilient(path: PathBuf) -> AnalysisEffect<String> {
///     with_retry_from_env(move || read_file_effect(path.clone()))
/// }
/// ```
pub fn with_retry_from_env<T, F>(effect_factory: F) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: Fn() -> AnalysisEffect<T> + Send + Sync + Clone + 'static,
{
    from_async(move |env: &RealEnv| {
        let env = env.clone();
        let factory = effect_factory.clone();

        async move {
            let config = env.config().retry.clone().unwrap_or_default();

            // If retries are disabled, just run the effect directly
            if !config.enabled {
                return factory().run(&env).await;
            }

            let start = Instant::now();
            let mut attempt = 0u32;

            loop {
                let effect = factory();
                match effect.run(&env).await {
                    Ok(value) => {
                        if attempt > 0 {
                            info!("Operation succeeded after {} retry attempt(s)", attempt);
                        }
                        return Ok(value);
                    }
                    Err(e) => {
                        let elapsed = start.elapsed();

                        if e.is_retryable() && config.should_retry(attempt, elapsed) {
                            attempt += 1;
                            warn!(
                                "Retrying operation (attempt {}/{}): {}",
                                attempt, config.max_retries, e
                            );

                            let delay = config.delay_for_attempt(attempt);
                            tokio::time::sleep(delay).await;
                        } else {
                            if attempt > 0 {
                                error!(
                                    "Operation failed after {} retry attempt(s): {}",
                                    attempt, e
                                );
                            }
                            return Err(e);
                        }
                    }
                }
            }
        }
    })
    .boxed()
}

/// Check if retries are enabled in the given config.
///
/// Returns `true` if the retry config is present and enabled.
pub fn is_retry_enabled(config: &DebtmapConfig) -> bool {
    config.retry.as_ref().map(|r| r.enabled).unwrap_or(true) // Default is enabled
}

/// Get the effective retry config from a DebtmapConfig.
///
/// Returns the configured retry settings or defaults if not set.
pub fn get_retry_config(config: &DebtmapConfig) -> RetryConfig {
    config.retry.clone().unwrap_or_default()
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

    // =========================================================================
    // Reader Pattern Tests (Spec 199)
    // =========================================================================

    #[tokio::test]
    async fn test_asks_config_returns_config_value() {
        use crate::config::IgnoreConfig;

        let config = DebtmapConfig {
            ignore: Some(IgnoreConfig {
                patterns: vec!["test/**".to_string()],
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        // Create effect that queries config
        let effect = asks_config::<Vec<String>, RealEnv, _>(|config| config.get_ignore_patterns());

        let patterns = effect.run(&env).await.unwrap();
        assert_eq!(patterns, vec!["test/**".to_string()]);
    }

    #[tokio::test]
    async fn test_asks_config_with_default_config() {
        let env = RealEnv::default();

        // Query ignore patterns from default config
        let effect = asks_config::<Vec<String>, RealEnv, _>(|config| config.get_ignore_patterns());

        let patterns = effect.run(&env).await.unwrap();
        assert!(patterns.is_empty()); // Default config has no ignore patterns
    }

    #[tokio::test]
    async fn test_asks_thresholds_with_thresholds() {
        use crate::config::ThresholdsConfig;

        let config = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                complexity: Some(15),
                max_file_length: Some(500),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect = asks_thresholds::<Option<u32>, RealEnv, _>(|thresholds| {
            thresholds.and_then(|t| t.complexity)
        });

        let complexity = effect.run(&env).await.unwrap();
        assert_eq!(complexity, Some(15));
    }

    #[tokio::test]
    async fn test_asks_thresholds_without_thresholds() {
        let env = RealEnv::default();

        let effect = asks_thresholds::<Option<u32>, RealEnv, _>(|thresholds| {
            thresholds.and_then(|t| t.complexity)
        });

        let complexity = effect.run(&env).await.unwrap();
        assert_eq!(complexity, None);
    }

    #[tokio::test]
    async fn test_asks_scoring_with_weights() {
        let config = DebtmapConfig {
            scoring: Some(ScoringWeights {
                coverage: 0.6,
                complexity: 0.3,
                dependency: 0.1,
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect =
            asks_scoring::<f64, RealEnv, _>(|scoring| scoring.map(|s| s.coverage).unwrap_or(0.5));

        let coverage = effect.run(&env).await.unwrap();
        assert!((coverage - 0.6).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_asks_entropy_enabled() {
        let config = DebtmapConfig {
            entropy: Some(EntropyConfig {
                enabled: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect =
            asks_entropy::<bool, RealEnv, _>(|entropy| entropy.map(|e| e.enabled).unwrap_or(true));

        let enabled = effect.run(&env).await.unwrap();
        assert!(!enabled);
    }

    #[tokio::test]
    async fn test_local_with_config_modifies_config() {
        use crate::config::IgnoreConfig;

        let original_config = DebtmapConfig {
            ignore: Some(IgnoreConfig {
                patterns: vec!["original/**".to_string()],
            }),
            ..Default::default()
        };
        let env = RealEnv::new(original_config);

        // Inner effect that queries config
        let inner = asks_config::<Vec<String>, RealEnv, _>(|config| config.get_ignore_patterns());

        // Wrap with local that modifies config
        let effect = local_with_config(
            |_config| DebtmapConfig {
                ignore: Some(IgnoreConfig {
                    patterns: vec!["modified/**".to_string()],
                }),
                ..Default::default()
            },
            inner,
        );

        let patterns = effect.run(&env).await.unwrap();
        assert_eq!(patterns, vec!["modified/**".to_string()]);
    }

    #[tokio::test]
    async fn test_local_with_config_restores_after() {
        use crate::config::IgnoreConfig;

        let original_config = DebtmapConfig {
            ignore: Some(IgnoreConfig {
                patterns: vec!["original/**".to_string()],
            }),
            ..Default::default()
        };
        let env = RealEnv::new(original_config.clone());

        // Run with modified config
        let inner = asks_config::<Vec<String>, RealEnv, _>(|config| config.get_ignore_patterns());
        let modified_effect = local_with_config(
            |_| DebtmapConfig {
                ignore: Some(IgnoreConfig {
                    patterns: vec!["modified/**".to_string()],
                }),
                ..Default::default()
            },
            inner,
        );
        let _ = modified_effect.run(&env).await.unwrap();

        // Original env should be unchanged (run a new query)
        let check_effect =
            asks_config::<Vec<String>, RealEnv, _>(|config| config.get_ignore_patterns());
        let patterns = check_effect.run(&env).await.unwrap();
        assert_eq!(patterns, vec!["original/**".to_string()]);
    }

    #[tokio::test]
    async fn test_ask_env_returns_cloned_env() {
        let config = DebtmapConfig::default();
        let env = RealEnv::new(config);

        let effect = ask_env::<RealEnv>();

        let cloned_env = effect.run(&env).await.unwrap();
        // Both should have the same config
        assert_eq!(
            format!("{:?}", cloned_env.config()),
            format!("{:?}", env.config())
        );
    }

    #[tokio::test]
    async fn test_reader_pattern_composition() {
        use stillwater::EffectExt;

        let config = DebtmapConfig {
            scoring: Some(ScoringWeights {
                coverage: 0.6,
                complexity: 0.4,
                dependency: 0.0,
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        // Compose multiple Reader queries
        let coverage_effect =
            asks_scoring::<f64, RealEnv, _>(|scoring| scoring.map(|s| s.coverage).unwrap_or(0.5));

        let complexity_effect = asks_scoring::<f64, RealEnv, _>(|scoring| {
            scoring.map(|s| s.complexity).unwrap_or(0.35)
        });

        // Use and_then to compose effects
        let combined =
            coverage_effect.and_then(move |cov| complexity_effect.map(move |comp| cov + comp));

        let sum = combined.run(&env).await.unwrap();
        assert!((sum - 1.0).abs() < 0.001); // 0.6 + 0.4 = 1.0
    }

    // =========================================================================
    // Retry Pattern Tests (Spec 205)
    // =========================================================================

    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_with_retry_succeeds_first_attempt() {
        let config = RetryConfig::default();
        let env = RealEnv::default();

        let effect = with_retry(|| effect_pure(42), config);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_retry_succeeds_after_transient_failure() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 10, // Short delay for tests
            jitter_factor: 0.0,
            ..Default::default()
        };
        let env = RealEnv::default();

        // Counter to track attempts
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_clone = attempt_count.clone();

        let effect = with_retry(
            move || {
                let count = attempt_clone.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    // Fail with retryable error for first 2 attempts
                    effect_fail(AnalysisError::io("Resource busy"))
                } else {
                    // Succeed on third attempt
                    effect_pure("success".to_string())
                }
            },
            config,
        );

        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        // Should have made 3 attempts (indices 0, 1, 2)
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_with_retry_fails_on_permanent_error() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 10,
            jitter_factor: 0.0,
            ..Default::default()
        };
        let env = RealEnv::default();

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_clone = attempt_count.clone();

        let effect: AnalysisEffect<String> = with_retry(
            move || {
                attempt_clone.fetch_add(1, Ordering::SeqCst);
                // Parse errors are not retryable
                effect_fail(AnalysisError::parse("Syntax error"))
            },
            config,
        );

        let result = effect.run(&env).await;

        assert!(result.is_err());
        // Should have only made 1 attempt (immediate failure, no retry)
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_with_retry_exhausts_retries() {
        let config = RetryConfig {
            max_retries: 2,
            base_delay_ms: 10,
            jitter_factor: 0.0,
            ..Default::default()
        };
        let env = RealEnv::default();

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_clone = attempt_count.clone();

        let effect: AnalysisEffect<String> = with_retry(
            move || {
                attempt_clone.fetch_add(1, Ordering::SeqCst);
                // Always fail with retryable error
                effect_fail(AnalysisError::io("Resource busy"))
            },
            config,
        );

        let result = effect.run(&env).await;

        assert!(result.is_err());
        // Initial attempt + 2 retries = 3 total attempts
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_with_retry_disabled() {
        let config = RetryConfig::disabled();
        let env = RealEnv::default();

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_clone = attempt_count.clone();

        let effect: AnalysisEffect<String> = with_retry(
            move || {
                attempt_clone.fetch_add(1, Ordering::SeqCst);
                effect_fail(AnalysisError::io("Resource busy"))
            },
            config,
        );

        let result = effect.run(&env).await;

        assert!(result.is_err());
        // With retries disabled, should only try once
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_with_retry_from_env_uses_config() {
        let config = DebtmapConfig {
            retry: Some(RetryConfig {
                enabled: true,
                max_retries: 2,
                base_delay_ms: 10,
                jitter_factor: 0.0,
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_clone = attempt_count.clone();

        let factory = move || {
            let count = attempt_clone.fetch_add(1, Ordering::SeqCst);
            if count < 1 {
                effect_fail(AnalysisError::io("Resource busy"))
            } else {
                effect_pure("success".to_string())
            }
        };

        let effect = with_retry_from_env(factory);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        // Should have made 2 attempts
        assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_with_retry_from_env_disabled() {
        let config = DebtmapConfig {
            retry: Some(RetryConfig::disabled()),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempt_clone = attempt_count.clone();

        let factory = move || {
            attempt_clone.fetch_add(1, Ordering::SeqCst);
            effect_fail::<String>(AnalysisError::io("Resource busy"))
        };

        let effect = with_retry_from_env(factory);
        let result = effect.run(&env).await;

        assert!(result.is_err());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_is_retry_enabled_default() {
        let config = DebtmapConfig::default();
        assert!(is_retry_enabled(&config));
    }

    #[test]
    fn test_is_retry_enabled_explicit_true() {
        let config = DebtmapConfig {
            retry: Some(RetryConfig {
                enabled: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(is_retry_enabled(&config));
    }

    #[test]
    fn test_is_retry_enabled_explicit_false() {
        let config = DebtmapConfig {
            retry: Some(RetryConfig::disabled()),
            ..Default::default()
        };
        assert!(!is_retry_enabled(&config));
    }

    #[test]
    fn test_get_retry_config_default() {
        let config = DebtmapConfig::default();
        let retry_config = get_retry_config(&config);

        assert!(retry_config.enabled);
        assert_eq!(retry_config.max_retries, 3);
    }

    #[test]
    fn test_get_retry_config_custom() {
        let config = DebtmapConfig {
            retry: Some(RetryConfig {
                enabled: true,
                max_retries: 5,
                base_delay_ms: 200,
                ..Default::default()
            }),
            ..Default::default()
        };
        let retry_config = get_retry_config(&config);

        assert!(retry_config.enabled);
        assert_eq!(retry_config.max_retries, 5);
        assert_eq!(retry_config.base_delay_ms, 200);
    }
}
