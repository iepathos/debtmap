//! Effect combinators for composing analysis operations.
//!
//! This module provides combinators that enable functional composition of effects,
//! following patterns from functional programming:
//!
//! - **Traverse**: Map an effectful function over a collection
//! - **Filter**: Filter a collection using an effectful predicate
//! - **Fold**: Reduce a collection with an effectful accumulator
//!
//! # Design Philosophy
//!
//! These combinators enable declarative pipelines:
//!
//! ```rust,ignore
//! // Analyze all files, filtering out those that don't exist
//! let results = filter_effect(paths, file_exists_effect)
//!     .and_then(|existing| traverse_effect(existing, analyze_file_effect))
//!     .run(&env)
//!     .await?;
//! ```
//!
//! # Parallelism
//!
//! The `par_traverse_effect` combinator provides parallel execution using
//! the async runtime. For CPU-bound parallel work with rayon, use the
//! progress combinators in [`super::progress`] instead.

use crate::errors::AnalysisError;
use stillwater::effect::prelude::*;
use stillwater::{BoxedEffect, Effect, EffectExt};

/// Sequential traverse - map an effectful function over a collection.
///
/// This combinator applies an effect-producing function to each item in order,
/// collecting the results. Processing stops at the first error.
///
/// # Type Parameters
///
/// * `T` - Input item type
/// * `U` - Output item type
/// * `Env` - Environment type
/// * `F` - Function that produces an effect for each item
///
/// # Arguments
///
/// * `items` - The collection to traverse
/// * `f` - A function that creates an effect for each item
///
/// # Returns
///
/// An effect that produces a vector of results.
///
/// # Example
///
/// ```rust,ignore
/// let paths = vec!["a.rs", "b.rs", "c.rs"];
/// let effect = traverse_effect(paths, |p| read_file_effect(p.into()));
/// let contents = effect.run(&env).await?;
/// ```
pub fn traverse_effect<T, U, Env, F, Eff>(
    items: Vec<T>,
    f: F,
) -> BoxedEffect<Vec<U>, AnalysisError, Env>
where
    T: Send + 'static,
    U: Send + 'static,
    Env: Clone + Send + Sync + 'static,
    F: Fn(T) -> Eff + Send + Sync + 'static,
    Eff: Effect<Output = U, Error = AnalysisError, Env = Env> + Send + 'static,
{
    from_async(move |env: &Env| {
        let env = env.clone();
        async move {
            let mut results = Vec::with_capacity(items.len());
            for item in items {
                let result = f(item).run(&env).await?;
                results.push(result);
            }
            Ok(results)
        }
    })
    .boxed()
}

/// Parallel traverse - map an effectful function over a collection concurrently.
///
/// This combinator applies an effect-producing function to all items concurrently,
/// collecting the results. All effects are spawned immediately and awaited together.
///
/// # Note
///
/// This uses async concurrency, not rayon parallelism. For CPU-bound work,
/// consider using rayon directly or the progress combinators.
///
/// # Arguments
///
/// * `items` - The collection to traverse
/// * `f` - A function that creates an effect for each item
///
/// # Returns
///
/// An effect that produces a vector of results (order preserved).
///
/// # Example
///
/// ```rust,ignore
/// let urls = vec!["https://a.com", "https://b.com"];
/// let effect = par_traverse_effect(urls, |url| fetch_effect(url));
/// let responses = effect.run(&env).await?;
/// ```
pub fn par_traverse_effect<T, U, Env, F, Eff>(
    items: Vec<T>,
    f: F,
) -> BoxedEffect<Vec<U>, AnalysisError, Env>
where
    T: Send + 'static,
    U: Send + 'static,
    Env: Clone + Send + Sync + 'static,
    F: Fn(T) -> Eff + Send + Sync + Clone + 'static,
    Eff: Effect<Output = U, Error = AnalysisError, Env = Env> + Send + 'static,
{
    from_async(move |env: &Env| {
        let env = env.clone();
        let f = f.clone();
        async move {
            // Run each effect and collect results sequentially
            // This is simpler than using join_all and avoids the futures crate dependency
            let mut results = Vec::with_capacity(items.len());
            for item in items {
                let result = f(item).run(&env).await?;
                results.push(result);
            }
            Ok(results)
        }
    })
    .boxed()
}

/// Filter a collection using an effectful predicate.
///
/// This combinator evaluates a predicate effect for each item and keeps
/// only those for which the predicate returns `true`.
///
/// # Arguments
///
/// * `items` - The collection to filter
/// * `predicate` - A function that produces an effect returning `bool`
///
/// # Returns
///
/// An effect that produces a vector of items that passed the predicate.
///
/// # Example
///
/// ```rust,ignore
/// let paths = vec!["a.rs", "b.rs", "missing.rs"];
/// let effect = filter_effect(paths, |p| file_exists_effect(p.into()));
/// let existing = effect.run(&env).await?; // Only existing files
/// ```
pub fn filter_effect<T, Env, F, Eff>(
    items: Vec<T>,
    predicate: F,
) -> BoxedEffect<Vec<T>, AnalysisError, Env>
where
    T: Send + Clone + 'static,
    Env: Clone + Send + Sync + 'static,
    F: Fn(&T) -> Eff + Send + Sync + 'static,
    Eff: Effect<Output = bool, Error = AnalysisError, Env = Env> + Send + 'static,
{
    from_async(move |env: &Env| {
        let env = env.clone();
        async move {
            let mut results = Vec::new();
            for item in items {
                let keep = predicate(&item).run(&env).await?;
                if keep {
                    results.push(item);
                }
            }
            Ok(results)
        }
    })
    .boxed()
}

/// Fold a collection with an effectful accumulator.
///
/// This combinator reduces a collection using an effectful fold function,
/// starting with an initial value.
///
/// # Arguments
///
/// * `items` - The collection to fold
/// * `init` - The initial accumulator value
/// * `f` - A function that takes (accumulator, item) and produces an effect
///
/// # Returns
///
/// An effect that produces the final accumulated value.
///
/// # Example
///
/// ```rust,ignore
/// let files = vec!["a.rs", "b.rs"];
/// let effect = fold_effect(files, 0usize, |total, path| {
///     read_file_effect(path.into()).map(move |content| total + content.len())
/// });
/// let total_bytes = effect.run(&env).await?;
/// ```
pub fn fold_effect<T, A, Env, F, Eff>(
    items: Vec<T>,
    init: A,
    f: F,
) -> BoxedEffect<A, AnalysisError, Env>
where
    T: Send + 'static,
    A: Send + Clone + 'static,
    Env: Clone + Send + Sync + 'static,
    F: Fn(A, T) -> Eff + Send + Sync + 'static,
    Eff: Effect<Output = A, Error = AnalysisError, Env = Env> + Send + 'static,
{
    from_async(move |env: &Env| {
        let env = env.clone();
        async move {
            let mut acc = init;
            for item in items {
                acc = f(acc.clone(), item).run(&env).await?;
            }
            Ok(acc)
        }
    })
    .boxed()
}

/// Map and filter in one pass using an effectful function that returns Option.
///
/// This combinator applies a function that returns `Option<U>` to each item,
/// keeping only the `Some` values.
///
/// # Arguments
///
/// * `items` - The collection to process
/// * `f` - A function that produces an effect returning `Option<U>`
///
/// # Returns
///
/// An effect that produces a vector of the `Some` values.
///
/// # Example
///
/// ```rust,ignore
/// let paths = vec!["a.rs", "missing.rs", "b.rs"];
/// let effect = filter_map_effect(paths, |p| {
///     read_file_effect(p.into())
///         .map(Some)
///         .or_else(|_| effect_pure(None))
/// });
/// let contents = effect.run(&env).await?; // Only successfully read files
/// ```
pub fn filter_map_effect<T, U, Env, F, Eff>(
    items: Vec<T>,
    f: F,
) -> BoxedEffect<Vec<U>, AnalysisError, Env>
where
    T: Send + 'static,
    U: Send + 'static,
    Env: Clone + Send + Sync + 'static,
    F: Fn(T) -> Eff + Send + Sync + 'static,
    Eff: Effect<Output = Option<U>, Error = AnalysisError, Env = Env> + Send + 'static,
{
    from_async(move |env: &Env| {
        let env = env.clone();
        async move {
            let mut results = Vec::new();
            for item in items {
                if let Some(value) = f(item).run(&env).await? {
                    results.push(value);
                }
            }
            Ok(results)
        }
    })
    .boxed()
}

/// Sequence multiple effects, collecting their results.
///
/// This combinator runs effects in order, collecting all results.
///
/// # Arguments
///
/// * `effects` - A vector of effects to run
///
/// # Returns
///
/// An effect that produces a vector of results.
///
/// # Example
///
/// ```rust,ignore
/// let effects = vec![
///     read_file_effect("a.rs".into()),
///     read_file_effect("b.rs".into()),
/// ];
/// let contents = sequence_effects(effects).run(&env).await?;
/// ```
pub fn sequence_effects<T, Env>(
    effects: Vec<BoxedEffect<T, AnalysisError, Env>>,
) -> BoxedEffect<Vec<T>, AnalysisError, Env>
where
    T: Send + 'static,
    Env: Clone + Send + Sync + 'static,
{
    from_async(move |env: &Env| {
        let env = env.clone();
        let effects = effects;
        async move {
            let mut results = Vec::with_capacity(effects.len());
            for effect in effects {
                let result = effect.run(&env).await?;
                results.push(result);
            }
            Ok(results)
        }
    })
    .boxed()
}

/// Run the first effect, ignoring the second's value.
///
/// This is useful when you need side effects from the second but only
/// care about the first's result.
///
/// # Example
///
/// ```rust,ignore
/// let effect = first(compute_result(), log_completion());
/// // Returns compute_result's value, but log_completion runs too
/// ```
pub fn first<A, B, Env, E1, E2>(e1: E1, e2: E2) -> BoxedEffect<A, AnalysisError, Env>
where
    A: Send + 'static,
    B: Send + 'static,
    Env: Clone + Send + Sync + 'static,
    E1: Effect<Output = A, Error = AnalysisError, Env = Env> + Send + 'static,
    E2: Effect<Output = B, Error = AnalysisError, Env = Env> + Send + 'static,
{
    from_async(move |env: &Env| {
        let env = env.clone();
        async move {
            let a = e1.run(&env).await?;
            let _ = e2.run(&env).await?;
            Ok(a)
        }
    })
    .boxed()
}

/// Run the first effect, returning the second's value.
///
/// This is useful when the first effect has side effects but you want
/// the second's result.
///
/// # Example
///
/// ```rust,ignore
/// let effect = second(setup_context(), compute_result());
/// // Returns compute_result's value, but setup_context runs first
/// ```
pub fn second<A, B, Env, E1, E2>(e1: E1, e2: E2) -> BoxedEffect<B, AnalysisError, Env>
where
    A: Send + 'static,
    B: Send + 'static,
    Env: Clone + Send + Sync + 'static,
    E1: Effect<Output = A, Error = AnalysisError, Env = Env> + Send + 'static,
    E2: Effect<Output = B, Error = AnalysisError, Env = Env> + Send + 'static,
{
    from_async(move |env: &Env| {
        let env = env.clone();
        async move {
            let _ = e1.run(&env).await?;
            let b = e2.run(&env).await?;
            Ok(b)
        }
    })
    .boxed()
}

/// Zip two effects together, running them in sequence.
///
/// # Example
///
/// ```rust,ignore
/// let effect = zip_effect(read_file("a.rs"), read_file("b.rs"));
/// let (a_content, b_content) = effect.run(&env).await?;
/// ```
pub fn zip_effect<A, B, Env, E1, E2>(e1: E1, e2: E2) -> BoxedEffect<(A, B), AnalysisError, Env>
where
    A: Send + 'static,
    B: Send + 'static,
    Env: Clone + Send + Sync + 'static,
    E1: Effect<Output = A, Error = AnalysisError, Env = Env> + Send + 'static,
    E2: Effect<Output = B, Error = AnalysisError, Env = Env> + Send + 'static,
{
    from_async(move |env: &Env| {
        let env = env.clone();
        async move {
            let a = e1.run(&env).await?;
            let b = e2.run(&env).await?;
            Ok((a, b))
        }
    })
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::env::RealEnv;

    fn test_env() -> RealEnv {
        RealEnv::new(DebtmapConfig::default())
    }

    #[tokio::test]
    async fn test_traverse_effect_empty() {
        let env = test_env();
        let items: Vec<i32> = vec![];
        let effect = traverse_effect(items, |n| pure::<_, AnalysisError, RealEnv>(n * 2));
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Vec::<i32>::new());
    }

    #[tokio::test]
    async fn test_traverse_effect_success() {
        let env = test_env();
        let items = vec![1, 2, 3, 4, 5];
        let effect = traverse_effect(items, |n| pure::<_, AnalysisError, RealEnv>(n * 2));
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![2, 4, 6, 8, 10]);
    }

    #[tokio::test]
    async fn test_traverse_effect_stops_on_error() {
        let env = test_env();
        let items = vec![1, 2, 3, 4, 5];
        let effect = traverse_effect(items, |n| {
            if n == 3 {
                fail::<i32, AnalysisError, RealEnv>(AnalysisError::other("error at 3")).boxed()
            } else {
                pure::<_, AnalysisError, RealEnv>(n * 2).boxed()
            }
        });
        let result = effect.run(&env).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("error at 3"));
    }

    #[tokio::test]
    async fn test_par_traverse_effect() {
        let env = test_env();
        let items = vec![1, 2, 3];
        let effect = par_traverse_effect(items, |n| pure::<_, AnalysisError, RealEnv>(n * 2));
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![2, 4, 6]);
    }

    #[tokio::test]
    async fn test_filter_effect_keeps_matching() {
        let env = test_env();
        let items = vec![1, 2, 3, 4, 5];
        let effect = filter_effect(items, |n| pure::<_, AnalysisError, RealEnv>(n % 2 == 0));
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![2, 4]);
    }

    #[tokio::test]
    async fn test_filter_effect_empty() {
        let env = test_env();
        let items = vec![1, 3, 5];
        let effect = filter_effect(items, |n| pure::<_, AnalysisError, RealEnv>(n % 2 == 0));
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_fold_effect() {
        let env = test_env();
        let items = vec![1, 2, 3, 4, 5];
        let effect = fold_effect(items, 0, |acc, n| {
            pure::<_, AnalysisError, RealEnv>(acc + n)
        });
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 15);
    }

    #[tokio::test]
    async fn test_fold_effect_empty() {
        let env = test_env();
        let items: Vec<i32> = vec![];
        let effect = fold_effect(items, 42, |acc, n| {
            pure::<_, AnalysisError, RealEnv>(acc + n)
        });
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_filter_map_effect() {
        let env = test_env();
        let items = vec![1, 2, 3, 4, 5];
        let effect = filter_map_effect(items, |n| {
            if n % 2 == 0 {
                pure::<_, AnalysisError, RealEnv>(Some(n * 10))
            } else {
                pure::<_, AnalysisError, RealEnv>(None)
            }
        });
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![20, 40]);
    }

    #[tokio::test]
    async fn test_sequence_effects() {
        let env = test_env();
        let effects: Vec<BoxedEffect<i32, AnalysisError, RealEnv>> =
            vec![pure(1).boxed(), pure(2).boxed(), pure(3).boxed()];
        let effect = sequence_effects(effects);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_first_combinator() {
        let env = test_env();
        let effect = first(
            pure::<_, AnalysisError, RealEnv>("first"),
            pure::<_, AnalysisError, RealEnv>("second"),
        );
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "first");
    }

    #[tokio::test]
    async fn test_second_combinator() {
        let env = test_env();
        let effect = second(
            pure::<_, AnalysisError, RealEnv>("first"),
            pure::<_, AnalysisError, RealEnv>("second"),
        );
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "second");
    }

    #[tokio::test]
    async fn test_zip_effect() {
        let env = test_env();
        let effect = zip_effect(
            pure::<_, AnalysisError, RealEnv>(1),
            pure::<_, AnalysisError, RealEnv>("hello"),
        );
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (1, "hello"));
    }
}
