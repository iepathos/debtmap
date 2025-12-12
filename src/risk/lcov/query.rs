//! Query methods for LcovData.
//!
//! This module extends `LcovData` with methods for querying coverage information.
//! It provides various strategies for looking up function coverage, including
//! indexed lookups, line-based fallbacks, and batch processing.
//!
//! # Stillwater Philosophy
//!
//! Query methods are mostly pure (read-only operations). The exception is
//! diagnostic tracking which is at the system boundary for debug purposes.
//!
//! # Lookup Strategies
//!
//! Coverage lookups use multiple strategies for flexibility:
//! 1. O(1) indexed lookup by function name
//! 2. O(log n) line-based fallback
//! 3. Path matching with normalization
//! 4. Function name matching with normalization
//!
//! # Example
//!
//! ```ignore
//! use std::path::Path;
//! use debtmap::risk::lcov::parse_lcov_file;
//!
//! let data = parse_lcov_file(Path::new("coverage.info"))?;
//!
//! // Simple lookup
//! let coverage = data.get_function_coverage(
//!     Path::new("src/lib.rs"),
//!     "my_function",
//! );
//!
//! // Lookup with line fallback
//! let coverage = data.get_function_coverage_with_line(
//!     Path::new("src/lib.rs"),
//!     "my_function",
//!     42,
//! );
//!
//! // Batch lookup for many functions
//! let queries = vec![
//!     (PathBuf::from("src/a.rs"), "func_a".to_string(), 10),
//!     (PathBuf::from("src/b.rs"), "func_b".to_string(), 20),
//! ];
//! let results = data.batch_get_function_coverage(&queries);
//! ```

use super::diagnostics::{track_match_attempt, track_match_success, track_match_zero};
use super::types::{FunctionCoverage, LcovData};
use crate::risk::function_name_matching::{find_matching_function, MatchableFunction};
use crate::risk::path_normalization::find_matching_path;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

impl LcovData {
    /// Set the LOC counter to use for consistent line counting.
    ///
    /// # Arguments
    ///
    /// * `loc_counter` - The LOC counter instance to use
    ///
    /// # Returns
    ///
    /// Self with the LOC counter set.
    pub fn with_loc_counter(mut self, loc_counter: crate::metrics::LocCounter) -> Self {
        self.loc_counter = Some(loc_counter);
        self
    }

    /// Get the LOC counter instance if set.
    pub fn loc_counter(&self) -> Option<&crate::metrics::LocCounter> {
        self.loc_counter.as_ref()
    }

    /// Recalculate total lines using LOC counter for consistency.
    ///
    /// This ensures coverage denominator matches the LOC count used elsewhere.
    /// Useful when you want the coverage percentage to align with LOC-based metrics.
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

    /// Get function coverage using O(1) indexed lookup.
    ///
    /// This method uses the pre-built coverage index for fast lookups,
    /// avoiding the O(n) linear search through function arrays.
    ///
    /// # Arguments
    ///
    /// * `file` - Path to the source file
    /// * `function_name` - Name of the function
    ///
    /// # Returns
    ///
    /// Coverage as a fraction (0.0 to 1.0), or None if not found.
    pub fn get_function_coverage(&self, file: &Path, function_name: &str) -> Option<f64> {
        self.coverage_index
            .get_function_coverage(file, function_name)
    }

    /// Get function coverage with line number fallback using O(log n) indexed lookup.
    ///
    /// Tries exact function name match first (O(1)), then falls back to
    /// line-based lookup (O(log n)) if needed.
    ///
    /// # Arguments
    ///
    /// * `file` - Path to the source file
    /// * `function_name` - Name of the function
    /// * `line` - Line number for fallback lookup
    ///
    /// # Returns
    ///
    /// Coverage as a fraction (0.0 to 1.0), or None if not found.
    pub fn get_function_coverage_with_line(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<f64> {
        self.coverage_index
            .get_function_coverage_with_line(file, function_name, line)
    }

    /// Get function coverage using exact boundaries from AST analysis.
    ///
    /// This is more accurate than guessing boundaries from LCOV data alone.
    /// Uses path normalization (Spec 201) and function name matching (Spec 202)
    /// to find the correct function even when names don't match exactly.
    ///
    /// **Integration of Specs 201, 202, and 203:**
    /// - Uses path normalization (Spec 201) to find matching files
    /// - Uses function name matching (Spec 202) to find matching functions
    /// - Returns 0.0 instead of None when LCOV provided but function not found (Spec 203)
    /// - Logs diagnostics when DEBTMAP_COVERAGE_DEBUG is set (Spec 203)
    ///
    /// # Arguments
    ///
    /// * `file` - Path to the source file
    /// * `function_name` - Name of the function
    /// * `_start_line` - Start line (reserved for future use)
    /// * `_end_line` - End line (reserved for future use)
    ///
    /// # Returns
    ///
    /// Coverage as a fraction (0.0 to 1.0), or None if file not in LCOV data.
    /// Returns Some(0.0) if file is in LCOV but function not found.
    pub fn get_function_coverage_with_bounds(
        &self,
        file: &Path,
        function_name: &str,
        _start_line: usize,
        _end_line: usize,
    ) -> Option<f64> {
        let debug_mode = std::env::var("DEBTMAP_COVERAGE_DEBUG").is_ok();

        // Track statistics for diagnostic mode (Spec 203 FR3)
        if debug_mode {
            track_match_attempt();
        }

        // Phase 1: Path matching using Spec 201
        let available_paths: Vec<PathBuf> = self.functions.keys().cloned().collect();
        let path_match = find_matching_path(file, &available_paths);

        if debug_mode {
            if let Some((_matched_path, strategy)) = &path_match {
                eprintln!(
                    "[COVERAGE] {}::{} Path:✓ Strategy:{:?}",
                    file.display(),
                    function_name,
                    strategy
                );
            } else {
                eprintln!(
                    "[COVERAGE] {}::{} Path:✗ (not found in {} paths)",
                    file.display(),
                    function_name,
                    available_paths.len()
                );
                track_match_zero();
                return Some(0.0); // Return 0% not None when LCOV provided
            }
        }

        let (matched_path, _path_strategy) = path_match?;

        // Phase 2: Function matching using Spec 202
        let functions = self.functions.get(matched_path)?;

        // Convert to MatchableFunction for the matching algorithm
        let matchable_funcs: Vec<MatchableFunction<&FunctionCoverage>> = functions
            .iter()
            .map(|f| MatchableFunction {
                name: f.name.clone(),
                data: f,
            })
            .collect();

        let func_match = find_matching_function(function_name, &matchable_funcs);

        if debug_mode {
            if let Some((matched_func, confidence)) = &func_match {
                eprintln!(
                    "[COVERAGE]   Func:✓ Confidence:{:?} Coverage:{:.1}%",
                    confidence, matched_func.data.coverage_percentage
                );
            } else {
                eprintln!(
                    "[COVERAGE]   Func:✗ (not found in {} functions)",
                    functions.len()
                );
                track_match_zero();
                return Some(0.0); // Return 0% not None when LCOV provided
            }
        }

        let (matched_func, _confidence) = func_match?;
        let coverage = matched_func.data.coverage_percentage / 100.0;

        // Track successful match in debug mode (Spec 203 FR3)
        if debug_mode && coverage > 0.0 {
            track_match_success();
        } else if debug_mode {
            track_match_zero();
        }

        Some(coverage)
    }

    /// Get overall coverage percentage.
    ///
    /// # Returns
    ///
    /// Overall coverage as a percentage (0.0 to 100.0).
    pub fn get_overall_coverage(&self) -> f64 {
        if self.total_lines == 0 {
            0.0
        } else {
            (self.lines_hit as f64 / self.total_lines as f64) * 100.0
        }
    }

    /// Get coverage percentage for a specific file.
    ///
    /// # Arguments
    ///
    /// * `file` - Path to the source file
    ///
    /// # Returns
    ///
    /// Coverage as a fraction (0.0 to 1.0), or None if file not found.
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

    /// Get uncovered lines for a function using O(1) indexed lookup.
    ///
    /// # Arguments
    ///
    /// * `file` - Path to the source file
    /// * `function_name` - Name of the function
    /// * `line` - Line number for fallback lookup
    ///
    /// # Returns
    ///
    /// List of uncovered line numbers, or None if function not found.
    pub fn get_function_uncovered_lines(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<Vec<usize>> {
        self.coverage_index
            .get_function_uncovered_lines(file, function_name, line)
    }

    /// Batch process coverage queries for multiple functions in parallel.
    ///
    /// This is more efficient when querying coverage for many functions at once.
    ///
    /// # Arguments
    ///
    /// * `queries` - List of (file, function_name, line) tuples
    ///
    /// # Returns
    ///
    /// Vector of coverage values in the same order as queries.
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

    /// Get coverage statistics for all files in parallel.
    ///
    /// # Returns
    ///
    /// HashMap mapping file paths to their coverage (as fractions 0.0 to 1.0).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::lcov::types::NormalizedFunctionName;
    use std::sync::Arc;

    fn create_test_lcov_data() -> LcovData {
        let mut functions = HashMap::new();

        functions.insert(
            PathBuf::from("/path/to/file.rs"),
            vec![
                FunctionCoverage {
                    name: "fully_covered".to_string(),
                    start_line: 10,
                    execution_count: 10,
                    coverage_percentage: 100.0,
                    uncovered_lines: vec![],
                    normalized: NormalizedFunctionName::simple("fully_covered"),
                },
                FunctionCoverage {
                    name: "partially_covered".to_string(),
                    start_line: 20,
                    execution_count: 5,
                    coverage_percentage: 50.0,
                    uncovered_lines: vec![22, 23],
                    normalized: NormalizedFunctionName::simple("partially_covered"),
                },
                FunctionCoverage {
                    name: "not_covered".to_string(),
                    start_line: 30,
                    execution_count: 0,
                    coverage_percentage: 0.0,
                    uncovered_lines: vec![30, 31, 32],
                    normalized: NormalizedFunctionName::simple("not_covered"),
                },
            ],
        );

        let mut data = LcovData {
            functions,
            total_lines: 10,
            lines_hit: 5,
            loc_counter: None,
            coverage_index: Arc::new(crate::risk::coverage_index::CoverageIndex::empty()),
        };

        data.build_index();
        data
    }

    #[test]
    fn test_get_function_coverage() {
        let data = create_test_lcov_data();
        let file_path = PathBuf::from("/path/to/file.rs");

        let coverage = data.get_function_coverage(&file_path, "fully_covered");
        assert_eq!(coverage, Some(1.0));

        let coverage = data.get_function_coverage(&file_path, "partially_covered");
        assert_eq!(coverage, Some(0.5));

        let coverage = data.get_function_coverage(&file_path, "not_covered");
        assert_eq!(coverage, Some(0.0));

        let coverage = data.get_function_coverage(&file_path, "nonexistent");
        assert_eq!(coverage, None);
    }

    #[test]
    fn test_get_function_coverage_with_line() {
        let data = create_test_lcov_data();
        let file_path = PathBuf::from("/path/to/file.rs");

        // Should find function by line number
        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 10);
        assert_eq!(coverage, Some(1.0));

        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 21);
        assert_eq!(coverage, Some(0.5));
    }

    #[test]
    fn test_get_overall_coverage() {
        let data = create_test_lcov_data();
        assert_eq!(data.get_overall_coverage(), 50.0);
    }

    #[test]
    fn test_get_overall_coverage_empty() {
        let data = LcovData::new();
        assert_eq!(data.get_overall_coverage(), 0.0);
    }

    #[test]
    fn test_get_file_coverage() {
        let data = create_test_lcov_data();
        let file_path = PathBuf::from("/path/to/file.rs");

        let coverage = data.get_file_coverage(&file_path);
        assert!(coverage.is_some());
        // Average of 100%, 50%, 0% = 50%
        assert!((coverage.unwrap() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_get_file_coverage_nonexistent() {
        let data = create_test_lcov_data();
        let file_path = PathBuf::from("/nonexistent/file.rs");

        let coverage = data.get_file_coverage(&file_path);
        assert!(coverage.is_none());
    }

    #[test]
    fn test_batch_get_function_coverage() {
        let data = create_test_lcov_data();

        let queries = vec![
            (
                PathBuf::from("/path/to/file.rs"),
                "fully_covered".to_string(),
                10,
            ),
            (
                PathBuf::from("/path/to/file.rs"),
                "not_covered".to_string(),
                30,
            ),
            (PathBuf::from("/nonexistent/file.rs"), "func".to_string(), 1),
        ];

        let results = data.batch_get_function_coverage(&queries);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Some(1.0));
        assert_eq!(results[1], Some(0.0));
        assert_eq!(results[2], None);
    }

    #[test]
    fn test_get_all_file_coverages() {
        let data = create_test_lcov_data();

        let coverages = data.get_all_file_coverages();

        assert_eq!(coverages.len(), 1);
        let file_coverage = coverages.get(&PathBuf::from("/path/to/file.rs")).unwrap();
        assert!((file_coverage - 0.5).abs() < 0.01);
    }
}
