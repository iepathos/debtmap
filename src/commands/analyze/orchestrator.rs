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
use crate::output::unified::{
    AnalysisPolicyReceipt, AnalysisReceipt, EvidenceReceipt, ExecutionReceipt, ScopeReceipt,
    ScopeStatus, SelectionReceipt, SourceRevisionReceipt,
};
use crate::output::{self, OutputConfig};
use crate::progress::ProgressManager;
use anyhow::Result;
use std::io::IsTerminal;

// Re-export for backward compatibility
pub use project_analysis::analyze_project;

/// Main entry point - orchestrates analysis (thin wrapper).
pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    setup_analysis_environment(&config);
    let (results, unified, file_outcomes) = run_analysis_phases(&config)?;
    process_and_output_results(unified, &config, &results, &file_outcomes)
}

/// Setup analysis environment (I/O).
fn setup_analysis_environment(config: &AnalyzeConfig) {
    super::config::setup_environment(config);
    super::config::setup_progress_manager(config.verbosity);
}

/// Run analysis and build unified results (I/O).
fn run_analysis_phases(
    config: &AnalyzeConfig,
) -> Result<(
    AnalysisResults,
    crate::priority::UnifiedAnalysis,
    project_analysis::FileOutcomeSummary,
)> {
    // Spec 214: Use extraction with metrics adapter for single-pass parsing
    let output = project_analysis::run_analysis_with_extraction(config)?;
    let project_analysis::ProjectAnalysisOutput {
        results,
        extracted_data,
        file_outcomes,
    } = output;
    let mut unified = build_unified_analysis_options(config, &results, extracted_data)?;

    pipeline::apply_file_context(&mut unified, &results.file_contexts);
    let filtered = pipeline::filter_by_categories(unified, config.filter_categories.as_deref());

    Ok((results, filtered, file_outcomes))
}

/// Build unified analysis from results (I/O).
fn build_unified_analysis_options(
    config: &AnalyzeConfig,
    results: &AnalysisResults,
    extracted_data: Option<
        std::collections::HashMap<std::path::PathBuf, crate::extraction::ExtractedFileData>,
    >,
) -> Result<crate::priority::UnifiedAnalysis> {
    // Spec 214: Reuse pre-extracted data from metrics phase (no re-extraction needed)
    let options = create_analysis_options_with_extracted(config, results, extracted_data);
    unified_analysis::perform_unified_analysis_with_options(options)
}

/// Create analysis options from config with pre-extracted data (spec 213).
fn create_analysis_options_with_extracted<'a>(
    config: &'a AnalyzeConfig,
    results: &'a AnalysisResults,
    extracted_data: Option<
        std::collections::HashMap<std::path::PathBuf, crate::extraction::ExtractedFileData>,
    >,
) -> unified_analysis::UnifiedAnalysisOptions<'a> {
    // Extract Rust files from already-discovered file contexts (avoids re-walking filesystem)
    let rust_files = extract_rust_files(results);

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
        rust_files: Some(rust_files),
        extracted_data, // Spec 213: Pre-extracted data for parallel analysis
        reference_time: config.reference_time,
    }
}

/// Extract Rust file paths from analysis results (pure).
fn extract_rust_files(results: &AnalysisResults) -> Vec<std::path::PathBuf> {
    results
        .file_contexts
        .keys()
        .filter(|path| path.extension().map(|ext| ext == "rs").unwrap_or(false))
        .cloned()
        .collect()
}

/// Process results and output (I/O).
fn process_and_output_results(
    unified: crate::priority::UnifiedAnalysis,
    config: &AnalyzeConfig,
    results: &AnalysisResults,
    file_outcomes: &project_analysis::FileOutcomeSummary,
) -> Result<()> {
    run_diagnostics_if_needed(&unified, config)?;
    handle_empty_results(&unified);
    cleanup_progress();
    output_results(unified, config, results, file_outcomes)
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
    file_outcomes: &project_analysis::FileOutcomeSummary,
) -> Result<()> {
    if should_use_tui(config) {
        launch_tui(analysis)
    } else {
        output_traditional(analysis, config, results, file_outcomes)
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
    file_outcomes: &project_analysis::FileOutcomeSummary,
) -> Result<()> {
    let output_config = create_output_config(config, &analysis, file_outcomes)?;
    output::output_unified_priorities_with_config(
        analysis,
        output_config,
        results,
        config.coverage_file.as_ref(),
    )
}

/// Create output configuration from analyze config.
fn create_output_config(
    config: &AnalyzeConfig,
    analysis: &crate::priority::UnifiedAnalysis,
    file_outcomes: &project_analysis::FileOutcomeSummary,
) -> Result<OutputConfig> {
    Ok(OutputConfig {
        top: config.top,
        tail: config.tail,
        summary: config.summary,
        verbosity: config.verbosity,
        output_file: config.output.clone(),
        output_format: Some(config.format),
        formatting_config: config._formatting_config,
        show_filter_stats: config.show_filter_stats,
        analysis_receipt: Some(create_analysis_receipt(config, analysis, file_outcomes)?),
    })
}

fn create_analysis_receipt(
    config: &AnalyzeConfig,
    analysis: &crate::priority::UnifiedAnalysis,
    file_outcomes: &project_analysis::FileOutcomeSummary,
) -> Result<AnalysisReceipt> {
    let policy = create_policy_receipt(config);
    let policy_fingerprint = policy.fingerprint()?;
    Ok(AnalysisReceipt {
        analysis_target: canonical_analysis_target(config),
        source_revision: detect_source_revision(config),
        reference_time: Some(config.reference_time.to_rfc3339()),
        policy,
        policy_fingerprint,
        evidence: create_evidence_receipt(config, analysis),
        selection: create_selection_receipt(config),
        execution: create_execution_receipt(config),
        scope: create_scope_receipt(analysis, file_outcomes),
        warnings: create_receipt_warnings(config, analysis, file_outcomes),
    })
}

fn create_policy_receipt(config: &AnalyzeConfig) -> AnalysisPolicyReceipt {
    let effective = crate::config::AnalysisPolicy::from_config(crate::config::get_config());
    let selected = canonical_language_names(config);
    AnalysisPolicyReceipt {
        languages: selected.clone(),
        language_policies: effective
            .languages
            .iter()
            .map(|policy| crate::output::unified::LanguagePolicyReceipt {
                language: policy.language.to_string().to_lowercase(),
                enabled: selected
                    .iter()
                    .any(|language| language == &policy.language.to_string().to_lowercase()),
                detect_complexity: policy.features.detect_complexity,
                detect_dead_code: policy.features.detect_dead_code,
                detect_duplication: policy.features.detect_duplication,
                generated_code: format!("{:?}", policy.generated_code).to_lowercase(),
            })
            .collect(),
        complexity_threshold: config.threshold_complexity,
        duplication_threshold_lines: config.threshold_duplication,
        duplication_similarity: effective.duplication.similarity_threshold,
        threshold_preset: debug_name(config.threshold_preset),
        semantic_analysis: !config.semantic_off,
        context_aware_scoring: !config.no_context_aware,
        god_object_detection: !config.no_god_object,
        functional_analysis: config.ast_functional_analysis,
        functional_analysis_profile: debug_name(config.functional_analysis_profile),
        aggregation: !config.no_aggregation,
        aggregation_method: config.aggregation_method.clone(),
        minimum_problematic_functions: config.min_problematic,
    }
}

fn canonical_language_names(config: &AnalyzeConfig) -> Vec<String> {
    crate::utils::language_parser::parse_languages(config.languages.clone())
        .into_iter()
        .map(|language| language.to_string().to_lowercase())
        .collect()
}

fn debug_name<T: std::fmt::Debug + Copy>(value: Option<T>) -> Option<String> {
    value.map(|item| format!("{item:?}").to_lowercase())
}

fn create_evidence_receipt(
    config: &AnalyzeConfig,
    analysis: &crate::priority::UnifiedAnalysis,
) -> EvidenceReceipt {
    EvidenceReceipt {
        coverage_requested: config.coverage_file.is_some(),
        coverage_loaded: analysis.has_coverage_data,
        coverage_source_kind: config.coverage_file.as_ref().map(|_| "lcov".to_string()),
        context_requested: config.enable_context,
        context_providers_requested: config.context_providers.clone(),
        context_providers_disabled: config.disable_context.clone().unwrap_or_default(),
    }
}

fn create_selection_receipt(config: &AnalyzeConfig) -> SelectionReceipt {
    SelectionReceipt {
        minimum_score_requested: config.min_score,
        minimum_priority_requested: config.min_priority.clone(),
        categories_requested: config.filter_categories.clone().unwrap_or_default(),
        aggregate_only: config.aggregate_only,
        top: config.top,
        tail: config.tail,
        file_limit_requested: config.max_files,
    }
}

fn create_execution_receipt(config: &AnalyzeConfig) -> ExecutionReceipt {
    ExecutionReceipt {
        parallel: config.parallel,
        jobs: config.jobs,
        multi_pass: config.multi_pass,
    }
}

fn create_scope_receipt(
    analysis: &crate::priority::UnifiedAnalysis,
    outcomes: &project_analysis::FileOutcomeSummary,
) -> ScopeReceipt {
    ScopeReceipt {
        discovered_files: Some(outcomes.discovered),
        analyzed_files: outcomes.analyzed,
        failed_files: Some(outcomes.failed),
        omitted_by_limit: Some(outcomes.omitted_by_limit),
        total_loc: analysis.total_lines_of_code,
        status: scope_status(outcomes),
    }
}

fn scope_status(outcomes: &project_analysis::FileOutcomeSummary) -> ScopeStatus {
    if outcomes.omitted_by_limit > 0 {
        ScopeStatus::Limited
    } else if outcomes.failed > 0 {
        ScopeStatus::Partial
    } else {
        ScopeStatus::Complete
    }
}

fn create_receipt_warnings(
    config: &AnalyzeConfig,
    analysis: &crate::priority::UnifiedAnalysis,
    outcomes: &project_analysis::FileOutcomeSummary,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if outcomes.failed > 0 {
        warnings.push(format!(
            "{} discovered files failed analysis",
            outcomes.failed
        ));
    }
    if outcomes.omitted_by_limit > 0 {
        warnings.push(format!(
            "{} discovered files were omitted by the file limit",
            outcomes.omitted_by_limit
        ));
    }
    if config.coverage_file.is_some() && !analysis.has_coverage_data {
        warnings.push("Coverage was requested but no coverage evidence was loaded".to_string());
    }
    if config.enable_context {
        warnings.push("Context provider success is not yet retained in the receipt".to_string());
    }
    warnings
}

fn canonical_analysis_target(config: &AnalyzeConfig) -> Option<std::path::PathBuf> {
    std::fs::canonicalize(&config.path)
        .or_else(|_| std::path::absolute(&config.path))
        .ok()
}

fn detect_source_revision(config: &AnalyzeConfig) -> Option<SourceRevisionReceipt> {
    let target = canonical_analysis_target(config)?;
    let directory = if target.is_dir() {
        target
    } else {
        target.parent()?.to_path_buf()
    };
    let commit = git_output(&directory, &["rev-parse", "HEAD"])?;
    let status = git_output(&directory, &["status", "--porcelain"])?;
    Some(SourceRevisionReceipt {
        commit,
        dirty: !status.is_empty(),
    })
}

fn git_output(directory: &std::path::Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}
