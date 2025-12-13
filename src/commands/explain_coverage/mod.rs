//! Coverage explanation module.
//!
//! This module explains how coverage detection works for a specific function,
//! showing which matching strategies are attempted and their results.
//!
//! # Module Structure
//!
//! Following the Stillwater philosophy of "Pure Core, Imperative Shell":
//!
//! - `types` - Data structures (configuration, results, strategy attempts)
//! - `strategies` - Pure functions for coverage matching strategies
//! - `formatter` - Pure functions for text formatting
//!
//! # Architecture
//!
//! The main entry point `explain_coverage` is a thin I/O shell that:
//! 1. Reads and parses the LCOV file (I/O)
//! 2. Delegates to pure strategy functions to find matches
//! 3. Outputs results in requested format (I/O)

mod formatter;
mod strategies;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export public API
pub use types::{DebugFormat, ExplainCoverageConfig, ExplainCoverageResult, StrategyAttempt};

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::risk::coverage_index::CoverageIndex;
use crate::risk::lcov::{parse_lcov_file, LcovData};

use formatter::format_text_report;
use strategies::{search_all_files, try_exact_match, try_path_strategies};

// =============================================================================
// Public API
// =============================================================================

/// Explain how coverage detection works for a specific function.
///
/// This is the thin I/O shell that orchestrates:
/// 1. Parsing LCOV data (I/O)
/// 2. Running pure matching strategies
/// 3. Outputting results (I/O)
pub fn explain_coverage(config: ExplainCoverageConfig) -> Result<()> {
    // I/O: Parse LCOV file
    let lcov_data = parse_lcov_file(&config.coverage_file).context("Failed to parse LCOV file")?;

    // Build coverage index
    let coverage_index = CoverageIndex::from_coverage(&lcov_data);

    // Pure: Run matching strategies and build result
    let result = run_coverage_detection(
        &config.function_name,
        config.file_path.as_ref(),
        &coverage_index,
        &lcov_data,
        config.verbose,
    );

    // I/O: Output results
    output_result(&result, config.format)?;

    Ok(())
}

// =============================================================================
// Pure Core
// =============================================================================

/// Pure function that runs coverage detection strategies.
fn run_coverage_detection(
    function_name: &str,
    file_path: Option<&PathBuf>,
    coverage_index: &CoverageIndex,
    lcov_data: &LcovData,
    verbose: bool,
) -> ExplainCoverageResult {
    let mut result = ExplainCoverageResult::new(function_name.to_string(), file_path.cloned());

    // Collect available functions and files
    collect_available_data(&mut result, lcov_data);

    // Run matching strategies
    match file_path {
        Some(path) => run_file_strategies(
            &mut result,
            path,
            function_name,
            coverage_index,
            lcov_data,
            verbose,
        ),
        None => run_global_strategies(&mut result, function_name, lcov_data),
    }

    result
}

/// Collect available functions and files from LCOV data.
fn collect_available_data(result: &mut ExplainCoverageResult, lcov_data: &LcovData) {
    for (file, functions) in lcov_data.functions.iter() {
        result.available_files.push(file.display().to_string());
        for func in functions.iter() {
            result
                .available_functions
                .push(format!("{}::{}", file.display(), func.name));
        }
    }
}

/// Run strategies when a file path is provided.
fn run_file_strategies(
    result: &mut ExplainCoverageResult,
    file_path: &Path,
    function_name: &str,
    coverage_index: &CoverageIndex,
    lcov_data: &LcovData,
    verbose: bool,
) {
    // Try exact match first
    let attempt = try_exact_match(coverage_index, file_path, function_name);
    result.add_attempt(attempt);

    // Try path strategies if not found and verbose
    if !result.coverage_found && verbose {
        let attempts = try_path_strategies(file_path, function_name, lcov_data);
        for attempt in attempts {
            result.add_attempt(attempt);
        }
    }
}

/// Run global search strategies when no file path is provided.
fn run_global_strategies(
    result: &mut ExplainCoverageResult,
    function_name: &str,
    lcov_data: &LcovData,
) {
    let attempts = search_all_files(function_name, lcov_data);
    for attempt in attempts {
        result.add_attempt(attempt);
    }
}

// =============================================================================
// I/O Shell
// =============================================================================

/// Output result in the requested format (I/O boundary).
fn output_result(result: &ExplainCoverageResult, format: DebugFormat) -> Result<()> {
    match format {
        DebugFormat::Json => {
            println!("{}", serde_json::to_string_pretty(result)?);
        }
        DebugFormat::Text => {
            // Get verbose from result - if coverage not found, always show attempts
            let verbose = !result.coverage_found;
            print!("{}", format_text_report(result, verbose));
        }
    }
    Ok(())
}

// =============================================================================
// Path Normalization Helper
// =============================================================================

/// Normalize a path by removing leading "./" prefix.
pub fn normalize_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);
    PathBuf::from(cleaned)
}
