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

/// Parse LCOV format content into CoverageData.
///
/// This is a pure function that parses LCOV content without I/O.
fn parse_lcov_content(content: &str, source_path: &Path) -> Result<CoverageData, AnalysisError> {
    let mut data = CoverageData::new();
    let mut current_file: Option<std::path::PathBuf> = None;
    let mut current_coverage: Option<FileCoverage> = None;

    for line in content.lines() {
        let line = line.trim();

        if let Some(sf) = line.strip_prefix("SF:") {
            // Source file
            if let (Some(path), Some(coverage)) = (current_file.take(), current_coverage.take()) {
                data.add_file_coverage(path, coverage);
            }
            current_file = Some(sf.into());
            current_coverage = Some(FileCoverage::new());
        } else if let Some(da) = line.strip_prefix("DA:") {
            // Line data: DA:line_number,hit_count
            if let Some(ref mut coverage) = current_coverage {
                let parts: Vec<&str> = da.split(',').collect();
                if parts.len() >= 2 {
                    if let (Ok(line_num), Ok(hits)) =
                        (parts[0].parse::<usize>(), parts[1].parse::<u64>())
                    {
                        coverage.add_line(line_num, hits);
                    }
                }
            }
        } else if line == "end_of_record" {
            // End of file record
            if let (Some(path), Some(coverage)) = (current_file.take(), current_coverage.take()) {
                data.add_file_coverage(path, coverage);
            }
        }
    }

    // Handle last file if no end_of_record
    if let (Some(path), Some(coverage)) = (current_file, current_coverage) {
        data.add_file_coverage(path, coverage);
    }

    // Check if we actually parsed anything
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
}
