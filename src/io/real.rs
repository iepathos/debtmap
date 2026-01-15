//! Production implementations of I/O traits.
//!
//! This module provides real implementations of the I/O traits defined in
//! [`crate::io::traits`] that perform actual file system and cache operations.
//!
//! # Usage
//!
//! These implementations are used in production code through the `RealEnv`
//! environment. For testing, use mock implementations instead.
//!
//! ```rust,ignore
//! use debtmap::io::real::RealFileSystem;
//! use debtmap::io::traits::FileSystem;
//!
//! let fs = RealFileSystem::new();
//! let content = fs.read_to_string(Path::new("src/main.rs"))?;
//! ```

use crate::errors::AnalysisError;
use crate::io::traits::{Cache, CoverageData, CoverageLoader, FileCoverage, FileSystem};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::RwLock;

/// Production file system implementation.
///
/// This implementation directly delegates to `std::fs` operations.
/// It is thread-safe and can be shared across analysis threads.
#[derive(Debug, Default, Clone)]
pub struct RealFileSystem;

impl RealFileSystem {
    /// Create a new real file system instance.
    pub fn new() -> Self {
        Self
    }
}

impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> Result<String, AnalysisError> {
        fs::read_to_string(path)
            .map_err(|e| AnalysisError::io_with_path(format!("Failed to read file: {}", e), path))
    }

    fn write(&self, path: &Path, content: &str) -> Result<(), AnalysisError> {
        fs::write(path, content)
            .map_err(|e| AnalysisError::io_with_path(format!("Failed to write file: {}", e), path))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, AnalysisError> {
        fs::read(path)
            .map_err(|e| AnalysisError::io_with_path(format!("Failed to read file: {}", e), path))
    }
}

/// Production coverage loader implementation.
///
/// This implementation parses actual coverage files in LCOV and
/// Cobertura formats.
#[derive(Debug, Default, Clone)]
pub struct RealCoverageLoader;

impl RealCoverageLoader {
    /// Create a new real coverage loader instance.
    pub fn new() -> Self {
        Self
    }
}

impl CoverageLoader for RealCoverageLoader {
    fn load_lcov(&self, path: &Path) -> Result<CoverageData, AnalysisError> {
        let content = fs::read_to_string(path).map_err(|e| {
            AnalysisError::coverage_with_path(format!("Failed to read LCOV file: {}", e), path)
        })?;

        parse_lcov_content(&content, path)
    }

    fn load_cobertura(&self, path: &Path) -> Result<CoverageData, AnalysisError> {
        // Cobertura XML parsing is not implemented in this foundation spec
        // It will be added in a future spec as needed
        Err(AnalysisError::coverage_with_path(
            "Cobertura format not yet implemented",
            path,
        ))
    }
}

/// Represents a parsed LCOV line.
#[derive(Debug, PartialEq)]
enum LcovLine {
    /// Source file declaration (SF:path)
    SourceFile(std::path::PathBuf),
    /// Line data (DA:line_number,hit_count)
    LineData { line: usize, hits: u64 },
    /// End of record marker
    EndOfRecord,
    /// Unknown or empty line (ignored)
    Unknown,
}

/// Parse a single LCOV line into its structured representation.
///
/// This is a pure function with no side effects.
fn parse_lcov_line(line: &str) -> LcovLine {
    let line = line.trim();

    if let Some(sf) = line.strip_prefix("SF:") {
        LcovLine::SourceFile(sf.into())
    } else if let Some(da) = line.strip_prefix("DA:") {
        parse_line_data(da)
    } else if line == "end_of_record" {
        LcovLine::EndOfRecord
    } else {
        LcovLine::Unknown
    }
}

/// Parse DA (line data) format: "line_number,hit_count".
fn parse_line_data(da: &str) -> LcovLine {
    let mut parts = da.split(',');

    let parsed = parts
        .next()
        .and_then(|line_str| line_str.parse::<usize>().ok())
        .zip(
            parts
                .next()
                .and_then(|hits_str| hits_str.parse::<u64>().ok()),
        );

    match parsed {
        Some((line, hits)) => LcovLine::LineData { line, hits },
        None => LcovLine::Unknown,
    }
}

/// Parser state for LCOV content processing.
struct LcovParserState {
    data: CoverageData,
    current_file: Option<std::path::PathBuf>,
    current_coverage: Option<FileCoverage>,
}

impl LcovParserState {
    fn new() -> Self {
        Self {
            data: CoverageData::new(),
            current_file: None,
            current_coverage: None,
        }
    }

    /// Process a single parsed line, returning updated state.
    fn process(mut self, line: LcovLine) -> Self {
        match line {
            LcovLine::SourceFile(path) => {
                self.finalize_current();
                self.current_file = Some(path);
                self.current_coverage = Some(FileCoverage::new());
            }
            LcovLine::LineData { line, hits } => {
                if let Some(ref mut coverage) = self.current_coverage {
                    coverage.add_line(line, hits);
                }
            }
            LcovLine::EndOfRecord => {
                self.finalize_current();
            }
            LcovLine::Unknown => {}
        }
        self
    }

    /// Finalize the current file record if one exists.
    fn finalize_current(&mut self) {
        if let (Some(path), Some(coverage)) =
            (self.current_file.take(), self.current_coverage.take())
        {
            self.data.add_file_coverage(path, coverage);
        }
    }

    /// Complete parsing and return the final coverage data.
    fn finish(mut self) -> CoverageData {
        self.finalize_current();
        self.data
    }
}

/// Parse LCOV format content into CoverageData.
///
/// This function uses a functional pipeline to parse LCOV content:
/// 1. Parse each line into a structured representation
/// 2. Fold over lines to accumulate coverage data
fn parse_lcov_content(content: &str, source_path: &Path) -> Result<CoverageData, AnalysisError> {
    let data = content
        .lines()
        .map(parse_lcov_line)
        .fold(LcovParserState::new(), LcovParserState::process)
        .finish();

    validate_coverage_data(data, content, source_path)
}

/// Validate that parsed coverage data is not unexpectedly empty.
fn validate_coverage_data(
    data: CoverageData,
    content: &str,
    source_path: &Path,
) -> Result<CoverageData, AnalysisError> {
    if data.files().next().is_none() && !content.is_empty() {
        return Err(AnalysisError::coverage_with_path(
            "No coverage data found in LCOV file",
            source_path,
        ));
    }
    Ok(data)
}

/// In-memory cache implementation.
///
/// This is a simple thread-safe in-memory cache for development and testing.
/// Production deployments may want to use a disk-based or distributed cache.
#[derive(Debug, Default)]
pub struct MemoryCache {
    data: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemoryCache {
    /// Create a new empty memory cache.
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl Cache for MemoryCache {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.data.read().ok()?.get(key).cloned()
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), AnalysisError> {
        self.data
            .write()
            .map_err(|e| AnalysisError::io(format!("Cache write lock failed: {}", e)))?
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn invalidate(&self, key: &str) -> Result<(), AnalysisError> {
        self.data
            .write()
            .map_err(|e| AnalysisError::io(format!("Cache write lock failed: {}", e)))?
            .remove(key);
        Ok(())
    }

    fn clear(&self) -> Result<(), AnalysisError> {
        self.data
            .write()
            .map_err(|e| AnalysisError::io(format!("Cache write lock failed: {}", e)))?
            .clear();
        Ok(())
    }
}

/// No-op cache implementation for when caching is disabled.
#[derive(Debug, Default, Clone)]
pub struct NoOpCache;

impl NoOpCache {
    /// Create a new no-op cache.
    pub fn new() -> Self {
        Self
    }
}

impl Cache for NoOpCache {
    fn get(&self, _key: &str) -> Option<Vec<u8>> {
        None
    }

    fn set(&self, _key: &str, _value: &[u8]) -> Result<(), AnalysisError> {
        Ok(())
    }

    fn invalidate(&self, _key: &str) -> Result<(), AnalysisError> {
        Ok(())
    }

    fn clear(&self) -> Result<(), AnalysisError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_real_filesystem_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let fs = RealFileSystem::new();

        // File doesn't exist yet
        assert!(!fs.exists(&file_path));
        assert!(!fs.is_file(&file_path));

        // Write and read
        fs.write(&file_path, "Hello, World!").unwrap();
        assert!(fs.exists(&file_path));
        assert!(fs.is_file(&file_path));

        let content = fs.read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_real_filesystem_read_nonexistent() {
        let fs = RealFileSystem::new();
        let result = fs.read_to_string(Path::new("/nonexistent/path/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_real_filesystem_is_dir() {
        let temp_dir = TempDir::new().unwrap();
        let fs = RealFileSystem::new();

        assert!(fs.is_dir(temp_dir.path()));
        assert!(!fs.is_file(temp_dir.path()));
    }

    #[test]
    fn test_parse_lcov_content() {
        let lcov_content = r#"
SF:src/main.rs
DA:1,5
DA:2,5
DA:3,0
end_of_record
SF:src/lib.rs
DA:1,1
DA:2,0
end_of_record
"#;
        let data = parse_lcov_content(lcov_content, Path::new("test.lcov")).unwrap();

        // Check src/main.rs: 2/3 lines hit
        let main_coverage = data.get_file_coverage(Path::new("src/main.rs")).unwrap();
        assert!((main_coverage - 66.67).abs() < 1.0);

        // Check src/lib.rs: 1/2 lines hit
        let lib_coverage = data.get_file_coverage(Path::new("src/lib.rs")).unwrap();
        assert!((lib_coverage - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_lcov_empty() {
        let data = parse_lcov_content("", Path::new("test.lcov")).unwrap();
        assert!(data.files().next().is_none());
    }

    #[test]
    fn test_memory_cache_operations() {
        let cache = MemoryCache::new();

        // Initially empty
        assert!(cache.get("key1").is_none());

        // Set and get
        cache.set("key1", b"value1").unwrap();
        assert_eq!(cache.get("key1"), Some(b"value1".to_vec()));

        // Invalidate
        cache.invalidate("key1").unwrap();
        assert!(cache.get("key1").is_none());

        // Clear
        cache.set("key1", b"value1").unwrap();
        cache.set("key2", b"value2").unwrap();
        cache.clear().unwrap();
        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_none());
    }

    #[test]
    fn test_noop_cache() {
        let cache = NoOpCache::new();

        // Set should succeed but get should return None
        cache.set("key1", b"value1").unwrap();
        assert!(cache.get("key1").is_none());

        // Operations should not fail
        cache.invalidate("key1").unwrap();
        cache.clear().unwrap();
    }

    #[test]
    fn test_parse_lcov_line_source_file() {
        let line = parse_lcov_line("SF:src/main.rs");
        assert_eq!(line, LcovLine::SourceFile("src/main.rs".into()));
    }

    #[test]
    fn test_parse_lcov_line_line_data() {
        let line = parse_lcov_line("DA:42,5");
        assert_eq!(line, LcovLine::LineData { line: 42, hits: 5 });
    }

    #[test]
    fn test_parse_lcov_line_end_of_record() {
        let line = parse_lcov_line("end_of_record");
        assert_eq!(line, LcovLine::EndOfRecord);
    }

    #[test]
    fn test_parse_lcov_line_unknown() {
        assert_eq!(parse_lcov_line(""), LcovLine::Unknown);
        assert_eq!(parse_lcov_line("  "), LcovLine::Unknown);
        assert_eq!(parse_lcov_line("# comment"), LcovLine::Unknown);
        assert_eq!(parse_lcov_line("TN:test"), LcovLine::Unknown);
    }

    #[test]
    fn test_parse_lcov_line_whitespace_handling() {
        let line = parse_lcov_line("  SF:src/lib.rs  ");
        assert_eq!(line, LcovLine::SourceFile("src/lib.rs".into()));
    }

    #[test]
    fn test_parse_line_data_invalid() {
        // Missing hit count
        assert_eq!(parse_line_data("42"), LcovLine::Unknown);
        // Non-numeric line
        assert_eq!(parse_line_data("abc,5"), LcovLine::Unknown);
        // Non-numeric hits
        assert_eq!(parse_line_data("42,abc"), LcovLine::Unknown);
        // Empty string
        assert_eq!(parse_line_data(""), LcovLine::Unknown);
    }

    #[test]
    fn test_parse_lcov_content_without_end_of_record() {
        // Some LCOV generators don't include end_of_record for the last file
        let lcov_content = "SF:src/main.rs\nDA:1,5\nDA:2,3";
        let data = parse_lcov_content(lcov_content, Path::new("test.lcov")).unwrap();
        let coverage = data.get_file_coverage(Path::new("src/main.rs")).unwrap();
        assert!((coverage - 100.0).abs() < 0.1); // Both lines hit
    }

    #[test]
    fn test_parse_lcov_content_invalid_format() {
        // Non-empty content but no valid LCOV data
        let result = parse_lcov_content("garbage\nmore garbage", Path::new("test.lcov"));
        assert!(result.is_err());
    }
}
