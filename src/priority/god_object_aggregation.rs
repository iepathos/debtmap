//! Pure aggregation functions for god object metrics.
//!
//! This module provides composable functions to aggregate metrics from
//! member functions into god object-level metrics.
//!
//! # Aggregation Strategies
//!
//! - **Complexity**: SUM of all functions (total burden)
//! - **Coverage**: Weighted average by function length
//! - **Dependencies**: Deduplicated from UnifiedDebtItems only (debt-focused)
//! - **Contextual Risk**: Average across member functions
//!
//! ## Dependency Aggregation Note
//!
//! Dependencies aggregate only from functions that became UnifiedDebtItems
//! (passed complexity/coverage thresholds). This provides a debt-focused view
//! of dependencies rather than complete architectural dependencies. Simple
//! functions (getters, setters) are filtered out and don't contribute.
//!
//! This means god objects may show zero dependencies if all their functions
//! are too simple to become debt items, which is working as intended.
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

use crate::core::FunctionMetrics;
use crate::priority::{TransitiveCoverage, UnifiedDebtItem};
use crate::risk::context::ContextualRisk;
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

/// Aggregate all metrics (composition of above functions).
pub fn aggregate_god_object_metrics(members: &[&UnifiedDebtItem]) -> GodObjectAggregatedMetrics {
    let (total_cyc, total_cog, max_nest) = aggregate_complexity_metrics(members);
    let weighted_cov = aggregate_coverage_metrics(members);
    let (callers, callees, up_count, down_count) = aggregate_dependency_metrics(members);
    let contextual_risk = aggregate_contextual_risk(members);

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
    }
}


/// Aggregate metrics directly from raw FunctionMetrics (for ALL functions including tests).
///
/// This function aggregates complexity from raw function metrics before any filtering,
/// ensuring god objects show the TRUE complexity of all their functions.
pub fn aggregate_from_raw_metrics(functions: &[FunctionMetrics]) -> GodObjectAggregatedMetrics {
    let total_cyclomatic = functions.iter().map(|f| f.cyclomatic).sum();
    let total_cognitive = functions.iter().map(|f| f.cognitive).sum();
    let max_nesting = functions.iter().map(|f| f.nesting).max().unwrap_or(0);

    // No coverage or dependency data available from raw metrics
    // These will need to come from unified items if available
    GodObjectAggregatedMetrics {
        total_cyclomatic,
        total_cognitive,
        max_nesting_depth: max_nesting,
        weighted_coverage: None,
        unique_upstream_callers: Vec::new(),
        unique_downstream_callees: Vec::new(),
        upstream_dependencies: 0,
        downstream_dependencies: 0,
        aggregated_contextual_risk: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::score_types::Score0To100;
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
                final_score: Score0To100::new(50.0),
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
            entropy_adjusted_cyclomatic: None,
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
}
