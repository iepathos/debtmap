//! Validate command handler
//!
//! This module contains the handler for the `validate` subcommand,
//! which validates code against quality thresholds.

use crate::cli::args::Commands;
use crate::commands::validate::ValidateConfig;
use anyhow::Result;

/// Extracts parameters from the Validate command variant.
///
/// This function handles the destructuring of CLI parameters from the Commands enum
/// and builds the ValidateConfig. It's separated to keep the main handler focused
/// on coordination.
pub fn extract_validate_params(command: Commands) -> Result<ValidateConfig> {
    if let Commands::Validate {
        path,
        config,
        coverage_file,
        format,
        output,
        enable_context,
        context_providers,
        disable_context,
        max_debt_density,
        top,
        tail,
        summary: _,
        semantic_off,
        explain_score: _,
        verbosity,
        no_parallel,
        jobs,
        show_splits,
    } = command
    {
        Ok(ValidateConfig {
            path,
            config,
            coverage_file,
            format,
            output,
            enable_context,
            context_providers,
            disable_context,
            max_debt_density,
            top,
            tail,
            semantic_off,
            verbosity,
            no_parallel,
            jobs,
            show_splits,
        })
    } else {
        Err(anyhow::anyhow!(
            "Invalid command: expected Validate variant"
        ))
    }
}

/// Handle the validate command
///
/// This is the entry point for the validate command. It coordinates:
/// 1. Extract parameters and build configuration
/// 2. Delegate to validation logic
///
/// # Architecture
///
/// This function follows the "pure core, imperative shell" pattern and serves as a thin
/// coordination layer. The heavy lifting is delegated to:
/// - `extract_validate_params`: Parameter extraction and config building
/// - `validate_project`: Core validation logic
pub fn handle_validate_command(command: Commands) -> Result<()> {
    let config = extract_validate_params(command)?;
    crate::commands::validate::validate_project(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::{DebugFormatArg, OutputFormat};
    use std::path::PathBuf;

    #[test]
    fn test_extract_validate_params_wrong_variant() {
        // Test that extract_validate_params returns an error for non-Validate commands
        let analyze_command = Commands::Analyze {
            path: PathBuf::from("."),
            format: OutputFormat::Terminal,
            output: None,
            threshold_complexity: 10,
            threshold_duplication: 50,
            languages: None,
            coverage_file: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            top: None,
            tail: None,
            summary: false,
            semantic_off: false,
            explain_score: false,
            verbosity: 0,
            compact: false,
            verbose_macro_warnings: false,
            show_macro_stats: false,
            group_by_category: false,
            min_priority: None,
            min_score: None,
            filter_categories: None,
            no_context_aware: false,
            threshold_preset: None,
            plain: false,
            no_parallel: false,
            jobs: 0,
            no_multi_pass: false,
            show_attribution: false,
            detail_level: None,
            aggregate_only: false,
            no_aggregation: false,
            aggregation_method: None,
            min_problematic: None,
            no_god_object: false,
            max_files: None,
            validate_loc: false,
            no_public_api_detection: false,
            public_api_threshold: 0.7,
            no_pattern_detection: false,
            patterns: None,
            pattern_threshold: 0.7,
            show_pattern_warnings: false,
            explain_metrics: false,
            debug_call_graph: false,
            trace_functions: None,
            call_graph_stats_only: false,
            debug_format: DebugFormatArg::Text,
            validate_call_graph: false,
            show_dependencies: false,
            no_dependencies: false,
            max_callers: 5,
            max_callees: 5,
            show_external: false,
            show_std_lib: false,
            ast_functional_analysis: false,
            functional_analysis_profile: None,
            min_split_methods: 10,
            min_split_lines: 150,
            show_splits: false,
            no_tui: false,
            quiet: false,
            streaming: false,
            stream_to: None,
            show_filter_stats: false,
        };

        let result = extract_validate_params(analyze_command);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("expected Validate variant"));
    }

    #[test]
    fn test_extract_validate_params_success() {
        let validate_command = Commands::Validate {
            path: PathBuf::from("/test/path"),
            config: Some(PathBuf::from("/config/path")),
            coverage_file: Some(PathBuf::from("/coverage.lcov")),
            format: Some(OutputFormat::Json),
            output: Some(PathBuf::from("/output.json")),
            enable_context: true,
            context_providers: Some(vec!["critical_path".to_string()]),
            disable_context: None,
            max_debt_density: Some(0.5),
            top: Some(10),
            tail: None,
            summary: false,
            semantic_off: false,
            explain_score: false,
            verbosity: 1,
            no_parallel: false,
            jobs: 4,
            show_splits: true,
        };

        let result = extract_validate_params(validate_command);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.path, PathBuf::from("/test/path"));
        assert_eq!(config.config, Some(PathBuf::from("/config/path")));
        assert_eq!(config.coverage_file, Some(PathBuf::from("/coverage.lcov")));
        assert_eq!(config.format, Some(OutputFormat::Json));
        assert_eq!(config.output, Some(PathBuf::from("/output.json")));
        assert!(config.enable_context);
        assert_eq!(
            config.context_providers,
            Some(vec!["critical_path".to_string()])
        );
        assert_eq!(config.max_debt_density, Some(0.5));
        assert_eq!(config.top, Some(10));
        assert_eq!(config.verbosity, 1);
        assert!(!config.no_parallel);
        assert_eq!(config.jobs, 4);
        assert!(config.show_splits);
    }

    #[test]
    fn test_extract_validate_params_minimal() {
        let validate_command = Commands::Validate {
            path: PathBuf::from("."),
            config: None,
            coverage_file: None,
            format: None,
            output: None,
            enable_context: false,
            context_providers: None,
            disable_context: None,
            max_debt_density: None,
            top: None,
            tail: None,
            summary: false,
            semantic_off: false,
            explain_score: false,
            verbosity: 0,
            no_parallel: false,
            jobs: 0,
            show_splits: false,
        };

        let result = extract_validate_params(validate_command);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.path, PathBuf::from("."));
        assert!(config.config.is_none());
        assert!(config.coverage_file.is_none());
        assert!(!config.enable_context);
        assert_eq!(config.verbosity, 0);
    }
}
