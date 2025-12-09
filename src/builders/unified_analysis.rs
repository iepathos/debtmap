use super::{call_graph, parallel_call_graph, parallel_unified_analysis};
use crate::{
    analysis::diagnostics::{DetailLevel, DiagnosticReporter, OutputFormat},
    analysis::multi_pass::{analyze_with_attribution, MultiPassOptions, MultiPassResult},
    analyzers::{call_graph_integration, FileAnalyzer},
    core::{AnalysisResults, DebtItem, FunctionMetrics, Language},
    priority::{
        self,
        call_graph::{CallGraph, FunctionId},
        debt_aggregator::DebtAggregator,
        debt_aggregator::FunctionId as AggregatorFunctionId,
        file_metrics::{FileDebtItem, FileDebtMetrics},
        score_types::Score0To100,
        scoring::debt_item,
        unified_scorer::Location,
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, UnifiedAnalysis,
        UnifiedAnalysisUtils, UnifiedDebtItem, UnifiedScore,
    },
    risk,
    utils::risk_analyzer::build_context_aggregator,
};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Options for unified analysis
pub struct UnifiedAnalysisOptions<'a> {
    pub results: &'a AnalysisResults,
    pub coverage_file: Option<&'a PathBuf>,
    pub semantic_off: bool,
    pub project_path: &'a Path,
    pub verbose_macro_warnings: bool,
    pub show_macro_stats: bool,
    pub parallel: bool,
    pub jobs: usize,
    pub multi_pass: bool,
    pub show_attribution: bool,
    pub aggregate_only: bool,
    pub no_aggregation: bool,
    pub aggregation_method: Option<String>,
    pub min_problematic: Option<usize>,
    pub no_god_object: bool,
    pub suppress_coverage_tip: bool,
    pub _formatting_config: crate::formatting::FormattingConfig,
    pub enable_context: bool,
    pub context_providers: Option<Vec<String>>,
    pub disable_context: Option<Vec<String>>,
}

pub fn perform_unified_analysis(
    results: &AnalysisResults,
    coverage_file: Option<&PathBuf>,
    _semantic_off: bool,
    project_path: &Path,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
) -> Result<UnifiedAnalysis> {
    perform_unified_analysis_with_options(UnifiedAnalysisOptions {
        results,
        coverage_file,
        semantic_off: _semantic_off,
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
    })
}

pub fn perform_unified_analysis_with_options(
    options: UnifiedAnalysisOptions,
) -> Result<UnifiedAnalysis> {
    let UnifiedAnalysisOptions {
        results,
        coverage_file,
        semantic_off,
        project_path,
        verbose_macro_warnings,
        show_macro_stats,
        parallel,
        jobs,
        multi_pass,
        show_attribution,
        aggregate_only: _aggregate_only,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
        suppress_coverage_tip,
        _formatting_config,
        enable_context,
        context_providers,
        disable_context,
    } = options;

    // Perform direct computation without caching
    perform_unified_analysis_computation(
        results,
        coverage_file,
        semantic_off,
        project_path,
        verbose_macro_warnings,
        show_macro_stats,
        parallel,
        jobs,
        multi_pass,
        show_attribution,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
        suppress_coverage_tip,
        _formatting_config,
        enable_context,
        context_providers,
        disable_context,
    )
}

/// Perform the actual unified analysis computation
#[allow(clippy::too_many_arguments)]
fn perform_unified_analysis_computation(
    results: &AnalysisResults,
    coverage_file: Option<&PathBuf>,
    _semantic_off: bool,
    project_path: &Path,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    parallel: bool,
    jobs: usize,
    multi_pass: bool,
    show_attribution: bool,
    no_aggregation: bool,
    aggregation_method: Option<String>,
    min_problematic: Option<usize>,
    no_god_object: bool,
    suppress_coverage_tip: bool,
    _formatting_config: crate::formatting::FormattingConfig,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
) -> Result<UnifiedAnalysis> {
    let mut call_graph = call_graph::build_initial_call_graph(&results.complexity.metrics);

    // Perform multi-pass analysis if enabled
    if multi_pass {
        // Show spinner for multi-pass analysis (spec 201)
        let spinner = crate::progress::ProgressManager::global()
            .map(|pm| pm.create_spinner("Analyzing code patterns"))
            .unwrap_or_else(indicatif::ProgressBar::hidden);

        perform_multi_pass_analysis(results, show_attribution)?;

        spinner.finish_and_clear();
    }

    // Progress tracking
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();

    // Phase 2: Building call graph (spec 195)
    crate::io::progress::AnalysisProgress::with_global(|p| p.start_phase(1));
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_start_stage(1); // call graph stage (index decreased by 1 after combining files+parse)
    }

    // Time call graph building
    let call_graph_start = std::time::Instant::now();
    let (framework_exclusions, function_pointer_used_functions) = if parallel {
        build_parallel_call_graph(project_path, &mut call_graph, jobs)?
    } else {
        build_sequential_call_graph(
            project_path,
            &mut call_graph,
            verbose_macro_warnings,
            show_macro_stats,
        )?
    };
    let call_graph_time = call_graph_start.elapsed();

    crate::io::progress::AnalysisProgress::with_global(|p| {
        p.update_progress(crate::io::progress::PhaseProgress::Progress {
            current: call_graph.node_count(),
            total: call_graph.node_count(),
        });
        p.complete_phase();
    });

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_complete_stage(1, format!("{} functions", call_graph.node_count()));
        manager.tui_set_progress(0.29); // ~2/7 stages complete
    }

    // Apply trait pattern detection to the merged call graph
    // This ensures trait methods (Default, Clone, constructors, etc.) are marked as entry points
    // after the enhanced graph has been merged in
    {
        use crate::analysis::call_graph::TraitRegistry;
        let trait_registry = TraitRegistry::new();
        trait_registry.detect_common_trait_patterns(&mut call_graph);
    }

    let coverage_loading_start = std::time::Instant::now();

    // Stage 3: Coverage loading (index 2 in TUI)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_start_stage(2); // coverage stage
    }

    // Show spinner for coverage loading if coverage file provided (spec 201)
    let spinner = if coverage_file.is_some() {
        crate::progress::ProgressManager::global()
            .map(|pm| pm.create_spinner("Loading coverage data"))
    } else {
        None
    };

    let coverage_data = load_coverage_data(coverage_file.cloned())?;

    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    let coverage_loading_time = coverage_loading_start.elapsed();

    // Calculate coverage percentage if data is available
    let coverage_percent = coverage_data.as_ref().map_or(0.0, |data| {
        if data.total_lines > 0 {
            (data.lines_hit as f64 / data.total_lines as f64) * 100.0
        } else {
            0.0
        }
    });

    if let Some(manager) = crate::progress::ProgressManager::global() {
        let metric = if coverage_data.is_some() {
            "loaded".to_string()
        } else {
            "skipped".to_string()
        };
        manager.tui_complete_stage(2, metric);
        manager.tui_set_progress(0.43); // ~3/7 stages complete

        // Update TUI stats with coverage percentage (preserves existing function/debt counts)
        manager.tui_update_coverage(coverage_percent);
    }

    // Emit warning if no coverage data provided (spec 108)
    // Suppress for validate command (spec 131)
    // Suppress when TUI is active to avoid disrupting progress display (the TUI shows "skipped" already)
    let tui_active = crate::progress::ProgressManager::global().is_some();
    if coverage_data.is_none() && !quiet_mode && !suppress_coverage_tip && !tui_active {
        use colored::*;
        eprintln!();
        eprintln!(
            "{} Coverage data not provided. Analysis will focus on complexity and code smells.",
            "[TIP]".bright_yellow()
        );
        eprintln!(
            "   For test gap detection, provide coverage with: {}",
            "--lcov-file coverage.info".bright_cyan()
        );
        eprintln!();
    }

    // Populate call graph data into function metrics for better analysis
    let mut enriched_metrics = call_graph_integration::populate_call_graph_data(
        results.complexity.metrics.clone(),
        &call_graph,
    );

    // Run inter-procedural purity propagation (spec 156)
    // Stage 4: Purity analysis (index 3 in TUI)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_start_stage(3); // purity analysis stage
    }

    let purity_spinner = crate::progress::ProgressManager::global()
        .map(|pm| pm.create_spinner("Analyzing function purity"))
        .unwrap_or_else(indicatif::ProgressBar::hidden);

    let purity_propagation_start = std::time::Instant::now();
    enriched_metrics = run_purity_propagation(&enriched_metrics, &call_graph);
    let _purity_propagation_time = purity_propagation_start.elapsed();

    purity_spinner.finish_and_clear();

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_complete_stage(3, format!("{} functions analyzed", enriched_metrics.len()));
        manager.tui_set_progress(0.57); // ~4/7 stages complete
    }

    // Progress is already tracked by unified AnalysisProgress system

    // Create context aggregator and risk analyzer for priority scoring (spec 202)
    // Stage 5: Context loading (index 4 in TUI)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_start_stage(4); // context stage
    }

    let context_spinner = if enable_context {
        crate::progress::ProgressManager::global()
            .map(|pm| pm.create_spinner("Loading project context"))
    } else {
        None
    };

    let risk_analyzer = if enable_context {
        let context_aggregator = build_context_aggregator(
            project_path,
            enable_context,
            context_providers,
            disable_context,
        );

        if let Some(aggregator) = context_aggregator {
            let debt_score = crate::debt::total_debt_score(&results.technical_debt.items) as f64;
            let debt_threshold = 100.0;

            Some(
                risk::RiskAnalyzer::default()
                    .with_debt_context(debt_score, debt_threshold)
                    .with_context_aggregator(aggregator),
            )
        } else {
            None
        }
    } else {
        None
    };

    if let Some(spinner) = context_spinner {
        spinner.finish_and_clear();
    }

    if let Some(manager) = crate::progress::ProgressManager::global() {
        let metric = if enable_context { "loaded" } else { "skipped" };
        manager.tui_complete_stage(4, metric);
        manager.tui_set_progress(0.83); // 5/6 stages complete
    }

    // Stage 6: Debt scoring and prioritization (index 5 in TUI)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_start_stage(5); // debt scoring and prioritization stage
    }

    // Show spinner for the main debt analysis computation
    let analysis_spinner = crate::progress::ProgressManager::global()
        .map(|pm| pm.create_spinner("Computing technical debt priorities"))
        .unwrap_or_else(indicatif::ProgressBar::hidden);

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
        coverage_loading_time,
        risk_analyzer,
        project_path,
        parallel,
        jobs,
    );

    analysis_spinner.finish_and_clear();

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_complete_stage(5, format!("{} items scored", result.items.len()));
        manager.tui_set_progress(1.0); // 100% complete
    }

    Ok(result)
}

/// Builds call graph using parallel processing
fn build_parallel_call_graph(
    project_path: &Path,
    call_graph: &mut CallGraph,
    jobs: usize,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    use crate::builders::parallel_call_graph::CallGraphPhase;

    let thread_count = if jobs == 0 { None } else { Some(jobs) };
    log_parallel_execution(jobs);

    let (parallel_graph, exclusions, used_funcs) = parallel_call_graph::build_call_graph_parallel(
        project_path,
        call_graph.clone(),
        thread_count,
        |progress| {
            if let Some(manager) = crate::progress::ProgressManager::global() {
                // Call graph stage is at index 1 (0=files, 1=call graph, 2=coverage, 3=purity, 4=context, 5=debt scoring)
                match progress.phase {
                    CallGraphPhase::DiscoveringFiles => {
                        manager.tui_update_subtask(
                            1,
                            0,
                            crate::tui::app::StageStatus::Active,
                            None,
                        );
                    }
                    CallGraphPhase::ParsingASTs => {
                        if progress.current == 0 {
                            manager.tui_update_subtask(
                                1,
                                0,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            manager.tui_update_subtask(
                                1,
                                1,
                                crate::tui::app::StageStatus::Active,
                                Some((0, progress.total)),
                            );
                        } else {
                            manager.tui_update_subtask(
                                1,
                                1,
                                crate::tui::app::StageStatus::Active,
                                Some((progress.current, progress.total)),
                            );
                        }
                    }
                    CallGraphPhase::ExtractingCalls => {
                        if progress.current == 0 {
                            manager.tui_update_subtask(
                                1,
                                1,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            manager.tui_update_subtask(
                                1,
                                2,
                                crate::tui::app::StageStatus::Active,
                                Some((0, progress.total)),
                            );
                        } else {
                            manager.tui_update_subtask(
                                1,
                                2,
                                crate::tui::app::StageStatus::Active,
                                Some((progress.current, progress.total)),
                            );
                        }
                    }
                    CallGraphPhase::LinkingModules => {
                        manager.tui_update_subtask(
                            1,
                            2,
                            crate::tui::app::StageStatus::Completed,
                            None,
                        );
                        manager.tui_update_subtask(
                            1,
                            3,
                            crate::tui::app::StageStatus::Active,
                            None,
                        );
                    }
                }
            }
        },
    )?;

    *call_graph = parallel_graph;

    Ok((exclusions, used_funcs))
}

/// Builds call graph using sequential processing
fn build_sequential_call_graph(
    project_path: &Path,
    call_graph: &mut CallGraph,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    use crate::builders::parallel_call_graph::CallGraphPhase;

    let (exclusions, used_funcs) = call_graph::process_rust_files_for_call_graph(
        project_path,
        call_graph,
        verbose_macro_warnings,
        show_macro_stats,
        |progress| {
            if let Some(manager) = crate::progress::ProgressManager::global() {
                // Call graph stage is at index 1 (0=files, 1=call graph, 2=coverage, 3=purity, 4=context, 5=debt scoring)
                match progress.phase {
                    CallGraphPhase::DiscoveringFiles => {
                        manager.tui_update_subtask(
                            1,
                            0,
                            crate::tui::app::StageStatus::Active,
                            None,
                        );
                    }
                    CallGraphPhase::ParsingASTs => {
                        if progress.current == 0 {
                            manager.tui_update_subtask(
                                1,
                                0,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            manager.tui_update_subtask(
                                1,
                                1,
                                crate::tui::app::StageStatus::Active,
                                Some((0, progress.total)),
                            );
                        } else {
                            manager.tui_update_subtask(
                                1,
                                1,
                                crate::tui::app::StageStatus::Active,
                                Some((progress.current, progress.total)),
                            );
                        }
                    }
                    CallGraphPhase::ExtractingCalls => {
                        if progress.current == 0 {
                            manager.tui_update_subtask(
                                1,
                                1,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            manager.tui_update_subtask(
                                1,
                                2,
                                crate::tui::app::StageStatus::Active,
                                Some((0, progress.total)),
                            );
                        } else {
                            manager.tui_update_subtask(
                                1,
                                2,
                                crate::tui::app::StageStatus::Active,
                                Some((progress.current, progress.total)),
                            );
                        }
                    }
                    CallGraphPhase::LinkingModules => {
                        manager.tui_update_subtask(
                            1,
                            2,
                            crate::tui::app::StageStatus::Completed,
                            None,
                        );
                        manager.tui_update_subtask(
                            1,
                            3,
                            crate::tui::app::StageStatus::Active,
                            None,
                        );
                    }
                }
            }
        },
    )?;

    Ok((exclusions, used_funcs))
}

/// Logs parallel execution information
fn log_parallel_execution(jobs: usize) {
    let thread_msg = if jobs == 0 {
        "all available".to_string()
    } else {
        jobs.to_string()
    };
    log::info!(
        "Using parallel call graph construction with {} threads",
        thread_msg
    );
}

/// Loads coverage data from the specified file
fn load_coverage_data(coverage_file: Option<PathBuf>) -> Result<Option<risk::lcov::LcovData>> {
    use std::time::Duration;

    match coverage_file {
        Some(lcov_path) => {
            // Start subsection 0: open file
            // Coverage stage is at index 2 (0=files, 1=call graph, 2=coverage, 3=purity, 4=context, 5=debt scoring)
            if let Some(manager) = crate::progress::ProgressManager::global() {
                manager.tui_update_subtask(2, 0, crate::tui::app::StageStatus::Active, None);
            }

            let data = risk::lcov::parse_lcov_file_with_callback(&lcov_path, |progress| {
                if let Some(manager) = crate::progress::ProgressManager::global() {
                    match progress {
                        risk::lcov::CoverageProgress::Initializing => {
                            // Subtask 0 complete: open file
                            manager.tui_update_subtask(
                                2,
                                0,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            std::thread::sleep(Duration::from_millis(150));
                            // Subtask 1 start: parse coverage
                            manager.tui_update_subtask(
                                2,
                                1,
                                crate::tui::app::StageStatus::Active,
                                None,
                            );
                        }
                        risk::lcov::CoverageProgress::Parsing { current, total } => {
                            manager.tui_update_subtask(
                                2,
                                1,
                                crate::tui::app::StageStatus::Active,
                                Some((current, total)),
                            );
                        }
                        risk::lcov::CoverageProgress::ComputingStats => {
                            // Subtask 1 complete
                            manager.tui_update_subtask(
                                2,
                                1,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            std::thread::sleep(Duration::from_millis(150));
                            // Subtask 2 start: compute stats
                            manager.tui_update_subtask(
                                2,
                                2,
                                crate::tui::app::StageStatus::Active,
                                None,
                            );
                        }
                        risk::lcov::CoverageProgress::Complete => {
                            manager.tui_update_subtask(
                                2,
                                2,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                        }
                    }
                }
            })
            .context("Failed to parse LCOV file")?;

            Ok(Some(data))
        }
        None => {
            // No coverage file provided - skip all subsections
            if let Some(manager) = crate::progress::ProgressManager::global() {
                manager.tui_update_subtask(2, 0, crate::tui::app::StageStatus::Completed, None);
                manager.tui_update_subtask(2, 1, crate::tui::app::StageStatus::Completed, None);
                manager.tui_update_subtask(2, 2, crate::tui::app::StageStatus::Completed, None);
            }
            Ok(None)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_unified_analysis_with_exclusions(
    metrics: &[FunctionMetrics],
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<priority::call_graph::FunctionId>,
    function_pointer_used_functions: Option<&HashSet<priority::call_graph::FunctionId>>,
    debt_items: Option<&[DebtItem]>,
    no_aggregation: bool,
    aggregation_method: Option<String>,
    min_problematic: Option<usize>,
    no_god_object: bool,
) -> UnifiedAnalysis {
    use std::time::Duration;
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
        Duration::from_secs(0),
        Duration::from_secs(0),
        None,           // No risk analyzer in wrapper function
        Path::new("."), // Default project path
        false,          // Default to sequential (for compatibility)
        0,              // Default jobs = 0 (auto-detect)
    )
}

#[allow(clippy::too_many_arguments)]
fn create_unified_analysis_with_exclusions_and_timing(
    metrics: &[FunctionMetrics],
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<priority::call_graph::FunctionId>,
    function_pointer_used_functions: Option<&HashSet<priority::call_graph::FunctionId>>,
    debt_items: Option<&[DebtItem]>,
    no_aggregation: bool,
    aggregation_method: Option<String>,
    min_problematic: Option<usize>,
    no_god_object: bool,
    call_graph_time: std::time::Duration,
    coverage_loading_time: std::time::Duration,
    risk_analyzer: Option<risk::RiskAnalyzer>,
    project_path: &Path,
    parallel: bool,
    jobs: usize,
) -> UnifiedAnalysis {
    // Use parallel mode based on function parameter (not environment variable)
    // Environment variable fallback is for backward compatibility
    let parallel_enabled = parallel
        || std::env::var("DEBTMAP_PARALLEL")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

    let jobs_count = if jobs > 0 {
        Some(jobs)
    } else {
        std::env::var("DEBTMAP_JOBS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
    };

    if parallel_enabled {
        return create_unified_analysis_parallel(
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
            jobs_count,
            call_graph_time,
            coverage_loading_time,
            risk_analyzer,
            project_path,
        );
    }
    use std::time::Instant;
    let start = Instant::now();
    let _quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();

    // Progress is tracked by unified AnalysisProgress system - no individual step output needed

    // Step 1: Initialize unified analysis with data flow graph (no subtask - instant operation)
    let mut unified = UnifiedAnalysis::new(call_graph.clone());

    // Step 2: Populate purity analysis
    unified.populate_purity_analysis(metrics);

    // Step 3: Find test-only functions
    let test_only_functions: HashSet<_> =
        call_graph.find_test_only_functions().into_iter().collect();

    // Subtask 0: Aggregate debt
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(5, 0, crate::tui::app::StageStatus::Active, None);
    }

    // Step 4: Setup debt aggregator
    let mut debt_aggregator = DebtAggregator::new();
    if let Some(debt_items) = debt_items {
        let function_mappings: Vec<(AggregatorFunctionId, usize, usize)> = metrics
            .iter()
            .map(|m| {
                let func_id = AggregatorFunctionId::new(m.file.clone(), m.name.clone(), m.line);
                (func_id, m.line, m.line + m.length)
            })
            .collect();

        let debt_items_vec: Vec<DebtItem> = debt_items.to_vec();
        debt_aggregator.aggregate_debt(debt_items_vec, &function_mappings);
    }

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(5, 0, crate::tui::app::StageStatus::Completed, None);
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 1: Score functions (main computational loop with progress)
    let total_metrics = metrics.len();
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            5,
            1,
            crate::tui::app::StageStatus::Active,
            Some((0, total_metrics)),
        );
    }

    // Step 5: Per-function debt analysis (main loop)
    let mut last_update = Instant::now();
    for (idx, metric) in metrics.iter().enumerate() {
        if should_skip_metric_for_debt_analysis(metric, call_graph, &test_only_functions) {
            // Throttled progress update
            if idx % 100 == 0 || last_update.elapsed() > std::time::Duration::from_millis(100) {
                if let Some(manager) = crate::progress::ProgressManager::global() {
                    manager.tui_update_subtask(
                        5,
                        1,
                        crate::tui::app::StageStatus::Active,
                        Some((idx + 1, total_metrics)),
                    );
                }
                last_update = Instant::now();
            }
            continue;
        }
        // Create debt items (spec 228: multi-debt returns Vec)
        let items = create_debt_item_from_metric_with_aggregator(
            metric,
            call_graph,
            coverage_data,
            framework_exclusions,
            function_pointer_used_functions,
            &debt_aggregator,
            Some(&unified.data_flow_graph),
            risk_analyzer.as_ref(),
            project_path,
        );

        // Add all debt items for this function (may be multiple)
        for item in items {
            unified.add_item(item);
        }

        // Throttled progress update
        if idx % 100 == 0 || last_update.elapsed() > std::time::Duration::from_millis(100) {
            if let Some(manager) = crate::progress::ProgressManager::global() {
                manager.tui_update_subtask(
                    5,
                    1,
                    crate::tui::app::StageStatus::Active,
                    Some((idx + 1, total_metrics)),
                );
            }
            last_update = Instant::now();
        }
    }

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            5,
            1,
            crate::tui::app::StageStatus::Completed,
            Some((total_metrics, total_metrics)),
        );
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 2: Filter results (error analysis, file analysis, sorting, filtering)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(5, 2, crate::tui::app::StageStatus::Active, None);
    }

    // Step 6: Error swallowing analysis - REMOVED
    // Error swallowing is no longer reported as separate debt items.
    // Instead, error swallowing info is attached to FunctionMetrics and
    // shown in the TUI patterns page for functions that have other debt types.
    // See: src/tui/results/detail_pages/patterns.rs (Error Handling section)
    let _ = debt_items; // Silence unused warning

    // Step 7: File-level analysis
    analyze_files_for_debt(
        &mut unified,
        metrics,
        coverage_data,
        no_god_object,
        risk_analyzer.as_ref(),
        project_path,
    );

    // Step 8: File aggregation has been removed - skip to step 9

    // Step 9: Final sorting and impact calculation
    unified.sort_by_priority();
    unified.calculate_total_impact();

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(5, 2, crate::tui::app::StageStatus::Completed, None);
    }

    // Set coverage data availability flag (spec 108)
    unified.has_coverage_data = coverage_data.is_some();

    if let Some(lcov) = coverage_data {
        unified.overall_coverage = Some(lcov.get_overall_coverage());
    }

    let total_time = start.elapsed();

    // Attach timing information for sequential analysis (spec 130)
    // Note: Sequential analysis doesn't track individual phase timings,
    // but we can attach the preliminary timings that were passed in
    unified.timings = Some(parallel_unified_analysis::AnalysisPhaseTimings {
        call_graph_building: call_graph_time,
        trait_resolution: std::time::Duration::from_secs(0),
        coverage_loading: coverage_loading_time,
        data_flow_creation: std::time::Duration::from_secs(0),
        purity_analysis: std::time::Duration::from_secs(0),
        test_detection: std::time::Duration::from_secs(0),
        debt_aggregation: std::time::Duration::from_secs(0),
        function_analysis: std::time::Duration::from_secs(0),
        file_analysis: std::time::Duration::from_secs(0),
        aggregation: std::time::Duration::from_secs(0),
        sorting: std::time::Duration::from_secs(0),
        total: total_time,
    });

    unified
}

#[allow(clippy::too_many_arguments)]
fn create_unified_analysis_parallel(
    metrics: &[FunctionMetrics],
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<priority::call_graph::FunctionId>,
    function_pointer_used_functions: Option<&HashSet<priority::call_graph::FunctionId>>,
    debt_items: Option<&[DebtItem]>,
    _no_aggregation: bool,
    _aggregation_method: Option<String>,
    _min_problematic: Option<usize>,
    no_god_object: bool,
    jobs: Option<usize>,
    call_graph_time: std::time::Duration,
    coverage_loading_time: std::time::Duration,
    risk_analyzer: Option<risk::RiskAnalyzer>,
    project_path: &Path,
) -> UnifiedAnalysis {
    use parallel_unified_analysis::{
        ParallelUnifiedAnalysisBuilder, ParallelUnifiedAnalysisOptions,
    };

    let options = ParallelUnifiedAnalysisOptions {
        parallel: true,
        jobs,
        batch_size: 100,
        progress: std::env::var("DEBTMAP_QUIET").is_err(),
    };

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph.clone(), options)
        .with_project_path(project_path.to_path_buf());

    // Add risk analyzer if provided (spec 202)
    if let Some(analyzer) = risk_analyzer {
        builder = builder.with_risk_analyzer(analyzer);
    }

    // Set preliminary timing values from call graph building and coverage loading
    builder.set_preliminary_timings(call_graph_time, coverage_loading_time);

    // Phase 1: Parallel initialization
    let (data_flow_graph, _purity, test_only_functions, debt_aggregator) =
        builder.execute_phase1_parallel(metrics, debt_items);

    // Phase 2: Parallel function processing
    let items = builder.execute_phase2_parallel(
        metrics,
        &test_only_functions,
        &debt_aggregator,
        &data_flow_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
    );

    // Subtask 2: Filter results (error analysis, file analysis, sorting, filtering) - PARALLEL
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(5, 2, crate::tui::app::StageStatus::Active, None);
    }

    // Error swallowing items - REMOVED
    // Error swallowing is no longer reported as separate debt items.
    // Instead, error swallowing info is attached to FunctionMetrics and
    // shown in the TUI patterns page for functions that have other debt types.
    let all_items = items;
    let _ = debt_items; // Silence unused warning

    // Phase 3: Parallel file analysis
    let file_items = builder.execute_phase3_parallel(metrics, coverage_data, no_god_object);

    // Build final unified analysis
    let (mut unified, timings) = builder.build(
        data_flow_graph,
        _purity,
        all_items,
        file_items,
        coverage_data,
    );

    // Attach timing information (spec 130)
    unified.timings = Some(timings);

    // Aggregation has been removed - no longer needed

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(5, 2, crate::tui::app::StageStatus::Completed, None);
    }

    unified
}

fn should_skip_metric_for_debt_analysis(
    metric: &FunctionMetrics,
    call_graph: &priority::CallGraph,
    test_only_functions: &HashSet<priority::call_graph::FunctionId>,
) -> bool {
    if metric.is_test || metric.in_test_module {
        return true;
    }

    if metric.name.contains("<closure@") {
        return true;
    }

    let func_id = priority::call_graph::FunctionId::new(
        metric.file.clone(),
        metric.name.clone(),
        metric.line,
    );

    if test_only_functions.contains(&func_id) {
        return true;
    }

    if metric.cyclomatic == 1 && metric.cognitive == 0 && metric.length <= 3 {
        let callees = call_graph.get_callees(&func_id);
        if callees.len() == 1 {
            return true;
        }
    }

    false
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_debt_item_from_metric_with_aggregator(
    metric: &FunctionMetrics,
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<priority::call_graph::FunctionId>,
    function_pointer_used_functions: Option<&HashSet<priority::call_graph::FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
    risk_analyzer: Option<&risk::RiskAnalyzer>,
    project_path: &Path,
) -> Vec<UnifiedDebtItem> {
    // Use the unified debt item creation (spec 201, spec 228: multi-debt)
    // Returns Vec<UnifiedDebtItem> - one per debt type found
    debt_item::create_unified_debt_item_with_aggregator_and_data_flow(
        metric,
        call_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        debt_aggregator,
        data_flow,
        risk_analyzer,
        project_path,
    )
}

/// Perform multi-pass analysis on the results
fn perform_multi_pass_analysis(results: &AnalysisResults, show_attribution: bool) -> Result<()> {
    // Group function metrics by file
    let mut files: std::collections::HashMap<PathBuf, Vec<&FunctionMetrics>> =
        std::collections::HashMap::new();

    for function in &results.complexity.metrics {
        files
            .entry(function.file.clone())
            .or_default()
            .push(function);
    }

    // For each file, perform multi-pass analysis
    for (file_path, _functions) in files {
        // Read the source file content
        if let Ok(source_content) = std::fs::read_to_string(&file_path) {
            // Determine language from file extension
            let language = Language::from_path(&file_path);

            let options = MultiPassOptions {
                language,
                detail_level: if show_attribution {
                    DetailLevel::Comprehensive
                } else {
                    DetailLevel::Standard
                },
                enable_recommendations: true,
                track_source_locations: true,
                generate_insights: true,
                output_format: OutputFormat::Json,
                performance_tracking: true,
            };

            match analyze_with_attribution(&source_content, language, options) {
                Ok(multi_pass_result) => {
                    // Print attribution information if requested
                    if show_attribution {
                        print_attribution_summary(&multi_pass_result, &file_path);

                        // Generate and print detailed diagnostic report
                        let reporter = DiagnosticReporter::new(
                            OutputFormat::Markdown,
                            DetailLevel::Comprehensive,
                        );
                        let diagnostic_report = reporter.generate_report(&multi_pass_result);
                        let formatted_report = reporter.format_report(&diagnostic_report);

                        println!("\n{}", formatted_report);
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Multi-pass analysis failed for {}: {}",
                        file_path.display(),
                        e
                    );
                }
            }
        }
    }

    Ok(())
}

/// Print a summary of the multi-pass analysis attribution
fn print_attribution_summary(result: &MultiPassResult, file_path: &Path) {
    println!("\n=== Multi-Pass Analysis: {} ===", file_path.display());
    println!("Raw Complexity: {}", result.raw_complexity.total_complexity);
    println!(
        "Normalized Complexity: {}",
        result.normalized_complexity.total_complexity
    );

    if let Some(ref perf) = result.performance_metrics {
        println!(
            "Analysis Time: {}ms (raw: {}ms, normalized: {}ms, attribution: {}ms)",
            perf.total_time_ms,
            perf.raw_analysis_time_ms,
            perf.normalized_analysis_time_ms,
            perf.attribution_time_ms
        );
    }

    println!("Insights: {} found", result.insights.len());
    println!(
        "Recommendations: {} generated",
        result.recommendations.len()
    );

    if !result.insights.is_empty() {
        println!("\nKey Insights:");
        for insight in result.insights.iter().take(3) {
            println!("  - {}", insight.description);
        }
    }
}

fn analyze_files_for_debt(
    unified: &mut UnifiedAnalysis,
    metrics: &[FunctionMetrics],
    coverage_data: Option<&risk::lcov::LcovData>,
    no_god_object: bool,
    risk_analyzer: Option<&risk::RiskAnalyzer>,
    project_path: &Path,
) {
    use crate::analyzers::file_analyzer::UnifiedFileAnalyzer;

    // Pure functional pipeline for file analysis
    let file_groups = group_functions_by_file(metrics);
    let total_files = file_groups.len();
    let file_analyzer = UnifiedFileAnalyzer::new(coverage_data.cloned());

    // Initialize progress tracking (maintaining design consistency with subtask 2)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            6,
            3,
            crate::tui::app::StageStatus::Active,
            Some((0, total_files)),
        );
    }

    // Process files with progress updates (preserving functional style)
    let mut processed_files = Vec::new();
    let mut last_update = std::time::Instant::now();

    for (idx, (file_path, functions)) in file_groups.into_iter().enumerate() {
        if let Ok(data) = process_single_file(
            file_path,
            functions,
            &file_analyzer,
            no_god_object,
            unified,
            project_path,
        ) {
            if data.file_metrics.calculate_score() > 50.0 {
                processed_files.push(data);
            }
        }

        // Throttled progress updates (100ms intervals maintain 60 FPS - spec DESIGN.md:179)
        if (idx + 1) % 10 == 0 || last_update.elapsed() > std::time::Duration::from_millis(100) {
            if let Some(manager) = crate::progress::ProgressManager::global() {
                manager.tui_update_subtask(
                    6,
                    3,
                    crate::tui::app::StageStatus::Active,
                    Some((idx + 1, total_files)),
                );
            }
            last_update = std::time::Instant::now();
        }
    }

    // Apply results to unified analysis (I/O at edges)
    apply_file_analysis_results(
        unified,
        processed_files,
        risk_analyzer,
        project_path,
        coverage_data,
    );
}

// Pure function to group functions by file
fn group_functions_by_file(
    metrics: &[FunctionMetrics],
) -> std::collections::HashMap<PathBuf, Vec<FunctionMetrics>> {
    let mut files_map = std::collections::HashMap::new();
    for metric in metrics {
        files_map
            .entry(metric.file.clone())
            .or_insert_with(Vec::new)
            .push(metric.clone());
    }
    files_map
}

// Data structure for processed file information
struct ProcessedFileData {
    file_path: PathBuf,
    file_metrics: FileDebtMetrics,
    god_analysis: Option<crate::organization::GodObjectAnalysis>,
    file_context: crate::analysis::FileContext,
    raw_functions: Vec<FunctionMetrics>, // Keep raw metrics for god object aggregation
    project_root: PathBuf,               // Project root for git context analysis
}

// Pure function to process a single file
fn process_single_file(
    file_path: PathBuf,
    functions: Vec<FunctionMetrics>,
    file_analyzer: &crate::analyzers::file_analyzer::UnifiedFileAnalyzer,
    no_god_object: bool,
    unified: &UnifiedAnalysis,
    project_root: &Path,
) -> Result<ProcessedFileData, Box<dyn std::error::Error>> {
    // Get base file metrics
    let file_metrics = file_analyzer.aggregate_functions(&functions);

    // Apply file content analysis
    let file_content = std::fs::read_to_string(&file_path)?;
    let enhanced_metrics = enhance_metrics_with_content(
        file_metrics,
        &file_content,
        file_analyzer,
        &file_path,
        no_god_object,
    )?;

    // Calculate function scores and update metrics
    let function_scores = calculate_function_scores(&functions, unified);
    let mut final_metrics = enhanced_metrics;
    final_metrics.function_scores = function_scores;

    // Detect file context for scoring adjustments (spec 166/168)
    use crate::analysis::FileContextDetector;
    use crate::core::Language;
    let language = Language::from_path(&file_path);
    let detector = FileContextDetector::new(language);
    let file_context = detector.detect(&file_path, &functions);

    // Generate god object analysis
    let god_analysis = create_god_object_analysis(&final_metrics);

    Ok(ProcessedFileData {
        file_path,
        file_metrics: final_metrics,
        god_analysis,
        file_context,
        raw_functions: functions, // Include raw metrics for aggregation
        project_root: project_root.to_path_buf(),
    })
}

// Pure function to enhance metrics with file content
fn enhance_metrics_with_content(
    mut file_metrics: FileDebtMetrics,
    content: &str,
    file_analyzer: &crate::analyzers::file_analyzer::UnifiedFileAnalyzer,
    file_path: &Path,
    no_god_object: bool,
) -> Result<FileDebtMetrics, Box<dyn std::error::Error>> {
    let actual_line_count = content.lines().count();
    file_metrics.total_lines = actual_line_count;

    // Recalculate uncovered lines based on actual line count
    file_metrics.uncovered_lines =
        calculate_uncovered_lines(file_metrics.coverage_percent, actual_line_count);

    // Apply god object detection if enabled
    if !no_god_object {
        file_metrics.god_object_analysis = detect_god_object_analysis(
            file_analyzer,
            file_path,
            content,
            &file_metrics,
            actual_line_count,
        );
    } else {
        file_metrics.god_object_analysis = None;
    }

    Ok(file_metrics)
}

// Pure function to calculate uncovered lines
fn calculate_uncovered_lines(coverage_percent: f64, line_count: usize) -> usize {
    ((1.0 - coverage_percent) * line_count as f64) as usize
}

// Pure function to detect god object analysis
fn detect_god_object_analysis(
    file_analyzer: &crate::analyzers::file_analyzer::UnifiedFileAnalyzer,
    file_path: &Path,
    content: &str,
    file_metrics: &FileDebtMetrics,
    actual_line_count: usize,
) -> Option<crate::organization::GodObjectAnalysis> {
    // Get analysis from file analyzer or existing analysis
    let mut god_analysis = file_analyzer
        .analyze_file(file_path, content)
        .ok()
        .and_then(|m| m.god_object_analysis)
        .or_else(|| file_metrics.god_object_analysis.clone());

    // Apply size-based god object detection
    if actual_line_count > 2000 || file_metrics.function_count > 50 {
        // If we have an existing analysis, update it
        if let Some(ref mut analysis) = god_analysis {
            analysis.is_god_object = true;
            if analysis.god_object_score == Score0To100::new(0.0) {
                analysis.god_object_score = Score0To100::new(
                    ((file_metrics.function_count as f64 / 50.0).min(2.0)) * 100.0,
                );
            }
        } else {
            // Create new minimal analysis for size-based detection
            god_analysis = Some(crate::organization::GodObjectAnalysis {
                is_god_object: true,
                method_count: file_metrics.function_count,
                field_count: 0,
                responsibility_count: 0,
                lines_of_code: actual_line_count,
                complexity_sum: file_metrics.total_complexity,
                god_object_score: Score0To100::new(
                    ((file_metrics.function_count as f64 / 50.0).min(2.0)) * 100.0,
                ),
                recommended_splits: Vec::new(),
                confidence: crate::organization::GodObjectConfidence::Probable,
                responsibilities: Vec::new(),
                responsibility_method_counts: std::collections::HashMap::new(),
                purity_distribution: None,
                module_structure: None,
                detection_type: crate::organization::DetectionType::GodFile,
                struct_name: None, // Size-based detection is always GodFile
                struct_line: None,
                visibility_breakdown: None,
                domain_count: 0,
                domain_diversity: 0.0,
                struct_ratio: 0.0,
                analysis_method: crate::organization::SplitAnalysisMethod::None,
                cross_domain_severity: None,
                domain_diversity_metrics: None,
            });
        }
    }

    god_analysis
}

// Pure function to calculate function scores
// Note: Returns empty vec to avoid circular dependency during file analysis
// File scores are calculated based on other metrics, not function debt scores
fn calculate_function_scores(
    _functions: &[FunctionMetrics],
    _unified: &UnifiedAnalysis,
) -> Vec<f64> {
    // Returning empty to maintain consistency with parallel implementation
    // and avoid circular dependency where file scores depend on function scores
    // which haven't been finalized yet
    Vec::new()
}

// Pure function to create god object analysis
fn create_god_object_analysis(
    file_metrics: &FileDebtMetrics,
) -> Option<crate::organization::GodObjectAnalysis> {
    // Simply return the existing analysis if present
    file_metrics.god_object_analysis.clone()
}

/// Pure function to create a UnifiedDebtItem from god object indicators (spec 207)
///
/// God objects are file-level technical debt items representing files with
/// too many responsibilities, methods, or fields. They bypass function-level
/// complexity filtering since they represent architectural issues rather than
/// individual function complexity.
pub fn create_god_object_debt_item(
    file_path: &Path,
    file_metrics: &FileDebtMetrics,
    god_analysis: &crate::organization::GodObjectAnalysis,
    mut aggregated_metrics: crate::priority::GodObjectAggregatedMetrics,
    coverage_data: Option<&risk::lcov::LcovData>,
) -> UnifiedDebtItem {
    // Fallback: If no function-level coverage, use file-level coverage from LCOV
    // This handles god files where all functions are too simple to be debt items
    // but the file still has test coverage
    if aggregated_metrics.weighted_coverage.is_none() {
        if let Some(coverage) = coverage_data {
            if let Some(file_coverage) = coverage.get_file_coverage(file_path) {
                aggregated_metrics.weighted_coverage = Some(priority::TransitiveCoverage {
                    direct: file_coverage,
                    transitive: 0.0,
                    propagated_from: vec![],
                    uncovered_lines: vec![],
                });
            }
        }
    }

    // Calculate unified score based on god object score (0-100 scale)
    let base_score = god_analysis.god_object_score.value();
    let tier = if base_score >= 50.0 { 1 } else { 2 };

    // Use aggregated coverage in score calculation (now with file-level fallback)
    let coverage_factor = aggregated_metrics
        .weighted_coverage
        .as_ref()
        .map(|cov| (1.0 - cov.direct) * 10.0)
        .unwrap_or(0.0);

    // Apply coverage as dampening multiplier (same as regular functions - spec 122)
    // 100% coverage → multiplier = 0.0 → near-zero score
    // 0% coverage → multiplier = 1.0 → full base score
    let coverage_multiplier = aggregated_metrics
        .weighted_coverage
        .as_ref()
        .map(|cov| 1.0 - cov.direct)
        .unwrap_or(1.0);
    let coverage_adjusted_score = base_score * coverage_multiplier;

    let mut unified_score = UnifiedScore {
        final_score: Score0To100::new(coverage_adjusted_score),
        complexity_factor: file_metrics.total_complexity as f64 / 10.0,
        coverage_factor,
        dependency_factor: calculate_god_object_risk(god_analysis) / 10.0,
        role_multiplier: 1.0,
        base_score: Some(base_score),
        exponential_factor: None,
        risk_boost: None,
        pre_adjustment_score: None,
        adjustment_applied: None,
        purity_factor: None,
        refactorability_factor: None,
        pattern_factor: None,
    };

    // Apply contextual risk to score if available (spec 255)
    if let Some(ref ctx_risk) = aggregated_metrics.aggregated_contextual_risk {
        unified_score = crate::priority::scoring::construction::apply_contextual_risk_to_score(
            unified_score,
            ctx_risk,
        );
    }

    // Unified debt type for all god object detections (spec 253)
    // Use detection_type in god_object_indicators to distinguish between GodClass, GodFile, GodModule
    let debt_type = DebtType::GodObject {
        methods: god_analysis.method_count as u32,
        fields: match god_analysis.detection_type {
            crate::organization::DetectionType::GodClass => Some(god_analysis.field_count as u32),
            crate::organization::DetectionType::GodFile
            | crate::organization::DetectionType::GodModule => None,
        },
        responsibilities: god_analysis.responsibility_count as u32,
        god_object_score: god_analysis.god_object_score,
        lines: god_analysis.lines_of_code as u32,
    };

    // Determine display name and line number based on detection type
    let (display_name, line_number) = match god_analysis.detection_type {
        crate::organization::DetectionType::GodClass => {
            // For GodClass, use struct name and struct line if available
            let name = god_analysis.struct_name.as_deref().unwrap_or_else(|| {
                file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            });
            let line = god_analysis.struct_line.unwrap_or(1);
            (name.to_string(), line)
        }
        crate::organization::DetectionType::GodFile
        | crate::organization::DetectionType::GodModule => {
            // For GodFile and GodModule, use file name and line 1
            let name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            (name.to_string(), 1)
        }
    };

    // Create impact metrics
    let expected_impact = ImpactMetrics {
        coverage_improvement: 0.0,
        lines_reduction: god_analysis.lines_of_code as u32
            / god_analysis.recommended_splits.len().max(1) as u32,
        complexity_reduction: file_metrics.total_complexity as f64
            / god_analysis.recommended_splits.len().max(1) as f64,
        risk_reduction: calculate_god_object_risk(god_analysis),
    };

    // Create recommendation
    let recommendation = create_god_object_recommendation(god_analysis);

    let tier_enum = if tier == 1 {
        crate::priority::RecommendationTier::T1CriticalArchitecture
    } else {
        crate::priority::RecommendationTier::T2ComplexUntested
    };

    UnifiedDebtItem {
        location: Location {
            file: file_path.to_path_buf(),
            function: display_name,
            line: line_number,
        },
        debt_type,
        unified_score,
        function_role: FunctionRole::Unknown, // Not applicable for file-level items
        recommendation,
        expected_impact,
        transitive_coverage: aggregated_metrics.weighted_coverage,
        upstream_dependencies: aggregated_metrics.upstream_dependencies,
        downstream_dependencies: aggregated_metrics.downstream_dependencies,
        upstream_callers: aggregated_metrics.unique_upstream_callers,
        downstream_callees: aggregated_metrics.unique_downstream_callees,
        nesting_depth: aggregated_metrics.max_nesting_depth,
        function_length: god_analysis.lines_of_code,
        cyclomatic_complexity: aggregated_metrics.total_cyclomatic,
        cognitive_complexity: aggregated_metrics.total_cognitive,
        entropy_details: None,
        entropy_adjusted_cyclomatic: None,
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
        is_pure: None,
        purity_confidence: None,
        purity_level: None,
        god_object_indicators: Some(god_analysis.clone()),
        tier: Some(tier_enum),
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        file_context: None,
        context_multiplier: None,
        context_type: None,
        language_specific: None,
        detected_pattern: None,
        contextual_risk: aggregated_metrics.aggregated_contextual_risk,
        file_line_count: Some(god_analysis.lines_of_code),
        responsibility_category: god_analysis.responsibilities.first().cloned(), // Primary responsibility from detailed list (spec 254)
        error_swallowing_count: None, // Not applicable for god object items
        error_swallowing_patterns: None,
    }
}

/// Calculate risk score for god object (spec 207)
fn calculate_god_object_risk(god_analysis: &crate::organization::GodObjectAnalysis) -> f64 {
    // More responsibilities and methods = higher risk
    let responsibility_risk = god_analysis.responsibility_count as f64 * 10.0;
    let method_risk = (god_analysis.method_count as f64 / 10.0).min(50.0);

    (responsibility_risk + method_risk).min(100.0)
}

/// Create actionable recommendation for god object (spec 207)
fn create_god_object_recommendation(
    god_analysis: &crate::organization::GodObjectAnalysis,
) -> ActionableRecommendation {
    // Calculate recommended split count based on responsibility count
    // Only use recommended_splits if it has 2+ meaningful splits
    // A single recommended split is nonsensical (can't split into 1 piece)
    let split_count = if god_analysis.recommended_splits.len() >= 2 {
        god_analysis.recommended_splits.len()
    } else {
        // Heuristic: Split into 2-5 modules based on responsibility count
        // For N responsibilities, recommend splitting into min(N, 5) modules, with minimum of 2
        god_analysis.responsibility_count.clamp(2, 5)
    };

    let primary_action = format!("Split into {} modules by responsibility", split_count);

    let rationale = format!(
        "{} responsibilities detected with {} methods/functions - splitting will improve maintainability",
        god_analysis.responsibility_count, god_analysis.method_count
    );

    ActionableRecommendation {
        primary_action,
        rationale,
        implementation_steps: Vec::new(),
        related_items: Vec::new(),
        steps: None,
        estimated_effort_hours: None,
    }
}

/// Analyze file-level git context for god objects.
/// Returns contextual risk based on file's git history.
/// This is a pure function that delegates to the risk analyzer's context aggregator.
pub fn analyze_file_git_context(
    file_path: &Path,
    risk_analyzer: &risk::RiskAnalyzer,
    project_root: &Path,
) -> Option<risk::context::ContextualRisk> {
    // Check if context aggregator is available
    if !risk_analyzer.has_context() {
        return None;
    }

    // For god objects, use a moderate base risk since they're inherently risky
    // due to their size and complexity. The git context multipliers will be
    // applied on top of this base value.
    // Base risk of 40 represents a "moderate-high" risk level that git history
    // can then amplify or reduce based on change patterns.
    let base_risk = 40.0;

    risk_analyzer.analyze_file_context(
        file_path.to_path_buf(),
        base_risk,
        project_root.to_path_buf(),
    )
}

// I/O function to apply results to unified analysis
fn apply_file_analysis_results(
    unified: &mut UnifiedAnalysis,
    processed_files: Vec<ProcessedFileData>,
    risk_analyzer: Option<&risk::RiskAnalyzer>,
    _project_root: &Path,
    coverage_data: Option<&risk::lcov::LcovData>,
) {
    use crate::priority::god_object_aggregation::{
        aggregate_from_raw_metrics, aggregate_god_object_metrics, extract_member_functions,
    };

    for file_data in processed_files {
        // Update god object indicators for functions in this file
        if let Some(god_analysis) = &file_data.god_analysis {
            update_function_god_indicators(unified, &file_data.file_path, god_analysis);

            // Aggregate from raw metrics first for complexity (includes ALL functions, even tests)
            let mut aggregated_metrics = aggregate_from_raw_metrics(&file_data.raw_functions);

            // Enrich with coverage/dependencies/risk from unified items
            // NOTE: Dependencies aggregate only from "problematic" functions that became debt items.
            // This provides a debt-focused view rather than complete architectural dependencies.
            let member_functions =
                extract_member_functions(unified.items.iter(), &file_data.file_path);
            if !member_functions.is_empty() {
                let item_metrics = aggregate_god_object_metrics(&member_functions);
                // Merge all contextual data from debt items
                aggregated_metrics.weighted_coverage = item_metrics.weighted_coverage;
                aggregated_metrics.unique_upstream_callers = item_metrics.unique_upstream_callers;
                aggregated_metrics.unique_downstream_callees =
                    item_metrics.unique_downstream_callees;
                aggregated_metrics.upstream_dependencies = item_metrics.upstream_dependencies;
                aggregated_metrics.downstream_dependencies = item_metrics.downstream_dependencies;

                // Spec 248: Prefer direct file-level git analysis over member aggregation
                aggregated_metrics.aggregated_contextual_risk = risk_analyzer
                    .and_then(|analyzer| {
                        analyze_file_git_context(
                            &file_data.file_path,
                            analyzer,
                            &file_data.project_root,
                        )
                    })
                    .or(item_metrics.aggregated_contextual_risk); // Fallback to member aggregation
            } else {
                // Spec 248: When no member functions, try direct file analysis
                aggregated_metrics.aggregated_contextual_risk =
                    risk_analyzer.and_then(|analyzer| {
                        analyze_file_git_context(
                            &file_data.file_path,
                            analyzer,
                            &file_data.project_root,
                        )
                    });
            }
            // If member_functions is empty, dependencies remain at 0 (no debt items = no deps to show)

            // NEW (spec 207): Create god object debt item for TUI display
            let god_item = create_god_object_debt_item(
                &file_data.file_path,
                &file_data.file_metrics,
                god_analysis,
                aggregated_metrics,
                coverage_data,
            );
            unified.add_item(god_item); // Exempt from complexity filtering (spec 207)
        }

        // Create and add file debt item
        let file_item = create_file_debt_item(file_data);
        unified.add_file_item(file_item);
    }
}

// Pure function to update function god indicators
fn update_function_god_indicators(
    unified: &mut UnifiedAnalysis,
    file_path: &Path,
    god_analysis: &crate::organization::GodObjectAnalysis,
) {
    for item in unified.items.iter_mut() {
        if item.location.file == *file_path {
            item.god_object_indicators = Some(god_analysis.clone());
        }
    }
}

// Pure function to create file debt item
fn create_file_debt_item(file_data: ProcessedFileData) -> FileDebtItem {
    // Use from_metrics with file context for proper score adjustment (spec 168)
    crate::priority::FileDebtItem::from_metrics(
        file_data.file_metrics,
        Some(&file_data.file_context),
    )
}

/// Run inter-procedural purity propagation on function metrics (spec 156)
fn run_purity_propagation(
    metrics: &[FunctionMetrics],
    call_graph: &priority::CallGraph,
) -> Vec<FunctionMetrics> {
    use crate::analysis::call_graph::{
        CrossModuleTracker, FrameworkPatternDetector, FunctionPointerTracker, RustCallGraph,
        TraitRegistry,
    };
    use crate::analysis::purity_analysis::PurityAnalyzer;
    use crate::analysis::purity_propagation::{PurityCallGraphAdapter, PurityPropagator};

    // Subtask 0: Build data flow graph
    // Purity stage is at index 3 (0=files, 1=call graph, 2=coverage, 3=purity, 4=context, 5=debt scoring)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(3, 0, crate::tui::app::StageStatus::Active, None);
    }

    // Create RustCallGraph wrapper around the base call graph
    let rust_graph = RustCallGraph {
        base_graph: call_graph.clone(),
        trait_registry: TraitRegistry::new(),
        function_pointer_tracker: FunctionPointerTracker::new(),
        framework_patterns: FrameworkPatternDetector::new(),
        cross_module_tracker: CrossModuleTracker::new(),
    };

    // Create call graph adapter
    let adapter = PurityCallGraphAdapter::from_rust_graph(rust_graph);

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(3, 0, crate::tui::app::StageStatus::Completed, None);
        // Brief pause to ensure subtask update is visible
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 1: Initial purity detection
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            3,
            1,
            crate::tui::app::StageStatus::Active,
            Some((0, metrics.len())),
        );
    }

    // Create purity analyzer and propagator
    let purity_analyzer = PurityAnalyzer::new();
    let mut propagator = PurityPropagator::new(adapter, purity_analyzer);

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            3,
            1,
            crate::tui::app::StageStatus::Completed,
            Some((metrics.len(), metrics.len())),
        );
        // Brief pause to ensure subtask update is visible
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 2: Purity propagation
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(3, 2, crate::tui::app::StageStatus::Active, None);
    }

    // Run propagation
    if let Err(e) = propagator.propagate(metrics) {
        log::warn!("Purity propagation failed: {}", e);
        if let Some(manager) = crate::progress::ProgressManager::global() {
            manager.tui_update_subtask(3, 2, crate::tui::app::StageStatus::Completed, None);
        }
        return metrics.to_vec();
    }

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(3, 2, crate::tui::app::StageStatus::Completed, None);
        // Brief pause to ensure subtask update is visible
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 3: Side effects analysis
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            3,
            3,
            crate::tui::app::StageStatus::Active,
            Some((0, metrics.len())),
        );
    }

    // Apply results to metrics
    let result = metrics
        .iter()
        .map(|metric| {
            let func_id = priority::call_graph::FunctionId::new(
                metric.file.clone(),
                metric.name.clone(),
                metric.line,
            );

            if let Some(result) = propagator.get_result(&func_id) {
                let mut updated = metric.clone();
                updated.is_pure = Some(
                    result.level == crate::analysis::purity_analysis::PurityLevel::StrictlyPure,
                );
                updated.purity_confidence = Some(result.confidence as f32);
                updated.purity_reason = Some(format!("{:?}", result.reason));
                updated
            } else {
                metric.clone()
            }
        })
        .collect();

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            3,
            3,
            crate::tui::app::StageStatus::Completed,
            Some((metrics.len(), metrics.len())),
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::context::git_history::GitHistoryProvider;
    use crate::risk::context::{ContextAggregator, ContextDetails};
    use std::path::PathBuf;

    #[test]
    fn test_analyze_file_git_context_returns_none_when_no_context_provider() {
        // Create a risk analyzer without context provider
        let risk_analyzer = risk::RiskAnalyzer::default();
        let file_path = PathBuf::from("src/test.rs");
        let project_root = PathBuf::from("/tmp/test_project");

        let result = analyze_file_git_context(&file_path, &risk_analyzer, &project_root);

        assert!(
            result.is_none(),
            "Should return None when context provider is missing"
        );
    }

    #[test]
    fn test_analyze_file_git_context_with_valid_provider() {
        // Create a context provider with test data
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let project_root = temp_dir.path().to_path_buf();

        // Initialize a git repo in the temp directory
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&project_root)
            .output()
            .expect("Failed to init git repo");

        // Configure git user for the test repo
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&project_root)
            .output()
            .expect("Failed to configure git user");

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&project_root)
            .output()
            .expect("Failed to configure git email");

        // Create a test file
        let test_file = project_root.join("src/test.rs");
        std::fs::create_dir_all(test_file.parent().unwrap()).expect("Failed to create src dir");
        std::fs::write(&test_file, "fn test() {}").expect("Failed to write test file");

        // Add and commit the file
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&project_root)
            .output()
            .expect("Failed to git add");

        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&project_root)
            .output()
            .expect("Failed to git commit");

        // Create git history provider and context aggregator
        let git_provider =
            GitHistoryProvider::new(project_root.clone()).expect("Failed to create git provider");

        let context_aggregator = ContextAggregator::new().with_provider(Box::new(git_provider));

        let risk_analyzer =
            risk::RiskAnalyzer::default().with_context_aggregator(context_aggregator);

        let result = analyze_file_git_context(&test_file, &risk_analyzer, &project_root);

        // Should return Some contextual risk when provider is available
        assert!(
            result.is_some(),
            "Should return Some when context provider is available"
        );

        if let Some(contextual_risk) = result {
            // Verify the contextual risk has git_history context
            let git_context = contextual_risk
                .contexts
                .iter()
                .find(|ctx| ctx.provider == "git_history");

            assert!(
                git_context.is_some(),
                "Should have git_history context in contexts"
            );

            // Verify it has Historical details
            if let Some(ctx) = git_context {
                match &ctx.details {
                    ContextDetails::Historical {
                        change_frequency,
                        author_count,
                        age_days,
                        ..
                    } => {
                        assert!(
                            *change_frequency >= 0.0,
                            "Should have valid change frequency"
                        );
                        assert!(*author_count >= 1, "Should have at least one author");
                        // age_days is u32, always >= 0, just verify it exists
                        let _ = age_days;
                    }
                    _ => panic!("Expected Historical context details"),
                }
            }
        }
    }

    #[test]
    fn test_analyze_file_git_context_handles_various_paths() {
        // Create a risk analyzer without context provider for simple path testing
        let risk_analyzer = risk::RiskAnalyzer::default();
        let project_root = PathBuf::from("/tmp/test_project");

        // Test various path formats
        let paths = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/nested/module/file.rs"),
            PathBuf::from("/absolute/path/to/file.rs"),
            PathBuf::from("./relative/path.rs"),
        ];

        for path in paths {
            let result = analyze_file_git_context(&path, &risk_analyzer, &project_root);
            // All should return None since no context provider
            assert!(
                result.is_none(),
                "Should handle path {} correctly",
                path.display()
            );
        }
    }
}
