use super::{call_graph, parallel_call_graph};
use crate::{
    analysis::diagnostics::{DetailLevel, DiagnosticReporter, OutputFormat},
    analysis::multi_pass::{analyze_with_attribution, MultiPassOptions, MultiPassResult},
    analyzers::FileAnalyzer,
    cache::{CacheKey, CallGraphCache},
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
    })
}

pub fn perform_unified_analysis_with_options(
    options: UnifiedAnalysisOptions,
) -> Result<UnifiedAnalysis> {
    let UnifiedAnalysisOptions {
        results,
        coverage_file,
        semantic_off: _semantic_off,
        project_path,
        verbose_macro_warnings,
        show_macro_stats,
        parallel,
        jobs,
        use_cache,
        multi_pass,
        show_attribution,
    } = options;
    let mut call_graph = call_graph::build_initial_call_graph(&results.complexity.metrics);

    // Perform multi-pass analysis if enabled
    if multi_pass {
        perform_multi_pass_analysis(results, show_attribution)?;
    }

    // Select execution strategy based on options
    let execution_strategy = determine_execution_strategy(parallel, use_cache);

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

    let coverage_data = load_coverage_data(coverage_file.cloned())?;

    Ok(create_unified_analysis_with_exclusions(
        &results.complexity.metrics,
        &call_graph,
        coverage_data.as_ref(),
        &framework_exclusions,
        Some(&function_pointer_used_functions),
        Some(&results.technical_debt.items),
    ))
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

pub fn create_unified_analysis_with_exclusions(
    metrics: &[FunctionMetrics],
    call_graph: &priority::CallGraph,
    coverage_data: Option<&risk::lcov::LcovData>,
    framework_exclusions: &HashSet<priority::call_graph::FunctionId>,
    function_pointer_used_functions: Option<&HashSet<priority::call_graph::FunctionId>>,
    debt_items: Option<&[DebtItem]>,
) -> UnifiedAnalysis {
    let mut unified = UnifiedAnalysis::new(call_graph.clone());

    unified.populate_purity_analysis(metrics);

    let test_only_functions: HashSet<_> =
        call_graph.find_test_only_functions().into_iter().collect();

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

    for metric in metrics {
        if should_skip_metric_for_debt_analysis(metric, call_graph, &test_only_functions) {
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
    }

    if let Some(debt_items) = debt_items {
        let error_swallowing_items = convert_error_swallowing_to_unified(debt_items, call_graph);
        for item in error_swallowing_items {
            unified.add_item(item);
        }
    }

    // Add file-level analysis
    analyze_files_for_debt(&mut unified, metrics, coverage_data);

    // Add file aggregation analysis
    let aggregation_config = crate::config::get_aggregation_config();
    if aggregation_config.enabled {
        let items_vec: Vec<UnifiedDebtItem> = unified.items.iter().cloned().collect();
        let file_aggregates = priority::aggregation::AggregationPipeline::aggregate_from_debt_items(
            &items_vec,
            &aggregation_config,
        );
        unified.file_aggregates = im::Vector::from(file_aggregates);
    }

    unified.sort_by_priority();
    unified.calculate_total_impact();

    if let Some(lcov) = coverage_data {
        unified.overall_coverage = Some(lcov.get_overall_coverage());
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

fn create_debt_item_from_metric_with_aggregator(
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
    for (_file_path, functions) in files_map {
        // Convert references to owned values for aggregate_functions
        let functions_owned: Vec<FunctionMetrics> = functions.iter().map(|&f| f.clone()).collect();

        // Get file-level metrics
        let mut file_metrics = file_analyzer.aggregate_functions(&functions_owned);

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
