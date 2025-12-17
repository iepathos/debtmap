//! Pure predicate functions for filtering debt items.
//!
//! This module provides composable, testable predicates for filtering
//! `UnifiedDebtItem` instances. Each predicate is a pure function that
//! takes an item and configuration, returning a boolean.
//!
//! # Design Principles
//!
//! - **Pure functions**: No side effects, deterministic output
//! - **Single responsibility**: Each predicate checks one thing
//! - **Composability**: Predicates can be combined
//! - **Testability**: Easy to unit test in isolation
//!
//! # Examples
//!
//! ```ignore
//! use debtmap::priority::filter_predicates::*;
//! use debtmap::priority::UnifiedDebtItem;
//!
//! # fn create_test_item(score: f64) -> UnifiedDebtItem { todo!() }
//! let item = create_test_item(50.0);
//!
//! // Check individual predicates
//! assert!(meets_score_threshold(&item, 10.0));
//! assert!(!meets_score_threshold(&item, 75.0));
//!
//! // Combine predicates
//! let passes = meets_score_threshold(&item, 10.0)
//!     && meets_complexity_thresholds(&item, 2, 5);
//! ```

use crate::priority::{DebtType, UnifiedDebtItem};
use serde::{Deserialize, Serialize};

/// Tracks filtering statistics for debugging and telemetry.
///
/// These statistics help identify why items are being filtered out,
/// which is crucial for debugging filtering issues.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilterStatistics {
    /// Total number of items processed (attempted to add)
    pub total_items_processed: usize,

    /// Items filtered due to score below threshold
    pub filtered_by_score: usize,

    /// Items filtered due to risk score below threshold
    pub filtered_by_risk: usize,

    /// Items filtered due to complexity below threshold
    pub filtered_by_complexity: usize,

    /// Items filtered as duplicates
    pub filtered_as_duplicate: usize,

    /// Items successfully added
    pub items_added: usize,
}

impl FilterStatistics {
    /// Create a new empty statistics tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get total items filtered (all rejection reasons).
    pub fn total_filtered(&self) -> usize {
        self.filtered_by_score
            + self.filtered_by_risk
            + self.filtered_by_complexity
            + self.filtered_as_duplicate
    }

    /// Get acceptance rate (percentage added vs processed).
    pub fn acceptance_rate(&self) -> f64 {
        if self.total_items_processed == 0 {
            return 0.0;
        }
        (self.items_added as f64 / self.total_items_processed as f64) * 100.0
    }
}

/// Check if item meets minimum score threshold.
///
/// # Examples
///
/// ```ignore
/// # use debtmap::priority::filter_predicates::meets_score_threshold;
/// # use debtmap::priority::UnifiedDebtItem;
/// # fn create_item_with_score(score: f64) -> UnifiedDebtItem { todo!() }
/// let high_score_item = create_item_with_score(50.0);
/// let low_score_item = create_item_with_score(2.0);
///
/// assert!(meets_score_threshold(&high_score_item, 10.0));
/// assert!(!meets_score_threshold(&low_score_item, 10.0));
/// ```
#[inline]
pub fn meets_score_threshold(item: &UnifiedDebtItem, min_score: f64) -> bool {
    item.unified_score.final_score.value() >= min_score
}

/// Check if item meets minimum risk threshold.
///
/// Non-risk items always pass this check. For risk items,
/// the risk score must be >= min_risk.
///
/// # Examples
///
/// ```ignore
/// # use debtmap::priority::filter_predicates::meets_risk_threshold;
/// # use debtmap::priority::UnifiedDebtItem;
/// # fn create_risk_item(risk_score: f64) -> UnifiedDebtItem { todo!() }
/// # fn create_complexity_item() -> UnifiedDebtItem { todo!() }
/// let risk_item = create_risk_item(0.8);
/// let normal_item = create_complexity_item();
///
/// assert!(meets_risk_threshold(&risk_item, 0.5));
/// assert!(!meets_risk_threshold(&risk_item, 0.9));
/// assert!(meets_risk_threshold(&normal_item, 0.9)); // Non-risk always passes
/// ```
#[inline]
pub fn meets_risk_threshold(item: &UnifiedDebtItem, min_risk: f64) -> bool {
    match &item.debt_type {
        DebtType::Risk { risk_score, .. } => *risk_score >= min_risk,
        _ => true, // Non-risk items pass by default
    }
}

/// Check if item is exempt from complexity filtering.
///
/// Exempted types:
/// - Test-related: `TestComplexityHotspot`, `TestTodo`, `TestDuplication`
/// - Architectural: `GodObject` (covers all god object detection types)
///
/// These types have different complexity characteristics and are
/// evaluated by other criteria.
///
/// # Examples
///
/// ```ignore
/// # use debtmap::priority::filter_predicates::is_exempt_from_complexity_filter;
/// # use debtmap::priority::UnifiedDebtItem;
/// # fn create_god_object_item() -> UnifiedDebtItem { todo!() }
/// # fn create_test_complexity_item() -> UnifiedDebtItem { todo!() }
/// # fn create_complexity_hotspot() -> UnifiedDebtItem { todo!() }
/// let god_object = create_god_object_item();
/// let test_item = create_test_complexity_item();
/// let regular_item = create_complexity_hotspot();
///
/// assert!(is_exempt_from_complexity_filter(&god_object));
/// assert!(is_exempt_from_complexity_filter(&test_item));
/// assert!(!is_exempt_from_complexity_filter(&regular_item));
/// ```
#[inline]
pub fn is_exempt_from_complexity_filter(item: &UnifiedDebtItem) -> bool {
    matches!(
        item.debt_type,
        DebtType::TestComplexityHotspot { .. }
            | DebtType::TestTodo { .. }
            | DebtType::TestDuplication { .. }
            | DebtType::GodObject { .. }
            | DebtType::ErrorSwallowing { .. } // Code-level pattern detection, not function-level complexity
    )
}

/// Check if item meets minimum complexity thresholds.
///
/// Exempt items (tests, god objects) always pass. Other items must
/// meet BOTH cyclomatic and cognitive complexity thresholds.
///
/// # Examples
///
/// ```ignore
/// # use debtmap::priority::filter_predicates::meets_complexity_thresholds;
/// # use debtmap::priority::UnifiedDebtItem;
/// # fn create_item(cyclomatic: u32, cognitive: u32) -> UnifiedDebtItem { todo!() }
/// # fn create_god_object_item(cyclomatic: u32, cognitive: u32) -> UnifiedDebtItem { todo!() }
/// let complex_item = create_item(10, 15);
/// let simple_item = create_item(1, 2);
/// let exempt_item = create_god_object_item(0, 0);
///
/// assert!(meets_complexity_thresholds(&complex_item, 5, 10));
/// assert!(!meets_complexity_thresholds(&simple_item, 5, 10));
/// assert!(meets_complexity_thresholds(&exempt_item, 5, 10)); // Exempt
/// ```
#[inline]
pub fn meets_complexity_thresholds(
    item: &UnifiedDebtItem,
    min_cyclomatic: u32,
    min_cognitive: u32,
) -> bool {
    if is_exempt_from_complexity_filter(item) {
        return true;
    }

    item.cyclomatic_complexity >= min_cyclomatic && item.cognitive_complexity >= min_cognitive
}

/// Check if two items are duplicates.
///
/// Items are duplicates if they have:
/// 1. Same file path
/// 2. Same line number
/// 3. Same debt type (by discriminant, not value)
///
/// # Examples
///
/// ```ignore
/// # use debtmap::priority::filter_predicates::is_duplicate_of;
/// # use debtmap::priority::{UnifiedDebtItem, DebtType};
/// # fn create_item(file: &str, line: u32, debt_type: DebtType) -> UnifiedDebtItem { todo!() }
/// # let DebtType::ComplexityHotspot = todo!();
/// # let DebtType::GodObject = todo!();
/// let item1 = create_item("file.rs", 10, DebtType::ComplexityHotspot);
/// let item2 = create_item("file.rs", 10, DebtType::ComplexityHotspot);
/// let item3 = create_item("file.rs", 10, DebtType::GodObject);
/// let item4 = create_item("file.rs", 20, DebtType::ComplexityHotspot);
///
/// assert!(is_duplicate_of(&item2, &item1)); // Same location, same type
/// assert!(!is_duplicate_of(&item3, &item1)); // Same location, different type
/// assert!(!is_duplicate_of(&item4, &item1)); // Different location
/// ```
#[inline]
pub fn is_duplicate_of(item: &UnifiedDebtItem, existing: &UnifiedDebtItem) -> bool {
    existing.location.file == item.location.file
        && existing.location.line == item.location.line
        && std::mem::discriminant(&existing.debt_type) == std::mem::discriminant(&item.debt_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        score_types::Score0To100, ActionableRecommendation, FunctionRole, ImpactMetrics, Location,
        UnifiedScore,
    };
    use std::path::PathBuf;

    fn create_test_item(
        score: f64,
        cyclomatic: u32,
        cognitive: u32,
        debt_type: DebtType,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_fn".to_string(),
                line: 10,
            },
            debt_type,
            unified_score: UnifiedScore {
                final_score: Score0To100::new(score),
                complexity_factor: 0.0,
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
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
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
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 10,
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
        }
    }

    #[test]
    fn score_threshold_filters_correctly() {
        let high = create_test_item(
            50.0,
            5,
            5,
            DebtType::ComplexityHotspot {
                cyclomatic: 5,
                cognitive: 5,
            },
        );
        let low = create_test_item(
            2.0,
            5,
            5,
            DebtType::ComplexityHotspot {
                cyclomatic: 5,
                cognitive: 5,
            },
        );

        assert!(meets_score_threshold(&high, 10.0));
        assert!(!meets_score_threshold(&low, 10.0));
    }

    #[test]
    fn risk_threshold_filters_risk_items() {
        let risk = create_test_item(
            10.0,
            5,
            5,
            DebtType::Risk {
                risk_score: 0.8,
                factors: vec!["test".to_string()],
            },
        );

        assert!(meets_risk_threshold(&risk, 0.5));
        assert!(!meets_risk_threshold(&risk, 0.9));
    }

    #[test]
    fn risk_threshold_passes_non_risk_items() {
        let normal = create_test_item(
            10.0,
            5,
            5,
            DebtType::ComplexityHotspot {
                cyclomatic: 5,
                cognitive: 5,
            },
        );

        assert!(meets_risk_threshold(&normal, 100.0)); // Always passes
    }

    #[test]
    fn god_objects_exempt_from_complexity() {
        let god_object = create_test_item(
            50.0,
            0, // Below threshold
            0, // Below threshold
            DebtType::GodObject {
                methods: 50,
                fields: Some(20),
                responsibilities: 10,
                god_object_score: Score0To100::new(85.0),
                lines: 500,
            },
        );

        assert!(is_exempt_from_complexity_filter(&god_object));
        assert!(meets_complexity_thresholds(&god_object, 2, 5));
    }

    #[test]
    fn test_types_exempt_from_complexity() {
        let test_item = create_test_item(
            20.0,
            1,
            1,
            DebtType::TestComplexityHotspot {
                cyclomatic: 1,
                cognitive: 1,
                threshold: 5,
            },
        );

        assert!(is_exempt_from_complexity_filter(&test_item));
        assert!(meets_complexity_thresholds(&test_item, 5, 10));
    }

    #[test]
    fn complexity_requires_both_thresholds() {
        let high_cyc = create_test_item(
            20.0,
            10,
            2,
            DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 2,
            },
        );
        let high_cog = create_test_item(
            20.0,
            2,
            10,
            DebtType::ComplexityHotspot {
                cyclomatic: 2,
                cognitive: 10,
            },
        );
        let both_high = create_test_item(
            20.0,
            10,
            10,
            DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 10,
            },
        );

        assert!(!meets_complexity_thresholds(&high_cyc, 5, 5)); // Cognitive too low
        assert!(!meets_complexity_thresholds(&high_cog, 5, 5)); // Cyclomatic too low
        assert!(meets_complexity_thresholds(&both_high, 5, 5)); // Both pass
    }

    #[test]
    fn duplicate_detection_checks_location_and_type() {
        let item1 = create_test_item(
            10.0,
            5,
            5,
            DebtType::ComplexityHotspot {
                cyclomatic: 5,
                cognitive: 5,
            },
        );
        let item2 = create_test_item(
            10.0,
            5,
            5,
            DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 10,
            },
        ); // Different values, same type
        let item3 = create_test_item(
            10.0,
            5,
            5,
            DebtType::GodObject {
                methods: 50,
                fields: Some(20),
                responsibilities: 10,
                god_object_score: Score0To100::new(85.0),
                lines: 500,
            },
        );

        assert!(is_duplicate_of(&item2, &item1)); // Same location + type discriminant
        assert!(!is_duplicate_of(&item3, &item1)); // Same location, different type
    }

    #[test]
    fn filter_statistics_calculates_totals() {
        let mut stats = FilterStatistics::new();
        stats.total_items_processed = 100;
        stats.filtered_by_score = 20;
        stats.filtered_by_complexity = 30;
        stats.filtered_as_duplicate = 10;
        stats.items_added = 40;

        assert_eq!(stats.total_filtered(), 60);
        assert_eq!(stats.acceptance_rate(), 40.0);
    }
}
