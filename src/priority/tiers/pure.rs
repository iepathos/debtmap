/// Pure tier classification logic using composed predicates.
///
/// This module implements the "Pure Core, Imperative Shell" principle by composing
/// small, testable predicate functions into tier classification logic.
use super::predicates::*;
use super::RecommendationTier;
use crate::priority::{TierConfig, UnifiedDebtItem};

/// Pure tier classification using composed predicates.
///
/// This is a pure function that composes smaller predicates to determine tier.
/// No side effects, fully deterministic, easily testable.
pub fn classify_tier(item: &UnifiedDebtItem, config: &TierConfig) -> RecommendationTier {
    if is_t1_architectural(item, config) {
        RecommendationTier::T1CriticalArchitecture
    } else if is_t2_complex_untested(item, config) {
        RecommendationTier::T2ComplexUntested
    } else if is_t3_testing_gap(item, config) {
        RecommendationTier::T3TestingGaps
    } else {
        RecommendationTier::T4Maintenance
    }
}

/// Checks if item is T1 architectural issue (composed from predicates).
fn is_t1_architectural(item: &UnifiedDebtItem, _config: &TierConfig) -> bool {
    is_god_or_error_issue(&item.debt_type) || has_t1_complexity(item)
}

/// Checks if debt type is god object/module or error handling issue.
fn is_god_or_error_issue(debt_type: &crate::priority::DebtType) -> bool {
    is_god_object(debt_type) || is_error_handling_issue(debt_type)
}

/// Checks if metrics indicate T1 complexity level.
fn has_t1_complexity(item: &UnifiedDebtItem) -> bool {
    has_extreme_score(item.unified_score.final_score)
        || has_t1_cyclomatic_metric(item)
        || has_t1_cognitive_metric(item)
        || has_t1_other_metrics(item)
}

/// Checks if item has T1-level cyclomatic complexity.
fn has_t1_cyclomatic_metric(item: &UnifiedDebtItem) -> bool {
    extract_effective_cyclomatic(&item.debt_type)
        .map(has_extreme_cyclomatic)
        .unwrap_or(false)
}

/// Checks if item has T1-level cognitive complexity.
fn has_t1_cognitive_metric(item: &UnifiedDebtItem) -> bool {
    extract_cognitive(&item.debt_type)
        .map(has_extreme_cognitive)
        .unwrap_or(false)
}

/// Checks if item has other T1-level metrics (nesting, complexity factor).
fn has_t1_other_metrics(item: &UnifiedDebtItem) -> bool {
    has_deep_nesting(item.nesting_depth)
        || has_high_complexity_factor(item.unified_score.complexity_factor)
}

/// Checks if item is T2 complex untested (composed from predicates).
fn is_t2_complex_untested(item: &UnifiedDebtItem, config: &TierConfig) -> bool {
    is_t2_testing_gap(item, config) || is_t2_complexity_hotspot(item)
}

/// Checks if item is T2-level testing gap.
fn is_t2_testing_gap(item: &UnifiedDebtItem, config: &TierConfig) -> bool {
    if !is_testing_gap(&item.debt_type) {
        return false;
    }

    let total_deps = item.upstream_dependencies + item.downstream_dependencies;

    has_high_cyclomatic(item.cyclomatic_complexity, config.t2_complexity_threshold)
        || has_many_dependencies(total_deps, config.t2_dependency_threshold)
        || is_entry_point(&item.function_role)
}

/// Checks if item is T2-level complexity hotspot.
fn is_t2_complexity_hotspot(item: &UnifiedDebtItem) -> bool {
    is_complexity_hotspot(&item.debt_type)
        && (has_t2_meaningful_complexity(item) || has_t2_meaningful_adjusted(item))
}

/// Checks if item has meaningful complexity signals for T2.
fn has_t2_meaningful_complexity(item: &UnifiedDebtItem) -> bool {
    has_moderate_complexity_factor(item.unified_score.complexity_factor)
        || has_moderate_cognitive(item.cognitive_complexity)
        || has_moderate_nesting(item.nesting_depth)
}

/// Checks if item has meaningful adjusted cyclomatic for T2.
fn has_t2_meaningful_adjusted(item: &UnifiedDebtItem) -> bool {
    extract_effective_cyclomatic(&item.debt_type)
        .map(has_moderate_adjusted_cyclomatic)
        .unwrap_or(false)
}

/// Checks if item is T3 testing gap.
fn is_t3_testing_gap(item: &UnifiedDebtItem, config: &TierConfig) -> bool {
    is_testing_gap(&item.debt_type)
        && has_t3_cyclomatic(item.cyclomatic_complexity, config.t3_complexity_threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::priority::{
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedScore,
    };
    use proptest::prelude::*;

    fn create_test_item(
        debt_type: DebtType,
        cyclomatic: u32,
        cognitive: u32,
        nesting: u32,
        deps: usize,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: "test.rs".into(),
                function: "test_fn".into(),
                line: 1,
            },
            debt_type,
            unified_score: UnifiedScore {
                complexity_factor: 0.0,
                coverage_factor: 0.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
                final_score: 0.0,
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
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Test".into(),
                rationale: "Test".into(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.0,
                complexity_reduction: 0.0,
                coverage_improvement: 0.0,
                lines_reduction: 0,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: deps,
            downstream_dependencies: deps,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: nesting,
            function_length: 10,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            entropy_details: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            is_pure: Some(false),
            purity_confidence: Some(0.0),
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
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
    fn test_classify_god_object_as_t1() {
        let item = create_test_item(
            DebtType::GodObject {
                methods: 100,
                fields: Some(50),
                responsibilities: 5,
                god_object_score: 95.0,
                lines: 500,
            },
            10,
            10,
            2,
            5,
        );
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
    }

    #[test]
    fn test_classify_error_swallowing_as_t1() {
        let item = create_test_item(
            DebtType::ErrorSwallowing {
                pattern: "unwrap".into(),
                context: None,
            },
            10,
            10,
            2,
            5,
        );
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
    }

    #[test]
    fn test_classify_extreme_score_as_t1() {
        let mut item = create_test_item(
            DebtType::ComplexityHotspot {
                cyclomatic: 30,
                cognitive: 25,
            },
            30,
            25,
            3,
            5,
        );
        item.unified_score.final_score = 11.0; // Extreme score
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
    }

    #[test]
    fn test_classify_extreme_cyclomatic_as_t1() {
        let item = create_test_item(
            DebtType::ComplexityHotspot {
                cyclomatic: 70,
                cognitive: 15,
            },
            70,
            15,
            2,
            5,
        );
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
    }

    #[test]
    fn test_classify_extreme_cognitive_as_t1() {
        let item = create_test_item(
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
            15,
            25,
            2,
            5,
        );
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
    }

    #[test]
    fn test_classify_deep_nesting_as_t1() {
        let item = create_test_item(
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 10,
            },
            15,
            10,
            5, // Deep nesting
            5,
        );
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
    }

    #[test]
    fn test_classify_high_complexity_factor_as_t1() {
        let mut item = create_test_item(
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 10,
            },
            15,
            10,
            2,
            5,
        );
        item.unified_score.complexity_factor = 5.5; // High complexity factor
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
    }

    #[test]
    fn test_classify_high_complexity_testing_gap_as_t2() {
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 20,
                cognitive: 25,
            },
            20,
            25,
            2,
            5,
        );
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T2ComplexUntested);
    }

    #[test]
    fn test_classify_high_deps_testing_gap_as_t2() {
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 10,
            },
            10,
            10,
            2,
            10, // 10 upstream + 10 downstream = 20 total, exceeds threshold
        );
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T2ComplexUntested);
    }

    #[test]
    fn test_classify_entry_point_testing_gap_as_t2() {
        let mut item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 10,
            },
            10,
            10,
            2,
            5,
        );
        item.function_role = FunctionRole::EntryPoint;
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T2ComplexUntested);
    }

    #[test]
    fn test_classify_moderate_complexity_hotspot_as_t2() {
        let mut item = create_test_item(
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 12,
            },
            15,
            12,
            3,
            5,
        );
        item.unified_score.complexity_factor = 2.5; // Moderate complexity factor
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T2ComplexUntested);
    }

    #[test]
    fn test_classify_moderate_testing_gap_as_t3() {
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 12,
                cognitive: 14,
            },
            12,
            14,
            2,
            3, // 3 upstream + 3 downstream = 6 total, below t2_dependency_threshold (10)
        );
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T3TestingGaps);
    }

    #[test]
    fn test_classify_low_complexity_as_t4() {
        let item = create_test_item(
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 5,
                cognitive: 6,
            },
            5,
            6,
            1,
            2,
        );
        let tier = classify_tier(&item, &TierConfig::default());
        assert_eq!(tier, RecommendationTier::T4Maintenance);
    }

    // Property-based tests for classification invariants

    proptest! {
        #[test]
        fn prop_god_objects_always_t1(
            methods in 1u32..200,
            fields in 1u32..100,
            responsibilities in 1u32..20,
            god_score in 0.0f64..100.0,
            cyclomatic in 1u32..100,
            cognitive in 1u32..100,
            nesting in 1u32..10,
            deps in 0usize..50,
        ) {
            let item = create_test_item(
                DebtType::GodObject {
                    methods,
                    fields: Some(fields),
                    responsibilities,
                    god_object_score: god_score.max(0.0),
                    lines: methods * 10,
                },
                cyclomatic,
                cognitive,
                nesting,
                deps,
            );
            let tier = classify_tier(&item, &TierConfig::default());
            prop_assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
        }

        #[test]
        fn prop_error_handling_always_t1(
            cyclomatic in 1u32..100,
            cognitive in 1u32..100,
            nesting in 1u32..10,
            deps in 0usize..50,
        ) {
            let item = create_test_item(
                DebtType::ErrorSwallowing {
                    pattern: "unwrap".into(),
                    context: None,
                },
                cyclomatic,
                cognitive,
                nesting,
                deps,
            );
            let tier = classify_tier(&item, &TierConfig::default());
            prop_assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
        }

        #[test]
        fn prop_classification_is_deterministic(
            cyclomatic in 1u32..100,
            cognitive in 1u32..100,
            nesting in 1u32..10,
            deps in 0usize..50,
        ) {
            let item = create_test_item(
                DebtType::TestingGap {
                    coverage: 0.0,
                    cyclomatic,
                    cognitive,
                },
                cyclomatic,
                cognitive,
                nesting,
                deps,
            );
            let config = TierConfig::default();
            let tier1 = classify_tier(&item, &config);
            let tier2 = classify_tier(&item, &config);
            prop_assert_eq!(tier1, tier2);
        }

        #[test]
        fn prop_extreme_score_triggers_t1(
            cyclomatic in 1u32..100,
            cognitive in 1u32..100,
            nesting in 1u32..4, // Keep nesting below T1 threshold (5)
            deps in 0usize..50,
        ) {
            let mut item = create_test_item(
                DebtType::ComplexityHotspot {
                    cyclomatic,
                    cognitive,
                },
                cyclomatic,
                cognitive,
                nesting,
                deps,
            );
            // Set extreme score (> 10.0)
            item.unified_score.final_score = 11.0;
            // Keep complexity_factor below threshold
            item.unified_score.complexity_factor = 4.0;

            let tier = classify_tier(&item, &TierConfig::default());
            prop_assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
        }

        #[test]
        fn prop_deep_nesting_triggers_t1(
            cyclomatic in 1u32..50, // Keep below extreme cyclomatic (51)
            cognitive in 1u32..19, // Keep below extreme cognitive (20)
            deps in 0usize..50,
        ) {
            let mut item = create_test_item(
                DebtType::ComplexityHotspot {
                    cyclomatic,
                    cognitive,
                },
                cyclomatic,
                cognitive,
                5, // Deep nesting (>= 5)
                deps,
            );
            // Keep score below extreme threshold
            item.unified_score.final_score = 8.0;
            // Keep complexity_factor below threshold
            item.unified_score.complexity_factor = 4.0;

            let tier = classify_tier(&item, &TierConfig::default());
            prop_assert_eq!(tier, RecommendationTier::T1CriticalArchitecture);
        }

        #[test]
        fn prop_tier_ordering_respected(
            cyclomatic in 1u32..100,
            cognitive in 1u32..100,
            nesting in 1u32..10,
            deps in 0usize..50,
        ) {
            let item = create_test_item(
                DebtType::TestingGap {
                    coverage: 0.0,
                    cyclomatic,
                    cognitive,
                },
                cyclomatic,
                cognitive,
                nesting,
                deps,
            );
            let tier = classify_tier(&item, &TierConfig::default());
            // Tier should always be one of the valid tiers
            prop_assert!(matches!(
                tier,
                RecommendationTier::T1CriticalArchitecture
                    | RecommendationTier::T2ComplexUntested
                    | RecommendationTier::T3TestingGaps
                    | RecommendationTier::T4Maintenance
            ));
        }

        #[test]
        fn prop_low_complexity_never_t1(
            deps in 0usize..9, // Keep below T2 dependency threshold (10)
        ) {
            // Create item with low complexity across all metrics
            let mut item = create_test_item(
                DebtType::TestingGap {
                    coverage: 0.0,
                    cyclomatic: 5,
                    cognitive: 5,
                },
                5,  // Low cyclomatic
                5,  // Low cognitive
                2,  // Low nesting (< 5)
                deps,
            );
            // Low score
            item.unified_score.final_score = 2.0;
            // Low complexity factor
            item.unified_score.complexity_factor = 1.0;

            let tier = classify_tier(&item, &TierConfig::default());
            // Should never be T1 with all low metrics
            prop_assert_ne!(tier, RecommendationTier::T1CriticalArchitecture);
        }
    }
}
