//! Effect utilities for analysis modules.
//!
//! This module provides effect wrappers and utilities for the analysis subsystem,
//! enabling effect-based patterns while maintaining backwards compatibility with
//! existing `anyhow::Result` based code.
//!
//! # Architecture
//!
//! The module follows a "pure core, effects shell" pattern:
//!
//! - **Pure functions**: Core analysis logic (AST analysis, metric computation,
//!   diagnostic generation) remains as pure functions with no effects.
//! - **Effect wrappers**: I/O operations and configuration access are wrapped
//!   in effects for testability and composability.
//!
//! # Reader Pattern Integration
//!
//! Configuration access uses the Reader pattern via `asks_config()` from
//! `crate::effects`, eliminating parameter threading:
//!
//! ```rust,ignore
//! use crate::analysis::effects::analyze_with_config_effect;
//!
//! fn analyze_effect(source: &str) -> AnalysisEffect<AnalysisResult> {
//!     asks_config(move |config| {
//!         let thresholds = config.thresholds.as_ref();
//!         // Use thresholds in analysis...
//!     })
//! }
//! ```
//!
//! # Error Accumulation
//!
//! For multi-file validation, use `AnalysisValidation` to accumulate all errors:
//!
//! ```rust,ignore
//! use crate::analysis::effects::validate_analysis_results;
//!
//! let validations = files.iter()
//!     .map(|f| validate_file_analysis(f))
//!     .collect();
//! let combined = combine_validations(validations);
//! ```

use crate::config::DebtmapConfig;
use crate::effects::{
    asks_config, effect_from_fn, effect_pure, validation_failure, validation_success,
    AnalysisEffect, AnalysisValidation,
};
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use stillwater::effect::prelude::*;
use stillwater::Effect;

// Re-export common effect types for convenience
pub use crate::effects::{
    combine_validations, run_effect, run_effect_async, run_effect_with_env, run_validation,
    validation_failures,
};

/// Run analysis function with config from environment.
///
/// This is the primary way to integrate pure analysis functions with the
/// effect system. The analysis function receives the config and should
/// return a Result.
///
/// # Example
///
/// ```rust,ignore
/// fn analyze_complexity(source: &str, config: &DebtmapConfig) -> Result<u32> {
///     // Pure analysis logic
/// }
///
/// let effect = analyze_with_config(|config| {
///     analyze_complexity(&source, config)
/// });
/// ```
pub fn analyze_with_config<T, F>(f: F) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce(&DebtmapConfig) -> Result<T, AnalysisError> + Send + 'static,
{
    effect_from_fn(move |env: &RealEnv| f(env.config()))
}

/// Run analysis function that may access environment.
///
/// Use this when the analysis needs more than just config access,
/// such as file system operations.
pub fn analyze_with_env<T, F>(f: F) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce(&RealEnv) -> Result<T, AnalysisError> + Send + 'static,
{
    effect_from_fn(f)
}

/// Lift a pure computation into an effect.
///
/// Use this to wrap pure analysis results that don't need I/O or config.
pub fn lift_pure<T: Send + 'static>(value: T) -> AnalysisEffect<T> {
    effect_pure(value)
}

/// Create a validation from an analysis result.
///
/// Converts a Result into a Validation for error accumulation.
pub fn validate_result<T: Clone>(result: Result<T, AnalysisError>) -> AnalysisValidation<T> {
    match result {
        Ok(value) => validation_success(value),
        Err(e) => validation_failure(e),
    }
}

/// Query analysis config and transform result.
///
/// Convenience wrapper around `asks_config` that returns an effect
/// suitable for chaining with other analysis operations.
///
/// # Example
///
/// ```rust,ignore
/// let threshold_effect = query_config(|config| {
///     config.thresholds
///         .as_ref()
///         .and_then(|t| t.complexity)
///         .unwrap_or(10)
/// });
/// ```
pub fn query_config<T, F>(f: F) -> impl Effect<Output = T, Error = AnalysisError, Env = RealEnv>
where
    T: Send + 'static,
    F: Fn(&DebtmapConfig) -> T + Send + Sync + 'static,
{
    asks_config::<T, RealEnv, _>(f)
}

/// Get complexity threshold from config with default fallback.
pub fn get_complexity_threshold() -> impl Effect<Output = u32, Error = AnalysisError, Env = RealEnv>
{
    query_config(|config| {
        config
            .thresholds
            .as_ref()
            .and_then(|t| t.complexity)
            .unwrap_or(10)
    })
}

/// Get file length threshold from config with default fallback.
pub fn get_file_length_threshold(
) -> impl Effect<Output = usize, Error = AnalysisError, Env = RealEnv> {
    query_config(|config| {
        config
            .thresholds
            .as_ref()
            .and_then(|t| t.max_file_length)
            .unwrap_or(500)
    })
}

/// Get cognitive complexity threshold from config with default fallback.
pub fn get_cognitive_threshold() -> impl Effect<Output = u32, Error = AnalysisError, Env = RealEnv>
{
    query_config(|config| {
        config
            .thresholds
            .as_ref()
            .and_then(|t| t.minimum_cognitive_complexity)
            .unwrap_or(15)
    })
}

/// Run multiple analysis effects in sequence, collecting results.
///
/// This is useful when you need to run a series of analyses that depend
/// on each other or must be run in order.
pub fn sequence_effects<T>(effects: Vec<AnalysisEffect<T>>) -> AnalysisEffect<Vec<T>>
where
    T: Send + 'static,
{
    from_async(move |env: &RealEnv| {
        let env = env.clone();
        let effects = effects;
        async move {
            let mut results = Vec::new();
            for effect in effects {
                let result = effect.run(&env).await?;
                results.push(result);
            }
            Ok(results)
        }
    })
    .boxed()
}

/// Map a function over multiple items, creating effects for each.
///
/// This creates an effect that processes each item through the provided
/// effect factory and collects the results. Unlike parallel processing,
/// this runs effects sequentially.
pub fn traverse_effect<T, U, F>(items: Vec<T>, f: F) -> AnalysisEffect<Vec<U>>
where
    T: Send + 'static,
    U: Send + 'static,
    F: Fn(T) -> AnalysisEffect<U> + Send + Sync + 'static,
{
    from_async(move |env: &RealEnv| {
        let env = env.clone();
        async move {
            let mut results = Vec::new();
            for item in items {
                let effect = f(item);
                let result = effect.run(&env).await?;
                results.push(result);
            }
            Ok(results)
        }
    })
    .boxed()
}

/// Convert an effect result to anyhow::Result for backwards compatibility.
///
/// This is the primary integration point for existing code that uses
/// `anyhow::Result`. New code should prefer effect-based APIs.
pub fn run_analysis_effect<T: Send + 'static>(
    effect: AnalysisEffect<T>,
    config: DebtmapConfig,
) -> anyhow::Result<T> {
    run_effect(effect, config)
}

/// Convert an effect result to anyhow::Result using default config.
pub fn run_analysis_effect_default<T: Send + 'static>(
    effect: AnalysisEffect<T>,
) -> anyhow::Result<T> {
    run_effect(effect, DebtmapConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ThresholdsConfig;

    #[test]
    fn test_lift_pure() {
        let effect = lift_pure(42);
        let result = run_analysis_effect_default(effect);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_validate_result_success() {
        let result: Result<i32, AnalysisError> = Ok(42);
        let validation = validate_result(result);
        assert!(validation.is_success());
    }

    #[test]
    fn test_validate_result_failure() {
        let result: Result<i32, AnalysisError> = Err(AnalysisError::validation("test error"));
        let validation = validate_result(result);
        assert!(validation.is_failure());
    }

    #[tokio::test]
    async fn test_query_config_with_thresholds() {
        let config = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                complexity: Some(15),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect = query_config(|c| {
            c.thresholds
                .as_ref()
                .and_then(|t| t.complexity)
                .unwrap_or(10)
        });
        let result = effect.run(&env).await.unwrap();
        assert_eq!(result, 15);
    }

    #[tokio::test]
    async fn test_get_complexity_threshold_default() {
        let env = RealEnv::default();
        let effect = get_complexity_threshold();
        let result = effect.run(&env).await.unwrap();
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_get_complexity_threshold_custom() {
        let config = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                complexity: Some(20),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect = get_complexity_threshold();
        let result = effect.run(&env).await.unwrap();
        assert_eq!(result, 20);
    }

    #[tokio::test]
    async fn test_analyze_with_config() {
        let config = DebtmapConfig {
            thresholds: Some(ThresholdsConfig {
                complexity: Some(5),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect = analyze_with_config(|config| {
            let threshold = config
                .thresholds
                .as_ref()
                .and_then(|t| t.complexity)
                .unwrap_or(10);
            Ok(threshold * 2)
        });

        let result = effect.run(&env).await.unwrap();
        assert_eq!(result, 10); // 5 * 2
    }

    #[tokio::test]
    async fn test_sequence_effects() {
        let env = RealEnv::default();

        let effects = vec![lift_pure(1), lift_pure(2), lift_pure(3)];

        let effect = sequence_effects(effects);
        let result = effect.run(&env).await.unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_traverse_effect() {
        let env = RealEnv::default();

        let items = vec![1, 2, 3, 4, 5];
        let effect = traverse_effect(items, |n| lift_pure(n * 2));

        let result = effect.run(&env).await.unwrap();
        assert_eq!(result, vec![2, 4, 6, 8, 10]);
    }
}
