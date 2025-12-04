//! # God Object Scoring (Pure Core)
//!
//! Pure functions for calculating god object scores and weights.
//!
//! ## Stillwater Architecture
//!
//! This is part of the **Pure Core** - deterministic math with no side effects.
//! All functions are:
//! - Deterministic: Same inputs → same outputs
//! - Side-effect free: No I/O, no mutations
//! - Composable: Can be chained together
//! - 100% testable: No mocks needed

use super::thresholds::GodObjectThresholds;

/// Calculate god object score from method, field, responsibility counts, and LOC.
///
/// **Pure function** - deterministic, no side effects.
///
/// # Arguments
///
/// * `method_count` - Number of methods in the type
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `thresholds` - God object thresholds for the language
///
/// # Returns
///
/// God object score (0-100+). Scores >70 indicate definite god objects.
///
/// # Scoring Logic
///
/// The score is calculated based on:
/// - Method factor: ratio of methods to threshold (capped at 3.0)
/// - Field factor: ratio of fields to threshold (capped at 3.0)
/// - Responsibility factor: ratio of responsibilities to 3.0 (capped at 3.0)
/// - Size factor: ratio of LOC to threshold (capped at 3.0)
///
/// Violation-based scaling:
/// - 1 violation: minimum score 30.0
/// - 2 violations: minimum score 50.0
/// - 3+ violations: minimum score 70.0
pub fn calculate_god_object_score(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    thresholds: &GodObjectThresholds,
) -> f64 {
    let method_factor = (method_count as f64 / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // Calculate violation count for minimum score determination
    let mut violation_count = 0;
    if method_count > thresholds.max_methods {
        violation_count += 1;
    }
    if field_count > thresholds.max_fields {
        violation_count += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violation_count += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violation_count += 1;
    }

    // Exponential scaling for severe violations
    let base_score = method_factor * field_factor * responsibility_factor * size_factor;

    // Apply appropriate scoring based on violation severity
    // More nuanced approach to prevent over-flagging moderate files
    if violation_count > 0 {
        // Graduated minimum scores based on violation count
        let base_min_score = match violation_count {
            1 => 30.0, // Single violation: Moderate score
            2 => 50.0, // Two violations: Borderline CRITICAL
            _ => 70.0, // Three+ violations: Likely CRITICAL
        };

        // Reduced multiplier from 50.0 to 20.0 for more conservative scoring
        let score = base_score * 20.0 * (violation_count as f64);
        score.max(base_min_score)
    } else {
        base_score * 10.0
    }
}

/// Calculate complexity-weighted god object score.
///
/// **Pure function** - deterministic, no side effects.
///
/// Unlike raw method counting, this function weights each method by its
/// cyclomatic complexity, ensuring that 100 simple functions (complexity 1-3)
/// score better than 10 complex functions (complexity 17+).
///
/// # Arguments
///
/// * `weighted_method_count` - Sum of complexity weights for all functions
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `avg_complexity` - Average cyclomatic complexity across functions
/// * `thresholds` - God object thresholds for the language
///
/// # Returns
///
/// God object score (0-100+). Scores >70 indicate definite god objects.
///
/// # Complexity Factors
///
/// - Low complexity (< 3.0): 0.7x multiplier (reward simple functions)
/// - Medium complexity (3.0-10.0): 1.0x multiplier (neutral)
/// - High complexity (> 10.0): 1.5x multiplier (penalize complex functions)
pub fn calculate_god_object_score_weighted(
    weighted_method_count: f64,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    avg_complexity: f64,
    thresholds: &GodObjectThresholds,
) -> f64 {
    // Use weighted count instead of raw count
    let method_factor = (weighted_method_count / thresholds.max_methods as f64).min(3.0);
    let field_factor = (field_count as f64 / thresholds.max_fields as f64).min(3.0);
    let responsibility_factor = (responsibility_count as f64 / 3.0).min(3.0);
    let size_factor = (lines_of_code as f64 / thresholds.max_lines as f64).min(3.0);

    // Add complexity bonus/penalty
    let complexity_factor = if avg_complexity < 3.0 {
        0.7 // Reward simple functions
    } else if avg_complexity > 10.0 {
        1.5 // Penalize complex functions
    } else {
        1.0
    };

    // Calculate violation count for minimum score determination
    let mut violation_count = 0;
    if weighted_method_count > thresholds.max_methods as f64 {
        violation_count += 1;
    }
    if field_count > thresholds.max_fields {
        violation_count += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violation_count += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violation_count += 1;
    }

    // Exponential scaling for severe violations
    let base_score = method_factor * field_factor * responsibility_factor * size_factor;

    // Apply complexity factor and ensure appropriate score for violations
    // Scale scores more conservatively to prevent small files from being CRITICAL
    if violation_count > 0 {
        // More nuanced minimum scores based on violation severity
        // 1 violation (e.g., just responsibilities): 30-50 range
        // 2 violations: 50-70 range
        // 3+ violations: 70+ range (CRITICAL territory)
        let base_min_score = match violation_count {
            1 => 30.0, // Moderate threshold - won't trigger CRITICAL (< 50)
            2 => 50.0, // High threshold - borderline CRITICAL
            _ => 70.0, // Multiple violations - likely CRITICAL
        };

        // Reduced multiplier from 50.0 to 20.0 for more conservative scoring
        let score = base_score * 20.0 * complexity_factor * (violation_count as f64);
        score.max(base_min_score)
    } else {
        base_score * 10.0 * complexity_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scoring_deterministic() {
        let thresholds = GodObjectThresholds::default();
        let score1 = calculate_god_object_score(20, 15, 5, 500, &thresholds);
        let score2 = calculate_god_object_score(20, 15, 5, 500, &thresholds);
        assert_eq!(score1, score2);
    }

    #[test]
    fn test_weighted_scoring_deterministic() {
        let thresholds = GodObjectThresholds::default();
        let score1 = calculate_god_object_score_weighted(25.0, 15, 5, 500, 5.0, &thresholds);
        let score2 = calculate_god_object_score_weighted(25.0, 15, 5, 500, 5.0, &thresholds);
        assert_eq!(score1, score2);
    }

    #[test]
    fn test_scoring_zero_methods() {
        let thresholds = GodObjectThresholds::default();
        let score = calculate_god_object_score(0, 0, 0, 0, &thresholds);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_scoring_zero_responsibilities() {
        let thresholds = GodObjectThresholds::default();
        let score = calculate_god_object_score(10, 5, 0, 100, &thresholds);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_scoring_threshold_boundary() {
        let thresholds = GodObjectThresholds::default();
        // Exactly at threshold should not trigger violation
        let score = calculate_god_object_score(
            thresholds.max_methods,
            thresholds.max_fields,
            thresholds.max_traits,
            thresholds.max_lines,
            &thresholds,
        );
        // Should use non-violation scoring (multiplier of 10.0)
        assert!(score < 30.0); // Less than minimum violation score
    }

    #[test]
    fn test_scoring_single_violation() {
        let thresholds = GodObjectThresholds::default();
        // Just over method threshold
        let score = calculate_god_object_score(
            thresholds.max_methods + 1,
            thresholds.max_fields,
            thresholds.max_traits,
            thresholds.max_lines,
            &thresholds,
        );
        // Should have minimum score of 30.0 for single violation
        assert!(score >= 30.0);
    }

    #[test]
    fn test_scoring_multiple_violations() {
        let thresholds = GodObjectThresholds::default();
        // Three violations
        let score = calculate_god_object_score(
            thresholds.max_methods + 10,
            thresholds.max_fields + 10,
            thresholds.max_traits + 1,
            thresholds.max_lines,
            &thresholds,
        );
        // Should have minimum score of 70.0 for 3+ violations
        assert!(score >= 70.0);
    }

    #[test]
    fn test_weighted_vs_unweighted_consistency() {
        let thresholds = GodObjectThresholds::default();
        // When weighted_count == method_count and avg_complexity is neutral
        let method_count = 20;
        let field_count = 15;
        let resp_count = 5;
        let loc = 500;

        let unweighted =
            calculate_god_object_score(method_count, field_count, resp_count, loc, &thresholds);
        let weighted = calculate_god_object_score_weighted(
            method_count as f64,
            field_count,
            resp_count,
            loc,
            5.0, // Medium complexity
            &thresholds,
        );

        // Should be equal when complexity factor is 1.0
        assert_eq!(unweighted, weighted);
    }

    #[test]
    fn test_weighted_low_complexity_bonus() {
        let thresholds = GodObjectThresholds::default();
        let normal = calculate_god_object_score_weighted(20.0, 15, 5, 500, 5.0, &thresholds);
        let low_complexity =
            calculate_god_object_score_weighted(20.0, 15, 5, 500, 2.0, &thresholds);

        // Low complexity should score lower (better)
        assert!(low_complexity < normal);
    }

    #[test]
    fn test_weighted_high_complexity_penalty() {
        let thresholds = GodObjectThresholds::default();
        let normal = calculate_god_object_score_weighted(20.0, 15, 5, 500, 5.0, &thresholds);
        let high_complexity =
            calculate_god_object_score_weighted(20.0, 15, 5, 500, 15.0, &thresholds);

        // High complexity should score higher (worse)
        assert!(high_complexity > normal);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn score_never_negative(
            method_count in 0..1000usize,
            field_count in 0..200usize,
            resp_count in 0..100usize,
            loc in 0..10000usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let score = calculate_god_object_score(
                method_count,
                field_count,
                resp_count,
                loc,
                &thresholds
            );
            prop_assert!(score >= 0.0);
        }

        #[test]
        fn weighted_score_never_negative(
            weighted_count in 0.0..1000.0f64,
            field_count in 0..200usize,
            resp_count in 0..100usize,
            loc in 0..10000usize,
            avg_complexity in 1.0..30.0f64
        ) {
            let thresholds = GodObjectThresholds::default();
            let score = calculate_god_object_score_weighted(
                weighted_count,
                field_count,
                resp_count,
                loc,
                avg_complexity,
                &thresholds
            );
            prop_assert!(score >= 0.0);
        }

        #[test]
        fn score_monotonic_in_methods(
            base in 10..100usize,
            delta in 1..50usize,
            field_count in 5..50usize,
            resp_count in 1..10usize,
            loc in 100..1000usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let score1 = calculate_god_object_score(base, field_count, resp_count, loc, &thresholds);
            let score2 = calculate_god_object_score(base + delta, field_count, resp_count, loc, &thresholds);
            prop_assert!(score2 >= score1);
        }

        #[test]
        fn score_monotonic_in_fields(
            method_count in 10..100usize,
            base in 5..50usize,
            delta in 1..20usize,
            resp_count in 1..10usize,
            loc in 100..1000usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let score1 = calculate_god_object_score(method_count, base, resp_count, loc, &thresholds);
            let score2 = calculate_god_object_score(method_count, base + delta, resp_count, loc, &thresholds);
            prop_assert!(score2 >= score1);
        }

        #[test]
        fn score_monotonic_in_responsibilities(
            method_count in 10..100usize,
            field_count in 5..50usize,
            base in 1..10usize,
            delta in 1..5usize,
            loc in 100..1000usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let score1 = calculate_god_object_score(method_count, field_count, base, loc, &thresholds);
            let score2 = calculate_god_object_score(method_count, field_count, base + delta, loc, &thresholds);
            prop_assert!(score2 >= score1);
        }

        #[test]
        fn weighted_score_reasonable_bounds(
            weighted_count in 1.0..500.0f64,
            field_count in 1..100usize,
            resp_count in 1..20usize,
            loc in 100..5000usize,
            avg_complexity in 1.0..20.0f64
        ) {
            let thresholds = GodObjectThresholds::default();
            let score = calculate_god_object_score_weighted(
                weighted_count,
                field_count,
                resp_count,
                loc,
                avg_complexity,
                &thresholds
            );
            // Score should be reasonable (not infinity, not NaN)
            prop_assert!(score.is_finite());
            // Score should be positive
            prop_assert!(score >= 0.0);
        }

        #[test]
        fn complexity_factor_affects_score(
            weighted_count in 20.0..100.0f64,
            field_count in 10..50usize,
            resp_count in 3..10usize,
            loc in 500..2000usize
        ) {
            let thresholds = GodObjectThresholds::default();
            let low_complexity = calculate_god_object_score_weighted(
                weighted_count,
                field_count,
                resp_count,
                loc,
                2.0,
                &thresholds
            );
            let high_complexity = calculate_god_object_score_weighted(
                weighted_count,
                field_count,
                resp_count,
                loc,
                15.0,
                &thresholds
            );
            // High complexity should always score worse (higher) than low complexity
            prop_assert!(high_complexity > low_complexity);
        }
    }
}
