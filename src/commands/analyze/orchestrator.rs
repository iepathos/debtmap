//! Orchestrator module for the analyze command.
//!
//! This module provides the main entry point and orchestrates I/O with pure functions.
//! It follows the "Shell" pattern - thin I/O composition layer.

use super::config::AnalyzeConfig;
use super::{diagnostics, pipeline};
use crate::analysis::FileContext;
use crate::builders::unified_analysis;
use crate::config::DebtmapConfig;
use crate::core::{AnalysisResults, DuplicationBlock, Language};
use crate::formatting::FormattingConfig;
use crate::output::{self, OutputConfig};
use crate::progress::ProgressManager;
use crate::tui::app::StageStatus;
use crate::utils::{analysis_helpers, language_parser};
use crate::{analysis_utils, io};
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// Main entry point - orchestrates analysis (thin wrapper).
pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    // Setup phase (I/O)
    super::config::setup_environment(&config);
    super::config::setup_progress_manager(config.verbosity);

    // Analysis phase (I/O)
    let results = run_analysis(&config)?;
    let mut unified = build_unified_analysis(&config, &results)?;

    // Transform phase (pure)
    pipeline::apply_file_context(&mut unified, &results.file_contexts);
    let filtered = pipeline::filter_by_categories(unified, config.filter_categories.as_deref());

    // Diagnostics phase (I/O)
    run_diagnostics_if_needed(&filtered, &config)?;

    // Check results (pure + I/O)
    handle_empty_results(&filtered);

    // Cleanup TUI (I/O)
    cleanup_progress();

    // Output phase (I/O)
    output_results(filtered, &config, &results)
}

/// Run project analysis (I/O).
fn run_analysis(config: &AnalyzeConfig) -> Result<AnalysisResults> {
    let languages = language_parser::parse_languages(config.languages.clone());
    analyze_project(
        config.path.clone(),
        languages,
        config.threshold_complexity,
        config.threshold_duplication,
        config.parallel,
        config._formatting_config,
    )
}

/// Build unified analysis from results (I/O).
fn build_unified_analysis(
    config: &AnalyzeConfig,
    results: &AnalysisResults,
) -> Result<crate::priority::UnifiedAnalysis> {
    unified_analysis::perform_unified_analysis_with_options(
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
        },
    )
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
        eprintln!("{}", info.message);
        eprintln!("Try adjusting filters:");
        eprintln!("  - Use --min-score <value> to lower the score threshold");
        eprintln!(
            "  - Current min_score threshold: {}",
            info.current_threshold
        );
        eprintln!("  - Use DEBTMAP_MIN_SCORE_THRESHOLD=0 to see all items");
    }
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
    let is_terminal = std::io::stdout().is_terminal();
    let is_ci = std::env::var("CI").is_ok();
    let use_tui = pipeline::should_use_tui(
        config.no_tui,
        config.format,
        &config.output,
        is_terminal,
        is_ci,
    );

    if use_tui {
        launch_tui(analysis)
    } else {
        output_traditional(analysis, config, results)
    }
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
    let output_config = OutputConfig {
        top: config.top,
        tail: config.tail,
        summary: config.summary,
        verbosity: config.verbosity,
        output_file: config.output.clone(),
        output_format: Some(config.format),
        formatting_config: config._formatting_config,
        show_filter_stats: config.show_filter_stats,
    };

    output::output_unified_priorities_with_config(
        analysis,
        output_config,
        results,
        config.coverage_file.as_ref(),
    )
}

/// Analyze project and return results (I/O).
pub fn analyze_project(
    path: PathBuf,
    languages: Vec<Language>,
    complexity_threshold: u32,
    duplication_threshold: usize,
    parallel_enabled: bool,
    formatting_config: FormattingConfig,
) -> Result<AnalysisResults> {
    setup_parallel_env(parallel_enabled);
    let config = crate::config::get_config();
    init_global_progress();

    // Phase 1: files parse
    start_files_phase();

    let files = discover_files(&path, &languages, config)?;
    update_file_count(files.len());

    configure_project_size(&files, parallel_enabled, formatting_config)?;

    let file_metrics = parse_files(&files);
    complete_parsing(files.len());

    let (all_functions, all_debt_items) = extract_data(&file_metrics);
    let file_contexts = analysis_utils::extract_file_contexts(&file_metrics);

    let duplications = detect_duplications(&files, duplication_threshold);
    complete_files_phase(files.len());

    Ok(build_results(
        path,
        &all_functions,
        all_debt_items,
        duplications.clone(),
        file_contexts,
        complexity_threshold,
        &file_metrics,
    ))
}

/// Set up parallel processing environment variable.
fn setup_parallel_env(parallel_enabled: bool) {
    if parallel_enabled {
        std::env::set_var("DEBTMAP_PARALLEL", "true");
    }
}

/// Initialize global progress tracker.
fn init_global_progress() {
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();
    if !quiet_mode {
        io::progress::AnalysisProgress::init_global();
    }
}

/// Start files phase tracking.
fn start_files_phase() {
    io::progress::AnalysisProgress::with_global(|p| p.start_phase(0));
    if let Some(manager) = ProgressManager::global() {
        manager.tui_start_stage(0);
        manager.tui_update_subtask(0, 0, StageStatus::Active, None);
    }
}

/// Discover project files.
fn discover_files(
    path: &Path,
    languages: &[Language],
    config: &DebtmapConfig,
) -> Result<Vec<PathBuf>> {
    let files = io::walker::find_project_files_with_config(path, languages.to_vec(), config)
        .context("Failed to find project files")?;

    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 0, StageStatus::Completed, None);
        std::thread::sleep(std::time::Duration::from_millis(150));
        manager.tui_update_subtask(0, 1, StageStatus::Active, None);
    }

    Ok(files)
}

/// Update progress with file count.
fn update_file_count(count: usize) {
    io::progress::AnalysisProgress::with_global(|p| {
        p.update_progress(io::progress::PhaseProgress::Count(count));
    });
}

/// Configure project size optimizations.
fn configure_project_size(
    files: &[PathBuf],
    parallel_enabled: bool,
    _formatting_config: FormattingConfig,
) -> Result<()> {
    analyze_and_configure_project_size(files, parallel_enabled, _formatting_config)
}

/// Parse files and collect metrics.
fn parse_files(files: &[PathBuf]) -> Vec<crate::core::FileMetrics> {
    analysis_utils::collect_file_metrics(files)
}

/// Complete parsing phase.
fn complete_parsing(file_count: usize) {
    io::progress::AnalysisProgress::with_global(|p| {
        p.update_progress(io::progress::PhaseProgress::Progress {
            current: file_count,
            total: file_count,
        });
        p.complete_phase();
    });

    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 1, StageStatus::Completed, None);
        std::thread::sleep(std::time::Duration::from_millis(150));
        manager.tui_update_subtask(0, 2, StageStatus::Active, None);
    }
}

/// Extract functions and debt items from file metrics.
fn extract_data(
    file_metrics: &[crate::core::FileMetrics],
) -> (
    Vec<crate::core::FunctionMetrics>,
    Vec<crate::core::DebtItem>,
) {
    let all_functions = analysis_utils::extract_all_functions(file_metrics);
    let all_debt_items = analysis_utils::extract_all_debt_items(file_metrics);

    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_counts(all_functions.len(), all_debt_items.len());
        manager.tui_update_subtask(0, 2, StageStatus::Completed, None);
        std::thread::sleep(std::time::Duration::from_millis(150));
        manager.tui_update_subtask(0, 3, StageStatus::Active, Some((0, 0)));
    }

    (all_functions, all_debt_items)
}

/// Detect code duplications.
fn detect_duplications(files: &[PathBuf], threshold: usize) -> Vec<DuplicationBlock> {
    let file_count = files.len();
    let duplications =
        analysis_helpers::detect_duplications_with_progress(files, threshold, |current, total| {
            if let Some(manager) = ProgressManager::global() {
                manager.tui_update_subtask(0, 3, StageStatus::Active, Some((current, total)));
            }
        });

    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 3, StageStatus::Completed, Some((file_count, file_count)));
    }

    duplications
}

/// Complete files phase.
fn complete_files_phase(file_count: usize) {
    if let Some(manager) = ProgressManager::global() {
        manager.tui_complete_stage(0, format!("{} files parsed", file_count));
        manager.tui_set_progress(0.22);
    }
}

/// Build analysis results.
fn build_results(
    path: PathBuf,
    all_functions: &[crate::core::FunctionMetrics],
    all_debt_items: Vec<crate::core::DebtItem>,
    duplications: Vec<DuplicationBlock>,
    file_contexts: HashMap<PathBuf, FileContext>,
    complexity_threshold: u32,
    file_metrics: &[crate::core::FileMetrics],
) -> AnalysisResults {
    let complexity_report =
        analysis_helpers::build_complexity_report(all_functions, complexity_threshold);
    let technical_debt =
        analysis_helpers::build_technical_debt_report(all_debt_items, duplications.clone());
    let dependencies = analysis_helpers::create_dependency_report(file_metrics);

    AnalysisResults {
        project_path: path,
        timestamp: Utc::now(),
        complexity: complexity_report,
        technical_debt,
        dependencies,
        duplications,
        file_contexts,
    }
}

/// Analyze project size and configure optimizations.
fn analyze_and_configure_project_size(
    files: &[PathBuf],
    parallel_enabled: bool,
    _formatting_config: FormattingConfig,
) -> Result<()> {
    let file_count = files.len();
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();

    if quiet_mode {
        return Ok(());
    }

    log_project_size_info(file_count, parallel_enabled);
    configure_large_project_env(file_count);

    Ok(())
}

/// Log project size information.
fn log_project_size_info(file_count: usize, parallel_enabled: bool) {
    match file_count {
        0..=100 => log::info!("Analyzing {} files (small project)", file_count),
        101..=500 => {
            log::info!("Analyzing {} files (medium project)", file_count);
            log_parallel_status(parallel_enabled);
        }
        501..=1000 => log::info!("Analyzing {} files (large project)", file_count),
        1001..=2000 => log::info!("Analyzing {} files (very large project)", file_count),
        _ => log_massive_project(file_count),
    }
}

/// Log parallel processing status.
fn log_parallel_status(parallel_enabled: bool) {
    if parallel_enabled {
        log::info!("Parallel processing enabled for better performance");
    } else {
        log::warn!("Using sequential processing (use default for better performance)");
    }
}

/// Log massive project warnings.
fn log_massive_project(file_count: usize) {
    log::warn!("Analyzing {} files (massive project)", file_count);
    log::warn!("Consider using .debtmapignore to exclude test/vendor directories");
    log::warn!("Focus analysis on specific modules with targeted paths");
}

/// Configure environment for large projects.
fn configure_large_project_env(file_count: usize) {
    if file_count > 500 {
        std::env::set_var("RUST_BACKTRACE", "0");
    }
}
