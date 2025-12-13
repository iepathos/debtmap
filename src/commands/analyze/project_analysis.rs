//! Project analysis module for the analyze command.
//!
//! This module handles the core project analysis logic including file discovery,
//! parsing, and metrics extraction. Follows the Shell pattern for I/O operations.

use crate::analysis::FileContext;
use crate::config::DebtmapConfig;
use crate::core::{AnalysisResults, DuplicationBlock, FileMetrics, FunctionMetrics, Language};
use crate::formatting::FormattingConfig;
use crate::io;
use crate::progress::ProgressManager;
use crate::tui::app::StageStatus;
use crate::utils::{analysis_helpers, language_parser};
use crate::{analysis_utils, core::DebtItem};
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::config::AnalyzeConfig;

/// Run project analysis (I/O).
pub fn run_analysis(config: &AnalyzeConfig) -> Result<AnalysisResults> {
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

    start_files_phase();

    let files = discover_files(&path, &languages, config)?;
    let file_metrics = parse_and_extract_metrics(&files, parallel_enabled, formatting_config)?;
    let (all_functions, all_debt_items, file_contexts) = extract_analysis_data(&file_metrics);

    let duplications = detect_duplications(&files, duplication_threshold);
    complete_files_phase(files.len());

    Ok(build_analysis_results(
        path,
        all_functions,
        all_debt_items,
        duplications,
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

/// Parse files and extract metrics with progress tracking.
fn parse_and_extract_metrics(
    files: &[PathBuf],
    parallel_enabled: bool,
    formatting_config: FormattingConfig,
) -> Result<Vec<FileMetrics>> {
    update_file_count(files.len());
    configure_project_size(files, parallel_enabled, formatting_config)?;

    let file_metrics = analysis_utils::collect_file_metrics(files);
    complete_parsing(files.len());

    Ok(file_metrics)
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
    let file_count = files.len();
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();

    if !quiet_mode {
        log_project_size_info(file_count, parallel_enabled);
        configure_large_project_env(file_count);
    }

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

/// Extract functions, debt items, and file contexts from metrics.
fn extract_analysis_data(
    file_metrics: &[FileMetrics],
) -> (Vec<FunctionMetrics>, Vec<DebtItem>, HashMap<PathBuf, FileContext>) {
    let all_functions = analysis_utils::extract_all_functions(file_metrics);
    let all_debt_items = analysis_utils::extract_all_debt_items(file_metrics);
    let file_contexts = analysis_utils::extract_file_contexts(file_metrics);

    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_counts(all_functions.len(), all_debt_items.len());
        manager.tui_update_subtask(0, 2, StageStatus::Completed, None);
        std::thread::sleep(std::time::Duration::from_millis(150));
        manager.tui_update_subtask(0, 3, StageStatus::Active, Some((0, 0)));
    }

    (all_functions, all_debt_items, file_contexts)
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

/// Build analysis results from collected data.
fn build_analysis_results(
    path: PathBuf,
    all_functions: Vec<FunctionMetrics>,
    all_debt_items: Vec<DebtItem>,
    duplications: Vec<DuplicationBlock>,
    file_contexts: HashMap<PathBuf, FileContext>,
    complexity_threshold: u32,
    file_metrics: &[FileMetrics],
) -> AnalysisResults {
    let complexity_report =
        analysis_helpers::build_complexity_report(&all_functions, complexity_threshold);
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
