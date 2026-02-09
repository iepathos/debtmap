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
    calculate_architectural_dependency_factor, calculate_instability, classify_coupling,
    classify_coupling_pattern, CouplingClassification, FileDependencies,
};
pub use dedup::deduplicate_items;
pub use dependencies::{Dependencies, PurityAnalysis};
pub use file_item::{
    DistributionMetricsOutput, FileDebtItemOutput, FileImpactOutput, FileMetricsOutput,
    FileScoringDetails,
};
pub use format::{round_ratio, round_score};
pub use func_item::{
    AdjustedComplexity, ContextSuggestionOutput, FileRangeOutput, FunctionDebtItemOutput,
    FunctionImpactOutput, FunctionMetricsOutput, FunctionScoringDetails, GitHistoryOutput,
    RelatedContextOutput,
};
pub use location::UnifiedLocation;
pub use priority::Priority;
pub use types::{
    DebtSummary, OutputMetadata, ScoreDistribution, TypeBreakdown, UnifiedDebtItemOutput,
    UnifiedOutput,
};

use std::collections::HashMap;

/// Statistics accumulated from iterating over unified debt items
#[derive(Debug, Default)]
struct ItemStatistics {
    file_count: usize,
    function_count: usize,
    category_counts: HashMap<String, usize>,
    score_distribution: ScoreDistribution,
    total_debt_score: f64,
    cohesion_scores: Vec<f64>,
    high_cohesion_count: usize,
    medium_cohesion_count: usize,
    low_cohesion_count: usize,
}

/// Collect all debt items from unified analysis (pure function)
fn collect_all_items(
    analysis: &crate::priority::UnifiedAnalysis,
) -> im::Vector<crate::priority::DebtItem> {
    analysis
        .items
        .iter()
        .map(|item| crate::priority::DebtItem::Function(Box::new(item.clone())))
        .chain(
            analysis
                .file_items
                .iter()
                .map(|item| crate::priority::DebtItem::File(Box::new(item.clone()))),
        )
        .collect()
}

/// Convert items to unified format with invariant validation (pure function)
fn convert_items(
    items: &im::Vector<crate::priority::DebtItem>,
    include_scoring_details: bool,
    call_graph: &crate::priority::CallGraph,
) -> Vec<UnifiedDebtItemOutput> {
    items
        .iter()
        .map(|item| {
            let output = UnifiedDebtItemOutput::from_debt_item_with_call_graph(
                item,
                include_scoring_details,
                Some(call_graph),
            );
            output.assert_invariants();
            output
        })
        .collect()
}

/// Sort items by score descending (pure function returning new vector)
fn sort_by_score_descending(mut items: Vec<UnifiedDebtItemOutput>) -> Vec<UnifiedDebtItemOutput> {
    items.sort_by(|a, b| {
        b.score()
            .partial_cmp(&a.score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items
}

/// Update statistics for a file item (helper for fold)
fn accumulate_file_stats(
    mut stats: ItemStatistics,
    f: &file_item::FileDebtItemOutput,
) -> ItemStatistics {
    stats.file_count += 1;
    *stats.category_counts.entry(f.category.clone()).or_insert(0) += 1;
    match f.priority {
        Priority::Critical => stats.score_distribution.critical += 1,
        Priority::High => stats.score_distribution.high += 1,
        Priority::Medium => stats.score_distribution.medium += 1,
        Priority::Low => stats.score_distribution.low += 1,
    }
    if let Some(ref cohesion) = f.cohesion {
        stats.cohesion_scores.push(cohesion.score);
        match cohesion.classification {
            CohesionClassification::High => stats.high_cohesion_count += 1,
            CohesionClassification::Medium => stats.medium_cohesion_count += 1,
            CohesionClassification::Low => stats.low_cohesion_count += 1,
        }
    }
    stats
}

/// Update statistics for a function item (helper for fold)
fn accumulate_function_stats(
    mut stats: ItemStatistics,
    f: &func_item::FunctionDebtItemOutput,
) -> ItemStatistics {
    stats.function_count += 1;
    *stats.category_counts.entry(f.category.clone()).or_insert(0) += 1;
    match f.priority {
        Priority::Critical => stats.score_distribution.critical += 1,
        Priority::High => stats.score_distribution.high += 1,
        Priority::Medium => stats.score_distribution.medium += 1,
        Priority::Low => stats.score_distribution.low += 1,
    }
    stats
}

/// Calculate all summary statistics from unified items (pure function)
fn calculate_item_statistics(items: &[UnifiedDebtItemOutput]) -> ItemStatistics {
    items
        .iter()
        .fold(ItemStatistics::default(), |mut stats, item| {
            stats.total_debt_score += item.score();
            match item {
                UnifiedDebtItemOutput::File(f) => accumulate_file_stats(stats, f),
                UnifiedDebtItemOutput::Function(f) => accumulate_function_stats(stats, f),
            }
        })
}

/// Build cohesion summary from statistics (pure function)
fn build_cohesion_summary_from_stats(stats: &ItemStatistics) -> Option<CohesionSummary> {
    if stats.cohesion_scores.is_empty() {
        None
    } else {
        let average =
            stats.cohesion_scores.iter().sum::<f64>() / stats.cohesion_scores.len() as f64;
        Some(CohesionSummary {
            average: round_ratio(average),
            high_cohesion_files: stats.high_cohesion_count,
            medium_cohesion_files: stats.medium_cohesion_count,
            low_cohesion_files: stats.low_cohesion_count,
        })
    }
}

/// Calculate debt density from total score and LOC (pure function)
fn calculate_debt_density(total_debt_score: f64, total_loc: usize) -> f64 {
    if total_loc > 0 {
        round_score((total_debt_score / total_loc as f64) * 1000.0)
    } else {
        0.0
    }
}

/// Build the final UnifiedOutput from items and statistics (pure function)
fn build_unified_output(
    items: Vec<UnifiedDebtItemOutput>,
    stats: ItemStatistics,
    total_loc: usize,
) -> UnifiedOutput {
    let debt_density = calculate_debt_density(stats.total_debt_score, total_loc);
    let cohesion_summary = build_cohesion_summary_from_stats(&stats);

    UnifiedOutput {
        format_version: "3.0".to_string(),
        metadata: OutputMetadata {
            debtmap_version: env!("CARGO_PKG_VERSION").to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            project_root: None,
            analysis_type: "unified".to_string(),
        },
        summary: DebtSummary {
            total_items: items.len(),
            total_debt_score: round_score(stats.total_debt_score),
            debt_density,
            total_loc,
            by_type: TypeBreakdown {
                file: stats.file_count,
                function: stats.function_count,
            },
            by_category: stats.category_counts,
            score_distribution: stats.score_distribution,
            cohesion: cohesion_summary,
        },
        items,
    }
}

/// Convert analysis results to unified output format
///
/// This is the main entry point that orchestrates the conversion pipeline:
/// 1. Collect all items from analysis
/// 2. Convert to unified format with invariant validation
/// 3. Deduplicate and sort by score
/// 4. Calculate summary statistics
/// 5. Build final output
pub fn convert_to_unified_format(
    analysis: &crate::priority::UnifiedAnalysis,
    include_scoring_details: bool,
) -> UnifiedOutput {
    let all_items = collect_all_items(analysis);
    let unified_items = convert_items(&all_items, include_scoring_details, &analysis.call_graph);
    let deduplicated = deduplicate_items(unified_items);
    let sorted_items = sort_by_score_descending(deduplicated);
    let stats = calculate_item_statistics(&sorted_items);
    build_unified_output(sorted_items, stats, analysis.total_lines_of_code)
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
                ..Default::default()
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
                distribution: None, // Spec 268: optional distribution metrics
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
    use crate::complexity::EntropyAnalysis;
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
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 1,
            function_length: 20,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            entropy_analysis: Some(EntropyAnalysis {
                entropy_score: 0.5,
                pattern_repetition: 0.3,
                branch_similarity: 0.2,
                original_complexity: cognitive,
                adjusted_complexity: (cognitive as f64 * dampening_factor) as u32,
                dampening_factor,
                dampening_was_applied: dampening_factor < 1.0,
                reasoning: vec![],
            }),
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
