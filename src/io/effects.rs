//! Effect-wrapped I/O operations for debtmap analysis.
//!
//! This module provides Effect-based wrappers around file system operations,
//! enabling pure functional composition while maintaining testability.
//!
//! # Design Philosophy
//!
//! All I/O operations are wrapped in Effect types, which:
//! - Defer execution until explicitly run with an environment
//! - Enable testing with mock environments
//! - Compose naturally with other effects using `and_then`, `map`, etc.
//! - Preserve error context through the call chain
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::io::effects::{read_file_effect, walk_dir_effect};
//! use debtmap::effects::run_effect;
//! use debtmap::config::DebtmapConfig;
//!
//! // Read a single file
//! let content = run_effect(
//!     read_file_effect("src/main.rs".into()),
//!     DebtmapConfig::default(),
//! )?;
//!
//! // Walk a directory and filter files
//! let rust_files = run_effect(
//!     walk_dir_effect("src".into())
//!         .map(|files| files.into_iter()
//!             .filter(|p| p.extension().map_or(false, |e| e == "rs"))
//!             .collect::<Vec<_>>()),
//!     DebtmapConfig::default(),
//! )?;
//! ```

use crate::effects::AnalysisEffect;
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use crate::io::walker::FileWalker;
use std::path::PathBuf;
use stillwater::effect::prelude::*;

// ============================================================================
// File Read Operations
// ============================================================================

/// Read file contents as an Effect.
///
/// This is the fundamental file reading operation wrapped as an Effect.
/// It uses the environment's file system trait for actual I/O, enabling
/// mock implementations in tests.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::read_file_effect;
///
/// let effect = read_file_effect("src/main.rs".into());
/// let content = run_effect(effect, DebtmapConfig::default())?;
/// ```
///
/// # Errors
///
/// Returns `AnalysisError::IoError` if:
/// - The file doesn't exist
/// - Permission is denied
/// - The file isn't valid UTF-8
pub fn read_file_effect(path: PathBuf) -> AnalysisEffect<String> {
    let path_display = path.display().to_string();
    from_fn(move |env: &RealEnv| {
        env.file_system().read_to_string(&path).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to read file: {}", e.message()), &path)
        })
    })
    .map_err(move |e| {
        // Add context about what we were trying to do
        AnalysisError::io_with_path(
            format!("Reading file '{}': {}", path_display, e.message()),
            e.path().cloned().unwrap_or_default(),
        )
    })
    .boxed()
}

/// Read file contents as bytes.
///
/// Similar to `read_file_effect` but returns raw bytes instead of a string.
/// Useful for binary files or when UTF-8 validation isn't needed.
pub fn read_file_bytes_effect(path: PathBuf) -> AnalysisEffect<Vec<u8>> {
    let path_display = path.display().to_string();
    from_fn(move |env: &RealEnv| {
        env.file_system().read_bytes(&path).map_err(|e| {
            AnalysisError::io_with_path(
                format!(
                    "Failed to read bytes from '{}': {}",
                    path_display,
                    e.message()
                ),
                &path,
            )
        })
    })
    .boxed()
}

// ============================================================================
// File Write Operations
// ============================================================================

/// Write content to a file as an Effect.
///
/// Creates the file if it doesn't exist, or overwrites if it does.
/// Parent directories must exist.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::write_file_effect;
///
/// let effect = write_file_effect("output/report.md".into(), content);
/// run_effect(effect, DebtmapConfig::default())?;
/// ```
///
/// # Errors
///
/// Returns `AnalysisError::IoError` if:
/// - Permission is denied
/// - Parent directory doesn't exist
/// - Disk is full
pub fn write_file_effect(path: PathBuf, content: String) -> AnalysisEffect<()> {
    let path_display = path.display().to_string();
    from_fn(move |env: &RealEnv| {
        env.file_system().write(&path, &content).map_err(|e| {
            AnalysisError::io_with_path(
                format!("Failed to write file '{}': {}", path_display, e.message()),
                &path,
            )
        })
    })
    .boxed()
}

// ============================================================================
// File Existence Checks
// ============================================================================

/// Check if a file exists as an Effect.
///
/// This is a pure check that doesn't fail - it returns `true` if the path
/// exists and is a file, `false` otherwise.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::file_exists_effect;
///
/// let effect = file_exists_effect("Cargo.toml".into());
/// let exists = run_effect(effect, DebtmapConfig::default())?;
/// ```
pub fn file_exists_effect(path: PathBuf) -> AnalysisEffect<bool> {
    from_fn(move |env: &RealEnv| Ok(env.file_system().is_file(&path))).boxed()
}

/// Check if a path exists (file or directory) as an Effect.
pub fn path_exists_effect(path: PathBuf) -> AnalysisEffect<bool> {
    from_fn(move |env: &RealEnv| Ok(env.file_system().exists(&path))).boxed()
}

/// Check if a path is a directory as an Effect.
pub fn is_directory_effect(path: PathBuf) -> AnalysisEffect<bool> {
    from_fn(move |env: &RealEnv| Ok(env.file_system().is_dir(&path))).boxed()
}

// ============================================================================
// Directory Walking
// ============================================================================

/// Walk a directory and return all files as an Effect.
///
/// Uses the configured FileWalker with default settings. For more control
/// over which files are returned, use `walk_dir_with_config_effect`.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::walk_dir_effect;
///
/// let effect = walk_dir_effect("src".into());
/// let files = run_effect(effect, DebtmapConfig::default())?;
/// for file in files {
///     println!("{}", file.display());
/// }
/// ```
///
/// # Errors
///
/// Returns `AnalysisError::IoError` if:
/// - The directory doesn't exist
/// - Permission is denied
pub fn walk_dir_effect(path: PathBuf) -> AnalysisEffect<Vec<PathBuf>> {
    let path_display = path.display().to_string();
    from_fn(move |_env: &RealEnv| {
        FileWalker::new(path.clone()).walk().map_err(|e| {
            AnalysisError::io_with_path(
                format!("Failed to walk directory '{}': {}", path_display, e),
                &path,
            )
        })
    })
    .boxed()
}

/// Walk a directory with configuration-based ignore patterns.
///
/// This uses the environment's configuration to determine which files
/// to include or exclude during the walk.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::walk_dir_with_config_effect;
/// use debtmap::core::Language;
///
/// let effect = walk_dir_with_config_effect(
///     "src".into(),
///     vec![Language::Rust],
/// );
/// let rust_files = run_effect(effect, config)?;
/// ```
pub fn walk_dir_with_config_effect(
    path: PathBuf,
    languages: Vec<crate::core::Language>,
) -> AnalysisEffect<Vec<PathBuf>> {
    let path_display = path.display().to_string();
    from_fn(move |env: &RealEnv| {
        let ignore_patterns = env.config().get_ignore_patterns();

        FileWalker::new(path.clone())
            .with_languages(languages.clone())
            .with_ignore_patterns(ignore_patterns)
            .walk()
            .map_err(|e| {
                AnalysisError::io_with_path(
                    format!("Failed to walk directory '{}': {}", path_display, e),
                    &path,
                )
            })
    })
    .boxed()
}

// ============================================================================
// Cache Operations
// ============================================================================

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
                    Err(_) => {
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

// ============================================================================
// Composed Operations
// ============================================================================

/// Read a file only if it exists, returning None otherwise.
///
/// This is a composed effect that combines existence check with reading.
/// Useful when a file is optional.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::read_file_if_exists_effect;
///
/// let effect = read_file_if_exists_effect("optional.toml".into());
/// let content = run_effect(effect, config)?; // Ok(None) if file doesn't exist
/// ```
pub fn read_file_if_exists_effect(path: PathBuf) -> AnalysisEffect<Option<String>> {
    let path_clone = path.clone();
    file_exists_effect(path.clone())
        .and_then(move |exists| {
            if exists {
                read_file_effect(path_clone).map(Some).boxed()
            } else {
                pure(None).boxed()
            }
        })
        .boxed()
}

/// Read multiple files in sequence, collecting all contents.
///
/// If any file fails to read, the entire operation fails.
/// For parallel reading, use `read_files_parallel_effect`.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::read_files_effect;
///
/// let paths = vec!["src/main.rs".into(), "src/lib.rs".into()];
/// let effect = read_files_effect(paths);
/// let contents = run_effect(effect, config)?; // Vec<String>
/// ```
pub fn read_files_effect(paths: Vec<PathBuf>) -> AnalysisEffect<Vec<String>> {
    let mut effects: Vec<AnalysisEffect<String>> =
        paths.into_iter().map(read_file_effect).collect();

    // Sequence all effects
    if effects.is_empty() {
        return pure(Vec::new()).boxed();
    }

    let first = effects.remove(0);
    effects
        .into_iter()
        .fold(first.map(|s| vec![s]).boxed(), |acc, eff| {
            acc.and_then(move |mut results| {
                eff.map(move |s| {
                    results.push(s);
                    results
                })
                .boxed()
            })
            .boxed()
        })
}

// ============================================================================
// Batch Analysis Operations (Spec 203)
// ============================================================================

/// Walk a directory and analyze all files in parallel.
///
/// This is a composed effect that combines directory walking with
/// parallel file analysis using stillwater's traverse pattern.
///
/// # Arguments
///
/// * `path` - Root directory to analyze
/// * `languages` - Languages to include in analysis
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::walk_and_analyze_effect;
/// use debtmap::core::Language;
///
/// let effect = walk_and_analyze_effect(
///     "src".into(),
///     vec![Language::Rust],
/// );
/// let results = run_effect(effect, config)?;
/// for result in results {
///     println!("{}: {} functions", result.path.display(),
///         result.metrics.complexity.functions.len());
/// }
/// ```
pub fn walk_and_analyze_effect(
    path: PathBuf,
    languages: Vec<crate::core::Language>,
) -> AnalysisEffect<Vec<crate::analyzers::batch::FileAnalysisResult>> {
    walk_dir_with_config_effect(path, languages)
        .and_then(crate::analyzers::batch::analyze_files_effect)
        .boxed()
}

/// Walk a directory and validate all files, accumulating errors.
///
/// This effect validates all discovered files and accumulates ALL
/// validation errors instead of failing at the first one.
///
/// # Arguments
///
/// * `path` - Root directory to validate
/// * `languages` - Languages to include
///
/// # Returns
///
/// An Effect that produces a Validation result with either:
/// - All successfully validated files
/// - All validation errors accumulated
pub fn walk_and_validate_effect(
    path: PathBuf,
    languages: Vec<crate::core::Language>,
) -> AnalysisEffect<crate::effects::AnalysisValidation<Vec<crate::analyzers::batch::ValidatedFile>>>
{
    walk_dir_with_config_effect(path, languages)
        .map(|files| crate::analyzers::batch::validate_files(&files))
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::effects::run_effect;
    use tempfile::TempDir;

    fn create_test_env() -> (TempDir, DebtmapConfig) {
        let temp_dir = TempDir::new().unwrap();
        (temp_dir, DebtmapConfig::default())
    }

    #[test]
    fn test_read_file_effect_success() {
        let (temp_dir, config) = create_test_env();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!").unwrap();

        let effect = read_file_effect(file_path);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[test]
    fn test_read_file_effect_not_found() {
        let config = DebtmapConfig::default();
        let effect = read_file_effect("/nonexistent/path/file.txt".into());
        let result = run_effect(effect, config);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to read"));
    }

    #[test]
    fn test_write_file_effect() {
        let (temp_dir, config) = create_test_env();
        let file_path = temp_dir.path().join("output.txt");

        let effect = write_file_effect(file_path.clone(), "Test content".to_string());
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "Test content");
    }

    #[test]
    fn test_file_exists_effect() {
        let (temp_dir, config) = create_test_env();
        let file_path = temp_dir.path().join("exists.txt");
        std::fs::write(&file_path, "").unwrap();

        // File exists
        let effect = file_exists_effect(file_path);
        assert!(run_effect(effect, config.clone()).unwrap());

        // File doesn't exist
        let effect = file_exists_effect(temp_dir.path().join("nonexistent.txt"));
        assert!(!run_effect(effect, config).unwrap());
    }

    #[test]
    fn test_is_directory_effect() {
        let (temp_dir, config) = create_test_env();
        let dir_path = temp_dir.path().join("subdir");
        std::fs::create_dir(&dir_path).unwrap();

        let effect = is_directory_effect(dir_path);
        assert!(run_effect(effect, config).unwrap());
    }

    #[test]
    fn test_walk_dir_effect() {
        let (temp_dir, config) = create_test_env();
        let src_dir = temp_dir.path().join("src");
        std::fs::create_dir(&src_dir).unwrap();
        std::fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(src_dir.join("lib.rs"), "pub fn hello() {}").unwrap();

        let effect = walk_dir_effect(src_dir);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_read_file_if_exists_effect() {
        let (temp_dir, config) = create_test_env();
        let file_path = temp_dir.path().join("optional.txt");
        std::fs::write(&file_path, "content").unwrap();

        // File exists
        let effect = read_file_if_exists_effect(file_path);
        let result = run_effect(effect, config.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("content".to_string()));

        // File doesn't exist
        let effect = read_file_if_exists_effect(temp_dir.path().join("missing.txt"));
        let result = run_effect(effect, config);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_cache_operations() {
        use crate::effects::run_effect_with_env;
        use crate::env::RealEnv;

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

    #[test]
    fn test_read_files_effect() {
        let (temp_dir, config) = create_test_env();
        std::fs::write(temp_dir.path().join("a.txt"), "A").unwrap();
        std::fs::write(temp_dir.path().join("b.txt"), "B").unwrap();

        let paths = vec![temp_dir.path().join("a.txt"), temp_dir.path().join("b.txt")];
        let effect = read_files_effect(paths);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn test_read_files_effect_empty() {
        let config = DebtmapConfig::default();
        let effect = read_files_effect(vec![]);
        let result = run_effect(effect, config);

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
