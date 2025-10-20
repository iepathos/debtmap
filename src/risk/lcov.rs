use super::coverage_index::CoverageIndex;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct FunctionCoverage {
    pub name: String,
    pub start_line: usize,
    pub execution_count: u64,
    pub coverage_percentage: f64,
    pub uncovered_lines: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct LcovData {
    pub functions: HashMap<PathBuf, Vec<FunctionCoverage>>,
    pub total_lines: usize,
    pub lines_hit: usize,
    /// LOC counter instance for consistent line counting across analysis modes
    loc_counter: Option<crate::metrics::LocCounter>,
    /// Pre-built index for O(1) function coverage lookups, wrapped in Arc for lock-free sharing across threads
    coverage_index: Arc<CoverageIndex>,
}

impl Default for LcovData {
    fn default() -> Self {
        Self::new()
    }
}

impl LcovData {
    /// Create a new empty LcovData instance
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            total_lines: 0,
            lines_hit: 0,
            loc_counter: None,
            coverage_index: Arc::new(CoverageIndex::empty()),
        }
    }

    /// Build the coverage index from current function data
    /// This should be called after modifying the functions HashMap
    pub fn build_index(&mut self) {
        self.coverage_index = Arc::new(CoverageIndex::from_coverage(self));
    }
}

pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
    use lcov::{Reader, Record};

    let reader = Reader::open_file(path)
        .with_context(|| format!("Failed to open LCOV file: {}", path.display()))?;

    let mut data = LcovData::default();
    let mut current_file: Option<PathBuf> = None;
    let mut file_functions: HashMap<String, FunctionCoverage> = HashMap::new();
    let mut file_lines: HashMap<usize, u64> = HashMap::new();
    let mut _lines_found = 0;
    let mut _lines_hit = 0;

    for record in reader {
        let record = record.with_context(|| "Failed to parse LCOV record")?;

        match record {
            Record::SourceFile { path } => {
                // Save previous file's data if any
                if let Some(file) = current_file.take() {
                    if !file_functions.is_empty() {
                        let mut funcs: Vec<FunctionCoverage> =
                            file_functions.drain().map(|(_, v)| v).collect();
                        funcs.sort_by_key(|f| f.start_line);
                        data.functions.insert(file, funcs);
                    }
                }
                current_file = Some(path);
                file_functions.clear();
                file_lines.clear();
            }

            Record::FunctionName { start_line, name } => {
                file_functions
                    .entry(name.clone())
                    .or_insert_with(|| FunctionCoverage {
                        name,
                        start_line: start_line as usize,
                        execution_count: 0,
                        coverage_percentage: 0.0,
                        uncovered_lines: Vec::new(),
                    });
            }

            Record::FunctionData { name, count } => {
                if let Some(func) = file_functions.get_mut(&name) {
                    func.execution_count = count;
                    // If no line data is available, use execution count to determine coverage
                    // Functions with count > 0 are considered 100% covered, 0 means 0% covered
                    if func.coverage_percentage == 0.0 && count > 0 {
                        func.coverage_percentage = 100.0;
                    }
                }
            }

            Record::LineData { line, count, .. } => {
                file_lines.insert(line as usize, count);
                if count > 0 {
                    _lines_hit += 1;
                }
            }

            Record::LinesFound { found } => {
                _lines_found = found as usize;
                data.total_lines += found as usize;
            }

            Record::LinesHit { hit } => {
                _lines_hit = hit as usize;
                data.lines_hit += hit as usize;
            }

            Record::EndOfRecord => {
                // Use parallel processing for function coverage calculation
                process_function_coverage_parallel(&mut file_functions, &file_lines);

                // Save the file's data
                if let Some(file) = current_file.take() {
                    if !file_functions.is_empty() {
                        let mut funcs: Vec<FunctionCoverage> =
                            file_functions.drain().map(|(_, v)| v).collect();
                        funcs.sort_by_key(|f| f.start_line);
                        data.functions.insert(file, funcs);
                    }
                }

                file_functions.clear();
                file_lines.clear();
            }

            _ => {} // Ignore other record types
        }
    }

    // Handle case where file doesn't end with EndOfRecord
    if let Some(file) = current_file {
        if !file_functions.is_empty() {
            let mut funcs: Vec<FunctionCoverage> = file_functions.drain().map(|(_, v)| v).collect();
            funcs.sort_by_key(|f| f.start_line);
            data.functions.insert(file, funcs);
        }
    }

    // Build the coverage index once after all data is loaded
    data.build_index();

    Ok(data)
}

/// Parallel processing function for calculating function coverage
/// This replaces the sequential processing in the EndOfRecord handler
fn process_function_coverage_parallel(
    file_functions: &mut HashMap<String, FunctionCoverage>,
    file_lines: &HashMap<usize, u64>,
) {
    // Early return if no data to process
    if file_functions.is_empty() || file_lines.is_empty() {
        return;
    }

    // Collect and sort function start lines for boundary detection (parallel-friendly)
    let func_boundaries: Vec<usize> = file_functions
        .par_iter()
        .map(|(_, func)| func.start_line)
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    // Convert file_lines HashMap to sorted Vec for efficient range queries
    let sorted_lines: Vec<(usize, u64)> = file_lines
        .par_iter()
        .map(|(line, count)| (*line, *count))
        .collect::<Vec<_>>()
        .into_par_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    // Use Mutex for thread-safe access to the functions HashMap
    let functions_mutex = Mutex::new(file_functions);

    // Process functions in parallel
    func_boundaries.par_iter().for_each(|&func_start| {
        // Calculate function coverage for this function
        let coverage_data =
            calculate_function_coverage_data(func_start, &func_boundaries, &sorted_lines);

        // Update the function in the mutex-protected HashMap
        if let Ok(mut functions) = functions_mutex.lock() {
            if let Some(func) = functions.values_mut().find(|f| f.start_line == func_start) {
                func.coverage_percentage = coverage_data.coverage_percentage;
                func.uncovered_lines = coverage_data.uncovered_lines;
            }
        }
    });
}

/// Calculate coverage data for a single function
/// Pure function that can be called in parallel
#[derive(Debug)]
struct FunctionCoverageData {
    coverage_percentage: f64,
    uncovered_lines: Vec<usize>,
}

fn calculate_function_coverage_data(
    func_start: usize,
    func_boundaries: &[usize],
    sorted_lines: &[(usize, u64)],
) -> FunctionCoverageData {
    // Find the next function's start line using binary search
    let next_func_idx = func_boundaries
        .binary_search(&func_start)
        .unwrap_or_else(|idx| idx);

    let func_end = if next_func_idx + 1 < func_boundaries.len() {
        func_boundaries[next_func_idx + 1]
    } else {
        usize::MAX
    };

    // Binary search for function's line range in sorted_lines
    let start_idx = sorted_lines
        .binary_search_by_key(&func_start, |(line, _)| *line)
        .unwrap_or_else(|idx| idx);
    let end_idx = sorted_lines
        .binary_search_by_key(&func_end, |(line, _)| *line)
        .unwrap_or_else(|idx| idx);

    let func_lines = &sorted_lines[start_idx..end_idx];

    if !func_lines.is_empty() {
        let covered = func_lines
            .par_iter()
            .filter(|(_, count)| *count > 0)
            .count();
        let coverage_percentage = (covered as f64 / func_lines.len() as f64) * 100.0;

        // Collect uncovered lines in parallel
        let uncovered_lines = func_lines
            .par_iter()
            .filter(|(_, count)| *count == 0)
            .map(|(line, _)| *line)
            .collect();

        FunctionCoverageData {
            coverage_percentage,
            uncovered_lines,
        }
    } else {
        FunctionCoverageData {
            coverage_percentage: 0.0,
            uncovered_lines: Vec::new(),
        }
    }
}

impl LcovData {
    /// Set the LOC counter to use for consistent line counting
    pub fn with_loc_counter(mut self, loc_counter: crate::metrics::LocCounter) -> Self {
        self.loc_counter = Some(loc_counter);
        self
    }

    /// Get the LOC counter instance if set
    pub fn loc_counter(&self) -> Option<&crate::metrics::LocCounter> {
        self.loc_counter.as_ref()
    }

    /// Recalculate total lines using LOC counter for consistency
    /// This ensures coverage denominator matches the LOC count used elsewhere
    pub fn recalculate_with_loc_counter(&mut self) {
        if let Some(counter) = &self.loc_counter {
            let files: Vec<PathBuf> = self.functions.keys().cloned().collect();
            let mut total_code_lines = 0;

            for file in &files {
                if counter.should_include(file) {
                    if let Ok(count) = counter.count_file(file) {
                        total_code_lines += count.code_lines;
                        log::debug!(
                            "LOC counter: {} has {} code lines",
                            file.display(),
                            count.code_lines
                        );
                    }
                }
            }

            log::debug!(
                "Recalculated total_lines using LocCounter: {} (was {})",
                total_code_lines,
                self.total_lines
            );
            self.total_lines = total_code_lines;
        }
    }

    /// Get function coverage using O(1) indexed lookup
    ///
    /// This method uses the pre-built coverage index for fast lookups,
    /// avoiding the O(n) linear search through function arrays.
    pub fn get_function_coverage(&self, file: &Path, function_name: &str) -> Option<f64> {
        self.coverage_index
            .get_function_coverage(file, function_name)
    }

    /// Get function coverage with line number fallback using O(log n) indexed lookup
    ///
    /// Tries exact function name match first (O(1)), then falls back to
    /// line-based lookup (O(log n)) if needed.
    pub fn get_function_coverage_with_line(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<f64> {
        self.coverage_index
            .get_function_coverage_with_line(file, function_name, line)
    }

    /// Get function coverage using exact boundaries from AST analysis
    /// This is more accurate than guessing boundaries from LCOV data alone
    pub fn get_function_coverage_with_bounds(
        &self,
        file: &Path,
        function_name: &str,
        start_line: usize,
        _end_line: usize,
    ) -> Option<f64> {
        // Use the same logic as get_function_coverage_with_line
        self.coverage_index
            .get_function_coverage_with_line(file, function_name, start_line)
    }

    pub fn get_overall_coverage(&self) -> f64 {
        if self.total_lines == 0 {
            0.0
        } else {
            (self.lines_hit as f64 / self.total_lines as f64) * 100.0
        }
    }

    pub fn get_file_coverage(&self, file: &Path) -> Option<f64> {
        self.functions.get(file).map(|funcs| {
            if funcs.is_empty() {
                0.0
            } else {
                // Use parallel processing for coverage calculation
                let sum: f64 = funcs.par_iter().map(|f| f.coverage_percentage).sum();
                sum / funcs.len() as f64 / 100.0 // Convert to fraction
            }
        })
    }

    /// Get uncovered lines for a specific function
    /// Get uncovered lines for a function using O(1) indexed lookup
    pub fn get_function_uncovered_lines(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<Vec<usize>> {
        self.coverage_index
            .get_function_uncovered_lines(file, function_name, line)
    }

    /// Batch process coverage queries for multiple functions in parallel
    /// This is more efficient when querying coverage for many functions at once
    pub fn batch_get_function_coverage(
        &self,
        queries: &[(PathBuf, String, usize)], // (file, function_name, line)
    ) -> Vec<Option<f64>> {
        queries
            .par_iter()
            .map(|(file, function_name, line)| {
                self.get_function_coverage_with_line(file, function_name, *line)
            })
            .collect()
    }

    /// Get coverage statistics for all files in parallel
    pub fn get_all_file_coverages(&self) -> HashMap<PathBuf, f64> {
        self.functions
            .par_iter()
            .map(|(path, funcs)| {
                let coverage = if funcs.is_empty() {
                    0.0
                } else {
                    let sum: f64 = funcs.par_iter().map(|f| f.coverage_percentage).sum();
                    sum / funcs.len() as f64 / 100.0 // Convert to fraction
                };
                (path.clone(), coverage)
            })
            .collect()
    }
}

/// Find a function by name using multiple matching strategies
/// Returns the first match found using these strategies in order:
/// 1. Exact match
/// 2. Normalized match (handling generics and special characters)
/// 3. Suffix match (for module-qualified names)
#[allow(dead_code)]
fn find_function_by_name<'a>(
    funcs: &'a [FunctionCoverage],
    target_name: &str,
) -> Option<&'a FunctionCoverage> {
    // For small function lists, use sequential search to avoid parallel overhead
    if funcs.len() < 10 {
        // Strategy 1: Exact match
        funcs
            .iter()
            .find(|f| f.name == target_name)
            // Strategy 2: Normalized match
            .or_else(|| {
                let normalized_target = normalize_function_name(target_name);
                funcs
                    .iter()
                    .find(|f| normalize_function_name(&f.name) == normalized_target)
            })
            // Strategy 3: Suffix match (e.g., "module::function" matches "function")
            .or_else(|| {
                funcs
                    .iter()
                    .find(|f| f.name.ends_with(target_name) || target_name.ends_with(&f.name))
            })
    } else {
        // For larger function lists, use parallel search
        // Strategy 1: Exact match (parallel)
        funcs
            .par_iter()
            .find_any(|f| f.name == target_name)
            // Strategy 2: Normalized match (parallel)
            .or_else(|| {
                let normalized_target = normalize_function_name(target_name);
                funcs
                    .par_iter()
                    .find_any(|f| normalize_function_name(&f.name) == normalized_target)
            })
            // Strategy 3: Suffix match (parallel)
            .or_else(|| {
                funcs
                    .par_iter()
                    .find_any(|f| f.name.ends_with(target_name) || target_name.ends_with(&f.name))
            })
    }
}

/// Find a function by line number with tolerance for AST/LCOV discrepancies
/// Pure function that searches for functions within a line range
#[allow(dead_code)]
fn find_function_by_line_with_tolerance(
    funcs: &[FunctionCoverage],
    target_line: usize,
    tolerance: usize,
) -> Option<&FunctionCoverage> {
    funcs
        .iter()
        .filter(|f| {
            let line_diff = (f.start_line as i32 - target_line as i32).abs();
            line_diff <= tolerance as i32
        })
        // If multiple matches within tolerance, prefer the closest one
        .min_by_key(|f| (f.start_line as i32 - target_line as i32).abs())
}

/// Normalize function names for matching
/// This is now a pure function used internally by the matching strategies
#[allow(dead_code)]
fn normalize_function_name(name: &str) -> String {
    // Handle Rust function names with generics and impl blocks
    name.replace(['<', '>'], "_")
        .replace("::", "_")
        .replace(' ', "_")
        .replace('\'', "")
}

/// Strategy 1: Check if query path ends with LCOV path
/// Example: query="/home/user/project/src/main.rs" matches lcov="src/main.rs"
#[allow(dead_code)]
fn matches_suffix_strategy(query_path: &Path, lcov_path: &Path) -> bool {
    query_path.ends_with(lcov_path)
}

/// Strategy 2: Check if LCOV path ends with normalized query path
/// Example: lcov="/home/user/project/src/main.rs" matches query="./src/main.rs"
#[allow(dead_code)]
fn matches_reverse_suffix_strategy(query_path: &Path, lcov_path: &Path) -> bool {
    let normalized_query = normalize_path(query_path);
    lcov_path.ends_with(&normalized_query)
}

/// Strategy 3: Check if normalized paths are equal
/// Example: lcov="./src/main.rs" matches query="src/main.rs"
#[allow(dead_code)]
fn matches_normalized_equality_strategy(query_path: &Path, lcov_path: &Path) -> bool {
    normalize_path(lcov_path) == normalize_path(query_path)
}

/// Find functions by normalizing and matching paths
/// This handles cases where LCOV has relative paths but queries use absolute paths, or vice versa
#[allow(dead_code)]
fn find_functions_by_path<'a>(
    functions: &'a HashMap<PathBuf, Vec<FunctionCoverage>>,
    query_path: &Path,
) -> Option<&'a Vec<FunctionCoverage>> {
    // Use parallel search for large function maps
    if functions.len() > 20 {
        functions
            .par_iter()
            .find_any(|(lcov_path, _)| matches_suffix_strategy(query_path, lcov_path))
            .map(|(_, funcs)| funcs)
            .or_else(|| {
                functions
                    .par_iter()
                    .find_any(|(lcov_path, _)| matches_reverse_suffix_strategy(query_path, lcov_path))
                    .map(|(_, funcs)| funcs)
            })
            .or_else(|| {
                functions
                    .par_iter()
                    .find_any(|(lcov_path, _)| matches_normalized_equality_strategy(query_path, lcov_path))
                    .map(|(_, funcs)| funcs)
            })
    } else {
        // Use sequential search for smaller maps to avoid parallel overhead
        functions
            .iter()
            .find(|(lcov_path, _)| matches_suffix_strategy(query_path, lcov_path))
            .map(|(_, funcs)| funcs)
            .or_else(|| {
                functions
                    .iter()
                    .find(|(lcov_path, _)| matches_reverse_suffix_strategy(query_path, lcov_path))
                    .map(|(_, funcs)| funcs)
            })
            .or_else(|| {
                functions
                    .iter()
                    .find(|(lcov_path, _)| matches_normalized_equality_strategy(query_path, lcov_path))
                    .map(|(_, funcs)| funcs)
            })
    }
}

/// Normalize a path by removing leading ./ and resolving components
#[allow(dead_code)]
fn normalize_path(path: &Path) -> PathBuf {
    // Convert to string, remove leading ./
    let path_str = path.to_string_lossy();
    let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);
    PathBuf::from(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_lcov_file() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,test_function
FNDA:5,test_function
FNF:1
FNH:1
DA:10,5
DA:11,5
DA:12,0
LF:3
LH:2
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        assert_eq!(data.total_lines, 3);
        assert_eq!(data.lines_hit, 2);

        let file_path = PathBuf::from("/path/to/file.rs");
        assert!(data.functions.contains_key(&file_path));

        let funcs = &data.functions[&file_path];
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "test_function");
        assert_eq!(funcs[0].execution_count, 5);
    }

    #[test]
    fn test_function_name_normalization() {
        assert_eq!(
            normalize_function_name("MyStruct::my_function"),
            normalize_function_name("MyStruct_my_function")
        );

        assert_eq!(
            normalize_function_name("function<T>"),
            normalize_function_name("function_T_")
        );
    }

    #[test]
    fn test_coverage_percentage_calculation() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,fully_covered
FN:20,partially_covered
FN:30,not_covered
FNDA:10,fully_covered
FNDA:5,partially_covered
FNDA:0,not_covered
DA:10,10
DA:11,10
DA:12,10
DA:20,5
DA:21,5
DA:22,0
DA:23,0
DA:30,0
DA:31,0
DA:32,0
LF:10
LH:5
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/file.rs");

        // Test fully covered function (100%)
        let coverage = data.get_function_coverage(&file_path, "fully_covered");
        assert_eq!(coverage, Some(1.0));

        // Test partially covered function (50%)
        let coverage = data.get_function_coverage(&file_path, "partially_covered");
        assert_eq!(coverage, Some(0.5));

        // Test uncovered function (0%)
        let coverage = data.get_function_coverage(&file_path, "not_covered");
        assert_eq!(coverage, Some(0.0));
    }

    #[test]
    fn test_get_function_coverage_with_line() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,function_at_line_10
FN:50,function_at_line_50
FNDA:5,function_at_line_10
FNDA:0,function_at_line_50
DA:10,5
DA:11,5
DA:50,0
DA:51,0
LF:4
LH:2
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/file.rs");

        // Test finding function by line number
        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 10);
        assert_eq!(coverage, Some(1.0)); // Should find function_at_line_10

        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 51);
        assert_eq!(coverage, Some(0.0)); // Should find function_at_line_50

        // Test line outside any function range
        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 200);
        assert_eq!(coverage, None);
    }

    #[test]
    fn test_overall_coverage_calculation() {
        let lcov_content = r#"TN:
SF:/path/to/file1.rs
DA:1,1
DA:2,1
DA:3,0
DA:4,0
LF:4
LH:2
end_of_record
SF:/path/to/file2.rs
DA:1,5
DA:2,5
DA:3,5
DA:4,5
LF:4
LH:4
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        // Overall coverage: 6 lines hit out of 8 total = 75%
        assert_eq!(data.get_overall_coverage(), 75.0);
    }

    #[test]
    fn test_empty_coverage_data() {
        let lcov_content = r#"TN:
SF:/path/to/empty.rs
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        assert_eq!(data.get_overall_coverage(), 0.0);
        assert_eq!(data.functions.len(), 0);
    }

    // Tests for find_functions_by_path
    mod find_functions_by_path_tests {
        use super::*;

        fn create_test_function_coverage(name: &str) -> Vec<FunctionCoverage> {
            vec![FunctionCoverage {
                name: name.to_string(),
                start_line: 10,
                execution_count: 5,
                coverage_percentage: 60.0,
                uncovered_lines: vec![12, 13],
            }]
        }

        #[test]
        fn test_strategy_1_suffix_matching_sequential() {
            // Test Strategy 1: query_path.ends_with(lcov_path)
            // Sequential path (<=20 items)
            let mut functions = HashMap::new();
            functions.insert(
                PathBuf::from("src/main.rs"),
                create_test_function_coverage("main"),
            );

            let query = PathBuf::from("/home/user/project/src/main.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "main");
        }

        #[test]
        fn test_strategy_2_reverse_suffix_matching_sequential() {
            // Test Strategy 2: lcov_path.ends_with(normalized_query)
            // Sequential path (<=20 items)
            let mut functions = HashMap::new();
            functions.insert(
                PathBuf::from("/home/user/project/src/lib.rs"),
                create_test_function_coverage("lib_func"),
            );

            let query = PathBuf::from("./src/lib.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "lib_func");
        }

        #[test]
        fn test_strategy_3_normalized_equality_sequential() {
            // Test Strategy 3: normalize_path(lcov_path) == normalize_path(query_path)
            // Sequential path (<=20 items)
            let mut functions = HashMap::new();
            functions.insert(
                PathBuf::from("./src/utils.rs"),
                create_test_function_coverage("util_func"),
            );

            let query = PathBuf::from("src/utils.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "util_func");
        }

        #[test]
        fn test_strategy_1_suffix_matching_parallel() {
            // Test Strategy 1 with >20 items (parallel path)
            let mut functions = HashMap::new();

            // Add 21 items to trigger parallel path
            for i in 0..21 {
                functions.insert(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    create_test_function_coverage(&format!("func_{}", i)),
                );
            }

            // Add our target
            functions.insert(
                PathBuf::from("src/target.rs"),
                create_test_function_coverage("target_func"),
            );

            let query = PathBuf::from("/home/user/project/src/target.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "target_func");
        }

        #[test]
        fn test_strategy_2_reverse_suffix_matching_parallel() {
            // Test Strategy 2 with >20 items (parallel path)
            let mut functions = HashMap::new();

            // Add 21 items to trigger parallel path
            for i in 0..21 {
                functions.insert(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    create_test_function_coverage(&format!("func_{}", i)),
                );
            }

            // Add our target with absolute path
            functions.insert(
                PathBuf::from("/home/user/project/src/target.rs"),
                create_test_function_coverage("target_func"),
            );

            // Query with relative path
            let query = PathBuf::from("./src/target.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "target_func");
        }

        #[test]
        fn test_strategy_3_normalized_equality_parallel() {
            // Test Strategy 3 with >20 items (parallel path)
            let mut functions = HashMap::new();

            // Add 21 items to trigger parallel path
            for i in 0..21 {
                functions.insert(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    create_test_function_coverage(&format!("func_{}", i)),
                );
            }

            // Add our target with ./ prefix
            functions.insert(
                PathBuf::from("./src/target.rs"),
                create_test_function_coverage("target_func"),
            );

            // Query without ./ prefix
            let query = PathBuf::from("src/target.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "target_func");
        }

        #[test]
        fn test_no_match_found() {
            // Test when no strategy matches
            let mut functions = HashMap::new();
            functions.insert(
                PathBuf::from("src/main.rs"),
                create_test_function_coverage("main"),
            );

            let query = PathBuf::from("src/different.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_none());
        }

        #[test]
        fn test_empty_map() {
            // Test with empty function map
            let functions = HashMap::new();
            let query = PathBuf::from("src/main.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_none());
        }

        #[test]
        fn test_single_item_map() {
            // Test edge case with exactly 1 item (sequential path)
            let mut functions = HashMap::new();
            functions.insert(
                PathBuf::from("src/single.rs"),
                create_test_function_coverage("single_func"),
            );

            let query = PathBuf::from("./src/single.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "single_func");
        }

        #[test]
        fn test_exactly_20_items() {
            // Test boundary case with exactly 20 items (should use sequential)
            let mut functions = HashMap::new();

            for i in 0..20 {
                functions.insert(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    create_test_function_coverage(&format!("func_{}", i)),
                );
            }

            let query = PathBuf::from("./src/file_10.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "func_10");
        }

        #[test]
        fn test_exactly_21_items() {
            // Test boundary case with exactly 21 items (should use parallel)
            let mut functions = HashMap::new();

            for i in 0..21 {
                functions.insert(
                    PathBuf::from(format!("src/file_{}.rs", i)),
                    create_test_function_coverage(&format!("func_{}", i)),
                );
            }

            let query = PathBuf::from("./src/file_10.rs");
            let result = find_functions_by_path(&functions, &query);

            assert!(result.is_some());
            assert_eq!(result.unwrap()[0].name, "func_10");
        }

        #[test]
        fn test_normalize_path_idempotency() {
            // Test that normalize_path is idempotent
            let path1 = PathBuf::from("./src/main.rs");
            let normalized1 = normalize_path(&path1);
            let normalized2 = normalize_path(&normalized1);

            assert_eq!(normalized1, normalized2);
        }

        #[test]
        fn test_normalize_path_removes_leading_dot_slash() {
            // Test that normalize_path removes leading ./
            let path = PathBuf::from("./src/main.rs");
            let normalized = normalize_path(&path);

            assert_eq!(normalized, PathBuf::from("src/main.rs"));
        }

        #[test]
        fn test_normalize_path_no_leading_dot_slash() {
            // Test that normalize_path doesn't affect paths without leading ./
            let path = PathBuf::from("src/main.rs");
            let normalized = normalize_path(&path);

            assert_eq!(normalized, PathBuf::from("src/main.rs"));
        }
    }

    // Tests for extracted strategy functions
    mod strategy_tests {
        use super::*;

        #[test]
        fn test_matches_suffix_strategy_basic() {
            let query = PathBuf::from("/home/user/project/src/main.rs");
            let lcov = PathBuf::from("src/main.rs");

            assert!(matches_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_suffix_strategy_no_match() {
            let query = PathBuf::from("/home/user/project/src/main.rs");
            let lcov = PathBuf::from("src/lib.rs");

            assert!(!matches_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_suffix_strategy_exact_match() {
            let query = PathBuf::from("src/main.rs");
            let lcov = PathBuf::from("src/main.rs");

            assert!(matches_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_reverse_suffix_strategy_basic() {
            let query = PathBuf::from("./src/lib.rs");
            let lcov = PathBuf::from("/home/user/project/src/lib.rs");

            assert!(matches_reverse_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_reverse_suffix_strategy_no_match() {
            let query = PathBuf::from("./src/main.rs");
            let lcov = PathBuf::from("/home/user/project/src/lib.rs");

            assert!(!matches_reverse_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_reverse_suffix_strategy_with_normalization() {
            let query = PathBuf::from("./src/utils.rs");
            let lcov = PathBuf::from("/home/user/src/utils.rs");

            assert!(matches_reverse_suffix_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_normalized_equality_strategy_basic() {
            let query = PathBuf::from("src/main.rs");
            let lcov = PathBuf::from("./src/main.rs");

            assert!(matches_normalized_equality_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_normalized_equality_strategy_both_normalized() {
            let query = PathBuf::from("./src/main.rs");
            let lcov = PathBuf::from("./src/main.rs");

            assert!(matches_normalized_equality_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_normalized_equality_strategy_no_match() {
            let query = PathBuf::from("src/main.rs");
            let lcov = PathBuf::from("src/lib.rs");

            assert!(!matches_normalized_equality_strategy(&query, &lcov));
        }

        #[test]
        fn test_matches_normalized_equality_strategy_different_depths() {
            let query = PathBuf::from("./src/main.rs");
            let lcov = PathBuf::from("./lib/main.rs");

            assert!(!matches_normalized_equality_strategy(&query, &lcov));
        }
    }
}
