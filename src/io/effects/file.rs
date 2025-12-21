//! File I/O effect operations.
//!
//! This module provides Effect-based wrappers around basic file operations:
//! - Reading file contents (text and bytes)
//! - Writing file contents
//! - Checking file/path existence
//! - Multi-file validation with error accumulation (Spec 003)
//!
//! # Design Philosophy
//!
//! Following Stillwater's "Pure Core, Imperative Shell" pattern, all operations
//! are wrapped in Effect types that defer execution until run with an environment.
//! This enables testing with mock file systems while maintaining composability.
//!
//! # Multi-File Validation (Spec 003)
//!
//! For batch file operations where partial success is acceptable, use
//! `read_files_with_accumulation` to collect all errors:
//!
//! ```rust,ignore
//! use debtmap::io::effects::read_files_with_accumulation;
//!
//! let paths = vec!["a.rs".into(), "b.rs".into(), "bad.rs".into()];
//! let result = run_effect(read_files_with_accumulation(paths), config)?;
//!
//! // Some files may have failed, but we got what we could
//! println!("Loaded {} files, {} errors", result.valid.len(), result.errors.len());
//! ```

use crate::effects::validation::{FileError, ValidatedFileSet};
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

// =============================================================================
// Multi-File Validation with Error Accumulation (Spec 003)
// =============================================================================

/// File content with its source path.
#[derive(Clone, Debug)]
pub struct FileContent {
    /// Path to the file.
    pub path: PathBuf,
    /// Contents of the file.
    pub content: String,
}

impl FileContent {
    /// Create a new FileContent.
    pub fn new(path: PathBuf, content: String) -> Self {
        Self { path, content }
    }
}

/// Read multiple files, accumulating errors instead of failing fast.
///
/// Unlike sequential file reading that fails at the first error, this function
/// attempts to read all files and collects successes and failures separately.
/// This is useful for batch operations where you want to process as many files
/// as possible even if some fail.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::read_files_with_accumulation;
///
/// let paths = vec!["good.rs".into(), "missing.rs".into(), "also_good.rs".into()];
/// let effect = read_files_with_accumulation(paths);
/// let result = run_effect(effect, config)?;
///
/// // Even though missing.rs failed, we got the other two files
/// assert_eq!(result.valid.len(), 2);
/// assert_eq!(result.errors.len(), 1);
/// ```
///
/// # Error Handling
///
/// Files that fail to read are added to the `errors` field with:
/// - The file path
/// - An error message describing the failure
/// - An error code for programmatic handling
///
/// The function never fails itself - it always returns a `ValidatedFileSet`.
pub fn read_files_with_accumulation(
    paths: Vec<PathBuf>,
) -> AnalysisEffect<ValidatedFileSet<FileContent>> {
    from_fn(move |env: &RealEnv| {
        let mut result = ValidatedFileSet::empty();

        for path in &paths {
            match env.file_system().read_to_string(path) {
                Ok(content) => {
                    result.add_valid(FileContent::new(path.clone(), content));
                }
                Err(e) => {
                    result.add_error(
                        FileError::new(path.clone(), format!("Failed to read: {}", e.message()))
                            .with_code("E001"),
                    );
                }
            }
        }

        Ok(result)
    })
    .boxed()
}

/// Read multiple files in parallel, accumulating errors.
///
/// Like `read_files_with_accumulation` but uses parallel execution for
/// better performance with many files.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::read_files_parallel_with_accumulation;
///
/// let paths = vec!["a.rs".into(), "b.rs".into(), "c.rs".into()];
/// let effect = read_files_parallel_with_accumulation(paths);
/// let result = run_effect_async(effect, config).await?;
/// ```
pub fn read_files_parallel_with_accumulation(
    paths: Vec<PathBuf>,
) -> AnalysisEffect<ValidatedFileSet<FileContent>> {
    from_async(move |env: &RealEnv| {
        let env = env.clone();
        let paths = paths.clone();

        async move {
            use tokio::task::JoinSet;

            let mut result = ValidatedFileSet::empty();
            let mut join_set = JoinSet::new();

            for path in paths {
                let env_clone = env.clone();
                let path_clone = path.clone();
                join_set.spawn(async move {
                    let read_result = env_clone.file_system().read_to_string(&path_clone);
                    (path_clone, read_result)
                });
            }

            while let Some(task_result) = join_set.join_next().await {
                match task_result {
                    Ok((path, read_result)) => match read_result {
                        Ok(content) => {
                            result.add_valid(FileContent::new(path, content));
                        }
                        Err(e) => {
                            result.add_error(
                                FileError::new(path, format!("Failed to read: {}", e.message()))
                                    .with_code("E001"),
                            );
                        }
                    },
                    Err(e) => {
                        // Task panicked or was cancelled
                        result.add_error(
                            FileError::new(
                                PathBuf::from("<unknown>"),
                                format!("Task error: {}", e),
                            )
                            .with_code("E900"),
                        );
                    }
                }
            }

            Ok(result)
        }
    })
    .boxed()
}

/// Process files with a custom function, accumulating errors.
///
/// This is a more flexible version that lets you provide a custom processing
/// function for each file.
///
/// # Type Parameters
///
/// - `T`: The output type for each successfully processed file
/// - `F`: The processing function
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::effects::process_files_with_accumulation;
///
/// // Parse each file as JSON
/// let effect = process_files_with_accumulation(paths, |path, content| {
///     serde_json::from_str(&content)
///         .map_err(|e| format!("JSON parse error: {}", e))
/// });
/// ```
pub fn process_files_with_accumulation<T, F>(
    paths: Vec<PathBuf>,
    processor: F,
) -> AnalysisEffect<ValidatedFileSet<T>>
where
    T: Send + 'static,
    F: Fn(&PathBuf, &str) -> Result<T, String> + Send + Sync + 'static,
{
    use std::sync::Arc;

    let processor = Arc::new(processor);

    from_fn(move |env: &RealEnv| {
        let mut result = ValidatedFileSet::empty();

        for path in &paths {
            match env.file_system().read_to_string(path) {
                Ok(content) => match processor(path, &content) {
                    Ok(processed) => {
                        result.add_valid(processed);
                    }
                    Err(e) => {
                        result.add_error(FileError::new(path.clone(), e).with_code("E010"));
                    }
                },
                Err(e) => {
                    result.add_error(
                        FileError::new(path.clone(), format!("Failed to read: {}", e.message()))
                            .with_code("E001"),
                    );
                }
            }
        }

        Ok(result)
    })
    .boxed()
}

/// Convert a ValidatedFileSet's strict result to an AnalysisEffect.
///
/// This helper converts the strict mode result (any error = failure) to
/// an Effect error, useful when you want fail-fast behavior but still
/// want all errors reported.
pub fn validated_file_set_to_strict_effect<T: Send + 'static>(
    file_set: ValidatedFileSet<T>,
) -> Result<Vec<T>, AnalysisError> {
    file_set.into_strict_result().map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        AnalysisError::multi_file(messages)
    })
}

/// Convert a ValidatedFileSet's lenient result to an AnalysisEffect.
///
/// This helper converts the lenient mode result (only fail if all files failed)
/// to an Effect error.
pub fn validated_file_set_to_lenient_effect<T: Send + 'static>(
    file_set: ValidatedFileSet<T>,
) -> Result<Vec<T>, AnalysisError> {
    file_set.into_lenient_result().map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        AnalysisError::multi_file(messages)
    })
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

    // =========================================================================
    // Multi-File Validation Tests (Spec 003)
    // =========================================================================

    #[test]
    fn test_read_files_with_accumulation_all_success() {
        let (temp_dir, config) = create_test_env();

        // Create test files
        std::fs::write(temp_dir.path().join("a.txt"), "content a").unwrap();
        std::fs::write(temp_dir.path().join("b.txt"), "content b").unwrap();
        std::fs::write(temp_dir.path().join("c.txt"), "content c").unwrap();

        let paths = vec![
            temp_dir.path().join("a.txt"),
            temp_dir.path().join("b.txt"),
            temp_dir.path().join("c.txt"),
        ];

        let effect = read_files_with_accumulation(paths);
        let result = run_effect(effect, config).unwrap();

        assert!(result.is_all_success());
        assert_eq!(result.valid.len(), 3);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_read_files_with_accumulation_partial_success() {
        let (temp_dir, config) = create_test_env();

        // Create only some files
        std::fs::write(temp_dir.path().join("good1.txt"), "content 1").unwrap();
        std::fs::write(temp_dir.path().join("good2.txt"), "content 2").unwrap();

        let paths = vec![
            temp_dir.path().join("good1.txt"),
            temp_dir.path().join("missing.txt"), // doesn't exist
            temp_dir.path().join("good2.txt"),
            temp_dir.path().join("also_missing.txt"), // doesn't exist
        ];

        let effect = read_files_with_accumulation(paths);
        let result = run_effect(effect, config).unwrap();

        assert!(result.is_partial_success());
        assert_eq!(result.valid.len(), 2);
        assert_eq!(result.errors.len(), 2);

        // Check that errors have the correct paths
        let error_paths: Vec<_> = result.errors.iter().map(|e| e.path.clone()).collect();
        assert!(error_paths
            .iter()
            .any(|p| p.file_name().unwrap() == "missing.txt"));
        assert!(error_paths
            .iter()
            .any(|p| p.file_name().unwrap() == "also_missing.txt"));
    }

    #[test]
    fn test_read_files_with_accumulation_all_failed() {
        let config = DebtmapConfig::default();

        let paths = vec![
            PathBuf::from("/nonexistent/a.txt"),
            PathBuf::from("/nonexistent/b.txt"),
        ];

        let effect = read_files_with_accumulation(paths);
        let result = run_effect(effect, config).unwrap();

        assert!(result.is_all_failed());
        assert_eq!(result.valid.len(), 0);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_read_files_with_accumulation_empty() {
        let config = DebtmapConfig::default();
        let paths: Vec<PathBuf> = vec![];

        let effect = read_files_with_accumulation(paths);
        let result = run_effect(effect, config).unwrap();

        assert!(!result.is_partial_success());
        assert!(!result.is_all_success()); // No files = not a success
        assert!(!result.is_all_failed()); // No errors = not a failure
        assert_eq!(result.valid.len(), 0);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_process_files_with_accumulation() {
        let (temp_dir, config) = create_test_env();

        // Create test files with numbers
        std::fs::write(temp_dir.path().join("num1.txt"), "42").unwrap();
        std::fs::write(temp_dir.path().join("num2.txt"), "not a number").unwrap(); // parse will fail
        std::fs::write(temp_dir.path().join("num3.txt"), "100").unwrap();

        let paths = vec![
            temp_dir.path().join("num1.txt"),
            temp_dir.path().join("num2.txt"),
            temp_dir.path().join("num3.txt"),
        ];

        let effect = process_files_with_accumulation(paths, |_path, content| {
            content
                .trim()
                .parse::<i32>()
                .map_err(|e| format!("Parse error: {}", e))
        });

        let result = run_effect(effect, config).unwrap();

        assert!(result.is_partial_success());
        assert_eq!(result.valid.len(), 2);
        assert_eq!(result.errors.len(), 1);
        assert!(result.valid.contains(&42));
        assert!(result.valid.contains(&100));
    }

    #[test]
    fn test_validated_file_set_to_strict_effect() {
        let mut file_set: ValidatedFileSet<String> = ValidatedFileSet::empty();
        file_set.add_valid("good".to_string());
        file_set.add_error(FileError::new("bad.txt", "error"));

        let result = validated_file_set_to_strict_effect(file_set);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("bad.txt"));
    }

    #[test]
    fn test_validated_file_set_to_lenient_effect() {
        let mut file_set: ValidatedFileSet<String> = ValidatedFileSet::empty();
        file_set.add_valid("good".to_string());
        file_set.add_error(FileError::new("bad.txt", "error"));

        let result = validated_file_set_to_lenient_effect(file_set);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["good".to_string()]);
    }

    #[test]
    fn test_file_content_struct() {
        let fc = FileContent::new(PathBuf::from("test.txt"), "content".to_string());
        assert_eq!(fc.path, PathBuf::from("test.txt"));
        assert_eq!(fc.content, "content");
    }
}
