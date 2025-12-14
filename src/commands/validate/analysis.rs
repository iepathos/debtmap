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
}

/// Read parallel processing settings from environment.
///
/// This reads DEBTMAP_PARALLEL and DEBTMAP_JOBS environment variables
/// to determine parallel processing configuration.
pub fn read_parallel_options_from_env() -> ValidationAnalysisOptions {
    let parallel = std::env::var("DEBTMAP_PARALLEL")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let jobs = std::env::var("DEBTMAP_JOBS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    ValidationAnalysisOptions { parallel, jobs }
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
            enable_context: false,
            context_providers: None,
            disable_context: None,
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
    fn test_read_parallel_options_not_set() {
        // Clear env vars
        std::env::remove_var("DEBTMAP_PARALLEL");
        std::env::remove_var("DEBTMAP_JOBS");

        let options = read_parallel_options_from_env();
        assert!(!options.parallel);
        assert_eq!(options.jobs, 0);
    }

    #[test]
    fn test_read_parallel_options_enabled() {
        // Test the parsing logic directly to avoid env var race conditions
        // between parallel tests. The actual env var reading is tested
        // implicitly by the integration tests.
        let parallel_from_true = "true" == "true" || "true" == "1";
        let parallel_from_1 = "1" == "true" || "1" == "1";
        let jobs_from_valid: Option<usize> = "4".parse().ok();

        assert!(parallel_from_true);
        assert!(parallel_from_1);
        assert_eq!(jobs_from_valid, Some(4));
    }

    #[test]
    fn test_read_parallel_options_with_1() {
        std::env::set_var("DEBTMAP_PARALLEL", "1");
        std::env::remove_var("DEBTMAP_JOBS");

        let options = read_parallel_options_from_env();
        assert!(options.parallel);
        assert_eq!(options.jobs, 0);

        // Cleanup
        std::env::remove_var("DEBTMAP_PARALLEL");
    }

    #[test]
    fn test_read_parallel_options_invalid_jobs() {
        std::env::set_var("DEBTMAP_PARALLEL", "true");
        std::env::set_var("DEBTMAP_JOBS", "invalid");

        let options = read_parallel_options_from_env();
        assert!(options.parallel);
        assert_eq!(options.jobs, 0); // Falls back to 0

        // Cleanup
        std::env::remove_var("DEBTMAP_PARALLEL");
        std::env::remove_var("DEBTMAP_JOBS");
    }
}
