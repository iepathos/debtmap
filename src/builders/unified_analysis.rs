//! Unified analysis orchestration with progress reporting.
//!
//! This module provides the entry points for unified analysis with progress/TUI
//! handling. All pure computation is delegated to `unified_analysis_phases`.
//!
//! Following Stillwater philosophy: Pure core (phases/), imperative shell (this file).

use super::{call_graph, parallel_call_graph, parallel_unified_analysis};
use crate::observability::{AnalysisPhase, set_phase_persistent, set_progress};
use crate::time_span;
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
use crate::core::{AnalysisResults, Language};
use crate::debt::suppression::parse_suppression_comments;
use crate::organization::GodObjectAnalysis;
use crate::priority::{
    DebtType, UnifiedAnalysis, UnifiedAnalysisUtils, UnifiedDebtItem,
    call_graph::{CallGraph, FunctionId},
};
use crate::risk;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const ANALYSIS_THREAD_STACK_SIZE: usize = 8 * 1024 * 1024;

/// Main entry point for unified analysis (simple version).
pub fn perform_unified_analysis(
    results: &AnalysisResults,
    coverage_file: Option<&PathBuf>,
    semantic_off: bool,
    project_path: &Path,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
) -> Result<UnifiedAnalysis> {
    let now = chrono::Utc::now();
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
        reference_time: now,
    })
}

/// Main entry point for unified analysis with full options.
///
/// debtmap:ignore\[complexity,testing\] - I/O orchestrator coordinating 5 analysis phases
/// with progress reporting. Pure logic delegated to core::phases::* modules.
/// Helper functions phase_to_subtask_index/build_progress_info are tested.
pub fn perform_unified_analysis_with_options(
    options: UnifiedAnalysisOptions,
) -> Result<UnifiedAnalysis> {
    time_span!("unified_analysis");

    // Create top-level span for unified analysis (spec 208)
    let span = info_span!(
        "unified_analysis",
        project = %options.project_path.display(),
        file_count = options.results.complexity.metrics.len(),
        parallel = options.parallel,
    );
    let _guard = span.enter();

    info!(
        file_count = options.results.complexity.metrics.len(),
        "Starting unified analysis"
    );

    // Set total file count for crash report progress tracking (spec 207)
    set_progress(0, options.results.complexity.metrics.len());

    let CallGraphStageResult {
        mut call_graph,
        framework_exclusions,
        function_pointer_used_functions,
        elapsed: call_graph_time,
    } = build_call_graph_stage(
        options.results,
        CallGraphStageOptions {
            project_path: options.project_path,
            parallel: options.parallel,
            jobs: options.jobs,
            verbose_macro_warnings: options.verbose_macro_warnings,
            show_macro_stats: options.show_macro_stats,
            rust_files: options.rust_files.as_deref(),
            extracted_data: options.extracted_data.as_ref(),
        },
    )?;

    // Apply trait patterns
    core::phases::call_graph::apply_trait_patterns(&mut call_graph);

    let (coverage_data, coverage_time) = load_coverage_stage(options.coverage_file)?;

    emit_coverage_tip(coverage_data.is_none(), options.suppress_coverage_tip);

    // Enrich metrics with call graph data
    let enriched_metrics = call_graph_integration::populate_call_graph_data(
        options.results.complexity.metrics.clone(),
        &call_graph,
    );

    let enriched_metrics = run_purity_stage(&enriched_metrics, &call_graph);
    let risk_analyzer = load_context_stage(ContextStageOptions {
        project_path: options.project_path,
        enable_context: options.enable_context,
        context_providers: options.context_providers,
        disable_context: options.disable_context,
        results: options.results,
        reference_time: options.reference_time,
    });
    let result = run_debt_scoring_stage(DebtScoringOptions {
        enriched_metrics: &enriched_metrics,
        call_graph: &call_graph,
        coverage_data: coverage_data.as_ref(),
        framework_exclusions: &framework_exclusions,
        function_pointer_used_functions: &function_pointer_used_functions,
        debt_items: &options.results.technical_debt.items,
        no_aggregation: options.no_aggregation,
        aggregation_method: options.aggregation_method,
        min_problematic: options.min_problematic,
        no_god_object: options.no_god_object,
        call_graph_time,
        coverage_time,
        risk_analyzer,
        project_path: options.project_path,
        parallel: options.parallel,
        jobs: options.jobs,
        extracted_data: options.extracted_data,
        reference_time: options.reference_time,
    });

    info!(
        total_items = result.items.len(),
        file_items = result.file_items.len(),
        "Unified analysis complete"
    );

    Ok(result)
}

struct CallGraphStageOptions<'a> {
    project_path: &'a Path,
    parallel: bool,
    jobs: usize,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    rust_files: Option<&'a [PathBuf]>,
    extracted_data:
        Option<&'a std::collections::HashMap<PathBuf, crate::extraction::ExtractedFileData>>,
}

struct CallGraphStageResult {
    call_graph: CallGraph,
    framework_exclusions: HashSet<FunctionId>,
    function_pointer_used_functions: HashSet<FunctionId>,
    elapsed: std::time::Duration,
}

fn build_call_graph_stage(
    results: &AnalysisResults,
    options: CallGraphStageOptions<'_>,
) -> Result<CallGraphStageResult> {
    let mut call_graph = call_graph::build_initial_call_graph(&results.complexity.metrics);

    report_stage_start(1);
    let start = std::time::Instant::now();
    let (framework_exclusions, function_pointer_used_functions) =
        build_rust_call_graph(&mut call_graph, &options)?;
    process_js_ts_call_graph(results, options.project_path, &mut call_graph);

    let elapsed = start.elapsed();
    report_stage_complete(1, format!("{} functions", call_graph.node_count()));

    Ok(CallGraphStageResult {
        call_graph,
        framework_exclusions,
        function_pointer_used_functions,
        elapsed,
    })
}

fn build_rust_call_graph(
    call_graph: &mut CallGraph,
    options: &CallGraphStageOptions<'_>,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    time_span!("call_graph_building", parent: "unified_analysis");
    let _span = info_span!("call_graph_building").entered();
    info!("Building call graph");

    let result = match options.extracted_data {
        Some(extracted) => build_extracted_call_graph(call_graph, extracted),
        None if options.parallel => build_call_graph_with_progress(
            options.project_path,
            call_graph,
            options.jobs,
            true,
            options.rust_files,
        )?,
        None => build_call_graph_with_progress_sequential(
            options.project_path,
            call_graph,
            options.verbose_macro_warnings,
            options.show_macro_stats,
            options.rust_files,
        )?,
    };

    debug!(functions = call_graph.node_count(), "Call graph built");
    Ok(result)
}

fn build_extracted_call_graph(
    call_graph: &mut CallGraph,
    extracted: &std::collections::HashMap<PathBuf, crate::extraction::ExtractedFileData>,
) -> (HashSet<FunctionId>, HashSet<FunctionId>) {
    info!("Building call graph from extracted data (spec 214)");
    let (graph, exclusions, fn_pointers) =
        parallel_call_graph::build_call_graph_from_extracted(call_graph.clone(), extracted);
    *call_graph = graph;
    (exclusions, fn_pointers)
}

fn process_js_ts_call_graph(
    results: &AnalysisResults,
    project_path: &Path,
    call_graph: &mut CallGraph,
) {
    let js_ts_files = collect_js_ts_files(results);
    if js_ts_files.is_empty() {
        return;
    }

    time_span!("typescript_call_graph", parent: "unified_analysis");
    let _span = info_span!("typescript_call_graph_building").entered();
    info!(
        "Processing {} JS/TS files for call graph",
        js_ts_files.len()
    );

    if let Err(e) = call_graph::process_typescript_files_for_call_graph(
        project_path,
        call_graph,
        Some(&js_ts_files),
    ) {
        warn!("Failed to process TypeScript call graph: {}", e);
    }
}

fn collect_js_ts_files(results: &AnalysisResults) -> Vec<PathBuf> {
    let mut files: Vec<_> = results
        .complexity
        .metrics
        .iter()
        .filter(|m| is_js_ts_file(&m.file))
        .map(|m| m.file.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    files.sort();
    files
}

fn is_js_ts_file(file: &Path) -> bool {
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(
        ext,
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts"
    )
}

fn load_coverage_stage(
    coverage_file: Option<&PathBuf>,
) -> Result<(Option<risk::lcov::LcovData>, std::time::Duration)> {
    report_stage_start(2);
    let start = std::time::Instant::now();
    let coverage_data = load_coverage_data(coverage_file)?;

    update_tui_coverage(coverage_data.as_ref());
    report_stage_complete(2, coverage_stage_metric(coverage_data.as_ref()));

    Ok((coverage_data, start.elapsed()))
}

fn load_coverage_data(coverage_file: Option<&PathBuf>) -> Result<Option<risk::lcov::LcovData>> {
    time_span!("coverage_loading", parent: "unified_analysis");
    let _span = info_span!("coverage_loading").entered();
    info!("Loading coverage data");

    let data = core::phases::coverage::load_coverage_data(coverage_file.cloned())?;
    if data.is_some() {
        debug!("Coverage data loaded");
    } else {
        debug!("No coverage data provided");
    }
    Ok(data)
}

fn update_tui_coverage(coverage_data: Option<&risk::lcov::LcovData>) {
    if let Some(manager) = crate::progress::ProgressManager::global() {
        let coverage_percent = core::phases::coverage::calculate_coverage_percent(coverage_data);
        manager.tui_update_coverage(coverage_percent);
    }
}

fn coverage_stage_metric(coverage_data: Option<&risk::lcov::LcovData>) -> &'static str {
    if coverage_data.is_some() {
        "loaded"
    } else {
        "skipped"
    }
}

fn run_purity_stage(
    enriched_metrics: &[crate::core::FunctionMetrics],
    call_graph: &CallGraph,
) -> Vec<crate::core::FunctionMetrics> {
    report_stage_start(3);
    let enriched_metrics = {
        time_span!("purity_analysis", parent: "unified_analysis");
        let _span = info_span!("purity_analysis").entered();
        info!("Analyzing function purity");
        let result = core::orchestration::run_purity_propagation(enriched_metrics, call_graph);
        debug!(functions = result.len(), "Purity analysis complete");
        result
    };
    report_stage_complete(3, format!("{} functions analyzed", enriched_metrics.len()));
    enriched_metrics
}

struct ContextStageOptions<'a> {
    project_path: &'a Path,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    results: &'a AnalysisResults,
    reference_time: chrono::DateTime<chrono::Utc>,
}

fn load_context_stage(options: ContextStageOptions<'_>) -> Option<risk::RiskAnalyzer> {
    report_stage_start(4);
    let risk_analyzer = {
        time_span!("context_loading", parent: "unified_analysis");
        let _span = info_span!("context_loading").entered();
        info!("Loading context providers");
        let risk_analyzer = build_risk_analyzer(
            options.project_path,
            options.enable_context,
            options.context_providers,
            options.disable_context,
            options.results,
            options.reference_time,
        );
        if risk_analyzer.is_some() {
            debug!("Context providers loaded");
        } else {
            debug!("Context analysis disabled or not available");
        }
        risk_analyzer
    };
    report_stage_complete(4, context_stage_metric(options.enable_context));
    risk_analyzer
}

fn context_stage_metric(enable_context: bool) -> &'static str {
    if enable_context { "loaded" } else { "skipped" }
}

struct DebtScoringOptions<'a> {
    enriched_metrics: &'a [crate::core::FunctionMetrics],
    call_graph: &'a CallGraph,
    coverage_data: Option<&'a risk::lcov::LcovData>,
    framework_exclusions: &'a HashSet<FunctionId>,
    function_pointer_used_functions: &'a HashSet<FunctionId>,
    debt_items: &'a [crate::core::DebtItem],
    no_aggregation: bool,
    aggregation_method: Option<String>,
    min_problematic: Option<usize>,
    no_god_object: bool,
    call_graph_time: std::time::Duration,
    coverage_time: std::time::Duration,
    risk_analyzer: Option<risk::RiskAnalyzer>,
    project_path: &'a Path,
    parallel: bool,
    jobs: usize,
    extracted_data:
        Option<std::collections::HashMap<PathBuf, crate::extraction::ExtractedFileData>>,
    reference_time: chrono::DateTime<chrono::Utc>,
}

fn run_debt_scoring_stage(options: DebtScoringOptions<'_>) -> UnifiedAnalysis {
    report_stage_start(5);
    let result = {
        time_span!("debt_scoring", parent: "unified_analysis");
        let _span = info_span!("debt_scoring").entered();
        info!("Scoring technical debt items");
        let result = create_unified_analysis_with_exclusions_and_timing(
            options.enriched_metrics,
            options.call_graph,
            options.coverage_data,
            options.framework_exclusions,
            Some(options.function_pointer_used_functions),
            Some(options.debt_items),
            options.no_aggregation,
            options.aggregation_method,
            options.min_problematic,
            options.no_god_object,
            options.call_graph_time,
            options.coverage_time,
            options.risk_analyzer,
            options.project_path,
            options.parallel,
            options.jobs,
            options.extracted_data,
            options.reference_time,
        );
        debug!(
            item_count = result.items.len(),
            file_items = result.file_items.len(),
            "Debt scoring complete"
        );
        result
    };
    report_stage_complete(5, format!("{} items scored", result.items.len()));
    result
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
    reference_time: chrono::DateTime<chrono::Utc>,
) -> UnifiedAnalysis {
    let enriched_metrics =
        call_graph_integration::populate_call_graph_data(metrics.to_vec(), call_graph);
    create_unified_analysis_with_exclusions_and_timing(
        &enriched_metrics,
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
        None, // Compatibility path uses metric facts without hidden source I/O
        reference_time,
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
    reference_time: chrono::DateTime<chrono::Utc>,
) -> UnifiedAnalysis {
    create_scheduled_analysis(
        metrics,
        call_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        debt_items,
        no_god_object,
        parallel,
        jobs,
        call_graph_time,
        coverage_time,
        risk_analyzer,
        project_path,
        extracted_data,
        reference_time,
    )
}

#[allow(clippy::too_many_arguments)]
fn create_scheduled_analysis(
    metrics: &[crate::core::FunctionMetrics],
    call_graph: &CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_items: Option<&[crate::core::DebtItem]>,
    no_god_object: bool,
    parallel: bool,
    jobs: usize,
    call_graph_time: std::time::Duration,
    coverage_time: std::time::Duration,
    risk_analyzer: Option<risk::RiskAnalyzer>,
    project_path: &Path,
    extracted_data: Option<
        std::collections::HashMap<PathBuf, crate::extraction::ExtractedFileData>,
    >,
    reference_time: chrono::DateTime<chrono::Utc>,
) -> UnifiedAnalysis {
    use parallel_unified_analysis::ParallelUnifiedAnalysisOptions;

    let options = ParallelUnifiedAnalysisOptions {
        parallel,
        jobs: if jobs > 0 { Some(jobs) } else { None },
        batch_size: 100,
        progress: std::env::var("DEBTMAP_QUIET").is_err(),
        reference_time,
    };

    with_analysis_pool(if parallel { jobs } else { 0 }, || {
        execute_parallel_analysis(
            metrics,
            call_graph,
            coverage_data,
            framework_exclusions,
            function_pointer_used_functions,
            debt_items,
            no_god_object,
            call_graph_time,
            coverage_time,
            risk_analyzer,
            project_path,
            extracted_data,
            options,
        )
    })
}

fn with_analysis_pool<T, F>(jobs: usize, operation: F) -> T
where
    T: Send,
    F: FnOnce() -> T + Send,
{
    if jobs == 0 {
        return operation();
    }

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .stack_size(ANALYSIS_THREAD_STACK_SIZE)
        .build();
    match pool {
        Ok(pool) => pool.install(operation),
        Err(error) => {
            log::warn!("Unable to configure {jobs} analysis workers: {error}");
            operation()
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_parallel_analysis(
    metrics: &[crate::core::FunctionMetrics],
    call_graph: &CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    debt_items: Option<&[crate::core::DebtItem]>,
    no_god_object: bool,
    call_graph_time: std::time::Duration,
    coverage_time: std::time::Duration,
    risk_analyzer: Option<risk::RiskAnalyzer>,
    project_path: &Path,
    extracted_data: Option<
        std::collections::HashMap<PathBuf, crate::extraction::ExtractedFileData>,
    >,
    options: parallel_unified_analysis::ParallelUnifiedAnalysisOptions,
) -> UnifiedAnalysis {
    use parallel_unified_analysis::ParallelUnifiedAnalysisBuilder;

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

    let items = builder.execute_phase2_parallel(
        metrics,
        &test_only_functions,
        &debt_aggregator,
        &data_flow_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
    );

    let file_items = builder.execute_phase3_parallel(metrics, coverage_data, no_god_object);

    let (mut unified, timings) =
        builder.build(data_flow_graph, purity, items, file_items, coverage_data);

    unified.timings = Some(timings);
    unified
}

/// Check if a god object should be suppressed based on file annotations.
/// Same logic as orchestration.rs - checks both file-level and struct-level suppressions.
fn is_god_object_suppressed_unified(
    god_analysis: &GodObjectAnalysis,
    file_content: &str,
    file_path: &std::path::Path,
) -> bool {
    use crate::organization::DetectionType;

    // Determine language from file extension
    let language = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext {
            "rs" => Language::Rust,
            "py" | "pyw" => Language::Python,
            _ => Language::Rust,
        })
        .unwrap_or(Language::Rust);

    let suppression_context = parse_suppression_comments(file_content, language, file_path);

    // Create a representative GodObject debt type for suppression checking
    let god_object_debt_type = DebtType::GodObject {
        methods: god_analysis.method_count as u32,
        fields: Some(god_analysis.field_count as u32),
        responsibilities: god_analysis.responsibility_count as u32,
        god_object_score: god_analysis.god_object_score,
        lines: god_analysis.lines_of_code as u32,
    };

    // First, always check for file-level suppression at the top of the file
    // A file-level annotation applies to all god objects in the file
    for check_line in 1..=6 {
        if suppression_context.is_suppressed(check_line, &god_object_debt_type) {
            return true;
        }
        if suppression_context.is_function_allowed(check_line, &god_object_debt_type) {
            return true;
        }
    }

    // For GodClass, also check near the struct definition line
    if let DetectionType::GodClass = god_analysis.detection_type {
        let struct_line = god_analysis.struct_line.unwrap_or(1);
        if suppression_context.is_suppressed(struct_line, &god_object_debt_type) {
            return true;
        }
        if suppression_context.is_function_allowed(struct_line, &god_object_debt_type) {
            return true;
        }
    }

    false
}

#[allow(clippy::too_many_arguments)]
pub(super) fn finalize_file_item_with_content(
    unified: &mut UnifiedAnalysis,
    mut file_item: crate::priority::FileDebtItem,
    raw_functions: &[crate::core::FunctionMetrics],
    coverage_data: Option<&risk::lcov::LcovData>,
    risk_analyzer: Option<&risk::RiskAnalyzer>,
    project_path: &Path,
    call_graph: &CallGraph,
    file_content: Option<&str>,
) -> crate::priority::FileDebtItem {
    let god_analysis = file_item
        .metrics
        .god_object_analysis
        .clone()
        .filter(|analysis| analysis.is_god_object);

    if let Some(god_analysis) = god_analysis {
        add_god_object_item(
            unified,
            &mut file_item,
            &god_analysis,
            raw_functions,
            coverage_data,
            risk_analyzer,
            project_path,
            call_graph,
            file_content,
        );
    }

    enrich_file_item_with_dependencies(file_item, &unified.items)
}

#[allow(clippy::too_many_arguments)]
fn add_god_object_item(
    unified: &mut UnifiedAnalysis,
    file_item: &mut crate::priority::FileDebtItem,
    god_analysis: &crate::organization::GodObjectAnalysis,
    raw_functions: &[crate::core::FunctionMetrics],
    coverage_data: Option<&risk::lcov::LcovData>,
    risk_analyzer: Option<&risk::RiskAnalyzer>,
    project_path: &Path,
    call_graph: &CallGraph,
    file_content: Option<&str>,
) {
    if is_god_object_suppressed_for_file(file_item, god_analysis, file_content) {
        file_item.metrics.god_object_analysis = None;
        return;
    }

    let (god_item, enriched) = build_god_object_item(
        &file_item.metrics.path,
        &file_item.metrics,
        god_analysis,
        raw_functions,
        coverage_data,
        risk_analyzer,
        project_path,
        unified,
        call_graph,
    );

    for item in unified.items.iter_mut() {
        if item.location.file == file_item.metrics.path {
            item.god_object_indicators = Some(enriched.clone());
        }
    }

    unified.add_item(god_item);
}

#[allow(clippy::too_many_arguments)]
fn build_god_object_item(
    file_path: &Path,
    file_metrics: &crate::priority::file_metrics::FileDebtMetrics,
    god_analysis: &crate::organization::GodObjectAnalysis,
    raw_functions: &[crate::core::FunctionMetrics],
    coverage_data: Option<&risk::lcov::LcovData>,
    risk_analyzer: Option<&risk::RiskAnalyzer>,
    project_path: &Path,
    unified: &UnifiedAnalysis,
    call_graph: &CallGraph,
) -> (UnifiedDebtItem, crate::organization::GodObjectAnalysis) {
    use crate::priority::context::{ContextConfig, generate_context_suggestion};
    use crate::priority::god_object_aggregation::aggregate_god_object_metrics_with_coverage;

    // Scope aggregation to the detected god object's methods (GodClass) or
    // keep file-wide aggregation for GodFile/GodModule. See
    // `filter_god_object_member_metrics` for the scoping rule.
    let mut aggregated =
        aggregate_god_object_metrics_with_coverage(raw_functions, god_analysis, coverage_data);

    aggregated.aggregated_contextual_risk = risk_analyzer
        .and_then(|analyzer| {
            core::phases::god_object::analyze_file_git_context(file_path, analyzer, project_path)
        })
        .or_else(|| member_contextual_risk(unified, file_path));

    let enriched =
        core::phases::god_object::enrich_god_analysis_with_aggregates(god_analysis, &aggregated);

    let mut god_item = core::phases::god_object::create_god_object_debt_item(
        file_path,
        file_metrics,
        &enriched,
        aggregated,
        coverage_data,
        Some(call_graph),
    );

    let context_config = ContextConfig::default();
    god_item.context_suggestion =
        generate_context_suggestion(&god_item, call_graph, &context_config);

    (god_item, enriched)
}

fn is_god_object_suppressed_for_file(
    file_item: &crate::priority::FileDebtItem,
    god_analysis: &crate::organization::GodObjectAnalysis,
    file_content: Option<&str>,
) -> bool {
    file_content.is_some_and(|content| {
        is_god_object_suppressed_unified(god_analysis, content, &file_item.metrics.path)
    })
}

fn member_contextual_risk(
    unified: &UnifiedAnalysis,
    file_path: &Path,
) -> Option<crate::risk::context::ContextualRisk> {
    use crate::priority::god_object_aggregation::{
        aggregate_god_object_metrics, extract_member_functions,
    };

    let members = extract_member_functions(unified.items.iter(), file_path);
    (!members.is_empty())
        .then(|| aggregate_god_object_metrics(&members).aggregated_contextual_risk)
        .flatten()
}

fn enrich_file_item_with_dependencies(
    mut file_item: crate::priority::FileDebtItem,
    unified_items: &crate::collections::Vector<crate::priority::UnifiedDebtItem>,
) -> crate::priority::FileDebtItem {
    use crate::priority::god_object_aggregation::{
        aggregate_dependency_metrics, extract_member_functions,
    };

    let members = extract_member_functions(unified_items.iter(), &file_item.metrics.path);
    let (callers, callees, afferent, efferent) = aggregate_dependency_metrics(&members);
    let mut callers: Vec<_> = callers.into_iter().collect();
    let mut callees: Vec<_> = callees.into_iter().collect();
    callers.sort();
    callees.sort();
    file_item.metrics.afferent_coupling = afferent;
    file_item.metrics.efferent_coupling = efferent;
    file_item.metrics.instability =
        crate::output::unified::calculate_instability(afferent, efferent);
    file_item.metrics.dependents = callers.into_iter().take(10).collect();
    file_item.metrics.dependencies_list = callees.into_iter().take(10).collect();
    file_item
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

// ============================================================================
// Call Graph Progress Helpers
// ============================================================================

/// Maps a `CallGraphPhase` to its corresponding TUI subtask index.
///
/// Returns `None` for phases that shouldn't be displayed (e.g., `DiscoveringFiles`
/// is skipped because files are reused from stage 0).
///
/// Subtask indices for stage 1 (call graph building):
/// - 0: Parse ASTs
/// - 1: Extract calls
/// - 2: Link modules
#[inline]
fn phase_to_subtask_index(phase: parallel_call_graph::CallGraphPhase) -> Option<usize> {
    use crate::builders::parallel_call_graph::CallGraphPhase;
    match phase {
        CallGraphPhase::DiscoveringFiles => None,
        CallGraphPhase::ParsingASTs => Some(0),
        CallGraphPhase::ExtractingCalls => Some(1),
        CallGraphPhase::LinkingModules => Some(2),
    }
}

/// Converts raw progress counters to progress info tuple.
///
/// Returns `None` if total is 0 (no progress to report).
#[inline]
fn build_progress_info(current: usize, total: usize) -> Option<(usize, usize)> {
    if total > 0 {
        Some((current, total))
    } else {
        None
    }
}

/// Updates TUI subtask status with proper phase transition handling.
///
/// Handles:
/// - Marking the previous subtask as completed when transitioning to a new phase
/// - Updating the current subtask as active with progress info
fn update_tui_subtask(
    manager: &crate::progress::ProgressManager,
    last_subtask: &mut usize,
    new_subtask: usize,
    progress_info: Option<(usize, usize)>,
) {
    use crate::tui::app::StageStatus;
    const CALL_GRAPH_STAGE: usize = 1;

    // Mark previous subtask as completed if we moved to a new phase
    if *last_subtask != usize::MAX && *last_subtask != new_subtask {
        manager.tui_update_subtask(
            CALL_GRAPH_STAGE,
            *last_subtask,
            StageStatus::Completed,
            None,
        );
    }
    *last_subtask = new_subtask;

    manager.tui_update_subtask(
        CALL_GRAPH_STAGE,
        new_subtask,
        StageStatus::Active,
        progress_info,
    );
}

/// Finalizes TUI progress by marking the last subtask as completed.
fn finalize_tui_progress(last_subtask: usize) {
    use crate::tui::app::StageStatus;
    const CALL_GRAPH_STAGE: usize = 1;

    if let Some(manager) = crate::progress::ProgressManager::global()
        && last_subtask != usize::MAX
    {
        manager.tui_update_subtask(CALL_GRAPH_STAGE, last_subtask, StageStatus::Completed, None);
    }
}

fn build_call_graph_with_progress(
    project_path: &Path,
    call_graph: &mut CallGraph,
    jobs: usize,
    _parallel: bool,
    rust_files: Option<&[PathBuf]>,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    use crate::tui::app::StageStatus;
    use std::sync::atomic::{AtomicUsize, Ordering};
    const CALL_GRAPH_STAGE: usize = 1;

    let thread_count = if jobs == 0 { None } else { Some(jobs) };
    let last_subtask = AtomicUsize::new(usize::MAX);

    let (graph, exclusions, used_funcs) =
        parallel_call_graph::build_call_graph_parallel_with_files(
            project_path,
            call_graph.clone(),
            thread_count,
            rust_files,
            |progress| {
                let Some(subtask_index) = phase_to_subtask_index(progress.phase) else {
                    return;
                };

                if let Some(manager) = crate::progress::ProgressManager::global() {
                    let prev = last_subtask.swap(subtask_index, Ordering::Relaxed);
                    if prev != usize::MAX && prev != subtask_index {
                        manager.tui_update_subtask(
                            CALL_GRAPH_STAGE,
                            prev,
                            StageStatus::Completed,
                            None,
                        );
                    }
                    manager.tui_update_subtask(
                        CALL_GRAPH_STAGE,
                        subtask_index,
                        StageStatus::Active,
                        build_progress_info(progress.current, progress.total),
                    );
                }
            },
        )?;

    finalize_tui_progress(last_subtask.load(Ordering::Relaxed));

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
    use std::cell::Cell;

    let last_subtask = Cell::new(usize::MAX);

    let result = call_graph::process_rust_files_for_call_graph_with_files(
        project_path,
        call_graph,
        verbose_macro_warnings,
        show_macro_stats,
        rust_files,
        |progress| {
            let Some(subtask_index) = phase_to_subtask_index(progress.phase) else {
                return;
            };

            if let Some(ref manager) = crate::progress::ProgressManager::global() {
                let mut last = last_subtask.get();
                update_tui_subtask(
                    manager,
                    &mut last,
                    subtask_index,
                    build_progress_info(progress.current, progress.total),
                );
                last_subtask.set(last);
            }
        },
    );

    finalize_tui_progress(last_subtask.get());
    result
}

fn build_risk_analyzer(
    project_path: &Path,
    enable_context: bool,
    context_providers: Option<Vec<String>>,
    disable_context: Option<Vec<String>>,
    results: &AnalysisResults,
    reference_time: chrono::DateTime<chrono::Utc>,
) -> Option<risk::RiskAnalyzer> {
    if !enable_context {
        return None;
    }

    let aggregator = crate::utils::risk_analyzer::build_context_aggregator(
        project_path,
        enable_context,
        context_providers,
        disable_context,
        Some(&results.complexity.metrics),
    )?;

    let debt_score = crate::debt::total_debt_score(&results.technical_debt.items) as f64;
    Some(
        risk::RiskAnalyzer::default()
            .with_debt_context(debt_score, 100.0)
            .with_context_aggregator(aggregator)
            .with_reference_time(reference_time),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::parallel_call_graph::CallGraphPhase;

    #[test]
    fn test_analyze_file_git_context_returns_none_when_no_context() {
        let risk_analyzer = risk::RiskAnalyzer::default();
        let file_path = PathBuf::from("src/test.rs");
        let project_root = PathBuf::from("/tmp/test");

        let result = analyze_file_git_context(&file_path, &risk_analyzer, &project_root);
        assert!(result.is_none());
    }

    #[test]
    fn analysis_pool_honors_explicit_worker_count() {
        let workers = with_analysis_pool(2, rayon::current_num_threads);

        assert_eq!(workers, 2);
    }

    // Tests for pure helper functions
    mod call_graph_progress_helpers {
        use super::*;

        #[test]
        fn phase_to_subtask_index_maps_phases_correctly() {
            assert_eq!(
                phase_to_subtask_index(CallGraphPhase::DiscoveringFiles),
                None
            );
            assert_eq!(phase_to_subtask_index(CallGraphPhase::ParsingASTs), Some(0));
            assert_eq!(
                phase_to_subtask_index(CallGraphPhase::ExtractingCalls),
                Some(1)
            );
            assert_eq!(
                phase_to_subtask_index(CallGraphPhase::LinkingModules),
                Some(2)
            );
        }

        #[test]
        fn build_progress_info_returns_none_for_zero_total() {
            assert_eq!(build_progress_info(0, 0), None);
            assert_eq!(build_progress_info(5, 0), None);
        }

        #[test]
        fn build_progress_info_returns_tuple_for_nonzero_total() {
            assert_eq!(build_progress_info(0, 10), Some((0, 10)));
            assert_eq!(build_progress_info(5, 10), Some((5, 10)));
            assert_eq!(build_progress_info(10, 10), Some((10, 10)));
        }
    }
}
