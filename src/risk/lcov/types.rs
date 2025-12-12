//! Core data types for LCOV coverage data.
//!
//! This module contains the foundational data structures used throughout the LCOV
//! parsing and querying system. All types are pure data structures with no I/O
//! operations or side effects.
//!
//! # Stillwater Philosophy
//!
//! This module represents the "still water" - immutable data definitions that
//! flow through the system. No dependencies on other lcov modules ensures
//! this can be the foundation layer.
//!
//! # Types
//!
//! - [`CoverageProgress`] - Progress state during LCOV file parsing
//! - [`NormalizedFunctionName`] - Normalized function name with matching variants
//! - [`FunctionCoverage`] - Coverage data for a single function
//! - [`LcovData`] - Parsed LCOV coverage data container

use super::super::coverage_index::CoverageIndex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Progress state during LCOV file parsing.
///
/// Used to report progress to callers during potentially long-running
/// LCOV file parsing operations.
///
/// # Example
///
/// ```ignore
/// use debtmap::risk::lcov::CoverageProgress;
///
/// fn report_progress(progress: CoverageProgress) {
///     match progress {
///         CoverageProgress::Initializing => println!("Opening file..."),
///         CoverageProgress::Parsing { current, total } => {
///             println!("Processing file {} of {}", current, total);
///         }
///         CoverageProgress::ComputingStats => println!("Computing statistics..."),
///         CoverageProgress::Complete => println!("Done!"),
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub enum CoverageProgress {
    /// Opening and initializing LCOV file reader
    Initializing,
    /// Parsing coverage records with file count progress
    /// (current_file, total_files_seen_so_far)
    Parsing { current: usize, total: usize },
    /// Computing final coverage statistics
    ComputingStats,
    /// Parsing complete
    Complete,
}

/// Normalized function name with multiple matching variants.
///
/// When matching function names between AST-derived names and LCOV coverage data,
/// various transformations may be needed. This struct captures both the normalized
/// form and extracted components to enable flexible matching strategies.
///
/// # Fields
///
/// * `full_path` - Full normalized path like "module::Struct::method"
/// * `method_name` - Just the method name like "method"
/// * `original` - Original demangled name for debugging
///
/// # Example
///
/// ```ignore
/// use debtmap::risk::lcov::NormalizedFunctionName;
///
/// let name = NormalizedFunctionName::simple("my_function");
/// assert_eq!(name.method_name, "my_function");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedFunctionName {
    /// Full normalized path: "module::Struct::method"
    pub full_path: String,

    /// Just the method name: "method"
    pub method_name: String,

    /// Original demangled name (for debugging)
    pub original: String,
}

impl NormalizedFunctionName {
    /// Create a simple NormalizedFunctionName for testing.
    ///
    /// Sets all fields to the same value for simple cases where
    /// no normalization is needed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let name = NormalizedFunctionName::simple("test_func");
    /// assert_eq!(name.full_path, "test_func");
    /// assert_eq!(name.method_name, "test_func");
    /// assert_eq!(name.original, "test_func");
    /// ```
    pub fn simple(name: &str) -> Self {
        Self {
            full_path: name.to_string(),
            method_name: name.to_string(),
            original: name.to_string(),
        }
    }
}

/// Coverage data for a single function.
///
/// Contains all coverage-related information for a function extracted from
/// LCOV data, including execution counts and line-level coverage details.
///
/// # Fields
///
/// * `name` - Function name (normalized form)
/// * `start_line` - Line number where function starts
/// * `execution_count` - Number of times function was executed
/// * `coverage_percentage` - Percentage of lines covered (0.0 to 100.0)
/// * `uncovered_lines` - List of line numbers that weren't covered
/// * `normalized` - Normalized name variants for matching
#[derive(Debug, Clone)]
pub struct FunctionCoverage {
    /// Function name (normalized form)
    pub name: String,
    /// Line number where function starts
    pub start_line: usize,
    /// Number of times function was executed
    pub execution_count: u64,
    /// Percentage of lines covered (0.0 to 100.0)
    pub coverage_percentage: f64,
    /// List of line numbers that weren't covered
    pub uncovered_lines: Vec<usize>,
    /// Normalized name variants for matching (not serialized)
    pub normalized: NormalizedFunctionName,
}

/// Parsed LCOV coverage data.
///
/// Contains all coverage information extracted from an LCOV file, organized
/// by file path for efficient lookup. Includes a pre-built index for O(1)
/// function coverage lookups.
///
/// # Thread Safety
///
/// The `coverage_index` is wrapped in `Arc` for lock-free sharing across
/// threads during parallel analysis operations.
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use debtmap::risk::lcov::{parse_lcov_file, LcovData};
///
/// let data = parse_lcov_file(Path::new("coverage.info"))?;
/// println!("Total lines: {}", data.total_lines);
/// println!("Lines hit: {}", data.lines_hit);
/// println!("Coverage: {:.1}%", data.get_overall_coverage());
/// ```
#[derive(Debug, Clone)]
pub struct LcovData {
    /// Map of file paths to their function coverage data
    pub functions: HashMap<PathBuf, Vec<FunctionCoverage>>,
    /// Total number of executable lines across all files
    pub total_lines: usize,
    /// Number of lines that were executed
    pub lines_hit: usize,
    /// LOC counter instance for consistent line counting across analysis modes
    pub(crate) loc_counter: Option<crate::metrics::LocCounter>,
    /// Pre-built index for O(1) function coverage lookups,
    /// wrapped in Arc for lock-free sharing across threads
    pub(crate) coverage_index: Arc<CoverageIndex>,
}

impl Default for LcovData {
    fn default() -> Self {
        Self::new()
    }
}

impl LcovData {
    /// Create a new empty LcovData instance.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let data = LcovData::new();
    /// assert_eq!(data.total_lines, 0);
    /// assert!(data.functions.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            total_lines: 0,
            lines_hit: 0,
            loc_counter: None,
            coverage_index: Arc::new(CoverageIndex::empty()),
        }
    }

    /// Build the coverage index from current function data.
    ///
    /// This should be called after modifying the functions HashMap
    /// to ensure the index is up to date for O(1) lookups.
    pub fn build_index(&mut self) {
        self.coverage_index = Arc::new(CoverageIndex::from_coverage(self));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalized_function_name_simple() {
        let name = NormalizedFunctionName::simple("test_func");
        assert_eq!(name.full_path, "test_func");
        assert_eq!(name.method_name, "test_func");
        assert_eq!(name.original, "test_func");
    }

    #[test]
    fn test_lcov_data_default() {
        let data = LcovData::default();
        assert_eq!(data.total_lines, 0);
        assert_eq!(data.lines_hit, 0);
        assert!(data.functions.is_empty());
    }

    #[test]
    fn test_coverage_progress_debug() {
        // Ensure CoverageProgress variants can be debug printed
        let progress = CoverageProgress::Parsing {
            current: 5,
            total: 10,
        };
        let debug_str = format!("{:?}", progress);
        assert!(debug_str.contains("Parsing"));
        assert!(debug_str.contains("5"));
        assert!(debug_str.contains("10"));
    }
}
