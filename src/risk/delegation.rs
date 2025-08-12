use crate::core::FunctionMetrics;

/// Detects if a function is likely a trivial delegation to another function
pub fn is_trivial_delegation(func: &FunctionMetrics) -> bool {
    // A function is considered a trivial delegation if:
    // 1. It has cyclomatic complexity of 1 (no branching)
    // 2. It has cognitive complexity <= 1 (trivial logic)
    // 3. It's very short (typically 1-3 lines)
    func.cyclomatic == 1 && func.cognitive <= 1 && func.length <= 3
}

/// Calculate a complexity weight factor for ROI calculations
/// Trivial functions get heavily penalized to avoid dominating recommendations
pub fn calculate_complexity_weight(func: &FunctionMetrics) -> f64 {
    match (func.cyclomatic, func.cognitive) {
        (1, 0..=1) => 0.1, // Trivial delegation - 90% reduction
        (1, 2..=3) => 0.3, // Very simple - 70% reduction
        (2..=3, _) => 0.5, // Simple - 50% reduction
        (4..=5, _) => 0.7, // Moderate - 30% reduction
        _ => 1.0,          // Complex - no reduction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_trivial_delegation() {
        let trivial = FunctionMetrics {
            name: "delegate".to_string(),
            file: PathBuf::from("main.rs"),
            line: 10,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            length: 1,
            is_test: false,
            visibility: None,
        };
        assert!(is_trivial_delegation(&trivial));

        let simple = FunctionMetrics {
            name: "simple".to_string(),
            file: PathBuf::from("main.rs"),
            line: 20,
            cyclomatic: 1,
            cognitive: 1,
            nesting: 0,
            length: 3,
            is_test: false,
            visibility: None,
        };
        assert!(is_trivial_delegation(&simple));

        let complex = FunctionMetrics {
            name: "complex".to_string(),
            file: PathBuf::from("main.rs"),
            line: 30,
            cyclomatic: 5,
            cognitive: 10,
            nesting: 2,
            length: 20,
            is_test: false,
            visibility: None,
        };
        assert!(!is_trivial_delegation(&complex));
    }

    #[test]
    fn test_calculate_complexity_weight() {
        let trivial = FunctionMetrics {
            name: "delegate".to_string(),
            file: PathBuf::from("main.rs"),
            line: 10,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            length: 1,
            is_test: false,
            visibility: None,
        };
        assert_eq!(calculate_complexity_weight(&trivial), 0.1);

        let simple = FunctionMetrics {
            name: "simple".to_string(),
            file: PathBuf::from("main.rs"),
            line: 20,
            cyclomatic: 2,
            cognitive: 3,
            nesting: 0,
            length: 10,
            is_test: false,
            visibility: None,
        };
        assert_eq!(calculate_complexity_weight(&simple), 0.5);

        let complex = FunctionMetrics {
            name: "complex".to_string(),
            file: PathBuf::from("main.rs"),
            line: 30,
            cyclomatic: 10,
            cognitive: 15,
            nesting: 3,
            length: 50,
            is_test: false,
            visibility: None,
        };
        assert_eq!(calculate_complexity_weight(&complex), 1.0);
    }
}
