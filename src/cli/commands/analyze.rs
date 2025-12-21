//! Analyze command handler
//!
//! This module contains the handler for the `analyze` subcommand,
//! including parameter extraction and configuration building.

use crate::cli::args::Commands;
use crate::cli::config_builder::{
    build_debug_config, build_display_config, build_feature_config, build_language_config,
    build_path_config, build_performance_config, build_threshold_config, compute_multi_pass,
    compute_verbosity, convert_context_providers, convert_disable_context,
    convert_filter_categories, convert_languages, convert_min_priority, convert_output_format,
    convert_threshold_preset, create_formatting_config, should_use_parallel, AnalysisFeatureConfig,
    DebugConfig, DisplayConfig, LanguageConfig, PathConfig, PerformanceConfig, ThresholdConfig,
};
use crate::cli::setup::{apply_environment_setup, get_worker_count, print_metrics_explanation};
use crate::error::{CliError, ConfigError};
use anyhow::Result;

/// Extracts and builds configuration from the Analyze command variant.
///
/// This function handles the destructuring of the 65 CLI parameters from the Commands enum
/// and builds all configuration groups. It's separated to keep the main handler focused
/// on coordination.
///
/// # Returns
///
/// Returns a tuple of configuration groups and special flags needed for handler coordination.
///
/// # Architecture Note
///
/// This is a necessary extraction function. While long, its complexity is structural
/// (destructuring + config building) rather than logical. The destructuring must happen
/// somewhere when working with clap's Commands enum.
#[allow(clippy::type_complexity)]
pub fn extract_analyze_params(
    command: Commands,
) -> Result<(
    PathConfig,
    ThresholdConfig,
    AnalysisFeatureConfig,
    DisplayConfig,
    PerformanceConfig,
    DebugConfig,
    LanguageConfig,
    bool, // explain_metrics flag
    bool, // no_context_aware flag
)> {
    if let Commands::Analyze {
        path,
        format,
        output,
        threshold_complexity,
        threshold_duplication,
        languages,
        coverage_file,
        enable_context,
        context_providers,
        disable_context,
        top,
        tail,
        summary,
        semantic_off,
        explain_score: _,
        verbosity,
        compact,
        verbose_macro_warnings,
        show_macro_stats,
        group_by_category,
        min_priority,
        min_score,
        filter_categories,
        no_context_aware,
        threshold_preset,
        plain,
        no_parallel,
        jobs,
        no_multi_pass,
        show_attribution,
        detail_level,
        aggregate_only,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
        max_files,
        validate_loc,
        no_public_api_detection,
        public_api_threshold,
        no_pattern_detection,
        patterns,
        pattern_threshold,
        show_pattern_warnings,
        explain_metrics,
        debug_call_graph,
        trace_functions,
        call_graph_stats_only,
        debug_format,
        validate_call_graph,
        show_dependencies,
        no_dependencies,
        max_callers,
        max_callees,
        show_external,
        show_std_lib,
        ast_functional_analysis,
        functional_analysis_profile,
        min_split_methods,
        min_split_lines,
        show_splits,
        no_tui,
        quiet: _,
        streaming: _,
        stream_to: _,
        show_filter_stats,
    } = command
    {
        // Build configuration groups using pure builder functions
        let path_cfg = build_path_config(
            path,
            output,
            coverage_file,
            max_files,
            min_priority,
            min_score,
            filter_categories,
            min_problematic,
        );

        let threshold_cfg = build_threshold_config(
            threshold_complexity,
            threshold_duplication,
            threshold_preset,
            public_api_threshold,
        );

        let feature_cfg = build_feature_config(
            enable_context,
            context_providers,
            disable_context,
            semantic_off,
            no_pattern_detection,
            patterns,
            pattern_threshold,
            no_god_object,
            no_public_api_detection,
            ast_functional_analysis,
            functional_analysis_profile,
            min_split_methods,
            min_split_lines,
            validate_loc,
            validate_call_graph,
        );

        let formatting_config = create_formatting_config(
            plain,
            show_dependencies,
            no_dependencies,
            max_callers,
            max_callees,
            show_external,
            show_std_lib,
            show_splits,
        );

        let display_cfg = build_display_config(
            format,
            compute_verbosity(verbosity, compact),
            summary,
            top,
            tail,
            group_by_category,
            show_attribution,
            detail_level,
            no_tui,
            show_filter_stats,
            formatting_config,
            no_context_aware,
        );

        let perf_cfg = build_performance_config(
            should_use_parallel(no_parallel),
            get_worker_count(jobs),
            compute_multi_pass(no_multi_pass),
            aggregate_only,
            no_aggregation,
        );

        let debug_cfg = build_debug_config(
            verbose_macro_warnings,
            show_macro_stats,
            debug_call_graph,
            trace_functions,
            call_graph_stats_only,
            debug_format,
            show_pattern_warnings,
            show_dependencies,
            no_dependencies,
        );

        let lang_cfg = build_language_config(
            languages,
            aggregation_method,
            max_callers,
            max_callees,
            show_external,
            show_std_lib,
        );

        Ok((
            path_cfg,
            threshold_cfg,
            feature_cfg,
            display_cfg,
            perf_cfg,
            debug_cfg,
            lang_cfg,
            explain_metrics,
            no_context_aware,
        ))
    } else {
        Err(anyhow::anyhow!("Invalid command: expected Analyze variant"))
    }
}

/// Handles the analyze command (coordination only).
///
/// This is the entry point for the analyze command. It coordinates the three main steps:
/// 1. Extract parameters and build configuration
/// 2. Apply environment setup (side effects)
/// 3. Delegate to analysis handler
///
/// # Architecture
///
/// This function follows the "pure core, imperative shell" pattern and serves as a thin
/// coordination layer (30-40 lines). The heavy lifting is delegated to:
/// - `extract_analyze_params`: Parameter extraction and config building
/// - `apply_environment_setup`: Side effects at the boundary
/// - `handle_analyze`: Core analysis logic
///
/// # Returns
///
/// Returns `Result<(), CliError>` for all CLI-related errors (configuration, validation,
/// or analysis execution errors).
///
/// # Specification
///
/// Implements specs 182 and 206: Refactor handle_analyze_command into composable functions
/// with clear error types. This handler is now 30-40 lines (coordination only), with
/// parameter extraction delegated to `extract_analyze_params` and uses typed errors
/// instead of nested Results.
pub fn handle_analyze_command(command: Commands) -> Result<(), CliError> {
    // Extract parameters and build configuration groups
    let (
        path_cfg,
        threshold_cfg,
        feature_cfg,
        display_cfg,
        perf_cfg,
        debug_cfg,
        lang_cfg,
        explain_metrics,
        no_context_aware,
    ) = extract_analyze_params(command).map_err(|e| CliError::InvalidCommand(e.to_string()))?;

    // Apply side effects (I/O at edges)
    apply_environment_setup(no_context_aware)
        .map_err(|e| CliError::Config(ConfigError::ValidationFailed(e.to_string())))?;

    // Handle explain-metrics flag (early return for info display)
    if explain_metrics {
        print_metrics_explanation();
        return Ok(());
    }

    // Build final configuration from component configs (unvalidated)
    let unvalidated_config = build_analyze_config(
        path_cfg,
        threshold_cfg,
        feature_cfg,
        display_cfg,
        perf_cfg,
        debug_cfg,
        lang_cfg,
    );

    // Validate configuration - transition to validated state
    let validated_config = unvalidated_config
        .validate()
        .map_err(|e| CliError::Config(ConfigError::ValidationFailed(e.to_string())))?;

    // Execute with validated configuration - compile-time guarantee of validation
    validated_config
        .execute()
        .map_err(|e| CliError::Config(ConfigError::ValidationFailed(e.to_string())))
}

/// Build analyze configuration from grouped configuration structs (spec 204)
fn build_analyze_config(
    p: PathConfig,
    t: ThresholdConfig,
    f: AnalysisFeatureConfig,
    d: DisplayConfig,
    pf: PerformanceConfig,
    db: DebugConfig,
    l: LanguageConfig,
) -> crate::commands::AnalyzeConfig<crate::commands::Unvalidated> {
    crate::commands::AnalyzeConfig::new(
        p.path,
        convert_output_format(d.format),
        p.output,
        t.complexity,
        t.duplication,
        convert_languages(l.languages),
        p.coverage_file,
        f.enable_context,
        convert_context_providers(f.context_providers),
        convert_disable_context(f.disable_context),
        d.top,
        d.tail,
        d.summary,
        f.semantic_off,
        d.verbosity,
        db.verbose_macro_warnings,
        db.show_macro_stats,
        d.group_by_category,
        convert_min_priority(p.min_priority),
        p.min_score,
        convert_filter_categories(p.filter_categories),
        d.no_context_aware,
        convert_threshold_preset(t.preset),
        d.formatting_config,
        pf.parallel,
        pf.jobs,
        pf.multi_pass,
        d.show_attribution,
        d.detail_level,
        pf.aggregate_only,
        pf.no_aggregation,
        l.aggregation_method,
        p.min_problematic,
        f.no_god_object,
        p.max_files,
        f.validate_loc,
        f.no_public_api_detection,
        t.public_api_threshold,
        f.no_pattern_detection,
        f.patterns,
        f.pattern_threshold,
        db.show_pattern_warnings,
        db.debug_call_graph,
        db.trace_functions,
        db.call_graph_stats_only,
        db.debug_format,
        f.validate_call_graph,
        db.show_dependencies,
        db.no_dependencies,
        l.max_callers,
        l.max_callees,
        l.show_external,
        l.show_std_lib,
        f.ast_functional_analysis,
        f.functional_analysis_profile,
        f.min_split_methods,
        f.min_split_lines,
        d.no_tui,
        d.show_filter_stats,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CliError;

    #[test]
    fn test_error_mapping_cli_to_app() {
        let cli_error = CliError::InvalidCommand("test error".to_string());
        let error_string = format!("{}", cli_error);
        assert!(error_string.contains("test error"));
    }

    #[test]
    fn test_handler_composition_with_errors() {
        // Simulate a Result from analyze domain
        let analyze_result: Result<(), anyhow::Error> = Err(anyhow::anyhow!("analysis failed"));

        // Map to CLI error domain
        let cli_result: Result<(), CliError> = analyze_result
            .map_err(|e| CliError::Config(ConfigError::ValidationFailed(e.to_string())));

        // Verify error was properly mapped
        assert!(cli_result.is_err());
        match cli_result.unwrap_err() {
            CliError::Config(ConfigError::ValidationFailed(msg)) => {
                assert!(msg.contains("analysis failed"));
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_pipeline_error_handling() {
        // Create a pipeline of Result transformations
        let result = Ok::<i32, String>(42)
            .map(|x| x * 2)
            .map_err(|e| format!("Stage 1: {}", e))
            .and_then(|x| {
                if x > 50 {
                    Ok(x)
                } else {
                    Err("Too small".to_string())
                }
            })
            .map_err(|e| format!("Stage 2: {}", e));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 84);
    }

    #[test]
    fn test_error_context_preservation() {
        use anyhow::Context;

        // Simulate a nested operation that adds context
        let result: Result<(), anyhow::Error> = Err(anyhow::anyhow!("root cause"))
            .context("Operation failed")
            .context("Command failed");

        // Map to CLI error preserving context chain
        let cli_result: Result<(), CliError> =
            result.map_err(|e| CliError::Config(ConfigError::ValidationFailed(format!("{:#}", e))));

        // Verify context is preserved
        assert!(cli_result.is_err());
        let error_msg = format!("{}", cli_result.unwrap_err());
        assert!(error_msg.contains("Command failed"));
    }
}
