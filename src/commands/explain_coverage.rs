use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::risk::coverage_index::CoverageIndex;
use crate::risk::lcov::parse_lcov_file;

#[derive(Debug, Clone, Copy)]
pub enum DebugFormat {
    Text,
    Json,
}

#[derive(Debug)]
pub struct ExplainCoverageConfig {
    pub path: PathBuf,
    pub coverage_file: PathBuf,
    pub function_name: String,
    pub file_path: Option<PathBuf>,
    pub verbose: bool,
    pub format: DebugFormat,
}

#[derive(Debug, Serialize, Deserialize)]
struct StrategyAttempt {
    strategy: String,
    success: bool,
    matched_function: Option<String>,
    matched_file: Option<String>,
    coverage_percentage: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExplainCoverageResult {
    function_name: String,
    file_path: Option<String>,
    coverage_found: bool,
    coverage_percentage: Option<f64>,
    matched_by_strategy: Option<String>,
    attempts: Vec<StrategyAttempt>,
    available_functions: Vec<String>,
    available_files: Vec<String>,
}

/// Explain how coverage detection works for a specific function
pub fn explain_coverage(config: ExplainCoverageConfig) -> Result<()> {
    // Parse LCOV data
    let lcov_data = parse_lcov_file(&config.coverage_file).context("Failed to parse LCOV file")?;

    // Build coverage index
    let coverage_index = CoverageIndex::from_coverage(&lcov_data);

    let mut result = ExplainCoverageResult {
        function_name: config.function_name.clone(),
        file_path: config.file_path.as_ref().map(|p| p.display().to_string()),
        coverage_found: false,
        coverage_percentage: None,
        matched_by_strategy: None,
        attempts: Vec::new(),
        available_functions: Vec::new(),
        available_files: Vec::new(),
    };

    // Collect available functions and files for debugging
    for (file, functions) in lcov_data.functions.iter() {
        result.available_files.push(file.display().to_string());
        for func in functions.iter() {
            result
                .available_functions
                .push(format!("{}::{}", file.display(), func.name));
        }
    }

    // If file path is provided, try with that file
    if let Some(file_path) = &config.file_path {
        // Try exact match first
        let attempt = try_exact_match(&coverage_index, file_path, &config.function_name);
        let success = attempt.success;
        result.attempts.push(attempt);

        if success {
            let last_attempt = result.attempts.last().unwrap();
            result.matched_by_strategy = Some(last_attempt.strategy.clone());
            result.coverage_found = true;
            result.coverage_percentage = last_attempt.coverage_percentage;
        }

        // Try path matching strategies
        if !result.coverage_found && config.verbose {
            let attempts = try_path_strategies(
                &coverage_index,
                file_path,
                &config.function_name,
                &lcov_data,
            );
            for attempt in attempts {
                let success = attempt.success;
                result.attempts.push(attempt);
                if success && !result.coverage_found {
                    let last_attempt = result.attempts.last().unwrap();
                    result.matched_by_strategy = Some(last_attempt.strategy.clone());
                    result.coverage_found = true;
                    result.coverage_percentage = last_attempt.coverage_percentage;
                }
            }
        }
    } else {
        // No file path provided, search all files
        let attempts = search_all_files(&coverage_index, &config.function_name, &lcov_data);
        for attempt in attempts {
            let success = attempt.success;
            result.attempts.push(attempt);
            if success && !result.coverage_found {
                let last_attempt = result.attempts.last().unwrap();
                result.matched_by_strategy = Some(last_attempt.strategy.clone());
                result.coverage_found = true;
                result.coverage_percentage = last_attempt.coverage_percentage;
            }
        }
    }

    // Output results
    match config.format {
        DebugFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        DebugFormat::Text => {
            output_text_format(&result, config.verbose);
        }
    }

    Ok(())
}

fn try_exact_match(
    coverage_index: &CoverageIndex,
    file_path: &Path,
    function_name: &str,
) -> StrategyAttempt {
    let coverage = coverage_index.get_function_coverage(file_path, function_name);

    StrategyAttempt {
        strategy: "exact_match".to_string(),
        success: coverage.is_some(),
        matched_function: if coverage.is_some() {
            Some(function_name.to_string())
        } else {
            None
        },
        matched_file: if coverage.is_some() {
            Some(file_path.display().to_string())
        } else {
            None
        },
        coverage_percentage: coverage,
    }
}

fn try_path_strategies(
    _coverage_index: &CoverageIndex,
    file_path: &PathBuf,
    function_name: &str,
    lcov_data: &crate::risk::lcov::LcovData,
) -> Vec<StrategyAttempt> {
    let mut attempts = Vec::new();

    // Strategy: Suffix matching
    for (lcov_file, functions) in &lcov_data.functions {
        if file_path.ends_with(lcov_file) || lcov_file.ends_with(file_path) {
            // Search for exact function name match
            if let Some(coverage_data) = functions.iter().find(|f| f.name == function_name) {
                attempts.push(StrategyAttempt {
                    strategy: "suffix_match".to_string(),
                    success: true,
                    matched_function: Some(function_name.to_string()),
                    matched_file: Some(lcov_file.display().to_string()),
                    coverage_percentage: Some(coverage_data.coverage_percentage / 100.0),
                });
                return attempts; // Return early on first success
            }

            // Try method name matching
            for coverage_data in functions {
                if coverage_data.normalized.method_name == function_name {
                    attempts.push(StrategyAttempt {
                        strategy: "method_name_match".to_string(),
                        success: true,
                        matched_function: Some(coverage_data.name.clone()),
                        matched_file: Some(lcov_file.display().to_string()),
                        coverage_percentage: Some(coverage_data.coverage_percentage / 100.0),
                    });
                    return attempts; // Return early on first success
                }
            }
        }
    }

    // Strategy: Normalized path equality (component-based)
    let query_components = crate::risk::path_normalization::normalize_path_components(file_path);
    for (lcov_file, functions) in &lcov_data.functions {
        let lcov_components = crate::risk::path_normalization::normalize_path_components(lcov_file);
        if lcov_components == query_components {
            if let Some(coverage_data) = functions.iter().find(|f| f.name == function_name) {
                attempts.push(StrategyAttempt {
                    strategy: "normalized_path_match".to_string(),
                    success: true,
                    matched_function: Some(function_name.to_string()),
                    matched_file: Some(lcov_file.display().to_string()),
                    coverage_percentage: Some(coverage_data.coverage_percentage / 100.0),
                });
                return attempts;
            }
        }
    }

    attempts.push(StrategyAttempt {
        strategy: "all_path_strategies".to_string(),
        success: false,
        matched_function: None,
        matched_file: None,
        coverage_percentage: None,
    });

    attempts
}

fn search_all_files(
    _coverage_index: &CoverageIndex,
    function_name: &str,
    lcov_data: &crate::risk::lcov::LcovData,
) -> Vec<StrategyAttempt> {
    let mut attempts = Vec::new();

    // Search for exact function name matches in all files
    for (lcov_file, functions) in &lcov_data.functions {
        if let Some(coverage_data) = functions.iter().find(|f| f.name == function_name) {
            attempts.push(StrategyAttempt {
                strategy: "global_function_name_match".to_string(),
                success: true,
                matched_function: Some(function_name.to_string()),
                matched_file: Some(lcov_file.display().to_string()),
                coverage_percentage: Some(coverage_data.coverage_percentage / 100.0),
            });
            return attempts; // Return on first match
        }
    }

    // Try method name matching across all files
    for (lcov_file, functions) in &lcov_data.functions {
        for coverage_data in functions {
            if coverage_data.normalized.method_name == function_name {
                attempts.push(StrategyAttempt {
                    strategy: "global_method_name_match".to_string(),
                    success: true,
                    matched_function: Some(coverage_data.name.clone()),
                    matched_file: Some(lcov_file.display().to_string()),
                    coverage_percentage: Some(coverage_data.coverage_percentage / 100.0),
                });
                return attempts; // Return on first match
            }
        }
    }

    attempts.push(StrategyAttempt {
        strategy: "global_search".to_string(),
        success: false,
        matched_function: None,
        matched_file: None,
        coverage_percentage: None,
    });

    attempts
}

// Pure formatting functions (the "still" core - testable with no I/O)

fn format_header(result: &ExplainCoverageResult) -> String {
    let mut output = String::new();
    output.push_str("Coverage Detection Explanation\n");
    output.push_str("==============================\n\n");
    output.push_str(&format!("Function: {}\n", result.function_name));
    if let Some(ref file) = result.file_path {
        output.push_str(&format!("File: {}\n", file));
    }
    output.push('\n');
    output
}

fn format_coverage_status(result: &ExplainCoverageResult) -> String {
    if result.coverage_found {
        format!(
            "✓ Coverage Found!\n  Strategy: {}\n  Coverage: {:.1}%\n\n",
            result.matched_by_strategy.as_ref().unwrap(),
            result.coverage_percentage.unwrap() * 100.0
        )
    } else {
        "✗ Coverage Not Found\n\n".to_string()
    }
}

fn format_attempt(attempt: &StrategyAttempt) -> String {
    let status = if attempt.success { "✓" } else { "✗" };
    let mut output = format!("  {} {}\n", status, attempt.strategy);

    if attempt.success {
        if let Some(ref matched_file) = attempt.matched_file {
            output.push_str(&format!("      File: {}\n", matched_file));
        }
        if let Some(ref matched_func) = attempt.matched_function {
            output.push_str(&format!("      Function: {}\n", matched_func));
        }
        if let Some(coverage) = attempt.coverage_percentage {
            output.push_str(&format!("      Coverage: {:.1}%\n", coverage * 100.0));
        }
    }
    output
}

fn format_matching_attempts(attempts: &[StrategyAttempt]) -> String {
    let mut output = String::from("Matching Attempts:\n------------------\n");
    for attempt in attempts {
        output.push_str(&format_attempt(attempt));
    }
    output.push('\n');
    output
}

fn format_truncated_list<T: AsRef<str>>(items: &[T], limit: usize) -> String {
    let mut output = String::new();
    for (i, item) in items.iter().take(limit).enumerate() {
        output.push_str(&format!("  {}. {}\n", i + 1, item.as_ref()));
    }
    if items.len() > limit {
        output.push_str(&format!("  ... and {} more\n", items.len() - limit));
    }
    output
}

fn format_available_files(files: &[String]) -> String {
    let mut output = format!("Available Files in LCOV ({} total):\n", files.len());
    output.push_str("----------------------------------\n");
    output.push_str(&format_truncated_list(files, 10));
    output.push('\n');
    output
}

fn find_matching_functions<'a>(
    function_name: &str,
    available_functions: &'a [String],
) -> Vec<&'a String> {
    let query = function_name.to_lowercase();
    available_functions
        .iter()
        .filter(|f| f.to_lowercase().contains(&query))
        .collect()
}

fn format_function_matches(function_name: &str, matching: &[&String]) -> String {
    let mut output = format!("  Functions containing '{}':\n", function_name);
    for func in matching.iter().take(10) {
        output.push_str(&format!("    - {}\n", func));
    }
    if matching.len() > 10 {
        output.push_str(&format!("    ... and {} more\n", matching.len() - 10));
    }
    output
}

fn format_first_functions(available_functions: &[String]) -> String {
    let mut output = String::from("  First 10 available functions:\n");
    for func in available_functions.iter().take(10) {
        output.push_str(&format!("    - {}\n", func));
    }
    if available_functions.len() > 10 {
        output.push_str(&format!(
            "    ... and {} more\n",
            available_functions.len() - 10
        ));
    }
    output
}

fn format_available_functions(result: &ExplainCoverageResult) -> String {
    let mut output = format!(
        "Available Functions in LCOV ({} total):\n",
        result.available_functions.len()
    );
    output.push_str("--------------------------------------\n");

    let matching = find_matching_functions(&result.function_name, &result.available_functions);

    if !matching.is_empty() {
        output.push_str(&format_function_matches(&result.function_name, &matching));
    } else {
        output.push_str(&format!(
            "  No functions found containing '{}'\n\n",
            result.function_name
        ));
        output.push_str(&format_first_functions(&result.available_functions));
    }
    output
}

/// Compose all sections into the complete text report (pure function)
fn format_text_report(result: &ExplainCoverageResult, verbose: bool) -> String {
    let mut output = format_header(result);
    output.push_str(&format_coverage_status(result));

    if verbose || !result.coverage_found {
        output.push_str(&format_matching_attempts(&result.attempts));
    }

    if !result.coverage_found {
        output.push_str(&format_available_files(&result.available_files));
        output.push_str(&format_available_functions(result));
    }

    output
}

// I/O shell (the "water" boundary - thin wrapper)
fn output_text_format(result: &ExplainCoverageResult, verbose: bool) {
    print!("{}", format_text_report(result, verbose));
}

// Make normalize_path public for use in this module
mod coverage_index_helpers {
    use std::path::{Path, PathBuf};

    pub fn normalize_path(path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);
        PathBuf::from(cleaned)
    }
}

// Re-export for use in coverage_index
pub use coverage_index_helpers::normalize_path;

#[cfg(test)]
mod tests {
    use super::*;

    // Test data builders (pure functions for creating test fixtures)
    fn make_result(
        function_name: &str,
        file_path: Option<&str>,
        coverage_found: bool,
    ) -> ExplainCoverageResult {
        ExplainCoverageResult {
            function_name: function_name.to_string(),
            file_path: file_path.map(String::from),
            coverage_found,
            coverage_percentage: if coverage_found { Some(0.85) } else { None },
            matched_by_strategy: if coverage_found {
                Some("exact_match".to_string())
            } else {
                None
            },
            attempts: Vec::new(),
            available_functions: Vec::new(),
            available_files: Vec::new(),
        }
    }

    fn make_attempt(strategy: &str, success: bool) -> StrategyAttempt {
        StrategyAttempt {
            strategy: strategy.to_string(),
            success,
            matched_function: if success {
                Some("test_fn".to_string())
            } else {
                None
            },
            matched_file: if success {
                Some("test.rs".to_string())
            } else {
                None
            },
            coverage_percentage: if success { Some(0.75) } else { None },
        }
    }

    #[test]
    fn format_header_without_file() {
        let result = make_result("my_function", None, false);
        let output = format_header(&result);

        assert!(output.contains("Coverage Detection Explanation"));
        assert!(output.contains("Function: my_function"));
        assert!(!output.contains("File:"));
    }

    #[test]
    fn format_header_with_file() {
        let result = make_result("my_function", Some("src/lib.rs"), false);
        let output = format_header(&result);

        assert!(output.contains("Function: my_function"));
        assert!(output.contains("File: src/lib.rs"));
    }

    #[test]
    fn format_coverage_status_found() {
        let result = make_result("test_fn", None, true);
        let output = format_coverage_status(&result);

        assert!(output.contains("✓ Coverage Found!"));
        assert!(output.contains("Strategy: exact_match"));
        assert!(output.contains("Coverage: 85.0%"));
    }

    #[test]
    fn format_coverage_status_not_found() {
        let result = make_result("test_fn", None, false);
        let output = format_coverage_status(&result);

        assert!(output.contains("✗ Coverage Not Found"));
        assert!(!output.contains("Strategy:"));
    }

    #[test]
    fn format_attempt_success() {
        let attempt = make_attempt("filename_match", true);
        let output = format_attempt(&attempt);

        assert!(output.contains("✓ filename_match"));
        assert!(output.contains("File: test.rs"));
        assert!(output.contains("Function: test_fn"));
        assert!(output.contains("Coverage: 75.0%"));
    }

    #[test]
    fn format_attempt_failure() {
        let attempt = make_attempt("fuzzy_search", false);
        let output = format_attempt(&attempt);

        assert!(output.contains("✗ fuzzy_search"));
        assert!(!output.contains("File:"));
        assert!(!output.contains("Function:"));
    }

    #[test]
    fn format_matching_attempts_multiple() {
        let attempts = vec![
            make_attempt("exact_match", false),
            make_attempt("filename_match", true),
        ];
        let output = format_matching_attempts(&attempts);

        assert!(output.contains("Matching Attempts:"));
        assert!(output.contains("✗ exact_match"));
        assert!(output.contains("✓ filename_match"));
    }

    #[test]
    fn format_truncated_list_under_limit() {
        let items = vec!["a.rs", "b.rs", "c.rs"];
        let output = format_truncated_list(&items, 10);

        assert!(output.contains("1. a.rs"));
        assert!(output.contains("2. b.rs"));
        assert!(output.contains("3. c.rs"));
        assert!(!output.contains("... and"));
    }

    #[test]
    fn format_truncated_list_over_limit() {
        let items: Vec<String> = (1..=15).map(|i| format!("file{}.rs", i)).collect();
        let output = format_truncated_list(&items, 10);

        assert!(output.contains("1. file1.rs"));
        assert!(output.contains("10. file10.rs"));
        assert!(!output.contains("11. file11.rs"));
        assert!(output.contains("... and 5 more"));
    }

    #[test]
    fn find_matching_functions_case_insensitive() {
        let available = vec![
            "module::MyFunction".to_string(),
            "other::my_function".to_string(),
            "unrelated::foo".to_string(),
        ];
        let matches = find_matching_functions("function", &available);

        assert_eq!(matches.len(), 2);
        assert!(matches.iter().any(|f| f.contains("MyFunction")));
        assert!(matches.iter().any(|f| f.contains("my_function")));
    }

    #[test]
    fn find_matching_functions_no_matches() {
        let available = vec!["module::foo".to_string(), "other::bar".to_string()];
        let matches = find_matching_functions("nonexistent", &available);

        assert!(matches.is_empty());
    }

    #[test]
    fn format_available_files_with_truncation() {
        let files: Vec<String> = (1..=15).map(|i| format!("src/file{}.rs", i)).collect();
        let output = format_available_files(&files);

        assert!(output.contains("Available Files in LCOV (15 total):"));
        assert!(output.contains("1. src/file1.rs"));
        assert!(output.contains("... and 5 more"));
    }

    #[test]
    fn format_available_functions_with_matches() {
        let mut result = make_result("parse", None, false);
        result.available_functions = vec![
            "module::parse_file".to_string(),
            "other::parse_line".to_string(),
            "unrelated::foo".to_string(),
        ];
        let output = format_available_functions(&result);

        assert!(output.contains("Functions containing 'parse':"));
        assert!(output.contains("- module::parse_file"));
        assert!(output.contains("- other::parse_line"));
        assert!(!output.contains("- unrelated::foo"));
    }

    #[test]
    fn format_available_functions_no_matches() {
        let mut result = make_result("nonexistent", None, false);
        result.available_functions = vec!["module::foo".to_string(), "other::bar".to_string()];
        let output = format_available_functions(&result);

        assert!(output.contains("No functions found containing 'nonexistent'"));
        assert!(output.contains("First 10 available functions:"));
        assert!(output.contains("- module::foo"));
    }

    #[test]
    fn format_text_report_coverage_found_not_verbose() {
        let result = make_result("test_fn", Some("src/lib.rs"), true);
        let output = format_text_report(&result, false);

        assert!(output.contains("✓ Coverage Found!"));
        // Should NOT show attempts when coverage found and not verbose
        assert!(!output.contains("Matching Attempts:"));
        // Should NOT show available files/functions when coverage found
        assert!(!output.contains("Available Files"));
    }

    #[test]
    fn format_text_report_coverage_found_verbose() {
        let mut result = make_result("test_fn", Some("src/lib.rs"), true);
        result.attempts = vec![make_attempt("exact_match", true)];
        let output = format_text_report(&result, true);

        assert!(output.contains("✓ Coverage Found!"));
        // SHOULD show attempts when verbose=true
        assert!(output.contains("Matching Attempts:"));
    }

    #[test]
    fn format_text_report_coverage_not_found() {
        let mut result = make_result("missing_fn", None, false);
        result.attempts = vec![make_attempt("exact_match", false)];
        result.available_files = vec!["src/main.rs".to_string()];
        result.available_functions = vec!["main::run".to_string()];
        let output = format_text_report(&result, false);

        assert!(output.contains("✗ Coverage Not Found"));
        // Should show attempts, available files, and functions when not found
        assert!(output.contains("Matching Attempts:"));
        assert!(output.contains("Available Files in LCOV"));
        assert!(output.contains("Available Functions in LCOV"));
    }
}
