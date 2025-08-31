use super::{call_graph, parallel_call_graph};
use crate::{
    cache::CallGraphCache,
    config,
    core::{self, AnalysisResults, DebtItem, FunctionMetrics, Language},
    io,
    priority::{
        self, debt_aggregator::DebtAggregator, debt_aggregator::FunctionId as AggregatorFunctionId,
        scoring::debt_item, unified_scorer::Location, ActionableRecommendation, DebtType,
        FunctionRole, ImpactMetrics, UnifiedAnalysis, UnifiedDebtItem, UnifiedScore,
    },
    risk,
    scoring::{EnhancedScorer, ScoringContext},
};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn perform_unified_analysis(
    results: &AnalysisResults,
    coverage_file: Option<&PathBuf>,
    _semantic_off: bool,
    project_path: &Path,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
) -> Result<UnifiedAnalysis> {
    perform_unified_analysis_with_options(
        results,
        coverage_file,
        _semantic_off,
        project_path,
        verbose_macro_warnings,
        show_macro_stats,
        false, // parallel
        0,     // jobs
        false, // use_cache
    )
}

pub fn perform_unified_analysis_with_options(
    results: &AnalysisResults,
    coverage_file: Option<&PathBuf>,
    _semantic_off: bool,
    project_path: &Path,
    verbose_macro_warnings: bool,
    show_macro_stats: bool,
    parallel: bool,
    jobs: usize,
    use_cache: bool,
) -> Result<UnifiedAnalysis> {
    let mut call_graph = call_graph::build_initial_call_graph(&results.complexity.metrics);
    
    let (framework_exclusions, function_pointer_used_functions) = if parallel {
        // Use parallel call graph construction
        let thread_msg = if jobs == 0 { 
            "all available".to_string() 
        } else { 
            jobs.to_string() 
        };
        log::info!("Using parallel call graph construction with {} threads", thread_msg);
        
        let (mut parallel_graph, exclusions, used_funcs) = 
            parallel_call_graph::build_call_graph_parallel(
                project_path,
                call_graph.clone(),
                if jobs == 0 { None } else { Some(jobs) },
                true, // show_progress
            )?;
        
        // Process Python files (still sequential for now)
        call_graph::process_python_files_for_call_graph(project_path, &mut parallel_graph)?;
        
        call_graph = parallel_graph;
        (exclusions, used_funcs)
    } else if use_cache {
        // Try to use cached call graph
        let config = config::get_config();
        let rust_files = io::walker::find_project_files_with_config(
            project_path,
            vec![Language::Rust],
            config,
        )?;
        
        let mut cache = CallGraphCache::new()?;
        let cache_key = CallGraphCache::generate_key(project_path, &rust_files, config)?;
        
        if let Some((cached_graph, exclusions, used_funcs)) = cache.get(&cache_key) {
            log::info!("Using cached call graph");
            call_graph = cached_graph;
            
            // Process Python files (still need to be done)
            call_graph::process_python_files_for_call_graph(project_path, &mut call_graph)?;
            
            (exclusions.into_iter().collect(), used_funcs.into_iter().collect())
        } else {
            log::info!("No valid cache found, building call graph from scratch");
            
            // Build normally and cache the result
            let (exclusions, used_funcs) = call_graph::process_rust_files_for_call_graph(
                project_path,
                &mut call_graph,
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
            
            call_graph::process_python_files_for_call_graph(project_path, &mut call_graph)?;
            
            (exclusions, used_funcs)
        }
    } else {
        // Use traditional sequential call graph construction
        let (exclusions, used_funcs) = call_graph::process_rust_files_for_call_graph(
            project_path,
            &mut call_graph,
            verbose_macro_warnings,
            show_macro_stats,
        )?;
        
        call_graph::process_python_files_for_call_graph(project_path, &mut call_graph)?;
        
        (exclusions, used_funcs)
    };

    let coverage_data = match coverage_file {
        Some(lcov_path) => {
            Some(risk::lcov::parse_lcov_file(lcov_path).context("Failed to parse LCOV file")?)
        }
        None => None,
    };

    Ok(create_unified_analysis_with_exclusions(
        &results.complexity.metrics,
        &call_graph,
        coverage_data.as_ref(),
        &framework_exclusions,
        Some(&function_pointer_used_functions),
        Some(&results.technical_debt.items),
    ))
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
