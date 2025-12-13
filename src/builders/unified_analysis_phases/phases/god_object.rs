//! Pure god object detection and debt item creation.
//!
//! This module provides pure functions for detecting god objects
//! and creating debt items without any I/O or progress reporting.

use crate::organization::GodObjectAnalysis;
use crate::priority::file_metrics::FileDebtMetrics;
use crate::priority::god_object_aggregation::GodObjectAggregatedMetrics;
use crate::priority::{
    score_types::Score0To100, ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics,
    TransitiveCoverage, UnifiedDebtItem, UnifiedScore,
};
use crate::risk::context::ContextualRisk;
use crate::risk::lcov::LcovData;
use std::path::Path;

/// Create a UnifiedDebtItem from god object indicators (pure).
///
/// God objects are file-level technical debt items representing files with
/// too many responsibilities, methods, or fields. They bypass function-level
/// complexity filtering since they represent architectural issues.
pub fn create_god_object_debt_item(
    file_path: &Path,
    file_metrics: &FileDebtMetrics,
    god_analysis: &GodObjectAnalysis,
    mut aggregated_metrics: GodObjectAggregatedMetrics,
    coverage_data: Option<&LcovData>,
) -> UnifiedDebtItem {
    // Fallback: If no function-level coverage, use file-level coverage from LCOV
    if aggregated_metrics.weighted_coverage.is_none() {
        if let Some(coverage) = coverage_data {
            if let Some(file_coverage) = coverage.get_file_coverage(file_path) {
                aggregated_metrics.weighted_coverage = Some(TransitiveCoverage {
                    direct: file_coverage,
                    transitive: 0.0,
                    propagated_from: vec![],
                    uncovered_lines: vec![],
                });
            }
        }
    }

    // Calculate unified score
    let unified_score = calculate_god_object_score(god_analysis, &aggregated_metrics);

    // Create debt type
    let debt_type = create_god_object_debt_type(god_analysis);

    // Determine display name and line number
    let (display_name, line_number) = determine_display_info(file_path, god_analysis);

    // Create impact metrics
    let expected_impact = calculate_god_object_impact(god_analysis, file_metrics);

    // Create recommendation
    let recommendation = create_god_object_recommendation(god_analysis);

    // Determine tier
    let base_score = god_analysis.god_object_score.value();
    let tier = if base_score >= 50.0 {
        crate::priority::RecommendationTier::T1CriticalArchitecture
    } else {
        crate::priority::RecommendationTier::T2ComplexUntested
    };

    UnifiedDebtItem {
        location: crate::priority::unified_scorer::Location {
            file: file_path.to_path_buf(),
            function: display_name,
            line: line_number,
        },
        debt_type,
        unified_score,
        function_role: FunctionRole::Unknown,
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
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
        is_pure: None,
        purity_confidence: None,
        purity_level: None,
        god_object_indicators: Some(god_analysis.clone()),
        tier: Some(tier),
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
        responsibility_category: god_analysis.responsibilities.first().cloned(),
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    }
}

/// Calculate unified score for god object (pure).
fn calculate_god_object_score(
    god_analysis: &GodObjectAnalysis,
    aggregated_metrics: &GodObjectAggregatedMetrics,
) -> UnifiedScore {
    let base_score = god_analysis.god_object_score.value();

    // Use aggregated coverage in score calculation
    let coverage_factor = aggregated_metrics
        .weighted_coverage
        .as_ref()
        .map(|cov| (1.0 - cov.direct) * 10.0)
        .unwrap_or(0.0);

    // Apply coverage as dampening multiplier
    let coverage_multiplier = aggregated_metrics
        .weighted_coverage
        .as_ref()
        .map(|cov| 1.0 - cov.direct)
        .unwrap_or(1.0);
    let coverage_adjusted_score = base_score * coverage_multiplier;

    let total_complexity = aggregated_metrics.total_cyclomatic + aggregated_metrics.total_cognitive;
    let mut unified_score = UnifiedScore {
        final_score: Score0To100::new(coverage_adjusted_score),
        complexity_factor: total_complexity as f64 / 10.0,
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

    // Apply contextual risk to score if available
    if let Some(ref ctx_risk) = aggregated_metrics.aggregated_contextual_risk {
        unified_score = crate::priority::scoring::construction::apply_contextual_risk_to_score(
            unified_score,
            ctx_risk,
        );
    }

    unified_score
}

/// Create debt type for god object (pure).
fn create_god_object_debt_type(god_analysis: &GodObjectAnalysis) -> DebtType {
    DebtType::GodObject {
        methods: god_analysis.method_count as u32,
        fields: match god_analysis.detection_type {
            crate::organization::DetectionType::GodClass => Some(god_analysis.field_count as u32),
            crate::organization::DetectionType::GodFile
            | crate::organization::DetectionType::GodModule => None,
        },
        responsibilities: god_analysis.responsibility_count as u32,
        god_object_score: god_analysis.god_object_score,
        lines: god_analysis.lines_of_code as u32,
    }
}

/// Determine display name and line number based on detection type (pure).
fn determine_display_info(file_path: &Path, god_analysis: &GodObjectAnalysis) -> (String, usize) {
    match god_analysis.detection_type {
        crate::organization::DetectionType::GodClass => {
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
            let name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            (name.to_string(), 1)
        }
    }
}

/// Calculate impact metrics for god object (pure).
fn calculate_god_object_impact(
    god_analysis: &GodObjectAnalysis,
    file_metrics: &FileDebtMetrics,
) -> ImpactMetrics {
    ImpactMetrics {
        coverage_improvement: 0.0,
        lines_reduction: god_analysis.lines_of_code as u32
            / god_analysis.recommended_splits.len().max(1) as u32,
        complexity_reduction: file_metrics.total_complexity as f64
            / god_analysis.recommended_splits.len().max(1) as f64,
        risk_reduction: calculate_god_object_risk(god_analysis),
    }
}

/// Calculate risk score for god object (pure).
pub fn calculate_god_object_risk(god_analysis: &GodObjectAnalysis) -> f64 {
    let responsibility_risk = god_analysis.responsibility_count as f64 * 10.0;
    let method_risk = (god_analysis.method_count as f64 / 10.0).min(50.0);
    (responsibility_risk + method_risk).min(100.0)
}

/// Create actionable recommendation for god object (pure).
pub fn create_god_object_recommendation(
    god_analysis: &GodObjectAnalysis,
) -> ActionableRecommendation {
    // Calculate recommended split count
    let split_count = if god_analysis.recommended_splits.len() >= 2 {
        god_analysis.recommended_splits.len()
    } else {
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

/// Update function god indicators in unified analysis (pure transformation).
pub fn enrich_god_analysis_with_aggregates(
    god_analysis: &GodObjectAnalysis,
    aggregated_metrics: &GodObjectAggregatedMetrics,
) -> GodObjectAnalysis {
    let mut enriched = god_analysis.clone();
    enriched.aggregated_entropy = aggregated_metrics.aggregated_entropy.clone();
    enriched.aggregated_error_swallowing_count =
        if aggregated_metrics.total_error_swallowing_count > 0 {
            Some(aggregated_metrics.total_error_swallowing_count)
        } else {
            None
        };
    enriched.aggregated_error_swallowing_patterns =
        if !aggregated_metrics.error_swallowing_patterns.is_empty() {
            Some(aggregated_metrics.error_swallowing_patterns.clone())
        } else {
            None
        };
    enriched
}

/// Analyze file-level git context for god objects (pure).
///
/// Returns contextual risk based on file's git history.
pub fn analyze_file_git_context(
    file_path: &std::path::Path,
    risk_analyzer: &crate::risk::RiskAnalyzer,
    project_root: &std::path::Path,
) -> Option<ContextualRisk> {
    if !risk_analyzer.has_context() {
        return None;
    }

    // Base risk of 40 represents moderate-high risk for god objects
    let base_risk = 40.0;

    risk_analyzer.analyze_file_context(
        file_path.to_path_buf(),
        base_risk,
        project_root.to_path_buf(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::god_object::ModuleSplit;
    use crate::organization::{DetectionType, GodObjectConfidence, SplitAnalysisMethod};
    use std::collections::HashMap;

    fn create_test_god_analysis() -> GodObjectAnalysis {
        GodObjectAnalysis {
            is_god_object: true,
            method_count: 50,
            field_count: 10,
            responsibility_count: 5,
            lines_of_code: 2000,
            complexity_sum: 100,
            god_object_score: Score0To100::new(75.0),
            recommended_splits: vec![
                ModuleSplit {
                    suggested_name: "module_a".to_string(),
                    responsibility: "data".to_string(),
                    estimated_lines: 1000,
                    method_count: 25,
                    ..Default::default()
                },
                ModuleSplit {
                    suggested_name: "module_b".to_string(),
                    responsibility: "io".to_string(),
                    estimated_lines: 1000,
                    method_count: 25,
                    ..Default::default()
                },
            ],
            confidence: GodObjectConfidence::Probable,
            responsibilities: vec!["data".to_string(), "io".to_string()],
            responsibility_method_counts: HashMap::new(),
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
        }
    }

    #[test]
    fn test_calculate_god_object_risk() {
        let analysis = create_test_god_analysis();
        let risk = calculate_god_object_risk(&analysis);

        // 5 responsibilities * 10 = 50
        // 50 methods / 10 = 5, capped at 50
        // Total = 55
        assert!(risk > 0.0);
        assert!(risk <= 100.0);
    }

    #[test]
    fn test_create_god_object_recommendation() {
        let analysis = create_test_god_analysis();
        let rec = create_god_object_recommendation(&analysis);

        assert!(rec.primary_action.contains("Split into"));
        assert!(rec.rationale.contains("5 responsibilities"));
    }

    #[test]
    fn test_determine_display_info_god_file() {
        let analysis = create_test_god_analysis();
        let file_path = std::path::PathBuf::from("/path/to/large_file.rs");
        let (name, line) = determine_display_info(&file_path, &analysis);

        assert_eq!(name, "large_file.rs");
        assert_eq!(line, 1);
    }

    #[test]
    fn test_determine_display_info_god_class() {
        let mut analysis = create_test_god_analysis();
        analysis.detection_type = DetectionType::GodClass;
        analysis.struct_name = Some("MyLargeStruct".to_string());
        analysis.struct_line = Some(42);

        let file_path = std::path::PathBuf::from("/path/to/file.rs");
        let (name, line) = determine_display_info(&file_path, &analysis);

        assert_eq!(name, "MyLargeStruct");
        assert_eq!(line, 42);
    }
}
