// Pure functions for scoring calculation (spec 68)

/// Calculate coverage factor from coverage percentage
pub fn calculate_coverage_factor(coverage_pct: f64) -> f64 {
    let coverage_gap = 1.0 - coverage_pct;
    // Use exponential scaling with baseline for differentiation
    (coverage_gap.powf(1.5) + 0.1).max(0.1)
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
        // Test various coverage percentages
        assert!((calculate_coverage_factor(0.0) - 1.1).abs() < 0.01); // 100% gap
        assert!((calculate_coverage_factor(0.5) - 0.453).abs() < 0.01); // 50% gap
        assert!((calculate_coverage_factor(1.0) - 0.1).abs() < 0.01); // 0% gap
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
        assert!((normalize_final_score(5.0) - 10.0).abs() < 0.01); // max(9.5 + (5.0-2.0)*0.25, 10.0) = 10.0
    }
}
