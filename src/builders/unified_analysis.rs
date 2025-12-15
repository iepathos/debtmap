//! Unified analysis orchestration with progress reporting.
//!
//! This module provides the entry points for unified analysis with progress/TUI
//! handling. All pure computation is delegated to `unified_analysis_phases`.
//!
//! Following Stillwater philosophy: Pure core (phases/), imperative shell (this file).

use super::{call_graph, parallel_call_graph, parallel_unified_analysis};
use crate::observability::{set_phase_persistent, set_progress, AnalysisPhase};
use tracing::{debug, info, info_span, warn};

// Re-export pure core modules
pub use super::unified_analysis_phases as core;

// Re-export types for backward compatibility
pub use super::unified_analysis_phases::options::UnifiedAnalysisOptions;
pub use super::unified_analysis_phases::phases::god_object::{
    analyze_file_git_context, create_god_object_debt_item,
};
pub use super::unified_analysis_phases::phases::scoring::create_debt_items_from_metric;

use crate::analyzers::call_graph_integration;
use crate::core::AnalysisResults;
use crate::priority::{
    call_graph::{CallGraph, FunctionId},
    debt_aggregator::DebtAggregator,
    UnifiedAnalysis, UnifiedAnalysisUtils, UnifiedDebtItem,
};
use crate::risk;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Main entry point for unified analysis (simple version).
pub fn perform_unified_analysis(
    results: &AnalysisResults,
    coverage_file: Option<&PathBuf>,
    semantic_off: bool,
    project_path: &Path,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
) -> Result<UnifiedAnalysis> {
    perform_unified_analysis_with_options(UnifiedAnalysisOptions {
        results,
        coverage_file,
        semantic_off,
        project_path,
        verbose_macro_warnings,
        show_macro_stats,
        parallel: false,
        jobs: 0,
        multi_pass: false,
        show_attribution: false,
        aggregate_only: false,
        no_aggregation: false,
        aggregation_method: None,
        min_problematic: None,
        no_god_object: false,
        suppress_coverage_tip: false,
        _formatting_config: crate::formatting::FormattingConfig::from_env(),
        enable_context: false,
        context_providers: None,
        disable_context: None,
        rust_files: None,     // Fallback to file discovery
        extracted_data: None, // Fallback to per-function parsing (spec 213)
    })
}

/// Main entry point for unified analysis with full options.
pub fn perform_unified_analysis_with_options(
    options: UnifiedAnalysisOptions,
) -> Result<UnifiedAnalysis> {
    let UnifiedAnalysisOptions {
        results,
        coverage_file,
        semantic_off: _,
        project_path,
        verbose_macro_warnings,
        show_macro_stats,
        parallel,
        jobs,
        multi_pass: _,
        show_attribution: _,
        aggregate_only: _,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
        suppress_coverage_tip,
        _formatting_config,
        enable_context,
        context_providers,
        disable_context,
        rust_files,
        extracted_data,
    } = options;

    // Create top-level span for unified analysis (spec 208)
    let span = info_span!(
        "unified_analysis",
        project = %project_path.display(),
        file_count = results.complexity.metrics.len(),
        parallel = parallel,
    );
    let _guard = span.enter();

    info!(
        file_count = results.complexity.metrics.len(),
        "Starting unified analysis"
    );

    // Set total file count for crash report progress tracking (spec 207)
    set_progress(0, results.complexity.metrics.len());

    // Build call graph with progress reporting
    let mut call_graph = call_graph::build_initial_call_graph(&results.complexity.metrics);

    // Progress: Call graph stage
    report_stage_start(1);
    let call_graph_start = std::time::Instant::now();

    let (framework_exclusions, function_pointer_used_functions) = {
        let _span = info_span!("call_graph_building").entered();
        info!("Building call graph");

        // Spec 214: Use extraction adapters when extracted data is available
        let result = if let Some(ref extracted) = extracted_data {
            info!("Building call graph from extracted data (spec 214)");
            let (graph, exclusions, fn_pointers) =
                parallel_call_graph::build_call_graph_from_extracted(call_graph.clone(), extracted);
            call_graph = graph;
            (exclusions, fn_pointers)
        } else if parallel {
            build_call_graph_with_progress(
                project_path,
                &mut call_graph,
                jobs,
                true,
                rust_files.as_deref(),
            )?
        } else {
            build_call_graph_with_progress_sequential(
                project_path,
                &mut call_graph,
                verbose_macro_warnings,
                show_macro_stats,
                rust_files.as_deref(),
            )?
        };

        debug!(functions = call_graph.node_count(), "Call graph built");
        result
    };

    let call_graph_time = call_graph_start.elapsed();
    report_stage_complete(1, format!("{} functions", call_graph.node_count()));

    // Apply trait patterns
    core::phases::call_graph::apply_trait_patterns(&mut call_graph);

    // Progress: Coverage stage
    report_stage_start(2);
    let coverage_start = std::time::Instant::now();

    let coverage_data = {
        let _span = info_span!("coverage_loading").entered();
        info!("Loading coverage data");
        let data = core::phases::coverage::load_coverage_data(coverage_file.cloned())?;
        if data.is_some() {
            debug!("Coverage data loaded");
        } else {
            debug!("No coverage data provided");
        }
        data
    };
    emit_coverage_tip(coverage_data.is_none(), suppress_coverage_tip);

    let coverage_time = coverage_start.elapsed();
    let coverage_metric = if coverage_data.is_some() {
        "loaded"
    } else {
        "skipped"
    };
    report_stage_complete(2, coverage_metric);

    // Enrich metrics with call graph data
    let enriched_metrics = call_graph_integration::populate_call_graph_data(
        results.complexity.metrics.clone(),
        &call_graph,
    );

    // Progress: Purity stage
    report_stage_start(3);
    let enriched_metrics = {
        let _span = info_span!("purity_analysis").entered();
        info!("Analyzing function purity");
        let result = core::orchestration::run_purity_propagation(&enriched_metrics, &call_graph);
        debug!(functions = result.len(), "Purity analysis complete");
        result
    };
    report_stage_complete(3, format!("{} functions analyzed", enriched_metrics.len()));

    // Progress: Context stage
    report_stage_start(4);
    let risk_analyzer = {
        let _span = info_span!("context_loading").entered();
        info!("Loading context providers");
        let result = build_risk_analyzer(
            project_path,
            enable_context,
            context_providers,
            disable_context,
            results,
        );
        if result.is_some() {
            debug!("Context providers loaded");
        } else {
            debug!("Context analysis disabled or not available");
        }
        result
    };
    let context_metric = if enable_context { "loaded" } else { "skipped" };
    report_stage_complete(4, context_metric);

    // Progress: Debt scoring stage
    report_stage_start(5);

    let result = {
        let _span = info_span!("debt_scoring").entered();
        info!("Scoring technical debt items");
        let result = create_unified_analysis_with_exclusions_and_timing(
            &enriched_metrics,
            &call_graph,
            coverage_data.as_ref(),
            &framework_exclusions,
            Some(&function_pointer_used_functions),
            Some(&results.technical_debt.items),
            no_aggregation,
            aggregation_method,
            min_problematic,
            no_god_object,
            call_graph_time,
            coverage_time,
            risk_analyzer,
            project_path,
            parallel,
            jobs,
            extracted_data,
        );
        debug!(
            item_count = result.items.len(),
            file_items = result.file_items.len(),
            "Debt scoring complete"
        );
        result
    };

    report_stage_complete(5, format!("{} items scored", result.items.len()));

    info!(
        total_items = result.items.len(),
        file_items = result.file_items.len(),
        "Unified analysis complete"
    );

    Ok(result)
}

/// Create unified analysis with exclusions (compatibility wrapper).
#[allow(clippy::too_many_arguments)]
pub fn create_unified_analysis_with_exclusions(
    metrics: &[crate::core::FunctionMetrics],
    call_graph: &CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_items: Option<&[crate::core::DebtItem]>,
    no_aggregation: bool,
    aggregation_method: Option<String>,
    min_problematic: Option<usize>,
    no_god_object: bool,
) -> UnifiedAnalysis {
    create_unified_analysis_with_exclusions_and_timing(
        metrics,
        call_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        debt_items,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
        std::time::Duration::from_secs(0),
        std::time::Duration::from_secs(0),
        None,
        Path::new("."),
        false,
        0,
        None, // No extracted data - fallback to per-function parsing (spec 213)
    )
}

/// Create debt item from metric (compatibility wrapper for parallel_unified_analysis).
#[allow(clippy::too_many_arguments)]
pub(super) fn create_debt_item_from_metric_with_aggregator(
    metric: &crate::core::FunctionMetrics,
    call_graph: &CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
    risk_analyzer: Option<&risk::RiskAnalyzer>,
    project_path: &Path,
) -> Vec<UnifiedDebtItem> {
    // Create empty cache for backward compatibility (will use fallback reads)
    let empty_cache = std::collections::HashMap::new();
    // Create detectors for backward compatibility (spec 196: ideally shared at higher level)
    let context_detector = crate::analysis::ContextDetector::new();
    let recommendation_engine = crate::priority::scoring::ContextRecommendationEngine::new();
    core::phases::scoring::create_debt_items_from_metric(
        metric,
        call_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        debt_aggregator,
        data_flow,
        risk_analyzer,
        project_path,
        &empty_cache,
        &context_detector,
        &recommendation_engine,
    )
}

// --- Internal implementation ---

#[allow(clippy::too_many_arguments)]
fn create_unified_analysis_with_exclusions_and_timing(
    metrics: &[crate::core::FunctionMetrics],
    call_graph: &CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_items: Option<&[crate::core::DebtItem]>,
    _no_aggregation: bool,
    _aggregation_method: Option<String>,
    _min_problematic: Option<usize>,
    no_god_object: bool,
    call_graph_time: std::time::Duration,
    coverage_time: std::time::Duration,
    risk_analyzer: Option<risk::RiskAnalyzer>,
    project_path: &Path,
    parallel: bool,
    jobs: usize,
    extracted_data: Option<
        std::collections::HashMap<PathBuf, crate::extraction::ExtractedFileData>,
    >,
) -> UnifiedAnalysis {
    // Use parallel path if enabled
    let parallel_enabled = parallel
        || std::env::var("DEBTMAP_PARALLEL")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

    if parallel_enabled {
        return create_parallel_analysis(
            metrics,
            call_graph,
            coverage_data,
            framework_exclusions,
            function_pointer_used_functions,
            debt_items,
            no_god_object,
            jobs,
            call_graph_time,
            coverage_time,
            risk_analyzer,
            project_path,
            extracted_data,
        );
    }

    // Sequential path using pure functions
    let start = std::time::Instant::now();

    let mut unified = UnifiedAnalysis::new(call_graph.clone());
    unified.populate_purity_analysis(metrics);

    let test_only_functions = core::phases::call_graph::find_test_only_functions(call_graph);
    let debt_aggregator = core::phases::scoring::setup_debt_aggregator(metrics, debt_items);
    let data_flow = crate::data_flow::DataFlowGraph::from_call_graph(call_graph.clone());

    // Build file line count cache (spec 195: I/O at boundary, once per unique file)
    let file_line_counts = core::phases::scoring::build_file_line_count_cache(metrics);

    // Process metrics to debt items (uses cached file line counts)
    let items = core::phases::scoring::process_metrics_to_debt_items(
        metrics,
        call_graph,
        &test_only_functions,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        &debt_aggregator,
        Some(&data_flow),
        risk_analyzer.as_ref(),
        project_path,
        &file_line_counts,
    );

    for item in items {
        unified.add_item(item);
    }

    // File analysis
    process_file_analysis(
        &mut unified,
        metrics,
        coverage_data,
        no_god_object,
        risk_analyzer.as_ref(),
        project_path,
    );

    // Finalize
    unified.sort_by_priority();
    unified.calculate_total_impact();
    unified.has_coverage_data = coverage_data.is_some();

    if let Some(lcov) = coverage_data {
        unified.overall_coverage = Some(lcov.get_overall_coverage());
    }

    unified.timings = Some(parallel_unified_analysis::AnalysisPhaseTimings {
        call_graph_building: call_graph_time,
        trait_resolution: std::time::Duration::from_secs(0),
        coverage_loading: coverage_time,
        data_flow_creation: std::time::Duration::from_secs(0),
        purity_analysis: std::time::Duration::from_secs(0),
        test_detection: std::time::Duration::from_secs(0),
        debt_aggregation: std::time::Duration::from_secs(0),
        function_analysis: std::time::Duration::from_secs(0),
        file_analysis: std::time::Duration::from_secs(0),
        aggregation: std::time::Duration::from_secs(0),
        sorting: std::time::Duration::from_secs(0),
        total: start.elapsed(),
    });

    unified
}

#[allow(clippy::too_many_arguments)]
fn create_parallel_analysis(
    metrics: &[crate::core::FunctionMetrics],
    call_graph: &CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_items: Option<&[crate::core::DebtItem]>,
    no_god_object: bool,
    jobs: usize,
    call_graph_time: std::time::Duration,
    coverage_time: std::time::Duration,
    risk_analyzer: Option<risk::RiskAnalyzer>,
    project_path: &Path,
    extracted_data: Option<
        std::collections::HashMap<PathBuf, crate::extraction::ExtractedFileData>,
    >,
) -> UnifiedAnalysis {
    use parallel_unified_analysis::{
        ParallelUnifiedAnalysisBuilder, ParallelUnifiedAnalysisOptions,
    };

    let options = ParallelUnifiedAnalysisOptions {
        parallel: true,
        jobs: if jobs > 0 { Some(jobs) } else { None },
        batch_size: 100,
        progress: std::env::var("DEBTMAP_QUIET").is_err(),
    };

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph.clone(), options)
        .with_project_path(project_path.to_path_buf());

    // Use pre-extracted data when available (spec 213)
    // This prevents proc-macro2 SourceMap overflow on large codebases
    if let Some(extracted) = extracted_data {
        builder = builder.with_extracted_data(extracted);
    }

    if let Some(analyzer) = risk_analyzer {
        builder = builder.with_risk_analyzer(analyzer);
    }

    builder.set_preliminary_timings(call_graph_time, coverage_time);

    let (data_flow_graph, purity, test_only_functions, debt_aggregator) =
        builder.execute_phase1_parallel(metrics, debt_items);

    let enriched_metrics =
        call_graph_integration::populate_call_graph_data(metrics.to_vec(), call_graph);

    let items = builder.execute_phase2_parallel(
        &enriched_metrics,
        &test_only_functions,
        &debt_aggregator,
        &data_flow_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
    );

    let file_items =
        builder.execute_phase3_parallel(&enriched_metrics, coverage_data, no_god_object);

    let (mut unified, timings) =
        builder.build(data_flow_graph, purity, items, file_items, coverage_data);

    unified.timings = Some(timings);
    unified
}

fn process_file_analysis(
    unified: &mut UnifiedAnalysis,
    metrics: &[crate::core::FunctionMetrics],
    coverage_data: Option<&risk::lcov::LcovData>,
    no_god_object: bool,
    risk_analyzer: Option<&risk::RiskAnalyzer>,
    project_path: &Path,
) {
    use crate::metrics::loc_counter::LocCounter;
    use crate::priority::god_object_aggregation::{
        aggregate_coverage_from_raw_metrics, aggregate_from_raw_metrics,
    };

    let file_groups = core::phases::file_analysis::group_functions_by_file(metrics);
    let loc_counter = LocCounter::default();

    for file_path in file_groups.keys() {
        if let Ok(loc_count) = loc_counter.count_file(file_path) {
            unified.register_analyzed_file(file_path.clone(), loc_count.physical_lines);
        }
    }

    for (file_path, functions) in file_groups {
        let file_content = std::fs::read_to_string(&file_path).ok();

        let mut processed = core::phases::file_analysis::process_file_metrics(
            file_path.clone(),
            functions,
            file_content.as_deref(),
            coverage_data,
            no_god_object,
            project_path,
        );

        // Clear function_scores for consistency with parallel path
        // (function scores come from scored debt items, not raw complexity aggregation)
        processed.file_metrics.function_scores = Vec::new();

        // Create file_item to get context-adjusted score (consistent with parallel path)
        let file_item = core::phases::file_analysis::create_file_debt_item(
            processed.file_metrics.clone(),
            Some(&processed.file_context),
        );

        let has_god_object = processed
            .god_analysis
            .as_ref()
            .is_some_and(|a| a.is_god_object);

        // Use adjusted score for threshold check (same as parallel path)
        if file_item.score > 50.0 || has_god_object {
            if let Some(god_analysis) = &processed.god_analysis {
                let mut aggregated = aggregate_from_raw_metrics(&processed.raw_functions);

                if let Some(lcov) = coverage_data {
                    aggregated.weighted_coverage =
                        aggregate_coverage_from_raw_metrics(&processed.raw_functions, lcov);
                }

                if let Some(analyzer) = risk_analyzer {
                    aggregated.aggregated_contextual_risk =
                        core::phases::god_object::analyze_file_git_context(
                            &processed.file_path,
                            analyzer,
                            &processed.project_root,
                        );
                }

                let enriched = core::phases::god_object::enrich_god_analysis_with_aggregates(
                    god_analysis,
                    &aggregated,
                );

                for item in unified.items.iter_mut() {
                    if item.location.file == processed.file_path {
                        item.god_object_indicators = Some(enriched.clone());
                    }
                }

                let god_item = core::phases::god_object::create_god_object_debt_item(
                    &processed.file_path,
                    &processed.file_metrics,
                    &enriched,
                    aggregated,
                    coverage_data,
                );
                unified.add_item(god_item);
            }

            // Use the already-created file_item (score already checked above)
            unified.add_file_item(file_item);
        }
    }
}

// --- Progress reporting helpers ---

/// Map TUI stage numbers to observability phases (spec 207)
fn stage_to_phase(stage: usize) -> Option<AnalysisPhase> {
    match stage {
        1 => Some(AnalysisPhase::CallGraphBuilding),
        2 => Some(AnalysisPhase::CoverageLoading),
        3 => Some(AnalysisPhase::PurityAnalysis),
        4 | 5 => Some(AnalysisPhase::DebtScoring),
        _ => None,
    }
}

fn report_stage_start(stage: usize) {
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_start_stage(stage);
    }
    // Also update unified progress for call graph stage (stage 1 -> phase 1)
    if stage == 1 {
        crate::io::progress::AnalysisProgress::with_global(|p| p.start_phase(1));
    }

    // Set observability phase for crash reports (spec 207)
    // The phase persists until overwritten by the next stage
    if let Some(phase) = stage_to_phase(stage) {
        set_phase_persistent(phase);
    }
}

fn report_stage_complete(stage: usize, metric: impl Into<String>) {
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_complete_stage(stage, metric.into());
        // Update overall progress: 6 stages total (0-5), each completion adds ~16.67%
        // Stage 0 is handled by project_analysis.rs, so we handle stages 1-5 here
        let progress = ((stage + 1) as f64) / 6.0;
        manager.tui_set_progress(progress);
    }
    // Also update unified progress for call graph stage (stage 1 -> phase 1)
    if stage == 1 {
        crate::io::progress::AnalysisProgress::with_global(|p| p.complete_phase());
    }
}

fn emit_coverage_tip(no_coverage: bool, suppress: bool) {
    let quiet = std::env::var("DEBTMAP_QUIET").is_ok();
    let tui_active = crate::progress::ProgressManager::global().is_some();

    if no_coverage && !quiet && !suppress && !tui_active {
        // Use tracing for structured logging instead of eprintln!
        warn!(
            "Coverage data not provided. Analysis will focus on complexity and code smells. \
             For test gap detection, provide coverage with: --lcov-file coverage.info"
        );
    }
}

fn build_call_graph_with_progress(
    project_path: &Path,
    call_graph: &mut CallGraph,
    jobs: usize,
    _parallel: bool,
    rust_files: Option<&[PathBuf]>,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    use crate::builders::parallel_call_graph::CallGraphPhase;
    use crate::tui::app::StageStatus;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let thread_count = if jobs == 0 { None } else { Some(jobs) };

    // Track the last active subtask to mark it completed on phase transition
    let last_subtask = AtomicUsize::new(usize::MAX);

    let (graph, exclusions, used_funcs) =
        parallel_call_graph::build_call_graph_parallel_with_files(
            project_path,
            call_graph.clone(),
            thread_count,
            rust_files,
            |progress| {
                // Map CallGraphPhase to TUI subtask index (stage 1)
                // Note: DiscoveringFiles is skipped (files reused from stage 0)
                // Subtasks: 0=parse ASTs, 1=extract calls, 2=link modules
                let subtask_index = match progress.phase {
                    CallGraphPhase::DiscoveringFiles => return, // Skip - not shown in TUI
                    CallGraphPhase::ParsingASTs => 0,
                    CallGraphPhase::ExtractingCalls => 1,
                    CallGraphPhase::LinkingModules => 2,
                };

                if let Some(manager) = crate::progress::ProgressManager::global() {
                    // Mark previous subtask as completed if we moved to a new phase
                    let prev = last_subtask.swap(subtask_index, Ordering::Relaxed);
                    if prev != usize::MAX && prev != subtask_index {
                        manager.tui_update_subtask(1, prev, StageStatus::Completed, None);
                    }

                    let progress_info = if progress.total > 0 {
                        Some((progress.current, progress.total))
                    } else {
                        None
                    };
                    manager.tui_update_subtask(
                        1,
                        subtask_index,
                        StageStatus::Active,
                        progress_info,
                    );
                }
            },
        )?;

    // Mark the last subtask as completed
    if let Some(manager) = crate::progress::ProgressManager::global() {
        let last = last_subtask.load(std::sync::atomic::Ordering::Relaxed);
        if last != usize::MAX {
            manager.tui_update_subtask(1, last, StageStatus::Completed, None);
        }
    }

    *call_graph = graph;
    Ok((exclusions, used_funcs))
}

fn build_call_graph_with_progress_sequential(
    project_path: &Path,
    call_graph: &mut CallGraph,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    rust_files: Option<&[PathBuf]>,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    use crate::builders::parallel_call_graph::CallGraphPhase;
    use crate::tui::app::StageStatus;
    use std::cell::Cell;

    // Track the last active subtask to mark it completed on phase transition
    let last_subtask = Cell::new(usize::MAX);

    let result = call_graph::process_rust_files_for_call_graph_with_files(
        project_path,
        call_graph,
        verbose_macro_warnings,
        show_macro_stats,
        rust_files,
        |progress| {
            // Map CallGraphPhase to TUI subtask index (stage 1)
            // Note: DiscoveringFiles is skipped (files reused from stage 0)
            // Subtasks: 0=parse ASTs, 1=extract calls, 2=link modules
            let subtask_index = match progress.phase {
                CallGraphPhase::DiscoveringFiles => return, // Skip - not shown in TUI
                CallGraphPhase::ParsingASTs => 0,
                CallGraphPhase::ExtractingCalls => 1,
                CallGraphPhase::LinkingModules => 2,
            };

            if let Some(manager) = crate::progress::ProgressManager::global() {
                // Mark previous subtask as completed if we moved to a new phase
                let prev = last_subtask.replace(subtask_index);
                if prev != usize::MAX && prev != subtask_index {
                    manager.tui_update_subtask(1, prev, StageStatus::Completed, None);
                }

                let progress_info = if progress.total > 0 {
                    Some((progress.current, progress.total))
                } else {
                    None
                };
                manager.tui_update_subtask(1, subtask_index, StageStatus::Active, progress_info);
            }
        },
    );

    // Mark the last subtask as completed
    if let Some(manager) = crate::progress::ProgressManager::global() {
        let last = last_subtask.get();
        if last != usize::MAX {
            manager.tui_update_subtask(1, last, StageStatus::Completed, None);
        }
    }

    result
}

fn build_risk_analyzer(
    project_path: &Path,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    results: &AnalysisResults,
) -> Option<risk::RiskAnalyzer> {
    if !enable_context {
        return None;
    }

    let aggregator = crate::utils::risk_analyzer::build_context_aggregator(
        project_path,
        enable_context,
        context_providers,
        disable_context,
    )?;

    let debt_score = crate::debt::total_debt_score(&results.technical_debt.items) as f64;
    Some(
        risk::RiskAnalyzer::default()
            .with_debt_context(debt_score, 100.0)
            .with_context_aggregator(aggregator),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_file_git_context_returns_none_when_no_context() {
        let risk_analyzer = risk::RiskAnalyzer::default();
        let file_path = PathBuf::from("src/test.rs");
        let project_root = PathBuf::from("/tmp/test");

        let result = analyze_file_git_context(&file_path, &risk_analyzer, &project_root);
        assert!(result.is_none());
    }
}
