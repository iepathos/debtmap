//! Bracket pattern for resource management (Spec 206).
//!
//! This module provides bracket-based resource management for debtmap operations,
//! ensuring proper cleanup regardless of success or failure. It wraps stillwater's
//! bracket pattern with debtmap-specific resource helpers.
//!
//! # Overview
//!
//! The bracket pattern guarantees cleanup even on error:
//! ```text
//! bracket(acquire, release, use_resource)
//! ```
//!
//! Resources are acquired, used, and then released in that order. The release
//! function runs even if the use function fails.
//!
//! # Available Resource Helpers
//!
//! - [`with_lock_file`]: Acquire exclusive access via lock file
//! - [`with_temp_dir`]: Create and cleanup temporary directory
//! - [`with_progress`]: Manage progress indicator lifecycle
//! - [`bracket_io`]: General bracket for I/O resources
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::resources::with_lock_file;
//! use debtmap::effects::effect_pure;
//!
//! let effect = with_lock_file(
//!     PathBuf::from("my.lock"),
//!     || effect_pure(42),
//! );
//!
//! // Lock is acquired, effect runs, lock is released even on error
//! ```

use crate::effects::AnalysisEffect;
use crate::env::RealEnv;
use crate::errors::AnalysisError;
use indicatif::ProgressBar;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use stillwater::effect::prelude::*;
use stillwater::{bracket, bracket_simple, Effect, EffectExt};

// =============================================================================
// Lock File Resource
// =============================================================================

/// Lock file resource for exclusive access.
///
/// This struct represents an acquired lock file. The file is created
/// on acquisition and deleted on release.
#[derive(Debug, Clone)]
pub struct LockFile {
    path: PathBuf,
}

impl LockFile {
    /// Get the path to the lock file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Acquire a lock file, run an effect, then release the lock.
///
/// This function provides exclusive access to a resource by creating a lock file.
/// The lock file is removed when the effect completes, even on error.
///
/// # Arguments
///
/// * `lock_path` - Path where the lock file will be created
/// * `effect_fn` - Function that creates the effect to run while holding the lock
///
/// # Errors
///
/// Returns an error if:
/// - The lock file already exists (another process may be running)
/// - The lock file cannot be created (permissions, disk full, etc.)
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::resources::with_lock_file;
/// use debtmap::effects::effect_pure;
///
/// fn analyze_with_lock(project: PathBuf) -> AnalysisEffect<i32> {
///     let lock_path = project.join(".debtmap.lock");
///     with_lock_file(lock_path, || effect_pure(42))
/// }
/// ```
pub fn with_lock_file<T, F, Eff>(lock_path: PathBuf, effect_fn: F) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce() -> Eff + Send + 'static,
    Eff: Effect<Output = T, Error = AnalysisError, Env = RealEnv> + Send + 'static,
{
    let acquire_path = lock_path.clone();

    bracket(
        // Acquire: Create the lock file (fails if exists)
        from_fn(move |_env: &RealEnv| {
            let _file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&acquire_path)
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::AlreadyExists {
                        AnalysisError::io_with_path(
                            "Lock file exists. Another process may be running.",
                            &acquire_path,
                        )
                    } else {
                        AnalysisError::io_with_path(
                            format!("Failed to create lock: {}", e),
                            &acquire_path,
                        )
                    }
                })?;
            Ok(LockFile { path: acquire_path })
        }),
        // Use: Run the provided effect
        move |_lock: LockFile| effect_fn(),
        // Release: Remove the lock file (ignore errors)
        |lock: LockFile| {
            from_fn(move |_env: &RealEnv| {
                let _ = std::fs::remove_file(&lock.path);
                Ok(())
            })
        },
    )
    .boxed()
}

// =============================================================================
// Temporary Directory Resource
// =============================================================================

/// Temporary directory resource.
///
/// This struct represents a temporary directory that will be cleaned up
/// when the bracket completes.
#[derive(Debug, Clone)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Get the path to the temporary directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Create a temporary directory, run an effect with it, then cleanup.
///
/// The temporary directory is created with a unique name using the given prefix.
/// It is automatically removed (along with all contents) when the effect completes.
///
/// # Arguments
///
/// * `prefix` - Prefix for the temporary directory name
/// * `effect_fn` - Function that receives the temp dir path and creates an effect
///
/// # Errors
///
/// Returns an error if the temporary directory cannot be created.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::resources::with_temp_dir;
/// use debtmap::effects::effect_pure;
///
/// fn process_with_staging(data: String) -> AnalysisEffect<i32> {
///     with_temp_dir("analysis", move |staging_path| {
///         // Use staging_path for intermediate files
///         // Directory is cleaned up automatically
///         effect_pure(42)
///     })
/// }
/// ```
pub fn with_temp_dir<T, F, Eff>(prefix: &str, effect_fn: F) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce(PathBuf) -> Eff + Send + 'static,
    Eff: Effect<Output = T, Error = AnalysisError, Env = RealEnv> + Send + 'static,
{
    let prefix = prefix.to_string();

    bracket(
        // Acquire: Create the temporary directory
        from_fn(move |_env: &RealEnv| {
            // Generate unique directory name
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let pid = std::process::id();
            let dir_name = format!("debtmap-{}-{}-{}", prefix, pid, timestamp);

            let temp_path = std::env::temp_dir().join(dir_name);

            std::fs::create_dir_all(&temp_path).map_err(|e| {
                AnalysisError::io_with_path(format!("Failed to create temp dir: {}", e), &temp_path)
            })?;

            Ok(TempDir { path: temp_path })
        }),
        // Use: Run the effect with the temp dir path
        move |temp: TempDir| effect_fn(temp.path.clone()),
        // Release: Remove the temp directory (ignore errors)
        |temp: TempDir| {
            from_fn(move |_env: &RealEnv| {
                let _ = std::fs::remove_dir_all(&temp.path);
                Ok(())
            })
        },
    )
    .boxed()
}

// =============================================================================
// Progress Indicator Resource
// =============================================================================

/// Progress indicator resource.
///
/// This struct wraps an indicatif ProgressBar and ensures it is properly
/// cleaned up when the bracket completes.
#[derive(Debug, Clone)]
pub struct ProgressHandle {
    bar: ProgressBar,
}

impl ProgressHandle {
    /// Get a reference to the progress bar.
    pub fn bar(&self) -> &ProgressBar {
        &self.bar
    }

    /// Increment the progress bar by the given amount.
    pub fn inc(&self, n: u64) {
        self.bar.inc(n);
    }

    /// Set the current position.
    pub fn set_position(&self, pos: u64) {
        self.bar.set_position(pos);
    }

    /// Update the message displayed.
    pub fn set_message(&self, msg: impl Into<std::borrow::Cow<'static, str>>) {
        self.bar.set_message(msg);
    }
}

/// Show a progress bar during an effect, cleanup on completion.
///
/// The progress bar is created with the given message and total count.
/// It is automatically cleaned up (finished and cleared) when the effect
/// completes, even on error.
///
/// # Arguments
///
/// * `message` - Message to display on the progress bar
/// * `total` - Total count for the progress bar
/// * `effect_fn` - Function that receives the progress handle and creates an effect
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::resources::with_progress;
/// use debtmap::effects::effect_pure;
///
/// fn process_files(files: Vec<PathBuf>) -> AnalysisEffect<i32> {
///     let file_count = files.len() as u64;
///     with_progress("Processing", file_count, move |progress| {
///         // Use progress.inc(1) as you process each file
///         effect_pure(42)
///     })
/// }
/// ```
pub fn with_progress<T, F, Eff>(message: &str, total: u64, effect_fn: F) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce(ProgressHandle) -> Eff + Send + 'static,
    Eff: Effect<Output = T, Error = AnalysisError, Env = RealEnv> + Send + 'static,
{
    let message = message.to_string();

    bracket_simple(
        // Acquire: Create the progress bar
        from_fn(move |_env: &RealEnv| {
            let bar = ProgressBar::new(total);
            bar.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{msg} [{bar:40}] {pos}/{len}")
                    .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar()),
            );
            bar.set_message(message);
            Ok(ProgressHandle { bar })
        }),
        // Use: Run the effect with the progress handle
        move |handle: ProgressHandle| effect_fn(handle.clone()),
        // Release: Finish and clear the progress bar (simple closure)
        |handle: ProgressHandle| {
            handle.bar.finish_and_clear();
        },
    )
    .boxed()
}

/// Show a spinner during an effect, cleanup on completion.
///
/// Unlike [`with_progress`], this shows an indeterminate spinner for
/// operations where the total count is unknown.
///
/// # Arguments
///
/// * `message` - Message to display with the spinner
/// * `effect_fn` - Function that creates the effect to run
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::resources::with_spinner;
/// use debtmap::effects::effect_pure;
///
/// fn long_running_task() -> AnalysisEffect<i32> {
///     with_spinner("Processing", || effect_pure(42))
/// }
/// ```
pub fn with_spinner<T, F, Eff>(message: &str, effect_fn: F) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce() -> Eff + Send + 'static,
    Eff: Effect<Output = T, Error = AnalysisError, Env = RealEnv> + Send + 'static,
{
    let message = message.to_string();

    bracket_simple(
        // Acquire: Create the spinner
        from_fn(move |_env: &RealEnv| {
            let bar = ProgressBar::new_spinner();
            bar.set_style(
                indicatif::ProgressStyle::default_spinner()
                    .template("{spinner} {msg}")
                    .unwrap_or_else(|_| indicatif::ProgressStyle::default_spinner()),
            );
            bar.set_message(message);
            bar.enable_steady_tick(std::time::Duration::from_millis(100));
            Ok(ProgressHandle { bar })
        }),
        // Use: Run the effect (doesn't need the handle for spinner)
        move |_handle: ProgressHandle| effect_fn(),
        // Release: Stop and clear the spinner
        |handle: ProgressHandle| {
            handle.bar.finish_and_clear();
        },
    )
    .boxed()
}

// =============================================================================
// File Handle Resource
// =============================================================================

/// File handle resource for bracket-based file I/O.
///
/// This struct wraps a file handle with `Arc` to make it cloneable,
/// which is required by stillwater's bracket pattern.
#[derive(Debug, Clone)]
pub struct FileHandle {
    file: std::sync::Arc<File>,
    path: PathBuf,
}

impl FileHandle {
    /// Get a reference to the file.
    ///
    /// Note: Since File is wrapped in Arc, multiple handles may share the same file.
    pub fn file(&self) -> &File {
        &self.file
    }

    /// Get the path to the file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Open a file for reading, run an effect, then close it.
///
/// The file is opened for reading and automatically closed when the
/// effect completes.
///
/// # Arguments
///
/// * `path` - Path to the file to open
/// * `effect_fn` - Function that receives the file handle and creates an effect
///
/// # Errors
///
/// Returns an error if the file cannot be opened.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::resources::with_file_read;
/// use debtmap::effects::effect_from_fn;
/// use std::io::Read;
///
/// fn read_file(path: PathBuf) -> AnalysisEffect<String> {
///     with_file_read(path, |handle| {
///         effect_from_fn(|_env| {
///             let mut content = String::new();
///             // Note: handle.file() is borrowed, need owned for this
///             // This is illustrative - real code would read differently
///             Ok(content)
///         })
///     })
/// }
/// ```
pub fn with_file_read<T, F, Eff>(path: PathBuf, effect_fn: F) -> AnalysisEffect<T>
where
    T: Send + 'static,
    F: FnOnce(FileHandle) -> Eff + Send + 'static,
    Eff: Effect<Output = T, Error = AnalysisError, Env = RealEnv> + Send + 'static,
{
    let open_path = path.clone();

    bracket_simple(
        // Acquire: Open the file
        from_fn(move |_env: &RealEnv| {
            let file = File::open(&open_path).map_err(|e| {
                AnalysisError::io_with_path(format!("Failed to open file: {}", e), &open_path)
            })?;
            Ok(FileHandle {
                file: std::sync::Arc::new(file),
                path: open_path,
            })
        }),
        // Use: Run the effect with the file handle
        move |handle: FileHandle| effect_fn(handle),
        // Release: File is dropped automatically (Rust handles this)
        |_handle: FileHandle| {},
    )
    .boxed()
}

// =============================================================================
// General Bracket Helper
// =============================================================================

/// General bracket for I/O resources with custom acquire/release.
///
/// This is a lower-level helper for cases where the predefined resource
/// helpers don't fit. It wraps stillwater's bracket with debtmap types.
///
/// # Type Parameters
///
/// * `R` - The resource type
/// * `T` - The result type
///
/// # Arguments
///
/// * `acquire_fn` - Function that acquires the resource
/// * `release_fn` - Function that releases the resource
/// * `use_fn` - Function that uses the resource
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::resources::bracket_io;
///
/// fn with_database_connection<T, F>(conn_string: &str, use_fn: F) -> AnalysisEffect<T>
/// where
///     T: Send + 'static,
///     F: FnOnce(DbConnection) -> AnalysisEffect<T> + Send + 'static,
/// {
///     bracket_io(
///         || connect_to_database(conn_string),
///         |conn| conn.close(),
///         use_fn,
///     )
/// }
/// ```
pub fn bracket_io<R, T, AcquireFn, ReleaseFn, UseFn, UseEff>(
    acquire_fn: AcquireFn,
    release_fn: ReleaseFn,
    use_fn: UseFn,
) -> AnalysisEffect<T>
where
    R: Clone + Send + 'static,
    T: Send + 'static,
    AcquireFn: FnOnce() -> Result<R, AnalysisError> + Send + 'static,
    ReleaseFn: FnOnce(R) + Send + 'static,
    UseFn: FnOnce(R) -> UseEff + Send + 'static,
    UseEff: Effect<Output = T, Error = AnalysisError, Env = RealEnv> + Send + 'static,
{
    bracket_simple(
        from_fn(move |_env: &RealEnv| acquire_fn()),
        move |resource: R| use_fn(resource),
        release_fn,
    )
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::effects::{effect_fail, effect_pure};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tempfile::TempDir as TempFileDir;

    // Helper to run effects in tests
    fn run_effect<T: Send + 'static>(effect: AnalysisEffect<T>) -> Result<T, AnalysisError> {
        let env = RealEnv::new(DebtmapConfig::default());
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(effect.run(&env))
    }

    // =========================================================================
    // Lock File Tests
    // =========================================================================

    #[test]
    fn test_with_lock_file_creates_and_removes_lock() {
        let temp = TempFileDir::new().unwrap();
        let lock_path = temp.path().join("test.lock");

        // Lock should not exist before
        assert!(!lock_path.exists());

        let effect = with_lock_file(lock_path.clone(), || effect_pure(42));
        let result = run_effect(effect);

        // Effect should succeed
        assert_eq!(result.unwrap(), 42);

        // Lock should be removed after
        assert!(!lock_path.exists());
    }

    #[test]
    fn test_with_lock_file_cleanup_on_error() {
        let temp = TempFileDir::new().unwrap();
        let lock_path = temp.path().join("test.lock");

        let effect: AnalysisEffect<i32> = with_lock_file(lock_path.clone(), || {
            effect_fail(AnalysisError::other("intentional failure"))
        });
        let result = run_effect(effect);

        // Effect should fail
        assert!(result.is_err());

        // Lock should still be removed
        assert!(!lock_path.exists(), "Lock file should be removed on error");
    }

    #[test]
    fn test_with_lock_file_fails_if_exists() {
        let temp = TempFileDir::new().unwrap();
        let lock_path = temp.path().join("test.lock");

        // Create lock file manually
        File::create(&lock_path).unwrap();

        let effect = with_lock_file(lock_path.clone(), || effect_pure(42));
        let result = run_effect(effect);

        // Should fail because lock exists
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Lock file exists"));
    }

    // =========================================================================
    // Temp Dir Tests
    // =========================================================================

    #[test]
    fn test_with_temp_dir_creates_and_removes() {
        let captured_path = Arc::new(std::sync::Mutex::new(None::<PathBuf>));
        let captured_clone = captured_path.clone();

        let effect = with_temp_dir("test", move |path| {
            *captured_clone.lock().unwrap() = Some(path.clone());
            // Verify directory exists during use
            assert!(path.exists());
            effect_pure(42)
        });
        let result = run_effect(effect);

        assert_eq!(result.unwrap(), 42);

        // Directory should be removed after
        let path = captured_path.lock().unwrap().clone().unwrap();
        assert!(!path.exists(), "Temp dir should be removed");
    }

    #[test]
    fn test_with_temp_dir_cleanup_on_error() {
        let captured_path = Arc::new(std::sync::Mutex::new(None::<PathBuf>));
        let captured_clone = captured_path.clone();

        let effect: AnalysisEffect<i32> = with_temp_dir("test", move |path| {
            *captured_clone.lock().unwrap() = Some(path.clone());
            effect_fail(AnalysisError::other("intentional failure"))
        });
        let result = run_effect(effect);

        assert!(result.is_err());

        // Directory should still be removed
        let path = captured_path.lock().unwrap().clone().unwrap();
        assert!(!path.exists(), "Temp dir should be removed on error");
    }

    #[test]
    fn test_with_temp_dir_unique_names() {
        let path1 = Arc::new(std::sync::Mutex::new(None::<PathBuf>));
        let path2 = Arc::new(std::sync::Mutex::new(None::<PathBuf>));

        let path1_clone = path1.clone();
        let path2_clone = path2.clone();

        let effect1 = with_temp_dir("test", move |path| {
            *path1_clone.lock().unwrap() = Some(path);
            effect_pure(1)
        });

        let effect2 = with_temp_dir("test", move |path| {
            *path2_clone.lock().unwrap() = Some(path);
            effect_pure(2)
        });

        run_effect(effect1).unwrap();
        run_effect(effect2).unwrap();

        let p1 = path1.lock().unwrap().clone().unwrap();
        let p2 = path2.lock().unwrap().clone().unwrap();

        // Paths should be different (unique)
        assert_ne!(p1, p2);
    }

    // =========================================================================
    // Progress Tests
    // =========================================================================

    #[test]
    fn test_with_progress_runs_effect() {
        let effect = with_progress("Testing", 100, |_handle| effect_pure(42));
        let result = run_effect(effect);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_with_progress_cleanup_on_error() {
        let effect: AnalysisEffect<i32> = with_progress("Testing", 100, |_handle| {
            effect_fail(AnalysisError::other("intentional failure"))
        });
        let result = run_effect(effect);

        // Effect should fail but not panic (cleanup happened)
        assert!(result.is_err());
    }

    #[test]
    fn test_with_spinner_runs_effect() {
        let effect = with_spinner("Processing", || effect_pure("done"));
        let result = run_effect(effect);
        assert_eq!(result.unwrap(), "done");
    }

    // =========================================================================
    // File Handle Tests
    // =========================================================================

    #[test]
    fn test_with_file_read_opens_and_closes() {
        let temp = TempFileDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let effect = with_file_read(file_path.clone(), |handle| {
            // File should be accessible
            assert!(handle.path().exists());
            effect_pure(42)
        });
        let result = run_effect(effect);

        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_with_file_read_fails_for_nonexistent() {
        let effect = with_file_read(PathBuf::from("/nonexistent/file.txt"), |_handle| {
            effect_pure(42)
        });
        let result = run_effect(effect);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to open"));
    }

    // =========================================================================
    // General Bracket Tests
    // =========================================================================

    #[test]
    fn test_bracket_io_custom_resource() {
        let acquired = Arc::new(AtomicBool::new(false));
        let released = Arc::new(AtomicBool::new(false));

        let acquired_clone = acquired.clone();
        let released_clone = released.clone();

        let effect = bracket_io(
            move || {
                acquired_clone.store(true, Ordering::SeqCst);
                Ok("resource".to_string())
            },
            move |_resource| {
                released_clone.store(true, Ordering::SeqCst);
            },
            |resource| {
                assert_eq!(resource, "resource");
                effect_pure(42)
            },
        );

        let result = run_effect(effect);

        assert_eq!(result.unwrap(), 42);
        assert!(acquired.load(Ordering::SeqCst));
        assert!(released.load(Ordering::SeqCst));
    }

    #[test]
    fn test_bracket_io_releases_on_error() {
        let released = Arc::new(AtomicBool::new(false));
        let released_clone = released.clone();

        let effect: AnalysisEffect<i32> = bracket_io(
            || Ok("resource".to_string()),
            move |_resource| {
                released_clone.store(true, Ordering::SeqCst);
            },
            |_resource| effect_fail(AnalysisError::other("intentional failure")),
        );

        let result = run_effect(effect);

        assert!(result.is_err());
        assert!(
            released.load(Ordering::SeqCst),
            "Resource should be released on error"
        );
    }

    // =========================================================================
    // Nested Resources Tests
    // =========================================================================

    #[test]
    fn test_nested_resources() {
        let temp = TempFileDir::new().unwrap();
        let lock_path = temp.path().join("test.lock");

        let inner_ran = Arc::new(AtomicBool::new(false));
        let inner_ran_clone = inner_ran.clone();

        // Nested: lock file -> temp dir -> effect
        let effect = with_lock_file(lock_path.clone(), move || {
            with_temp_dir("nested", move |_temp_path| {
                inner_ran_clone.store(true, Ordering::SeqCst);
                effect_pure(42)
            })
        });

        let result = run_effect(effect);

        assert_eq!(result.unwrap(), 42);
        assert!(inner_ran.load(Ordering::SeqCst));
        assert!(!lock_path.exists(), "Lock should be cleaned up");
    }
}
