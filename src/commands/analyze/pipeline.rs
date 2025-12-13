//! Pure transformation functions for the analyze command.
//!
//! This module contains pure functions with no I/O - the "Core" in
//! "Pure Core, Imperative Shell". All functions are deterministic
//! and easily testable.

use crate::analysis::FileContext;
use crate::cli::OutputFormat;
use crate::priority::{DebtCategory, UnifiedAnalysis, UnifiedAnalysisUtils};
use std::collections::HashMap;
use std::path::PathBuf;

/// Filter unified analysis by debt categories (pure).
///
/// Returns the original analysis unchanged if no categories specified
/// or if none of the specified categories are valid.
pub fn filter_by_categories(
    analysis: UnifiedAnalysis,
    filter_categories: Option<&[String]>,
) -> UnifiedAnalysis {
    let categories = parse_categories(filter_categories);
    if categories.is_empty() {
        return analysis;
    }
    analysis.filter_by_categories(&categories)
}

/// Parse category strings into DebtCategory enum values (pure).
fn parse_categories(filter_cats: Option<&[String]>) -> Vec<DebtCategory> {
    filter_cats
        .map(|cats| {
            cats.iter()
                .filter_map(|s| DebtCategory::from_string(s))
                .collect()
        })
        .unwrap_or_default()
}

/// Apply file context adjustments for test file scoring (pure).
pub fn apply_file_context(
    analysis: &mut UnifiedAnalysis,
    file_contexts: &HashMap<PathBuf, FileContext>,
) {
    analysis.apply_file_context_adjustments(file_contexts);
}

/// Determine if interactive TUI should be used (pure).
///
/// Returns false if any of these conditions apply:
/// - Explicitly disabled with --no-tui
/// - Non-terminal format specified (JSON, Markdown, HTML)
/// - Output file specified
/// - stdout is not a terminal (piped/redirected)
/// - CI environment detected
pub fn should_use_tui(
    no_tui: bool,
    format: OutputFormat,
    output_file: &Option<PathBuf>,
    is_terminal: bool,
    is_ci: bool,
) -> bool {
    !no_tui
        && matches!(format, OutputFormat::Terminal)
        && output_file.is_none()
        && is_terminal
        && !is_ci
}

/// Check if results are empty and return appropriate message (pure).
pub struct EmptyResultsInfo {
    pub message: String,
    pub current_threshold: String,
}

/// Check for empty results and return info if empty (pure).
pub fn check_empty_results(
    items_count: usize,
    file_items_count: usize,
    min_score_env: Option<&str>,
) -> Option<EmptyResultsInfo> {
    if items_count > 0 || file_items_count > 0 {
        return None;
    }

    let current_threshold = min_score_env
        .map(String::from)
        .unwrap_or_else(|| "3.0 (default)".to_string());

    Some(EmptyResultsInfo {
        message: "No technical debt items found matching current thresholds.".to_string(),
        current_threshold,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_use_tui_disabled_when_no_tui_flag() {
        assert!(!should_use_tui(
            true,
            OutputFormat::Terminal,
            &None,
            true,
            false
        ));
    }

    #[test]
    fn should_use_tui_disabled_for_json_format() {
        assert!(!should_use_tui(
            false,
            OutputFormat::Json,
            &None,
            true,
            false
        ));
    }

    #[test]
    fn should_use_tui_disabled_for_markdown_format() {
        assert!(!should_use_tui(
            false,
            OutputFormat::Markdown,
            &None,
            true,
            false
        ));
    }

    #[test]
    fn should_use_tui_disabled_for_html_format() {
        assert!(!should_use_tui(
            false,
            OutputFormat::Html,
            &None,
            true,
            false
        ));
    }

    #[test]
    fn should_use_tui_disabled_when_output_file_specified() {
        let output = Some(PathBuf::from("output.json"));
        assert!(!should_use_tui(
            false,
            OutputFormat::Terminal,
            &output,
            true,
            false
        ));
    }

    #[test]
    fn should_use_tui_disabled_when_not_terminal() {
        assert!(!should_use_tui(
            false,
            OutputFormat::Terminal,
            &None,
            false,
            false
        ));
    }

    #[test]
    fn should_use_tui_disabled_in_ci() {
        assert!(!should_use_tui(
            false,
            OutputFormat::Terminal,
            &None,
            true,
            true
        ));
    }

    #[test]
    fn should_use_tui_enabled_when_all_conditions_met() {
        assert!(should_use_tui(
            false,
            OutputFormat::Terminal,
            &None,
            true,
            false
        ));
    }

    #[test]
    fn check_empty_results_returns_none_for_nonempty() {
        assert!(check_empty_results(5, 0, None).is_none());
        assert!(check_empty_results(0, 3, None).is_none());
        assert!(check_empty_results(2, 1, None).is_none());
    }

    #[test]
    fn check_empty_results_returns_info_for_empty() {
        let result = check_empty_results(0, 0, None);
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.current_threshold, "3.0 (default)");
    }

    #[test]
    fn check_empty_results_uses_env_threshold() {
        let result = check_empty_results(0, 0, Some("5.0"));
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.current_threshold, "5.0");
    }

    #[test]
    fn parse_categories_empty_for_none() {
        let result = parse_categories(None);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_categories_empty_for_empty_slice() {
        let empty: Vec<String> = vec![];
        let result = parse_categories(Some(&empty));
        assert!(result.is_empty());
    }
}
