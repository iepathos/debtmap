//! Page availability predicates for detail view.
//!
//! This module contains pure functions for determining which detail
//! pages are available based on item data. Extracted from ResultsApp
//! following the single responsibility principle.
//!
//! # Design
//!
//! All functions are pure predicates - they take item data and return
//! a boolean indicating whether specific data is present.

use crate::data_flow::DataFlowGraph;
use crate::priority::{call_graph::FunctionId, UnifiedDebtItem};

use super::detail_page::DetailPage;

/// Check if item has git context data.
///
/// Pure predicate - returns true if contextual risk info is present.
pub fn has_git_context(item: &UnifiedDebtItem) -> bool {
    item.contextual_risk.is_some()
}

/// Check if item has any pattern-related data.
///
/// This is a compound predicate that checks for various pattern indicators:
/// - Pattern analysis results
/// - Detected pattern classification
/// - Purity analysis
/// - Language-specific features
/// - Entropy details
/// - Error swallowing patterns
/// - God object indicators with aggregated data
///
/// Pure predicate - depends only on the item data.
pub fn has_pattern_data(item: &UnifiedDebtItem, data_flow: &DataFlowGraph) -> bool {
    has_direct_pattern_data(item)
        || has_purity_info(item, data_flow)
        || has_god_object_pattern_data(item)
}

/// Check for direct pattern analysis data on the item.
fn has_direct_pattern_data(item: &UnifiedDebtItem) -> bool {
    item.pattern_analysis.is_some()
        || item.detected_pattern.is_some()
        || item.is_pure.is_some()
        || item.language_specific.is_some()
        || item.entropy_details.is_some()
        || item.error_swallowing_count.is_some()
        || item.error_swallowing_patterns.is_some()
}

/// Check for purity info in the data flow graph.
fn has_purity_info(item: &UnifiedDebtItem, data_flow: &DataFlowGraph) -> bool {
    let func_id = make_function_id(item);
    data_flow.get_purity_info(&func_id).is_some()
}

/// Check for god object pattern data with aggregated info.
fn has_god_object_pattern_data(item: &UnifiedDebtItem) -> bool {
    item.god_object_indicators
        .as_ref()
        .map(|god| {
            god.is_god_object
                && (god.aggregated_entropy.is_some()
                    || god.aggregated_error_swallowing_count.is_some()
                    || god
                        .aggregated_error_swallowing_patterns
                        .as_ref()
                        .map(|p| !p.is_empty())
                        .unwrap_or(false))
        })
        .unwrap_or(false)
}

/// Check if item has data flow analysis data.
///
/// Pure predicate - checks for purity info, mutation info, or IO operations
/// in the data flow graph.
pub fn has_data_flow_data(item: &UnifiedDebtItem, data_flow: &DataFlowGraph) -> bool {
    let func_id = make_function_id(item);

    data_flow.get_purity_info(&func_id).is_some()
        || data_flow.get_mutation_info(&func_id).is_some()
        || data_flow.get_io_operations(&func_id).is_some()
}

/// Create a FunctionId from a UnifiedDebtItem.
fn make_function_id(item: &UnifiedDebtItem) -> FunctionId {
    FunctionId::new(
        item.location.file.clone(),
        item.location.function.clone(),
        item.location.line,
    )
}

/// Get available detail pages for an item.
///
/// Returns a vector of pages that have relevant data for the given item.
/// Overview, Score Breakdown, and Dependencies are always available;
/// Responsibilities is always available at the end.
/// Git Context, Patterns, and Data Flow are conditional.
pub fn available_pages(
    item: Option<&UnifiedDebtItem>,
    data_flow: &DataFlowGraph,
) -> Vec<DetailPage> {
    let mut pages = vec![
        DetailPage::Overview,
        DetailPage::ScoreBreakdown,
        DetailPage::Dependencies,
    ];

    if let Some(item) = item {
        if has_git_context(item) {
            pages.push(DetailPage::GitContext);
        }

        if has_pattern_data(item, data_flow) {
            pages.push(DetailPage::Patterns);
        }

        if has_data_flow_data(item, data_flow) {
            pages.push(DetailPage::DataFlow);
        }
    }

    pages.push(DetailPage::Responsibilities);
    pages
}

/// Check if a specific page is available for the item.
pub fn is_page_available(
    page: DetailPage,
    item: Option<&UnifiedDebtItem>,
    data_flow: &DataFlowGraph,
) -> bool {
    available_pages(item, data_flow).contains(&page)
}

/// Get the next available page, wrapping around.
pub fn next_available_page(
    current: DetailPage,
    item: Option<&UnifiedDebtItem>,
    data_flow: &DataFlowGraph,
) -> DetailPage {
    let available = available_pages(item, data_flow);
    if available.is_empty() {
        return DetailPage::Overview;
    }

    let current_idx = available.iter().position(|&p| p == current).unwrap_or(0);
    let next_idx = (current_idx + 1) % available.len();
    available[next_idx]
}

/// Get the previous available page, wrapping around.
pub fn prev_available_page(
    current: DetailPage,
    item: Option<&UnifiedDebtItem>,
    data_flow: &DataFlowGraph,
) -> DetailPage {
    let available = available_pages(item, data_flow);
    if available.is_empty() {
        return DetailPage::Overview;
    }

    let current_idx = available.iter().position(|&p| p == current).unwrap_or(0);
    let prev_idx = if current_idx == 0 {
        available.len() - 1
    } else {
        current_idx - 1
    };
    available[prev_idx]
}

/// Ensure a page is valid for the current item.
///
/// Returns the page if available, or Overview as fallback.
pub fn ensure_valid_page(
    page: DetailPage,
    item: Option<&UnifiedDebtItem>,
    data_flow: &DataFlowGraph,
) -> DetailPage {
    if is_page_available(page, item, data_flow) {
        page
    } else {
        DetailPage::Overview
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        unified_scorer::Location, ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics,
        UnifiedScore,
    };
    use std::path::PathBuf;

    fn create_test_item() -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_fn".to_string(),
                line: 1,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 5,
                cognitive: 10,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 5.0,
                role_multiplier: 1.0,
                final_score: 50.0,
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
                primary_action: "Test".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 0.0,
                coverage_improvement: 0.0,
                lines_reduction: 0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 10,
            cyclomatic_complexity: 5,
            cognitive_complexity: 10,
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

    fn create_empty_data_flow() -> DataFlowGraph {
        DataFlowGraph::new()
    }

    #[test]
    fn test_has_git_context_none() {
        let item = create_test_item();
        assert!(!has_git_context(&item));
    }

    #[test]
    fn test_has_git_context_some() {
        let mut item = create_test_item();
        item.contextual_risk = Some(crate::risk::context::ContextualRisk {
            base_risk: 0.5,
            contextual_risk: 0.7,
            contexts: vec![],
            explanation: "Test context".to_string(),
        });
        assert!(has_git_context(&item));
    }

    #[test]
    fn test_has_direct_pattern_data_none() {
        let item = create_test_item();
        assert!(!has_direct_pattern_data(&item));
    }

    #[test]
    fn test_has_direct_pattern_data_is_pure() {
        let mut item = create_test_item();
        item.is_pure = Some(true);
        assert!(has_direct_pattern_data(&item));
    }

    #[test]
    fn test_has_pattern_data_basic() {
        let item = create_test_item();
        let data_flow = create_empty_data_flow();
        assert!(!has_pattern_data(&item, &data_flow));
    }

    #[test]
    fn test_has_data_flow_data_none() {
        let item = create_test_item();
        let data_flow = create_empty_data_flow();
        assert!(!has_data_flow_data(&item, &data_flow));
    }

    #[test]
    fn test_available_pages_minimal() {
        let item = create_test_item();
        let data_flow = create_empty_data_flow();
        let pages = available_pages(Some(&item), &data_flow);

        // Minimal: Overview, ScoreBreakdown, Dependencies, Responsibilities
        assert_eq!(pages.len(), 4);
        assert_eq!(pages[0], DetailPage::Overview);
        assert_eq!(pages[1], DetailPage::ScoreBreakdown);
        assert_eq!(pages[2], DetailPage::Dependencies);
        assert_eq!(pages[3], DetailPage::Responsibilities);
    }

    #[test]
    fn test_available_pages_with_git_context() {
        let mut item = create_test_item();
        item.contextual_risk = Some(crate::risk::context::ContextualRisk {
            base_risk: 0.5,
            contextual_risk: 0.7,
            contexts: vec![],
            explanation: "Test context".to_string(),
        });
        let data_flow = create_empty_data_flow();
        let pages = available_pages(Some(&item), &data_flow);

        // Should have Git Context
        assert!(pages.contains(&DetailPage::GitContext));
    }

    #[test]
    fn test_available_pages_none_item() {
        let data_flow = create_empty_data_flow();
        let pages = available_pages(None, &data_flow);

        // Only Overview, ScoreBreakdown, Dependencies, Responsibilities
        assert_eq!(pages.len(), 4);
    }

    #[test]
    fn test_is_page_available_overview() {
        let item = create_test_item();
        let data_flow = create_empty_data_flow();
        assert!(is_page_available(
            DetailPage::Overview,
            Some(&item),
            &data_flow
        ));
    }

    #[test]
    fn test_is_page_available_patterns_false() {
        let item = create_test_item();
        let data_flow = create_empty_data_flow();
        assert!(!is_page_available(
            DetailPage::Patterns,
            Some(&item),
            &data_flow
        ));
    }

    #[test]
    fn test_next_available_page_wraps() {
        let item = create_test_item();
        let data_flow = create_empty_data_flow();

        // From Responsibilities (last) should go to Overview (first)
        let next = next_available_page(DetailPage::Responsibilities, Some(&item), &data_flow);
        assert_eq!(next, DetailPage::Overview);
    }

    #[test]
    fn test_prev_available_page_wraps() {
        let item = create_test_item();
        let data_flow = create_empty_data_flow();

        // From Overview (first) should go to Responsibilities (last)
        let prev = prev_available_page(DetailPage::Overview, Some(&item), &data_flow);
        assert_eq!(prev, DetailPage::Responsibilities);
    }

    #[test]
    fn test_ensure_valid_page_valid() {
        let item = create_test_item();
        let data_flow = create_empty_data_flow();

        let result = ensure_valid_page(DetailPage::Overview, Some(&item), &data_flow);
        assert_eq!(result, DetailPage::Overview);
    }

    #[test]
    fn test_ensure_valid_page_invalid_falls_back() {
        let item = create_test_item();
        let data_flow = create_empty_data_flow();

        // Patterns not available without pattern data
        let result = ensure_valid_page(DetailPage::Patterns, Some(&item), &data_flow);
        assert_eq!(result, DetailPage::Overview);
    }
}
