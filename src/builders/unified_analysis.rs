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
    if coverage_data.is_none() && !quiet_mode && !suppress_coverage_tip {
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
        manager.tui_set_progress(0.71); // ~5/7 stages complete
    }

    // Stage 6: Debt scoring (index 5 in TUI)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_start_stage(5); // debt scoring stage
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
        manager.tui_set_progress(0.86); // ~6/7 stages complete
    }

    // Stage 7: Prioritization (index 6 in TUI - final stage)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_start_stage(6); // prioritization stage
        manager.tui_complete_stage(6, "complete");
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
                match progress.phase {
                    CallGraphPhase::DiscoveringFiles => {
                        manager.tui_update_subtask(
                            2,
                            0,
                            crate::tui::app::StageStatus::Active,
                            None,
                        );
                    }
                    CallGraphPhase::ParsingASTs => {
                        if progress.current == 0 {
                            manager.tui_update_subtask(
                                2,
                                0,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            manager.tui_update_subtask(
                                2,
                                1,
                                crate::tui::app::StageStatus::Active,
                                Some((0, progress.total)),
                            );
                        } else {
                            manager.tui_update_subtask(
                                2,
                                1,
                                crate::tui::app::StageStatus::Active,
                                Some((progress.current, progress.total)),
                            );
                        }
                    }
                    CallGraphPhase::ExtractingCalls => {
                        if progress.current == 0 {
                            manager.tui_update_subtask(
                                2,
                                1,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            manager.tui_update_subtask(
                                2,
                                2,
                                crate::tui::app::StageStatus::Active,
                                Some((0, progress.total)),
                            );
                        } else {
                            manager.tui_update_subtask(
                                2,
                                2,
                                crate::tui::app::StageStatus::Active,
                                Some((progress.current, progress.total)),
                            );
                        }
                    }
                    CallGraphPhase::LinkingModules => {
                        manager.tui_update_subtask(
                            2,
                            2,
                            crate::tui::app::StageStatus::Completed,
                            None,
                        );
                        manager.tui_update_subtask(
                            2,
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
                match progress.phase {
                    CallGraphPhase::DiscoveringFiles => {
                        manager.tui_update_subtask(
                            2,
                            0,
                            crate::tui::app::StageStatus::Active,
                            None,
                        );
                    }
                    CallGraphPhase::ParsingASTs => {
                        if progress.current == 0 {
                            manager.tui_update_subtask(
                                2,
                                0,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            manager.tui_update_subtask(
                                2,
                                1,
                                crate::tui::app::StageStatus::Active,
                                Some((0, progress.total)),
                            );
                        } else {
                            manager.tui_update_subtask(
                                2,
                                1,
                                crate::tui::app::StageStatus::Active,
                                Some((progress.current, progress.total)),
                            );
                        }
                    }
                    CallGraphPhase::ExtractingCalls => {
                        if progress.current == 0 {
                            manager.tui_update_subtask(
                                2,
                                1,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            manager.tui_update_subtask(
                                2,
                                2,
                                crate::tui::app::StageStatus::Active,
                                Some((0, progress.total)),
                            );
                        } else {
                            manager.tui_update_subtask(
                                2,
                                2,
                                crate::tui::app::StageStatus::Active,
                                Some((progress.current, progress.total)),
                            );
                        }
                    }
                    CallGraphPhase::LinkingModules => {
                        manager.tui_update_subtask(
                            2,
                            2,
                            crate::tui::app::StageStatus::Completed,
                            None,
                        );
                        manager.tui_update_subtask(
                            2,
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
            if let Some(manager) = crate::progress::ProgressManager::global() {
                manager.tui_update_subtask(3, 0, crate::tui::app::StageStatus::Active, None);
            }

            let data = risk::lcov::parse_lcov_file_with_callback(&lcov_path, |progress| {
                if let Some(manager) = crate::progress::ProgressManager::global() {
                    match progress {
                        risk::lcov::CoverageProgress::Initializing => {
                            // Subtask 0 complete: open file
                            manager.tui_update_subtask(
                                3,
                                0,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            std::thread::sleep(Duration::from_millis(150));
                            // Subtask 1 start: parse coverage
                            manager.tui_update_subtask(
                                3,
                                1,
                                crate::tui::app::StageStatus::Active,
                                None,
                            );
                        }
                        risk::lcov::CoverageProgress::Parsing { current, total } => {
                            manager.tui_update_subtask(
                                3,
                                1,
                                crate::tui::app::StageStatus::Active,
                                Some((current, total)),
                            );
                        }
                        risk::lcov::CoverageProgress::ComputingStats => {
                            // Subtask 1 complete
                            manager.tui_update_subtask(
                                3,
                                1,
                                crate::tui::app::StageStatus::Completed,
                                None,
                            );
                            std::thread::sleep(Duration::from_millis(150));
                            // Subtask 2 start: compute stats
                            manager.tui_update_subtask(
                                3,
                                2,
                                crate::tui::app::StageStatus::Active,
                                None,
                            );
                        }
                        risk::lcov::CoverageProgress::Complete => {
                            manager.tui_update_subtask(
                                3,
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
                manager.tui_update_subtask(3, 0, crate::tui::app::StageStatus::Completed, None);
                manager.tui_update_subtask(3, 1, crate::tui::app::StageStatus::Completed, None);
                manager.tui_update_subtask(3, 2, crate::tui::app::StageStatus::Completed, None);
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

    // Subtask 0: Initialize (data flow graph, purity, test detection)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(6, 0, crate::tui::app::StageStatus::Active, None);
    }

    // Step 1: Initialize unified analysis with data flow graph
    let mut unified = UnifiedAnalysis::new(call_graph.clone());

    // Step 2: Populate purity analysis
    unified.populate_purity_analysis(metrics);

    // Step 3: Find test-only functions
    let test_only_functions: HashSet<_> =
        call_graph.find_test_only_functions().into_iter().collect();

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(6, 0, crate::tui::app::StageStatus::Completed, None);
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 1: Aggregate debt
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(6, 1, crate::tui::app::StageStatus::Active, None);
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
        manager.tui_update_subtask(6, 1, crate::tui::app::StageStatus::Completed, None);
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 2: Score functions (main computational loop with progress)
    let total_metrics = metrics.len();
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            6,
            2,
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
                        6,
                        2,
                        crate::tui::app::StageStatus::Active,
                        Some((idx + 1, total_metrics)),
                    );
                }
                last_update = Instant::now();
            }
            continue;
        }
        // Create debt item (spec 201: returns None for clean dispatchers)
        if let Some(item) = create_debt_item_from_metric_with_aggregator(
            metric,
            call_graph,
            coverage_data,
            framework_exclusions,
            function_pointer_used_functions,
            &debt_aggregator,
            Some(&unified.data_flow_graph),
            risk_analyzer.as_ref(),
            project_path,
        ) {
            unified.add_item(item);
        }
        // If None (clean dispatcher), skip adding the item - no debt to report

        // Throttled progress update
        if idx % 100 == 0 || last_update.elapsed() > std::time::Duration::from_millis(100) {
            if let Some(manager) = crate::progress::ProgressManager::global() {
                manager.tui_update_subtask(
                    6,
                    2,
                    crate::tui::app::StageStatus::Active,
                    Some((idx + 1, total_metrics)),
                );
            }
            last_update = Instant::now();
        }
    }

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            6,
            2,
            crate::tui::app::StageStatus::Completed,
            Some((total_metrics, total_metrics)),
        );
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 3: Filter results (error analysis, file analysis, sorting, filtering)
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(6, 3, crate::tui::app::StageStatus::Active, None);
    }

    // Step 6: Error swallowing analysis
    if let Some(debt_items) = debt_items {
        let error_swallowing_items = convert_error_swallowing_to_unified(debt_items, call_graph);
        for item in error_swallowing_items {
            unified.add_item(item);
        }
    }

    // Step 7: File-level analysis
    analyze_files_for_debt(&mut unified, metrics, coverage_data, no_god_object);

    // Step 8: File aggregation has been removed - skip to step 9

    // Step 9: Final sorting and impact calculation
    unified.sort_by_priority();
    unified.calculate_total_impact();

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(6, 3, crate::tui::app::StageStatus::Completed, None);
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

    // Subtask 3: Filter results (error analysis, file analysis, sorting, filtering) - PARALLEL
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(6, 3, crate::tui::app::StageStatus::Active, None);
    }

    // Add error swallowing items
    let mut all_items = items;
    if let Some(debt_items) = debt_items {
        let error_swallowing_items = convert_error_swallowing_to_unified(debt_items, call_graph);
        all_items.extend(error_swallowing_items);
    }

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
        manager.tui_update_subtask(6, 3, crate::tui::app::StageStatus::Completed, None);
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
) -> Option<UnifiedDebtItem> {
    // Use the unified debt item creation which already calculates the score correctly (spec 201)
    // Returns None if the debt pattern doesn't warrant a recommendation (e.g., clean dispatcher)
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

fn convert_error_swallowing_to_unified(
    debt_items: &[DebtItem],
    _call_graph: &priority::CallGraph,
) -> Vec<UnifiedDebtItem> {
    debt_items
        .iter()
        .filter(|item| {
            matches!(
                item.debt_type,
                crate::core::DebtType::ErrorSwallowing { .. }
            )
        })
        .map(|item| {
            let unified_score = UnifiedScore {
                complexity_factor: 3.0,
                coverage_factor: 5.0,
                dependency_factor: 4.0,
                role_multiplier: 1.2,
                final_score: 5.5,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            };

            let pattern = item.message.split(':').next().unwrap_or("Error swallowing");
            let context = item.context.clone();

            UnifiedDebtItem {
                location: Location {
                    file: item.file.clone(),
                    function: format!("line_{}", item.line),
                    line: item.line,
                },
                debt_type: DebtType::ErrorSwallowing {
                    pattern: pattern.to_string(),
                    context,
                },
                unified_score,
                function_role: FunctionRole::Unknown,
                recommendation: ActionableRecommendation {
                    primary_action: format!("Fix error swallowing at line {}", item.line),
                    rationale: item.message.clone(),
                    implementation_steps: vec![
                        "Replace error swallowing with proper error handling".to_string(),
                        "Log errors at minimum, even if they can't be handled".to_string(),
                        "Consider propagating errors to caller with ?".to_string(),
                    ],
                    related_items: vec![],
                    steps: None,
                    estimated_effort_hours: None,
                },
                expected_impact: ImpactMetrics {
                    coverage_improvement: 0.0,
                    lines_reduction: 0,
                    complexity_reduction: 0.0,
                    risk_reduction: 3.5,
                },
                transitive_coverage: None,
                file_context: None,
                upstream_dependencies: 0,
                downstream_dependencies: 0,
                upstream_callers: vec![],
                downstream_callees: vec![],
                nesting_depth: 0,
                function_length: 0,
                cyclomatic_complexity: 0,
                cognitive_complexity: 0,
                entropy_details: None,
                entropy_adjusted_cyclomatic: None,
                entropy_adjusted_cognitive: None,
                entropy_dampening_factor: None,
                is_pure: None,
                purity_confidence: None,
                purity_level: None,
                god_object_indicators: None,
                tier: None,
                function_context: None,
                context_confidence: None,
                contextual_recommendation: None,
                pattern_analysis: None,
                context_multiplier: None,
                context_type: None,
                language_specific: None, // No language-specific data for error swallowing items (spec 190)
                detected_pattern: None, // No pattern detection for error swallowing items (spec 204)
                contextual_risk: None,
                file_line_count: None, // No file line count caching for error swallowing items (spec 204)
            }
        })
        .collect()
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
        if let Ok(data) =
            process_single_file(file_path, functions, &file_analyzer, no_god_object, unified)
        {
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
    apply_file_analysis_results(unified, processed_files);
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
}

// Pure function to process a single file
fn process_single_file(
    file_path: PathBuf,
    functions: Vec<FunctionMetrics>,
    file_analyzer: &crate::analyzers::file_analyzer::UnifiedFileAnalyzer,
    no_god_object: bool,
    unified: &UnifiedAnalysis,
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
        file_metrics.god_object_indicators = detect_god_object_indicators(
            file_analyzer,
            file_path,
            content,
            &file_metrics,
            actual_line_count,
        );
    } else {
        file_metrics.god_object_indicators = create_empty_god_object_indicators();
    }

    Ok(file_metrics)
}

// Pure function to calculate uncovered lines
fn calculate_uncovered_lines(coverage_percent: f64, line_count: usize) -> usize {
    ((1.0 - coverage_percent) * line_count as f64) as usize
}

// Pure function to detect god object indicators
fn detect_god_object_indicators(
    file_analyzer: &crate::analyzers::file_analyzer::UnifiedFileAnalyzer,
    file_path: &Path,
    content: &str,
    file_metrics: &FileDebtMetrics,
    actual_line_count: usize,
) -> crate::priority::file_metrics::GodObjectIndicators {
    let mut god_indicators = file_analyzer
        .analyze_file(file_path, content)
        .ok()
        .map(|m| m.god_object_indicators)
        .unwrap_or_else(|| file_metrics.god_object_indicators.clone());

    // Apply size-based god object detection
    if actual_line_count > 2000 || file_metrics.function_count > 50 {
        god_indicators.is_god_object = true;
        if god_indicators.god_object_score == 0.0 {
            god_indicators.god_object_score = (file_metrics.function_count as f64 / 50.0).min(2.0);
        }
    }

    god_indicators
}

// Pure function to create empty god object indicators
fn create_empty_god_object_indicators() -> crate::priority::file_metrics::GodObjectIndicators {
    crate::priority::file_metrics::GodObjectIndicators {
        methods_count: 0,
        fields_count: 0,
        responsibilities: 0,
        is_god_object: false,
        god_object_score: 0.0,
        responsibility_names: Vec::new(),
        recommended_splits: Vec::new(),
        module_structure: None,
        domain_count: 0,
        domain_diversity: 0.0,
        struct_ratio: 0.0,
        analysis_method: crate::priority::file_metrics::SplitAnalysisMethod::None,
        cross_domain_severity: None,
        domain_diversity_metrics: None,
        detection_type: None,
    }
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

/// Convert file_metrics::SplitAnalysisMethod to organization::SplitAnalysisMethod
fn convert_to_org_split_method(
    method: crate::priority::file_metrics::SplitAnalysisMethod,
) -> crate::organization::SplitAnalysisMethod {
    match method {
        crate::priority::file_metrics::SplitAnalysisMethod::None => {
            crate::organization::SplitAnalysisMethod::None
        }
        crate::priority::file_metrics::SplitAnalysisMethod::CrossDomain => {
            crate::organization::SplitAnalysisMethod::CrossDomain
        }
        crate::priority::file_metrics::SplitAnalysisMethod::MethodBased => {
            crate::organization::SplitAnalysisMethod::MethodBased
        }
        crate::priority::file_metrics::SplitAnalysisMethod::Hybrid => {
            crate::organization::SplitAnalysisMethod::Hybrid
        }
        crate::priority::file_metrics::SplitAnalysisMethod::TypeBased => {
            crate::organization::SplitAnalysisMethod::TypeBased
        }
    }
}

/// Convert file_metrics::RecommendationSeverity to organization::RecommendationSeverity
fn convert_to_org_severity(
    severity: crate::priority::file_metrics::RecommendationSeverity,
) -> crate::organization::RecommendationSeverity {
    match severity {
        crate::priority::file_metrics::RecommendationSeverity::Critical => {
            crate::organization::RecommendationSeverity::Critical
        }
        crate::priority::file_metrics::RecommendationSeverity::High => {
            crate::organization::RecommendationSeverity::High
        }
        crate::priority::file_metrics::RecommendationSeverity::Medium => {
            crate::organization::RecommendationSeverity::Medium
        }
        crate::priority::file_metrics::RecommendationSeverity::Low => {
            crate::organization::RecommendationSeverity::Low
        }
    }
}

// Pure function to create god object analysis
fn create_god_object_analysis(
    file_metrics: &FileDebtMetrics,
) -> Option<crate::organization::GodObjectAnalysis> {
    if !file_metrics.god_object_indicators.is_god_object {
        return None;
    }

    Some(crate::organization::GodObjectAnalysis {
        is_god_object: file_metrics.god_object_indicators.is_god_object,
        method_count: file_metrics.god_object_indicators.methods_count,
        field_count: file_metrics.god_object_indicators.fields_count,
        responsibility_count: file_metrics.god_object_indicators.responsibilities,
        lines_of_code: file_metrics.total_lines,
        complexity_sum: file_metrics.total_complexity,
        god_object_score: file_metrics.god_object_indicators.god_object_score * 100.0,
        recommended_splits: Vec::new(),
        confidence: crate::organization::GodObjectConfidence::Definite,
        responsibilities: Vec::new(),
        purity_distribution: None,
        module_structure: file_metrics.god_object_indicators.module_structure.clone(),
        detection_type: crate::organization::DetectionType::GodFile,
        visibility_breakdown: None, // Spec 134: Added for compatibility
        domain_count: file_metrics.god_object_indicators.domain_count,
        domain_diversity: file_metrics.god_object_indicators.domain_diversity,
        struct_ratio: file_metrics.god_object_indicators.struct_ratio,
        analysis_method: convert_to_org_split_method(
            file_metrics.god_object_indicators.analysis_method,
        ),
        cross_domain_severity: file_metrics
            .god_object_indicators
            .cross_domain_severity
            .map(convert_to_org_severity),
        domain_diversity_metrics: None, // Spec 152: Added for struct-based analysis
    })
}

/// Pure function to create a UnifiedDebtItem from god object indicators (spec 207)
///
/// God objects are file-level technical debt items representing files with
/// too many responsibilities, methods, or fields. They bypass function-level
/// complexity filtering since they represent architectural issues rather than
/// individual function complexity.
fn create_god_object_debt_item(
    file_path: &Path,
    file_metrics: &FileDebtMetrics,
    god_analysis: &crate::organization::GodObjectAnalysis,
) -> UnifiedDebtItem {
    // Calculate unified score based on god object score (0-100 scale)
    let base_score = god_analysis.god_object_score;
    let tier = if base_score >= 50.0 { 1 } else { 2 };

    let unified_score = UnifiedScore {
        final_score: base_score,
        complexity_factor: file_metrics.total_complexity as f64 / 10.0,
        coverage_factor: 0.0, // File-level item, no coverage score
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

    // Determine debt type based on detection type
    let debt_type = match god_analysis.detection_type {
        crate::organization::DetectionType::GodClass => DebtType::GodObject {
            methods: god_analysis.method_count as u32,
            fields: god_analysis.field_count as u32,
            responsibilities: god_analysis.responsibility_count as u32,
            god_object_score: god_analysis.god_object_score,
        },
        crate::organization::DetectionType::GodFile
        | crate::organization::DetectionType::GodModule => DebtType::GodModule {
            functions: god_analysis.method_count as u32,
            lines: god_analysis.lines_of_code as u32,
            responsibilities: god_analysis.responsibility_count as u32,
        },
    };

    // Extract file name for display
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

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
            function: file_name.to_string(),
            line: 1, // File-level item starts at line 1
        },
        debt_type,
        unified_score,
        function_role: FunctionRole::Unknown, // Not applicable for file-level items
        recommendation,
        expected_impact,
        transitive_coverage: None,
        upstream_dependencies: 0,
        downstream_dependencies: 0,
        upstream_callers: Vec::new(),
        downstream_callees: Vec::new(),
        nesting_depth: 0,
        function_length: god_analysis.lines_of_code,
        cyclomatic_complexity: 0, // File-level metric, not function-level
        cognitive_complexity: 0,  // File-level metric, not function-level
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
        contextual_risk: None,
        file_line_count: Some(god_analysis.lines_of_code),
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
    let split_count = god_analysis.recommended_splits.len().max(1);

    let primary_action = match god_analysis.detection_type {
        crate::organization::DetectionType::GodClass => {
            format!(
                "Split god object into {} modules by responsibility",
                split_count
            )
        }
        crate::organization::DetectionType::GodFile
        | crate::organization::DetectionType::GodModule => {
            format!("Split god module into {} focused modules", split_count)
        }
    };

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

// I/O function to apply results to unified analysis
fn apply_file_analysis_results(
    unified: &mut UnifiedAnalysis,
    processed_files: Vec<ProcessedFileData>,
) {
    for file_data in processed_files {
        // Update god object indicators for functions in this file
        if let Some(god_analysis) = &file_data.god_analysis {
            update_function_god_indicators(unified, &file_data.file_path, god_analysis);

            // NEW (spec 207): Create god object debt item for TUI display
            let god_item = create_god_object_debt_item(
                &file_data.file_path,
                &file_data.file_metrics,
                god_analysis,
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
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(5, 0, crate::tui::app::StageStatus::Active, None);
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
        manager.tui_update_subtask(5, 0, crate::tui::app::StageStatus::Completed, None);
        // Brief pause to ensure subtask update is visible
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 1: Initial purity detection
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            5,
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
            5,
            1,
            crate::tui::app::StageStatus::Completed,
            Some((metrics.len(), metrics.len())),
        );
        // Brief pause to ensure subtask update is visible
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 2: Purity propagation
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(5, 2, crate::tui::app::StageStatus::Active, None);
    }

    // Run propagation
    if let Err(e) = propagator.propagate(metrics) {
        log::warn!("Purity propagation failed: {}", e);
        if let Some(manager) = crate::progress::ProgressManager::global() {
            manager.tui_update_subtask(5, 2, crate::tui::app::StageStatus::Completed, None);
        }
        return metrics.to_vec();
    }

    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(5, 2, crate::tui::app::StageStatus::Completed, None);
        // Brief pause to ensure subtask update is visible
        std::thread::sleep(std::time::Duration::from_millis(150));
    }

    // Subtask 3: Side effects analysis
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(
            5,
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
            5,
            3,
            crate::tui::app::StageStatus::Completed,
            Some((metrics.len(), metrics.len())),
        );
    }

    result
}
