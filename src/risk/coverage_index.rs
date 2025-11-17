use super::lcov::{normalize_demangled_name, FunctionCoverage, LcovData};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Normalize a path by removing leading ./
pub fn normalize_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);
    PathBuf::from(cleaned)
}

/// Pre-indexed coverage data for O(1) function lookups
///
/// # Data Structure
///
/// Uses nested HashMap for efficient lookups:
/// - Outer map: file path → functions in that file
/// - Inner map: function name → coverage data
///
/// # Performance Characteristics
///
/// - **Build Time**: O(n) where n = coverage records
/// - **Exact Match Lookup**: O(1) - file hash + function hash
/// - **Path Strategy Lookup**: O(m) where m = number of files
/// - **Memory**: ~200 bytes per function + ~100 bytes per file
///
/// # Lookup Strategies
///
/// 1. **Exact match**: O(1) hash lookups
/// 2. **Suffix matching**: O(files) iteration + O(1) lookup
/// 3. **Normalized equality**: O(files) iteration + O(1) lookup
///
/// The nested structure ensures we only iterate over files (typically ~375)
/// not functions (typically ~1,500), providing 4x-50x speedup for path matching.
///
/// # Usage
///
/// Build the index once from parsed LCOV data, then share it across threads
/// using `Arc` for concurrent access. The index provides O(1) lookups by
/// file path and function name, making it efficient for parallel analysis
/// of large codebases.
#[derive(Debug, Clone)]
pub struct CoverageIndex {
    /// Nested structure: file → (function_name → coverage)
    /// Enables O(1) file lookup followed by O(1) function lookup
    by_file: HashMap<PathBuf, HashMap<String, FunctionCoverage>>,

    /// Coverage records indexed by file path + line number for range queries
    /// BTreeMap allows efficient range queries for finding functions by line
    by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>>,

    /// Pre-computed set of all file paths for faster iteration in fallback strategies
    file_paths: Vec<PathBuf>,

    /// Statistics for debugging and monitoring
    stats: CoverageIndexStats,
}

/// Statistics about coverage index for observability
#[derive(Debug, Clone)]
pub struct CoverageIndexStats {
    pub total_files: usize,
    pub total_records: usize,
    pub index_build_time: Duration,
    pub estimated_memory_bytes: usize,
}

impl CoverageIndex {
    /// Create an empty coverage index
    pub fn empty() -> Self {
        CoverageIndex {
            by_file: HashMap::new(),
            by_line: HashMap::new(),
            file_paths: Vec::new(),
            stats: CoverageIndexStats {
                total_files: 0,
                total_records: 0,
                index_build_time: Duration::from_secs(0),
                estimated_memory_bytes: 0,
            },
        }
    }

    /// Build coverage index from LCOV data (O(n) operation)
    ///
    /// This creates two indexes:
    /// 1. Nested HashMap for O(1) file + function lookups
    /// 2. BTreeMap for line-based range queries
    pub fn from_coverage(coverage: &LcovData) -> Self {
        let start = Instant::now();

        let mut by_file: HashMap<PathBuf, HashMap<String, FunctionCoverage>> = HashMap::new();
        let mut by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>> = HashMap::new();
        let mut total_records = 0;

        for (file_path, functions) in &coverage.functions {
            // Build inner HashMap for this file's functions
            let mut file_functions = HashMap::new();
            let mut line_map = BTreeMap::new();

            for func in functions {
                total_records += 1;

                // Insert into nested structure
                file_functions.insert(func.name.clone(), func.clone());

                // Index by start_line for range queries
                line_map.insert(func.start_line, func.clone());
            }

            if !file_functions.is_empty() {
                by_file.insert(file_path.clone(), file_functions);
            }

            if !line_map.is_empty() {
                by_line.insert(file_path.clone(), line_map);
            }
        }

        // Pre-compute file paths for faster iteration
        let file_paths: Vec<PathBuf> = by_file.keys().cloned().collect();

        let index_build_time = start.elapsed();
        let total_files = by_file.len();

        // Estimate memory usage: ~200 bytes per record + ~100 bytes per file
        let estimated_memory_bytes = total_records * 200 + file_paths.len() * 100;

        CoverageIndex {
            by_file,
            by_line,
            file_paths,
            stats: CoverageIndexStats {
                total_files,
                total_records,
                index_build_time,
                estimated_memory_bytes,
            },
        }
    }

    /// Get function coverage by exact name (O(1) lookup)
    ///
    /// This is the primary lookup method and should be used when the exact
    /// function name is known. Also tries path normalization strategies.
    pub fn get_function_coverage(&self, file: &Path, function_name: &str) -> Option<f64> {
        log::debug!(
            "Looking up coverage for function '{}' in file '{}'",
            function_name,
            file.display()
        );

        // Normalize the query function name (remove angle brackets, etc.)
        let normalized_query = normalize_demangled_name(function_name);
        let query_name = &normalized_query.full_path;

        // O(1) exact match: file lookup + function lookup
        if let Some(file_functions) = self.by_file.get(file) {
            // Try exact match with normalized query first
            if let Some(f) = file_functions.get(query_name) {
                log::debug!(
                    "✓ Found via exact match (normalized): {}% coverage",
                    f.coverage_percentage
                );
                return Some(f.coverage_percentage / 100.0);
            }
            // Try original query (in case it was already normalized)
            if query_name != function_name {
                if let Some(f) = file_functions.get(function_name) {
                    log::debug!(
                        "✓ Found via exact match (original): {}% coverage",
                        f.coverage_percentage
                    );
                    return Some(f.coverage_percentage / 100.0);
                }
            }
        }

        log::debug!("Exact match failed, trying path strategies...");

        // O(files) fallback strategies - much faster than O(functions)
        // Try with normalized query first, then original
        if let Some(result) = self.find_by_path_strategies(file, query_name) {
            return Some(result.coverage_percentage / 100.0);
        }
        if query_name != function_name {
            self.find_by_path_strategies(file, function_name)
                .map(|f| f.coverage_percentage / 100.0)
        } else {
            None
        }
    }

    /// Try multiple path matching strategies to handle relative/absolute path mismatches
    ///
    /// This method iterates over FILES (not functions), providing O(files) complexity
    /// instead of O(functions). For 375 files with ~4 functions each, this is
    /// 375 iterations vs 1,500, a 4x speedup.
    ///
    /// Matching strategies in order:
    /// 1. Exact name match (query path matches file path)
    /// 2. Method name match (for Rust methods, match just the final segment)
    /// 3. Suffix/path matching strategies
    fn find_by_path_strategies(
        &self,
        query_path: &Path,
        function_name: &str,
    ) -> Option<&FunctionCoverage> {
        let normalized_query = normalize_path(query_path);

        log::debug!("Strategy 1: Suffix matching (query.ends_with(lcov_file))");
        // Strategy 1: Suffix matching - iterate over FILES not functions
        for file_path in &self.file_paths {
            if query_path.ends_with(file_path) {
                log::debug!("  Found path match: '{}'", file_path.display());
                // O(1) lookup once we find the file
                if let Some(file_functions) = self.by_file.get(file_path) {
                    // Try exact match first
                    if let Some(coverage) = file_functions.get(function_name) {
                        log::debug!(
                            "  ✓ Matched function name exactly: {}%",
                            coverage.coverage_percentage
                        );
                        return Some(coverage);
                    }
                    // Try function name suffix match (for querying with short form)
                    // Allows querying "ResumeExecutor::method" to match stored "prodigy::cook::resume::ResumeExecutor::method"
                    for func in file_functions.values() {
                        if func.name.ends_with(function_name) || function_name.ends_with(&func.name)
                        {
                            log::debug!(
                                "  ✓ Matched function name via suffix: query '{}' matches stored '{}': {}%",
                                function_name,
                                func.name,
                                func.coverage_percentage
                            );
                            return Some(func);
                        }
                    }
                    // Try method name match (for Rust methods)
                    for func in file_functions.values() {
                        if func.normalized.method_name == function_name {
                            log::debug!(
                                "  ✓ Matched method name '{}' -> '{}': {}%",
                                func.name,
                                func.normalized.method_name,
                                func.coverage_percentage
                            );
                            return Some(func);
                        }
                    }
                    log::debug!("  ✗ No function match in this file");
                }
            }
        }

        log::debug!("Strategy 2: Reverse suffix matching (lcov_file.ends_with(query))");
        // Strategy 2: Reverse suffix matching - iterate over FILES
        for file_path in &self.file_paths {
            if file_path.ends_with(&normalized_query) {
                log::debug!("  Found path match: '{}'", file_path.display());
                if let Some(file_functions) = self.by_file.get(file_path) {
                    // Try exact match first
                    if let Some(coverage) = file_functions.get(function_name) {
                        log::debug!(
                            "  ✓ Matched function name exactly: {}%",
                            coverage.coverage_percentage
                        );
                        return Some(coverage);
                    }
                    // Try function name suffix match (for querying with short form)
                    for func in file_functions.values() {
                        if func.name.ends_with(function_name) || function_name.ends_with(&func.name)
                        {
                            log::debug!(
                                "  ✓ Matched function name via suffix: query '{}' matches stored '{}': {}%",
                                function_name,
                                func.name,
                                func.coverage_percentage
                            );
                            return Some(func);
                        }
                    }
                    // Try method name match (for Rust methods)
                    for func in file_functions.values() {
                        if func.normalized.method_name == function_name {
                            log::debug!(
                                "  ✓ Matched method name '{}' -> '{}': {}%",
                                func.name,
                                func.normalized.method_name,
                                func.coverage_percentage
                            );
                            return Some(func);
                        }
                    }
                    log::debug!("  ✗ No function match in this file");
                }
            }
        }

        log::debug!("Strategy 3: Normalized path equality");
        // Strategy 3: Normalized equality - iterate over FILES
        for file_path in &self.file_paths {
            if normalize_path(file_path) == normalized_query {
                log::debug!("  Found path match: '{}'", file_path.display());
                if let Some(file_functions) = self.by_file.get(file_path) {
                    // Try exact match first
                    if let Some(coverage) = file_functions.get(function_name) {
                        log::debug!(
                            "  ✓ Matched function name exactly: {}%",
                            coverage.coverage_percentage
                        );
                        return Some(coverage);
                    }
                    // Try function name suffix match (for querying with short form)
                    for func in file_functions.values() {
                        if func.name.ends_with(function_name) || function_name.ends_with(&func.name)
                        {
                            log::debug!(
                                "  ✓ Matched function name via suffix: query '{}' matches stored '{}': {}%",
                                function_name,
                                func.name,
                                func.coverage_percentage
                            );
                            return Some(func);
                        }
                    }
                    // Try method name match (for Rust methods)
                    for func in file_functions.values() {
                        if func.normalized.method_name == function_name {
                            log::debug!(
                                "  ✓ Matched method name '{}' -> '{}': {}%",
                                func.name,
                                func.normalized.method_name,
                                func.coverage_percentage
                            );
                            return Some(func);
                        }
                    }
                    log::debug!("  ✗ No function match in this file");
                }
            }
        }

        log::debug!("✗ All path strategies failed");
        None
    }

    /// Get function coverage using line number with tolerance (O(log n) lookup)
    ///
    /// Falls back to line-based lookup when exact name match fails.
    /// Uses BTreeMap range query for efficient lookups.
    pub fn get_function_coverage_with_line(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<f64> {
        // Try exact name match with O(1) nested lookup first
        if let Some(file_functions) = self.by_file.get(file) {
            if let Some(f) = file_functions.get(function_name) {
                return Some(f.coverage_percentage / 100.0);
            }
        }

        // Try line-based lookup (O(log n)) - faster than path matching strategies
        if let Some(coverage) = self
            .find_function_by_line(file, line, 2)
            .map(|f| f.coverage_percentage / 100.0)
        {
            return Some(coverage);
        }

        // Only fall back to path matching strategies if line lookup fails
        self.find_by_path_strategies(file, function_name)
            .map(|f| f.coverage_percentage / 100.0)
    }

    /// Get uncovered lines for a function
    pub fn get_function_uncovered_lines(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<Vec<usize>> {
        // O(1) file lookup + O(1) function lookup
        if let Some(file_functions) = self.by_file.get(file) {
            if let Some(func) = file_functions.get(function_name) {
                return Some(func.uncovered_lines.clone());
            }
        }

        // Try path matching strategies
        if let Some(func) = self.find_by_path_strategies(file, function_name) {
            return Some(func.uncovered_lines.clone());
        }

        // Fallback to line-based lookup
        self.find_function_by_line(file, line, 2)
            .map(|f| f.uncovered_lines.clone())
    }

    /// Find function by line number with tolerance (private helper)
    ///
    /// Searches for a function whose start_line is within `tolerance` of the target line.
    /// Returns the closest matching function.
    fn find_function_by_line(
        &self,
        file: &Path,
        target_line: usize,
        tolerance: usize,
    ) -> Option<&FunctionCoverage> {
        let line_map = self.by_line.get(file)?;

        // Define search range with tolerance
        let min_line = target_line.saturating_sub(tolerance);
        let max_line = target_line.saturating_add(tolerance);

        // Use BTreeMap range query to find functions in range
        line_map
            .range(min_line..=max_line)
            .min_by_key(|(line, _)| line.abs_diff(target_line))
            .map(|(_, func)| func)
    }

    /// Get index statistics
    pub fn stats(&self) -> &CoverageIndexStats {
        &self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::lcov::NormalizedFunctionName;

    fn create_test_function_coverage(
        name: &str,
        start_line: usize,
        execution_count: u64,
        coverage_percentage: f64,
        uncovered_lines: Vec<usize>,
    ) -> FunctionCoverage {
        FunctionCoverage {
            name: name.to_string(),
            start_line,
            execution_count,
            coverage_percentage,
            uncovered_lines,
            normalized: NormalizedFunctionName {
                full_path: name.to_string(),
                method_name: name.to_string(),
                original: name.to_string(),
            },
        }
    }

    fn create_test_coverage() -> LcovData {
        let mut coverage = LcovData::default();

        let test_functions = vec![
            create_test_function_coverage("func_a", 10, 5, 100.0, vec![]),
            create_test_function_coverage("func_b", 20, 3, 75.0, vec![22, 24]),
            create_test_function_coverage("func_c", 30, 0, 0.0, vec![30, 31, 32, 33]),
        ];

        coverage
            .functions
            .insert(PathBuf::from("test.rs"), test_functions);
        coverage
    }

    #[test]
    fn test_index_build() {
        let coverage = create_test_coverage();
        let index = CoverageIndex::from_coverage(&coverage);

        assert_eq!(index.stats.total_files, 1);
        assert_eq!(index.stats.total_records, 3);
        assert!(index.stats.index_build_time < Duration::from_millis(10));
    }

    #[test]
    fn test_exact_function_lookup() {
        let coverage = create_test_coverage();
        let index = CoverageIndex::from_coverage(&coverage);

        // Test exact match
        assert_eq!(
            index.get_function_coverage(Path::new("test.rs"), "func_a"),
            Some(1.0) // 100% as fraction
        );
        assert_eq!(
            index.get_function_coverage(Path::new("test.rs"), "func_b"),
            Some(0.75) // 75% as fraction
        );
        assert_eq!(
            index.get_function_coverage(Path::new("test.rs"), "func_c"),
            Some(0.0)
        );
    }

    #[test]
    fn test_function_not_found() {
        let coverage = create_test_coverage();
        let index = CoverageIndex::from_coverage(&coverage);

        assert_eq!(
            index.get_function_coverage(Path::new("test.rs"), "nonexistent"),
            None
        );
        assert_eq!(
            index.get_function_coverage(Path::new("other.rs"), "func_a"),
            None
        );
    }

    #[test]
    fn test_line_based_lookup() {
        let coverage = create_test_coverage();
        let index = CoverageIndex::from_coverage(&coverage);

        // Test exact line match
        assert_eq!(
            index.get_function_coverage_with_line(Path::new("test.rs"), "unknown", 10),
            Some(1.0)
        );

        // Test within tolerance
        assert_eq!(
            index.get_function_coverage_with_line(Path::new("test.rs"), "unknown", 11),
            Some(1.0) // Should find func_a at line 10
        );

        // Test line 21 should match func_b at line 20
        assert_eq!(
            index.get_function_coverage_with_line(Path::new("test.rs"), "unknown", 21),
            Some(0.75)
        );
    }

    #[test]
    fn test_uncovered_lines() {
        let coverage = create_test_coverage();
        let index = CoverageIndex::from_coverage(&coverage);

        assert_eq!(
            index.get_function_uncovered_lines(Path::new("test.rs"), "func_a", 10),
            Some(vec![])
        );
        assert_eq!(
            index.get_function_uncovered_lines(Path::new("test.rs"), "func_b", 20),
            Some(vec![22, 24])
        );
        assert_eq!(
            index.get_function_uncovered_lines(Path::new("test.rs"), "func_c", 30),
            Some(vec![30, 31, 32, 33])
        );
    }

    #[test]
    fn test_empty_coverage() {
        let coverage = LcovData::default();
        let index = CoverageIndex::from_coverage(&coverage);

        assert_eq!(index.stats.total_files, 0);
        assert_eq!(index.stats.total_records, 0);
        assert_eq!(
            index.get_function_coverage(Path::new("test.rs"), "func"),
            None
        );
    }

    #[test]
    fn test_multiple_files() {
        let mut coverage = LcovData::default();

        coverage.functions.insert(
            PathBuf::from("file1.rs"),
            vec![create_test_function_coverage("func1", 5, 10, 100.0, vec![])],
        );

        coverage.functions.insert(
            PathBuf::from("file2.rs"),
            vec![create_test_function_coverage(
                "func2",
                15,
                0,
                0.0,
                vec![15, 16],
            )],
        );

        let index = CoverageIndex::from_coverage(&coverage);

        assert_eq!(index.stats.total_files, 2);
        assert_eq!(index.stats.total_records, 2);

        assert_eq!(
            index.get_function_coverage(Path::new("file1.rs"), "func1"),
            Some(1.0)
        );
        assert_eq!(
            index.get_function_coverage(Path::new("file2.rs"), "func2"),
            Some(0.0)
        );
    }
}
