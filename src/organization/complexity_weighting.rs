/// Calculate complexity-weighted god object score.
///
/// Unlike raw method counting, this module provides functions that weight each method by its
/// cyclomatic complexity, ensuring that 100 simple functions (complexity 1-3)
/// score better than 10 complex functions (complexity 17+).
///
/// # Weighting Formula
///
/// Each function contributes: `max(1, complexity / 3)^1.5`
///
/// Examples:
/// - Complexity 1 → weight 0.33 (simple helper)
/// - Complexity 3 → weight 1.0 (baseline)
/// - Complexity 17 → weight 8.2 (needs refactoring)
/// - Complexity 33 → weight 22.9 (critical problem)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityWeight {
    pub raw_value: u32,
    pub weighted_value: f64,
    pub weight_multiplier: f64,
}

impl ComplexityWeight {
    /// Calculate complexity weight for a single function.
    ///
    /// # Arguments
    ///
    /// * `cyclomatic_complexity` - The cyclomatic complexity of the function
    ///
    /// # Returns
    ///
    /// A `ComplexityWeight` containing the raw complexity, weighted value, and multiplier
    pub fn calculate(cyclomatic_complexity: u32) -> Self {
        let normalized = cyclomatic_complexity.max(1) as f64 / 3.0;
        let weighted_value = normalized.powf(1.5);

        Self {
            raw_value: cyclomatic_complexity,
            weighted_value,
            weight_multiplier: weighted_value / cyclomatic_complexity as f64,
        }
    }
}

/// Pure function to calculate complexity weight for a single function.
///
/// # Arguments
///
/// * `complexity` - The cyclomatic complexity of the function
///
/// # Returns
///
/// The weighted value (f64) representing the function's contribution to god object score
pub fn calculate_complexity_weight(complexity: u32) -> f64 {
    let normalized = complexity.max(1) as f64 / 3.0;
    normalized.powf(1.5)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComplexityInfo {
    pub name: String,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub is_test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityWeightedAnalysis {
    pub raw_method_count: usize,
    pub weighted_method_count: f64,
    pub avg_complexity: f64,
    pub max_complexity: u32,
    pub high_complexity_count: usize, // complexity > 10
}

/// Aggregate weighted complexity from a collection of functions.
///
/// # Arguments
///
/// * `functions` - Slice of function complexity information
///
/// # Returns
///
/// The sum of complexity weights for all functions
pub fn aggregate_weighted_complexity(functions: &[FunctionComplexityInfo]) -> f64 {
    functions
        .iter()
        .filter(|f| !f.is_test)
        .map(|f| calculate_complexity_weight(f.cyclomatic_complexity))
        .sum()
}

/// Calculate the average complexity of a collection of functions.
///
/// # Arguments
///
/// * `functions` - Slice of function complexity information
///
/// # Returns
///
/// The average cyclomatic complexity, or 0.0 if no functions provided
pub fn calculate_avg_complexity(functions: &[FunctionComplexityInfo]) -> f64 {
    let non_test_functions: Vec<_> = functions.iter().filter(|f| !f.is_test).collect();

    if non_test_functions.is_empty() {
        return 0.0;
    }

    let sum: u32 = non_test_functions
        .iter()
        .map(|f| f.cyclomatic_complexity)
        .sum();

    sum as f64 / non_test_functions.len() as f64
}

/// Calculate complexity factor penalty/bonus for scoring.
///
/// # Arguments
///
/// * `avg_complexity` - The average cyclomatic complexity across functions
///
/// # Returns
///
/// A multiplier factor: <1.0 for simple functions, >1.0 for complex functions
pub fn calculate_complexity_penalty(avg_complexity: f64) -> f64 {
    if avg_complexity < 3.0 {
        0.7 // Reward simple functions
    } else if avg_complexity > 10.0 {
        1.5 // Penalize complex functions
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_complexity_weight_simple() {
        let weight = calculate_complexity_weight(1);
        assert!((weight - 0.19).abs() < 0.01);
    }

    #[test]
    fn test_calculate_complexity_weight_baseline() {
        let weight = calculate_complexity_weight(3);
        assert!((weight - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_complexity_weight_moderate() {
        let weight = calculate_complexity_weight(9);
        assert!((weight - 5.2).abs() < 0.1);
    }

    #[test]
    fn test_calculate_complexity_weight_high() {
        let weight = calculate_complexity_weight(17);
        assert!((weight - 13.5).abs() < 0.2);
    }

    #[test]
    fn test_calculate_complexity_weight_critical() {
        let weight = calculate_complexity_weight(33);
        assert!((weight - 36.5).abs() < 0.5);
    }

    #[test]
    fn test_complexity_weight_struct() {
        let weight = ComplexityWeight::calculate(17);
        assert_eq!(weight.raw_value, 17);
        assert!((weight.weighted_value - 13.5).abs() < 0.2);
    }

    #[test]
    fn test_aggregate_weighted_complexity_simple_functions() {
        let functions = vec![
            FunctionComplexityInfo {
                name: "fn1".to_string(),
                cyclomatic_complexity: 1,
                cognitive_complexity: 1,
                is_test: false,
            },
            FunctionComplexityInfo {
                name: "fn2".to_string(),
                cyclomatic_complexity: 1,
                cognitive_complexity: 1,
                is_test: false,
            },
            FunctionComplexityInfo {
                name: "fn3".to_string(),
                cyclomatic_complexity: 1,
                cognitive_complexity: 1,
                is_test: false,
            },
        ];

        let weighted = aggregate_weighted_complexity(&functions);
        // 3 functions @ complexity 1 each ≈ 0.19 * 3 ≈ 0.57
        assert!((weighted - 0.57).abs() < 0.1);
    }

    #[test]
    fn test_aggregate_weighted_complexity_complex_functions() {
        let functions = vec![
            FunctionComplexityInfo {
                name: "fn1".to_string(),
                cyclomatic_complexity: 17,
                cognitive_complexity: 15,
                is_test: false,
            },
            FunctionComplexityInfo {
                name: "fn2".to_string(),
                cyclomatic_complexity: 17,
                cognitive_complexity: 15,
                is_test: false,
            },
        ];

        let weighted = aggregate_weighted_complexity(&functions);
        // 2 functions @ complexity 17 each ≈ 13.5 * 2 ≈ 27.0
        assert!((weighted - 27.0).abs() < 0.5);
    }

    #[test]
    fn test_aggregate_weighted_complexity_excludes_tests() {
        let functions = vec![
            FunctionComplexityInfo {
                name: "fn1".to_string(),
                cyclomatic_complexity: 1,
                cognitive_complexity: 1,
                is_test: false,
            },
            FunctionComplexityInfo {
                name: "test_fn".to_string(),
                cyclomatic_complexity: 10,
                cognitive_complexity: 10,
                is_test: true,
            },
        ];

        let weighted = aggregate_weighted_complexity(&functions);
        // Should only count fn1, not the test function
        assert!((weighted - 0.19).abs() < 0.1);
    }

    #[test]
    fn test_calculate_avg_complexity() {
        let functions = vec![
            FunctionComplexityInfo {
                name: "fn1".to_string(),
                cyclomatic_complexity: 1,
                cognitive_complexity: 1,
                is_test: false,
            },
            FunctionComplexityInfo {
                name: "fn2".to_string(),
                cyclomatic_complexity: 3,
                cognitive_complexity: 3,
                is_test: false,
            },
            FunctionComplexityInfo {
                name: "fn3".to_string(),
                cyclomatic_complexity: 5,
                cognitive_complexity: 5,
                is_test: false,
            },
        ];

        let avg = calculate_avg_complexity(&functions);
        assert!((avg - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_avg_complexity_excludes_tests() {
        let functions = vec![
            FunctionComplexityInfo {
                name: "fn1".to_string(),
                cyclomatic_complexity: 2,
                cognitive_complexity: 2,
                is_test: false,
            },
            FunctionComplexityInfo {
                name: "test_fn".to_string(),
                cyclomatic_complexity: 20,
                cognitive_complexity: 20,
                is_test: true,
            },
        ];

        let avg = calculate_avg_complexity(&functions);
        // Should only count fn1
        assert!((avg - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_avg_complexity_empty() {
        let functions = vec![];
        let avg = calculate_avg_complexity(&functions);
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn test_calculate_complexity_penalty_simple() {
        let penalty = calculate_complexity_penalty(2.0);
        assert_eq!(penalty, 0.7);
    }

    #[test]
    fn test_calculate_complexity_penalty_baseline() {
        let penalty = calculate_complexity_penalty(5.0);
        assert_eq!(penalty, 1.0);
    }

    #[test]
    fn test_calculate_complexity_penalty_complex() {
        let penalty = calculate_complexity_penalty(15.0);
        assert_eq!(penalty, 1.5);
    }

    #[test]
    fn test_weighted_vs_raw_count_simple_functions() {
        // 100 functions @ complexity 1 should have lower weighted count than 10 @ complexity 17
        let simple_functions: Vec<FunctionComplexityInfo> = (0..100)
            .map(|i| FunctionComplexityInfo {
                name: format!("fn{}", i),
                cyclomatic_complexity: 1,
                cognitive_complexity: 1,
                is_test: false,
            })
            .collect();

        let complex_functions: Vec<FunctionComplexityInfo> = (0..10)
            .map(|i| FunctionComplexityInfo {
                name: format!("fn{}", i),
                cyclomatic_complexity: 17,
                cognitive_complexity: 15,
                is_test: false,
            })
            .collect();

        let simple_weighted = aggregate_weighted_complexity(&simple_functions);
        let complex_weighted = aggregate_weighted_complexity(&complex_functions);

        // Simple: 100 * 0.19 ≈ 19
        // Complex: 10 * 13.5 ≈ 135
        assert!(simple_weighted < complex_weighted);
        assert!((simple_weighted - 19.0).abs() < 2.0);
        assert!((complex_weighted - 135.0).abs() < 5.0);
    }
}
