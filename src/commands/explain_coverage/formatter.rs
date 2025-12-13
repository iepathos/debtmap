//! Pure formatting functions for coverage explanation output.
//!
//! Following the Stillwater philosophy, these functions are the "still" core -
//! pure functions that transform data into formatted strings with no I/O.
//! The actual I/O (printing) happens at the boundary in mod.rs.

use super::types::{ExplainCoverageResult, StrategyAttempt};

/// Format the report header with function and file information.
pub fn format_header(result: &ExplainCoverageResult) -> String {
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

/// Format the coverage status (found or not found).
pub fn format_coverage_status(result: &ExplainCoverageResult) -> String {
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

/// Format a single strategy attempt.
pub fn format_attempt(attempt: &StrategyAttempt) -> String {
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

/// Format all matching attempts.
pub fn format_matching_attempts(attempts: &[StrategyAttempt]) -> String {
    let mut output = String::from("Matching Attempts:\n------------------\n");
    for attempt in attempts {
        output.push_str(&format_attempt(attempt));
    }
    output.push('\n');
    output
}

/// Format a list with truncation after limit.
pub fn format_truncated_list<T: AsRef<str>>(items: &[T], limit: usize) -> String {
    let mut output = String::new();
    for (i, item) in items.iter().take(limit).enumerate() {
        output.push_str(&format!("  {}. {}\n", i + 1, item.as_ref()));
    }
    if items.len() > limit {
        output.push_str(&format!("  ... and {} more\n", items.len() - limit));
    }
    output
}

/// Format available files section.
pub fn format_available_files(files: &[String]) -> String {
    let mut output = format!("Available Files in LCOV ({} total):\n", files.len());
    output.push_str("----------------------------------\n");
    output.push_str(&format_truncated_list(files, 10));
    output.push('\n');
    output
}

/// Find functions matching a query (case-insensitive).
pub fn find_matching_functions<'a>(
    function_name: &str,
    available_functions: &'a [String],
) -> Vec<&'a String> {
    let query = function_name.to_lowercase();
    available_functions
        .iter()
        .filter(|f| f.to_lowercase().contains(&query))
        .collect()
}

/// Format function matches section.
pub fn format_function_matches(function_name: &str, matching: &[&String]) -> String {
    let mut output = format!("  Functions containing '{}':\n", function_name);
    for func in matching.iter().take(10) {
        output.push_str(&format!("    - {}\n", func));
    }
    if matching.len() > 10 {
        output.push_str(&format!("    ... and {} more\n", matching.len() - 10));
    }
    output
}

/// Format first N available functions.
pub fn format_first_functions(available_functions: &[String]) -> String {
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

/// Format available functions section.
pub fn format_available_functions(result: &ExplainCoverageResult) -> String {
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

/// Compose all sections into the complete text report (pure function).
pub fn format_text_report(result: &ExplainCoverageResult, verbose: bool) -> String {
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
