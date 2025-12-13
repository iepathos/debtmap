//! Orchestrator module for the analyze command.
//!
//! This module provides the main entry point and orchestrates I/O with pure functions.
//! It follows the "Shell" pattern - thin I/O composition layer that delegates to
//! specialized modules for heavy lifting.

use super::config::AnalyzeConfig;
use super::{diagnostics, pipeline, project_analysis};
use crate::builders::unified_analysis;
use crate::core::AnalysisResults;
use crate::io;
use crate::output::{self, OutputConfig};
use crate::progress::ProgressManager;
use anyhow::Result;
use std::io::IsTerminal;

// Re-export for backward compatibility
pub use project_analysis::analyze_project;

/// Main entry point - orchestrates analysis (thin wrapper).
pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    setup_analysis_environment(&config);
    let (results, unified) = run_analysis_phases(&config)?;
    process_and_output_results(unified, &config, &results)
}

/// Setup analysis environment (I/O).
fn setup_analysis_environment(config: &AnalyzeConfig) {
    super::config::setup_environment(config);
    super::config::setup_progress_manager(config.verbosity);
}

/// Run analysis and build unified results (I/O).
fn run_analysis_phases(
    config: &AnalyzeConfig,
) -> Result<(AnalysisResults, crate::priority::UnifiedAnalysis)> {
    let results = project_analysis::run_analysis(config)?;
    let mut unified = build_unified_analysis_options(config, &results)?;

    pipeline::apply_file_context(&mut unified, &results.file_contexts);
    let filtered = pipeline::filter_by_categories(unified, config.filter_categories.as_deref());

    Ok((results, filtered))
}

/// Build unified analysis from results (I/O).
fn build_unified_analysis_options(
    config: &AnalyzeConfig,
    results: &AnalysisResults,
) -> Result<crate::priority::UnifiedAnalysis> {
    let options = create_analysis_options(config, results);
    unified_analysis::perform_unified_analysis_with_options(options)
}

/// Create analysis options from config.
fn create_analysis_options<'a>(
    config: &'a AnalyzeConfig,
    results: &'a AnalysisResults,
) -> unified_analysis::UnifiedAnalysisOptions<'a> {
    unified_analysis::UnifiedAnalysisOptions {
        results,
        coverage_file: config.coverage_file.as_ref(),
        semantic_off: config.semantic_off,
        project_path: &config.path,
        verbose_macro_warnings: config.verbose_macro_warnings,
        show_macro_stats: config.show_macro_stats,
        parallel: config.parallel,
        jobs: config.jobs,
        multi_pass: config.multi_pass,
        show_attribution: config.show_attribution,
        aggregate_only: config.aggregate_only,
        no_aggregation: config.no_aggregation,
        aggregation_method: config.aggregation_method.clone(),
        min_problematic: config.min_problematic,
        no_god_object: config.no_god_object,
        suppress_coverage_tip: false,
        _formatting_config: config._formatting_config,
        enable_context: config.enable_context,
        context_providers: config.context_providers.clone(),
        disable_context: config.disable_context.clone(),
    }
}

/// Process results and output (I/O).
fn process_and_output_results(
    unified: crate::priority::UnifiedAnalysis,
    config: &AnalyzeConfig,
    results: &AnalysisResults,
) -> Result<()> {
    run_diagnostics_if_needed(&unified, config)?;
    handle_empty_results(&unified);
    cleanup_progress();
    output_results(unified, config, results)
}

/// Run diagnostics if needed (I/O).
fn run_diagnostics_if_needed(
    analysis: &crate::priority::UnifiedAnalysis,
    config: &AnalyzeConfig,
) -> Result<()> {
    if config.needs_diagnostics() {
        diagnostics::handle_call_graph(analysis, config)?;
    }
    Ok(())
}

/// Handle empty results notification (I/O).
fn handle_empty_results(analysis: &crate::priority::UnifiedAnalysis) {
    let min_score_env = std::env::var("DEBTMAP_MIN_SCORE_THRESHOLD").ok();
    let empty_info = pipeline::check_empty_results(
        analysis.items.len(),
        analysis.file_items.len(),
        min_score_env.as_deref(),
    );

    if let Some(info) = empty_info {
        print_empty_results_help(&info);
    }
}

/// Print help message for empty results.
fn print_empty_results_help(info: &pipeline::EmptyResultsInfo) {
    eprintln!("{}", info.message);
    eprintln!("Try adjusting filters:");
    eprintln!("  - Use --min-score <value> to lower the score threshold");
    eprintln!(
        "  - Current min_score threshold: {}",
        info.current_threshold
    );
    eprintln!("  - Use DEBTMAP_MIN_SCORE_THRESHOLD=0 to see all items");
}

/// Cleanup TUI and progress (I/O).
fn cleanup_progress() {
    if let Some(manager) = ProgressManager::global() {
        manager.tui_set_progress(1.0);
        manager.tui_cleanup();
    }
    io::progress::AnalysisProgress::with_global(|p| p.finish());
}

/// Output results to terminal or file (I/O).
fn output_results(
    analysis: crate::priority::UnifiedAnalysis,
    config: &AnalyzeConfig,
    results: &AnalysisResults,
) -> Result<()> {
    if should_use_tui(config) {
        launch_tui(analysis)
    } else {
        output_traditional(analysis, config, results)
    }
}

/// Determine if TUI should be used.
fn should_use_tui(config: &AnalyzeConfig) -> bool {
    let is_terminal = std::io::stdout().is_terminal();
    let is_ci = std::env::var("CI").is_ok();
    pipeline::should_use_tui(
        config.no_tui,
        config.format,
        &config.output,
        is_terminal,
        is_ci,
    )
}

/// Launch interactive TUI results explorer (I/O).
fn launch_tui(analysis: crate::priority::UnifiedAnalysis) -> Result<()> {
    use crate::tui::results::ResultsExplorer;
    let mut explorer = ResultsExplorer::new(analysis)?;
    explorer.run()
}

/// Output using traditional text/JSON/markdown format (I/O).
fn output_traditional(
    analysis: crate::priority::UnifiedAnalysis,
    config: &AnalyzeConfig,
    results: &AnalysisResults,
) -> Result<()> {
    let output_config = create_output_config(config);
    output::output_unified_priorities_with_config(
        analysis,
        output_config,
        results,
        config.coverage_file.as_ref(),
    )
}

/// Create output configuration from analyze config.
fn create_output_config(config: &AnalyzeConfig) -> OutputConfig {
    OutputConfig {
        top: config.top,
        tail: config.tail,
        summary: config.summary,
        verbosity: config.verbosity,
        output_file: config.output.clone(),
        output_format: Some(config.format),
        formatting_config: config._formatting_config,
        show_filter_stats: config.show_filter_stats,
    }
}
