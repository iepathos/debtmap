//! Pure god object detection and debt item creation.
//!
//! This module provides pure functions for detecting god objects
//! and creating debt items without any I/O or progress reporting.

use crate::organization::GodObjectAnalysis;
use crate::priority::file_metrics::FileDebtMetrics;
use crate::priority::god_object_aggregation::GodObjectAggregatedMetrics;
use crate::priority::{
    ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, TransitiveCoverage,
    UnifiedDebtItem, UnifiedScore,
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
    let base_score = god_analysis.god_object_score;
    let tier = if base_score >= 50.0 {
        crate::priority::RecommendationTier::T1CriticalArchitecture
    } else {
        crate::priority::RecommendationTier::T2ComplexUntested
    };

    // Determine appropriate function role based on detection type (spec 233)
    // God objects are architectural issues - classify based on their nature
    let function_role = classify_god_object_role(god_analysis);

    UnifiedDebtItem {
        location: crate::priority::unified_scorer::Location {
            file: file_path.to_path_buf(),
            function: display_name,
            line: line_number,
        },
        debt_type,
        unified_score,
        function_role,
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
        entropy_details: aggregated_metrics.aggregated_entropy.clone(),
        entropy_analysis: None, // TODO(spec 218): Convert from aggregated_entropy
        entropy_adjusted_cognitive: aggregated_metrics
            .aggregated_entropy
            .as_ref()
            .map(|e| e.adjusted_cognitive),
        entropy_dampening_factor: aggregated_metrics
            .aggregated_entropy
            .as_ref()
            .map(|e| e.dampening_factor),
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
        context_suggestion: None,
    }
}

/// Calculate unified score for god object (pure).
fn calculate_god_object_score(
    god_analysis: &GodObjectAnalysis,
    aggregated_metrics: &GodObjectAggregatedMetrics,
) -> UnifiedScore {
    let base_score = god_analysis.god_object_score;

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
    let has_coverage_data = aggregated_metrics.weighted_coverage.is_some();
    let mut unified_score = UnifiedScore {
        final_score: coverage_adjusted_score.max(0.0),
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
        // Spec 260: Score transparency fields
        debt_adjustment: None,
        pre_normalization_score: None,
        structural_multiplier: Some(1.0),
        has_coverage_data,
        contextual_risk_multiplier: None,
        pre_contextual_score: None,
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

/// Classify function role for god object items (spec 233, refined spec 270).
///
/// God objects are architectural-level debt, not function-level. However, we assign
/// a meaningful role based on their characteristics to improve filtering and prioritization.
///
/// Classification priority (first match wins):
/// 1. IOWrapper: >40% impure methods (I/O operations dominate)
/// 2. PureLogic: Trait-dominated struct (>50% trait-mandated methods like syn::Visit)
/// 3. PureLogic: Mostly pure functions (weighted/raw ratio < 40%)
/// 4. PureLogic: Default for large structs without clear orchestration evidence
///
/// Note: "Orchestrator" role is reserved for structs that actively coordinate
/// calls to other components. A struct with many visitor methods is NOT an
/// orchestrator - it's implementing a traversal pattern.
fn classify_god_object_role(god_analysis: &GodObjectAnalysis) -> FunctionRole {
    // 1. If purity distribution shows mostly impure operations, classify as IOWrapper
    if let Some(ref purity) = god_analysis.purity_distribution {
        // Check if more than 40% of methods are impure (I/O-related)
        let total_methods = purity.pure_count + purity.probably_pure_count + purity.impure_count;
        if total_methods > 0 {
            let io_ratio = purity.impure_count as f64 / total_methods as f64;
            if io_ratio > 0.4 {
                return FunctionRole::IOWrapper;
            }
        }
    }

    // 2. Check trait method summary - if trait methods dominate, this is likely
    // a legitimate implementation (visitor, serializer, etc.), not an orchestrator
    if let Some(ref trait_summary) = god_analysis.trait_method_summary {
        // If >50% of methods are trait-mandated, classify as PureLogic
        // These are structural requirements, not evidence of orchestration
        if trait_summary.mandated_ratio() > 0.5 {
            return FunctionRole::PureLogic;
        }
    }

    // 3. Check if mostly pure functions based on weighted method count
    // If weighted_method_count is <40% of raw count, these are mostly pure helpers
    // This indicates a cohesive utility module, not an orchestrator
    if let Some(weighted) = god_analysis.weighted_method_count {
        let raw = god_analysis.method_count as f64;
        if raw > 0.0 && weighted / raw < 0.4 {
            // Mostly pure functions - this is a utility module, not an orchestrator
            return FunctionRole::PureLogic;
        }
    }

    // 4. Default to PureLogic for large structs
    // "Orchestrator" should only be assigned when there's clear evidence of
    // coordination (e.g., many calls to other components, delegation patterns).
    // High method count alone is not evidence of orchestration - it could be
    // a visitor pattern, utility module, or large API surface.
    FunctionRole::PureLogic
}

/// Calculate classification confidence for god object role (spec 233, refined spec 270).
///
/// Returns a confidence score between 0.0 and 1.0 based on how strongly
/// the analysis data supports the role classification.
///
/// Confidence factors:
/// - IOWrapper: Higher impurity ratio = higher confidence
/// - PureLogic (trait-dominated): Higher mandated ratio = higher confidence
/// - PureLogic (pure utility): Lower weighted/raw ratio = higher confidence
/// - PureLogic (default): Base confidence for unclear cases
#[allow(dead_code)]
pub fn calculate_role_confidence(god_analysis: &GodObjectAnalysis) -> f64 {
    // Check IOWrapper classification confidence
    if let Some(ref purity) = god_analysis.purity_distribution {
        let total_methods = purity.pure_count + purity.probably_pure_count + purity.impure_count;
        if total_methods > 0 {
            let io_ratio = purity.impure_count as f64 / total_methods as f64;
            if io_ratio > 0.4 {
                // IOWrapper confidence: 60% base + up to 35% based on how far above 40% we are
                // io_ratio of 1.0 = 95% confidence, io_ratio of 0.4 = 60% confidence
                return 0.60 + ((io_ratio - 0.4) / 0.6) * 0.35;
            }
        }
    }

    // Check trait-dominated classification confidence
    if let Some(ref trait_summary) = god_analysis.trait_method_summary {
        let mandated_ratio = trait_summary.mandated_ratio();
        if mandated_ratio > 0.5 {
            // Trait-dominated confidence: 70% base + up to 25% based on how dominant
            // mandated_ratio of 1.0 = 95% confidence, ratio of 0.5 = 70% confidence
            return 0.70 + ((mandated_ratio - 0.5) / 0.5) * 0.25;
        }
    }

    // Check pure utility module confidence
    if let Some(weighted) = god_analysis.weighted_method_count {
        let raw = god_analysis.method_count as f64;
        if raw > 0.0 {
            let purity_ratio = weighted / raw;
            if purity_ratio < 0.4 {
                // Pure utility confidence: 75% base + up to 20% based on how pure
                // purity_ratio of 0.0 = 95% confidence, ratio of 0.4 = 75% confidence
                return 0.75 + ((0.4 - purity_ratio) / 0.4) * 0.20;
            }
        }
    }

    // Default PureLogic: moderate confidence (65-75%)
    // More methods/responsibilities = higher confidence it's intentional design
    let method_score = (god_analysis.method_count as f64 / 20.0).min(1.0);
    let responsibility_score = (god_analysis.responsibility_count as f64 / 5.0).min(1.0);
    let size_score = (method_score + responsibility_score) / 2.0;
    0.65 + size_score * 0.10
}

/// Determine display name and line number based on detection type (pure).
///
/// For GodClass: Returns the struct name and its line number.
/// For GodFile/GodModule: Returns "[file-scope]" to clarify this is file-level debt (spec 233).
///
/// Using "[file-scope]" instead of the filename prevents confusion where
/// users might think "large_file.rs" is a function name when it's actually
/// the file being analyzed.
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
            // Use "[file-scope]" to indicate this is file-level debt, not a function (spec 233)
            // The actual filename is already in the location.file field
            ("[file-scope]".to_string(), 1)
        }
    }
}

/// Validate that a function-level debt item has a proper function name (spec 233).
///
/// Returns true if the function name is valid (not a filename or placeholder).
/// This is used to detect potential classification issues.
#[allow(dead_code)]
pub fn is_valid_function_name(function_name: &str) -> bool {
    // Check for common filename extensions
    let filename_patterns = [".rs", ".py", ".js", ".ts", ".go", ".java", ".cpp", ".c"];
    let is_filename = filename_patterns
        .iter()
        .any(|ext| function_name.ends_with(ext));

    // Check for file-scope placeholder
    let is_placeholder = function_name == "[file-scope]";

    // Valid function names should not be filenames (unless it's the placeholder)
    !is_filename || is_placeholder
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
///
/// Recommendations are context-specific based on the detected function role (spec 233).
pub fn create_god_object_recommendation(
    god_analysis: &GodObjectAnalysis,
) -> ActionableRecommendation {
    // Classify role to generate context-specific recommendation
    let role = classify_god_object_role(god_analysis);
    create_god_object_recommendation_with_role(god_analysis, role)
}

/// Create actionable recommendation for god object with explicit role (pure).
///
/// Generates context-specific recommendations based on the function role:
/// - Orchestrator: Extract coordination logic into smaller orchestrators
/// - IOWrapper: Separate I/O operations from business logic
/// - PureLogic: Split pure computation into focused modules
fn create_god_object_recommendation_with_role(
    god_analysis: &GodObjectAnalysis,
    role: FunctionRole,
) -> ActionableRecommendation {
    // Calculate recommended split count
    let split_count = if god_analysis.recommended_splits.len() >= 2 {
        god_analysis.recommended_splits.len()
    } else {
        god_analysis.responsibility_count.clamp(2, 5)
    };

    // Check if this is a cohesive utility module (mostly pure functions)
    let is_pure_utility_module = god_analysis
        .weighted_method_count
        .map(|w| w / (god_analysis.method_count as f64) < 0.4)
        .unwrap_or(false);

    // Generate role-specific primary action (spec 233)
    let primary_action = match role {
        FunctionRole::Orchestrator => format!(
            "Extract {} sub-orchestrators to reduce coordination complexity",
            split_count
        ),
        FunctionRole::IOWrapper => format!(
            "Separate {} I/O handlers from business logic",
            god_analysis.responsibility_count
        ),
        FunctionRole::PureLogic if is_pure_utility_module => {
            format!(
                "Review {} detected responsibilities - consider grouping related helpers into submodules",
                god_analysis.responsibility_count
            )
        }
        _ => format!("Split into {} modules by responsibility", split_count),
    };

    // Generate role-specific rationale (spec 233)
    let rationale = match role {
        FunctionRole::Orchestrator => format!(
            "High coordination complexity: {} responsibilities with {} methods - \
            extracting sub-orchestrators will reduce cognitive load and improve testability",
            god_analysis.responsibility_count, god_analysis.method_count
        ),
        FunctionRole::IOWrapper => format!(
            "Mixed I/O concerns: {} responsibilities detected - \
            separating I/O from pure logic enables better testing and reduces coupling",
            god_analysis.responsibility_count
        ),
        FunctionRole::PureLogic if is_pure_utility_module => {
            let weighted = god_analysis.weighted_method_count.unwrap_or(0.0);
            format!(
                "Utility module with {} pure helper functions (weighted: {:.0}). \
                High function count but low effective complexity - \
                verify if detected responsibilities represent distinct concerns",
                god_analysis.method_count, weighted
            )
        }
        _ => format!(
            "{} responsibilities detected with {} methods/functions - \
            splitting will improve maintainability and enable focused testing",
            god_analysis.responsibility_count, god_analysis.method_count
        ),
    };

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
            weighted_method_count: None,
            field_count: 10,
            responsibility_count: 5,
            lines_of_code: 2000,
            complexity_sum: 100,
            god_object_score: 75.0,
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
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,   // Spec 211
            trait_method_summary: None, // Spec 217
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

        // Default test fixture (50 methods, 5 responsibilities) is now classified as PureLogic
        // (spec 270: high method count alone is not evidence of orchestration)
        // so it should recommend splitting by responsibility
        assert!(
            rec.primary_action.contains("Split into")
                || rec.primary_action.contains("modules by responsibility"),
            "Expected pure logic recommendation (split by responsibility), got: {}",
            rec.primary_action
        );
        assert!(rec.rationale.contains("responsibilities"));
    }

    #[test]
    fn test_determine_display_info_god_file() {
        let analysis = create_test_god_analysis();
        let file_path = std::path::PathBuf::from("/path/to/large_file.rs");
        let (name, line) = determine_display_info(&file_path, &analysis);

        // Spec 233: GodFile should use "[file-scope]" to avoid confusion with function names
        assert_eq!(name, "[file-scope]");
        assert_eq!(line, 1);
    }

    #[test]
    fn test_is_valid_function_name() {
        // File-scope placeholder is valid (for file-level debt)
        assert!(is_valid_function_name("[file-scope]"));

        // Normal function names are valid
        assert!(is_valid_function_name("process_data"));
        assert!(is_valid_function_name("MyStruct"));
        assert!(is_valid_function_name("calculate_risk"));

        // Filenames are NOT valid as function names (indicates misclassification)
        assert!(!is_valid_function_name("large_file.rs"));
        assert!(!is_valid_function_name("module.py"));
        assert!(!is_valid_function_name("handler.ts"));
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

    // Spec 233/270: God object role classification tests
    // Spec 270: Changed default from Orchestrator to PureLogic for large structs
    // High method count alone is not evidence of orchestration

    #[test]
    fn test_classify_god_object_role_pure_logic_many_methods() {
        let mut analysis = create_test_god_analysis();
        analysis.method_count = 20; // Many methods
        analysis.responsibility_count = 2;
        analysis.purity_distribution = None;
        analysis.trait_method_summary = None;

        let role = classify_god_object_role(&analysis);
        assert_eq!(
            role,
            FunctionRole::PureLogic,
            "God object with many methods should default to PureLogic (not Orchestrator)"
        );
    }

    #[test]
    fn test_classify_god_object_role_pure_logic_many_responsibilities() {
        let mut analysis = create_test_god_analysis();
        analysis.method_count = 5;
        analysis.responsibility_count = 10; // Many responsibilities
        analysis.purity_distribution = None;
        analysis.trait_method_summary = None;

        let role = classify_god_object_role(&analysis);
        assert_eq!(
            role,
            FunctionRole::PureLogic,
            "God object with many responsibilities should default to PureLogic (not Orchestrator)"
        );
    }

    #[test]
    fn test_classify_god_object_role_pure_logic_small() {
        let mut analysis = create_test_god_analysis();
        analysis.method_count = 5; // < 10 methods
        analysis.responsibility_count = 2; // < 3 responsibilities
        analysis.purity_distribution = None;

        let role = classify_god_object_role(&analysis);
        assert_eq!(
            role,
            FunctionRole::PureLogic,
            "Small god object should be PureLogic"
        );
    }

    #[test]
    fn test_classify_god_object_role_pure_logic_trait_dominated() {
        use crate::organization::god_object::TraitMethodSummary;

        let mut analysis = create_test_god_analysis();
        analysis.method_count = 30;
        analysis.responsibility_count = 8;
        analysis.purity_distribution = None;
        // Trait-dominated: 60% of methods are trait-mandated (like syn::Visit)
        analysis.trait_method_summary = Some(TraitMethodSummary {
            mandated_count: 18,
            by_trait: [("syn::Visit".into(), 18)].into_iter().collect(),
            weighted_count: 13.8, // 18 * 0.1 + 12 * 1.0
            extractable_count: 12,
            total_methods: 30,
        });

        let role = classify_god_object_role(&analysis);
        assert_eq!(
            role,
            FunctionRole::PureLogic,
            "Trait-dominated struct (>50% trait-mandated) should be PureLogic"
        );
    }

    #[test]
    fn test_classify_god_object_role_io_wrapper_high_impure() {
        use crate::organization::god_object::PurityDistribution;

        let mut analysis = create_test_god_analysis();
        analysis.method_count = 20;
        analysis.responsibility_count = 4;
        // More than 40% impure
        analysis.purity_distribution = Some(PurityDistribution {
            pure_count: 2,
            probably_pure_count: 2,
            impure_count: 6, // 6/10 = 60% > 40%
            pure_weight_contribution: 0.2,
            probably_pure_weight_contribution: 0.2,
            impure_weight_contribution: 0.6,
        });

        let role = classify_god_object_role(&analysis);
        assert_eq!(
            role,
            FunctionRole::IOWrapper,
            "God object with high impure ratio should be IOWrapper"
        );
    }

    #[test]
    fn test_classify_god_object_role_pure_logic_low_impure() {
        use crate::organization::god_object::PurityDistribution;

        let mut analysis = create_test_god_analysis();
        analysis.method_count = 20;
        analysis.responsibility_count = 4;
        analysis.trait_method_summary = None;
        // Less than 40% impure
        analysis.purity_distribution = Some(PurityDistribution {
            pure_count: 6,
            probably_pure_count: 2,
            impure_count: 2, // 2/10 = 20% < 40%
            pure_weight_contribution: 0.6,
            probably_pure_weight_contribution: 0.2,
            impure_weight_contribution: 0.2,
        });

        let role = classify_god_object_role(&analysis);
        assert_eq!(
            role,
            FunctionRole::PureLogic,
            "God object with low impure ratio should be PureLogic (not IOWrapper, not Orchestrator)"
        );
    }

    // Spec 233: Context-specific recommendation tests

    #[test]
    fn test_recommendation_orchestrator_role() {
        let mut analysis = create_test_god_analysis();
        analysis.method_count = 20;
        analysis.responsibility_count = 4;

        let rec = create_god_object_recommendation_with_role(&analysis, FunctionRole::Orchestrator);

        assert!(
            rec.primary_action.contains("sub-orchestrators"),
            "Orchestrator recommendation should mention sub-orchestrators"
        );
        assert!(
            rec.rationale.contains("coordination complexity"),
            "Orchestrator rationale should mention coordination"
        );
    }

    #[test]
    fn test_recommendation_io_wrapper_role() {
        let mut analysis = create_test_god_analysis();
        analysis.responsibility_count = 3;

        let rec = create_god_object_recommendation_with_role(&analysis, FunctionRole::IOWrapper);

        assert!(
            rec.primary_action.contains("I/O handlers"),
            "IOWrapper recommendation should mention I/O handlers"
        );
        assert!(
            rec.rationale.contains("I/O concerns"),
            "IOWrapper rationale should mention I/O concerns"
        );
    }

    #[test]
    fn test_recommendation_pure_logic_role() {
        let mut analysis = create_test_god_analysis();
        analysis.responsibility_count = 3;

        let rec = create_god_object_recommendation_with_role(&analysis, FunctionRole::PureLogic);

        assert!(
            rec.primary_action.contains("Split into"),
            "PureLogic recommendation should mention splitting"
        );
        assert!(
            rec.rationale.contains("maintainability"),
            "PureLogic rationale should mention maintainability"
        );
    }

    // Spec 233: Role classification confidence tests

    #[test]
    fn test_confidence_io_wrapper_high() {
        use crate::organization::god_object::PurityDistribution;

        let mut analysis = create_test_god_analysis();
        // 80% impure - very high confidence
        analysis.purity_distribution = Some(PurityDistribution {
            pure_count: 1,
            probably_pure_count: 1,
            impure_count: 8, // 8/10 = 80%
            pure_weight_contribution: 0.1,
            probably_pure_weight_contribution: 0.1,
            impure_weight_contribution: 0.8,
        });

        let confidence = calculate_role_confidence(&analysis);
        // Should be high confidence (>= 80%)
        assert!(
            confidence >= 0.80,
            "High impure ratio should give high confidence: {:.2}%",
            confidence * 100.0
        );
    }

    #[test]
    fn test_confidence_io_wrapper_threshold() {
        use crate::organization::god_object::PurityDistribution;

        let mut analysis = create_test_god_analysis();
        // 50% impure - just above threshold
        analysis.purity_distribution = Some(PurityDistribution {
            pure_count: 3,
            probably_pure_count: 2,
            impure_count: 5, // 5/10 = 50%
            pure_weight_contribution: 0.3,
            probably_pure_weight_contribution: 0.2,
            impure_weight_contribution: 0.5,
        });

        let confidence = calculate_role_confidence(&analysis);
        // Should be moderate confidence (60-70%)
        assert!(
            (0.60..0.70).contains(&confidence),
            "At-threshold impure ratio should give moderate confidence: {:.2}%",
            confidence * 100.0
        );
    }

    #[test]
    fn test_confidence_pure_logic_large_struct() {
        let mut analysis = create_test_god_analysis();
        analysis.method_count = 25;
        analysis.responsibility_count = 5;
        analysis.purity_distribution = None;
        analysis.trait_method_summary = None;

        let confidence = calculate_role_confidence(&analysis);
        // Large struct defaults to PureLogic with moderate-high confidence (65-75%)
        // More methods/responsibilities = higher confidence in intentional design
        assert!(
            (0.65..0.80).contains(&confidence),
            "Large struct should have moderate-high confidence: {:.2}%",
            confidence * 100.0
        );
    }

    #[test]
    fn test_confidence_pure_logic_moderate_struct() {
        let mut analysis = create_test_god_analysis();
        analysis.method_count = 12;
        analysis.responsibility_count = 3;
        analysis.purity_distribution = None;
        analysis.trait_method_summary = None;

        let confidence = calculate_role_confidence(&analysis);
        // Moderate struct defaults to PureLogic with moderate confidence (65-75%)
        assert!(
            (0.65..0.75).contains(&confidence),
            "Moderate struct should have moderate confidence: {:.2}%",
            confidence * 100.0
        );
    }

    #[test]
    fn test_confidence_pure_logic_small_struct() {
        let mut analysis = create_test_god_analysis();
        analysis.method_count = 5;
        analysis.responsibility_count = 2;
        analysis.purity_distribution = None;
        analysis.trait_method_summary = None;

        let confidence = calculate_role_confidence(&analysis);
        // Small struct defaults to PureLogic with moderate confidence (65-70%)
        assert!(
            (0.65..0.70).contains(&confidence),
            "Small struct should have moderate confidence: {:.2}%",
            confidence * 100.0
        );
    }

    #[test]
    fn test_confidence_trait_dominated() {
        use crate::organization::god_object::TraitMethodSummary;

        let mut analysis = create_test_god_analysis();
        analysis.method_count = 30;
        analysis.purity_distribution = None;
        // 80% trait-mandated
        analysis.trait_method_summary = Some(TraitMethodSummary {
            mandated_count: 24,
            by_trait: [("syn::Visit".into(), 24)].into_iter().collect(),
            weighted_count: 8.4,
            extractable_count: 6,
            total_methods: 30,
        });

        let confidence = calculate_role_confidence(&analysis);
        // Trait-dominated structs have high confidence (85-95%)
        assert!(
            confidence >= 0.85,
            "Trait-dominated struct should have high confidence: {:.2}%",
            confidence * 100.0
        );
    }
}
