//! Output destination abstractions for debtmap writers.
//!
//! This module provides the `OutputDestination` trait and implementations
//! for various output destinations (file, memory, stdout). This enables
//! testable, composable output operations.
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::io::destinations::{FileDestination, MemoryDestination};
//!
//! // Write to file
//! let file_dest = FileDestination::new("report.md".into());
//! file_dest.write_str("# Report")?;
//!
//! // Write to memory (for testing)
//! let mem_dest = MemoryDestination::new();
//! mem_dest.write_str("# Report")?;
//! let content = mem_dest.get_content();
//! assert!(content.contains("# Report"));
//! ```

use crate::errors::AnalysisError;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Trait for output destinations that can receive analysis output.
///
/// This trait abstracts over different output targets (files, memory buffers,
/// stdout), enabling testable and composable output operations.
pub trait OutputDestination: Send + Sync {
    /// Write string content to the destination.
    fn write_str(&self, content: &str) -> Result<(), AnalysisError>;

    /// Flush any buffered content.
    fn flush(&self) -> Result<(), AnalysisError>;

    /// Get a description of the destination for error messages.
    fn description(&self) -> String;
}

/// File system output destination.
///
/// Writes output directly to a file on the file system.
#[derive(Debug, Clone)]
pub struct FileDestination {
    path: PathBuf,
}

impl FileDestination {
    /// Create a new file destination.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Get the path this destination writes to.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl OutputDestination for FileDestination {
    fn write_str(&self, content: &str) -> Result<(), AnalysisError> {
        std::fs::write(&self.path, content).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to write to file: {}", e), &self.path)
        })
    }

    fn flush(&self) -> Result<(), AnalysisError> {
        // File writes are already flushed
        Ok(())
    }

    fn description(&self) -> String {
        format!("file:{}", self.path.display())
    }
}

/// In-memory output destination for testing.
///
/// Captures all output in a thread-safe buffer that can be inspected
/// after writing. This is primarily useful for testing output writers
/// without touching the file system.
#[derive(Debug, Clone)]
pub struct MemoryDestination {
    buffer: Arc<RwLock<String>>,
}

impl Default for MemoryDestination {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryDestination {
    /// Create a new in-memory destination with an empty buffer.
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(RwLock::new(String::new())),
        }
    }

    /// Get the current content of the buffer.
    pub fn get_content(&self) -> String {
        self.buffer.read().expect("RwLock poisoned").clone()
    }

    /// Clear the buffer.
    pub fn clear(&self) {
        self.buffer.write().expect("RwLock poisoned").clear();
    }

    /// Get the number of bytes written.
    pub fn len(&self) -> usize {
        self.buffer.read().expect("RwLock poisoned").len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.read().expect("RwLock poisoned").is_empty()
    }
}

impl OutputDestination for MemoryDestination {
    fn write_str(&self, content: &str) -> Result<(), AnalysisError> {
        self.buffer
            .write()
            .expect("RwLock poisoned")
            .push_str(content);
        Ok(())
    }

    fn flush(&self) -> Result<(), AnalysisError> {
        // In-memory destination doesn't need flushing
        Ok(())
    }

    fn description(&self) -> String {
        "memory".to_string()
    }
}

/// Standard output destination.
///
/// Writes output to stdout. Useful for CLI applications.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdoutDestination;

impl StdoutDestination {
    /// Create a new stdout destination.
    pub fn new() -> Self {
        Self
    }
}

impl OutputDestination for StdoutDestination {
    fn write_str(&self, content: &str) -> Result<(), AnalysisError> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        handle
            .write_all(content.as_bytes())
            .map_err(|e| AnalysisError::io(format!("Failed to write to stdout: {}", e)))?;
        Ok(())
    }

    fn flush(&self) -> Result<(), AnalysisError> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        handle
            .flush()
            .map_err(|e| AnalysisError::io(format!("Failed to flush stdout: {}", e)))
    }

    fn description(&self) -> String {
        "stdout".to_string()
    }
}

/// Standard error destination.
///
/// Writes output to stderr. Useful for error messages and diagnostics.
#[derive(Debug, Clone, Copy, Default)]
pub struct StderrDestination;

impl StderrDestination {
    /// Create a new stderr destination.
    pub fn new() -> Self {
        Self
    }
}

impl OutputDestination for StderrDestination {
    fn write_str(&self, content: &str) -> Result<(), AnalysisError> {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        handle
            .write_all(content.as_bytes())
            .map_err(|e| AnalysisError::io(format!("Failed to write to stderr: {}", e)))?;
        Ok(())
    }

    fn flush(&self) -> Result<(), AnalysisError> {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        handle
            .flush()
            .map_err(|e| AnalysisError::io(format!("Failed to flush stderr: {}", e)))
    }

    fn description(&self) -> String {
        "stderr".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_destination_write() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");

        let dest = FileDestination::new(path.clone());
        dest.write_str("Hello, World!").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_file_destination_description() {
        let dest = FileDestination::new(PathBuf::from("/tmp/test.md"));
        assert!(dest.description().contains("test.md"));
    }

    #[test]
    fn test_memory_destination_write() {
        let dest = MemoryDestination::new();

        dest.write_str("Hello").unwrap();
        dest.write_str(", World!").unwrap();

        assert_eq!(dest.get_content(), "Hello, World!");
    }

    #[test]
    fn test_memory_destination_clear() {
        let dest = MemoryDestination::new();
        dest.write_str("Some content").unwrap();

        assert!(!dest.is_empty());

        dest.clear();

        assert!(dest.is_empty());
        assert_eq!(dest.len(), 0);
    }

    #[test]
    fn test_memory_destination_thread_safe() {
        use std::thread;

        let dest = MemoryDestination::new();
        let dest_clone = dest.clone();

        let handle = thread::spawn(move || {
            dest_clone.write_str("Thread content").unwrap();
        });

        handle.join().unwrap();

        assert!(dest.get_content().contains("Thread content"));
    }

    #[test]
    fn test_stdout_destination_description() {
        let dest = StdoutDestination::new();
        assert_eq!(dest.description(), "stdout");
    }

    #[test]
    fn test_stderr_destination_description() {
        let dest = StderrDestination::new();
        assert_eq!(dest.description(), "stderr");
    }
}
