//! Unified analysis computation for validation.
//!
//! This module handles computing the unified analysis metrics used
//! during validation. It wraps the shared analysis pipeline with
//! validation-specific configuration.

use super::types::ValidateConfig;
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

/// Derive analysis options from the validated CLI configuration.
pub fn options_from_config(config: &ValidateConfig) -> ValidationAnalysisOptions {
    ValidationAnalysisOptions {
        parallel: !config.no_parallel,
        jobs: config.jobs,
        enable_context: config.enable_context,
        context_providers: config.context_providers.clone(),
        disable_context: config.disable_context.clone(),
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
            reference_time: chrono::Utc::now(),
        },
    )
    .expect("Unified analysis failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_default_validation_analysis_options() {
        let options = ValidationAnalysisOptions::default();
        assert!(!options.parallel);
        assert_eq!(options.jobs, 0);
    }

    #[test]
    fn options_from_config_preserves_explicit_execution_settings() {
        let config = ValidateConfig {
            path: PathBuf::from("."),
            config: None,
            coverage_file: None,
            format: None,
            output: None,
            enable_context: true,
            context_providers: Some(vec!["git_history".to_string()]),
            disable_context: Some(vec!["dependency".to_string()]),
            max_debt_density: None,
            top: None,
            tail: None,
            semantic_off: false,
            verbosity: 0,
            no_parallel: true,
            jobs: 3,
            show_splits: false,
        };

        let options = options_from_config(&config);

        assert!(!options.parallel);
        assert_eq!(options.jobs, 3);
        assert!(options.enable_context);
        assert_eq!(options.context_providers, config.context_providers);
        assert_eq!(options.disable_context, config.disable_context);
    }
}
