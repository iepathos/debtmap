use super::lcov::{normalize_demangled_name, strip_trailing_generics, FunctionCoverage, LcovData};
use super::path_normalization::normalize_path_components;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Normalize a path by removing leading ./
///
/// # Deprecated
///
/// This function is deprecated in favor of `normalize_path_components()` which
/// provides better cross-platform support. It is kept for backward compatibility
/// and will be removed in a future version.
#[deprecated(
    since = "0.1.0",
    note = "Use normalize_path_components() for better cross-platform support"
)]
pub fn normalize_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);
    PathBuf::from(cleaned)
}

/// Generate name variants for coverage matching
///
/// For trait implementation methods, LCOV may store just the method name
/// (e.g., `visit_expr`) while debtmap stores the full qualified name
/// (e.g., `RecursiveMatchDetector::visit_expr`).
///
/// This function generates variant names to try during coverage lookup:
/// 1. Method name only (last segment after `::`), if the name contains `::`
///
/// # Examples
///
/// ```
/// # use debtmap::risk::coverage_index::generate_name_variants;
/// let variants: Vec<String> = generate_name_variants("RecursiveMatchDetector::visit_expr").collect();
/// assert_eq!(variants, vec!["visit_expr"]);
///
/// let variants: Vec<String> = generate_name_variants("simple_function").collect();
/// assert_eq!(variants.len(), 0); // No variants for simple functions
/// ```
///
/// # Performance
///
/// O(1) time complexity - only splits on `::` delimiter
pub fn generate_name_variants(function_name: &str) -> impl Iterator<Item = String> + '_ {
    // Extract method name from "Type::method" or "path::to::Type::method"
    function_name
        .rsplit("::")
        .next()
        .filter(|method_name| {
            // Only generate variant if:
            // 1. The original name contains :: (is qualified)
            // 2. The method name is different from the full name
            function_name.contains("::") && *method_name != function_name
        })
        .map(|s| s.to_string())
        .into_iter()
}

/// Aggregated coverage from multiple monomorphized versions of a generic function
///
/// Uses intersection strategy: a line is covered if ANY monomorphization covers it
/// (i.e., uncovered only if ALL versions leave it uncovered).
#[derive(Debug, Clone)]
pub struct AggregateCoverage {
    /// Aggregate coverage percentage (averaged across all versions)
    pub coverage_pct: f64,
    /// Intersection of uncovered lines across all versions
    pub uncovered_lines: Vec<usize>,
    /// Number of monomorphized versions found
    pub version_count: usize,
}

impl AggregateCoverage {
    /// Create an aggregate from a single function (no monomorphization)
    fn single(func: &FunctionCoverage) -> Self {
        Self {
            coverage_pct: func.coverage_percentage,
            uncovered_lines: func.uncovered_lines.clone(),
            version_count: 1,
        }
    }
}

/// Pure function: Merge coverage data from multiple monomorphizations.
///
/// Uses intersection strategy: a line is uncovered only if ALL versions leave it uncovered.
/// This conservative approach ensures we don't claim coverage that doesn't exist in all paths.
///
/// # Strategy
///
/// - A line is **covered** if ANY monomorphization covers it
/// - A line is **uncovered** only if ALL monomorphizations leave it uncovered
/// - Coverage percentage is averaged across all versions
///
/// # Examples
///
/// For a generic function with two monomorphizations:
/// - `execute::<WorkflowExecutor>` - 70% coverage, uncovered: [10, 20, 30]
/// - `execute::<MockExecutor>` - 80% coverage, uncovered: [20, 40]
///
/// Result: 75% coverage (average), uncovered: \[20\] (intersection - only line uncovered in BOTH)
///
/// # Performance
///
/// O(m*n) time complexity where m is number of monomorphizations
/// and n is average number of uncovered lines.
fn merge_coverage(coverages: Vec<&FunctionCoverage>) -> AggregateCoverage {
    if coverages.is_empty() {
        return AggregateCoverage {
            coverage_pct: 0.0,
            uncovered_lines: vec![],
            version_count: 0,
        };
    }

    if coverages.len() == 1 {
        return AggregateCoverage {
            coverage_pct: coverages[0].coverage_percentage,
            uncovered_lines: coverages[0].uncovered_lines.clone(),
            version_count: 1,
        };
    }

    // Intersection strategy: line is uncovered only if ALL versions leave it uncovered
    let mut uncovered_in_all: HashSet<usize> =
        coverages[0].uncovered_lines.iter().copied().collect();

    for coverage in &coverages[1..] {
        let uncovered_set: HashSet<usize> = coverage.uncovered_lines.iter().copied().collect();
        uncovered_in_all = uncovered_in_all
            .intersection(&uncovered_set)
            .copied()
            .collect();
    }

    // Average coverage percentage across all versions
    let avg_coverage: f64 =
        coverages.iter().map(|c| c.coverage_percentage).sum::<f64>() / coverages.len() as f64;

    let mut uncovered_lines: Vec<usize> = uncovered_in_all.into_iter().collect();
    uncovered_lines.sort_unstable();

    AggregateCoverage {
        coverage_pct: avg_coverage,
        uncovered_lines,
        version_count: coverages.len(),
    }
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

    /// Index from base function name to all monomorphized versions
    /// Maps (file, base_name) -> \[monomorphized_names\] for O(1) generic function lookup
    base_function_index: HashMap<(PathBuf, String), Vec<String>>,

    /// Index from method name to actual function names for trait method matching
    /// Maps (file, method_name) -> \[actual_function_names\] for O(1) variant matching
    /// Example: (recursive_detector.rs, "visit_expr") -> ["_RNvXs0_...visit_expr"]
    method_name_index: HashMap<(PathBuf, String), Vec<String>>,

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
            base_function_index: HashMap::new(),
            method_name_index: HashMap::new(),
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
    /// This creates four indexes:
    /// 1. Nested HashMap for O(1) file + function lookups
    /// 2. BTreeMap for line-based range queries
    /// 3. Base function index for generic/monomorphized function aggregation
    /// 4. Method name index for trait method variant matching
    pub fn from_coverage(coverage: &LcovData) -> Self {
        let start = Instant::now();

        let mut by_file: HashMap<PathBuf, HashMap<String, FunctionCoverage>> = HashMap::new();
        let mut by_line: HashMap<PathBuf, BTreeMap<usize, FunctionCoverage>> = HashMap::new();
        let mut base_function_index: HashMap<(PathBuf, String), Vec<String>> = HashMap::new();
        let mut method_name_index: HashMap<(PathBuf, String), Vec<String>> = HashMap::new();
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

                // Extract base name and update generic function index
                let base_name_cow = strip_trailing_generics(&func.name);
                let base_name = base_name_cow.as_ref();
                if base_name != func.name {
                    // This is a monomorphized function - add to index
                    base_function_index
                        .entry((file_path.clone(), base_name.to_string()))
                        .or_default()
                        .push(func.name.clone());
                }

                // Extract method name for trait method matching
                // Use the normalized method_name field if available
                let method_name = &func.normalized.method_name;
                if !method_name.is_empty() && method_name != &func.name {
                    // Add to method name index for O(1) variant lookups
                    method_name_index
                        .entry((file_path.clone(), method_name.clone()))
                        .or_default()
                        .push(func.name.clone());
                }
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
            base_function_index,
            method_name_index,
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
    /// 1. Component-based suffix matching (robust cross-platform)
    /// 2. Legacy path suffix matching (backward compatibility)
    /// 3. Component-based exact match
    fn find_by_path_strategies(
        &self,
        query_path: &Path,
        function_name: &str,
    ) -> Option<&FunctionCoverage> {
        let query_components = normalize_path_components(query_path);

        log::debug!("Strategy 1: Component-based suffix matching");
        // Strategy 1: Component-based suffix matching - cross-platform robust
        for file_path in &self.file_paths {
            let file_components = normalize_path_components(file_path);

            // Check if query components are a suffix of file components
            if !query_components.is_empty() && query_components.len() <= file_components.len() {
                let file_suffix =
                    &file_components[file_components.len() - query_components.len()..];
                if query_components == file_suffix {
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
                            if func.name.ends_with(function_name)
                                || function_name.ends_with(&func.name)
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
        }

        log::debug!("Strategy 2: Component-based reverse suffix matching");
        // Strategy 2: Reverse suffix matching - file is suffix of query
        for file_path in &self.file_paths {
            let file_components = normalize_path_components(file_path);

            // Check if file components are a suffix of query components
            if !file_components.is_empty() && file_components.len() <= query_components.len() {
                let query_suffix =
                    &query_components[query_components.len() - file_components.len()..];
                if file_components == query_suffix {
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
                            if func.name.ends_with(function_name)
                                || function_name.ends_with(&func.name)
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
        }

        log::debug!("Strategy 3: Component-based exact equality");
        // Strategy 3: Exact component match
        for file_path in &self.file_paths {
            let file_components = normalize_path_components(file_path);
            if query_components == file_components {
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
    /// Also tries aggregated coverage for generic/monomorphized functions.
    ///
    /// # Name Variant Matching (for trait methods)
    ///
    /// Tries multiple name variants before falling back to line-based lookup:
    /// 1. Full qualified name (e.g., `RecursiveMatchDetector::visit_expr`)
    /// 2. Method name only (e.g., `visit_expr`)
    ///
    /// This handles cases where LCOV stores demangled symbols with just the method
    /// name, while debtmap stores the full qualified name including the impl type.
    pub fn get_function_coverage_with_line(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<f64> {
        log::debug!(
            "Attempting coverage lookup for '{}' at {}:{}",
            function_name,
            file.display(),
            line
        );

        // Try aggregated coverage first (handles generics)
        if let Some(agg) = self.get_aggregated_coverage(file, function_name) {
            log::debug!(
                "✓ Coverage found via aggregated match: {:.1}%",
                agg.coverage_pct
            );
            return Some(agg.coverage_pct / 100.0);
        }

        // Try name variants (for trait methods where LCOV may use simplified names)
        for variant in generate_name_variants(function_name) {
            log::debug!("Trying name variant: '{}'", variant);
            // First try O(1) aggregated lookup
            if let Some(agg) = self.get_aggregated_coverage(file, &variant) {
                log::debug!(
                    "✓ Coverage found via name variant '{}': {:.1}%",
                    variant,
                    agg.coverage_pct
                );
                return Some(agg.coverage_pct / 100.0);
            }
            // If that fails, try path strategies (handles file path mismatches)
            if let Some(func) = self.find_by_path_strategies(file, &variant) {
                log::debug!(
                    "✓ Coverage found via name variant '{}' with path strategies: {:.1}%",
                    variant,
                    func.coverage_percentage
                );
                return Some(func.coverage_percentage / 100.0);
            }
        }

        // Try line-based lookup (O(log n)) - faster than path matching strategies
        log::debug!("Trying line-based lookup with tolerance ±2");
        match self.find_function_by_line(file, line, 2) {
            Some(f) => {
                log::debug!(
                    "✓ Coverage found via line-based fallback: matched '{}' at line {}, coverage {:.1}%",
                    f.name,
                    f.start_line,
                    f.coverage_percentage
                );
                return Some(f.coverage_percentage / 100.0);
            }
            None => {
                // Log diagnostic info about why line-based lookup failed
                if !self.by_line.contains_key(file) {
                    log::warn!(
                        "File '{}' not found in line-based index (has {} files indexed)",
                        file.display(),
                        self.by_line.len()
                    );
                } else {
                    let line_map = &self.by_line[file];
                    log::debug!(
                        "Line-based lookup failed: file has {} indexed functions, searched for line {} with ±2 tolerance",
                        line_map.len(),
                        line
                    );

                    // Show nearby lines to help diagnose tolerance issues
                    let min_line = line.saturating_sub(5);
                    let max_line = line.saturating_add(5);
                    let nearby_lines: Vec<usize> = line_map
                        .range(min_line..=max_line)
                        .map(|(l, _)| *l)
                        .collect();
                    if !nearby_lines.is_empty() {
                        log::debug!("Nearby indexed lines (±5): {:?}", nearby_lines);
                    }
                }
            }
        }

        // Only fall back to path matching strategies if line lookup fails
        log::debug!("Trying path matching strategies");
        match self.find_by_path_strategies(file, function_name) {
            Some(f) => {
                log::debug!(
                    "✓ Coverage found via path matching: {:.1}%",
                    f.coverage_percentage
                );
                Some(f.coverage_percentage / 100.0)
            }
            None => {
                log::warn!(
                    "✗ No coverage found for '{}' at {}:{} after all strategies",
                    function_name,
                    file.display(),
                    line
                );
                None
            }
        }
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
    /// This is a fallback mechanism for when name-based matching fails.
    /// It's particularly useful for:
    /// - Trait implementation methods (name format varies)
    /// - Generic functions (multiple monomorphizations)
    /// - Functions where LCOV and AST disagree on naming
    ///
    /// # Arguments
    /// * `file` - Path to source file
    /// * `target_line` - Line number to search for
    /// * `tolerance` - Number of lines above/below to check (typically 2)
    ///
    /// # Returns
    /// The closest function within tolerance, or None if no match found.
    ///
    /// # Algorithm
    /// Uses BTreeMap range query for O(log n) performance. When multiple
    /// functions are within tolerance, returns the closest by absolute distance.
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

        log::trace!(
            "Searching line-based index: target={}, range={}..={}, index_size={}",
            target_line,
            min_line,
            max_line,
            line_map.len()
        );

        // Use BTreeMap range query to find functions in range (inclusive on both ends)
        let result = line_map
            .range(min_line..=max_line)
            .min_by_key(|(line, _)| line.abs_diff(target_line))
            .map(|(_, func)| func);

        if let Some(func) = result {
            log::trace!(
                "Found function '{}' at line {} (distance: {})",
                func.name,
                func.start_line,
                func.start_line.abs_diff(target_line)
            );
        } else {
            log::trace!("No function found within tolerance range");
        }

        result
    }

    /// Get index statistics
    pub fn stats(&self) -> &CoverageIndexStats {
        &self.stats
    }

    /// Find all monomorphizations of a function and aggregate coverage.
    ///
    /// Uses pre-built index for O(1) lookup of monomorphized versions.
    ///
    /// # Arguments
    ///
    /// * `file` - The file path containing the function
    /// * `function_name` - The base function name (without generic parameters)
    ///
    /// # Returns
    ///
    /// `Some(AggregateCoverage)` if any monomorphizations are found, `None` otherwise
    fn get_aggregated_coverage(
        &self,
        file: &Path,
        function_name: &str,
    ) -> Option<AggregateCoverage> {
        // Try exact match first (O(1))
        if let Some(file_functions) = self.by_file.get(file) {
            if let Some(exact) = file_functions.get(function_name) {
                return Some(AggregateCoverage::single(exact));
            }
        }

        // Try method name index for trait methods (O(1))
        // This handles cases where LCOV stores "visit_expr" but we're looking up "Type::visit_expr"
        if let Some(matching_functions) = self
            .method_name_index
            .get(&(file.to_path_buf(), function_name.to_string()))
        {
            let coverages: Vec<&FunctionCoverage> = matching_functions
                .iter()
                .filter_map(|name| self.by_file.get(file).and_then(|funcs| funcs.get(name)))
                .collect();

            if !coverages.is_empty() {
                log::debug!(
                    "✓ Found {} matching functions via method_name_index for '{}'",
                    coverages.len(),
                    function_name
                );
                return Some(merge_coverage(coverages));
            }
        }

        // Try monomorphized versions using index (O(1))
        if let Some(versions) = self
            .base_function_index
            .get(&(file.to_path_buf(), function_name.to_string()))
        {
            let coverages: Vec<&FunctionCoverage> = versions
                .iter()
                .filter_map(|name| self.by_file.get(file).and_then(|funcs| funcs.get(name)))
                .collect();

            if !coverages.is_empty() {
                return Some(merge_coverage(coverages));
            }
        }

        None
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

    #[test]
    fn test_merge_coverage_intersection() {
        let cov1 = create_test_function_coverage("func", 10, 5, 70.0, vec![10, 20, 30]);
        let cov2 = create_test_function_coverage("func", 10, 3, 80.0, vec![20, 40]);

        let agg = merge_coverage(vec![&cov1, &cov2]);
        assert_eq!(agg.version_count, 2);
        assert_eq!(agg.coverage_pct, 75.0); // Average: (70 + 80) / 2
                                            // Intersection: only line 20 is uncovered in BOTH versions
        assert_eq!(agg.uncovered_lines.len(), 1);
        assert!(agg.uncovered_lines.contains(&20));
        assert!(!agg.uncovered_lines.contains(&10)); // Covered in cov2
        assert!(!agg.uncovered_lines.contains(&40)); // Covered in cov1
    }

    #[test]
    fn test_merge_coverage_all_covered_in_some() {
        // If ANY version covers a line, it's considered covered (intersection)
        let cov1 = create_test_function_coverage("func", 10, 5, 50.0, vec![10, 20]);
        let cov2 = create_test_function_coverage("func", 10, 3, 50.0, vec![30, 40]);

        let agg = merge_coverage(vec![&cov1, &cov2]);
        // No lines uncovered in BOTH versions
        assert_eq!(agg.uncovered_lines.len(), 0);
    }

    #[test]
    fn test_merge_coverage_single() {
        let cov = create_test_function_coverage("func", 10, 5, 75.0, vec![10, 20]);

        let agg = merge_coverage(vec![&cov]);
        assert_eq!(agg.version_count, 1);
        assert_eq!(agg.coverage_pct, 75.0);
        assert_eq!(agg.uncovered_lines, vec![10, 20]);
    }

    #[test]
    fn test_merge_coverage_empty() {
        let agg = merge_coverage(vec![]);
        assert_eq!(agg.version_count, 0);
        assert_eq!(agg.coverage_pct, 0.0);
        assert_eq!(agg.uncovered_lines.len(), 0);
    }

    #[test]
    fn test_monomorphized_function_indexing() {
        use crate::risk::lcov::NormalizedFunctionName;

        let mut coverage = LcovData::default();

        // Create monomorphized versions of the same function
        coverage.functions.insert(
            PathBuf::from("test.rs"),
            vec![
                FunctionCoverage {
                    name: "Type::method::<WorkflowExecutor>".to_string(),
                    start_line: 10,
                    execution_count: 5,
                    coverage_percentage: 70.0,
                    uncovered_lines: vec![10, 20, 30],
                    normalized: NormalizedFunctionName {
                        full_path: "Type::method".to_string(),
                        method_name: "method".to_string(),
                        original: "Type::method::<WorkflowExecutor>".to_string(),
                    },
                },
                FunctionCoverage {
                    name: "Type::method::<MockExecutor>".to_string(),
                    start_line: 10,
                    execution_count: 3,
                    coverage_percentage: 80.0,
                    uncovered_lines: vec![20, 40],
                    normalized: NormalizedFunctionName {
                        full_path: "Type::method".to_string(),
                        method_name: "method".to_string(),
                        original: "Type::method::<MockExecutor>".to_string(),
                    },
                },
            ],
        );

        let index = CoverageIndex::from_coverage(&coverage);

        // Query for the base function name (without generics)
        let agg = index.get_aggregated_coverage(Path::new("test.rs"), "Type::method");
        assert!(agg.is_some());

        let agg = agg.unwrap();
        assert_eq!(agg.version_count, 2);
        assert_eq!(agg.coverage_pct, 75.0); // (70 + 80) / 2
                                            // Only line 20 is uncovered in both versions
        assert_eq!(agg.uncovered_lines, vec![20]);
    }

    #[test]
    fn test_generate_name_variants_trait_method() {
        let variants: Vec<String> =
            generate_name_variants("RecursiveMatchDetector::visit_expr").collect();
        assert_eq!(variants, vec!["visit_expr"]);
    }

    #[test]
    fn test_generate_name_variants_nested_path() {
        let variants: Vec<String> = generate_name_variants("crate::module::Type::method").collect();
        assert_eq!(variants, vec!["method"]);
    }

    #[test]
    fn test_generate_name_variants_simple_function() {
        let variants: Vec<String> = generate_name_variants("simple_function").collect();
        assert_eq!(variants.len(), 0); // No variants for functions without ::
    }

    #[test]
    fn test_generate_name_variants_single_segment() {
        let variants: Vec<String> = generate_name_variants("main").collect();
        assert_eq!(variants.len(), 0); // No variants for single segment names
    }

    #[test]
    fn test_trait_method_coverage_match_by_method_name() {
        use crate::risk::lcov::NormalizedFunctionName;

        let mut coverage = LcovData::default();

        // Simulate LCOV storing just the method name (common for trait implementations)
        coverage.functions.insert(
            PathBuf::from("src/complexity/recursive_detector.rs"),
            vec![FunctionCoverage {
                name: "visit_expr".to_string(),
                start_line: 177,
                execution_count: 3507,
                coverage_percentage: 90.2,
                uncovered_lines: vec![200, 205],
                normalized: NormalizedFunctionName {
                    full_path: "visit_expr".to_string(),
                    method_name: "visit_expr".to_string(),
                    original: "visit_expr".to_string(),
                },
            }],
        );

        let index = CoverageIndex::from_coverage(&coverage);

        // Query with full qualified name (what debtmap stores)
        let coverage = index.get_function_coverage_with_line(
            Path::new("src/complexity/recursive_detector.rs"),
            "RecursiveMatchDetector::visit_expr",
            177,
        );

        // Should find coverage via method name variant matching
        assert!(coverage.is_some());
        assert_eq!(coverage.unwrap(), 0.902); // 90.2% as fraction
    }

    #[test]
    fn test_trait_method_coverage_no_regression_exact_match() {
        use crate::risk::lcov::NormalizedFunctionName;

        let mut coverage = LcovData::default();

        // LCOV stores full qualified name (ideal case)
        coverage.functions.insert(
            PathBuf::from("src/test.rs"),
            vec![FunctionCoverage {
                name: "MyType::my_method".to_string(),
                start_line: 10,
                execution_count: 100,
                coverage_percentage: 95.0,
                uncovered_lines: vec![15],
                normalized: NormalizedFunctionName {
                    full_path: "MyType::my_method".to_string(),
                    method_name: "my_method".to_string(),
                    original: "MyType::my_method".to_string(),
                },
            }],
        );

        let index = CoverageIndex::from_coverage(&coverage);

        // Query with full qualified name - should still work via exact match
        let coverage = index.get_function_coverage_with_line(
            Path::new("src/test.rs"),
            "MyType::my_method",
            10,
        );

        assert!(coverage.is_some());
        assert_eq!(coverage.unwrap(), 0.95);
    }

    #[test]
    fn test_trait_method_coverage_method_name_conflict() {
        use crate::risk::lcov::NormalizedFunctionName;

        let mut coverage = LcovData::default();

        // Two different types with same method name, both stored in LCOV
        coverage.functions.insert(
            PathBuf::from("src/test.rs"),
            vec![
                FunctionCoverage {
                    name: "TypeA::process".to_string(),
                    start_line: 10,
                    execution_count: 50,
                    coverage_percentage: 80.0,
                    uncovered_lines: vec![12],
                    normalized: NormalizedFunctionName {
                        full_path: "TypeA::process".to_string(),
                        method_name: "process".to_string(),
                        original: "TypeA::process".to_string(),
                    },
                },
                FunctionCoverage {
                    name: "TypeB::process".to_string(),
                    start_line: 30,
                    execution_count: 75,
                    coverage_percentage: 90.0,
                    uncovered_lines: vec![35],
                    normalized: NormalizedFunctionName {
                        full_path: "TypeB::process".to_string(),
                        method_name: "process".to_string(),
                        original: "TypeB::process".to_string(),
                    },
                },
            ],
        );

        let index = CoverageIndex::from_coverage(&coverage);

        // Query TypeA::process - should get correct coverage
        let coverage_a =
            index.get_function_coverage_with_line(Path::new("src/test.rs"), "TypeA::process", 10);
        assert_eq!(coverage_a.unwrap(), 0.80);

        // Query TypeB::process - should get correct coverage
        let coverage_b =
            index.get_function_coverage_with_line(Path::new("src/test.rs"), "TypeB::process", 30);
        assert_eq!(coverage_b.unwrap(), 0.90);
    }

    // Tests for Spec 182: Line-Based Coverage Fallback Reliability

    #[test]
    fn test_line_based_fallback_exact_match() {
        let mut coverage = LcovData::default();
        coverage.functions.insert(
            PathBuf::from("test.rs"),
            vec![create_test_function_coverage("foo", 100, 10, 85.0, vec![])],
        );
        let index = CoverageIndex::from_coverage(&coverage);

        // Query with wrong name but exact line - should find via line-based fallback
        let result = index.get_function_coverage_with_line(
            Path::new("test.rs"),
            "WRONG_NAME", // Name won't match
            100,          // Exact line
        );

        assert!(
            result.is_some(),
            "Line-based fallback should find function at exact line"
        );
        assert_eq!(result.unwrap(), 0.85);
    }

    #[test]
    fn test_line_based_fallback_within_tolerance() {
        let mut coverage = LcovData::default();
        coverage.functions.insert(
            PathBuf::from("test.rs"),
            vec![create_test_function_coverage("foo", 100, 10, 85.0, vec![])],
        );
        let index = CoverageIndex::from_coverage(&coverage);

        // Try lines 98-102 (all within ±2 tolerance)
        for line in 98..=102 {
            let result =
                index.get_function_coverage_with_line(Path::new("test.rs"), "WRONG_NAME", line);

            assert!(
                result.is_some(),
                "Line {} should match function at 100 with ±2 tolerance",
                line
            );
            assert_eq!(result.unwrap(), 0.85);
        }
    }

    #[test]
    fn test_line_based_fallback_outside_tolerance() {
        let mut coverage = LcovData::default();
        coverage.functions.insert(
            PathBuf::from("test.rs"),
            vec![create_test_function_coverage("foo", 100, 10, 85.0, vec![])],
        );
        let index = CoverageIndex::from_coverage(&coverage);

        // Line 97 is just outside ±2 tolerance (100 - 2 = 98)
        let result = index.get_function_coverage_with_line(Path::new("test.rs"), "WRONG_NAME", 97);

        assert!(
            result.is_none(),
            "Line 97 should be outside ±2 tolerance of line 100"
        );

        // Line 103 is just outside ±2 tolerance (100 + 2 = 102)
        let result = index.get_function_coverage_with_line(Path::new("test.rs"), "WRONG_NAME", 103);

        assert!(
            result.is_none(),
            "Line 103 should be outside ±2 tolerance of line 100"
        );
    }

    #[test]
    fn test_line_based_fallback_chooses_closest() {
        let mut coverage = LcovData::default();
        coverage.functions.insert(
            PathBuf::from("test.rs"),
            vec![
                create_test_function_coverage("func_at_100", 100, 10, 80.0, vec![]),
                create_test_function_coverage("func_at_105", 105, 10, 90.0, vec![]),
            ],
        );
        let index = CoverageIndex::from_coverage(&coverage);

        // Line 102 is closer to 100 (distance 2) than 105 (distance 3)
        let result = index.get_function_coverage_with_line(Path::new("test.rs"), "WRONG_NAME", 102);

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            0.80,
            "Should match function at line 100 (closer)"
        );

        // Line 103 is closer to 105 (distance 2) than 100 (distance 3)
        let result = index.get_function_coverage_with_line(Path::new("test.rs"), "WRONG_NAME", 104);

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            0.90,
            "Should match function at line 105 (closer)"
        );
    }

    #[test]
    fn test_line_based_fallback_boundary_conditions() {
        let mut coverage = LcovData::default();
        coverage.functions.insert(
            PathBuf::from("test.rs"),
            vec![
                create_test_function_coverage("func_at_0", 0, 10, 70.0, vec![]),
                create_test_function_coverage(
                    "func_at_usize_max",
                    usize::MAX - 5,
                    10,
                    75.0,
                    vec![],
                ),
            ],
        );
        let index = CoverageIndex::from_coverage(&coverage);

        // Test line 0 with tolerance (should handle underflow correctly)
        let result = index.get_function_coverage_with_line(Path::new("test.rs"), "WRONG_NAME", 0);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 0.70);

        // Test near usize::MAX (should handle overflow correctly)
        let result = index.get_function_coverage_with_line(
            Path::new("test.rs"),
            "WRONG_NAME",
            usize::MAX - 4,
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 0.75);
    }

    #[test]
    fn test_line_index_populated_for_all_functions() {
        let mut coverage = LcovData::default();
        let test_functions = vec![
            create_test_function_coverage("func_a", 10, 5, 100.0, vec![]),
            create_test_function_coverage("func_b", 20, 3, 75.0, vec![22, 24]),
            create_test_function_coverage("func_c", 30, 0, 0.0, vec![30, 31, 32, 33]),
        ];
        coverage
            .functions
            .insert(PathBuf::from("test.rs"), test_functions);

        let index = CoverageIndex::from_coverage(&coverage);

        // Verify all functions are in the line index
        let file = Path::new("test.rs");
        assert!(
            index.by_line.contains_key(file),
            "File should be in line index"
        );

        let line_map = &index.by_line[file];
        assert_eq!(line_map.len(), 3, "All 3 functions should be in line index");
        assert!(
            line_map.contains_key(&10),
            "Function at line 10 should be indexed"
        );
        assert!(
            line_map.contains_key(&20),
            "Function at line 20 should be indexed"
        );
        assert!(
            line_map.contains_key(&30),
            "Function at line 30 should be indexed"
        );
    }

    #[test]
    fn test_line_based_fallback_with_trait_method() {
        use crate::risk::lcov::NormalizedFunctionName;

        let mut coverage = LcovData::default();

        // Simulate LCOV storing just the method name (common for trait implementations)
        coverage.functions.insert(
            PathBuf::from("src/test.rs"),
            vec![FunctionCoverage {
                name: "visit_expr".to_string(),
                start_line: 177,
                execution_count: 3507,
                coverage_percentage: 90.2,
                uncovered_lines: vec![200, 205],
                normalized: NormalizedFunctionName {
                    full_path: "visit_expr".to_string(),
                    method_name: "visit_expr".to_string(),
                    original: "visit_expr".to_string(),
                },
            }],
        );

        let index = CoverageIndex::from_coverage(&coverage);

        // Query with full qualified name that doesn't match, but correct line
        // This simulates the RecursiveMatchDetector::visit_expr case
        let coverage = index.get_function_coverage_with_line(
            Path::new("src/test.rs"),
            "SomeType::visit_expr", // Won't match stored "visit_expr" by aggregated lookup
            177,
        );

        // Should find coverage via line-based fallback
        assert!(
            coverage.is_some(),
            "Line-based fallback should find coverage when name doesn't match exactly"
        );
        assert_eq!(coverage.unwrap(), 0.902);
    }

    #[test]
    fn test_line_based_fallback_empty_file() {
        let coverage = LcovData::default();
        let index = CoverageIndex::from_coverage(&coverage);

        // Query for non-existent file
        let result =
            index.get_function_coverage_with_line(Path::new("nonexistent.rs"), "func", 100);

        assert!(result.is_none(), "Should return None for non-existent file");
    }

    #[test]
    fn test_tolerance_calculation_inclusive_range() {
        let mut coverage = LcovData::default();
        coverage.functions.insert(
            PathBuf::from("test.rs"),
            vec![
                create_test_function_coverage("func_98", 98, 10, 70.0, vec![]),
                create_test_function_coverage("func_100", 100, 10, 80.0, vec![]),
                create_test_function_coverage("func_102", 102, 10, 90.0, vec![]),
            ],
        );
        let index = CoverageIndex::from_coverage(&coverage);

        // Verify range is inclusive on both ends for target line 100 with tolerance 2
        // Should match lines 98, 100, 102 (range 98..=102)

        // Test exact boundary: line 98 (min_line = 100 - 2 = 98)
        let result = index.find_function_by_line(Path::new("test.rs"), 100, 2);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().name,
            "func_100",
            "Should find closest function at 100"
        );

        // Test that all functions in range are considered
        let result = index.find_function_by_line(Path::new("test.rs"), 99, 2);
        assert!(result.is_some());
        // 99 is equidistant from 98 and 100, should pick first (min_by_key behavior)
    }
}
