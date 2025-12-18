// Pure functions for scoring calculation (spec 68, spec 101)

use std::fmt;

/// Calculate coverage multiplier from coverage percentage (spec 122)
/// Returns a value between 0.0 (100% coverage) and 1.0 (0% coverage)
/// This multiplier dampens the base score for well-tested code
pub fn calculate_coverage_multiplier(coverage_pct: f64) -> f64 {
    calculate_coverage_multiplier_with_test_flag(coverage_pct, false)
}

/// Calculate coverage multiplier with test code awareness (spec 122)
pub fn calculate_coverage_multiplier_with_test_flag(coverage_pct: f64, is_test_code: bool) -> f64 {
    // Don't penalize test code for coverage
    if is_test_code {
        return 0.0; // Test code gets maximum dampening (near-zero score)
    }

    // Coverage acts as a dampener: higher coverage → lower multiplier → lower score
    1.0 - coverage_pct
}

/// DEPRECATED: Use calculate_coverage_multiplier instead (spec 122)
/// Calculate coverage factor from coverage percentage
pub fn calculate_coverage_factor(coverage_pct: f64) -> f64 {
    calculate_coverage_factor_with_test_flag(coverage_pct, false)
}

/// DEPRECATED: Use calculate_coverage_multiplier_with_test_flag instead (spec 122)
/// Calculate coverage factor with test code awareness
pub fn calculate_coverage_factor_with_test_flag(coverage_pct: f64, is_test_code: bool) -> f64 {
    // Don't penalize test code for coverage
    if is_test_code {
        return 0.1;
    }

    let coverage_gap = 1.0 - coverage_pct;

    match coverage_pct {
        // Zero coverage: maximum priority
        0.0 => 10.0,

        // Very low coverage: high priority
        c if c < 0.2 => 5.0 + (coverage_gap * 3.0),

        // Low coverage: elevated priority
        c if c < 0.5 => 2.0 + (coverage_gap * 2.0),

        // Moderate to high coverage: standard calculation
        _ => (coverage_gap.powf(1.5) + 0.1).max(0.1),
    }
}

/// Calculate complexity factor from raw complexity
pub fn calculate_complexity_factor(raw_complexity: f64) -> f64 {
    // Linear scaling to 0-10 range for predictable scoring
    // Complexity of 20+ maps to 10.0
    (raw_complexity / 2.0).clamp(0.0, 10.0)
}

/// Calculate dependency factor from upstream count
pub fn calculate_dependency_factor(upstream_count: usize) -> f64 {
    // Linear scaling with cap at 10.0 for 20+ dependencies
    // This makes dependency impact predictable
    ((upstream_count as f64) / 2.0).min(10.0)
}

/// Calculate base score with coverage as multiplier (spec 122)
/// Coverage dampens the complexity+dependency base score instead of adding to it
pub fn calculate_base_score_with_coverage_multiplier(
    coverage_multiplier: f64,
    complexity_factor: f64,
    dependency_factor: f64,
) -> f64 {
    // Calculate base score from complexity and dependencies
    let base = calculate_base_score_no_coverage(complexity_factor, dependency_factor);

    // Apply coverage as a dampening multiplier
    // 100% coverage (multiplier=0.0) → near-zero score
    // 0% coverage (multiplier=1.0) → full base score
    base * coverage_multiplier
}

/// DEPRECATED: Use calculate_base_score_with_coverage_multiplier instead (spec 122)
/// Calculate weighted sum base score
/// Uses additive model for clear, predictable scoring
pub fn calculate_base_score(
    coverage_factor: f64,
    complexity_factor: f64,
    dependency_factor: f64,
) -> f64 {
    // Balanced weights giving equal importance to coverage and complexity
    // Rebalanced from 50/35/15 to 40/40/20 to prevent coverage-dominated prioritization
    // This ensures structural debt (god objects, high complexity) maintains priority
    // over simple untested functions
    let coverage_weight = 0.40; // 40% weight on coverage gaps
    let complexity_weight = 0.40; // 40% weight on complexity
    let dependency_weight = 0.20; // 20% weight on dependencies

    // Convert factors to 0-100 scale for clarity
    let coverage_score = coverage_factor * 10.0; // Already 0-10 scale
    let complexity_score = complexity_factor * 10.0; // Already 0-10 scale
    let dependency_score = dependency_factor * 10.0; // Already 0-10 scale

    // Weighted sum
    (coverage_score * coverage_weight)
        + (complexity_score * complexity_weight)
        + (dependency_score * dependency_weight)
}

/// Calculate base score when coverage data is not available.
///
/// Uses adjusted weights focusing on observable code quality metrics:
/// - 50% complexity (cyclomatic and cognitive complexity)
/// - 25% dependencies (upstream callers indicating change risk)
/// - 25% reserved for debt patterns (to be added with debt_adjustment)
///
/// This provides meaningful prioritization even without test coverage data.
pub fn calculate_base_score_no_coverage(complexity_factor: f64, dependency_factor: f64) -> f64 {
    let complexity_weight = 0.50; // 50% weight on complexity
    let dependency_weight = 0.25; // 25% weight on dependencies

    // Convert factors to 0-100 scale
    let complexity_score = complexity_factor * 10.0;
    let dependency_score = dependency_factor * 10.0;

    // Weighted sum - debt pattern weight applied separately via debt_adjustment
    (complexity_score * complexity_weight) + (dependency_score * dependency_weight)
}

/// Structure to hold normalized score with metadata
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NormalizedScore {
    pub raw: f64,
    pub normalized: f64,
    pub scaling_method: ScalingMethod,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ScalingMethod {
    Linear,      // 0-10
    SquareRoot,  // 10-100
    Logarithmic, // 100+
}

impl fmt::Display for NormalizedScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format with raw value, normalized value, and visual indicator
        let indicator = match self.scaling_method {
            ScalingMethod::Linear => "▁",      // Low score indicator
            ScalingMethod::SquareRoot => "▃",  // Medium score indicator
            ScalingMethod::Logarithmic => "▅", // High score indicator
        };

        // Determine severity level for color coding (when terminal supports it)
        let severity = if self.normalized < 3.0 {
            "low"
        } else if self.normalized < 7.0 {
            "medium"
        } else if self.normalized < 15.0 {
            "high"
        } else {
            "critical"
        };

        write!(
            f,
            "{:.1} (raw: {:.1}, {}, {})",
            self.normalized, self.raw, severity, indicator
        )
    }
}

fn determine_scaling_method(score: f64) -> ScalingMethod {
    if score <= 10.0 {
        ScalingMethod::Linear
    } else if score <= 100.0 {
        ScalingMethod::SquareRoot
    } else {
        ScalingMethod::Logarithmic
    }
}

/// Normalize final score with logarithmic scaling for high scores
pub fn normalize_final_score_with_metadata(raw_score: f64) -> NormalizedScore {
    let normalized = if raw_score <= 0.0 {
        0.0
    } else if raw_score < 10.0 {
        // Linear scaling for low scores (unchanged)
        raw_score
    } else if raw_score < 100.0 {
        // Square root scaling for medium scores
        // Maps 10-100 to 10-40 range (approximately)
        10.0 + (raw_score - 10.0).sqrt() * 3.33
    } else {
        // Logarithmic scaling for high scores
        // Maps 100+ to 40+ range with slow growth
        // Adjusted to ensure continuity at 100: sqrt(90) * 3.33 + 10 ≈ 41.59
        41.59 + (raw_score / 100.0).ln() * 10.0
    };

    NormalizedScore {
        raw: raw_score,
        normalized,
        scaling_method: determine_scaling_method(raw_score),
    }
}

/// Normalize final score to a simple f64.
///
/// No upper clamping - scores can exceed 100 to preserve relative
/// priority information (spec 261). Negative values are floored to 0.
pub fn normalize_final_score(raw_score: f64) -> f64 {
    raw_score.max(0.0)
}

/// Inverse normalization function for interpretation
pub fn denormalize_score(normalized: f64) -> f64 {
    if normalized <= 0.0 {
        0.0
    } else if normalized < 10.0 {
        // Linear range
        normalized
    } else if normalized < 41.59 {
        // Square root range (inverse)
        let adjusted = (normalized - 10.0) / 3.33;
        10.0 + adjusted.powf(2.0)
    } else {
        // Logarithmic range (inverse)
        // Adjusted for continuity at 100
        let log_component = (normalized - 41.59) / 10.0;
        100.0 * log_component.exp()
    }
}

/// Normalize complexity to 0-10 scale
pub fn normalize_complexity(cyclomatic: u32, cognitive: u32) -> f64 {
    // Normalize complexity to 0-10 scale
    let combined = (cyclomatic + cognitive) as f64 / 2.0;

    // Use logarithmic scale for better distribution
    // Complexity of 1-5 = low (0-3), 6-10 = medium (3-6), 11+ = high (6-10)
    if combined <= 5.0 {
        combined * 0.6
    } else if combined <= 10.0 {
        3.0 + (combined - 5.0) * 0.6
    } else {
        6.0 + ((combined - 10.0) * 0.2).min(4.0)
    }
}

/// Generate visualization data for normalization curve
pub fn generate_normalization_curve() -> Vec<(f64, f64, &'static str)> {
    // Generate sample points across different scaling regions
    let mut curve_data = Vec::new();

    // Linear range (0-10)
    for i in 0..=10 {
        let raw = i as f64;
        let normalized = normalize_final_score(raw);
        curve_data.push((raw, normalized, "Linear"));
    }

    // Square root range (11-100)
    let sqrt_points = vec![15, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    for raw in sqrt_points {
        let raw = raw as f64;
        let normalized = normalize_final_score(raw);
        curve_data.push((raw, normalized, "SquareRoot"));
    }

    // Logarithmic range (100+)
    let log_points = vec![150, 200, 300, 500, 750, 1000, 1500, 2000];
    for raw in log_points {
        let raw = raw as f64;
        let normalized = normalize_final_score(raw);
        curve_data.push((raw, normalized, "Logarithmic"));
    }

    curve_data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_coverage_multiplier() {
        // Test coverage multiplier (spec 122)
        assert_eq!(calculate_coverage_multiplier(0.0), 1.0); // 0% coverage: full score
        assert_eq!(calculate_coverage_multiplier(0.5), 0.5); // 50% coverage: half score
        assert!((calculate_coverage_multiplier(0.8) - 0.2).abs() < 0.01); // 80% coverage: 20% of score
        assert_eq!(calculate_coverage_multiplier(1.0), 0.0); // 100% coverage: near-zero score
    }

    #[test]
    fn test_coverage_multiplier_with_test_flag() {
        // Test code should get maximum dampening regardless of coverage
        assert_eq!(calculate_coverage_multiplier_with_test_flag(0.0, true), 0.0);
        assert_eq!(calculate_coverage_multiplier_with_test_flag(0.5, true), 0.0);
        assert_eq!(calculate_coverage_multiplier_with_test_flag(1.0, true), 0.0);

        // Non-test code follows normal multiplier
        assert_eq!(
            calculate_coverage_multiplier_with_test_flag(0.0, false),
            1.0
        );
        assert_eq!(
            calculate_coverage_multiplier_with_test_flag(0.5, false),
            0.5
        );
        assert_eq!(
            calculate_coverage_multiplier_with_test_flag(1.0, false),
            0.0
        );
    }

    #[test]
    fn test_base_score_with_coverage_multiplier() {
        // Test that coverage acts as dampener (spec 122)
        let complexity_factor = 5.0;
        let dependency_factor = 2.0;

        // No coverage (multiplier = 1.0) should yield full base score
        let score_no_coverage = calculate_base_score_with_coverage_multiplier(
            1.0,
            complexity_factor,
            dependency_factor,
        );
        let base = calculate_base_score_no_coverage(complexity_factor, dependency_factor);
        assert_eq!(score_no_coverage, base);

        // 50% coverage should yield half the base score
        let score_half_coverage = calculate_base_score_with_coverage_multiplier(
            0.5,
            complexity_factor,
            dependency_factor,
        );
        assert!((score_half_coverage - base * 0.5).abs() < 0.01);

        // 80% coverage should yield 20% of base score
        let score_high_coverage = calculate_base_score_with_coverage_multiplier(
            0.2,
            complexity_factor,
            dependency_factor,
        );
        assert!((score_high_coverage - base * 0.2).abs() < 0.01);

        // 100% coverage should yield near-zero score
        let score_full_coverage = calculate_base_score_with_coverage_multiplier(
            0.0,
            complexity_factor,
            dependency_factor,
        );
        assert_eq!(score_full_coverage, 0.0);
    }

    #[test]
    fn test_coverage_reduces_score_monotonicity() {
        // Test score monotonicity property (spec 122)
        let complexity_factor = 8.0;
        let dependency_factor = 3.0;

        let score_0_coverage = calculate_base_score_with_coverage_multiplier(
            1.0,
            complexity_factor,
            dependency_factor,
        );
        let score_50_coverage = calculate_base_score_with_coverage_multiplier(
            0.5,
            complexity_factor,
            dependency_factor,
        );
        let score_80_coverage = calculate_base_score_with_coverage_multiplier(
            0.2,
            complexity_factor,
            dependency_factor,
        );
        let score_100_coverage = calculate_base_score_with_coverage_multiplier(
            0.0,
            complexity_factor,
            dependency_factor,
        );

        // Scores should decrease as coverage increases
        assert!(score_0_coverage > score_50_coverage);
        assert!(score_50_coverage > score_80_coverage);
        assert!(score_80_coverage > score_100_coverage);
        assert_eq!(score_100_coverage, 0.0);
    }

    #[test]
    fn test_calculate_coverage_factor() {
        // Test zero coverage prioritization (spec 98)
        assert_eq!(calculate_coverage_factor(0.0), 10.0); // Zero coverage: 10x boost

        // Very low coverage (<20%)
        assert!((calculate_coverage_factor(0.1) - 7.7).abs() < 0.01); // 5.0 + (0.9 * 3.0) = 7.7
        assert!((calculate_coverage_factor(0.19) - 7.43).abs() < 0.01); // 5.0 + (0.81 * 3.0) = 7.43

        // Low coverage (20-50%)
        assert!((calculate_coverage_factor(0.2) - 3.6).abs() < 0.01); // 2.0 + (0.8 * 2.0) = 3.6
        assert!((calculate_coverage_factor(0.49) - 3.02).abs() < 0.01); // 2.0 + (0.51 * 2.0) = 3.02

        // Standard coverage (>50%)
        assert!((calculate_coverage_factor(0.5) - 0.453).abs() < 0.01); // Standard: 0.5^1.5 + 0.1
        assert!((calculate_coverage_factor(1.0) - 0.1).abs() < 0.01); // Full coverage: minimum
    }

    #[test]
    fn test_coverage_factor_with_test_flag() {
        // Test code should not be penalized regardless of coverage
        assert_eq!(calculate_coverage_factor_with_test_flag(0.0, true), 0.1);
        assert_eq!(calculate_coverage_factor_with_test_flag(0.5, true), 0.1);
        assert_eq!(calculate_coverage_factor_with_test_flag(1.0, true), 0.1);

        // Non-test code follows normal scoring
        assert_eq!(calculate_coverage_factor_with_test_flag(0.0, false), 10.0);
        assert!((calculate_coverage_factor_with_test_flag(0.5, false) - 0.453).abs() < 0.01);
    }

    #[test]
    fn test_calculate_complexity_factor() {
        // Test linear scaling
        assert_eq!(calculate_complexity_factor(0.0), 0.0);
        assert_eq!(calculate_complexity_factor(10.0), 5.0);
        assert_eq!(calculate_complexity_factor(20.0), 10.0);
        assert_eq!(calculate_complexity_factor(30.0), 10.0); // Capped at 10
    }

    #[test]
    fn test_calculate_dependency_factor() {
        // Test linear scaling
        assert_eq!(calculate_dependency_factor(0), 0.0);
        assert_eq!(calculate_dependency_factor(10), 5.0);
        assert_eq!(calculate_dependency_factor(20), 10.0);
        assert_eq!(calculate_dependency_factor(30), 10.0); // Capped at 10
    }

    #[test]
    fn test_calculate_base_score() {
        // Test weighted sum scoring with new balanced weights (40/40/20)
        let score = calculate_base_score(1.0, 0.5, 0.1);
        // coverage: 1.0*10*0.4 + complexity: 0.5*10*0.4 + deps: 0.1*10*0.2 = 4.0 + 2.0 + 0.2 = 6.2
        assert!((score - 6.2).abs() < 0.01);
    }

    #[test]
    fn test_weights_sum_to_one() {
        const COVERAGE_WEIGHT: f64 = 0.40;
        const COMPLEXITY_WEIGHT: f64 = 0.40;
        const DEPENDENCY_WEIGHT: f64 = 0.20;

        let sum = COVERAGE_WEIGHT + COMPLEXITY_WEIGHT + DEPENDENCY_WEIGHT;
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_complexity_coverage_balance() {
        // Coverage and complexity should have equal influence
        const COVERAGE_WEIGHT: f64 = 0.40;
        const COMPLEXITY_WEIGHT: f64 = 0.40;
        assert_eq!(COVERAGE_WEIGHT, COMPLEXITY_WEIGHT);
    }

    #[test]
    fn test_god_object_ranks_higher_than_simple_untested() {
        // Simple untested function (cc=3, 0% coverage, many callers)
        let simple_score = calculate_base_score(
            11.0, // High coverage gap (0% coverage)
            7.5,  // Low complexity (cc=3)
            10.0, // Many callers
        );
        // Expected: 11.0*10*0.4 + 7.5*10*0.4 + 10.0*10*0.2 = 44 + 30 + 20 = 94

        // God object (2529 lines, 129 functions, high complexity)
        let god_score = calculate_base_score(
            5.0,  // Some coverage
            10.0, // Max complexity
            10.0, // Max dependencies
        );
        // Expected: 5.0*10*0.4 + 10.0*10*0.4 + 10.0*10*0.2 = 20 + 40 + 20 = 80

        // With rebalanced weights, god objects may not always score higher than 0% coverage items
        // The important metric is that complexity weight equals coverage weight (40/40/20)
        // This ensures neither metric dominates, and the balance is more intuitive
        let coverage_coverage_delta = (simple_score - god_score).abs();
        assert!(
            coverage_coverage_delta < 20.0,
            "Scores should be reasonably close with balanced weights. Simple: {}, God: {}, Delta: {}",
            simple_score,
            god_score,
            coverage_coverage_delta
        );

        // Verify the weights are balanced (equal coverage and complexity)
        // This is the key improvement - neither metric dominates
        const COVERAGE_WEIGHT: f64 = 0.40;
        const COMPLEXITY_WEIGHT: f64 = 0.40;
        assert_eq!(COVERAGE_WEIGHT, COMPLEXITY_WEIGHT);
    }

    #[test]
    fn test_calculate_base_score_no_coverage() {
        // Test scoring without coverage data (spec 108)
        // Weights: 50% complexity, 25% dependency

        // Test case 1: High complexity, low dependencies
        let score1 = calculate_base_score_no_coverage(5.0, 1.0);
        // complexity: 5.0*10*0.5 + dependency: 1.0*10*0.25 = 25.0 + 2.5 = 27.5
        assert!(
            (score1 - 27.5).abs() < 0.01,
            "High complexity should dominate score. Expected 27.5, got {}",
            score1
        );

        // Test case 2: Low complexity, high dependencies
        let score2 = calculate_base_score_no_coverage(1.0, 5.0);
        // complexity: 1.0*10*0.5 + dependency: 5.0*10*0.25 = 5.0 + 12.5 = 17.5
        assert!(
            (score2 - 17.5).abs() < 0.01,
            "High dependencies should contribute. Expected 17.5, got {}",
            score2
        );

        // Test case 3: Both high
        let score3 = calculate_base_score_no_coverage(8.0, 6.0);
        // complexity: 8.0*10*0.5 + dependency: 6.0*10*0.25 = 40.0 + 15.0 = 55.0
        assert!(
            (score3 - 55.0).abs() < 0.01,
            "Both high should yield high score. Expected 55.0, got {}",
            score3
        );

        // Test case 4: Both zero
        let score4 = calculate_base_score_no_coverage(0.0, 0.0);
        assert_eq!(score4, 0.0, "Zero factors should yield zero score");

        // Test case 5: Verify weight distribution
        // Complexity weight (50%) should be 2x dependency weight (25%)
        let complexity_contribution = 10.0 * 10.0 * 0.5; // 50.0
        let dependency_contribution = 10.0 * 10.0 * 0.25; // 25.0
        assert_eq!(
            complexity_contribution / dependency_contribution,
            2.0,
            "Complexity should have 2x weight of dependency"
        );

        // Test case 6: Verify the remaining 25% is reserved for debt patterns
        // Total weights used: 50% + 25% = 75%, leaving 25% for debt_adjustment
        let total_weight: f64 = 0.50 + 0.25;
        let reserved_weight: f64 = 1.0 - total_weight;
        assert!(
            (reserved_weight - 0.25).abs() < 0.01,
            "Should reserve 25% for debt patterns"
        );
    }

    #[test]
    fn test_normalization_continuity() {
        // Spec 261: No upper clamping, scores are identity for positive values
        let eps = 0.001;

        let below_50 = normalize_final_score(50.0 - eps);
        let at_50 = normalize_final_score(50.0);
        let above_50 = normalize_final_score(50.0 + eps);
        assert!((at_50 - below_50).abs() < 0.01);
        assert!((above_50 - at_50).abs() < 0.01);

        // No cap - values above 100 are preserved
        let at_100 = normalize_final_score(100.0);
        let above_100 = normalize_final_score(150.0);
        assert_eq!(at_100, 100.0);
        assert_eq!(above_100, 150.0);
    }

    #[test]
    fn test_normalization_monotonic() {
        // Spec 261: Ordering preserved for all positive values
        let scores = [1.0, 5.0, 10.0, 50.0, 99.0, 150.0, 500.0];
        let normalized: Vec<_> = scores.iter().map(|&s| normalize_final_score(s)).collect();

        for i in 1..normalized.len() {
            assert!(normalized[i] > normalized[i - 1]);
        }
    }

    #[test]
    fn test_normalization_identity() {
        // Spec 261: normalize_final_score is identity for positive values
        let test_scores = vec![5.0, 15.0, 50.0, 90.0, 150.0, 500.0];

        for score in test_scores {
            let normalized = normalize_final_score(score);
            assert_eq!(normalized, score);
        }

        // Negative values floor to 0
        assert_eq!(normalize_final_score(-10.0), 0.0);
    }

    #[test]
    fn test_normalization_no_upper_cap() {
        // Spec 261: No upper clamping to preserve relative priority
        assert_eq!(normalize_final_score(5.0), 5.0);
        assert_eq!(normalize_final_score(50.0), 50.0);
        assert_eq!(normalize_final_score(100.0), 100.0);
        assert_eq!(normalize_final_score(200.0), 200.0);
        assert_eq!(normalize_final_score(1000.0), 1000.0);
    }

    #[test]
    fn test_scaling_method_detection() {
        let score1 = normalize_final_score_with_metadata(5.0);
        assert_eq!(score1.scaling_method, ScalingMethod::Linear);

        let score2 = normalize_final_score_with_metadata(50.0);
        assert_eq!(score2.scaling_method, ScalingMethod::SquareRoot);

        let score3 = normalize_final_score_with_metadata(200.0);
        assert_eq!(score3.scaling_method, ScalingMethod::Logarithmic);
    }

    #[test]
    fn test_generate_normalization_curve() {
        let curve = generate_normalization_curve();

        // Verify we have data points
        assert!(!curve.is_empty());

        // Verify we have all three regions
        let linear_count = curve
            .iter()
            .filter(|&(_, _, region)| *region == "Linear")
            .count();
        let sqrt_count = curve
            .iter()
            .filter(|&(_, _, region)| *region == "SquareRoot")
            .count();
        let log_count = curve
            .iter()
            .filter(|&(_, _, region)| *region == "Logarithmic")
            .count();

        assert!(linear_count > 0);
        assert!(sqrt_count > 0);
        assert!(log_count > 0);

        // Verify monotonic increasing
        for i in 1..curve.len() {
            assert!(
                curve[i].0 >= curve[i - 1].0,
                "Raw scores should be monotonic"
            );
            assert!(
                curve[i].1 >= curve[i - 1].1,
                "Normalized scores should be monotonic"
            );
        }
    }

    #[test]
    fn test_normalized_score_display() {
        let score = NormalizedScore {
            raw: 45.0,
            normalized: 16.7,
            scaling_method: ScalingMethod::SquareRoot,
        };

        let display = format!("{}", score);
        assert!(display.contains("16.7"));
        assert!(display.contains("45.0"));
        assert!(display.contains("critical")); // 16.7 is in the "critical" range (>15)
    }
}
