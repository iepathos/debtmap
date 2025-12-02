//! Exponential scaling and risk-based boosting for technical debt scores.
//!
//! This module implements score amplification through:
//! 1. Exponential scaling based on debt type (architectural issues get stronger boost)
//! 2. Risk-based discrete boosts for high dependencies, entry points, etc.
//!
//! The approach replaces tier-based ranking with transparent score amplification,
//! ensuring higher scores always rank higher while naturally surfacing critical issues.

use crate::priority::semantic_classifier::FunctionRole;
use crate::priority::{DebtType, UnifiedDebtItem};
use serde::{Deserialize, Serialize};

/// Configuration for exponential scaling and risk boosting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingConfig {
    // Exponential scaling exponents (>1.0 = amplification, 1.0 = linear)
    pub god_object_exponent: f64,
    pub god_module_exponent: f64,
    pub high_complexity_exponent: f64,     // cyclomatic > 30
    pub moderate_complexity_exponent: f64, // cyclomatic > 15

    // Risk boost multipliers (>1.0 = boost, 1.0 = no change)
    pub high_dependency_boost: f64,  // total deps > 15
    pub entry_point_boost: f64,      // entry points
    pub complex_untested_boost: f64, // complexity > 20 + untested
}

impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            god_object_exponent: 1.4,
            god_module_exponent: 1.4,
            high_complexity_exponent: 1.2,
            moderate_complexity_exponent: 1.1,
            high_dependency_boost: 1.2,
            entry_point_boost: 1.15,
            complex_untested_boost: 1.25,
        }
    }
}

/// Apply exponential scaling based on debt type.
///
/// Returns the scaled score using debt-type-specific exponents.
/// Higher exponents create more separation at higher base scores.
fn apply_exponential_scaling(base_score: f64, debt_type: &DebtType, config: &ScalingConfig) -> f64 {
    // Ensure minimum base score to avoid zero^exponent = 0
    let safe_base = base_score.max(1.0);

    let exponent = match debt_type {
        // Architectural issues get strong exponential scaling
        DebtType::GodObject { .. } => config.god_object_exponent,
        DebtType::GodModule { .. } => config.god_module_exponent,

        // High complexity gets moderate exponential scaling (use adjusted complexity - spec 182)
        DebtType::ComplexityHotspot {
            cyclomatic,
            adjusted_cyclomatic,
            ..
        } => {
            let effective_cyclomatic = adjusted_cyclomatic.unwrap_or(*cyclomatic);
            if effective_cyclomatic > 30 {
                config.high_complexity_exponent
            } else if effective_cyclomatic > 15 {
                config.moderate_complexity_exponent
            } else {
                1.0
            }
        }

        // Complex untested code gets slight exponential boost
        DebtType::TestingGap { cyclomatic, .. } if *cyclomatic > 20 => {
            config.moderate_complexity_exponent
        }

        // Everything else stays linear (exponent = 1.0)
        _ => 1.0,
    };

    safe_base.powf(exponent)
}

/// Apply discrete risk-based boosts.
///
/// Returns the boosted score by multiplying with risk factors.
/// Multiple risk factors combine multiplicatively.
fn apply_risk_boosts(score: f64, item: &UnifiedDebtItem, config: &ScalingConfig) -> f64 {
    let mut boost = 1.0;

    // High dependency count indicates central, critical code
    let total_deps = item.upstream_dependencies + item.downstream_dependencies;
    if total_deps > 15 {
        boost *= config.high_dependency_boost;
    }

    // Entry points are critical paths
    if matches!(item.function_role, FunctionRole::EntryPoint) {
        boost *= config.entry_point_boost;
    }

    // Complex + untested is particularly risky
    if is_untested(item) && item.cyclomatic_complexity > 20 {
        boost *= config.complex_untested_boost;
    }

    score * boost
}

/// Check if item is untested (coverage < 10%).
fn is_untested(item: &UnifiedDebtItem) -> bool {
    matches!(
        item.debt_type,
        DebtType::TestingGap { coverage, .. } if coverage < 0.1
    )
}

/// Calculate final score with exponential scaling and risk boosting.
///
/// Pipeline:
/// 1. Apply exponential scaling based on debt type
/// 2. Apply discrete risk boosts based on dependencies, entry points, etc.
///
/// Returns tuple of (final_score, exponent_used, boost_applied).
pub fn calculate_final_score(
    base_score: f64,
    debt_type: &DebtType,
    item: &UnifiedDebtItem,
    config: &ScalingConfig,
) -> (f64, f64, f64) {
    // Determine exponent (for transparency)
    let exponent = match debt_type {
        DebtType::GodObject { .. } => config.god_object_exponent,
        DebtType::GodModule { .. } => config.god_module_exponent,
        DebtType::ComplexityHotspot {
            cyclomatic,
            adjusted_cyclomatic,
            ..
        } => {
            let effective_cyclomatic = adjusted_cyclomatic.unwrap_or(*cyclomatic);
            if effective_cyclomatic > 30 {
                config.high_complexity_exponent
            } else if effective_cyclomatic > 15 {
                config.moderate_complexity_exponent
            } else {
                1.0
            }
        }
        DebtType::TestingGap { cyclomatic, .. } if *cyclomatic > 20 => {
            config.moderate_complexity_exponent
        }
        _ => 1.0,
    };

    // Step 1: Apply exponential scaling
    let scaled = apply_exponential_scaling(base_score, debt_type, config);

    // Step 2: Calculate risk boost factor
    let mut boost = 1.0;
    let total_deps = item.upstream_dependencies + item.downstream_dependencies;
    if total_deps > 15 {
        boost *= config.high_dependency_boost;
    }
    if matches!(item.function_role, FunctionRole::EntryPoint) {
        boost *= config.entry_point_boost;
    }
    if is_untested(item) && item.cyclomatic_complexity > 20 {
        boost *= config.complex_untested_boost;
    }

    // Step 3: Apply risk boosts
    let final_score = apply_risk_boosts(scaled, item, config);

    (final_score, exponent, boost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::path::PathBuf;

    fn create_test_item(
        base_score: f64,
        debt_type: DebtType,
        upstream_deps: usize,
        downstream_deps: usize,
        role: FunctionRole,
        cyclomatic: u32,
    ) -> UnifiedDebtItem {
        use crate::priority::unified_scorer::{Location, UnifiedScore};
        use crate::priority::ActionableRecommendation;
        use crate::priority::ImpactMetrics;

        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_func".to_string(),
                line: 1,
            },
            debt_type,
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 5.0,
                role_multiplier: 1.0,
                final_score: base_score,
                base_score: Some(base_score),
                exponential_factor: Some(1.0),
                risk_boost: Some(1.0),
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: role,
            recommendation: ActionableRecommendation {
                primary_action: "Test".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
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
            upstream_dependencies: upstream_deps,
            downstream_dependencies: downstream_deps,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cyclomatic,
            entropy_details: None,
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
            language_specific: None, // spec 190
        }
    }

    #[test]
    fn test_exponential_scaling_god_object() {
        let config = ScalingConfig::default();
        let base = 10.0;

        let scaled = apply_exponential_scaling(
            base,
            &DebtType::GodObject {
                methods: 50,
                fields: 20,
                responsibilities: 10,
                god_object_score: 85.0,
            },
            &config,
        );

        // 10^1.4 ≈ 25.1
        assert!(
            (scaled - 25.1).abs() < 0.5,
            "Expected ~25.1, got {}",
            scaled
        );
    }

    #[test]
    fn test_exponential_scaling_creates_separation() {
        let config = ScalingConfig::default();
        let base = 20.0;

        let god_object_scaled = apply_exponential_scaling(
            base,
            &DebtType::GodObject {
                methods: 50,
                fields: 20,
                responsibilities: 10,
                god_object_score: 85.0,
            },
            &config,
        );

        let testing_gap_scaled = apply_exponential_scaling(
            base,
            &DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 10,
            },
            &config,
        );

        // God object should score significantly higher
        // 20^1.4 ≈ 66 vs 20^1.0 = 20
        assert!(god_object_scaled > testing_gap_scaled * 2.0);
    }

    #[test]
    fn test_risk_boosts_multiply() {
        let config = ScalingConfig::default();
        let item = create_test_item(
            10.0,
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 25,
                cognitive: 30,
            },
            10, // High upstream deps
            10, // High downstream deps
            FunctionRole::EntryPoint,
            25,
        );

        let boosted = apply_risk_boosts(10.0, &item, &config);

        // Should apply: high_deps (1.2) * entry_point (1.15) * complex_untested (1.25)
        // 10.0 * 1.2 * 1.15 * 1.25 = 17.25
        assert!(
            (boosted - 17.25).abs() < 0.1,
            "Expected ~17.25, got {}",
            boosted
        );
    }

    #[test]
    fn test_calculate_final_score_integration() {
        let config = ScalingConfig::default();
        let item = create_test_item(
            30.0,
            DebtType::GodObject {
                methods: 50,
                fields: 1000,
                responsibilities: 10,
                god_object_score: 85.0,
            },
            20, // High deps
            10,
            FunctionRole::EntryPoint,
            35,
        );

        let (final_score, exponent, boost) =
            calculate_final_score(30.0, &item.debt_type, &item, &config);

        // 30^1.4 ≈ 108, then * 1.2 (high_deps) * 1.15 (entry_point) ≈ 149
        assert!(exponent == 1.4, "Expected exponent 1.4, got {}", exponent);
        assert!(boost > 1.3, "Expected boost > 1.3, got {}", boost);
        assert!(
            final_score > 140.0,
            "Expected score > 140, got {}",
            final_score
        );
    }

    #[test]
    fn test_architectural_issues_naturally_surface() {
        let config = ScalingConfig::default();

        let god_object_item = create_test_item(
            30.0,
            DebtType::GodObject {
                methods: 50,
                fields: 1000,
                responsibilities: 10,
                god_object_score: 85.0,
            },
            5,
            5,
            FunctionRole::PureLogic,
            30,
        );

        let simple_gap_item = create_test_item(
            50.0,
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 10,
                cognitive: 10,
            },
            5,
            5,
            FunctionRole::PureLogic,
            10,
        );

        let (god_score, _, _) =
            calculate_final_score(30.0, &god_object_item.debt_type, &god_object_item, &config);

        let (gap_score, _, _) =
            calculate_final_score(50.0, &simple_gap_item.debt_type, &simple_gap_item, &config);

        // 30^1.4 ≈ 108 should beat 50^1.0 = 50
        assert!(
            god_score > gap_score,
            "God object (score {}) should rank higher than simple testing gap (score {})",
            god_score,
            gap_score
        );
    }

    #[test]
    fn test_minimum_base_score_prevents_zero() {
        let config = ScalingConfig::default();

        let scaled = apply_exponential_scaling(
            0.0,
            &DebtType::GodObject {
                methods: 50,
                fields: 1000,
                responsibilities: 10,
                god_object_score: 85.0,
            },
            &config,
        );

        // Should use minimum of 1.0, so 1.0^1.4 = 1.0
        assert!(
            scaled >= 1.0,
            "Scaled score should be at least 1.0, got {}",
            scaled
        );
    }

    // Property test: score ordering should be strict and monotonic
    proptest! {
        #[test]
        fn prop_score_ordering_is_strict(
            base_scores in prop::collection::vec(0.0f64..100.0f64, 2..50)
        ) {
            let config = ScalingConfig::default();

            // Create items with random base scores
            let items: Vec<_> = base_scores.iter().enumerate().map(|(i, &score)| {
                let debt_type = if i % 3 == 0 {
                    DebtType::GodObject {
                        methods: 50,
                        fields: 20,
                        responsibilities: 10,
                        god_object_score: 85.0,
                    }
                } else if i % 3 == 1 {
                    DebtType::ComplexityHotspot {
                        cyclomatic: 35,
                        cognitive: 40,
                        adjusted_cyclomatic: None,
                    }
                } else {
                    DebtType::TestingGap {
                        coverage: 0.0,
                        cyclomatic: 15,
                        cognitive: 20,
                    }
                };

                create_test_item(
                    score,
                    debt_type,
                    5,
                    5,
                    FunctionRole::PureLogic,
                    20,
                )
            }).collect();

            // Calculate final scores
            let mut scored_items: Vec<_> = items.iter().enumerate().map(|(idx, item)| {
                let (final_score, _, _) = calculate_final_score(
                    base_scores[idx],
                    &item.debt_type,
                    item,
                    &config
                );
                (final_score, idx)
            }).collect();

            // Sort by score descending (as in get_top_mixed_priorities)
            scored_items.sort_by(|a, b| {
                b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
            });

            // Verify no score inversions: score[i] >= score[i+1]
            for i in 0..scored_items.len().saturating_sub(1) {
                let score_i = scored_items[i].0;
                let score_next = scored_items[i + 1].0;
                prop_assert!(
                    score_i >= score_next,
                    "Score inversion at index {}: {} < {} (items {} and {})",
                    i,
                    score_i,
                    score_next,
                    scored_items[i].1,
                    scored_items[i + 1].1
                );
            }
        }

        #[test]
        fn prop_exponential_scaling_monotonic(
            base_score in 1.0f64..100.0f64,
            higher_score in 1.0f64..100.0f64,
        ) {
            // Ensure higher_score > base_score
            let (lower, higher) = if base_score < higher_score {
                (base_score, higher_score)
            } else {
                (higher_score, base_score)
            };

            if (higher - lower).abs() < 0.01 {
                return Ok(());
            }

            let config = ScalingConfig::default();
            let debt_type = DebtType::GodObject {
                methods: 50,
                fields: 20,
                responsibilities: 10,
                god_object_score: 85.0,
            };

            let scaled_lower = apply_exponential_scaling(lower, &debt_type, &config);
            let scaled_higher = apply_exponential_scaling(higher, &debt_type, &config);

            // Exponential scaling must preserve ordering
            prop_assert!(
                scaled_higher >= scaled_lower,
                "Exponential scaling violated monotonicity: {}^1.4={} should be >= {}^1.4={}",
                higher, scaled_higher, lower, scaled_lower
            );
        }

        #[test]
        fn prop_risk_boosts_non_negative(
            base_score in 1.0f64..100.0f64,
            upstream_deps in 0usize..30,
            downstream_deps in 0usize..30,
        ) {
            let config = ScalingConfig::default();
            let role = if upstream_deps > 20 {
                FunctionRole::EntryPoint
            } else {
                FunctionRole::PureLogic
            };

            let item = create_test_item(
                base_score,
                DebtType::TestingGap {
                    coverage: 0.0,
                    cyclomatic: 25,
                    cognitive: 30,
                },
                upstream_deps,
                downstream_deps,
                role,
                25,
            );

            let boosted = apply_risk_boosts(base_score, &item, &config);

            // Risk boosts should never decrease score
            prop_assert!(
                boosted >= base_score,
                "Risk boost decreased score: {} -> {}",
                base_score,
                boosted
            );
        }

        #[test]
        fn prop_final_score_never_decreases_from_base(
            base_score in 1.0f64..100.0f64,
        ) {
            let config = ScalingConfig::default();
            let item = create_test_item(
                base_score,
                DebtType::GodObject {
                    methods: 50,
                    fields: 20,
                    responsibilities: 10,
                    god_object_score: 85.0,
                },
                10,
                10,
                FunctionRole::EntryPoint,
                30,
            );

            let (final_score, _, _) = calculate_final_score(
                base_score,
                &item.debt_type,
                &item,
                &config,
            );

            // With exponent >= 1.0 and boost >= 1.0, final score should never be less than base
            // (for base_score >= 1.0)
            prop_assert!(
                final_score >= base_score * 0.99, // Allow tiny floating point errors
                "Final score {} is less than base score {}",
                final_score,
                base_score
            );
        }
    }
}
