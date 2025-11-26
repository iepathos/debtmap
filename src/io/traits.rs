//! I/O trait definitions for debtmap analysis operations.
//!
//! This module defines traits that abstract over I/O operations, enabling:
//! - Testability through mock implementations
//! - Separation of pure analysis logic from I/O side effects
//! - Dependency injection for the effect system
//!
//! # Design Philosophy
//!
//! The "pure core, imperative shell" pattern requires separating I/O from
//! business logic. These traits define the I/O capabilities that analysis
//! operations need, allowing the core logic to remain pure while I/O is
//! handled at the boundaries.
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::io::traits::FileSystem;
//!
//! fn analyze_with_fs<F: FileSystem>(fs: &F, path: &Path) -> Result<Metrics> {
//!     let content = fs.read_to_string(path)?;
//!     // ... pure analysis logic ...
//! }
//! ```

use crate::errors::AnalysisError;
use std::path::Path;

/// File system operations trait.
///
/// This trait abstracts over file system operations, enabling:
/// - Unit testing with mock file systems
/// - Virtual file system implementations
/// - Read-only or sandboxed file access
///
/// # Implementation Notes
///
/// Implementations should be thread-safe (`Send + Sync`) to support
/// parallel analysis across multiple files.
pub trait FileSystem: Send + Sync {
    /// Read a file's contents as a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns `AnalysisError::IoError` if:
    /// - The file doesn't exist
    /// - Permission is denied
    /// - The file isn't valid UTF-8
    fn read_to_string(&self, path: &Path) -> Result<String, AnalysisError>;

    /// Write content to a file, creating it if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns `AnalysisError::IoError` if:
    /// - Permission is denied
    /// - Parent directory doesn't exist
    /// - Disk is full
    fn write(&self, path: &Path, content: &str) -> Result<(), AnalysisError>;

    /// Check if a path exists (file or directory).
    fn exists(&self, path: &Path) -> bool;

    /// Check if a path is a file.
    fn is_file(&self, path: &Path) -> bool;

    /// Check if a path is a directory.
    fn is_dir(&self, path: &Path) -> bool;

    /// Read a file's contents as raw bytes.
    ///
    /// # Errors
    ///
    /// Returns `AnalysisError::IoError` if:
    /// - The file doesn't exist
    /// - Permission is denied
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, AnalysisError>;
}

/// Coverage data loading trait.
///
/// This trait abstracts over loading coverage data from various formats
/// (LCOV, Cobertura, etc.), enabling:
/// - Testing with mock coverage data
/// - Support for multiple coverage formats
/// - Lazy loading optimizations
///
/// # Coverage Data Model
///
/// Coverage is represented as line-level hit counts, which can be
/// aggregated to function-level or file-level coverage percentages.
pub trait CoverageLoader: Send + Sync {
    /// Load coverage data from an LCOV format file.
    ///
    /// # Errors
    ///
    /// Returns `AnalysisError::CoverageError` if:
    /// - The file doesn't exist
    /// - The file isn't valid LCOV format
    /// - The file is corrupted
    fn load_lcov(&self, path: &Path) -> Result<CoverageData, AnalysisError>;

    /// Load coverage data from a Cobertura XML file.
    ///
    /// # Errors
    ///
    /// Returns `AnalysisError::CoverageError` if:
    /// - The file doesn't exist
    /// - The file isn't valid Cobertura XML
    fn load_cobertura(&self, path: &Path) -> Result<CoverageData, AnalysisError>;
}

/// Coverage data container.
///
/// This is a simplified representation of coverage data that can be
/// queried for file-level and function-level coverage.
#[derive(Debug, Clone, Default)]
pub struct CoverageData {
    /// File path to line coverage mapping
    file_coverage: std::collections::HashMap<std::path::PathBuf, FileCoverage>,
}

/// Coverage data for a single file.
#[derive(Debug, Clone, Default)]
pub struct FileCoverage {
    /// Line number to hit count mapping (1-indexed)
    line_hits: std::collections::HashMap<usize, u64>,
    /// Total lines that are executable
    pub total_lines: usize,
    /// Total lines that were hit
    pub hit_lines: usize,
}

impl CoverageData {
    /// Create empty coverage data.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get coverage percentage for a file (0.0 to 100.0).
    pub fn get_file_coverage(&self, path: &Path) -> Option<f64> {
        self.file_coverage.get(path).map(|fc| {
            if fc.total_lines == 0 {
                100.0
            } else {
                (fc.hit_lines as f64 / fc.total_lines as f64) * 100.0
            }
        })
    }

    /// Get coverage percentage for a function by line range.
    pub fn get_function_coverage(
        &self,
        file_path: &Path,
        start_line: usize,
        end_line: usize,
    ) -> Option<f64> {
        self.file_coverage.get(file_path).map(|fc| {
            let lines_in_range: Vec<_> = fc
                .line_hits
                .iter()
                .filter(|(&line, _)| line >= start_line && line <= end_line)
                .collect();

            if lines_in_range.is_empty() {
                return 0.0;
            }

            let hit_count = lines_in_range.iter().filter(|(_, &hits)| hits > 0).count();
            (hit_count as f64 / lines_in_range.len() as f64) * 100.0
        })
    }

    /// Add coverage data for a file.
    pub fn add_file_coverage(&mut self, path: std::path::PathBuf, coverage: FileCoverage) {
        self.file_coverage.insert(path, coverage);
    }

    /// Check if coverage data is available for a file.
    pub fn has_file(&self, path: &Path) -> bool {
        self.file_coverage.contains_key(path)
    }

    /// Get all files with coverage data.
    pub fn files(&self) -> impl Iterator<Item = &std::path::PathBuf> {
        self.file_coverage.keys()
    }
}

impl FileCoverage {
    /// Create empty file coverage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a line hit count.
    pub fn add_line(&mut self, line: usize, hits: u64) {
        self.line_hits.insert(line, hits);
        self.total_lines += 1;
        if hits > 0 {
            self.hit_lines += 1;
        }
    }

    /// Get hit count for a specific line.
    pub fn get_line_hits(&self, line: usize) -> Option<u64> {
        self.line_hits.get(&line).copied()
    }
}

/// Cache operations trait.
///
/// This trait abstracts over caching mechanisms, enabling:
/// - Testing without persistent storage
/// - Different cache backends (memory, disk, distributed)
/// - Cache invalidation strategies
///
/// # Serialization
///
/// Cache values must be serializable. The default implementation uses
/// bincode for efficient binary serialization.
pub trait Cache: Send + Sync {
    /// Get a cached value by key.
    ///
    /// Returns `None` if the key doesn't exist or the value can't be
    /// deserialized to the requested type.
    fn get(&self, key: &str) -> Option<Vec<u8>>;

    /// Set a cached value.
    ///
    /// # Errors
    ///
    /// Returns `AnalysisError::IoError` if the cache write fails.
    fn set(&self, key: &str, value: &[u8]) -> Result<(), AnalysisError>;

    /// Remove a cached value.
    ///
    /// # Errors
    ///
    /// Returns `AnalysisError::IoError` if cache invalidation fails.
    fn invalidate(&self, key: &str) -> Result<(), AnalysisError>;

    /// Clear all cached values.
    ///
    /// # Errors
    ///
    /// Returns `AnalysisError::IoError` if cache clear fails.
    fn clear(&self) -> Result<(), AnalysisError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_data_empty() {
        let data = CoverageData::new();
        assert!(data
            .get_file_coverage(Path::new("nonexistent.rs"))
            .is_none());
    }

    #[test]
    fn test_coverage_data_with_file() {
        let mut data = CoverageData::new();
        let mut file_coverage = FileCoverage::new();
        file_coverage.add_line(1, 5);
        file_coverage.add_line(2, 0);
        file_coverage.add_line(3, 3);
        data.add_file_coverage("test.rs".into(), file_coverage);

        let coverage = data.get_file_coverage(Path::new("test.rs")).unwrap();
        // 2 out of 3 lines hit = 66.67%
        assert!((coverage - 66.67).abs() < 1.0);
    }

    #[test]
    fn test_function_coverage() {
        let mut data = CoverageData::new();
        let mut file_coverage = FileCoverage::new();
        // Lines 1-5 for function A
        file_coverage.add_line(1, 5);
        file_coverage.add_line(2, 5);
        file_coverage.add_line(3, 5);
        file_coverage.add_line(4, 0);
        file_coverage.add_line(5, 0);
        // Lines 10-12 for function B
        file_coverage.add_line(10, 1);
        file_coverage.add_line(11, 1);
        file_coverage.add_line(12, 1);
        data.add_file_coverage("test.rs".into(), file_coverage);

        // Function A: 3/5 = 60%
        let func_a = data
            .get_function_coverage(Path::new("test.rs"), 1, 5)
            .unwrap();
        assert!((func_a - 60.0).abs() < 0.1);

        // Function B: 3/3 = 100%
        let func_b = data
            .get_function_coverage(Path::new("test.rs"), 10, 12)
            .unwrap();
        assert!((func_b - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_file_coverage_line_access() {
        let mut fc = FileCoverage::new();
        fc.add_line(1, 5);
        fc.add_line(2, 0);

        assert_eq!(fc.get_line_hits(1), Some(5));
        assert_eq!(fc.get_line_hits(2), Some(0));
        assert_eq!(fc.get_line_hits(3), None);
    }
}
