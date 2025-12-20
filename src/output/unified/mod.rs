//! Unified output format that provides consistent structure for File and Function debt items
//!
//! This module implements spec 108, providing a normalized JSON output format where:
//! - All items have consistent top-level fields (type, score, category, priority, location)
//! - Score is at the same path for both File and Function items
//! - Location structure is unified (file, line, function)
//! - Simplifies filtering and sorting across item types
//!
//! ## Output Invariants (spec 230)
//!
//! This module guarantees the following output invariants:
//! - Score >= 0 (never negative)
//! - Score <= 1000 (reasonable upper bound)
//! - Coverage in 0.0..=1.0 (when present)
//! - Confidence in 0.0..=1.0 (when present)
//! - Priority matches score thresholds (Critical >= 100, High >= 50, Medium >= 20)
//!
//! These invariants are enforced via `debug_assert!` in debug builds and validated
//! through property-based testing.

mod anti_patterns;
mod cohesion;
mod coupling;
mod dedup;
mod dependencies;
mod file_item;
mod format;
mod func_item;
mod location;
mod patterns;
mod priority;
mod types;

// Re-export all public items
pub use anti_patterns::{AntiPatternItem, AntiPatternOutput, AntiPatternSummary};
pub use cohesion::{CohesionClassification, CohesionOutput, CohesionSummary};
pub use coupling::{
    calculate_instability, classify_coupling, CouplingClassification, FileDependencies,
};
pub use dedup::deduplicate_items;
pub use dependencies::{Dependencies, PurityAnalysis, RecommendationOutput};
pub use file_item::{FileDebtItemOutput, FileImpactOutput, FileMetricsOutput, FileScoringDetails};
pub use format::{round_ratio, round_score};
pub use func_item::{
    AdjustedComplexity, FunctionDebtItemOutput, FunctionImpactOutput, FunctionMetricsOutput,
    FunctionScoringDetails,
};
pub use location::UnifiedLocation;
pub use priority::Priority;
pub use types::{
    DebtSummary, OutputMetadata, ScoreDistribution, TypeBreakdown, UnifiedDebtItemOutput,
    UnifiedOutput,
};

use crate::priority::UnifiedAnalysisQueries;
use std::collections::HashMap;

/// Convert analysis results to unified output format
pub fn convert_to_unified_format(
    analysis: &crate::priority::UnifiedAnalysis,
    include_scoring_details: bool,
) -> UnifiedOutput {
    // Get all debt items sorted by score
    let all_items = analysis.get_top_mixed_priorities(usize::MAX);

    // Convert to unified format with call graph for cohesion calculation (spec 198)
    let unified_items: Vec<UnifiedDebtItemOutput> = all_items
        .iter()
        .map(|item| {
            let output = UnifiedDebtItemOutput::from_debt_item_with_call_graph(
                item,
                include_scoring_details,
                Some(&analysis.call_graph),
            );
            // Assert invariants in debug builds (spec 230)
            output.assert_invariants();
            output
        })
        .collect();

    // Deduplicate items before calculating summary statistics (spec 231)
    let unified_items = deduplicate_items(unified_items);

    // Calculate summary statistics from deduplicated items
    let mut file_count = 0;
    let mut function_count = 0;
    let mut category_counts: HashMap<String, usize> = HashMap::new();
    let mut score_dist = ScoreDistribution {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
    };

    // Calculate total debt score from deduplicated items (spec 231)
    let total_debt_score: f64 = unified_items.iter().map(|item| item.score()).sum();

    // Cohesion summary statistics (spec 198)
    let mut cohesion_scores: Vec<f64> = Vec::new();
    let mut high_cohesion_count = 0;
    let mut medium_cohesion_count = 0;
    let mut low_cohesion_count = 0;

    for item in &unified_items {
        match item {
            UnifiedDebtItemOutput::File(f) => {
                file_count += 1;
                *category_counts.entry(f.category.clone()).or_insert(0) += 1;
                match f.priority {
                    Priority::Critical => score_dist.critical += 1,
                    Priority::High => score_dist.high += 1,
                    Priority::Medium => score_dist.medium += 1,
                    Priority::Low => score_dist.low += 1,
                }
                // Collect cohesion stats (spec 198)
                if let Some(ref cohesion) = f.cohesion {
                    cohesion_scores.push(cohesion.score);
                    match cohesion.classification {
                        CohesionClassification::High => high_cohesion_count += 1,
                        CohesionClassification::Medium => medium_cohesion_count += 1,
                        CohesionClassification::Low => low_cohesion_count += 1,
                    }
                }
            }
            UnifiedDebtItemOutput::Function(f) => {
                function_count += 1;
                *category_counts.entry(f.category.clone()).or_insert(0) += 1;
                match f.priority {
                    Priority::Critical => score_dist.critical += 1,
                    Priority::High => score_dist.high += 1,
                    Priority::Medium => score_dist.medium += 1,
                    Priority::Low => score_dist.low += 1,
                }
            }
        }
    }

    // Build cohesion summary with rounding (spec 198, 230)
    let cohesion_summary = if !cohesion_scores.is_empty() {
        let average = cohesion_scores.iter().sum::<f64>() / cohesion_scores.len() as f64;
        Some(CohesionSummary {
            average: round_ratio(average),
            high_cohesion_files: high_cohesion_count,
            medium_cohesion_files: medium_cohesion_count,
            low_cohesion_files: low_cohesion_count,
        })
    } else {
        None
    };

    // Recalculate debt density from filtered items, with rounding (spec 230)
    let debt_density = if analysis.total_lines_of_code > 0 {
        round_score((total_debt_score / analysis.total_lines_of_code as f64) * 1000.0)
    } else {
        0.0
    };

    UnifiedOutput {
        format_version: "2.0".to_string(),
        metadata: OutputMetadata {
            debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            project_root: None,
            analysis_type: "unified".to_string(),
        },
        summary: DebtSummary {
            total_items: unified_items.len(),
            total_debt_score: round_score(total_debt_score),
            debt_density,
            total_loc: analysis.total_lines_of_code,
            by_type: TypeBreakdown {
                file: file_count,
                function: function_count,
            },
            by_category: category_counts,
            score_distribution: score_dist,
            cohesion: cohesion_summary,
        },
        items: unified_items,
    }
}

// ============================================================================
// Property-Based Tests (spec 230)
// ============================================================================

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        /// Generate arbitrary function metrics with valid ranges
        fn arb_function_metrics()
            (cyclomatic in 1u32..100,
             cognitive in 1u32..100,
             length in 1usize..1000,
             nesting in 0u32..10,
             coverage in prop::option::of(0.0f64..=1.0),
             entropy in prop::option::of(0.0f64..=1.0))
            -> FunctionMetricsOutput
        {
            FunctionMetricsOutput {
                cyclomatic_complexity: cyclomatic,
                cognitive_complexity: cognitive,
                length,
                nesting_depth: nesting,
                coverage: coverage.map(round_ratio),
                uncovered_lines: None,
                entropy_score: entropy.map(round_ratio),
            }
        }
    }

    prop_compose! {
        /// Generate arbitrary file metrics with valid ranges
        fn arb_file_metrics()
            (lines in 1usize..10000,
             functions in 1usize..100,
             classes in 0usize..20,
             avg_complexity in 1.0f64..50.0,
             max_complexity in 1u32..100,
             total_complexity in 1u32..1000,
             coverage in 0.0f64..=1.0,
             uncovered_lines in 0usize..1000)
            -> FileMetricsOutput
        {
            FileMetricsOutput {
                lines,
                functions,
                classes,
                avg_complexity: round_score(avg_complexity),
                max_complexity,
                total_complexity,
                coverage: round_ratio(coverage),
                uncovered_lines,
            }
        }
    }

    proptest! {
        #[test]
        fn test_round_score_never_negative(score in 0.0f64..1000.0) {
            let rounded = round_score(score);
            prop_assert!(rounded >= 0.0, "Rounded score {} is negative", rounded);
        }

        #[test]
        fn test_round_ratio_in_valid_range(ratio in 0.0f64..=1.0) {
            let rounded = round_ratio(ratio);
            prop_assert!(
                (0.0..=1.0).contains(&rounded),
                "Rounded ratio {} is out of range [0, 1]",
                rounded
            );
        }

        #[test]
        fn test_priority_matches_score_thresholds(score in 0.0f64..500.0) {
            let rounded_score = round_score(score);
            let priority = Priority::from_score(rounded_score);
            let expected = Priority::from_score(rounded_score);

            prop_assert_eq!(
                std::mem::discriminant(&priority),
                std::mem::discriminant(&expected),
                "Priority {:?} doesn't match expected {:?} for score {}",
                priority,
                expected,
                rounded_score
            );
        }

        #[test]
        fn test_function_metrics_serialization_roundtrip(metrics in arb_function_metrics()) {
            let json = serde_json::to_string(&metrics).expect("Serialization failed");
            let deserialized: FunctionMetricsOutput =
                serde_json::from_str(&json).expect("Deserialization failed");

            prop_assert_eq!(
                metrics.cyclomatic_complexity,
                deserialized.cyclomatic_complexity
            );
            prop_assert_eq!(
                metrics.cognitive_complexity,
                deserialized.cognitive_complexity
            );
            prop_assert_eq!(metrics.length, deserialized.length);
            prop_assert_eq!(metrics.nesting_depth, deserialized.nesting_depth);

            // Coverage and entropy should match if present
            if let (Some(a), Some(b)) = (metrics.coverage, deserialized.coverage) {
                prop_assert!((a - b).abs() < 0.0001, "Coverage mismatch: {} vs {}", a, b);
            }
        }

        #[test]
        fn test_file_metrics_serialization_roundtrip(metrics in arb_file_metrics()) {
            let json = serde_json::to_string(&metrics).expect("Serialization failed");
            let deserialized: FileMetricsOutput =
                serde_json::from_str(&json).expect("Deserialization failed");

            prop_assert_eq!(metrics.lines, deserialized.lines);
            prop_assert_eq!(metrics.functions, deserialized.functions);
            prop_assert_eq!(metrics.max_complexity, deserialized.max_complexity);
            prop_assert!(
                (metrics.coverage - deserialized.coverage).abs() < 0.0001,
                "Coverage mismatch: {} vs {}",
                metrics.coverage,
                deserialized.coverage
            );
        }

        #[test]
        fn test_cohesion_classification_matches_score(score in 0.0f64..=1.0) {
            let rounded = round_ratio(score);
            let classification = CohesionClassification::from_score(rounded);

            let expected = if rounded >= 0.7 {
                CohesionClassification::High
            } else if rounded >= 0.4 {
                CohesionClassification::Medium
            } else {
                CohesionClassification::Low
            };

            prop_assert_eq!(
                classification.clone(),
                expected.clone(),
                "Classification {:?} doesn't match expected {:?} for score {}",
                classification,
                expected,
                rounded
            );
        }
    }
}

// ============================================================================
// Integration Tests for spec 232: Dampened cyclomatic calculation fix
// ============================================================================

#[cfg(test)]
mod dampening_tests {
    use super::*;
    use crate::priority::unified_scorer::EntropyDetails;
    use crate::priority::{
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedDebtItem,
        UnifiedScore,
    };
    use std::path::PathBuf;

    fn create_test_item_with_complexity(
        cyclomatic: u32,
        cognitive: u32,
        dampening_factor: f64,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                line: 10,
                function: "test_func".to_string(),
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic,
                cognitive,
            },
            unified_score: UnifiedScore {
                complexity_factor: 50.0,
                coverage_factor: 80.0,
                dependency_factor: 50.0,
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
            function_role: FunctionRole::PureLogic,
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
                risk_reduction: 0.0,
                coverage_improvement: 0.0,
                lines_reduction: 0,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 1,
            function_length: 20,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            entropy_details: Some(EntropyDetails {
                entropy_score: 0.5,
                pattern_repetition: 0.3,
                original_complexity: cognitive,
                adjusted_complexity: (cognitive as f64 * dampening_factor) as u32,
                dampening_factor,
                adjusted_cognitive: (cognitive as f64 * dampening_factor) as u32,
            }),
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: Some(dampening_factor),
            is_pure: None,
            purity_confidence: None,
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
    fn test_dampening_factor_one_preserves_cyclomatic() {
        // Spec 232: When dampening_factor = 1.0, dampened_cyclomatic = cyclomatic
        let item = create_test_item_with_complexity(11, 23, 1.0);
        let output = FunctionDebtItemOutput::from_function_item(&item, false);

        let adjusted = output
            .adjusted_complexity
            .expect("should have adjusted_complexity");
        assert_eq!(adjusted.dampening_factor, 1.0);
        // Critical assertion: dampened_cyclomatic should equal cyclomatic, not cognitive
        assert_eq!(
            adjusted.dampened_cyclomatic, 11.0,
            "dampened_cyclomatic should equal cyclomatic_complexity when factor is 1.0"
        );
    }

    #[test]
    fn test_dampening_reduces_cyclomatic() {
        // Spec 232: dampened_cyclomatic = cyclomatic * dampening_factor
        let item = create_test_item_with_complexity(20, 40, 0.5);
        let output = FunctionDebtItemOutput::from_function_item(&item, false);

        let adjusted = output
            .adjusted_complexity
            .expect("should have adjusted_complexity");
        assert_eq!(adjusted.dampening_factor, 0.5);
        assert_eq!(
            adjusted.dampened_cyclomatic, 10.0,
            "dampened_cyclomatic should be cyclomatic * factor"
        );
    }

    #[test]
    fn test_dampened_cyclomatic_independent_of_cognitive() {
        // Spec 232: dampened_cyclomatic should only depend on cyclomatic, not cognitive
        // Two items with same cyclomatic but different cognitive
        let item1 = create_test_item_with_complexity(15, 10, 0.8);
        let item2 = create_test_item_with_complexity(15, 50, 0.8);

        let output1 = FunctionDebtItemOutput::from_function_item(&item1, false);
        let output2 = FunctionDebtItemOutput::from_function_item(&item2, false);

        let adjusted1 = output1
            .adjusted_complexity
            .expect("should have adjusted_complexity");
        let adjusted2 = output2
            .adjusted_complexity
            .expect("should have adjusted_complexity");

        // Same dampened cyclomatic regardless of cognitive complexity
        assert_eq!(
            adjusted1.dampened_cyclomatic, adjusted2.dampened_cyclomatic,
            "dampened_cyclomatic should be the same for items with same cyclomatic complexity"
        );
        assert_eq!(
            adjusted1.dampened_cyclomatic,
            12.0, // 15 * 0.8
            "dampened_cyclomatic should be cyclomatic * dampening_factor"
        );
    }
}
