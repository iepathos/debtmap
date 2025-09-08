// Pure functions for scoring calculation (spec 68)

/// Calculate coverage factor from coverage percentage
pub fn calculate_coverage_factor(coverage_pct: f64) -> f64 {
    calculate_coverage_factor_with_test_flag(coverage_pct, false)
}

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
    if raw_complexity <= 1.0 {
        raw_complexity * 0.5 // Low complexity gets lower factor
    } else if raw_complexity <= 3.0 {
        0.5 + (raw_complexity - 1.0) * 0.3 // Medium-low complexity
    } else {
        raw_complexity.powf(0.8) // Higher complexity uses power scaling
    }
}

/// Calculate dependency factor from upstream count
pub fn calculate_dependency_factor(upstream_count: usize) -> f64 {
    if upstream_count == 0 {
        0.1 // No dependencies = low factor
    } else if upstream_count <= 2 {
        0.3 + upstream_count as f64 * 0.2 // Small dependency boost
    } else {
        ((upstream_count as f64 + 1.0).sqrt() / 2.0).min(2.0) // Sqrt scaling for many dependencies
    }
}

/// Calculate multiplicative base score
pub fn calculate_base_score(
    coverage_factor: f64,
    complexity_factor: f64,
    dependency_factor: f64,
) -> f64 {
    // Apply small constants to avoid zero multiplication
    let complexity_component = (complexity_factor + 0.1).max(0.1);
    let dependency_component = (dependency_factor + 0.1).max(0.1);
    coverage_factor * complexity_component * dependency_component
}

/// Apply complexity-coverage interaction bonus
pub fn apply_interaction_bonus(base_score: f64, coverage_pct: f64, raw_complexity: f64) -> f64 {
    if coverage_pct < 0.5 && raw_complexity > 5.0 {
        base_score * 1.5 // 50% bonus for complex untested code
    } else {
        base_score
    }
}

/// Normalize final score to 0-10 range with better distribution
pub fn normalize_final_score(raw_score: f64) -> f64 {
    // Improved normalization for better score distribution (fixing spec 68)
    // Raw scores are typically 0.01-2.0, so we need better granularity

    if raw_score <= 0.01 {
        0.0 // Trivial or fully tested
    } else if raw_score <= 0.1 {
        // Very low scores: map to 0-2 range
        raw_score * 20.0 // 0.01-0.1 -> 0.2-2.0
    } else if raw_score <= 0.3 {
        // Low scores: map to 2-4 range
        2.0 + (raw_score - 0.1) * 10.0 // 0.1-0.3 -> 2.0-4.0
    } else if raw_score <= 0.6 {
        // Medium scores: map to 4-6 range
        4.0 + (raw_score - 0.3) * 6.67 // 0.3-0.6 -> 4.0-6.0
    } else if raw_score <= 1.0 {
        // Medium-high scores: map to 6-8 range
        6.0 + (raw_score - 0.6) * 5.0 // 0.6-1.0 -> 6.0-8.0
    } else if raw_score <= 2.0 {
        // High scores: map to 8-9.5 range
        8.0 + (raw_score - 1.0) * 1.5 // 1.0-2.0 -> 8.0-9.5
    } else if raw_score <= 10.0 {
        // Very high scores: keep existing scaling
        9.5 + (raw_score - 2.0) * 0.25
    } else {
        // Extreme scores: use square root scaling for gradual increase
        10.0 + (raw_score - 10.0).sqrt()
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

#[cfg(test)]
mod tests {
    use super::*;

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
        // Test different complexity ranges
        assert_eq!(calculate_complexity_factor(0.5), 0.25); // Low * 0.5
        assert_eq!(calculate_complexity_factor(1.0), 0.5); // Low boundary
        assert!((calculate_complexity_factor(2.0) - 0.8).abs() < 0.01); // Medium
        assert!((calculate_complexity_factor(5.0) - 3.62).abs() < 0.1); // High, power scaling - 5.0^0.8 = ~3.62
    }

    #[test]
    fn test_calculate_dependency_factor() {
        assert_eq!(calculate_dependency_factor(0), 0.1); // No deps
        assert_eq!(calculate_dependency_factor(1), 0.5); // 1 dep
        assert_eq!(calculate_dependency_factor(2), 0.7); // 2 deps
        assert!((calculate_dependency_factor(10) - 1.65).abs() < 0.1); // Many deps
    }

    #[test]
    fn test_calculate_base_score() {
        // Test multiplicative scoring
        let score = calculate_base_score(1.0, 0.5, 0.1);
        // (1.0) * (0.5 + 0.1) * (0.1 + 0.1) = 1.0 * 0.6 * 0.2 = 0.12
        assert!((score - 0.12).abs() < 0.01);
    }

    #[test]
    fn test_apply_interaction_bonus() {
        // No bonus case
        assert_eq!(apply_interaction_bonus(1.0, 0.8, 3.0), 1.0);

        // Bonus case: low coverage + high complexity
        assert_eq!(apply_interaction_bonus(1.0, 0.3, 7.0), 1.5);
    }

    #[test]
    fn test_normalize_final_score_ranges() {
        // Test each range boundary
        assert_eq!(normalize_final_score(0.005), 0.0);
        assert_eq!(normalize_final_score(0.04), 0.8); // 0.04 * 20 = 0.8
        assert!((normalize_final_score(0.1) - 2.0).abs() < 0.01); // 0.1 is boundary -> 2.0
        assert!((normalize_final_score(0.4) - 4.667).abs() < 0.01); // 4.0 + (0.4-0.3)*6.67 = ~4.667
        assert!((normalize_final_score(1.5) - 8.75).abs() < 0.01); // 8.0 + (1.5-1.0)*1.5 = 8.75
                                                                   // With no cap, 5.0 raw score: 9.5 + (5.0-2.0)*0.25 = 10.25
        assert!((normalize_final_score(5.0) - 10.25).abs() < 0.01);
    }
}
