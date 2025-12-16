//! # Metrics Types
//!
//! Types for representing purity distribution and other metrics.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - data structures with no behavior.

use serde::{Deserialize, Serialize};

/// Distribution of functions by purity level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurityDistribution {
    pub pure_count: usize,
    pub probably_pure_count: usize,
    pub impure_count: usize,
    pub pure_weight_contribution: f64,
    pub probably_pure_weight_contribution: f64,
    pub impure_weight_contribution: f64,
}

// ============================================================================
// Spec 211: Method Complexity Weighting
// ============================================================================

/// Aggregated complexity metrics for a struct's methods (Spec 211).
///
/// These metrics provide a deeper view into the complexity burden of a type,
/// enabling more accurate God Object scoring. A struct with 15 complex methods
/// should score higher than one with 15 simple accessor methods.
///
/// ## Usage in Scoring
///
/// The `calculate_complexity_factor` function uses these metrics to produce a
/// multiplier that adjusts the God Object score based on method complexity.
///
/// ## Stillwater Principle: Pure Data
///
/// This struct contains only data - no behavior. All calculations are performed
/// by separate pure functions in `scoring.rs`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ComplexityMetrics {
    /// Sum of cyclomatic complexity across all methods.
    ///
    /// Higher values indicate more decision points in the code, making it
    /// harder to understand and test.
    pub total_cyclomatic: u32,

    /// Sum of cognitive complexity across all methods.
    ///
    /// Cognitive complexity measures how hard code is to understand,
    /// penalizing nested control flow more heavily than cyclomatic.
    pub total_cognitive: u32,

    /// Highest cyclomatic complexity of any single method.
    ///
    /// A single very complex method is a stronger God Object signal than
    /// uniformly moderate complexity.
    pub max_cyclomatic: u32,

    /// Highest cognitive complexity of any single method.
    pub max_cognitive: u32,

    /// Average cyclomatic complexity per method.
    pub avg_cyclomatic: f64,

    /// Average cognitive complexity per method.
    pub avg_cognitive: f64,

    /// Standard deviation of cyclomatic complexity.
    ///
    /// High variance indicates inconsistent quality - some methods are well
    /// written while others are not.
    pub complexity_variance: f64,

    /// Maximum nesting depth across all methods.
    ///
    /// Deep nesting makes code hard to follow and is a code smell.
    pub max_nesting: u32,
}

/// Per-method complexity information for complexity metrics calculation.
///
/// Used as input to `calculate_complexity_metrics`.
#[derive(Debug, Clone, Default)]
pub struct MethodComplexity {
    /// Cyclomatic complexity of this method.
    pub cyclomatic: u32,

    /// Cognitive complexity of this method.
    pub cognitive: u32,

    /// Maximum nesting depth in this method.
    pub max_nesting: u32,
}

/// Calculate aggregated complexity metrics from per-method complexities.
///
/// **Pure function** - deterministic, no side effects.
///
/// # Arguments
///
/// * `method_complexities` - Per-method complexity data
///
/// # Returns
///
/// Aggregated complexity metrics for use in scoring.
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{MethodComplexity, calculate_complexity_metrics};
///
/// let methods = vec![
///     MethodComplexity { cyclomatic: 5, cognitive: 4, max_nesting: 2 },
///     MethodComplexity { cyclomatic: 15, cognitive: 12, max_nesting: 4 },
/// ];
///
/// let metrics = calculate_complexity_metrics(&methods);
/// assert_eq!(metrics.total_cyclomatic, 20);
/// assert_eq!(metrics.max_cyclomatic, 15);
/// assert!((metrics.avg_cyclomatic - 10.0).abs() < 0.01);
/// ```
pub fn calculate_complexity_metrics(method_complexities: &[MethodComplexity]) -> ComplexityMetrics {
    if method_complexities.is_empty() {
        return ComplexityMetrics::default();
    }

    let cyclomatic_values: Vec<u32> = method_complexities.iter().map(|m| m.cyclomatic).collect();
    let cognitive_values: Vec<u32> = method_complexities.iter().map(|m| m.cognitive).collect();

    let total_cyclomatic: u32 = cyclomatic_values.iter().sum();
    let total_cognitive: u32 = cognitive_values.iter().sum();
    let max_cyclomatic = *cyclomatic_values.iter().max().unwrap_or(&0);
    let max_cognitive = *cognitive_values.iter().max().unwrap_or(&0);

    let n = method_complexities.len() as f64;
    let avg_cyclomatic = total_cyclomatic as f64 / n;
    let avg_cognitive = total_cognitive as f64 / n;

    // Calculate variance (standard deviation of cyclomatic complexity)
    let variance: f64 = cyclomatic_values
        .iter()
        .map(|&c| (c as f64 - avg_cyclomatic).powi(2))
        .sum::<f64>()
        / n;
    let complexity_variance = variance.sqrt();

    let max_nesting = method_complexities
        .iter()
        .map(|m| m.max_nesting)
        .max()
        .unwrap_or(0);

    ComplexityMetrics {
        total_cyclomatic,
        total_cognitive,
        max_cyclomatic,
        max_cognitive,
        avg_cyclomatic,
        avg_cognitive,
        complexity_variance,
        max_nesting,
    }
}

#[cfg(test)]
mod complexity_metrics_tests {
    use super::*;

    #[test]
    fn test_empty_methods_returns_default() {
        let metrics = calculate_complexity_metrics(&[]);
        assert_eq!(metrics, ComplexityMetrics::default());
    }

    #[test]
    fn test_single_method_metrics() {
        let methods = vec![MethodComplexity {
            cyclomatic: 10,
            cognitive: 8,
            max_nesting: 3,
        }];

        let metrics = calculate_complexity_metrics(&methods);

        assert_eq!(metrics.total_cyclomatic, 10);
        assert_eq!(metrics.total_cognitive, 8);
        assert_eq!(metrics.max_cyclomatic, 10);
        assert_eq!(metrics.max_cognitive, 8);
        assert!((metrics.avg_cyclomatic - 10.0).abs() < 0.01);
        assert!((metrics.avg_cognitive - 8.0).abs() < 0.01);
        assert!((metrics.complexity_variance - 0.0).abs() < 0.01);
        assert_eq!(metrics.max_nesting, 3);
    }

    #[test]
    fn test_multiple_methods_aggregation() {
        let methods = vec![
            MethodComplexity {
                cyclomatic: 5,
                cognitive: 4,
                max_nesting: 2,
            },
            MethodComplexity {
                cyclomatic: 15,
                cognitive: 12,
                max_nesting: 4,
            },
            MethodComplexity {
                cyclomatic: 10,
                cognitive: 8,
                max_nesting: 3,
            },
        ];

        let metrics = calculate_complexity_metrics(&methods);

        assert_eq!(metrics.total_cyclomatic, 30);
        assert_eq!(metrics.total_cognitive, 24);
        assert_eq!(metrics.max_cyclomatic, 15);
        assert_eq!(metrics.max_cognitive, 12);
        assert!((metrics.avg_cyclomatic - 10.0).abs() < 0.01);
        assert!((metrics.avg_cognitive - 8.0).abs() < 0.01);
        assert_eq!(metrics.max_nesting, 4);
        // Variance: ((5-10)^2 + (15-10)^2 + (10-10)^2) / 3 = 50/3 = 16.67
        // Std dev: sqrt(16.67) ≈ 4.08
        assert!((metrics.complexity_variance - 4.08).abs() < 0.1);
    }

    #[test]
    fn test_all_same_complexity_zero_variance() {
        let methods = vec![
            MethodComplexity {
                cyclomatic: 5,
                cognitive: 5,
                max_nesting: 2,
            },
            MethodComplexity {
                cyclomatic: 5,
                cognitive: 5,
                max_nesting: 2,
            },
            MethodComplexity {
                cyclomatic: 5,
                cognitive: 5,
                max_nesting: 2,
            },
        ];

        let metrics = calculate_complexity_metrics(&methods);
        assert!((metrics.complexity_variance - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_deterministic_output() {
        let methods = vec![
            MethodComplexity {
                cyclomatic: 7,
                cognitive: 6,
                max_nesting: 3,
            },
            MethodComplexity {
                cyclomatic: 12,
                cognitive: 10,
                max_nesting: 4,
            },
        ];

        let metrics1 = calculate_complexity_metrics(&methods);
        let metrics2 = calculate_complexity_metrics(&methods);

        assert_eq!(metrics1, metrics2);
    }
}
