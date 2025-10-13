// Unified test calculation module - single source of truth for test count recommendations

/// Complexity tier determines which formula to use for test calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityTier {
    Simple,   // cyclo ≤ 10
    Moderate, // 10 < cyclo ≤ 30
    High,     // 30 < cyclo ≤ 50
    Extreme,  // cyclo > 50
}

impl ComplexityTier {
    /// Determine complexity tier from cyclomatic complexity value
    pub fn from_cyclomatic(cyclo: u32) -> Self {
        match cyclo {
            0..=10 => ComplexityTier::Simple,
            11..=30 => ComplexityTier::Moderate,
            31..=50 => ComplexityTier::High,
            _ => ComplexityTier::Extreme,
        }
    }
}

/// Test recommendation with calculation audit trail
#[derive(Debug, Clone, PartialEq)]
pub struct TestRecommendation {
    /// Number of tests needed to close coverage gap
    pub count: u32,

    /// Formula used for calculation (for transparency)
    pub formula_used: String,

    /// Human-readable rationale
    pub rationale: String,
}

/// Calculate number of tests needed to achieve full coverage
///
/// # Formula Selection
///
/// This function uses different formulas based on cyclomatic complexity:
///
/// - **Simple (≤10)**: Linear relationship
///   - Formula: `cyclomatic × coverage_gap`
///   - Rationale: Each test typically covers one execution path
///   - Example: cyclo=5, gap=40% → 2 tests
///
/// - **Moderate (11-30)**: Square root relationship
///   - Formula: `sqrt(cyclomatic) × 1.5 + 2`
///   - Rationale: Tests cover overlapping paths through shared conditions
///   - Example: cyclo=20, gap=50% → 4-5 tests
///
/// - **High (31-50)**: Linear (conservative)
///   - Formula: `cyclomatic × coverage_gap`
///   - Rationale: Complexity too high for path overlap assumptions
///   - Example: cyclo=33, gap=34% → 11 tests
///
/// - **Extreme (>50)**: Property-based testing
///   - Formula: `sqrt(cyclomatic) × 1.5 + 2 + 3 property suites`
///   - Rationale: Manual path testing becomes intractable
///   - Example: cyclo=60 → recommend proptest/quickcheck
///
/// # Research Basis
///
/// Formulas based on:
/// - McCabe, T. J. (1976). "A Complexity Measure"
/// - Myers, G. (2004). "The Art of Software Testing"
/// - Empirical analysis of debtmap's own test suite
///
/// # Examples
///
/// ```rust
/// use debtmap::priority::scoring::test_calculation::{calculate_tests_needed, ComplexityTier};
///
/// // Simple function
/// let result = calculate_tests_needed(5, 0.6, None);
/// assert_eq!(result.count, 2);
///
/// // Complex function (the bug case from spec 109)
/// let result = calculate_tests_needed(33, 0.661, None);
/// assert_eq!(result.count, 12);  // Not 3! (ceil(33 × 0.339) = 12)
/// ```
pub fn calculate_tests_needed(
    cyclomatic: u32,
    coverage_percent: f64,
    tier: Option<ComplexityTier>,
) -> TestRecommendation {
    let tier = tier.unwrap_or_else(|| ComplexityTier::from_cyclomatic(cyclomatic));
    let coverage_gap = 1.0 - coverage_percent;

    // Fully covered functions need no additional tests
    if coverage_percent >= 1.0 {
        return TestRecommendation {
            count: 0,
            formula_used: "fully_covered".to_string(),
            rationale: "Function has full coverage".to_string(),
        };
    }

    let (count, formula, rationale) = match tier {
        ComplexityTier::Simple => {
            // Linear: each test typically covers one path
            let tests = (cyclomatic as f64 * coverage_gap).ceil() as u32;
            let tests = tests.max(2); // Minimum 2 tests (happy path + edge case)
            (
                tests,
                format!(
                    "cyclomatic × coverage_gap = {} × {:.2} = {}",
                    cyclomatic, coverage_gap, tests
                ),
                "Simple functions: one test per execution path".to_string(),
            )
        }

        ComplexityTier::Moderate => {
            // Square root: tests cover multiple overlapping paths
            let ideal_tests = (cyclomatic as f64).sqrt() * 1.5 + 2.0;
            let current_tests = ideal_tests * coverage_percent;
            let needed = (ideal_tests - current_tests).ceil() as u32;
            (
                needed,
                format!(
                    "sqrt(cyclo) × 1.5 + 2 - current = sqrt({}) × 1.5 + 2 - {:.1} = {}",
                    cyclomatic, current_tests, needed
                ),
                "Moderate functions: tests cover overlapping paths via shared conditions"
                    .to_string(),
            )
        }

        ComplexityTier::High => {
            // Linear (conservative): complexity too high for overlap assumptions
            // This fixes the bug from spec 109 where cyclo=33 should produce 11 tests, not 3
            let tests = (cyclomatic as f64 * coverage_gap).ceil() as u32;
            let tests = tests.max(3); // Minimum 3 tests for high complexity
            (
                tests,
                format!(
                    "cyclomatic × coverage_gap = {} × {:.2} = {}",
                    cyclomatic, coverage_gap, tests
                ),
                "High complexity: linear formula (conservative approach for independent paths)"
                    .to_string(),
            )
        }

        ComplexityTier::Extreme => {
            // For extreme complexity, suggest property-based testing
            let structural_tests = ((cyclomatic as f64).sqrt() * 1.5 + 2.0).ceil() as u32;
            let property_tests = 3; // Recommend 3 property-based test suites
            (
                structural_tests + property_tests,
                format!(
                    "{} structural + {} property-based test suites",
                    structural_tests, property_tests
                ),
                "Extreme complexity: combine structural and property-based testing".to_string(),
            )
        }
    };

    TestRecommendation {
        count,
        formula_used: formula,
        rationale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_tier_from_cyclomatic() {
        assert_eq!(ComplexityTier::from_cyclomatic(5), ComplexityTier::Simple);
        assert_eq!(ComplexityTier::from_cyclomatic(10), ComplexityTier::Simple);
        assert_eq!(
            ComplexityTier::from_cyclomatic(11),
            ComplexityTier::Moderate
        );
        assert_eq!(
            ComplexityTier::from_cyclomatic(30),
            ComplexityTier::Moderate
        );
        assert_eq!(ComplexityTier::from_cyclomatic(31), ComplexityTier::High);
        assert_eq!(ComplexityTier::from_cyclomatic(50), ComplexityTier::High);
        assert_eq!(ComplexityTier::from_cyclomatic(51), ComplexityTier::Extreme);
    }

    #[test]
    fn test_simple_function_linear_calculation() {
        // Simple function: cyclo=5, coverage=60%
        let result = calculate_tests_needed(5, 0.6, None);
        assert_eq!(result.count, 2); // ceil(5 × 0.4) = 2
        assert!(result.formula_used.contains("cyclomatic × coverage_gap"));
        assert!(result.rationale.contains("Simple functions"));
    }

    #[test]
    fn test_moderate_function_sqrt_calculation() {
        // Moderate function: cyclo=20, coverage=50%
        let result = calculate_tests_needed(20, 0.5, None);
        // sqrt(20) × 1.5 + 2 = 8.7, half covered (4.35), need ~4
        assert!(result.count >= 4 && result.count <= 5);
        assert!(result.formula_used.contains("sqrt"));
        assert!(result.rationale.contains("Moderate functions"));
    }

    #[test]
    fn test_extreme_complexity_case_cyclo_33() {
        // The bug case from spec 109: cyclo=33, coverage=66.1%
        let result = calculate_tests_needed(33, 0.661, None);

        // With High tier using linear formula:
        // 33 × (1 - 0.661) = 33 × 0.339 = 11.187 → ceil() = 12 tests
        // Note: The original bug showed "3 tests" which was wrong
        // The spec estimated 11, but mathematically ceil(11.187) = 12
        assert_eq!(
            result.count, 12,
            "Bug fix: cyclo=33 with 66.1% coverage should need ~11-12 tests, not 3"
        );
        assert!(result.formula_used.contains("cyclomatic × coverage_gap"));
        assert!(result.rationale.contains("High complexity"));
    }

    #[test]
    fn test_all_tiers_produce_consistent_results() {
        let test_cases = vec![
            (5, 0.8, ComplexityTier::Simple),
            (15, 0.6, ComplexityTier::Moderate),
            (33, 0.661, ComplexityTier::High),
            (60, 0.5, ComplexityTier::Extreme),
        ];

        for (cyclo, coverage, tier) in test_cases {
            let result1 = calculate_tests_needed(cyclo, coverage, Some(tier));
            let result2 = calculate_tests_needed(cyclo, coverage, Some(tier));

            assert_eq!(
                result1.count, result2.count,
                "Non-deterministic calculation for cyclo={}",
                cyclo
            );
            assert_eq!(result1.formula_used, result2.formula_used);
            assert_eq!(result1.rationale, result2.rationale);
        }
    }

    #[test]
    fn test_full_coverage_returns_zero() {
        let result = calculate_tests_needed(20, 1.0, None);
        assert_eq!(result.count, 0);
        assert_eq!(result.formula_used, "fully_covered");
        assert!(result.rationale.contains("full coverage"));
    }

    #[test]
    fn test_minimum_two_tests_for_simple() {
        let result = calculate_tests_needed(2, 0.0, Some(ComplexityTier::Simple));
        assert!(
            result.count >= 2,
            "Should always recommend at least 2 tests for simple functions"
        );
    }

    #[test]
    fn test_zero_coverage_simple() {
        let result = calculate_tests_needed(5, 0.0, None);
        assert_eq!(result.count, 5); // 5 × 1.0 = 5
    }

    #[test]
    fn test_zero_coverage_moderate() {
        let result = calculate_tests_needed(20, 0.0, None);
        // sqrt(20) × 1.5 + 2 = 8.7 → 9
        assert!(result.count >= 8 && result.count <= 9);
    }

    #[test]
    fn test_zero_coverage_high() {
        let result = calculate_tests_needed(35, 0.0, None);
        // Linear: 35 × 1.0 = 35
        assert_eq!(result.count, 35);
    }

    #[test]
    fn test_extreme_complexity_recommends_property_testing() {
        let result = calculate_tests_needed(60, 0.5, None);
        assert!(result.rationale.contains("property-based"));
        assert!(result.formula_used.contains("property-based test suites"));
    }

    #[test]
    fn test_boundary_at_tier_transitions() {
        // Test at boundary of Simple -> Moderate (cyclo = 10 vs 11)
        let simple_10 = calculate_tests_needed(10, 0.5, None);
        let moderate_11 = calculate_tests_needed(11, 0.5, None);

        // Simple uses linear (10 × 0.5 = 5)
        assert_eq!(simple_10.count, 5);

        // Moderate uses sqrt (sqrt(11) × 1.5 + 2 ≈ 6.97, half covered ≈ 3.5, need ~3-4)
        assert!(moderate_11.count >= 3 && moderate_11.count <= 4);

        // Test at boundary of Moderate -> High (cyclo = 30 vs 31)
        let moderate_30 = calculate_tests_needed(30, 0.5, None);
        let high_31 = calculate_tests_needed(31, 0.5, None);

        // Moderate uses sqrt (sqrt(30) × 1.5 + 2 ≈ 10.2, half covered ≈ 5.1, need ~5)
        assert!(moderate_30.count >= 4 && moderate_30.count <= 6);

        // High uses linear (31 × 0.5 = 15.5 → 16)
        assert_eq!(high_31.count, 16);
    }

    #[test]
    fn test_high_coverage_small_gap() {
        // High coverage (95%) with moderate complexity
        let result = calculate_tests_needed(20, 0.95, None);
        // sqrt(20) × 1.5 + 2 ≈ 8.7 total, 95% covered ≈ 8.3, need ~0-1
        assert!(result.count <= 1);
    }
}
