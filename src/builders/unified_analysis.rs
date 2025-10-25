use super::{call_graph, parallel_call_graph, parallel_unified_analysis};
use crate::{
    analysis::diagnostics::{DetailLevel, DiagnosticReporter, OutputFormat},
    analysis::multi_pass::{analyze_with_attribution, MultiPassOptions, MultiPassResult},
    analyzers::{call_graph_integration, FileAnalyzer},
    cache::{CacheKey, CallGraphCache, UnifiedAnalysisCache, UnifiedAnalysisCacheKey},
    config,
    core::{self, AnalysisResults, DebtItem, FunctionMetrics, Language},
    io,
    priority::{
        self,
        call_graph::{CallGraph, FunctionId},
        debt_aggregator::DebtAggregator,
        debt_aggregator::FunctionId as AggregatorFunctionId,
        file_metrics::{FileDebtItem, FileDebtMetrics, FileImpact},
        scoring::debt_item,
        unified_scorer::Location,
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, UnifiedAnalysis,
        UnifiedDebtItem, UnifiedScore,
    },
    risk,
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
    pub formatting_config: crate::formatting::FormattingConfig,
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
        formatting_config: crate::formatting::FormattingConfig::from_env(),
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
        formatting_config,
    } = options;

    // Extract files and create computation parameters in pure functional style
    let analysis_params = create_analysis_parameters(
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
        formatting_config,
    );

    // Apply caching strategy using function composition
    apply_caching_strategy(&analysis_params)
        .unwrap_or_else(|| perform_direct_computation(analysis_params))
}

// Pure function to create analysis parameters
#[allow(clippy::too_many_arguments)]
fn create_analysis_parameters<'a>(
    results: &'a AnalysisResults,
    coverage_file: Option<&'a PathBuf>,
    semantic_off: bool,
    project_path: &'a Path,
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
    formatting_config: crate::formatting::FormattingConfig,
) -> AnalysisParameters<'a> {
    let files = extract_unique_files(&results.complexity.metrics);
    let cache_config = CacheConfiguration::new(use_cache, files.len(), coverage_file.is_some());

    AnalysisParameters {
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
        files,
        cache_config,
        formatting_config,
    }
}

// Pure function to extract unique files from metrics
fn extract_unique_files(metrics: &[FunctionMetrics]) -> Vec<PathBuf> {
    metrics
        .iter()
        .map(|m| m.file.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

// Data structure to hold analysis parameters
struct AnalysisParameters<'a> {
    results: &'a AnalysisResults,
    coverage_file: Option<&'a PathBuf>,
    semantic_off: bool,
    project_path: &'a Path,
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
    files: Vec<PathBuf>,
    cache_config: CacheConfiguration,
    formatting_config: crate::formatting::FormattingConfig,
}

// Pure function for cache configuration
struct CacheConfiguration {
    should_cache: bool,
}

impl CacheConfiguration {
    fn new(use_cache: bool, file_count: usize, has_coverage: bool) -> Self {
        Self {
            should_cache: use_cache
                && UnifiedAnalysisCache::should_use_cache(file_count, has_coverage),
        }
    }
}

// Functional approach to caching strategy
fn apply_caching_strategy(params: &AnalysisParameters) -> Option<Result<UnifiedAnalysis>> {
    if !params.cache_config.should_cache {
        return None;
    }

    attempt_cached_analysis(params).or_else(|| None)
}

// Pure function for cache attempt
fn attempt_cached_analysis(params: &AnalysisParameters) -> Option<Result<UnifiedAnalysis>> {
    let mut unified_cache = UnifiedAnalysisCache::new(Some(params.project_path)).ok()?;
    let cache_key = create_cache_key(params).ok()?;

    unified_cache.get(&cache_key).map(|cached_analysis| {
        log_cache_hit(params.formatting_config);
        Ok(cached_analysis)
    })
}

// Function composition for computation with caching
#[allow(dead_code)]
fn attempt_computation_with_caching(params: AnalysisParameters) -> Option<Result<UnifiedAnalysis>> {
    let mut unified_cache = UnifiedAnalysisCache::new(Some(params.project_path)).ok()?;
    let cache_key = create_cache_key(&params).ok()?;

    Some(
        perform_computation(&params)
            .and_then(|result| cache_result(&mut unified_cache, cache_key, result, params.files)),
    )
}

// Pure function to create cache key
fn create_cache_key(params: &AnalysisParameters) -> Result<UnifiedAnalysisCacheKey> {
    UnifiedAnalysisCache::generate_key(
        params.project_path,
        &params.files,
        params.results.complexity.summary.max_complexity,
        50, // Default duplication threshold
        params.coverage_file.map(|p| p.as_path()),
        params.semantic_off,
        params.parallel,
    )
}

// I/O function for logging cache hit
fn log_cache_hit(formatting_config: crate::formatting::FormattingConfig) {
    let quiet_mode = std::env::var("DEBTMAP_QUIET").is_ok();
    let use_emoji =
        formatting_config.emoji.should_use_emoji() && std::env::var("DEBTMAP_NO_EMOJI").is_err();
    if !quiet_mode {
        if use_emoji {
            eprintln!("🎯 Using cached unified analysis ✓");
        } else {
            eprintln!("Using cached unified analysis [OK]");
        }
    }
}

// Function to cache computation result
#[allow(dead_code)]
fn cache_result(
    cache: &mut UnifiedAnalysisCache,
    cache_key: UnifiedAnalysisCacheKey,
    result: UnifiedAnalysis,
    files: Vec<PathBuf>,
) -> Result<UnifiedAnalysis> {
    if let Err(e) = cache.put(cache_key, result.clone(), files) {
        log::warn!("Failed to cache unified analysis: {}", e);
    }
    Ok(result)
}

// Direct computation without caching
fn perform_direct_computation(params: AnalysisParameters) -> Result<UnifiedAnalysis> {
    perform_computation(&params)
}

// Core computation function (extracted for reuse)
fn perform_computation(params: &AnalysisParameters) -> Result<UnifiedAnalysis> {
    perform_unified_analysis_computation(
        params.results,
        params.coverage_file,
        params.semantic_off,
        params.project_path,
        params.verbose_macro_warnings,
        params.show_macro_stats,
        params.parallel,
        params.jobs,
        params.cache_config.should_cache,
        params.multi_pass,
        params.show_attribution,
        params.no_aggregation,
        params.aggregation_method.clone(),
        params.min_problematic,
        params.no_god_object,
        params.formatting_config,
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
    formatting_config: crate::formatting::FormattingConfig,
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
    let use_emoji =
        formatting_config.emoji.should_use_emoji() && std::env::var("DEBTMAP_NO_EMOJI").is_err();
    // Progress will be shown by the parallel builder itself

    // Time call graph building
    let call_graph_start = std::time::Instant::now();
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
    let call_graph_time = call_graph_start.elapsed();

    if !quiet_mode {
        if use_emoji {
            eprintln!(" ✓");
            eprint!("🔍 Resolving trait method calls...");
        } else {
            eprintln!(" [OK]");
            eprint!("Resolving trait method calls...");
        }
        std::io::Write::flush(&mut std::io::stderr()).unwrap();
    }

    // Integrate trait resolution to reduce false positives
    let trait_resolution_start = std::time::Instant::now();
    let trait_resolution_stats =
        integrate_trait_resolution(project_path, &mut call_graph, verbose_macro_warnings)?;
    let trait_resolution_time = trait_resolution_start.elapsed();

    if !quiet_mode {
        if use_emoji {
            eprintln!(" ✓");
        } else {
            eprintln!(" [OK]");
        }

        // Display trait resolution statistics in verbose mode
        if verbose_macro_warnings {
            if use_emoji {
                eprintln!(
                    "🔗 Resolved {} trait method calls",
                    trait_resolution_stats.resolved_calls
                );
                eprintln!(
                    "🎯 Marked {} trait implementations as callable",
                    trait_resolution_stats.marked_implementations
                );
            } else {
                eprintln!(
                    "Resolved {} trait method calls",
                    trait_resolution_stats.resolved_calls
                );
                eprintln!(
                    "Marked {} trait implementations as callable",
                    trait_resolution_stats.marked_implementations
                );
            }
        }

        if use_emoji {
            eprint!("📊 Loading coverage data...");
        } else {
            eprint!("Loading coverage data...");
        }
        std::io::Write::flush(&mut std::io::stderr()).unwrap();
    }

    let coverage_loading_start = std::time::Instant::now();
    let coverage_data = load_coverage_data(coverage_file.cloned())?;
    let coverage_loading_time = coverage_loading_start.elapsed();

    // Emit warning if no coverage data provided (spec 108)
    if coverage_data.is_none() && !quiet_mode {
        use colored::*;
        eprintln!();
        eprintln!(
            "{} Coverage data not provided. Analysis will focus on complexity and code smells.",
            "💡 TIP:".bright_yellow()
        );
        eprintln!(
            "   For test gap detection, provide coverage with: {}",
            "--lcov-file coverage.info".bright_cyan()
        );
        eprintln!();
    }

    if !quiet_mode {
        if use_emoji {
            eprintln!(" ✓");
            eprint!("🎯 Creating unified analysis... ");
        } else {
            eprintln!(" [OK]");
            eprint!("Creating unified analysis... ");
        }
        std::io::Write::flush(&mut std::io::stderr()).unwrap();
    }

    // Populate call graph data into function metrics for better analysis
    let enriched_metrics = call_graph_integration::populate_call_graph_data(
        results.complexity.metrics.clone(),
        &call_graph,
    );

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
        trait_resolution_time,
        coverage_loading_time,
    );

    // Only print the checkmark if not in parallel mode (parallel mode prints its own progress)
    let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    if !quiet_mode && !parallel_enabled {
        if use_emoji {
            eprintln!(" ✓");
        } else {
            eprintln!(" [OK]");
        }
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
        Duration::from_secs(0),
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
    trait_resolution_time: std::time::Duration,
    coverage_loading_time: std::time::Duration,
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
            call_graph_time,
            trait_resolution_time,
            coverage_loading_time,
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
                let func_id = AggregatorFunctionId::new(m.file.clone(), m.name.clone(), m.line);
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

    // Step 8: File aggregation has been removed - skip to step 9

    // Step 9: Final sorting and impact calculation
    let step_start = Instant::now();
    unified.sort_by_priority();
    unified.calculate_total_impact();

    // Set coverage data availability flag (spec 108)
    unified.has_coverage_data = coverage_data.is_some();

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
    _no_aggregation: bool,
    _aggregation_method: Option<String>,
    _min_problematic: Option<usize>,
    no_god_object: bool,
    jobs: Option<usize>,
    call_graph_time: std::time::Duration,
    trait_resolution_time: std::time::Duration,
    coverage_loading_time: std::time::Duration,
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

    // Set preliminary timing values from call graph building, trait resolution, and coverage loading
    builder.set_preliminary_timings(
        call_graph_time,
        trait_resolution_time,
        coverage_loading_time,
    );

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
    let (unified, _timings) = builder.build(
        data_flow_graph,
        _purity,
        all_items,
        file_items,
        coverage_data,
    );

    // Aggregation has been removed - no longer needed

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

pub(super) fn create_debt_item_from_metric_with_aggregator(
    metric: &FunctionMetrics,
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<priority::call_graph::FunctionId>,
    function_pointer_used_functions: Option<&HashSet<priority::call_graph::FunctionId>>,
    debt_aggregator: &DebtAggregator,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> UnifiedDebtItem {
    // Use the unified debt item creation which already calculates the score correctly
    debt_item::create_unified_debt_item_with_aggregator_and_data_flow(
        metric,
        call_graph,
        coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        debt_aggregator,
        data_flow,
    )
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
                pre_adjustment_score: None,
                adjustment_applied: None,
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
                tier: None,
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
    let file_analyzer = UnifiedFileAnalyzer::new(coverage_data.cloned());

    // Process each file using functional composition
    let processed_files: Vec<ProcessedFileData> = file_groups
        .into_iter()
        .map(|(file_path, functions)| {
            process_single_file(file_path, functions, &file_analyzer, no_god_object, unified)
        })
        .filter_map(|result| result.ok())
        .filter(|data| data.score > 50.0) // Filter significant files
        .collect();

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
    score: f64,
    recommendation: String,
    god_analysis: Option<crate::organization::GodObjectAnalysis>,
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

    // Calculate overall file score
    let score = final_metrics.calculate_score();

    // Generate recommendation and god object analysis
    let recommendation = final_metrics.generate_recommendation();
    let god_analysis = create_god_object_analysis(&final_metrics);

    Ok(ProcessedFileData {
        file_path,
        file_metrics: final_metrics,
        score,
        recommendation,
        god_analysis,
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
    })
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
    FileDebtItem {
        metrics: file_data.file_metrics.clone(),
        score: file_data.score,
        priority_rank: 0, // Will be set during sorting
        recommendation: file_data.recommendation,
        impact: FileImpact {
            complexity_reduction: file_data.file_metrics.avg_complexity
                * file_data.file_metrics.function_count as f64
                * 0.2,
            maintainability_improvement: file_data.score / 10.0,
            test_effort: file_data.file_metrics.uncovered_lines as f64 * 0.1,
        },
    }
}

/// Statistics about trait resolution
#[derive(Debug, Clone, Default)]
struct TraitResolutionStats {
    resolved_calls: usize,
    marked_implementations: usize,
}

/// Integrate trait resolution into the call graph to reduce false positives
fn integrate_trait_resolution(
    _project_path: &Path,
    call_graph: &mut crate::priority::call_graph::CallGraph,
    _verbose: bool,
) -> Result<TraitResolutionStats> {
    use crate::analysis::call_graph::TraitRegistry;

    // Build trait registry from the project
    let trait_registry = TraitRegistry::new();

    // Detect common trait patterns (Default, Clone, etc.) and mark them as entry points
    trait_registry.detect_common_trait_patterns(call_graph);

    // Resolve trait method calls to concrete implementations
    let resolved_count = trait_registry.resolve_trait_method_calls(call_graph);

    // Get statistics about trait usage
    let trait_stats = trait_registry.get_statistics();

    Ok(TraitResolutionStats {
        resolved_calls: resolved_count,
        marked_implementations: trait_stats.total_implementations,
    })
}
