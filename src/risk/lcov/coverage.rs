//! Parallel coverage calculation.
//!
//! This module provides pure functions for computing function coverage
//! percentages from line execution data. All calculations are done using
//! parallel processing for performance on large codebases.
//!
//! # Stillwater Philosophy
//!
//! The coverage calculation functions are pure:
//! - They take immutable data as input
//! - They produce deterministic results
//! - They can be safely run in parallel
//!
//! # Performance
//!
//! Uses rayon for parallel iteration, making efficient use of multiple
//! CPU cores when processing large numbers of functions.

use super::types::FunctionCoverage;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;

/// Coverage data for a single function (intermediate result).
///
/// This struct holds the calculated coverage information for a function
/// before it's merged back into the `FunctionCoverage` struct.
#[derive(Debug)]
pub struct FunctionCoverageData {
    /// Percentage of lines covered (0.0 to 100.0)
    pub coverage_percentage: f64,
    /// List of line numbers that weren't executed
    pub uncovered_lines: Vec<usize>,
}

/// Calculate coverage data for a single function.
///
/// Pure function that can be called in parallel. Uses binary search
/// for efficient range queries on sorted line data.
///
/// # Arguments
///
/// * `func_start` - The starting line number of the function
/// * `func_boundaries` - Sorted list of all function start lines in the file
/// * `sorted_lines` - Sorted list of (line_number, execution_count) pairs
///
/// # Returns
///
/// A `FunctionCoverageData` containing the coverage percentage and
/// list of uncovered lines.
///
/// # Algorithm
///
/// 1. Find the function's end line (start of next function or EOF)
/// 2. Binary search to find lines in the function's range
/// 3. Count covered vs uncovered lines
/// 4. Calculate percentage and collect uncovered line numbers
///
/// # Performance
///
/// O(log n) for binary searches, O(m) for line counting where m is
/// the number of lines in the function.
pub fn calculate_function_coverage_data(
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

/// Process all functions in a file in parallel.
///
/// This function calculates coverage percentages and uncovered lines for
/// all functions in a file using parallel processing.
///
/// # Arguments
///
/// * `file_functions` - Mutable map of function name to coverage data
/// * `file_lines` - Map of line number to execution count
///
/// # Side Effects
///
/// Modifies `file_functions` in place, updating `coverage_percentage`
/// and `uncovered_lines` for each function.
///
/// # Performance
///
/// Uses rayon for parallel iteration. The function boundaries and line
/// data are pre-sorted for efficient binary search queries.
pub fn process_function_coverage_parallel(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::lcov::types::NormalizedFunctionName;

    #[test]
    fn test_calculate_function_coverage_full() {
        let func_boundaries = vec![10, 20, 30];
        let sorted_lines = vec![
            (10, 5),
            (11, 3),
            (12, 7),
            (20, 0),
            (21, 0),
            (30, 1),
            (31, 1),
        ];

        // Function at line 10 has all lines covered
        let result = calculate_function_coverage_data(10, &func_boundaries, &sorted_lines);
        assert_eq!(result.coverage_percentage, 100.0);
        assert!(result.uncovered_lines.is_empty());
    }

    #[test]
    fn test_calculate_function_coverage_partial() {
        let func_boundaries = vec![10, 20, 30];
        let sorted_lines = vec![
            (10, 5),
            (11, 0),
            (12, 7),
            (20, 0),
            (21, 0),
            (30, 1),
            (31, 1),
        ];

        // Function at line 10 has 2/3 lines covered
        let result = calculate_function_coverage_data(10, &func_boundaries, &sorted_lines);
        assert!((result.coverage_percentage - 66.67).abs() < 0.1);
        assert_eq!(result.uncovered_lines, vec![11]);
    }

    #[test]
    fn test_calculate_function_coverage_none() {
        let func_boundaries = vec![10, 20, 30];
        let sorted_lines = vec![
            (10, 5),
            (11, 3),
            (12, 7),
            (20, 0),
            (21, 0),
            (30, 1),
            (31, 1),
        ];

        // Function at line 20 has no lines covered
        let result = calculate_function_coverage_data(20, &func_boundaries, &sorted_lines);
        assert_eq!(result.coverage_percentage, 0.0);
        assert_eq!(result.uncovered_lines, vec![20, 21]);
    }

    #[test]
    fn test_calculate_function_coverage_empty() {
        let func_boundaries = vec![10];
        let sorted_lines: Vec<(usize, u64)> = vec![];

        let result = calculate_function_coverage_data(10, &func_boundaries, &sorted_lines);
        assert_eq!(result.coverage_percentage, 0.0);
        assert!(result.uncovered_lines.is_empty());
    }

    #[test]
    fn test_process_function_coverage_parallel() {
        let mut functions = HashMap::new();
        functions.insert(
            "func_a".to_string(),
            FunctionCoverage {
                name: "func_a".to_string(),
                start_line: 10,
                execution_count: 1,
                coverage_percentage: 0.0,
                uncovered_lines: vec![],
                normalized: NormalizedFunctionName::simple("func_a"),
            },
        );
        functions.insert(
            "func_b".to_string(),
            FunctionCoverage {
                name: "func_b".to_string(),
                start_line: 20,
                execution_count: 0,
                coverage_percentage: 0.0,
                uncovered_lines: vec![],
                normalized: NormalizedFunctionName::simple("func_b"),
            },
        );

        let mut lines = HashMap::new();
        lines.insert(10, 5);
        lines.insert(11, 5);
        lines.insert(20, 0);
        lines.insert(21, 0);

        process_function_coverage_parallel(&mut functions, &lines);

        let func_a = functions.get("func_a").unwrap();
        assert_eq!(func_a.coverage_percentage, 100.0);

        let func_b = functions.get("func_b").unwrap();
        assert_eq!(func_b.coverage_percentage, 0.0);
        assert_eq!(func_b.uncovered_lines, vec![20, 21]);
    }

    #[test]
    fn test_process_function_coverage_empty_functions() {
        let mut functions = HashMap::new();
        let lines = HashMap::new();

        // Should not panic
        process_function_coverage_parallel(&mut functions, &lines);
    }

    #[test]
    fn test_process_function_coverage_empty_lines() {
        let mut functions = HashMap::new();
        functions.insert(
            "func".to_string(),
            FunctionCoverage {
                name: "func".to_string(),
                start_line: 10,
                execution_count: 1,
                coverage_percentage: 50.0, // Pre-existing value
                uncovered_lines: vec![],
                normalized: NormalizedFunctionName::simple("func"),
            },
        );
        let lines = HashMap::new();

        // Should not panic, and should not modify the function
        process_function_coverage_parallel(&mut functions, &lines);

        // Coverage should be unchanged since there's no line data to process
        let func = functions.get("func").unwrap();
        assert_eq!(func.coverage_percentage, 50.0);
    }
}
