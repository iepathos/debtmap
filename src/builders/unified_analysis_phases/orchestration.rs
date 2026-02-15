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
use crate::core::{AnalysisResults, FunctionMetrics, Language};
use crate::data_flow::DataFlowGraph;
use crate::debt::suppression::{parse_suppression_comments, SuppressionContext};
use crate::organization::GodObjectAnalysis;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::DebtType;
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

/// Check if a file should be processed based on score and god object status.
fn should_process_file(score: f64, has_god_object: bool) -> bool {
    score > 50.0 || has_god_object
}

/// Check if a god object should be suppressed based on file annotations.
///
/// Supports both file-level (GodFile/GodModule) and struct-level (GodClass) suppressions:
/// - For GodFile/GodModule: checks for `debtmap:ignore[god_object]` at lines 1-5
/// - For GodClass: checks near the struct definition AND at file level
///
/// A file-level `debtmap:ignore[god_object]` annotation at line 1-5 applies to ALL
/// god objects in the file, including struct-level GodClass detections.
///
/// Returns true if the god object should be excluded from analysis output.
fn is_god_object_suppressed(
    god_analysis: &GodObjectAnalysis,
    suppression_context: &SuppressionContext,
) -> bool {
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
    // This allows placing the annotation immediately before the struct
    if let crate::organization::DetectionType::GodClass = god_analysis.detection_type {
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

/// Context for god object processing.
struct GodObjectProcessingContext<'a> {
    coverage_data: Option<&'a LcovData>,
    risk_analyzer: Option<&'a RiskAnalyzer>,
    call_graph: &'a CallGraph,
}

/// Result of god object processing.
struct GodObjectProcessingResult {
    enriched_analysis: crate::organization::GodObjectAnalysis,
    debt_item: crate::priority::UnifiedDebtItem,
}

/// Process god object analysis and create enriched data (pure except for git context).
fn process_god_object(
    processed: &file_analysis::ProcessedFileData,
    god_analysis: &crate::organization::GodObjectAnalysis,
    ctx: &GodObjectProcessingContext<'_>,
) -> GodObjectProcessingResult {
    use crate::priority::context::{generate_context_suggestion, ContextConfig};
    use crate::priority::god_object_aggregation::{
        aggregate_coverage_from_raw_metrics, aggregate_from_raw_metrics,
    };

    // Aggregate metrics from raw functions (pure)
    let mut aggregated_metrics = aggregate_from_raw_metrics(&processed.raw_functions);

    // Aggregate coverage
    if let Some(lcov) = ctx.coverage_data {
        aggregated_metrics.weighted_coverage =
            aggregate_coverage_from_raw_metrics(&processed.raw_functions, lcov);
    }

    // Analyze file git context
    if let Some(analyzer) = ctx.risk_analyzer {
        aggregated_metrics.aggregated_contextual_risk = god_object::analyze_file_git_context(
            &processed.file_path,
            analyzer,
            &processed.project_root,
        );
    }

    // Enrich god analysis with aggregates (pure)
    let enriched_analysis =
        god_object::enrich_god_analysis_with_aggregates(god_analysis, &aggregated_metrics);

    // Create god object debt item (pure)
    let mut debt_item = god_object::create_god_object_debt_item(
        &processed.file_path,
        &processed.file_metrics,
        &enriched_analysis,
        aggregated_metrics,
        ctx.coverage_data,
        Some(ctx.call_graph),
    );

    // Generate context suggestion for AI agents (spec 263)
    let context_config = ContextConfig::default();
    debt_item.context_suggestion =
        generate_context_suggestion(&debt_item, ctx.call_graph, &context_config);

    GodObjectProcessingResult {
        enriched_analysis,
        debt_item,
    }
}

/// Update god object indicators for items in the same file.
fn update_god_indicators_for_file(
    items: &mut im::Vector<crate::priority::UnifiedDebtItem>,
    file_path: &std::path::PathBuf,
    enriched_analysis: &crate::organization::GodObjectAnalysis,
) {
    for item in items.iter_mut() {
        if item.location.file == *file_path {
            item.god_object_indicators = Some(enriched_analysis.clone());
        }
    }
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
    let file_groups = file_analysis::group_functions_by_file(metrics);
    register_file_loc_counts(unified, &file_groups);

    let god_ctx = GodObjectProcessingContext {
        coverage_data,
        risk_analyzer,
        call_graph,
    };

    for (file_path, functions) in file_groups {
        let file_content = std::fs::read_to_string(&file_path).ok();

        // Parse suppression context for this file
        let suppression_context = file_content.as_ref().map(|content| {
            let language = file_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| match ext {
                    "rs" => Language::Rust,
                    "py" | "pyw" => Language::Python,
                    _ => Language::Rust,
                })
                .unwrap_or(Language::Rust);
            parse_suppression_comments(content, language, &file_path)
        });

        let processed = file_analysis::process_file_metrics(
            file_path.clone(),
            functions,
            file_content.as_deref(),
            coverage_data,
            no_god_object,
            project_path,
        );

        let score = processed.file_metrics.calculate_score();
        let has_god_object = processed
            .god_analysis
            .as_ref()
            .is_some_and(|a| a.is_god_object);

        if !should_process_file(score, has_god_object) {
            continue;
        }

        // Process god object if present (spec 206: check is_god_object even when god_analysis exists)
        let god_object_suppressed = if let Some(god_analysis) =
            processed.god_analysis.as_ref().filter(|a| a.is_god_object)
        {
            // Check if this god object should be suppressed via debtmap:ignore[god_object] annotation
            let is_suppressed = suppression_context
                .as_ref()
                .is_some_and(|ctx| is_god_object_suppressed(god_analysis, ctx));

            if !is_suppressed {
                let result = process_god_object(&processed, god_analysis, &god_ctx);
                update_god_indicators_for_file(
                    &mut unified.items,
                    &processed.file_path,
                    &result.enriched_analysis,
                );
                unified.add_item(result.debt_item);
            }
            is_suppressed
        } else {
            false
        };

        // Create file item, clearing god_object_analysis if suppressed
        let mut file_metrics = processed.file_metrics;
        if god_object_suppressed {
            file_metrics.god_object_analysis = None;
        }
        let file_item =
            file_analysis::create_file_debt_item(file_metrics, Some(&processed.file_context));
        unified.add_file_item(file_item);
    }
}

/// Register LOC counts for analyzed files.
fn register_file_loc_counts(
    unified: &mut UnifiedAnalysis,
    file_groups: &std::collections::HashMap<std::path::PathBuf, Vec<FunctionMetrics>>,
) {
    use crate::metrics::loc_counter::LocCounter;

    let loc_counter = LocCounter::default();
    for file_path in file_groups.keys() {
        if let Ok(loc_count) = loc_counter.count_file(file_path) {
            unified.register_analyzed_file(file_path.clone(), loc_count.physical_lines);
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

    #[test]
    fn test_should_process_file_high_score() {
        assert!(should_process_file(60.0, false));
        assert!(should_process_file(51.0, false));
    }

    #[test]
    fn test_should_process_file_low_score() {
        assert!(!should_process_file(50.0, false));
        assert!(!should_process_file(30.0, false));
    }

    #[test]
    fn test_should_process_file_with_god_object() {
        assert!(should_process_file(30.0, true));
        assert!(should_process_file(0.0, true));
    }

    #[test]
    fn test_should_process_file_boundary() {
        // Exactly 50.0 should NOT be processed (> 50.0 required)
        assert!(!should_process_file(50.0, false));
        // Just above threshold should be processed
        assert!(should_process_file(50.1, false));
    }

    #[test]
    fn test_is_god_object_suppressed_with_file_annotation() {
        use crate::organization::{DetectionType, GodObjectConfidence, SplitAnalysisMethod};
        use std::path::PathBuf;

        // Create god object analysis for a file-level detection
        let god_analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 50,
            weighted_method_count: None,
            field_count: 10,
            responsibility_count: 5,
            lines_of_code: 2000,
            complexity_sum: 100,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data".to_string()],
            responsibility_method_counts: std::collections::HashMap::new(),
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodFile,
            struct_name: None,
            struct_line: None,
            struct_location: None,
            visibility_breakdown: None,
            domain_count: 2,
            domain_diversity: 0.5,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,
            trait_method_summary: None,
        };

        // Test with suppression annotation at line 1
        let file_path = PathBuf::from("test.rs");
        let content = r#"// debtmap:ignore[god_object] - High cohesion: all functions implement merge queue management
use std::io;
fn main() {}
"#;
        let suppression_context = parse_suppression_comments(content, Language::Rust, &file_path);

        // Should be suppressed because of the annotation at line 1
        assert!(
            is_god_object_suppressed(&god_analysis, &suppression_context),
            "God object with file-level ignore annotation should be suppressed"
        );
    }

    #[test]
    fn test_is_god_object_not_suppressed_without_annotation() {
        use crate::organization::{DetectionType, GodObjectConfidence, SplitAnalysisMethod};
        use std::path::PathBuf;

        let god_analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 50,
            weighted_method_count: None,
            field_count: 10,
            responsibility_count: 5,
            lines_of_code: 2000,
            complexity_sum: 100,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data".to_string()],
            responsibility_method_counts: std::collections::HashMap::new(),
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodFile,
            struct_name: None,
            struct_line: None,
            struct_location: None,
            visibility_breakdown: None,
            domain_count: 2,
            domain_diversity: 0.5,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,
            trait_method_summary: None,
        };

        // Test without any suppression annotation
        let file_path = PathBuf::from("test.rs");
        let content = r#"use std::io;
fn main() {}
"#;
        let suppression_context = parse_suppression_comments(content, Language::Rust, &file_path);

        // Should NOT be suppressed because there's no annotation
        assert!(
            !is_god_object_suppressed(&god_analysis, &suppression_context),
            "God object without ignore annotation should not be suppressed"
        );
    }

    #[test]
    fn test_is_god_object_suppressed_struct_level() {
        use crate::organization::{DetectionType, GodObjectConfidence, SplitAnalysisMethod};
        use std::path::PathBuf;

        // Create god object analysis for a struct-level detection (GodClass)
        let god_analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 50,
            weighted_method_count: None,
            field_count: 10,
            responsibility_count: 5,
            lines_of_code: 2000,
            complexity_sum: 100,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data".to_string()],
            responsibility_method_counts: std::collections::HashMap::new(),
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            struct_name: Some("MergeQueueManager".to_string()),
            struct_line: Some(10), // Struct is at line 10
            struct_location: None,
            visibility_breakdown: None,
            domain_count: 2,
            domain_diversity: 0.5,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,
            trait_method_summary: None,
        };

        // Test with suppression annotation before the struct (at line 9)
        let file_path = PathBuf::from("test.rs");
        let content = r#"use std::io;

// Some code...


// More code...


// debtmap:ignore[god_object] - Coordinator struct by design
pub struct MergeQueueManager {
    field: String,
}
"#;
        let suppression_context = parse_suppression_comments(content, Language::Rust, &file_path);

        // Should be suppressed because of the annotation before the struct
        assert!(
            is_god_object_suppressed(&god_analysis, &suppression_context),
            "GodClass with ignore annotation before struct should be suppressed"
        );
    }

    #[test]
    fn test_is_god_object_wrong_debt_type_not_suppressed() {
        use crate::organization::{DetectionType, GodObjectConfidence, SplitAnalysisMethod};
        use std::path::PathBuf;

        let god_analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 50,
            weighted_method_count: None,
            field_count: 10,
            responsibility_count: 5,
            lines_of_code: 2000,
            complexity_sum: 100,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data".to_string()],
            responsibility_method_counts: std::collections::HashMap::new(),
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodFile,
            struct_name: None,
            struct_line: None,
            struct_location: None,
            visibility_breakdown: None,
            domain_count: 2,
            domain_diversity: 0.5,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,
            trait_method_summary: None,
        };

        // Test with a different suppression type (testing, not god_object)
        let file_path = PathBuf::from("test.rs");
        let content = r#"// debtmap:ignore[testing] - Not a god_object annotation
use std::io;
fn main() {}
"#;
        let suppression_context = parse_suppression_comments(content, Language::Rust, &file_path);

        // Should NOT be suppressed because the annotation is for testing, not god_object
        assert!(
            !is_god_object_suppressed(&god_analysis, &suppression_context),
            "God object should not be suppressed by testing annotation"
        );
    }

    #[test]
    fn test_is_god_class_suppressed_by_file_level_annotation() {
        use crate::organization::{DetectionType, GodObjectConfidence, SplitAnalysisMethod};
        use std::path::PathBuf;

        // Simulates the real-world case: hosaka's merge.rs has a GodClass (MergeQueueManager)
        // at line 469, but the suppression annotation is at line 1 of the file.
        // The file-level annotation should apply to all god objects in the file.
        let god_analysis = GodObjectAnalysis {
            is_god_object: true,
            method_count: 50,
            weighted_method_count: None,
            field_count: 10,
            responsibility_count: 5,
            lines_of_code: 2000,
            complexity_sum: 100,
            god_object_score: 75.0,
            recommended_splits: vec![],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data".to_string()],
            responsibility_method_counts: std::collections::HashMap::new(),
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            struct_name: Some("MergeQueueManager".to_string()),
            struct_line: Some(469), // Struct is far down in the file
            struct_location: None,
            visibility_breakdown: None,
            domain_count: 2,
            domain_diversity: 0.5,
            struct_ratio: 0.0,
            analysis_method: SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,
            trait_method_summary: None,
        };

        // File-level annotation at line 1 should apply to the GodClass at line 469
        let file_path = PathBuf::from("merge.rs");
        let content = r#"// debtmap:ignore[god_object] - High cohesion (0.95): all functions implement merge queue management
// as a single domain. Splitting would reduce cohesion.
use super::template::MergeMode;
use serde::{Deserialize, Serialize};
// ... many more lines ...
pub struct MergeQueueManager {
    field: String,
}
"#;
        let suppression_context = parse_suppression_comments(content, Language::Rust, &file_path);

        // Should be suppressed because of the file-level annotation at line 1
        assert!(
            is_god_object_suppressed(&god_analysis, &suppression_context),
            "GodClass should be suppressed by file-level annotation at line 1"
        );
    }
}
