//! Pure handler functions for LCOV record types.
//!
//! This module contains pure functions that transform parser state without I/O.
//! Following Stillwater philosophy, these handlers are the "pure core" that
//! processes each LCOV record type.
//!
//! # Stillwater Philosophy
//!
//! All handler functions are pure transformations:
//! - They take mutable state and record data as input
//! - They modify state deterministically
//! - They have no I/O operations
//! - They can be tested in isolation
//!
//! # Handler Types
//!
//! Each LCOV record type has a corresponding handler:
//! - `handle_source_file` - SF: records
//! - `handle_function_name` - FN: records
//! - `handle_function_data` - FNDA: records
//! - `handle_line_data` - DA: records
//! - `handle_lines_found` - LF: records
//! - `handle_lines_hit` - LH: records
//! - `handle_end_of_record` - end_of_record markers

use super::coverage::process_function_coverage_parallel;
use super::demangle::demangle_function_name;
use super::normalize::normalize_demangled_name;
use super::types::{FunctionCoverage, LcovData, NormalizedFunctionName};
use std::collections::HashMap;
use std::path::PathBuf;

/// Mutable state during LCOV parsing - the "water flowing through".
///
/// This struct holds all the intermediate state needed while parsing
/// an LCOV file. It accumulates data for the current file until an
/// EndOfRecord marker is reached.
pub(crate) struct LcovParserState {
    /// The accumulated coverage data being built
    pub data: LcovData,
    /// Current file being processed
    pub current_file: Option<PathBuf>,
    /// Functions for the current file (keyed by normalized name)
    pub file_functions: HashMap<String, FunctionCoverage>,
    /// Line execution counts for the current file
    pub file_lines: HashMap<usize, u64>,
    /// Count of files processed (for progress reporting)
    pub file_count: usize,
}

impl LcovParserState {
    /// Create a new parser state.
    pub fn new() -> Self {
        Self {
            data: LcovData::default(),
            current_file: None,
            file_functions: HashMap::new(),
            file_lines: HashMap::new(),
            file_count: 0,
        }
    }
}

impl Default for LcovParserState {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a new FunctionCoverage from a normalized name and start line.
///
/// Pure function - creates new data without side effects.
///
/// # Arguments
///
/// * `normalized` - The normalized function name
/// * `start_line` - The line number where the function starts
///
/// # Returns
///
/// A new `FunctionCoverage` with zeroed coverage data.
pub fn create_function_coverage(
    normalized: NormalizedFunctionName,
    start_line: u32,
) -> FunctionCoverage {
    FunctionCoverage {
        name: normalized.full_path.clone(),
        start_line: start_line as usize,
        execution_count: 0,
        coverage_percentage: 0.0,
        uncovered_lines: Vec::new(),
        normalized,
    }
}

/// Finalize file functions into a sorted Vec.
///
/// Pure function - transforms HashMap to sorted Vec.
/// The HashMap is drained during this operation.
///
/// # Arguments
///
/// * `file_functions` - The HashMap of functions to finalize
///
/// # Returns
///
/// A Vec of functions sorted by start line.
pub fn finalize_file_functions(
    file_functions: &mut HashMap<String, FunctionCoverage>,
) -> Vec<FunctionCoverage> {
    let mut funcs: Vec<FunctionCoverage> = file_functions.drain().map(|(_, v)| v).collect();
    funcs.sort_by_key(|f| f.start_line);
    funcs
}

/// Handle SourceFile record - save previous file and start new one.
///
/// This handler is called when a new SF: (SourceFile) record is encountered.
/// It saves any accumulated data from the previous file and prepares
/// for the new file.
///
/// # Arguments
///
/// * `state` - The parser state to modify
/// * `path` - The path of the new source file
pub(crate) fn handle_source_file(state: &mut LcovParserState, path: PathBuf) {
    // Save previous file's data if any
    if let Some(file) = state.current_file.take() {
        if !state.file_functions.is_empty() {
            let funcs = finalize_file_functions(&mut state.file_functions);
            state.data.functions.insert(file, funcs);
        }
    }
    state.current_file = Some(path);
    state.file_functions.clear();
    state.file_lines.clear();
}

/// Handle FunctionName record - register a function definition.
///
/// This handler is called when an FN: (FunctionName) record is encountered.
/// It demangles and normalizes the function name, then registers it
/// for the current file.
///
/// # Deduplication
///
/// If a function with the same normalized name already exists, the existing
/// entry is kept. This handles cases where the same function appears multiple
/// times with different monomorphizations.
///
/// # Arguments
///
/// * `state` - The parser state to modify
/// * `start_line` - The line number where the function starts
/// * `name` - The function name (possibly mangled)
pub(crate) fn handle_function_name(state: &mut LcovParserState, start_line: u32, name: String) {
    let demangled = demangle_function_name(&name);
    let normalized = normalize_demangled_name(&demangled);

    // Use normalized full_path as key to consolidate duplicates
    state
        .file_functions
        .entry(normalized.full_path.clone())
        .or_insert_with(|| create_function_coverage(normalized, start_line));
}

/// Handle FunctionData record - update execution count.
///
/// This handler is called when an FNDA: (FunctionData) record is encountered.
/// It updates the execution count for a previously registered function.
///
/// # Execution Count
///
/// When consolidating multiple monomorphizations, the maximum execution
/// count is kept. This represents the total coverage across all variants.
///
/// # Arguments
///
/// * `state` - The parser state to modify
/// * `name` - The function name (possibly mangled)
/// * `count` - The execution count
pub(crate) fn handle_function_data(state: &mut LcovParserState, name: String, count: u64) {
    let demangled = demangle_function_name(&name);
    let normalized = normalize_demangled_name(&demangled);

    if let Some(func) = state.file_functions.get_mut(&normalized.full_path) {
        // Keep the maximum execution count when consolidating
        func.execution_count = func.execution_count.max(count);
        // Functions with count > 0 are considered 100% covered if no line data
        if func.coverage_percentage == 0.0 && count > 0 {
            func.coverage_percentage = 100.0;
        }
    }
}

/// Handle LineData record - track line execution counts.
///
/// This handler is called when a DA: (LineData) record is encountered.
/// It records the execution count for a specific line.
///
/// # Arguments
///
/// * `state` - The parser state to modify
/// * `line` - The line number
/// * `count` - The execution count
pub(crate) fn handle_line_data(state: &mut LcovParserState, line: u32, count: u64) {
    state.file_lines.insert(line as usize, count);
}

/// Handle LinesFound record - update total line count.
///
/// This handler is called when an LF: (LinesFound) record is encountered.
/// It accumulates the total line count across all files.
///
/// # Arguments
///
/// * `state` - The parser state to modify
/// * `found` - The number of executable lines found
pub(crate) fn handle_lines_found(state: &mut LcovParserState, found: u32) {
    state.data.total_lines += found as usize;
}

/// Handle LinesHit record - update hit line count.
///
/// This handler is called when an LH: (LinesHit) record is encountered.
/// It accumulates the hit line count across all files.
///
/// # Arguments
///
/// * `state` - The parser state to modify
/// * `hit` - The number of lines that were executed
pub(crate) fn handle_lines_hit(state: &mut LcovParserState, hit: u32) {
    state.data.lines_hit += hit as usize;
}

/// Handle EndOfRecord - finalize current file's coverage data.
///
/// This handler is called when an end_of_record marker is encountered.
/// It calculates per-function coverage using parallel processing,
/// saves the file's data, and prepares for the next file.
///
/// # Arguments
///
/// * `state` - The parser state to modify
pub(crate) fn handle_end_of_record(state: &mut LcovParserState) {
    // Use parallel processing for function coverage calculation
    process_function_coverage_parallel(&mut state.file_functions, &state.file_lines);

    // Save the file's data
    if let Some(file) = state.current_file.take() {
        if !state.file_functions.is_empty() {
            let funcs = finalize_file_functions(&mut state.file_functions);
            state.data.functions.insert(file, funcs);
        }
    }

    state.file_functions.clear();
    state.file_lines.clear();
    state.file_count += 1;
}

/// Handle incomplete file - finalize any remaining data without EndOfRecord.
///
/// This handler is called when the file ends without a proper end_of_record
/// marker. It ensures any accumulated data is not lost.
///
/// # Arguments
///
/// * `state` - The parser state to modify
pub(crate) fn handle_incomplete_file(state: &mut LcovParserState) {
    if let Some(file) = state.current_file.take() {
        if !state.file_functions.is_empty() {
            let funcs = finalize_file_functions(&mut state.file_functions);
            state.data.functions.insert(file, funcs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_function_coverage() {
        let normalized = NormalizedFunctionName {
            full_path: "module::func".to_string(),
            method_name: "func".to_string(),
            original: "module::func".to_string(),
        };

        let coverage = create_function_coverage(normalized, 42);

        assert_eq!(coverage.name, "module::func");
        assert_eq!(coverage.start_line, 42);
        assert_eq!(coverage.execution_count, 0);
        assert_eq!(coverage.coverage_percentage, 0.0);
        assert!(coverage.uncovered_lines.is_empty());
    }

    #[test]
    fn test_finalize_file_functions_sorts_by_line() {
        let mut functions = HashMap::new();

        // Insert in non-sorted order
        functions.insert(
            "func_c".to_string(),
            FunctionCoverage {
                name: "func_c".to_string(),
                start_line: 30,
                execution_count: 1,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
                normalized: NormalizedFunctionName::simple("func_c"),
            },
        );
        functions.insert(
            "func_a".to_string(),
            FunctionCoverage {
                name: "func_a".to_string(),
                start_line: 10,
                execution_count: 1,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
                normalized: NormalizedFunctionName::simple("func_a"),
            },
        );
        functions.insert(
            "func_b".to_string(),
            FunctionCoverage {
                name: "func_b".to_string(),
                start_line: 20,
                execution_count: 1,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
                normalized: NormalizedFunctionName::simple("func_b"),
            },
        );

        let sorted = finalize_file_functions(&mut functions);

        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].start_line, 10);
        assert_eq!(sorted[1].start_line, 20);
        assert_eq!(sorted[2].start_line, 30);
        assert!(functions.is_empty(), "HashMap should be drained");
    }

    #[test]
    fn test_handle_function_name_deduplicates() {
        let mut state = LcovParserState::new();
        state.current_file = Some(PathBuf::from("test.rs"));

        // Add the same function twice with different mangled forms
        handle_function_name(&mut state, 10, "my_func".to_string());
        handle_function_name(&mut state, 10, "my_func".to_string());

        // Should only have one entry
        assert_eq!(state.file_functions.len(), 1);
    }

    #[test]
    fn test_handle_function_data_updates_execution_count() {
        let mut state = LcovParserState::new();
        state.current_file = Some(PathBuf::from("test.rs"));

        // First register the function
        handle_function_name(&mut state, 10, "my_func".to_string());

        // Then update with execution data
        handle_function_data(&mut state, "my_func".to_string(), 5);

        let func = state.file_functions.get("my_func").unwrap();
        assert_eq!(func.execution_count, 5);
        assert_eq!(func.coverage_percentage, 100.0);
    }

    #[test]
    fn test_handle_function_data_keeps_max_count() {
        let mut state = LcovParserState::new();
        state.current_file = Some(PathBuf::from("test.rs"));

        handle_function_name(&mut state, 10, "my_func".to_string());

        // Multiple execution data entries - should keep max
        handle_function_data(&mut state, "my_func".to_string(), 3);
        handle_function_data(&mut state, "my_func".to_string(), 7);
        handle_function_data(&mut state, "my_func".to_string(), 5);

        let func = state.file_functions.get("my_func").unwrap();
        assert_eq!(
            func.execution_count, 7,
            "Should keep maximum execution count"
        );
    }

    #[test]
    fn test_handle_line_data_tracks_lines() {
        let mut state = LcovParserState::new();

        handle_line_data(&mut state, 10, 5);
        handle_line_data(&mut state, 20, 0);
        handle_line_data(&mut state, 30, 3);

        assert_eq!(state.file_lines.len(), 3);
        assert_eq!(state.file_lines.get(&10), Some(&5));
        assert_eq!(state.file_lines.get(&20), Some(&0));
        assert_eq!(state.file_lines.get(&30), Some(&3));
    }

    #[test]
    fn test_handle_source_file_transitions() {
        let mut state = LcovParserState::new();

        // Start with first file and add function
        handle_source_file(&mut state, PathBuf::from("file1.rs"));
        handle_function_name(&mut state, 10, "func1".to_string());
        handle_function_data(&mut state, "func1".to_string(), 1);

        // Transition to second file
        handle_source_file(&mut state, PathBuf::from("file2.rs"));

        // First file's data should be saved
        assert!(state
            .data
            .functions
            .contains_key(&PathBuf::from("file1.rs")));

        // Current file should be the new one
        assert_eq!(state.current_file, Some(PathBuf::from("file2.rs")));

        // File functions should be cleared
        assert!(state.file_functions.is_empty());
    }

    #[test]
    fn test_handle_incomplete_file() {
        let mut state = LcovParserState::new();

        // Setup file with data but no EndOfRecord
        state.current_file = Some(PathBuf::from("incomplete.rs"));
        handle_function_name(&mut state, 10, "orphan_func".to_string());
        handle_function_data(&mut state, "orphan_func".to_string(), 2);

        // Call incomplete handler
        handle_incomplete_file(&mut state);

        // Data should be saved despite missing EndOfRecord
        assert!(state
            .data
            .functions
            .contains_key(&PathBuf::from("incomplete.rs")));
        let funcs = state
            .data
            .functions
            .get(&PathBuf::from("incomplete.rs"))
            .unwrap();
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "orphan_func");
    }

    #[test]
    fn test_handle_lines_found_accumulates() {
        let mut state = LcovParserState::new();

        handle_lines_found(&mut state, 100);
        handle_lines_found(&mut state, 50);

        assert_eq!(state.data.total_lines, 150);
    }

    #[test]
    fn test_handle_lines_hit_accumulates() {
        let mut state = LcovParserState::new();

        handle_lines_hit(&mut state, 80);
        handle_lines_hit(&mut state, 40);

        assert_eq!(state.data.lines_hit, 120);
    }

    #[test]
    fn test_parser_state_initial_values() {
        let state = LcovParserState::new();

        assert!(state.current_file.is_none());
        assert!(state.file_functions.is_empty());
        assert!(state.file_lines.is_empty());
        assert_eq!(state.file_count, 0);
        assert_eq!(state.data.total_lines, 0);
        assert_eq!(state.data.lines_hit, 0);
    }
}
