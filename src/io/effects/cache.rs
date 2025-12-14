//! Cache effect operations.
//!
//! This module provides Effect-based wrappers around cache operations:
//! - Getting cached values with automatic deserialization
//! - Setting cached values with automatic serialization
//! - Invalidating cache entries
//! - Clearing all cache entries
//!
//! # Design Philosophy
//!
//! Cache operations are wrapped in Effect types, allowing caching behavior
//! to be tested with mock cache implementations and composed with other effects.

use crate::effects::AnalysisEffect;
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use stillwater::effect::prelude::*;

/// Get a value from the cache as an Effect.
///
/// Returns `None` if the key doesn't exist or the cached value
/// can't be deserialized to the expected type.
///
/// # Type Parameters
///
/// - `T`: The type to deserialize the cached value to. Must implement
///   `serde::de::DeserializeOwned`.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::cache_get_effect;
///
/// let effect = cache_get_effect::<FileMetrics>("analysis:src/main.rs".into());
/// if let Some(cached) = run_effect(effect, config)? {
///     println!("Using cached result");
/// }
/// ```
pub fn cache_get_effect<T>(key: String) -> AnalysisEffect<Option<T>>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    from_fn(move |env: &RealEnv| {
        match env.cache().get(&key) {
            Some(bytes) => {
                // Try to deserialize the cached value
                match bincode::deserialize(&bytes) {
                    Ok(value) => Ok(Some(value)),
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to deserialize cache value for key '{}': {}",
                            key, e
                        );
                        // Cache value corrupted or wrong type - treat as miss
                        Ok(None)
                    }
                }
            }
            None => Ok(None),
        }
    })
    .boxed()
}

/// Set a value in the cache as an Effect.
///
/// The value is serialized using bincode for efficient storage.
///
/// # Type Parameters
///
/// - `T`: The type to cache. Must implement `serde::Serialize`.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::cache_set_effect;
///
/// let metrics = analyze_file(&path)?;
/// let effect = cache_set_effect("analysis:src/main.rs".into(), metrics);
/// run_effect(effect, config)?;
/// ```
pub fn cache_set_effect<T>(key: String, value: T) -> AnalysisEffect<()>
where
    T: serde::Serialize + Send + 'static,
{
    from_fn(move |env: &RealEnv| {
        let bytes = bincode::serialize(&value).map_err(|e| {
            AnalysisError::other(format!(
                "Failed to serialize cache value for '{}': {}",
                key, e
            ))
        })?;

        env.cache().set(&key, &bytes).map_err(|e| {
            AnalysisError::other(format!("Cache write failed for '{}': {}", key, e.message()))
        })
    })
    .boxed()
}

/// Invalidate a cache entry as an Effect.
///
/// Removes the cached value for the given key. No error is returned
/// if the key doesn't exist.
pub fn cache_invalidate_effect(key: String) -> AnalysisEffect<()> {
    from_fn(move |env: &RealEnv| {
        env.cache().invalidate(&key).map_err(|e| {
            AnalysisError::other(format!(
                "Cache invalidation failed for '{}': {}",
                key,
                e.message()
            ))
        })
    })
    .boxed()
}

/// Clear all cache entries as an Effect.
pub fn cache_clear_effect() -> AnalysisEffect<()> {
    from_fn(|env: &RealEnv| {
        env.cache()
            .clear()
            .map_err(|e| AnalysisError::other(format!("Cache clear failed: {}", e.message())))
    })
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::run_effect_with_env;
    use crate::env::RealEnv;

    #[test]
    fn test_cache_operations() {
        // Use a single shared environment to test cache persistence
        let env = RealEnv::default();

        // Set and get (same env)
        let set_effect = cache_set_effect("test_key".into(), vec![1, 2, 3]);
        assert!(run_effect_with_env(set_effect, &env).is_ok());

        let get_effect = cache_get_effect::<Vec<i32>>("test_key".into());
        let result = run_effect_with_env(get_effect, &env);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(vec![1, 2, 3]));

        // Get nonexistent key
        let get_effect = cache_get_effect::<Vec<i32>>("nonexistent".into());
        let result = run_effect_with_env(get_effect, &env);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Invalidate
        let inv_effect = cache_invalidate_effect("test_key".into());
        assert!(run_effect_with_env(inv_effect, &env).is_ok());

        let get_effect = cache_get_effect::<Vec<i32>>("test_key".into());
        assert!(run_effect_with_env(get_effect, &env).unwrap().is_none());
    }
}
