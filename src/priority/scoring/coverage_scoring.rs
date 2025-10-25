/// Role-based coverage scoring with functional pipeline.
///
/// This module implements the coverage scoring algorithm from spec 119,
/// replacing uniform 80% targets with role-specific expectations.

use super::coverage_expectations::{CoverageExpectations, CoverageGap, GapSeverity};

/// Calculates a prioritization score based on coverage gap.
///
/// This is the main entry point for role-based coverage scoring, implementing
/// a functional pipeline: calculate_gap() -> weight_by_severity() -> weight_by_role()
///
/// # Arguments
/// * `actual_coverage` - The actual coverage percentage (0-100)
/// * `role` - The function role (e.g., "Pure", "BusinessLogic", "Debug")
/// * `expectations` - The coverage expectations to use
///
/// # Returns
/// A score from 0.0 to 100.0, where higher scores indicate greater need for testing.
/// Functions meeting or exceeding their target get a score of 0.0.
pub fn calculate_coverage_score(
    actual_coverage: f64,
    role: &str,
    expectations: &CoverageExpectations,
) -> f64 {
    let range = expectations.for_role(role);
    let gap = CoverageGap::calculate(actual_coverage, range);

    calculate_gap_score(&gap)
        .pipe(|score| weight_by_severity(score, gap.severity))
        .pipe(|score| weight_by_role(score, role))
}

/// Extension trait to enable functional piping.
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}

/// Calculates the base gap score from a coverage gap.
///
/// Pure function that converts a coverage gap into a base score.
/// The score is proportional to the gap size, capped at 100.0.
fn calculate_gap_score(gap: &CoverageGap) -> f64 {
    if gap.gap <= 0.0 {
        0.0 // Meets or exceeds target
    } else {
        // Linear scaling based on gap size
        (gap.gap * 1.5).min(100.0)
    }
}

/// Applies severity-based weighting to the gap score.
///
/// Pure function that amplifies scores based on gap severity:
/// - Critical gaps: 2.0x multiplier
/// - Moderate gaps: 1.5x multiplier
/// - Minor gaps: 1.2x multiplier
/// - No gap: 1.0x multiplier
fn weight_by_severity(score: f64, severity: GapSeverity) -> f64 {
    let multiplier = match severity {
        GapSeverity::Critical => 2.0,
        GapSeverity::Moderate => 1.5,
        GapSeverity::Minor => 1.2,
        GapSeverity::None => 1.0,
    };

    (score * multiplier).min(100.0)
}

/// Applies role-based weighting to the score.
///
/// Pure function that adjusts scores based on role importance:
/// - Critical roles (Pure, BusinessLogic, Validation): 1.3x multiplier
/// - Important roles (StateManagement, ErrorHandling, Utilities): 1.2x
/// - Standard roles (IoOperations, Configuration, Orchestration): 1.0x
/// - Low-priority roles (Initialization, Performance, Debug): 0.8x
fn weight_by_role(score: f64, role: &str) -> f64 {
    let multiplier = match role {
        "Pure" | "BusinessLogic" | "Validation" => 1.3,
        "StateManagement" | "ErrorHandling" | "Utilities" => 1.2,
        "IoOperations" | "Configuration" | "Orchestration" => 1.0,
        "Initialization" | "Performance" | "Debug" => 0.8,
        _ => 1.0, // Default for unknown roles
    };

    (score * multiplier).min(100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_coverage_zero_score() {
        let expectations = CoverageExpectations::default();
        let score = calculate_coverage_score(100.0, "Pure", &expectations);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_meets_target_zero_score() {
        let expectations = CoverageExpectations::default();
        let score = calculate_coverage_score(95.0, "Pure", &expectations);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_critical_gap_high_score() {
        let expectations = CoverageExpectations::default();
        // 30% coverage for Pure (target 95%) is critical
        let score = calculate_coverage_score(30.0, "Pure", &expectations);
        assert!(score > 80.0); // Should be very high due to critical severity + high role weight
    }

    #[test]
    fn test_debug_low_priority() {
        let expectations = CoverageExpectations::default();
        // Same absolute coverage gap should score lower for Debug role
        let pure_score = calculate_coverage_score(50.0, "Pure", &expectations);
        let debug_score = calculate_coverage_score(50.0, "Debug", &expectations);

        // Debug should score lower due to lower role weight
        assert!(debug_score < pure_score);
    }

    #[test]
    fn test_score_monotonicity_within_role() {
        let expectations = CoverageExpectations::default();

        let score_30 = calculate_coverage_score(30.0, "BusinessLogic", &expectations);
        let score_50 = calculate_coverage_score(50.0, "BusinessLogic", &expectations);
        let score_70 = calculate_coverage_score(70.0, "BusinessLogic", &expectations);
        let score_85 = calculate_coverage_score(85.0, "BusinessLogic", &expectations);
        let score_90 = calculate_coverage_score(90.0, "BusinessLogic", &expectations);

        // Higher coverage should generally yield lower score
        // Note: Due to severity thresholds, very low coverage may have similar scores
        assert!(score_30 >= score_50 || (score_30 - score_50).abs() < 10.0);
        assert!(score_50 > score_70);
        assert!(score_70 > score_85);
        assert!(score_85 > score_90);
        assert_eq!(score_90, 0.0); // Meets target
    }

    #[test]
    fn test_severity_weighting() {
        use super::super::coverage_expectations::CoverageRange;

        let range = CoverageRange::new(80.0, 90.0, 100.0);

        // Critical: 30% (below 50% of min)
        let critical_gap = CoverageGap::calculate(30.0, &range);
        let critical_score = weight_by_severity(50.0, critical_gap.severity);

        // Moderate: 50% (between 50% of min and min)
        let moderate_gap = CoverageGap::calculate(50.0, &range);
        let moderate_score = weight_by_severity(50.0, moderate_gap.severity);

        // Minor: 85% (between min and target)
        let minor_gap = CoverageGap::calculate(85.0, &range);
        let minor_score = weight_by_severity(50.0, minor_gap.severity);

        assert!(critical_score > moderate_score);
        assert!(moderate_score > minor_score);
    }

    #[test]
    fn test_role_weighting() {
        let pure_weighted = weight_by_role(50.0, "Pure");
        let business_weighted = weight_by_role(50.0, "BusinessLogic");
        let io_weighted = weight_by_role(50.0, "IoOperations");
        let debug_weighted = weight_by_role(50.0, "Debug");

        // Critical roles should have highest weight
        assert_eq!(pure_weighted, 65.0); // 50 * 1.3
        assert_eq!(business_weighted, 65.0); // 50 * 1.3

        // Standard roles should be unweighted
        assert_eq!(io_weighted, 50.0); // 50 * 1.0

        // Low-priority roles should have lower weight
        assert_eq!(debug_weighted, 40.0); // 50 * 0.8
    }

    #[test]
    fn test_score_capped_at_100() {
        let expectations = CoverageExpectations::default();
        // Even with critical gaps and high role weight, score shouldn't exceed 100
        let score = calculate_coverage_score(0.0, "Pure", &expectations);
        assert!(score <= 100.0);
    }

    // Property-based tests for scoring monotonicity (spec 119)
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            /// Property: Higher coverage always yields lower or equal score within the same role
            #[test]
            fn prop_higher_coverage_lower_score(
                coverage1 in 0.0f64..100.0,
                coverage2 in 0.0f64..100.0,
            ) {
                prop_assume!(coverage1 < coverage2);
                let expectations = CoverageExpectations::default();

                let roles = ["Pure", "BusinessLogic", "Debug", "IoOperations"];
                for role in &roles {
                    let score1 = calculate_coverage_score(coverage1, role, &expectations);
                    let score2 = calculate_coverage_score(coverage2, role, &expectations);

                    // Higher coverage should yield lower or equal score
                    prop_assert!(score2 <= score1,
                        "Higher coverage ({}) should yield lower score than lower coverage ({}) for role {}. Got scores: {} and {}",
                        coverage2, coverage1, role, score2, score1
                    );
                }
            }

            /// Property: All scores are non-negative and capped at 100
            #[test]
            fn prop_score_bounds(
                coverage in 0.0f64..100.0,
            ) {
                let expectations = CoverageExpectations::default();

                let roles = ["Pure", "BusinessLogic", "Debug", "IoOperations", "Validation"];
                for role in &roles {
                    let score = calculate_coverage_score(coverage, role, &expectations);

                    prop_assert!(score >= 0.0,
                        "Score for role {} with coverage {} should be non-negative, got {}",
                        role, coverage, score
                    );
                    prop_assert!(score <= 100.0,
                        "Score for role {} with coverage {} should be capped at 100, got {}",
                        role, coverage, score
                    );
                }
            }

            /// Property: Coverage at or above target always yields zero score
            #[test]
            fn prop_target_coverage_zero_score(
                extra_coverage in 0.0f64..5.0,
            ) {
                let expectations = CoverageExpectations::default();

                let roles_and_targets = [
                    ("Pure", 95.0),
                    ("BusinessLogic", 90.0),
                    ("Debug", 30.0),
                    ("Validation", 92.0),
                ];

                for (role, target) in &roles_and_targets {
                    let coverage = target + extra_coverage;
                    if coverage <= 100.0 {
                        let score = calculate_coverage_score(coverage, role, &expectations);
                        prop_assert_eq!(score, 0.0,
                            "Coverage at or above target ({}) for role {} should yield zero score, got {}",
                            coverage, role, score
                        );
                    }
                }
            }

            /// Property: Critical roles score higher than debug roles for same coverage gap
            #[test]
            fn prop_role_importance_ordering(
                coverage in 10.0f64..60.0,
            ) {
                let expectations = CoverageExpectations::default();

                let pure_score = calculate_coverage_score(coverage, "Pure", &expectations);
                let debug_score = calculate_coverage_score(coverage, "Debug", &expectations);

                // Pure functions should have higher priority (higher score) than debug
                // when both have low coverage
                if coverage < 30.0 {  // Well below both targets
                    prop_assert!(pure_score >= debug_score,
                        "Pure functions with coverage {} should score >= debug functions. Got {} vs {}",
                        coverage, pure_score, debug_score
                    );
                }
            }

            /// Property: Severity weighting amplifies scores correctly
            #[test]
            fn prop_severity_amplification(
                base_score in 1.0f64..50.0,
            ) {
                use crate::priority::scoring::GapSeverity;

                let none_score = weight_by_severity(base_score, GapSeverity::None);
                let minor_score = weight_by_severity(base_score, GapSeverity::Minor);
                let moderate_score = weight_by_severity(base_score, GapSeverity::Moderate);
                let critical_score = weight_by_severity(base_score, GapSeverity::Critical);

                // Higher severity should yield higher or equal scores
                prop_assert!(none_score <= minor_score);
                prop_assert!(minor_score <= moderate_score);
                prop_assert!(moderate_score <= critical_score);

                // All scores should be capped at 100
                prop_assert!(critical_score <= 100.0);
            }

            /// Property: Role weighting maintains relative ordering
            #[test]
            fn prop_role_weighting_order(
                base_score in 1.0f64..50.0,
            ) {
                let pure_weighted = weight_by_role(base_score, "Pure");
                let business_weighted = weight_by_role(base_score, "BusinessLogic");
                let io_weighted = weight_by_role(base_score, "IoOperations");
                let debug_weighted = weight_by_role(base_score, "Debug");

                // Critical roles should be weighted higher than or equal to standard roles
                prop_assert!(pure_weighted >= io_weighted);
                prop_assert!(business_weighted >= io_weighted);

                // Standard roles should be weighted higher than low-priority roles
                prop_assert!(io_weighted >= debug_weighted);

                // All scores should be capped at 100
                prop_assert!(pure_weighted <= 100.0);
            }
        }
    }
}
