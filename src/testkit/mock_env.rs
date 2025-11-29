//! Mock environment for testing debtmap analysis operations.
//!
//! This module provides [`DebtmapTestEnv`], an in-memory implementation of
//! [`AnalysisEnv`] that enables fast, isolated tests
//! without real I/O operations.
//!
//! # Architecture
//!
//! `DebtmapTestEnv` implements the same traits as production environments:
//! - [`FileSystem`]: In-memory file storage
//! - [`CoverageLoader`]: Mock coverage data
//! - [`Cache`]: In-memory cache
//!
//! # Thread Safety
//!
//! `DebtmapTestEnv` is `Send + Sync + Clone`, making it suitable for:
//! - Parallel test execution with rayon
//! - Async tests with tokio
//! - Shared test fixtures
//!
//! The internal state uses `Arc<RwLock<_>>` for safe concurrent access.

use crate::config::DebtmapConfig;
use crate::env::AnalysisEnv;
use crate::errors::AnalysisError;
use crate::io::traits::{Cache, CoverageData, CoverageLoader, FileCoverage, FileSystem};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// In-memory test environment for debtmap analysis.
///
/// Provides a complete mock implementation of [`AnalysisEnv`] with:
/// - In-memory file system with readable/writable files
/// - Mock coverage data for testing coverage-based analysis
/// - In-memory cache for testing caching behavior
/// - Configurable settings via fluent builder API
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::testkit::DebtmapTestEnv;
/// use debtmap::env::AnalysisEnv;
///
/// let env = DebtmapTestEnv::new()
///     .with_file("src/main.rs", "fn main() { println!(\"Hello\"); }")
///     .with_file("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }")
///     .with_coverage_percentage("src/main.rs", 80.0)
///     .with_coverage_percentage("src/lib.rs", 100.0);
///
/// // Now use env with analysis functions
/// let content = env.file_system().read_to_string("src/main.rs".as_ref()).unwrap();
/// assert!(content.contains("fn main"));
/// ```
///
/// # Performance
///
/// All operations are in-memory with no I/O overhead:
/// - File reads: ~1μs (vs ~50ms with real files)
/// - Coverage lookups: ~1μs
/// - Cache operations: ~1μs
#[derive(Clone)]
pub struct DebtmapTestEnv {
    files: Arc<RwLock<HashMap<PathBuf, String>>>,
    coverage: Arc<RwLock<CoverageData>>,
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    config: DebtmapConfig,
}

impl DebtmapTestEnv {
    /// Create a new empty test environment.
    ///
    /// The environment starts with:
    /// - Empty file system
    /// - No coverage data
    /// - Empty cache
    /// - Default configuration
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            coverage: Arc::new(RwLock::new(CoverageData::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: DebtmapConfig::default(),
        }
    }

    /// Add a file to the mock file system.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let env = DebtmapTestEnv::new()
    ///     .with_file("test.rs", "fn foo() {}");
    /// ```
    pub fn with_file(self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.files
            .write()
            .expect("Lock poisoned")
            .insert(path.into(), content.into());
        self
    }

    /// Add multiple files at once.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let env = DebtmapTestEnv::new()
    ///     .with_files(vec![
    ///         ("src/main.rs", "fn main() {}"),
    ///         ("src/lib.rs", "pub fn lib() {}"),
    ///         ("tests/test.rs", "#[test] fn test() {}"),
    ///     ]);
    /// ```
    pub fn with_files<'a>(mut self, files: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        for (path, content) in files {
            self = self.with_file(path, content);
        }
        self
    }

    /// Add coverage data for a file with specific line hits.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use debtmap::io::traits::FileCoverage;
    ///
    /// let mut fc = FileCoverage::new();
    /// fc.add_line(1, 5);  // Line 1 hit 5 times
    /// fc.add_line(2, 0);  // Line 2 not hit
    /// fc.add_line(3, 3);  // Line 3 hit 3 times
    ///
    /// let env = DebtmapTestEnv::new()
    ///     .with_coverage("test.rs", fc);
    /// ```
    pub fn with_coverage(self, path: impl Into<PathBuf>, coverage: FileCoverage) -> Self {
        self.coverage
            .write()
            .expect("Lock poisoned")
            .add_file_coverage(path.into(), coverage);
        self
    }

    /// Add coverage data as a simple percentage.
    ///
    /// Creates synthetic coverage data with the specified percentage.
    /// Useful for quick test setups where exact line coverage isn't important.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let env = DebtmapTestEnv::new()
    ///     .with_coverage_percentage("src/main.rs", 75.0);  // 75% coverage
    /// ```
    pub fn with_coverage_percentage(self, path: impl Into<PathBuf>, percentage: f64) -> Self {
        let mut fc = FileCoverage::new();
        // Create 100 lines with hits matching the percentage
        let hit_lines = (percentage as usize).min(100);
        for i in 1..=100 {
            fc.add_line(i, if i <= hit_lines { 1 } else { 0 });
        }
        self.with_coverage(path, fc)
    }

    /// Set the configuration for this environment.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use debtmap::config::DebtmapConfig;
    ///
    /// let config = DebtmapConfig::default();
    /// let env = DebtmapTestEnv::new()
    ///     .with_config(config);
    /// ```
    pub fn with_config(mut self, config: DebtmapConfig) -> Self {
        self.config = config;
        self
    }

    /// Add a cache entry with raw bytes.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let env = DebtmapTestEnv::new()
    ///     .with_cache_entry("key", b"value");
    /// ```
    pub fn with_cache_entry(self, key: impl Into<String>, value: impl AsRef<[u8]>) -> Self {
        self.cache
            .write()
            .expect("Lock poisoned")
            .insert(key.into(), value.as_ref().to_vec());
        self
    }

    /// Check if a file exists in the mock file system.
    pub fn has_file(&self, path: impl AsRef<Path>) -> bool {
        self.files
            .read()
            .expect("Lock poisoned")
            .contains_key(path.as_ref())
    }

    /// Get all file paths in the mock file system.
    pub fn file_paths(&self) -> Vec<PathBuf> {
        self.files
            .read()
            .expect("Lock poisoned")
            .keys()
            .cloned()
            .collect()
    }

    /// Clear all files from the mock file system.
    pub fn clear_files(&self) {
        self.files.write().expect("Lock poisoned").clear();
    }

    /// Clear all coverage data.
    pub fn clear_coverage(&self) {
        *self.coverage.write().expect("Lock poisoned") = CoverageData::new();
    }

    /// Clear all cache entries.
    pub fn clear_cache(&self) {
        self.cache.write().expect("Lock poisoned").clear();
    }

    /// Reset the environment to empty state (files, coverage, cache).
    pub fn reset(&self) {
        self.clear_files();
        self.clear_coverage();
        self.clear_cache();
    }
}

impl Default for DebtmapTestEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for DebtmapTestEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let file_count = self.files.read().map(|f| f.len()).unwrap_or(0);
        f.debug_struct("DebtmapTestEnv")
            .field("file_count", &file_count)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

// Implement AnalysisEnv trait
impl AnalysisEnv for DebtmapTestEnv {
    fn file_system(&self) -> &dyn FileSystem {
        self
    }

    fn coverage_loader(&self) -> &dyn CoverageLoader {
        self
    }

    fn cache(&self) -> &dyn Cache {
        self
    }

    fn config(&self) -> &DebtmapConfig {
        &self.config
    }

    fn with_config(self, config: DebtmapConfig) -> Self {
        Self { config, ..self }
    }
}

// Implement FileSystem trait
impl FileSystem for DebtmapTestEnv {
    fn read_to_string(&self, path: &Path) -> Result<String, AnalysisError> {
        self.files
            .read()
            .expect("Lock poisoned")
            .get(path)
            .cloned()
            .ok_or_else(|| AnalysisError::io(format!("File not found: {}", path.display())))
    }

    fn write(&self, path: &Path, content: &str) -> Result<(), AnalysisError> {
        self.files
            .write()
            .expect("Lock poisoned")
            .insert(path.to_path_buf(), content.to_string());
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.read().expect("Lock poisoned").contains_key(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.exists(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        // Check if any file has this path as a prefix
        let files = self.files.read().expect("Lock poisoned");
        files.keys().any(|file_path| {
            file_path
                .parent()
                .map(|p| p.starts_with(path))
                .unwrap_or(false)
        })
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, AnalysisError> {
        self.read_to_string(path).map(|s| s.into_bytes())
    }
}

// Implement CoverageLoader trait
impl CoverageLoader for DebtmapTestEnv {
    fn load_lcov(&self, _path: &Path) -> Result<CoverageData, AnalysisError> {
        // Return the stored coverage data (ignoring path since it's mock)
        Ok(self.coverage.read().expect("Lock poisoned").clone())
    }

    fn load_cobertura(&self, path: &Path) -> Result<CoverageData, AnalysisError> {
        // Use same mock data for cobertura
        self.load_lcov(path)
    }
}

// Implement Cache trait
impl Cache for DebtmapTestEnv {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.cache.read().expect("Lock poisoned").get(key).cloned()
    }

    fn set(&self, key: &str, value: &[u8]) -> Result<(), AnalysisError> {
        self.cache
            .write()
            .expect("Lock poisoned")
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn invalidate(&self, key: &str) -> Result<(), AnalysisError> {
        self.cache.write().expect("Lock poisoned").remove(key);
        Ok(())
    }

    fn clear(&self) -> Result<(), AnalysisError> {
        self.cache.write().expect("Lock poisoned").clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_env_is_empty() {
        let env = DebtmapTestEnv::new();
        assert!(!env.has_file("any.rs"));
        assert!(env.file_paths().is_empty());
    }

    #[test]
    fn test_with_file() {
        let env = DebtmapTestEnv::new().with_file("test.rs", "fn main() {}");

        assert!(env.has_file("test.rs"));
        let content = env
            .file_system()
            .read_to_string(Path::new("test.rs"))
            .unwrap();
        assert_eq!(content, "fn main() {}");
    }

    #[test]
    fn test_with_files() {
        let env = DebtmapTestEnv::new().with_files(vec![
            ("a.rs", "fn a() {}"),
            ("b.rs", "fn b() {}"),
            ("c.rs", "fn c() {}"),
        ]);

        assert_eq!(env.file_paths().len(), 3);
        assert!(env.has_file("a.rs"));
        assert!(env.has_file("b.rs"));
        assert!(env.has_file("c.rs"));
    }

    #[test]
    fn test_file_not_found() {
        let env = DebtmapTestEnv::new();
        let result = env.file_system().read_to_string(Path::new("missing.rs"));
        assert!(result.is_err());
    }

    #[test]
    fn test_write_and_read() {
        let env = DebtmapTestEnv::new();
        env.file_system()
            .write(Path::new("new.rs"), "fn new() {}")
            .unwrap();

        let content = env
            .file_system()
            .read_to_string(Path::new("new.rs"))
            .unwrap();
        assert_eq!(content, "fn new() {}");
    }

    #[test]
    fn test_coverage_percentage() {
        let env = DebtmapTestEnv::new().with_coverage_percentage("test.rs", 75.0);

        let coverage = env.coverage_loader().load_lcov(Path::new("")).unwrap();
        let pct = coverage.get_file_coverage(Path::new("test.rs")).unwrap();
        assert!((pct - 75.0).abs() < 1.0);
    }

    #[test]
    fn test_cache_operations() {
        let env = DebtmapTestEnv::new();

        // Set and get
        env.cache().set("key", b"value").unwrap();
        assert_eq!(env.cache().get("key"), Some(b"value".to_vec()));

        // Invalidate
        env.cache().invalidate("key").unwrap();
        assert!(env.cache().get("key").is_none());

        // Set again and clear
        env.cache().set("key1", b"v1").unwrap();
        env.cache().set("key2", b"v2").unwrap();
        env.cache().clear().unwrap();
        assert!(env.cache().get("key1").is_none());
        assert!(env.cache().get("key2").is_none());
    }

    #[test]
    fn test_with_config() {
        use crate::config::IgnoreConfig;

        let config = DebtmapConfig {
            ignore: Some(IgnoreConfig {
                patterns: vec!["test".to_string()],
            }),
            ..Default::default()
        };

        let env = DebtmapTestEnv::new().with_config(config);
        assert!(env.config().ignore.is_some());
    }

    #[test]
    fn test_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DebtmapTestEnv>();
    }

    #[test]
    fn test_is_clone() {
        let env1 = DebtmapTestEnv::new().with_file("test.rs", "fn main() {}");
        let env2 = env1.clone();

        // Both should have the file (shared state via Arc)
        assert!(env1.has_file("test.rs"));
        assert!(env2.has_file("test.rs"));
    }

    #[test]
    fn test_reset() {
        let env = DebtmapTestEnv::new()
            .with_file("test.rs", "fn main() {}")
            .with_coverage_percentage("test.rs", 50.0)
            .with_cache_entry("key", b"value");

        env.reset();

        assert!(!env.has_file("test.rs"));
        assert!(env.cache().get("key").is_none());
    }

    #[test]
    fn test_analysis_env_trait() {
        let env = DebtmapTestEnv::new().with_file("test.rs", "fn main() {}");

        // Test AnalysisEnv methods work through the trait
        let _fs = env.file_system();
        let _cl = env.coverage_loader();
        let _cache = env.cache();
        let _config = env.config();

        // Test with_config
        let env2 = env.with_config(DebtmapConfig::default());
        assert!(env2.has_file("test.rs"));
    }

    #[test]
    fn test_is_dir() {
        let env = DebtmapTestEnv::new()
            .with_file("src/main.rs", "fn main() {}")
            .with_file("src/lib.rs", "pub fn lib() {}");

        assert!(env.file_system().is_dir(Path::new("src")));
        assert!(!env.file_system().is_dir(Path::new("other")));
    }

    #[test]
    fn test_read_bytes() {
        let env = DebtmapTestEnv::new().with_file("test.rs", "fn main() {}");

        let bytes = env.file_system().read_bytes(Path::new("test.rs")).unwrap();
        assert_eq!(bytes, b"fn main() {}");
    }
}
