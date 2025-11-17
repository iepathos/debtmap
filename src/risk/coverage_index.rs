use super::lcov::{FunctionCoverage, LcovData};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ============================================================================
// Pure Function Matching Logic
// ============================================================================

/// Match strategies for finding functions in coverage data
#[derive(Debug, PartialEq, Eq)]
enum MatchStrategy {
    /// Exact match on function name
    ExactName,
    /// Match on method name only (final segment after ::)
    MethodName,
    /// Match on suffix of normalized full path (e.g., Type::method matches crate::Type::method)
    SuffixPath,
}

/// Result of attempting to match a function
#[derive(Debug)]
struct FunctionMatch<'a> {
    coverage: &'a FunctionCoverage,
    strategy: MatchStrategy,
}

/// Pure function: Check if query function name matches a coverage record
///
/// Tries multiple strategies in order of specificity:
/// 1. Exact name match
/// 2. Suffix match on full path (for partial paths like "Type::method")
/// 3. Method name match (for bare method names like "method")
///
/// Note: Suffix matching must come before method name matching to correctly
/// distinguish "Type::method" (suffix) from "method" (method name).
fn matches_function(query: &str, coverage: &FunctionCoverage) -> Option<MatchStrategy> {
    // Strategy 1: Exact match
    if coverage.name == query {
        return Some(MatchStrategy::ExactName);
    }

    // Strategy 2: Suffix match on normalized full path
    // Handles: query="Type::method" matches full="crate::module::Type::method"
    // Must check this BEFORE method name to avoid false positives
    if query.contains("::") && coverage.normalized.full_path.ends_with(query) {
        // Verify it's a proper path segment boundary (preceded by :: or start of string)
        let full = &coverage.normalized.full_path;
        if full == query {
            return Some(MatchStrategy::ExactName);
        }
        // Check if the character before the match is :: or this is the full path
        if let Some(prefix_len) = full.len().checked_sub(query.len()) {
            if prefix_len == 0 || full[..prefix_len].ends_with("::") {
                return Some(MatchStrategy::SuffixPath);
            }
        }
    }

    // Strategy 3: Method name match (bare method name, no ::)
    // Only matches if query is a simple identifier without path separators
    if coverage.normalized.method_name == query {
        return Some(MatchStrategy::MethodName);
    }

    None
}

/// Pure function: Find first matching function in a collection
fn find_matching_function<'a>(
    query: &str,
    functions: &'a HashMap<String, FunctionCoverage>,
) -> Option<FunctionMatch<'a>> {
    // Try exact match first (O(1) hash lookup)
    if let Some(coverage) = functions.get(query) {
        return Some(FunctionMatch {
            coverage,
            strategy: MatchStrategy::ExactName,
        });
    }

    // Fall back to iterating and trying all strategies
    for coverage in functions.values() {
        if let Some(strategy) = matches_function(query, coverage) {
            return Some(FunctionMatch { coverage, strategy });
        }
    }

    None
}

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

        // O(1) exact match: file lookup + function lookup
        if let Some(file_functions) = self.by_file.get(file) {
            if let Some(f) = file_functions.get(function_name) {
                log::debug!(
                    "✓ Found via exact match: {}% coverage",
                    f.coverage_percentage
                );
                return Some(f.coverage_percentage / 100.0);
            }
        }

        log::debug!("Exact match failed, trying path strategies...");

        // O(files) fallback strategies - much faster than O(functions)
        self.find_by_path_strategies(file, function_name)
            .map(|f| f.coverage_percentage / 100.0)
    }

    /// Try multiple path matching strategies to handle relative/absolute path mismatches
    ///
    /// This method iterates over FILES (not functions), providing O(files) complexity
    /// instead of O(functions). For 375 files with ~4 functions each, this is
    /// 375 iterations vs 1,500, a 4x speedup.
    ///
    /// Uses pure function matching logic for each file found.
    fn find_by_path_strategies(
        &self,
        query_path: &Path,
        function_name: &str,
    ) -> Option<&FunctionCoverage> {
        let normalized_query = normalize_path(query_path);

        // Path matching strategies - try each one until we find a file match
        let path_strategies: [(
            &str,
            Box<dyn Fn(&Path, &Path, &PathBuf) -> bool>,
        ); 3] = [
            (
                "Suffix matching (query.ends_with(lcov_file))",
                Box::new(|query_path, _, file_path| query_path.ends_with(file_path)),
            ),
            (
                "Reverse suffix matching (lcov_file.ends_with(query))",
                Box::new(|_, normalized_query, file_path| file_path.ends_with(normalized_query)),
            ),
            (
                "Normalized path equality",
                Box::new(|_, normalized_query, file_path| {
                    normalize_path(file_path) == *normalized_query
                }),
            ),
        ];

        for (strategy_name, path_matches) in &path_strategies {
            log::debug!("Path strategy: {}", strategy_name);
            for file_path in &self.file_paths {
                if path_matches(query_path, &normalized_query, file_path) {
                    log::debug!("  Found path match: '{}'", file_path.display());
                    if let Some(file_functions) = self.by_file.get(file_path) {
                        // Use pure function matching
                        if let Some(func_match) =
                            find_matching_function(function_name, file_functions)
                        {
                            log::debug!(
                                "  ✓ Matched via {:?}: '{}' -> {}%",
                                func_match.strategy,
                                func_match.coverage.name,
                                func_match.coverage.coverage_percentage
                            );
                            return Some(func_match.coverage);
                        }
                        log::debug!("  ✗ No function match in this file");
                    }
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

    // ========================================================================
    // Pure Function Matching Tests
    // ========================================================================

    mod function_matching_tests {
        use super::*;
        use crate::risk::lcov::NormalizedFunctionName;

        fn make_coverage(name: &str, full_path: &str, method_name: &str) -> FunctionCoverage {
            FunctionCoverage {
                name: name.to_string(),
                start_line: 1,
                execution_count: 0,
                coverage_percentage: 50.0,
                uncovered_lines: vec![],
                normalized: NormalizedFunctionName {
                    full_path: full_path.to_string(),
                    method_name: method_name.to_string(),
                    original: name.to_string(),
                },
            }
        }

        #[test]
        fn test_exact_name_match() {
            let coverage = make_coverage(
                "prodigy::cook::workflow::resume::execute_mapreduce_resume",
                "prodigy::cook::workflow::resume::execute_mapreduce_resume",
                "execute_mapreduce_resume",
            );

            let strategy = matches_function(
                "prodigy::cook::workflow::resume::execute_mapreduce_resume",
                &coverage,
            );
            assert_eq!(strategy, Some(MatchStrategy::ExactName));
        }

        #[test]
        fn test_suffix_path_match_impl_method() {
            let coverage = make_coverage(
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "execute_remaining_steps",
            );

            // Query with Type::method (what debtmap typically uses)
            let strategy =
                matches_function("ResumeExecutor::execute_remaining_steps", &coverage);
            assert_eq!(
                strategy,
                Some(MatchStrategy::SuffixPath),
                "Should match Type::method against full::path::Type::method"
            );
        }

        #[test]
        fn test_suffix_path_match_with_module() {
            let coverage = make_coverage(
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "execute_remaining_steps",
            );

            // Query with module::Type::method
            let strategy = matches_function(
                "resume::ResumeExecutor::execute_remaining_steps",
                &coverage,
            );
            assert_eq!(
                strategy,
                Some(MatchStrategy::SuffixPath),
                "Should match module::Type::method suffix"
            );
        }

        #[test]
        fn test_method_name_only_match() {
            let coverage = make_coverage(
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "execute_remaining_steps",
            );

            // Query with bare method name
            let strategy = matches_function("execute_remaining_steps", &coverage);
            assert_eq!(
                strategy,
                Some(MatchStrategy::MethodName),
                "Should match bare method name"
            );
        }

        #[test]
        fn test_no_match_partial_type_name() {
            let coverage = make_coverage(
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "execute_remaining_steps",
            );

            // Should NOT match partial type name (not on :: boundary)
            let strategy = matches_function("Executor::execute_remaining_steps", &coverage);
            assert_eq!(
                strategy, None,
                "Should NOT match partial type name that doesn't align with :: boundary"
            );
        }

        #[test]
        fn test_no_match_wrong_function() {
            let coverage = make_coverage(
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "execute_remaining_steps",
            );

            let strategy = matches_function("different_function", &coverage);
            assert_eq!(strategy, None, "Should not match wrong function name");
        }

        #[test]
        fn test_find_matching_function_exact() {
            let mut functions = HashMap::new();
            functions.insert(
                "full::path::func".to_string(),
                make_coverage("full::path::func", "full::path::func", "func"),
            );

            let result = find_matching_function("full::path::func", &functions);
            assert!(result.is_some());
            assert_eq!(result.unwrap().strategy, MatchStrategy::ExactName);
        }

        #[test]
        fn test_find_matching_function_suffix() {
            let mut functions = HashMap::new();
            functions.insert(
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps"
                    .to_string(),
                make_coverage(
                    "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                    "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                    "execute_remaining_steps",
                ),
            );

            let result =
                find_matching_function("ResumeExecutor::execute_remaining_steps", &functions);
            assert!(
                result.is_some(),
                "Should find function using suffix path match"
            );
            assert_eq!(result.unwrap().strategy, MatchStrategy::SuffixPath);
        }

        #[test]
        fn test_find_matching_function_method_name() {
            let mut functions = HashMap::new();
            functions.insert(
                "crate::module::Type::method".to_string(),
                make_coverage("crate::module::Type::method", "crate::module::Type::method", "method"),
            );

            let result = find_matching_function("method", &functions);
            assert!(result.is_some(), "Should find function using method name");
            assert_eq!(result.unwrap().strategy, MatchStrategy::MethodName);
        }

        #[test]
        fn test_find_matching_function_priority_order() {
            // Test that exact match takes priority over suffix match
            let mut functions = HashMap::new();

            // Add two functions where one is exact and one is suffix
            functions.insert(
                "Type::method".to_string(),
                make_coverage("Type::method", "Type::method", "method"),
            );
            functions.insert(
                "crate::other::Type::method".to_string(),
                make_coverage(
                    "crate::other::Type::method",
                    "crate::other::Type::method",
                    "method",
                ),
            );

            let result = find_matching_function("Type::method", &functions);
            assert!(result.is_some());
            // Should find the exact match via hash lookup
            assert_eq!(result.unwrap().strategy, MatchStrategy::ExactName);
        }

        #[test]
        fn test_matches_function_standalone_vs_impl() {
            // Standalone function
            let standalone = make_coverage(
                "prodigy::cook::workflow::resume::execute_mapreduce_resume",
                "prodigy::cook::workflow::resume::execute_mapreduce_resume",
                "execute_mapreduce_resume",
            );

            // Should match bare function name
            assert_eq!(
                matches_function("execute_mapreduce_resume", &standalone),
                Some(MatchStrategy::MethodName)
            );

            // Should match full path
            assert_eq!(
                matches_function(
                    "prodigy::cook::workflow::resume::execute_mapreduce_resume",
                    &standalone
                ),
                Some(MatchStrategy::ExactName)
            );

            // Impl method
            let impl_method = make_coverage(
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "prodigy::cook::workflow::resume::ResumeExecutor::execute_remaining_steps",
                "execute_remaining_steps",
            );

            // Should match Type::method
            assert_eq!(
                matches_function("ResumeExecutor::execute_remaining_steps", &impl_method),
                Some(MatchStrategy::SuffixPath)
            );

            // Should match bare method name
            assert_eq!(
                matches_function("execute_remaining_steps", &impl_method),
                Some(MatchStrategy::MethodName)
            );
        }
    }
}
