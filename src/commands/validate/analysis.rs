//! Unified analysis computation for validation.
//!
//! This module handles computing the unified analysis metrics used
//! during validation. It wraps the shared analysis pipeline with
//! validation-specific configuration.

use crate::builders::unified_analysis;
use crate::core::AnalysisResults;
use crate::priority::UnifiedAnalysis;
use std::path::PathBuf;

/// Options for configuring unified analysis during validation.
#[derive(Default)]
pub struct ValidationAnalysisOptions {
    /// Whether parallel processing is enabled
    pub parallel: bool,
    /// Number of parallel jobs (0 = auto)
    pub jobs: usize,
    /// Enable context-aware analysis
    pub enable_context: bool,
    /// Specific context providers to use
    pub context_providers: Option<Vec<String>>,
    /// Context providers to disable
    pub disable_context: Option<Vec<String>>,
}

/// Parse a parallel flag value.
///
/// Returns `true` if the value is "true" or "1".
fn parse_parallel_flag(value: &str) -> bool {
    value == "true" || value == "1"
}

/// Parse a jobs value.
///
/// Returns the parsed number, or 0 if invalid.
fn parse_jobs_value(value: &str) -> usize {
    value.parse::<usize>().unwrap_or(0)
}

/// Read parallel processing settings from environment.
///
/// This reads DEBTMAP_PARALLEL and DEBTMAP_JOBS environment variables
/// to determine parallel processing configuration.
pub fn read_parallel_options_from_env() -> ValidationAnalysisOptions {
    let parallel = std::env::var("DEBTMAP_PARALLEL")
        .map(|v| parse_parallel_flag(&v))
        .unwrap_or(false);

    let jobs = std::env::var("DEBTMAP_JOBS")
        .ok()
        .map(|v| parse_jobs_value(&v))
        .unwrap_or(0);

    ValidationAnalysisOptions {
        parallel,
        jobs,
        enable_context: false,
        context_providers: None,
        disable_context: None,
    }
}

/// Calculate unified analysis metrics for validation.
///
/// This performs the shared unified analysis pipeline with validation-specific
/// settings (e.g., suppressing coverage tips).
pub fn calculate_unified_analysis(
    results: &AnalysisResults,
    coverage_file: Option<&PathBuf>,
    options: &ValidationAnalysisOptions,
) -> UnifiedAnalysis {
    unified_analysis::perform_unified_analysis_with_options(
        unified_analysis::UnifiedAnalysisOptions {
            results,
            coverage_file,
            semantic_off: false,
            project_path: &results.project_path,
            verbose_macro_warnings: false,
            show_macro_stats: false,
            parallel: options.parallel,
            jobs: options.jobs,
            multi_pass: false,
            show_attribution: false,
            aggregate_only: false,
            no_aggregation: false,
            aggregation_method: None,
            min_problematic: None,
            no_god_object: false,
            suppress_coverage_tip: true, // Suppress coverage TIP for validate (spec 131)
            _formatting_config: Default::default(),
            enable_context: options.enable_context,
            context_providers: options.context_providers.clone(),
            disable_context: options.disable_context.clone(),
            rust_files: None,     // Validate doesn't have pre-discovered files
            extracted_data: None, // Validate doesn't pre-extract (spec 213)
        },
    )
    .expect("Unified analysis failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_validation_analysis_options() {
        let options = ValidationAnalysisOptions::default();
        assert!(!options.parallel);
        assert_eq!(options.jobs, 0);
    }

    #[test]
    fn test_parse_parallel_flag_true() {
        assert!(parse_parallel_flag("true"));
    }

    #[test]
    fn test_parse_parallel_flag_1() {
        assert!(parse_parallel_flag("1"));
    }

    #[test]
    fn test_parse_parallel_flag_false() {
        assert!(!parse_parallel_flag("false"));
        assert!(!parse_parallel_flag("0"));
        assert!(!parse_parallel_flag(""));
        assert!(!parse_parallel_flag("yes"));
    }

    #[test]
    fn test_parse_jobs_value_valid() {
        assert_eq!(parse_jobs_value("4"), 4);
        assert_eq!(parse_jobs_value("1"), 1);
        assert_eq!(parse_jobs_value("16"), 16);
    }

    #[test]
    fn test_parse_jobs_value_invalid() {
        assert_eq!(parse_jobs_value("invalid"), 0);
        assert_eq!(parse_jobs_value(""), 0);
        assert_eq!(parse_jobs_value("-1"), 0);
    }
}
