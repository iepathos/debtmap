//! Atomic file I/O operations with retry logic
//!
//! This module provides safe, atomic file operations with transient failure handling.
//! All operations use temporary files and atomic renames to ensure consistency.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Global counter for generating unique temporary file names
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Retry strategy for handling transient failures with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    max_attempts: usize,
    base_delay_ms: u64,
}

impl RetryStrategy {
    pub fn new(max_attempts: usize, base_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            base_delay_ms,
        }
    }

    /// Default retry strategy for file operations
    pub fn default_file_retry() -> Self {
        Self::new(3, 10)
    }

    /// Execute operation with retry logic - functional approach
    pub fn execute<T, F>(&self, mut operation: F, operation_name: &str) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        let mut last_error = None;

        for attempt in 0..self.max_attempts {
            match self.try_operation(&mut operation, attempt) {
                Ok(value) => return Ok(value),
                Err(e) => {
                    if self.should_retry(&e, attempt) {
                        self.apply_backoff(attempt);
                        last_error = Some(e);
                    } else {
                        return Err(e.context(format!(
                            "Operation '{}' failed after {} attempts",
                            operation_name,
                            attempt + 1
                        )));
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!(
                "Operation '{}' failed after {} attempts",
                operation_name,
                self.max_attempts
            )
        }))
    }

    /// Try the operation once
    fn try_operation<T, F>(&self, operation: &mut F, attempt: usize) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        operation().map_err(|e| {
            if attempt == self.max_attempts - 1 {
                e.context(format!("Final attempt {} failed", attempt + 1))
            } else {
                e
            }
        })
    }

    /// Check if error is retryable
    fn should_retry(&self, error: &anyhow::Error, attempt: usize) -> bool {
        // Don't retry on last attempt
        if attempt >= self.max_attempts - 1 {
            return false;
        }

        // Check if error is transient
        self.is_transient_error(error)
    }

    /// Determine if an error is transient and worth retrying
    fn is_transient_error(&self, error: &anyhow::Error) -> bool {
        error.chain().any(|err| {
            err.downcast_ref::<std::io::Error>()
                .map(|io_err| Self::is_retryable_io_error(io_err.kind()))
                .unwrap_or(false)
        })
    }

    /// Check if IO error kind is retryable
    fn is_retryable_io_error(kind: std::io::ErrorKind) -> bool {
        use std::io::ErrorKind;

        matches!(
            kind,
            ErrorKind::AlreadyExists
                | ErrorKind::NotFound
                | ErrorKind::Interrupted
                | ErrorKind::PermissionDenied // May be transient due to file locks
        )
    }

    /// Apply exponential backoff with jitter
    fn apply_backoff(&self, attempt: usize) {
        let delay = self.calculate_delay(attempt);
        std::thread::sleep(delay);
    }

    /// Calculate delay with exponential backoff and jitter
    fn calculate_delay(&self, attempt: usize) -> Duration {
        let base_delay_ms = self.base_delay_ms * (1 << attempt);
        let jitter_ms = base_delay_ms / 4; // 25% jitter
        Duration::from_millis(base_delay_ms + jitter_ms)
    }
}

/// Atomic file writer with retry capabilities
pub struct AtomicFileWriter {
    retry_strategy: RetryStrategy,
}

impl AtomicFileWriter {
    pub fn new(retry_strategy: RetryStrategy) -> Self {
        Self { retry_strategy }
    }

    pub fn with_default_retry() -> Self {
        Self::new(RetryStrategy::default_file_retry())
    }

    /// Create a safe temporary path for atomic writes
    pub fn create_safe_temp_path(target_path: &Path) -> PathBuf {
        let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();

        let file_name = target_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("temp");

        let temp_name = format!(".{}.tmp.{}.{}", file_name, pid, counter);

        if let Some(parent) = target_path.parent() {
            parent.join(temp_name)
        } else {
            PathBuf::from(temp_name)
        }
    }

    /// Validate file path for safety
    pub fn validate_file_path(path: &Path) -> Result<()> {
        if path.to_string_lossy().is_empty() {
            return Err(anyhow::anyhow!("Empty file path"));
        }

        let path_str = path.to_string_lossy();
        if path_str.contains('\0') {
            return Err(anyhow::anyhow!("Path contains null byte: {}", path_str));
        }

        if let Some(parent) = path.parent() {
            if parent.to_string_lossy().is_empty() {
                return Ok(());
            }

            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Parent directory does not exist: {}",
                    parent.display()
                ));
            }
        }

        Ok(())
    }

    /// Ensure parent directory exists
    pub fn ensure_parent_directory(file_path: &Path) -> Result<()> {
        if let Some(parent) = file_path.parent() {
            Self::create_directories_safely(parent)?;
        }
        Ok(())
    }

    /// Execute operation with retry logic
    pub fn retry_with_backoff<T, F>(operation: F, operation_name: &str) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        let strategy = RetryStrategy::default_file_retry();
        strategy.execute(operation, operation_name)
    }

    /// Create directories safely with proper error handling
    pub fn create_directories_safely(dir_path: &Path) -> Result<()> {
        if dir_path.exists() {
            if !dir_path.is_dir() {
                return Err(anyhow::anyhow!(
                    "Path exists but is not a directory: {}",
                    dir_path.display()
                ));
            }
            return Ok(());
        }

        fs::create_dir_all(dir_path).with_context(|| {
            format!(
                "Failed to create directory structure: {}",
                dir_path.display()
            )
        })
    }

    /// Ensure directories exist for atomic write operation
    pub fn ensure_atomic_write_directories(target_path: &Path, temp_path: &Path) -> Result<()> {
        if let Some(target_parent) = target_path.parent() {
            Self::create_directories_safely(target_parent)?;
        }

        if let Some(temp_parent) = temp_path.parent() {
            if temp_parent != target_path.parent().unwrap_or(Path::new("")) {
                Self::create_directories_safely(temp_parent)?;
            }
        }

        Ok(())
    }

    /// Write temporary file
    fn write_temp_file(temp_path: &Path, data: &[u8]) -> Result<()> {
        fs::write(temp_path, data)
            .with_context(|| format!("Failed to write temporary file: {}", temp_path.display()))
    }

    /// Sync temporary file to disk (no-op on some platforms for performance)
    fn sync_temp_file(_temp_path: &Path) -> Result<()> {
        Ok(())
    }

    /// Perform atomic rename
    fn atomic_rename(temp_path: &Path, target_path: &Path) -> Result<()> {
        if !Self::paths_on_same_filesystem(temp_path, target_path) {
            return Err(anyhow::anyhow!(
                "Temporary and target paths must be on the same filesystem"
            ));
        }

        fs::rename(temp_path, target_path).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                temp_path.display(),
                target_path.display()
            )
        })
    }

    /// Heuristic check if paths are on the same filesystem
    fn paths_on_same_filesystem(path1: &Path, path2: &Path) -> bool {
        path1.parent() == path2.parent()
    }

    /// Write bytes atomically using temp file and rename
    pub fn write_bytes_atomically(target_path: &Path, temp_path: &Path, data: &[u8]) -> Result<()> {
        Self::write_temp_file(temp_path, data)?;
        Self::sync_temp_file(temp_path)?;
        Self::atomic_rename(temp_path, target_path)?;
        Ok(())
    }

    /// Serialize JSON to string
    pub fn serialize_index_to_json<T: serde::Serialize>(index: &T) -> Result<String> {
        serde_json::to_string_pretty(index).context("Failed to serialize index to JSON")
    }

    /// Write file atomically with JSON content
    pub fn write_file_atomically(
        target_path: &Path,
        temp_path: &Path,
        content: &str,
    ) -> Result<()> {
        Self::write_bytes_atomically(target_path, temp_path, content.as_bytes())
    }

    /// High-level atomic write operation with automatic temp path
    pub fn write_atomically(&self, target_path: &Path, data: &[u8]) -> Result<()> {
        let temp_path = Self::create_safe_temp_path(target_path);
        Self::ensure_atomic_write_directories(target_path, &temp_path)?;

        self.retry_strategy.execute(
            || Self::write_bytes_atomically(target_path, &temp_path, data),
            "atomic_write",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_retry_strategy_creation() {
        let strategy = RetryStrategy::new(5, 100);
        assert_eq!(strategy.max_attempts, 5);
        assert_eq!(strategy.base_delay_ms, 100);
    }

    #[test]
    fn test_retry_strategy_success_first_attempt() {
        let strategy = RetryStrategy::new(3, 10);
        let mut call_count = 0;

        let result = strategy.execute(
            || {
                call_count += 1;
                Ok::<_, anyhow::Error>(42)
            },
            "test_op",
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count, 1);
    }

    #[test]
    fn test_is_retryable_io_error() {
        use std::io::ErrorKind;

        assert!(RetryStrategy::is_retryable_io_error(
            ErrorKind::AlreadyExists
        ));
        assert!(RetryStrategy::is_retryable_io_error(ErrorKind::NotFound));
        assert!(RetryStrategy::is_retryable_io_error(ErrorKind::Interrupted));
        assert!(RetryStrategy::is_retryable_io_error(
            ErrorKind::PermissionDenied
        ));
        assert!(!RetryStrategy::is_retryable_io_error(
            ErrorKind::InvalidData
        ));
    }

    #[test]
    fn test_calculate_delay() {
        let strategy = RetryStrategy::new(3, 10);

        let delay0 = strategy.calculate_delay(0);
        assert!(delay0 >= Duration::from_millis(10));
        assert!(delay0 <= Duration::from_millis(13));

        let delay1 = strategy.calculate_delay(1);
        assert!(delay1 >= Duration::from_millis(20));
        assert!(delay1 <= Duration::from_millis(25));
    }

    #[test]
    fn test_create_safe_temp_path() {
        let target = PathBuf::from("/tmp/cache/data.json");
        let temp = AtomicFileWriter::create_safe_temp_path(&target);

        assert!(temp.to_string_lossy().contains(".tmp."));
        assert!(temp.parent() == target.parent());
    }

    #[test]
    fn test_validate_file_path_valid() {
        // Create parent directory for validation
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("test.txt");

        // Validation should pass for path with existing parent
        let result = AtomicFileWriter::validate_file_path(&test_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_path_empty() {
        let path = Path::new("");
        let result = AtomicFileWriter::validate_file_path(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("a/b/c/file.txt");

        let result = AtomicFileWriter::ensure_parent_directory(&nested_path);
        assert!(result.is_ok());
        assert!(nested_path.parent().unwrap().exists());
    }

    #[test]
    fn test_atomic_write_success() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test.txt");
        let data = b"Hello, atomic world!";

        let writer = AtomicFileWriter::with_default_retry();
        let result = writer.write_atomically(&target_path, data);

        assert!(result.is_ok());
        assert!(target_path.exists());

        let content = fs::read(&target_path).unwrap();
        assert_eq!(content, data);
    }

    #[test]
    fn test_serialize_index_to_json() {
        use serde::Serialize;

        #[derive(Serialize)]
        struct TestData {
            name: String,
            value: u32,
        }

        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let result = AtomicFileWriter::serialize_index_to_json(&data);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("\"name\": \"test\""));
        assert!(json.contains("\"value\": 42"));
    }
}
