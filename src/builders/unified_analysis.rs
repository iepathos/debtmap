use super::{call_graph, parallel_call_graph, parallel_unified_analysis};
use crate::{
    analysis::diagnostics::{DetailLevel, DiagnosticReporter, OutputFormat},
    analysis::multi_pass::{analyze_with_attribution, MultiPassOptions, MultiPassResult},
    analyzers::FileAnalyzer,
    cache::{CacheKey, CallGraphCache, UnifiedAnalysisCache},
    config,
    core::{self, AnalysisResults, DebtItem, FunctionMetrics, Language},
    io,
    priority::{
        self,
        call_graph::{CallGraph, FunctionId},
        debt_aggregator::DebtAggregator,
        debt_aggregator::FunctionId as AggregatorFunctionId,
        scoring::debt_item,
        unified_scorer::Location,
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, UnifiedAnalysis,
        UnifiedDebtItem, UnifiedScore,
    },
    risk,
    scoring::{EnhancedScorer, ScoringContext},
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
    pub use_cache: bool,
    pub multi_pass: bool,
    pub show_attribution: bool,
    pub aggregate_only: bool,
    pub no_aggregation: bool,
    pub aggregation_method: Option<String>,
    pub min_problematic: Option<usize>,
    pub no_god_object: bool,
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
        use_cache: false,
        multi_pass: false,
        show_attribution: false,
        aggregate_only: false,
        no_aggregation: false,
        aggregation_method: None,
        min_problematic: None,
        no_god_object: false,
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
        use_cache,
        multi_pass,
        show_attribution,
        aggregate_only: _aggregate_only,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
    } = options;

    // Check if we should use unified analysis caching
    let files: Vec<PathBuf> = results
        .complexity
        .metrics
        .iter()
        .map(|m| m.file.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let should_cache =
        use_cache && UnifiedAnalysisCache::should_use_cache(files.len(), coverage_file.is_some());

    // Try to get cached unified analysis result
    if should_cache {
        if let Ok(mut unified_cache) = UnifiedAnalysisCache::new(Some(project_path)) {
            // Generate cache key
            if let Ok(cache_key) = UnifiedAnalysisCache::generate_key(
                project_path,
                &files,
                results.complexity.summary.max_complexity, // Use max complexity as threshold proxy
                50,                                        // Default duplication threshold
                coverage_file.as_ref().map(|p| p.as_path()),
                semantic_off,
                parallel,
            ) {
                // Try to get cached result
                if let Some(cached_analysis) = unified_cache.get(&cache_key) {
                    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();
                    if !quiet_mode {
                        eprintln!("🎯 Using cached unified analysis ✓");
                    }
                    return Ok(cached_analysis);
                }

                // Cache miss - proceed with analysis and cache the result
                let analysis_result = perform_unified_analysis_computation(
                    results,
                    coverage_file,
                    semantic_off,
                    project_path,
                    verbose_macro_warnings,
                    show_macro_stats,
                    parallel,
                    jobs,
                    use_cache,
                    multi_pass,
                    show_attribution,
                    no_aggregation,
                    aggregation_method,
                    min_problematic,
                    no_god_object,
                )?;

                // Cache the computed result
                if let Err(e) = unified_cache.put(cache_key, analysis_result.clone(), files) {
                    log::warn!("Failed to cache unified analysis: {}", e);
                }

                return Ok(analysis_result);
            }
        }
    }

    // No caching - proceed with direct computation
    perform_unified_analysis_computation(
        results,
        coverage_file,
        semantic_off,
        project_path,
        verbose_macro_warnings,
        show_macro_stats,
        parallel,
        jobs,
        use_cache,
        multi_pass,
        show_attribution,
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
    )
}

/// Perform the actual unified analysis computation (extracted for caching)
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
    use_cache: bool,
    multi_pass: bool,
    show_attribution: bool,
    no_aggregation: bool,
    aggregation_method: Option<String>,
    min_problematic: Option<usize>,
    no_god_object: bool,
) -> Result<UnifiedAnalysis> {
    let mut call_graph = call_graph::build_initial_call_graph(&results.complexity.metrics);

    // Perform multi-pass analysis if enabled
    if multi_pass {
        perform_multi_pass_analysis(results, show_attribution)?;
    }

    // Select execution strategy based on options
    let execution_strategy = determine_execution_strategy(parallel, use_cache);

    // Progress tracking
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();
    if !quiet_mode {
        eprint!("🔗 Building call graph...");
        std::io::Write::flush(&mut std::io::stderr()).unwrap();
    }

    let (framework_exclusions, function_pointer_used_functions) = match execution_strategy {
        ExecutionStrategy::Parallel => {
            build_parallel_call_graph(project_path, &mut call_graph, jobs)?
        }
        ExecutionStrategy::Cached => build_cached_call_graph(
            project_path,
            &mut call_graph,
            verbose_macro_warnings,
            show_macro_stats,
        )?,
        ExecutionStrategy::Sequential => build_sequential_call_graph(
            project_path,
            &mut call_graph,
            verbose_macro_warnings,
            show_macro_stats,
        )?,
    };

    if !quiet_mode {
        eprintln!(" ✓");
        eprint!("📊 Loading coverage data...");
        std::io::Write::flush(&mut std::io::stderr()).unwrap();
    }

    let coverage_data = load_coverage_data(coverage_file.cloned())?;

    if !quiet_mode {
        eprintln!(" ✓");
        eprint!("🎯 Creating unified analysis... ");
        std::io::Write::flush(&mut std::io::stderr()).unwrap();
    }

    let result = create_unified_analysis_with_exclusions(
        &results.complexity.metrics,
        &call_graph,
        coverage_data.as_ref(),
        &framework_exclusions,
        Some(&function_pointer_used_functions),
        Some(&results.technical_debt.items),
        no_aggregation,
        aggregation_method,
        min_problematic,
        no_god_object,
    );

    // Only print the checkmark if not in parallel mode (parallel mode prints its own progress)
    let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    if !quiet_mode && !parallel_enabled {
        eprintln!(" ✓");
    }

    Ok(result)
}

/// Determines the execution strategy based on configuration options
fn determine_execution_strategy(parallel: bool, use_cache: bool) -> ExecutionStrategy {
    match (parallel, use_cache) {
        (true, _) => ExecutionStrategy::Parallel,
        (false, true) => ExecutionStrategy::Cached,
        (false, false) => ExecutionStrategy::Sequential,
    }
}

/// Execution strategies for call graph construction
enum ExecutionStrategy {
    Parallel,
    Cached,
    Sequential,
}

/// Builds call graph using parallel processing
fn build_parallel_call_graph(
    project_path: &Path,
    call_graph: &mut CallGraph,
    jobs: usize,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    let thread_count = if jobs == 0 { None } else { Some(jobs) };
    log_parallel_execution(jobs);

    let (mut parallel_graph, exclusions, used_funcs) =
        parallel_call_graph::build_call_graph_parallel(
            project_path,
            call_graph.clone(),
            thread_count,
            true, // show_progress
        )?;

    // Process Python files (still sequential for now)
    call_graph::process_python_files_for_call_graph(project_path, &mut parallel_graph)?;
    *call_graph = parallel_graph;

    Ok((exclusions, used_funcs))
}

/// Builds call graph with caching support
fn build_cached_call_graph(
    project_path: &Path,
    call_graph: &mut CallGraph,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    let config = config::get_config();
    let rust_files =
        io::walker::find_project_files_with_config(project_path, vec![Language::Rust], config)?;

    let mut cache = CallGraphCache::new()?;
    let cache_key = CallGraphCache::generate_key(project_path, &rust_files, config)?;

    if let Some((cached_graph, exclusions, used_funcs)) = cache.get(&cache_key) {
        use_cached_graph(
            project_path,
            call_graph,
            cached_graph,
            exclusions,
            used_funcs,
        )
    } else {
        build_and_cache_graph(
            project_path,
            call_graph,
            verbose_macro_warnings,
            show_macro_stats,
            cache,
            cache_key,
            rust_files,
        )
    }
}

/// Uses a cached call graph
fn use_cached_graph(
    project_path: &Path,
    call_graph: &mut CallGraph,
    cached_graph: CallGraph,
    exclusions: Vec<FunctionId>,
    used_funcs: Vec<FunctionId>,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    log::info!("Using cached call graph");
    *call_graph = cached_graph;

    // Process Python files (still need to be done)
    call_graph::process_python_files_for_call_graph(project_path, call_graph)?;

    Ok((
        exclusions.into_iter().collect(),
        used_funcs.into_iter().collect(),
    ))
}

/// Builds and caches a new call graph
fn build_and_cache_graph(
    project_path: &Path,
    call_graph: &mut CallGraph,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    mut cache: CallGraphCache,
    cache_key: CacheKey,
    rust_files: Vec<PathBuf>,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    log::info!("No valid cache found, building call graph from scratch");

    let (exclusions, used_funcs) = call_graph::process_rust_files_for_call_graph(
        project_path,
        call_graph,
        verbose_macro_warnings,
        show_macro_stats,
    )?;

    // Cache the result
    cache.put(
        cache_key,
        call_graph.clone(),
        exclusions.iter().cloned().collect(),
        used_funcs.iter().cloned().collect(),
        rust_files,
    )?;

    call_graph::process_python_files_for_call_graph(project_path, call_graph)?;

    Ok((exclusions, used_funcs))
}

/// Builds call graph using sequential processing
fn build_sequential_call_graph(
    project_path: &Path,
    call_graph: &mut CallGraph,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    let (exclusions, used_funcs) = call_graph::process_rust_files_for_call_graph(
        project_path,
        call_graph,
        verbose_macro_warnings,
        show_macro_stats,
    )?;

    call_graph::process_python_files_for_call_graph(project_path, call_graph)?;

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
    match coverage_file {
        Some(lcov_path) => Ok(Some(
            risk::lcov::parse_lcov_file(&lcov_path).context("Failed to parse LCOV file")?,
        )),
        None => Ok(None),
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
    // Check if parallel mode is enabled
    let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let jobs = std::env::var("DEBTMAP_JOBS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok());

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
            jobs,
        );
    }
    use std::time::Instant;
    let start = Instant::now();
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();

    // Step 1: Initialize unified analysis with data flow graph
    let step_start = Instant::now();
    let mut unified = UnifiedAnalysis::new(call_graph.clone());
    if !quiet_mode {
        eprintln!("  ⏱️  Data flow graph creation: {:?}", step_start.elapsed());
    }

    // Step 2: Populate purity analysis
    let step_start = Instant::now();
    unified.populate_purity_analysis(metrics);
    if !quiet_mode {
        eprintln!(
            "  ⏱️  Purity analysis ({} functions): {:?}",
            metrics.len(),
            step_start.elapsed()
        );
    }

    // Step 3: Find test-only functions
    let step_start = Instant::now();
    let test_only_functions: HashSet<_> =
        call_graph.find_test_only_functions().into_iter().collect();
    if !quiet_mode {
        eprintln!(
            "  ⏱️  Test function detection ({} found): {:?}",
            test_only_functions.len(),
            step_start.elapsed()
        );
    }

    // Step 4: Setup debt aggregator
    let step_start = Instant::now();
    let mut debt_aggregator = DebtAggregator::new();
    if let Some(debt_items) = debt_items {
        let function_mappings: Vec<(AggregatorFunctionId, usize, usize)> = metrics
            .iter()
            .map(|m| {
                let func_id = AggregatorFunctionId {
                    file: m.file.clone(),
                    name: m.name.clone(),
                    start_line: m.line,
                    end_line: m.line + m.length,
                };
                (func_id, m.line, m.line + m.length)
            })
            .collect();

        let debt_items_vec: Vec<DebtItem> = debt_items.to_vec();
        debt_aggregator.aggregate_debt(debt_items_vec, &function_mappings);
    }
    if !quiet_mode {
        eprintln!("  ⏱️  Debt aggregator setup: {:?}", step_start.elapsed());
    }

    // Step 5: Per-function debt analysis (main loop)
    let step_start = Instant::now();
    let mut processed_count = 0;
    let mut skipped_count = 0;
    for metric in metrics {
        if should_skip_metric_for_debt_analysis(metric, call_graph, &test_only_functions) {
            skipped_count += 1;
            continue;
        }
        let item = create_debt_item_from_metric_with_aggregator(
            metric,
            call_graph,
            coverage_data,
            framework_exclusions,
            function_pointer_used_functions,
            &debt_aggregator,
            Some(&unified.data_flow_graph),
        );
        unified.add_item(item);
        processed_count += 1;
    }
    if !quiet_mode {
        eprintln!(
            "  ⏱️  Per-function analysis ({} processed, {} skipped): {:?}",
            processed_count,
            skipped_count,
            step_start.elapsed()
        );
    }

    // Step 6: Error swallowing analysis
    let step_start = Instant::now();
    let mut error_swallow_count = 0;
    if let Some(debt_items) = debt_items {
        let error_swallowing_items = convert_error_swallowing_to_unified(debt_items, call_graph);
        error_swallow_count = error_swallowing_items.len();
        for item in error_swallowing_items {
            unified.add_item(item);
        }
    }
    if !quiet_mode && error_swallow_count > 0 {
        eprintln!(
            "  ⏱️  Error swallowing conversion ({} items): {:?}",
            error_swallow_count,
            step_start.elapsed()
        );
    }

    // Step 7: File-level analysis
    let step_start = Instant::now();
    analyze_files_for_debt(&mut unified, metrics, coverage_data, no_god_object);
    if !quiet_mode {
        eprintln!("  ⏱️  File-level analysis: {:?}", step_start.elapsed());
    }

    // Step 8: File aggregation analysis
    let step_start = Instant::now();
    let mut aggregation_config = crate::config::get_aggregation_config();

    // Override with CLI flags
    if no_aggregation {
        aggregation_config.enabled = false;
    }

    if let Some(method_str) = aggregation_method {
        aggregation_config.method = match method_str.as_str() {
            "sum" => priority::aggregation::AggregationMethod::Sum,
            "weighted_sum" => priority::aggregation::AggregationMethod::WeightedSum,
            "logarithmic_sum" => priority::aggregation::AggregationMethod::LogarithmicSum,
            "max_plus_average" => priority::aggregation::AggregationMethod::MaxPlusAverage,
            _ => aggregation_config.method, // Keep default if invalid
        };
    }

    // Apply min_problematic if specified
    if let Some(min_prob) = min_problematic {
        aggregation_config.min_functions_for_aggregation = min_prob;
    }

    if aggregation_config.enabled {
        // Try to aggregate from UnifiedDebtItems first (for scored items)
        let items_vec: Vec<UnifiedDebtItem> = unified.items.iter().cloned().collect();
        let mut file_aggregates = if !items_vec.is_empty() {
            priority::aggregation::AggregationPipeline::aggregate_from_debt_items(
                &items_vec,
                &aggregation_config,
            )
        } else {
            // If no debt items passed the threshold, aggregate from all metrics
            // This ensures aggregation works even for files with many small issues
            priority::aggregation::AggregationPipeline::aggregate_from_metrics(
                metrics,
                &aggregation_config,
            )
        };

        // If we have items but not all files are represented, also aggregate from metrics
        // to ensure all files with functions are included
        if items_vec.is_empty()
            || file_aggregates.len()
                < metrics
                    .iter()
                    .map(|m| &m.file)
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    / 2
        {
            file_aggregates = priority::aggregation::AggregationPipeline::aggregate_from_metrics(
                metrics,
                &aggregation_config,
            );
        }

        unified.file_aggregates = im::Vector::from(file_aggregates);
    }
    if !quiet_mode {
        eprintln!(
            "  ⏱️  File aggregation (enabled={}): {:?}",
            aggregation_config.enabled,
            step_start.elapsed()
        );
    }

    // Step 9: Final sorting and impact calculation
    let step_start = Instant::now();
    unified.sort_by_priority();
    unified.calculate_total_impact();

    if let Some(lcov) = coverage_data {
        unified.overall_coverage = Some(lcov.get_overall_coverage());
    }
    if !quiet_mode {
        eprintln!(
            "  ⏱️  Final sorting & impact calc: {:?}",
            step_start.elapsed()
        );
        eprintln!("  ⏱️  TOTAL unified analysis time: {:?}", start.elapsed());
    }

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
    no_aggregation: bool,
    aggregation_method: Option<String>,
    min_problematic: Option<usize>,
    no_god_object: bool,
    jobs: Option<usize>,
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

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph.clone(), options);

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

    // Add error swallowing items
    let mut all_items = items;
    if let Some(debt_items) = debt_items {
        let error_swallowing_items = convert_error_swallowing_to_unified(debt_items, call_graph);
        all_items.extend(error_swallowing_items);
    }

    // Phase 3: Parallel file analysis
    let file_items = builder.execute_phase3_parallel(metrics, coverage_data, no_god_object);

    // Build final unified analysis
    let (mut unified, _timings) = builder.build(
        data_flow_graph,
        _purity,
        all_items,
        file_items,
        coverage_data,
    );

    // Handle aggregation
    let mut aggregation_config = crate::config::get_aggregation_config();

    if no_aggregation {
        aggregation_config.enabled = false;
    }

    if let Some(method_str) = aggregation_method {
        aggregation_config.method = match method_str.as_str() {
            "sum" => priority::aggregation::AggregationMethod::Sum,
            "weighted_sum" => priority::aggregation::AggregationMethod::WeightedSum,
            "logarithmic_sum" => priority::aggregation::AggregationMethod::LogarithmicSum,
            "max_plus_average" => priority::aggregation::AggregationMethod::MaxPlusAverage,
            _ => aggregation_config.method,
        };
    }

    if let Some(min_prob) = min_problematic {
        aggregation_config.min_functions_for_aggregation = min_prob;
    }

    if aggregation_config.enabled {
        let items_vec: Vec<UnifiedDebtItem> = unified.items.iter().cloned().collect();
        let file_aggregates = if !items_vec.is_empty() {
            priority::aggregation::AggregationPipeline::aggregate_from_debt_items(
                &items_vec,
                &aggregation_config,
            )
        } else {
            priority::aggregation::AggregationPipeline::aggregate_from_metrics(
                metrics,
                &aggregation_config,
            )
        };

        unified.file_aggregates = im::Vector::from(file_aggregates);
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

    let func_id = priority::call_graph::FunctionId {
        file: metric.file.clone(),
        name: metric.name.clone(),
        line: metric.line,
    };

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

pub(super) fn create_debt_item_from_metric_with_aggregator(
    metric: &FunctionMetrics,
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<priority::call_graph::FunctionId>,
    function_pointer_used_functions: Option<&HashSet<priority::call_graph::FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> UnifiedDebtItem {
    let mut scoring_context = ScoringContext::new(call_graph.clone());

    if let Some(lcov) = coverage_data {
        scoring_context = scoring_context.with_coverage(lcov.clone());
    }

    let test_files: HashSet<PathBuf> = HashSet::new();
    scoring_context = scoring_context.with_test_files(test_files);

    let scorer = EnhancedScorer::new(&scoring_context);
    let score_breakdown = scorer.score_function_with_aggregator(metric, debt_aggregator);

    let mut item = debt_item::create_unified_debt_item_with_aggregator_and_data_flow(
        metric,
        call_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        debt_aggregator,
        data_flow,
    );

    item.unified_score.final_score = score_breakdown.total;
    item
}

fn convert_error_swallowing_to_unified(
    debt_items: &[DebtItem],
    _call_graph: &priority::CallGraph,
) -> Vec<UnifiedDebtItem> {
    debt_items
        .iter()
        .filter(|item| item.debt_type == core::DebtType::ErrorSwallowing)
        .map(|item| {
            let unified_score = UnifiedScore {
                complexity_factor: 3.0,
                coverage_factor: 5.0,
                dependency_factor: 4.0,
                role_multiplier: 1.2,
                final_score: 5.5,
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
                },
                expected_impact: ImpactMetrics {
                    coverage_improvement: 0.0,
                    lines_reduction: 0,
                    complexity_reduction: 0.0,
                    risk_reduction: 3.5,
                },
                transitive_coverage: None,
                upstream_dependencies: 0,
                downstream_dependencies: 0,
                upstream_callers: vec![],
                downstream_callees: vec![],
                nesting_depth: 0,
                function_length: 0,
                cyclomatic_complexity: 0,
                cognitive_complexity: 0,
                entropy_details: None,
                is_pure: None,
                purity_confidence: None,
                god_object_indicators: None,
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
    use crate::priority::file_metrics::{FileDebtItem, FileImpact};
    use std::collections::HashMap;

    // Group functions by file
    let mut files_map: HashMap<PathBuf, Vec<&FunctionMetrics>> = HashMap::new();
    for metric in metrics {
        files_map
            .entry(metric.file.clone())
            .or_default()
            .push(metric);
    }

    let file_analyzer = UnifiedFileAnalyzer::new(coverage_data.cloned());

    // Analyze each file
    for (file_path, functions) in files_map {
        // Convert references to owned values for aggregate_functions
        let functions_owned: Vec<FunctionMetrics> = functions.iter().map(|&f| f.clone()).collect();

        // Get file-level metrics
        let mut file_metrics = file_analyzer.aggregate_functions(&functions_owned);

        // Read file content to get accurate line count and god object analysis
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            // Get accurate line count
            let actual_line_count = content.lines().count();
            file_metrics.total_lines = actual_line_count;

            // Recalculate uncovered lines based on actual line count
            file_metrics.uncovered_lines =
                ((1.0 - file_metrics.coverage_percent) * actual_line_count as f64) as usize;

            // Run god object detection if enabled
            if !no_god_object {
                let god_indicators = file_analyzer
                    .analyze_file(&file_path, &content)
                    .ok()
                    .map(|m| m.god_object_indicators)
                    .unwrap_or_else(|| file_metrics.god_object_indicators.clone());
                file_metrics.god_object_indicators = god_indicators;

                // Update god object detection based on actual line count
                if actual_line_count > 2000 || file_metrics.function_count > 50 {
                    file_metrics.god_object_indicators.is_god_object = true;
                    if file_metrics.god_object_indicators.god_object_score == 0.0 {
                        file_metrics.god_object_indicators.god_object_score =
                            (file_metrics.function_count as f64 / 50.0).min(2.0);
                    }
                }
            }
        } else if !no_god_object {
            // Disable god object detection
            file_metrics.god_object_indicators =
                crate::priority::file_metrics::GodObjectIndicators {
                    methods_count: 0,
                    fields_count: 0,
                    responsibilities: 0,
                    is_god_object: false,
                    god_object_score: 0.0,
                };
        }

        // Calculate function scores for this file
        let mut function_scores = Vec::new();
        for func in &functions_owned {
            // Get the score from unified items if it exists
            let score = unified
                .items
                .iter()
                .find(|item| item.location.file == func.file && item.location.function == func.name)
                .map(|item| item.unified_score.final_score)
                .unwrap_or(0.0);
            function_scores.push(score);
        }
        file_metrics.function_scores = function_scores;

        // Calculate file score
        let score = file_metrics.calculate_score();

        // Update god object indicators for functions in this file
        if file_metrics.god_object_indicators.is_god_object {
            // Convert GodObjectIndicators to GodObjectAnalysis for UnifiedDebtItem
            let god_analysis = crate::organization::GodObjectAnalysis {
                is_god_object: file_metrics.god_object_indicators.is_god_object,
                method_count: file_metrics.god_object_indicators.methods_count,
                field_count: file_metrics.god_object_indicators.fields_count,
                responsibility_count: file_metrics.god_object_indicators.responsibilities,
                lines_of_code: file_metrics.total_lines,
                complexity_sum: file_metrics.total_complexity,
                god_object_score: file_metrics.god_object_indicators.god_object_score * 100.0, // Convert to percentage
                recommended_splits: Vec::new(),
                confidence: crate::organization::GodObjectConfidence::Definite,
                responsibilities: Vec::new(),
            };

            for item in unified.items.iter_mut() {
                if item.location.file == file_path {
                    item.god_object_indicators = Some(god_analysis.clone());
                }
            }
        }

        // Only add file items with significant scores
        if score > 50.0 {
            // Threshold for file-level items
            let recommendation = file_metrics.generate_recommendation();

            let file_item = FileDebtItem {
                metrics: file_metrics.clone(),
                score,
                priority_rank: 0, // Will be set during sorting
                recommendation,
                impact: FileImpact {
                    complexity_reduction: file_metrics.avg_complexity
                        * file_metrics.function_count as f64
                        * 0.2,
                    maintainability_improvement: score / 10.0,
                    test_effort: file_metrics.uncovered_lines as f64 * 0.1,
                },
            };

            unified.add_file_item(file_item);
        }
    }
}
