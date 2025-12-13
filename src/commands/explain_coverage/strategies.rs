//! Coverage matching strategies.
//!
//! Pure functions that implement different strategies for matching
//! functions to their coverage data. Following the Stillwater philosophy,
//! these functions have no side effects and return strategy attempts.

use std::path::Path;

use crate::risk::coverage_index::CoverageIndex;
use crate::risk::lcov::LcovData;
use crate::risk::path_normalization::normalize_path_components;

use super::types::StrategyAttempt;

/// Try exact match of file path and function name.
pub fn try_exact_match(
    coverage_index: &CoverageIndex,
    file_path: &Path,
    function_name: &str,
) -> StrategyAttempt {
    match coverage_index.get_function_coverage(file_path, function_name) {
        Some(coverage) => StrategyAttempt::success(
            "exact_match",
            function_name.to_string(),
            file_path.display().to_string(),
            coverage,
        ),
        None => StrategyAttempt::failure("exact_match"),
    }
}

/// Try various path-based matching strategies.
///
/// Attempts in order:
/// 1. Suffix matching (file path ends with or is ended by)
/// 2. Method name extraction matching
/// 3. Normalized path component matching
pub fn try_path_strategies(
    file_path: &Path,
    function_name: &str,
    lcov_data: &LcovData,
) -> Vec<StrategyAttempt> {
    // Try suffix matching first
    if let Some(attempt) = try_suffix_match(file_path, function_name, lcov_data) {
        return vec![attempt];
    }

    // Try method name matching
    if let Some(attempt) = try_method_name_match(file_path, function_name, lcov_data) {
        return vec![attempt];
    }

    // Try normalized path matching
    if let Some(attempt) = try_normalized_path_match(file_path, function_name, lcov_data) {
        return vec![attempt];
    }

    vec![StrategyAttempt::failure("all_path_strategies")]
}

/// Search all files for a function by name.
///
/// Attempts in order:
/// 1. Exact function name match across all files
/// 2. Method name extraction match across all files
pub fn search_all_files(function_name: &str, lcov_data: &LcovData) -> Vec<StrategyAttempt> {
    // Try exact function name match
    if let Some(attempt) = try_global_function_match(function_name, lcov_data) {
        return vec![attempt];
    }

    // Try method name match
    if let Some(attempt) = try_global_method_match(function_name, lcov_data) {
        return vec![attempt];
    }

    vec![StrategyAttempt::failure("global_search")]
}

// =============================================================================
// Internal Strategy Implementations
// =============================================================================

fn try_suffix_match(
    file_path: &Path,
    function_name: &str,
    lcov_data: &LcovData,
) -> Option<StrategyAttempt> {
    for (lcov_file, functions) in &lcov_data.functions {
        if file_path.ends_with(lcov_file) || lcov_file.ends_with(file_path) {
            if let Some(coverage_data) = functions.iter().find(|f| f.name == function_name) {
                return Some(StrategyAttempt::success(
                    "suffix_match",
                    function_name.to_string(),
                    lcov_file.display().to_string(),
                    coverage_data.coverage_percentage / 100.0,
                ));
            }
        }
    }
    None
}

fn try_method_name_match(
    file_path: &Path,
    function_name: &str,
    lcov_data: &LcovData,
) -> Option<StrategyAttempt> {
    for (lcov_file, functions) in &lcov_data.functions {
        if file_path.ends_with(lcov_file) || lcov_file.ends_with(file_path) {
            for coverage_data in functions {
                if coverage_data.normalized.method_name == function_name {
                    return Some(StrategyAttempt::success(
                        "method_name_match",
                        coverage_data.name.clone(),
                        lcov_file.display().to_string(),
                        coverage_data.coverage_percentage / 100.0,
                    ));
                }
            }
        }
    }
    None
}

fn try_normalized_path_match(
    file_path: &Path,
    function_name: &str,
    lcov_data: &LcovData,
) -> Option<StrategyAttempt> {
    let query_components = normalize_path_components(file_path);

    for (lcov_file, functions) in &lcov_data.functions {
        let lcov_components = normalize_path_components(lcov_file);
        if lcov_components == query_components {
            if let Some(coverage_data) = functions.iter().find(|f| f.name == function_name) {
                return Some(StrategyAttempt::success(
                    "normalized_path_match",
                    function_name.to_string(),
                    lcov_file.display().to_string(),
                    coverage_data.coverage_percentage / 100.0,
                ));
            }
        }
    }
    None
}

fn try_global_function_match(function_name: &str, lcov_data: &LcovData) -> Option<StrategyAttempt> {
    for (lcov_file, functions) in &lcov_data.functions {
        if let Some(coverage_data) = functions.iter().find(|f| f.name == function_name) {
            return Some(StrategyAttempt::success(
                "global_function_name_match",
                function_name.to_string(),
                lcov_file.display().to_string(),
                coverage_data.coverage_percentage / 100.0,
            ));
        }
    }
    None
}

fn try_global_method_match(function_name: &str, lcov_data: &LcovData) -> Option<StrategyAttempt> {
    for (lcov_file, functions) in &lcov_data.functions {
        for coverage_data in functions {
            if coverage_data.normalized.method_name == function_name {
                return Some(StrategyAttempt::success(
                    "global_method_name_match",
                    coverage_data.name.clone(),
                    lcov_file.display().to_string(),
                    coverage_data.coverage_percentage / 100.0,
                ));
            }
        }
    }
    None
}
