//! Effects-based orchestration for unified analysis.
//!
//! This module provides the orchestration layer that composes pure functions
//! with effects for I/O and progress reporting. It uses the effects system
//! from spec 262 for clean separation of concerns.

use super::phases::{call_graph, file_analysis, god_object, scoring};
use crate::analysis::call_graph::{
    CrossModuleTracker, FrameworkPatternDetector, FunctionPointerTracker, RustCallGraph,
    TraitRegistry,
};
use crate::analysis::purity_analysis::PurityAnalyzer;
use crate::analysis::purity_propagation::{PurityCallGraphAdapter, PurityPropagator};
use crate::core::{AnalysisResults, FunctionMetrics};
use crate::data_flow::DataFlowGraph;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::{UnifiedAnalysis, UnifiedAnalysisUtils};
use crate::risk::lcov::LcovData;
use crate::risk::RiskAnalyzer;
use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

/// Timing information for analysis phases.
#[derive(Debug, Clone, Default)]
pub struct AnalysisTimings {
    pub call_graph_building: Duration,
    pub coverage_loading: Duration,
    pub purity_propagation: Duration,
    pub debt_scoring: Duration,
    pub file_analysis: Duration,
    pub total: Duration,
}

/// Context for analysis orchestration.
pub struct AnalysisContext<'a> {
    pub results: &'a AnalysisResults,
    pub project_path: &'a Path,
    pub coverage_data: Option<&'a LcovData>,
    pub risk_analyzer: Option<&'a RiskAnalyzer>,
    pub no_god_object: bool,
    pub no_aggregation: bool,
    pub aggregation_method: Option<String>,
    pub min_problematic: Option<usize>,
}

/// Run purity propagation on function metrics (pure transformation).
pub fn run_purity_propagation(
    metrics: &[FunctionMetrics],
    call_graph: &CallGraph,
) -> Vec<FunctionMetrics> {
    // Create RustCallGraph wrapper
    let rust_graph = RustCallGraph {
        base_graph: call_graph.clone(),
        trait_registry: TraitRegistry::new(),
        function_pointer_tracker: FunctionPointerTracker::new(),
        framework_patterns: FrameworkPatternDetector::new(),
        cross_module_tracker: CrossModuleTracker::new(),
    };

    // Create call graph adapter
    let adapter = PurityCallGraphAdapter::from_rust_graph(rust_graph);

    // Create purity analyzer and propagator
    let purity_analyzer = PurityAnalyzer::new();
    let mut propagator = PurityPropagator::new(adapter, purity_analyzer);

    // Run propagation - failures are expected when external dependencies are called
    if let Err(e) = propagator.propagate(metrics) {
        log::debug!("Purity propagation skipped (external deps): {}", e);
        return metrics.to_vec();
    }

    // Apply results to metrics
    metrics
        .iter()
        .map(|metric| {
            let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

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
        .collect()
}

/// Create unified analysis from analysis results (orchestrates pure functions).
///
/// This is the main orchestration function that composes all analysis phases.
/// It handles progress reporting at the boundaries while keeping core logic pure.
#[allow(clippy::too_many_arguments)]
pub fn create_unified_analysis(
    ctx: &AnalysisContext,
    call_graph: &CallGraph,
    framework_exclusions: &HashSet<FunctionId>,
    function_pointer_used_functions: Option<&HashSet<FunctionId>>,
    enriched_metrics: &[FunctionMetrics],
    timings: &mut AnalysisTimings,
) -> UnifiedAnalysis {
    let start = Instant::now();

    // Initialize unified analysis
    let mut unified = UnifiedAnalysis::new(call_graph.clone());

    // Populate purity analysis
    unified.populate_purity_analysis(enriched_metrics);

    // Find test-only functions (pure)
    let test_only_functions = call_graph::find_test_only_functions(call_graph);

    // Setup debt aggregator (pure)
    let debt_aggregator =
        scoring::setup_debt_aggregator(enriched_metrics, Some(&ctx.results.technical_debt.items));

    // Create data flow graph
    let data_flow_graph = DataFlowGraph::from_call_graph(call_graph.clone());

    // Build file line count cache (spec 195: I/O at boundary, once per unique file)
    let file_line_counts = scoring::build_file_line_count_cache(enriched_metrics);

    // Score functions (pure, main computation - uses cached file line counts)
    let debt_items = scoring::process_metrics_to_debt_items(
        enriched_metrics,
        call_graph,
        &test_only_functions,
        ctx.coverage_data,
        framework_exclusions,
        function_pointer_used_functions,
        &debt_aggregator,
        Some(&data_flow_graph),
        ctx.risk_analyzer,
        ctx.project_path,
        &file_line_counts,
    );

    // Add debt items
    for item in debt_items {
        unified.add_item(item);
    }

    timings.debt_scoring = start.elapsed();

    // File analysis (pure)
    let file_analysis_start = Instant::now();
    process_file_analysis(
        &mut unified,
        enriched_metrics,
        ctx.coverage_data,
        ctx.no_god_object,
        ctx.risk_analyzer,
        ctx.project_path,
        call_graph,
    );
    timings.file_analysis = file_analysis_start.elapsed();

    // Final sorting and impact calculation
    unified.sort_by_priority();
    unified.calculate_total_impact();

    // Set coverage data availability
    unified.has_coverage_data = ctx.coverage_data.is_some();
    if let Some(lcov) = ctx.coverage_data {
        unified.overall_coverage = Some(lcov.get_overall_coverage());
    }

    timings.total = start.elapsed();

    unified
}

/// Process file-level analysis (orchestrates pure functions).
fn process_file_analysis(
    unified: &mut UnifiedAnalysis,
    metrics: &[FunctionMetrics],
    coverage_data: Option<&LcovData>,
    no_god_object: bool,
    risk_analyzer: Option<&RiskAnalyzer>,
    project_path: &Path,
    call_graph: &CallGraph,
) {
    use crate::metrics::loc_counter::LocCounter;

    // Group functions by file (pure)
    let file_groups = file_analysis::group_functions_by_file(metrics);

    // Register analyzed files for LOC calculation
    let loc_counter = LocCounter::default();
    for file_path in file_groups.keys() {
        if let Ok(loc_count) = loc_counter.count_file(file_path) {
            unified.register_analyzed_file(file_path.clone(), loc_count.physical_lines);
        }
    }

    // Process each file
    for (file_path, functions) in file_groups {
        // Read file content (I/O at boundary)
        let file_content = std::fs::read_to_string(&file_path).ok();

        // Process file metrics (pure)
        let processed = file_analysis::process_file_metrics(
            file_path.clone(),
            functions,
            file_content.as_deref(),
            coverage_data,
            no_god_object,
            project_path,
        );

        // Check score threshold
        let score = processed.file_metrics.calculate_score();
        let has_god_object = processed
            .god_analysis
            .as_ref()
            .is_some_and(|a| a.is_god_object);

        if score > 50.0 || has_god_object {
            // Process god object if present AND it's actually a god object
            // Spec 206: The cohesion gate may have filtered out god objects, so
            // we must check is_god_object even when god_analysis exists
            if let Some(god_analysis) = &processed.god_analysis {
                if god_analysis.is_god_object {
                    // Aggregate metrics from raw functions (pure)
                    use crate::priority::god_object_aggregation::{
                        aggregate_coverage_from_raw_metrics, aggregate_from_raw_metrics,
                    };

                    let mut aggregated_metrics =
                        aggregate_from_raw_metrics(&processed.raw_functions);

                    // Aggregate coverage
                    if let Some(lcov) = coverage_data {
                        aggregated_metrics.weighted_coverage =
                            aggregate_coverage_from_raw_metrics(&processed.raw_functions, lcov);
                    }

                    // Analyze file git context
                    if let Some(analyzer) = risk_analyzer {
                        aggregated_metrics.aggregated_contextual_risk =
                            god_object::analyze_file_git_context(
                                &processed.file_path,
                                analyzer,
                                &processed.project_root,
                            );
                    }

                    // Enrich god analysis with aggregates (pure)
                    let enriched_god_analysis = god_object::enrich_god_analysis_with_aggregates(
                        god_analysis,
                        &aggregated_metrics,
                    );

                    // Update function god indicators
                    for item in unified.items.iter_mut() {
                        if item.location.file == processed.file_path {
                            item.god_object_indicators = Some(enriched_god_analysis.clone());
                        }
                    }

                    // Create god object debt item (pure)
                    let mut god_item = god_object::create_god_object_debt_item(
                        &processed.file_path,
                        &processed.file_metrics,
                        &enriched_god_analysis,
                        aggregated_metrics,
                        coverage_data,
                    );

                    // Generate context suggestion for AI agents (spec 263)
                    use crate::priority::context::{generate_context_suggestion, ContextConfig};
                    let context_config = ContextConfig::default();
                    god_item.context_suggestion =
                        generate_context_suggestion(&god_item, call_graph, &context_config);

                    unified.add_item(god_item);
                }
            }

            // Create file debt item (pure)
            let file_item = file_analysis::create_file_debt_item(
                processed.file_metrics,
                Some(&processed.file_context),
            );
            unified.add_file_item(file_item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_timings_default() {
        let timings = AnalysisTimings::default();
        assert_eq!(timings.total, Duration::from_secs(0));
    }
}
