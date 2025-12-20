//! Pure aggregation functions for god object metrics.
//!
//! This module provides composable functions to aggregate metrics from
//! member functions into god object-level metrics.
//!
//! # Aggregation Strategies
//!
//! - **Complexity**: SUM of all functions (total burden)
//! - **Coverage**: Weighted average by function length
//! - **Dependencies**: Aggregated from ALL raw FunctionMetrics (complete architectural view)
//! - **Contextual Risk**: Average across member functions
//!
//! ## Dependency Aggregation
//!
//! Dependencies are aggregated from raw FunctionMetrics to provide a complete
//! architectural view of god object dependencies. This ensures that even if
//! individual functions don't exceed complexity thresholds, the god object
//! still shows all its cross-file dependencies for proper assessment.
//!
//! # Examples
//!
//! ```rust,ignore
//! let members = extract_member_functions(items.iter(), &file_path);
//! let metrics = aggregate_god_object_metrics(&members);
//!
//! assert!(metrics.total_cyclomatic > 0);
//! assert!(metrics.weighted_coverage.is_some());
//! ```

use crate::complexity::entropy_core::{EntropyConfig, EntropyScore, UniversalEntropyCalculator};
use crate::core::FunctionMetrics;
use crate::priority::unified_scorer::EntropyDetails;
use crate::priority::{TransitiveCoverage, UnifiedDebtItem};
use crate::risk::context::ContextualRisk;
use crate::risk::lcov::LcovData;
use std::collections::HashSet;
use std::path::Path;

/// Aggregated metrics from member functions.
#[derive(Debug, Clone)]
pub struct GodObjectAggregatedMetrics {
    pub total_cyclomatic: u32,
    pub total_cognitive: u32,
    pub max_nesting_depth: u32,
    pub weighted_coverage: Option<TransitiveCoverage>,
    pub unique_upstream_callers: Vec<String>,
    pub unique_downstream_callees: Vec<String>,
    pub upstream_dependencies: usize,
    pub downstream_dependencies: usize,
    pub aggregated_contextual_risk: Option<ContextualRisk>,
    /// Total count of error swallowing patterns across all functions
    pub total_error_swallowing_count: u32,
    /// Unique error swallowing pattern types found
    pub error_swallowing_patterns: Vec<String>,
    /// Aggregated entropy analysis from member functions
    pub aggregated_entropy: Option<EntropyDetails>,
}

/// Extract member functions for a file.
///
/// Pure function that filters items by file path.
#[inline]
pub fn extract_member_functions<'a>(
    items: impl Iterator<Item = &'a UnifiedDebtItem>,
    file_path: &Path,
) -> Vec<&'a UnifiedDebtItem> {
    items
        .filter(|item| item.location.file == file_path)
        .collect()
}

/// Aggregate complexity: sum cyclomatic/cognitive, max nesting.
pub fn aggregate_complexity_metrics(members: &[&UnifiedDebtItem]) -> (u32, u32, u32) {
    let total_cyclomatic = members.iter().map(|m| m.cyclomatic_complexity).sum();
    let total_cognitive = members.iter().map(|m| m.cognitive_complexity).sum();
    let max_nesting = members.iter().map(|m| m.nesting_depth).max().unwrap_or(0);

    (total_cyclomatic, total_cognitive, max_nesting)
}

/// Aggregate coverage: weighted average by function length.
pub fn aggregate_coverage_metrics(members: &[&UnifiedDebtItem]) -> Option<TransitiveCoverage> {
    let coverages: Vec<_> = members
        .iter()
        .filter_map(|m| {
            m.transitive_coverage
                .as_ref()
                .map(|c| (c, m.function_length))
        })
        .collect();

    if coverages.is_empty() {
        return None;
    }

    let total_length: usize = coverages.iter().map(|(_, len)| len).sum();
    if total_length == 0 {
        return None;
    }

    let weighted_direct = coverages
        .iter()
        .map(|(cov, len)| cov.direct * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    let weighted_transitive = coverages
        .iter()
        .map(|(cov, len)| cov.transitive * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Collect all unique uncovered lines
    let uncovered_lines: Vec<usize> = coverages
        .iter()
        .flat_map(|(cov, _)| &cov.uncovered_lines)
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // Collect all unique propagated_from function IDs
    let propagated_from = coverages
        .iter()
        .flat_map(|(cov, _)| &cov.propagated_from)
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    Some(TransitiveCoverage {
        direct: weighted_direct,
        transitive: weighted_transitive,
        propagated_from,
        uncovered_lines,
    })
}

/// Aggregate dependencies: unique set deduplication.
pub fn aggregate_dependency_metrics(
    members: &[&UnifiedDebtItem],
) -> (Vec<String>, Vec<String>, usize, usize) {
    let mut unique_callers: HashSet<String> = HashSet::new();
    let mut unique_callees: HashSet<String> = HashSet::new();

    for item in members {
        unique_callers.extend(item.upstream_callers.iter().cloned());
        unique_callees.extend(item.downstream_callees.iter().cloned());
    }

    let upstream_count = unique_callers.len();
    let downstream_count = unique_callees.len();

    (
        unique_callers.into_iter().collect(),
        unique_callees.into_iter().collect(),
        upstream_count,
        downstream_count,
    )
}

/// Aggregate contextual risk: combine base and contextual risk from members.
pub fn aggregate_contextual_risk(members: &[&UnifiedDebtItem]) -> Option<ContextualRisk> {
    let risks: Vec<_> = members
        .iter()
        .filter_map(|m| m.contextual_risk.as_ref())
        .collect();

    if risks.is_empty() {
        return None;
    }

    // Average base risk
    let avg_base_risk = risks.iter().map(|r| r.base_risk).sum::<f64>() / risks.len() as f64;

    // Average contextual risk
    let avg_contextual_risk =
        risks.iter().map(|r| r.contextual_risk).sum::<f64>() / risks.len() as f64;

    // Collect all unique contexts
    let all_contexts: Vec<_> = risks.iter().flat_map(|r| &r.contexts).cloned().collect();

    let explanation = format!(
        "Aggregated from {} functions (avg base: {:.1}, avg contextual: {:.1})",
        risks.len(),
        avg_base_risk,
        avg_contextual_risk
    );

    Some(ContextualRisk {
        base_risk: avg_base_risk,
        contextual_risk: avg_contextual_risk,
        contexts: all_contexts,
        explanation,
    })
}

/// Aggregate error swallowing metrics from FunctionMetrics.
pub fn aggregate_error_swallowing(functions: &[FunctionMetrics]) -> (u32, Vec<String>) {
    let total_count = functions
        .iter()
        .filter_map(|f| f.error_swallowing_count)
        .sum();

    let mut unique_patterns: HashSet<String> = HashSet::new();
    for func in functions {
        if let Some(ref patterns) = func.error_swallowing_patterns {
            unique_patterns.extend(patterns.iter().cloned());
        }
    }

    (total_count, unique_patterns.into_iter().collect())
}

/// Aggregate dependency metrics from raw FunctionMetrics.
///
/// This provides a complete architectural view of dependencies by aggregating
/// from ALL functions in the file, not just those that became debt items.
/// This ensures god objects show their true blast radius.
pub fn aggregate_dependency_metrics_from_raw(
    functions: &[FunctionMetrics],
) -> (Vec<String>, Vec<String>, usize, usize) {
    let mut unique_callers: HashSet<String> = HashSet::new();
    let mut unique_callees: HashSet<String> = HashSet::new();

    for func in functions {
        if let Some(ref callers) = func.upstream_callers {
            unique_callers.extend(callers.iter().cloned());
        }
        if let Some(ref callees) = func.downstream_callees {
            unique_callees.extend(callees.iter().cloned());
        }
    }

    let upstream_count = unique_callers.len();
    let downstream_count = unique_callees.len();

    (
        unique_callers.into_iter().collect(),
        unique_callees.into_iter().collect(),
        upstream_count,
        downstream_count,
    )
}

/// Aggregate entropy analysis from member UnifiedDebtItems.
///
/// Returns weighted average entropy metrics based on function length.
/// Uses original (undampened) complexity values for the aggregate summary.
pub fn aggregate_entropy_metrics(members: &[&UnifiedDebtItem]) -> Option<EntropyDetails> {
    let entropy_data: Vec<_> = members
        .iter()
        .filter_map(|m| m.entropy_details.as_ref().map(|e| (e, m.function_length)))
        .collect();

    if entropy_data.is_empty() {
        return None;
    }

    let total_length: usize = entropy_data.iter().map(|(_, len)| len).sum();
    if total_length == 0 {
        return None;
    }

    // Weighted average of entropy scores
    let weighted_entropy = entropy_data
        .iter()
        .map(|(e, len)| e.entropy_score * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Weighted average of pattern repetition
    let weighted_repetition = entropy_data
        .iter()
        .map(|(e, len)| e.pattern_repetition * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Weighted average of dampening factor
    let weighted_dampening = entropy_data
        .iter()
        .map(|(e, len)| e.dampening_factor * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Sum original complexity across all members
    let total_original: u32 = entropy_data
        .iter()
        .map(|(e, _)| e.original_complexity)
        .sum();
    let total_adjusted: u32 = entropy_data.iter().map(|(e, _)| e.adjusted_cognitive).sum();

    Some(EntropyDetails {
        entropy_score: weighted_entropy,
        pattern_repetition: weighted_repetition,
        original_complexity: total_original,
        adjusted_complexity: total_adjusted,
        dampening_factor: weighted_dampening,
        adjusted_cognitive: total_adjusted,
    })
}

/// Aggregate all metrics (composition of above functions).
pub fn aggregate_god_object_metrics(members: &[&UnifiedDebtItem]) -> GodObjectAggregatedMetrics {
    let (total_cyc, total_cog, max_nest) = aggregate_complexity_metrics(members);
    let weighted_cov = aggregate_coverage_metrics(members);
    let (callers, callees, up_count, down_count) = aggregate_dependency_metrics(members);
    let contextual_risk = aggregate_contextual_risk(members);
    let entropy = aggregate_entropy_metrics(members);

    // Note: Error swallowing is aggregated from raw FunctionMetrics, not UnifiedDebtItem
    // This function sets defaults; use aggregate_from_raw_metrics for full error swallowing data
    GodObjectAggregatedMetrics {
        total_cyclomatic: total_cyc,
        total_cognitive: total_cog,
        max_nesting_depth: max_nest,
        weighted_coverage: weighted_cov,
        unique_upstream_callers: callers,
        unique_downstream_callees: callees,
        upstream_dependencies: up_count,
        downstream_dependencies: down_count,
        aggregated_contextual_risk: contextual_risk,
        total_error_swallowing_count: 0,
        error_swallowing_patterns: Vec::new(),
        aggregated_entropy: entropy,
    }
}

// =============================================================================
// Pure helper functions for entropy aggregation (Stillwater principles)
// =============================================================================

/// Extracts entropy data tuples from function metrics.
///
/// Pure function - filters functions that have entropy scores and returns
/// tuples of (entropy_score, length, cognitive_complexity).
fn extract_entropy_data(functions: &[FunctionMetrics]) -> Vec<(&EntropyScore, usize, u32)> {
    functions
        .iter()
        .filter_map(|f| f.entropy_score.as_ref().map(|e| (e, f.length, f.cognitive)))
        .collect()
}

/// Calculates weighted average of a metric from entropy data.
///
/// Pure function that computes a length-weighted average using the provided
/// extractor function.
fn weighted_average<F>(data: &[(&EntropyScore, usize, u32)], total_length: usize, f: F) -> f64
where
    F: Fn(&EntropyScore) -> f64,
{
    data.iter()
        .map(|(e, len, _)| f(e) * (*len as f64))
        .sum::<f64>()
        / total_length as f64
}

/// Sums a u32 field from entropy data tuples.
///
/// Pure function for aggregating cognitive complexity values.
fn sum_cognitive(data: &[(&EntropyScore, usize, u32)]) -> u32 {
    data.iter().map(|(_, _, cog)| cog).sum()
}

/// Calculates total length from entropy data tuples.
fn total_length(data: &[(&EntropyScore, usize, u32)]) -> usize {
    data.iter().map(|(_, len, _)| *len).sum()
}

/// Aggregate entropy from raw FunctionMetrics.
///
/// Returns weighted average entropy based on function length from ALL functions,
/// not just those that became debt items.
///
/// Composed from pure helper functions following Stillwater principles.
pub fn aggregate_entropy_from_raw(functions: &[FunctionMetrics]) -> Option<EntropyDetails> {
    let data = extract_entropy_data(functions);
    let len = total_length(&data);

    if data.is_empty() || len == 0 {
        return None;
    }

    let entropy = weighted_average(&data, len, |e| e.token_entropy);
    let repetition = weighted_average(&data, len, |e| e.pattern_repetition);
    let total_cognitive = sum_cognitive(&data);

    let calculator = UniversalEntropyCalculator::new(EntropyConfig::default());
    let dampening_factor = calculator.calculate_dampening_factor(entropy, repetition);
    let adjusted_cognitive = (total_cognitive as f64 * dampening_factor) as u32;

    Some(EntropyDetails {
        entropy_score: entropy,
        pattern_repetition: repetition,
        original_complexity: total_cognitive,
        adjusted_complexity: adjusted_cognitive,
        dampening_factor,
        adjusted_cognitive,
    })
}

/// Aggregate coverage from raw FunctionMetrics using LCOV data.
///
/// This function looks up coverage for ALL functions in the file from LCOV data,
/// not just those that became UnifiedDebtItems. This ensures god objects show
/// accurate coverage metrics even when member functions are filtered out by
/// complexity thresholds.
///
/// Returns a weighted average coverage based on function length.
pub fn aggregate_coverage_from_raw_metrics(
    functions: &[FunctionMetrics],
    coverage: &LcovData,
) -> Option<TransitiveCoverage> {
    if functions.is_empty() {
        return None;
    }

    // Collect coverage data for each function
    let mut coverage_data: Vec<(f64, usize, Vec<usize>)> = Vec::with_capacity(functions.len());

    for func in functions {
        let end_line = func.line + func.length.saturating_sub(1);
        // Use get_function_coverage_with_bounds for accurate AST-based matching
        let direct_coverage = coverage
            .get_function_coverage_with_bounds(&func.file, &func.name, func.line, end_line)
            .unwrap_or(0.0);

        let uncovered = coverage
            .get_function_uncovered_lines(&func.file, &func.name, func.line)
            .unwrap_or_default();

        coverage_data.push((direct_coverage, func.length, uncovered));
    }

    let total_length: usize = coverage_data.iter().map(|(_, len, _)| len).sum();
    if total_length == 0 {
        return None;
    }

    // Calculate weighted average coverage
    let weighted_direct = coverage_data
        .iter()
        .map(|(cov, len, _)| cov * (*len as f64))
        .sum::<f64>()
        / total_length as f64;

    // Collect all uncovered lines (deduplicated)
    let all_uncovered: Vec<usize> = coverage_data
        .iter()
        .flat_map(|(_, _, uncovered)| uncovered.iter().copied())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    Some(TransitiveCoverage {
        direct: weighted_direct,
        transitive: weighted_direct, // For god objects, transitive == direct
        propagated_from: vec![],
        uncovered_lines: all_uncovered,
    })
}

/// Aggregate metrics directly from raw FunctionMetrics (for ALL functions including tests).
///
/// This function aggregates complexity and dependencies from raw function metrics
/// before any filtering, ensuring god objects show:
/// - TRUE complexity of all their functions
/// - TRUE architectural dependencies (complete blast radius)
///
/// Note: Coverage is NOT aggregated here. Use `aggregate_coverage_from_raw_metrics`
/// separately with LCOV data for coverage metrics.
pub fn aggregate_from_raw_metrics(functions: &[FunctionMetrics]) -> GodObjectAggregatedMetrics {
    let total_cyclomatic = functions.iter().map(|f| f.cyclomatic).sum();
    let total_cognitive = functions.iter().map(|f| f.cognitive).sum();
    let max_nesting = functions.iter().map(|f| f.nesting).max().unwrap_or(0);

    // Aggregate error swallowing from raw metrics
    let (total_error_swallowing, error_patterns) = aggregate_error_swallowing(functions);

    // Aggregate entropy from raw metrics (available for all functions)
    let aggregated_entropy = aggregate_entropy_from_raw(functions);

    // Aggregate dependencies from raw metrics (complete architectural view)
    let (
        unique_upstream_callers,
        unique_downstream_callees,
        upstream_dependencies,
        downstream_dependencies,
    ) = aggregate_dependency_metrics_from_raw(functions);

    GodObjectAggregatedMetrics {
        total_cyclomatic,
        total_cognitive,
        max_nesting_depth: max_nesting,
        weighted_coverage: None,
        unique_upstream_callers,
        unique_downstream_callees,
        upstream_dependencies,
        downstream_dependencies,
        aggregated_contextual_risk: None,
        total_error_swallowing_count: total_error_swallowing,
        error_swallowing_patterns: error_patterns,
        aggregated_entropy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::priority::{
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedScore,
    };
    use std::path::PathBuf;

    fn create_test_item(
        file: &str,
        cyc: u32,
        cog: u32,
        nest: u32,
        length: usize,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from(file),
                function: "test_fn".to_string(),
                line: 1,
            },
            debt_type: DebtType::Complexity {
                cyclomatic: cyc,
                cognitive: cog,
            },
            unified_score: UnifiedScore {
                final_score: 50.0,
                complexity_factor: 5.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                base_score: None,
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
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            function_role: FunctionRole::Unknown,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: Vec::new(),
                related_items: Vec::new(),
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: Vec::new(),
            downstream_callees: Vec::new(),
            nesting_depth: nest,
            function_length: length,
            cyclomatic_complexity: cyc,
            cognitive_complexity: cog,
            entropy_details: None,
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
            file_context: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
            context_suggestion: None,
        }
    }

    #[test]
    fn test_complexity_aggregation_sums_and_maxes() {
        let members = vec![
            create_test_item("file.rs", 5, 10, 2, 50),
            create_test_item("file.rs", 10, 15, 5, 100),
            create_test_item("file.rs", 15, 20, 3, 75),
        ];
        let member_refs: Vec<_> = members.iter().collect();

        let (total_cyc, total_cog, max_nest) = aggregate_complexity_metrics(&member_refs);

        assert_eq!(total_cyc, 30); // 5 + 10 + 15
        assert_eq!(total_cog, 45); // 10 + 15 + 20
        assert_eq!(max_nest, 5); // max(2, 5, 3)
    }

    #[test]
    fn test_coverage_weighted_average() {
        let mut members = vec![
            create_test_item("file.rs", 0, 0, 0, 10),
            create_test_item("file.rs", 0, 0, 0, 50),
            create_test_item("file.rs", 0, 0, 0, 40),
        ];

        // Add coverage data
        members[0].transitive_coverage = Some(TransitiveCoverage {
            direct: 0.8,
            transitive: 0.9,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        members[1].transitive_coverage = Some(TransitiveCoverage {
            direct: 0.2,
            transitive: 0.3,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });
        members[2].transitive_coverage = Some(TransitiveCoverage {
            direct: 0.5,
            transitive: 0.6,
            propagated_from: vec![],
            uncovered_lines: vec![],
        });

        let member_refs: Vec<_> = members.iter().collect();
        let cov = aggregate_coverage_metrics(&member_refs).unwrap();

        // (10*0.8 + 50*0.2 + 40*0.5) / 100 = 38/100 = 0.38
        assert!((cov.direct - 0.38).abs() < 0.01);
        assert!((cov.transitive - 0.48).abs() < 0.01); // (10*0.9 + 50*0.3 + 40*0.6) / 100
    }

    #[test]
    fn test_dependencies_deduplicate() {
        let mut members = vec![
            create_test_item("file.rs", 0, 0, 0, 10),
            create_test_item("file.rs", 0, 0, 0, 20),
        ];

        members[0].upstream_callers = vec!["main".to_string(), "init".to_string()];
        members[0].downstream_callees = vec!["log".to_string()];
        members[1].upstream_callers = vec!["main".to_string(), "process".to_string()]; // "main" is duplicate
        members[1].downstream_callees = vec!["log".to_string(), "db".to_string()]; // "log" is duplicate

        let member_refs: Vec<_> = members.iter().collect();
        let (callers, _callees, up_count, down_count) = aggregate_dependency_metrics(&member_refs);

        assert_eq!(up_count, 3); // main, init, process (deduplicated)
        assert_eq!(down_count, 2); // log, db (deduplicated)
        assert!(callers.contains(&"main".to_string()));
        assert!(callers.contains(&"init".to_string()));
        assert!(callers.contains(&"process".to_string()));
    }

    #[test]
    fn test_extract_member_functions() {
        let items = vec![
            create_test_item("file1.rs", 5, 10, 2, 50),
            create_test_item("file2.rs", 10, 15, 3, 100),
            create_test_item("file1.rs", 15, 20, 4, 75),
        ];

        let members = extract_member_functions(items.iter(), Path::new("file1.rs"));

        assert_eq!(members.len(), 2);
        assert!(members
            .iter()
            .all(|m| m.location.file == Path::new("file1.rs")));
    }

    #[test]
    fn test_aggregate_god_object_metrics_composition() {
        let members = vec![
            create_test_item("file.rs", 5, 10, 2, 50),
            create_test_item("file.rs", 10, 15, 5, 100),
        ];
        let member_refs: Vec<_> = members.iter().collect();

        let metrics = aggregate_god_object_metrics(&member_refs);

        assert_eq!(metrics.total_cyclomatic, 15);
        assert_eq!(metrics.total_cognitive, 25);
        assert_eq!(metrics.max_nesting_depth, 5);
        assert!(metrics.weighted_coverage.is_none()); // No coverage data
    }

    #[test]
    fn test_aggregate_error_swallowing() {
        let functions = vec![
            FunctionMetrics {
                name: "func1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                cyclomatic: 5,
                cognitive: 5,
                nesting: 1,
                length: 20,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: Some(2),
                error_swallowing_patterns: Some(vec![
                    "if let Ok(...) without else branch".to_string()
                ]),
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "func2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 25,
                cyclomatic: 3,
                cognitive: 3,
                nesting: 1,
                length: 15,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: Some(3),
                error_swallowing_patterns: Some(vec![
                    "if let Ok(...) without else branch".to_string(),
                    "let _ = discarding Result".to_string(),
                ]),
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "func3".to_string(),
                file: PathBuf::from("test.rs"),
                line: 50,
                cyclomatic: 2,
                cognitive: 2,
                nesting: 1,
                length: 10,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None, // No error swallowing
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
        ];

        let (total, patterns) = aggregate_error_swallowing(&functions);

        assert_eq!(total, 5); // 2 + 3 = 5
        assert_eq!(patterns.len(), 2); // 2 unique patterns
        assert!(patterns.contains(&"if let Ok(...) without else branch".to_string()));
        assert!(patterns.contains(&"let _ = discarding Result".to_string()));
    }

    #[test]
    fn test_aggregate_entropy_metrics_weighted_average() {
        // Create items with different entropy details and lengths
        let mut item1 = create_test_item("file.rs", 10, 20, 2, 100); // length 100
        item1.entropy_details = Some(EntropyDetails {
            entropy_score: 0.4,
            pattern_repetition: 0.6,
            original_complexity: 20,
            adjusted_complexity: 16,
            dampening_factor: 0.8,
            adjusted_cognitive: 16,
        });

        let mut item2 = create_test_item("file.rs", 15, 30, 3, 200); // length 200
        item2.entropy_details = Some(EntropyDetails {
            entropy_score: 0.5,
            pattern_repetition: 0.3,
            original_complexity: 30,
            adjusted_complexity: 27,
            dampening_factor: 0.9,
            adjusted_cognitive: 27,
        });

        let members = vec![&item1, &item2];
        let result = aggregate_entropy_metrics(&members).expect("should have entropy");

        // Weighted average: (100*0.4 + 200*0.5) / 300 = 140/300 ≈ 0.467
        assert!((result.entropy_score - 0.467).abs() < 0.01);

        // Weighted repetition: (100*0.6 + 200*0.3) / 300 = 120/300 = 0.4
        assert!((result.pattern_repetition - 0.4).abs() < 0.01);

        // Weighted dampening: (100*0.8 + 200*0.9) / 300 = 260/300 ≈ 0.867
        assert!((result.dampening_factor - 0.867).abs() < 0.01);

        // Sums: 20 + 30 = 50, 16 + 27 = 43
        assert_eq!(result.original_complexity, 50);
        assert_eq!(result.adjusted_cognitive, 43);
    }

    #[test]
    fn test_aggregate_entropy_metrics_empty() {
        let item1 = create_test_item("file.rs", 10, 20, 2, 100);
        let item2 = create_test_item("file.rs", 15, 30, 3, 200);
        // Neither has entropy_details

        let members = vec![&item1, &item2];
        let result = aggregate_entropy_metrics(&members);

        assert!(result.is_none());
    }

    #[test]
    fn test_aggregate_entropy_metrics_partial() {
        // Only one item has entropy
        let mut item1 = create_test_item("file.rs", 10, 20, 2, 100);
        item1.entropy_details = Some(EntropyDetails {
            entropy_score: 0.4,
            pattern_repetition: 0.6,
            original_complexity: 20,
            adjusted_complexity: 16,
            dampening_factor: 0.8,
            adjusted_cognitive: 16,
        });

        let item2 = create_test_item("file.rs", 15, 30, 3, 200); // No entropy

        let members = vec![&item1, &item2];
        let result = aggregate_entropy_metrics(&members).expect("should have entropy from item1");

        // Only item1 contributes, so values are from item1 only
        assert!((result.entropy_score - 0.4).abs() < 0.001);
        assert_eq!(result.original_complexity, 20);
    }

    #[test]
    fn test_aggregate_god_object_metrics_includes_entropy() {
        let mut item1 = create_test_item("file.rs", 10, 20, 2, 100);
        item1.entropy_details = Some(EntropyDetails {
            entropy_score: 0.4,
            pattern_repetition: 0.6,
            original_complexity: 20,
            adjusted_complexity: 16,
            dampening_factor: 0.8,
            adjusted_cognitive: 16,
        });

        let members = vec![&item1];
        let metrics = aggregate_god_object_metrics(&members);

        assert!(metrics.aggregated_entropy.is_some());
        let entropy = metrics.aggregated_entropy.unwrap();
        assert!((entropy.entropy_score - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_aggregate_from_raw_metrics_includes_error_swallowing() {
        let functions = vec![FunctionMetrics {
            name: "func1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 10,
            cognitive: 15,
            nesting: 3,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: Some(4),
            error_swallowing_patterns: Some(vec!["match with ignored Err variant".to_string()]),
            entropy_analysis: None,
        }];

        let metrics = aggregate_from_raw_metrics(&functions);

        assert_eq!(metrics.total_cyclomatic, 10);
        assert_eq!(metrics.total_cognitive, 15);
        assert_eq!(metrics.total_error_swallowing_count, 4);
        assert_eq!(metrics.error_swallowing_patterns.len(), 1);
        assert!(metrics
            .error_swallowing_patterns
            .contains(&"match with ignored Err variant".to_string()));
    }

    #[test]
    fn test_aggregate_entropy_from_raw() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let functions = vec![
            FunctionMetrics {
                name: "func1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                cyclomatic: 10,
                cognitive: 20,
                nesting: 2,
                length: 100,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: Some(RawEntropyScore {
                    token_entropy: 0.4,
                    pattern_repetition: 0.6,
                    branch_similarity: 0.0,
                    effective_complexity: 5.0,
                    unique_variables: 0,
                    max_nesting: 0,
                    dampening_applied: 0.0,
                }),
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "func2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 50,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 1,
                length: 50,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: Some(RawEntropyScore {
                    token_entropy: 0.5,
                    pattern_repetition: 0.3,
                    branch_similarity: 0.0,
                    effective_complexity: 3.0,
                    unique_variables: 0,
                    max_nesting: 0,
                    dampening_applied: 0.0,
                }),
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
        ];

        let result = aggregate_entropy_from_raw(&functions).expect("should have entropy");

        // Weighted average: (100*0.4 + 50*0.5) / 150 = 65/150 ≈ 0.433
        assert!((result.entropy_score - 0.433).abs() < 0.01);

        // Weighted repetition: (100*0.6 + 50*0.3) / 150 = 75/150 = 0.5
        assert!((result.pattern_repetition - 0.5).abs() < 0.01);

        // Original complexity: 20 + 10 = 30
        assert_eq!(result.original_complexity, 30);
    }

    #[test]
    fn test_aggregate_from_raw_metrics_includes_entropy() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let functions = vec![FunctionMetrics {
            name: "func1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 10,
            cognitive: 20,
            nesting: 2,
            length: 100,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: Some(RawEntropyScore {
                token_entropy: 0.4,
                pattern_repetition: 0.6,
                branch_similarity: 0.0,
                effective_complexity: 5.0,
                unique_variables: 0,
                max_nesting: 0,
                dampening_applied: 0.0,
            }),
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }];

        let metrics = aggregate_from_raw_metrics(&functions);

        assert!(metrics.aggregated_entropy.is_some());
        let entropy = metrics.aggregated_entropy.unwrap();
        assert!((entropy.entropy_score - 0.4).abs() < 0.001);
        assert_eq!(entropy.original_complexity, 20);
    }

    // =========================================================================
    // Tests for pure helper functions (Stillwater refactoring)
    // =========================================================================

    #[test]
    fn test_extract_entropy_data_filters_correctly() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let functions = vec![
            FunctionMetrics {
                name: "with_entropy".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 1,
                length: 50,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: Some(RawEntropyScore {
                    token_entropy: 0.4,
                    pattern_repetition: 0.6,
                    branch_similarity: 0.0,
                    effective_complexity: 5.0,
                    unique_variables: 0,
                    max_nesting: 0,
                    dampening_applied: 0.0,
                }),
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "without_entropy".to_string(),
                file: PathBuf::from("test.rs"),
                line: 50,
                cyclomatic: 3,
                cognitive: 5,
                nesting: 1,
                length: 25,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None, // No entropy
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
        ];

        let data = extract_entropy_data(&functions);
        assert_eq!(data.len(), 1); // Only one function has entropy
        assert_eq!(data[0].1, 50); // Length of function with entropy
        assert_eq!(data[0].2, 10); // Cognitive of function with entropy
    }

    #[test]
    fn test_weighted_average_calculation() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let e1 = RawEntropyScore {
            token_entropy: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.0,
            effective_complexity: 5.0,
            unique_variables: 0,
            max_nesting: 0,
            dampening_applied: 0.0,
        };
        let e2 = RawEntropyScore {
            token_entropy: 0.6,
            pattern_repetition: 0.2,
            branch_similarity: 0.0,
            effective_complexity: 3.0,
            unique_variables: 0,
            max_nesting: 0,
            dampening_applied: 0.0,
        };

        // length 100, cognitive 20 and length 50, cognitive 10
        let data: Vec<(&RawEntropyScore, usize, u32)> = vec![(&e1, 100, 20), (&e2, 50, 10)];

        // (100*0.4 + 50*0.6) / 150 = 70/150 ≈ 0.467
        let avg = weighted_average(&data, 150, |e| e.token_entropy);
        assert!((avg - 0.467).abs() < 0.01);

        // (100*0.6 + 50*0.2) / 150 = 70/150 ≈ 0.467
        let rep = weighted_average(&data, 150, |e| e.pattern_repetition);
        assert!((rep - 0.467).abs() < 0.01);
    }

    #[test]
    fn test_sum_cognitive() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let e = RawEntropyScore {
            token_entropy: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.0,
            effective_complexity: 5.0,
            unique_variables: 0,
            max_nesting: 0,
            dampening_applied: 0.0,
        };

        let data: Vec<(&RawEntropyScore, usize, u32)> =
            vec![(&e, 100, 20), (&e, 50, 15), (&e, 25, 5)];

        let total = sum_cognitive(&data);
        assert_eq!(total, 40); // 20 + 15 + 5
    }

    #[test]
    fn test_total_length() {
        use crate::complexity::entropy_core::EntropyScore as RawEntropyScore;

        let e = RawEntropyScore {
            token_entropy: 0.4,
            pattern_repetition: 0.6,
            branch_similarity: 0.0,
            effective_complexity: 5.0,
            unique_variables: 0,
            max_nesting: 0,
            dampening_applied: 0.0,
        };

        let data: Vec<(&RawEntropyScore, usize, u32)> = vec![(&e, 100, 20), (&e, 50, 15)];

        let total = total_length(&data);
        assert_eq!(total, 150); // 100 + 50
    }

    #[test]
    fn test_calculate_dampening_factor_direct() {
        use crate::complexity::entropy_core::{EntropyConfig, UniversalEntropyCalculator};

        let calculator = UniversalEntropyCalculator::new(EntropyConfig::default());

        // Test with various entropy/repetition combinations
        let dampening = calculator.calculate_dampening_factor(0.4, 0.6);
        assert!((0.5..=1.0).contains(&dampening));

        // Low entropy, high repetition should result in lower effective complexity
        let low_dampening = calculator.calculate_dampening_factor(0.2, 0.8);
        assert!(low_dampening >= 0.5);

        // High entropy, low repetition should result in higher effective complexity
        let high_dampening = calculator.calculate_dampening_factor(0.8, 0.2);
        assert!(high_dampening <= 1.0);
    }

    #[test]
    fn test_aggregate_dependency_metrics_from_raw() {
        let functions = vec![
            FunctionMetrics {
                name: "func1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 1,
                length: 50,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: Some(vec!["caller1".to_string(), "caller2".to_string()]),
                downstream_callees: Some(vec!["callee1".to_string()]),
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
            FunctionMetrics {
                name: "func2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 10,
                cyclomatic: 3,
                cognitive: 5,
                nesting: 1,
                length: 30,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: Some(vec!["caller2".to_string(), "caller3".to_string()]), // caller2 is duplicate
                downstream_callees: Some(vec!["callee2".to_string(), "callee3".to_string()]),
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
                error_swallowing_count: None,
                error_swallowing_patterns: None,
                entropy_analysis: None,
            },
        ];

        let (callers, callees, upstream_count, downstream_count) =
            aggregate_dependency_metrics_from_raw(&functions);

        // Should deduplicate: caller1, caller2, caller3 = 3 unique
        assert_eq!(upstream_count, 3);
        // Should deduplicate: callee1, callee2, callee3 = 3 unique
        assert_eq!(downstream_count, 3);

        assert!(callers.contains(&"caller1".to_string()));
        assert!(callers.contains(&"caller2".to_string()));
        assert!(callers.contains(&"caller3".to_string()));

        assert!(callees.contains(&"callee1".to_string()));
        assert!(callees.contains(&"callee2".to_string()));
        assert!(callees.contains(&"callee3".to_string()));
    }

    #[test]
    fn test_aggregate_from_raw_metrics_includes_dependencies() {
        let functions = vec![FunctionMetrics {
            name: "func1".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 5,
            cognitive: 10,
            nesting: 1,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: Some(vec!["caller1".to_string(), "caller2".to_string()]),
            downstream_callees: Some(vec!["callee1".to_string()]),
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }];

        let metrics = aggregate_from_raw_metrics(&functions);

        // Dependencies should be populated from raw metrics
        assert_eq!(metrics.upstream_dependencies, 2);
        assert_eq!(metrics.downstream_dependencies, 1);
        assert_eq!(metrics.unique_upstream_callers.len(), 2);
        assert_eq!(metrics.unique_downstream_callees.len(), 1);
    }
}
