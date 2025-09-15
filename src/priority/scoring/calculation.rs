// Pure functions for scoring calculation (spec 68, spec 101)

use std::fmt;

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

/// Calculate weighted sum base score
/// Uses additive model for clear, predictable scoring
pub fn calculate_base_score(
    coverage_factor: f64,
    complexity_factor: f64,
    dependency_factor: f64,
) -> f64 {
    // Default weights (can be overridden by config)
    let coverage_weight = 0.50; // 50% weight on coverage gaps
    let complexity_weight = 0.35; // 35% weight on complexity
    let dependency_weight = 0.15; // 15% weight on dependencies

    // Convert factors to 0-100 scale for clarity
    let coverage_score = coverage_factor * 10.0; // Already 0-10 scale
    let complexity_score = complexity_factor * 10.0; // Already 0-10 scale
    let dependency_score = dependency_factor * 10.0; // Already 0-10 scale

    // Weighted sum
    (coverage_score * coverage_weight)
        + (complexity_score * complexity_weight)
        + (dependency_score * dependency_weight)
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

/// Normalize final score to a simple f64 (backwards compatibility)
pub fn normalize_final_score(raw_score: f64) -> f64 {
    // Simple linear scaling to 0-100 range for clarity
    // No complex transformations that distort relative differences
    raw_score.clamp(0.0, 100.0)
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
        // Test weighted sum scoring
        let score = calculate_base_score(1.0, 0.5, 0.1);
        // coverage: 1.0*10*0.5 + complexity: 0.5*10*0.35 + deps: 0.1*10*0.15 = 5.0 + 1.75 + 0.15 = 6.9
        assert!((score - 6.9).abs() < 0.01);
    }

    #[test]
    fn test_normalization_continuity() {
        // Linear scaling is continuous everywhere
        let eps = 0.001;

        let below_50 = normalize_final_score(50.0 - eps);
        let at_50 = normalize_final_score(50.0);
        let above_50 = normalize_final_score(50.0 + eps);
        assert!((at_50 - below_50).abs() < 0.01);
        assert!((above_50 - at_50).abs() < 0.01);

        // At cap
        let at_100 = normalize_final_score(100.0);
        let above_100 = normalize_final_score(101.0);
        assert_eq!(at_100, 100.0);
        assert_eq!(above_100, 100.0);
    }

    #[test]
    fn test_normalization_monotonic() {
        // Verify ordering is preserved (up to cap)
        let scores = [1.0, 5.0, 10.0, 50.0, 99.0];
        let normalized: Vec<_> = scores.iter().map(|&s| normalize_final_score(s)).collect();

        for i in 1..normalized.len() {
            assert!(normalized[i] > normalized[i - 1]);
        }

        // Values above 100 are capped
        assert_eq!(normalize_final_score(100.0), 100.0);
        assert_eq!(normalize_final_score(500.0), 100.0);
    }

    #[test]
    fn test_inverse_function() {
        // With linear scaling, inverse should work for values up to 100
        let test_scores = vec![5.0, 15.0, 50.0, 90.0];

        for score in test_scores {
            let normalized = normalize_final_score(score);
            assert_eq!(normalized, score); // Linear = identity up to 100
        }

        // Above 100 is capped
        assert_eq!(normalize_final_score(150.0), 100.0);
    }

    #[test]
    fn test_normalization_ranges() {
        // Linear scaling preserves values up to 100
        assert_eq!(normalize_final_score(5.0), 5.0);
        assert_eq!(normalize_final_score(50.0), 50.0);
        assert_eq!(normalize_final_score(100.0), 100.0);
        assert_eq!(normalize_final_score(200.0), 100.0); // Capped
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
