//! File I/O effect operations.
//!
//! This module provides Effect-based wrappers around basic file operations:
//! - Reading file contents (text and bytes)
//! - Writing file contents
//! - Checking file/path existence
//!
//! # Design Philosophy
//!
//! Following Stillwater's "Pure Core, Imperative Shell" pattern, all operations
//! are wrapped in Effect types that defer execution until run with an environment.
//! This enables testing with mock file systems while maintaining composability.

use crate::effects::AnalysisEffect;
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use std::path::PathBuf;
use stillwater::effect::prelude::*;

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
}
