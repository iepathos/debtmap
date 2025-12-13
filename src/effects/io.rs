//! File I/O effects for the analysis pipeline.
//!
//! This module provides effect-based wrappers for file system operations,
//! enabling testability and composability. All I/O operations are performed
//! through the environment's file system trait, allowing mock implementations
//! in tests.
//!
//! # Design Philosophy
//!
//! Following the "pure core, imperative shell" pattern:
//! - File I/O is isolated in effect wrappers
//! - Core analysis logic remains pure
//! - Test environments can provide mock file systems
//!
//! # Available Effects
//!
//! | Effect | Description |
//! |--------|-------------|
//! | [`read_file_effect`] | Read file contents as UTF-8 string |
//! | [`read_file_bytes_effect`] | Read file contents as raw bytes |
//! | [`file_exists_effect`] | Check if a file exists |
//! | [`is_file_effect`] | Check if path is a file |
//! | [`is_dir_effect`] | Check if path is a directory |
//! | [`write_file_effect`] | Write string content to file |
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::effects::io::{read_file_effect, file_exists_effect};
//! use debtmap::env::RealEnv;
//!
//! async fn analyze_if_exists(path: PathBuf) -> AnalysisEffect<Option<String>> {
//!     file_exists_effect(path.clone())
//!         .and_then(move |exists| {
//!             if exists {
//!                 read_file_effect(path).map(Some)
//!             } else {
//!                 effect_pure(None)
//!             }
//!         })
//! }
//! ```

use crate::env::AnalysisEnv;
use crate::errors::AnalysisError;
use std::path::PathBuf;
use stillwater::effect::prelude::*;
use stillwater::Effect;

/// Read a file's contents as a UTF-8 string.
///
/// This effect reads the entire file into memory as a string. For large files
/// or binary content, consider using [`read_file_bytes_effect`] instead.
///
/// # Arguments
///
/// * `path` - The path to the file to read
///
/// # Returns
///
/// An effect that produces the file contents as a string.
///
/// # Errors
///
/// Returns `AnalysisError::IoError` if:
/// - The file doesn't exist
/// - Permission is denied
/// - The file isn't valid UTF-8
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::io::read_file_effect;
///
/// let effect = read_file_effect("src/main.rs".into());
/// let content = effect.run(&env).await?;
/// ```
pub fn read_file_effect<Env>(
    path: PathBuf,
) -> impl Effect<Output = String, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
{
    from_fn(move |env: &Env| {
        env.file_system()
            .read_to_string(&path)
            .map_err(|e| match e {
                AnalysisError::IoError { message, path: _ } => {
                    AnalysisError::io_with_path(message, &path)
                }
                other => other,
            })
    })
}

/// Read a file's contents as raw bytes.
///
/// This effect reads the entire file into memory as a byte vector. Use this
/// for binary files or when UTF-8 validation isn't needed.
///
/// # Arguments
///
/// * `path` - The path to the file to read
///
/// # Returns
///
/// An effect that produces the file contents as bytes.
///
/// # Example
///
/// ```rust,ignore
/// let effect = read_file_bytes_effect("image.png".into());
/// let bytes = effect.run(&env).await?;
/// ```
pub fn read_file_bytes_effect<Env>(
    path: PathBuf,
) -> impl Effect<Output = Vec<u8>, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
{
    from_fn(move |env: &Env| {
        env.file_system().read_bytes(&path).map_err(|e| match e {
            AnalysisError::IoError { message, path: _ } => {
                AnalysisError::io_with_path(message, &path)
            }
            other => other,
        })
    })
}

/// Check if a file or directory exists.
///
/// This effect checks for the existence of a path without reading its contents.
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Returns
///
/// An effect that produces `true` if the path exists, `false` otherwise.
///
/// # Example
///
/// ```rust,ignore
/// let effect = file_exists_effect("config.toml".into());
/// let exists = effect.run(&env).await?;
/// if exists {
///     // Load config...
/// }
/// ```
pub fn file_exists_effect<Env>(
    path: PathBuf,
) -> impl Effect<Output = bool, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
{
    stillwater::asks(move |env: &Env| env.file_system().exists(&path))
}

/// Check if a path is a file.
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Returns
///
/// An effect that produces `true` if the path is a file, `false` otherwise.
pub fn is_file_effect<Env>(
    path: PathBuf,
) -> impl Effect<Output = bool, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
{
    stillwater::asks(move |env: &Env| env.file_system().is_file(&path))
}

/// Check if a path is a directory.
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Returns
///
/// An effect that produces `true` if the path is a directory, `false` otherwise.
pub fn is_dir_effect<Env>(
    path: PathBuf,
) -> impl Effect<Output = bool, Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
{
    stillwater::asks(move |env: &Env| env.file_system().is_dir(&path))
}

/// Write string content to a file.
///
/// This effect creates or overwrites a file with the given content.
///
/// # Arguments
///
/// * `path` - The path to write to
/// * `content` - The string content to write
///
/// # Returns
///
/// An effect that produces `()` on success.
///
/// # Errors
///
/// Returns `AnalysisError::IoError` if:
/// - Permission is denied
/// - Parent directory doesn't exist
/// - Disk is full
///
/// # Example
///
/// ```rust,ignore
/// let effect = write_file_effect("output.txt".into(), "analysis results".into());
/// effect.run(&env).await?;
/// ```
pub fn write_file_effect<Env>(
    path: PathBuf,
    content: String,
) -> impl Effect<Output = (), Error = AnalysisError, Env = Env>
where
    Env: AnalysisEnv + Clone + Send + Sync + 'static,
{
    from_fn(move |env: &Env| {
        env.file_system()
            .write(&path, &content)
            .map_err(|e| match e {
                AnalysisError::IoError { message, path: _ } => {
                    AnalysisError::io_with_path(message, &path)
                }
                other => other,
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::env::RealEnv;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file_effect_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!").unwrap();

        let env = RealEnv::new(DebtmapConfig::default());
        let effect = read_file_effect::<RealEnv>(file_path);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[tokio::test]
    async fn test_read_file_effect_not_found() {
        let env = RealEnv::new(DebtmapConfig::default());
        let effect = read_file_effect::<RealEnv>("/nonexistent/file.txt".into());
        let result = effect.run(&env).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.category(), "I/O");
    }

    #[tokio::test]
    async fn test_read_file_bytes_effect() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");
        let bytes = vec![0u8, 1, 2, 3, 4, 255];
        std::fs::write(&file_path, &bytes).unwrap();

        let env = RealEnv::new(DebtmapConfig::default());
        let effect = read_file_bytes_effect::<RealEnv>(file_path);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), bytes);
    }

    #[tokio::test]
    async fn test_file_exists_effect_true() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("exists.txt");
        std::fs::write(&file_path, "content").unwrap();

        let env = RealEnv::new(DebtmapConfig::default());
        let effect = file_exists_effect::<RealEnv>(file_path);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_file_exists_effect_false() {
        let env = RealEnv::new(DebtmapConfig::default());
        let effect = file_exists_effect::<RealEnv>("/definitely/not/here.txt".into());
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_is_file_effect() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        std::fs::write(&file_path, "content").unwrap();

        let env = RealEnv::new(DebtmapConfig::default());

        // Check file
        let effect = is_file_effect::<RealEnv>(file_path);
        assert!(effect.run(&env).await.unwrap());

        // Check directory (should be false)
        let effect = is_file_effect::<RealEnv>(temp_dir.path().to_path_buf());
        assert!(!effect.run(&env).await.unwrap());
    }

    #[tokio::test]
    async fn test_is_dir_effect() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        std::fs::write(&file_path, "content").unwrap();

        let env = RealEnv::new(DebtmapConfig::default());

        // Check directory
        let effect = is_dir_effect::<RealEnv>(temp_dir.path().to_path_buf());
        assert!(effect.run(&env).await.unwrap());

        // Check file (should be false)
        let effect = is_dir_effect::<RealEnv>(file_path);
        assert!(!effect.run(&env).await.unwrap());
    }

    #[tokio::test]
    async fn test_write_file_effect() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("output.txt");

        let env = RealEnv::new(DebtmapConfig::default());
        let effect = write_file_effect::<RealEnv>(file_path.clone(), "test content".into());
        let result = effect.run(&env).await;

        assert!(result.is_ok());

        // Verify the file was written
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_write_file_effect_overwrites() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("overwrite.txt");
        std::fs::write(&file_path, "original").unwrap();

        let env = RealEnv::new(DebtmapConfig::default());
        let effect = write_file_effect::<RealEnv>(file_path.clone(), "new content".into());
        effect.run(&env).await.unwrap();

        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content");
    }
}
