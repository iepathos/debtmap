use super::lcov::{FunctionCoverage, LcovData};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Normalize a path by removing leading ./
fn normalize_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);
    PathBuf::from(cleaned)
}

/// Pre-indexed coverage data for O(1) function lookups
///
/// # Performance Characteristics
///
/// - **Build Time**: O(n) where n = coverage records
/// - **Lookup Time**: O(1) for exact name match, O(log m) for line-based lookup
/// - **Memory**: ~200 bytes per coverage record
///
/// # Usage
///
/// Build the index once from parsed LCOV data, then share it across threads
/// using `Arc` for concurrent access. The index provides O(1) lookups by
/// file path and function name, making it efficient for parallel analysis
/// of large codebases.
#[derive(Debug, Clone)]
pub struct CoverageIndex {
    /// Coverage records indexed by (file, function_name) for O(1) lookup
    by_function: HashMap<(PathBuf, String), FunctionCoverage>,

    /// Coverage records indexed by file path + line number for range queries
    /// BTreeMap allows efficient range queries for finding functions by line
    by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>>,

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
            by_function: HashMap::new(),
            by_line: HashMap::new(),
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
    /// 1. HashMap for exact (file, function_name) lookups
    /// 2. BTreeMap for line-based range queries
    pub fn from_coverage(coverage: &LcovData) -> Self {
        let start = Instant::now();

        let mut by_function = HashMap::new();
        let mut by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>> = HashMap::new();
        let mut total_records = 0;

        for (file_path, functions) in &coverage.functions {
            let mut line_map = BTreeMap::new();

            for func in functions {
                total_records += 1;

                // Index by exact (file, function_name) for O(1) lookup
                by_function.insert((file_path.clone(), func.name.clone()), func.clone());

                // Index by start_line for range queries
                line_map.insert(func.start_line, func.clone());
            }

            if !line_map.is_empty() {
                by_line.insert(file_path.clone(), line_map);
            }
        }

        let index_build_time = start.elapsed();
        let total_files = coverage.functions.len();

        // Estimate memory usage: ~200 bytes per record (rough estimate)
        let estimated_memory_bytes = total_records * 200;

        CoverageIndex {
            by_function,
            by_line,
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
        // Try exact match first
        if let Some(f) = self
            .by_function
            .get(&(file.to_path_buf(), function_name.to_string()))
        {
            return Some(f.coverage_percentage / 100.0);
        }

        // Try path matching strategies
        self.find_by_path_strategies(file, function_name)
            .map(|f| f.coverage_percentage / 100.0)
    }

    /// Try multiple path matching strategies to handle relative/absolute path mismatches
    fn find_by_path_strategies(
        &self,
        query_path: &Path,
        function_name: &str,
    ) -> Option<&FunctionCoverage> {
        let normalized_query = normalize_path(query_path);

        // Strategy 1: Check if query path ends with any indexed path
        for ((indexed_path, fname), coverage) in &self.by_function {
            if fname == function_name && query_path.ends_with(indexed_path) {
                return Some(coverage);
            }
        }

        // Strategy 2: Check if any indexed path ends with query path
        for ((indexed_path, fname), coverage) in &self.by_function {
            if fname == function_name && indexed_path.ends_with(&normalized_query) {
                return Some(coverage);
            }
        }

        // Strategy 3: Normalize both and compare
        for ((indexed_path, fname), coverage) in &self.by_function {
            if fname == function_name && normalize_path(indexed_path) == normalized_query {
                return Some(coverage);
            }
        }

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
        // Try exact name match with direct file path first (O(1))
        if let Some(f) = self
            .by_function
            .get(&(file.to_path_buf(), function_name.to_string()))
        {
            return Some(f.coverage_percentage / 100.0);
        }

        // Try line-based lookup first (O(log n)) - faster than path matching strategies
        if let Some(coverage) = self
            .find_function_by_line(file, line, 2)
            .map(|f| f.coverage_percentage / 100.0)
        {
            return Some(coverage);
        }

        // Only fall back to expensive path matching strategies if line lookup fails
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
        // Try exact name match first
        if let Some(func) = self
            .by_function
            .get(&(file.to_path_buf(), function_name.to_string()))
        {
            return Some(func.uncovered_lines.clone());
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

    fn create_test_coverage() -> LcovData {
        let mut coverage = LcovData::default();

        let test_functions = vec![
            FunctionCoverage {
                name: "func_a".to_string(),
                start_line: 10,
                execution_count: 5,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
            },
            FunctionCoverage {
                name: "func_b".to_string(),
                start_line: 20,
                execution_count: 3,
                coverage_percentage: 75.0,
                uncovered_lines: vec![22, 24],
            },
            FunctionCoverage {
                name: "func_c".to_string(),
                start_line: 30,
                execution_count: 0,
                coverage_percentage: 0.0,
                uncovered_lines: vec![30, 31, 32, 33],
            },
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
            vec![FunctionCoverage {
                name: "func1".to_string(),
                start_line: 5,
                execution_count: 10,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
            }],
        );

        coverage.functions.insert(
            PathBuf::from("file2.rs"),
            vec![FunctionCoverage {
                name: "func2".to_string(),
                start_line: 15,
                execution_count: 0,
                coverage_percentage: 0.0,
                uncovered_lines: vec![15, 16],
            }],
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
