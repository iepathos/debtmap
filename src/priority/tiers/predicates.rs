/// Pure predicate functions for tier classification.
///
/// Each predicate tests ONE specific condition with no side effects.
/// All functions are deterministic, pure, and independently testable.
#[allow(unused_imports)]
use crate::priority::{DebtType, FunctionRole};

// ============================================================================
// T1 ARCHITECTURAL ISSUE PREDICATES
// ============================================================================

/// Checks if debt type is a god object or god module.
#[inline]
pub fn is_god_object(debt_type: &DebtType) -> bool {
    matches!(debt_type, DebtType::GodObject { .. })
}

/// Checks if debt type is an error handling issue.
#[inline]
pub fn is_error_handling_issue(debt_type: &DebtType) -> bool {
    matches!(
        debt_type,
        DebtType::ErrorSwallowing { .. } | DebtType::AsyncMisuse { .. }
    )
}

/// Checks if item has extreme final score (> 10.0, after exponential scaling).
#[inline]
pub fn has_extreme_score(score: f64) -> bool {
    score > 10.0
}

/// Checks if item has extreme effective cyclomatic complexity (> 50).
#[inline]
pub fn has_extreme_cyclomatic(effective_cyclomatic: u32) -> bool {
    effective_cyclomatic > 50
}

/// Checks if item has extreme cognitive complexity (>= 20).
#[inline]
pub fn has_extreme_cognitive(cognitive: u32) -> bool {
    cognitive >= 20
}

/// Checks if item has very deep nesting (>= 5 levels).
#[inline]
pub fn has_deep_nesting(nesting_depth: u32) -> bool {
    nesting_depth >= 5
}

/// Checks if item has high complexity factor (> 5.0).
/// Complexity factor is weighted: 30% cyclomatic + 70% cognitive.
#[inline]
pub fn has_high_complexity_factor(complexity_factor: f64) -> bool {
    complexity_factor > 5.0
}

// ============================================================================
// T2 COMPLEX UNTESTED PREDICATES
// ============================================================================

/// Checks if debt type is a testing gap.
#[inline]
pub fn is_testing_gap(debt_type: &DebtType) -> bool {
    matches!(debt_type, DebtType::TestingGap { .. })
}

/// Checks if debt type is a complexity hotspot.
#[inline]
pub fn is_complexity_hotspot(debt_type: &DebtType) -> bool {
    matches!(debt_type, DebtType::ComplexityHotspot { .. })
}

/// Checks if item has high cyclomatic complexity for its tier.
#[inline]
pub fn has_high_cyclomatic(cyclomatic: u32, threshold: u32) -> bool {
    cyclomatic >= threshold
}

/// Checks if item has many dependencies (>= threshold).
#[inline]
pub fn has_many_dependencies(total_deps: usize, threshold: usize) -> bool {
    total_deps >= threshold
}

/// Checks if item is an entry point function.
#[inline]
pub fn is_entry_point(function_role: &FunctionRole) -> bool {
    matches!(function_role, FunctionRole::EntryPoint)
}

/// Checks if item has moderate complexity factor (>= 2.0).
#[inline]
pub fn has_moderate_complexity_factor(complexity_factor: f64) -> bool {
    complexity_factor >= 2.0
}

/// Checks if item has moderate cognitive complexity (>= 12).
#[inline]
pub fn has_moderate_cognitive(cognitive: u32) -> bool {
    cognitive >= 12
}

/// Checks if item has moderate nesting (>= 3 levels).
#[inline]
pub fn has_moderate_nesting(nesting_depth: u32) -> bool {
    nesting_depth >= 3
}

/// Checks if effective cyclomatic is in moderate range (8-50).
#[inline]
pub fn has_moderate_adjusted_cyclomatic(effective_cyclomatic: u32) -> bool {
    (8..=50).contains(&effective_cyclomatic)
}

// ============================================================================
// T3 TESTING GAP PREDICATES
// ============================================================================

/// Checks if item has moderate cyclomatic complexity for T3.
#[inline]
pub fn has_t3_cyclomatic(cyclomatic: u32, threshold: u32) -> bool {
    cyclomatic >= threshold
}

// ============================================================================
// HELPER FUNCTIONS (Pure, for extracting values from debt types)
// ============================================================================

/// Extract cyclomatic complexity from complexity hotspot debt type.
/// Note: Returns raw cyclomatic (not dampened) as it's a structural metric.
pub fn extract_effective_cyclomatic(debt_type: &DebtType) -> Option<u32> {
    match debt_type {
        DebtType::ComplexityHotspot { cyclomatic, .. } => Some(*cyclomatic),
        _ => None,
    }
}

/// Extract cognitive complexity from complexity hotspot debt type.
pub fn extract_cognitive(debt_type: &DebtType) -> Option<u32> {
    match debt_type {
        DebtType::ComplexityHotspot { cognitive, .. } => Some(*cognitive),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_god_object() {
        assert!(is_god_object(&DebtType::GodObject {
            methods: 100,
            fields: Some(50),
            responsibilities: 5,
            god_object_score: 95.0,
            lines: 500,
        }));
        // GodModule was unified into GodObject (spec 253)
        assert!(is_god_object(&DebtType::GodObject {
            methods: 100,
            fields: None, // No fields for module-level god objects
            responsibilities: 5,
            god_object_score: 95.0,
            lines: 1000,
        }));
        assert!(!is_god_object(&DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 10,
        }));
    }

    #[test]
    fn test_is_error_handling_issue() {
        assert!(is_error_handling_issue(&DebtType::ErrorSwallowing {
            pattern: "unwrap".into(),
            context: None,
        }));
        assert!(is_error_handling_issue(&DebtType::AsyncMisuse {
            pattern: "blocking in async".into(),
            performance_impact: "high".into(),
        }));
        assert!(!is_error_handling_issue(&DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 10,
        }));
    }

    #[test]
    fn test_has_extreme_score() {
        assert!(has_extreme_score(10.1));
        assert!(has_extreme_score(100.0));
        assert!(!has_extreme_score(10.0));
        assert!(!has_extreme_score(9.9));
    }

    #[test]
    fn test_has_extreme_cyclomatic() {
        assert!(has_extreme_cyclomatic(51));
        assert!(has_extreme_cyclomatic(100));
        assert!(!has_extreme_cyclomatic(50));
        assert!(!has_extreme_cyclomatic(30));
    }

    #[test]
    fn test_has_extreme_cognitive() {
        assert!(has_extreme_cognitive(20));
        assert!(has_extreme_cognitive(50));
        assert!(!has_extreme_cognitive(19));
        assert!(!has_extreme_cognitive(10));
    }

    #[test]
    fn test_has_deep_nesting() {
        assert!(has_deep_nesting(5));
        assert!(has_deep_nesting(10));
        assert!(!has_deep_nesting(4));
        assert!(!has_deep_nesting(2));
    }

    #[test]
    fn test_has_high_complexity_factor() {
        assert!(has_high_complexity_factor(5.1));
        assert!(has_high_complexity_factor(10.0));
        assert!(!has_high_complexity_factor(5.0));
        assert!(!has_high_complexity_factor(2.0));
    }

    #[test]
    fn test_is_testing_gap() {
        assert!(is_testing_gap(&DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 10,
        }));
        assert!(!is_testing_gap(&DebtType::GodObject {
            methods: 100,
            fields: Some(50),
            responsibilities: 5,
            god_object_score: 95.0,
            lines: 500,
        }));
    }

    #[test]
    fn test_is_complexity_hotspot() {
        assert!(is_complexity_hotspot(&DebtType::ComplexityHotspot {
            cyclomatic: 25,
            cognitive: 15,
        }));
        assert!(!is_complexity_hotspot(&DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 10,
        }));
    }

    #[test]
    fn test_has_high_cyclomatic() {
        assert!(has_high_cyclomatic(15, 15));
        assert!(has_high_cyclomatic(20, 15));
        assert!(!has_high_cyclomatic(14, 15));
        assert!(!has_high_cyclomatic(10, 15));
    }

    #[test]
    fn test_has_many_dependencies() {
        assert!(has_many_dependencies(10, 10));
        assert!(has_many_dependencies(20, 10));
        assert!(!has_many_dependencies(9, 10));
        assert!(!has_many_dependencies(5, 10));
    }

    #[test]
    fn test_is_entry_point() {
        assert!(is_entry_point(&FunctionRole::EntryPoint));
        assert!(!is_entry_point(&FunctionRole::PureLogic));
        assert!(!is_entry_point(&FunctionRole::IOWrapper));
    }

    #[test]
    fn test_has_moderate_complexity_factor() {
        assert!(has_moderate_complexity_factor(2.0));
        assert!(has_moderate_complexity_factor(5.0));
        assert!(!has_moderate_complexity_factor(1.9));
        assert!(!has_moderate_complexity_factor(1.0));
    }

    #[test]
    fn test_has_moderate_cognitive() {
        assert!(has_moderate_cognitive(12));
        assert!(has_moderate_cognitive(20));
        assert!(!has_moderate_cognitive(11));
        assert!(!has_moderate_cognitive(5));
    }

    #[test]
    fn test_has_moderate_nesting() {
        assert!(has_moderate_nesting(3));
        assert!(has_moderate_nesting(5));
        assert!(!has_moderate_nesting(2));
        assert!(!has_moderate_nesting(1));
    }

    #[test]
    fn test_has_moderate_adjusted_cyclomatic() {
        assert!(has_moderate_adjusted_cyclomatic(8));
        assert!(has_moderate_adjusted_cyclomatic(25));
        assert!(has_moderate_adjusted_cyclomatic(50));
        assert!(!has_moderate_adjusted_cyclomatic(7));
        assert!(!has_moderate_adjusted_cyclomatic(51));
    }

    #[test]
    fn test_has_t3_cyclomatic() {
        assert!(has_t3_cyclomatic(10, 10));
        assert!(has_t3_cyclomatic(15, 10));
        assert!(!has_t3_cyclomatic(9, 10));
        assert!(!has_t3_cyclomatic(5, 10));
    }

    #[test]
    fn test_extract_effective_cyclomatic() {
        // Returns raw cyclomatic (adjusted_cyclomatic is ignored)
        let debt = DebtType::ComplexityHotspot {
            cyclomatic: 25,
            cognitive: 15,
        };
        assert_eq!(extract_effective_cyclomatic(&debt), Some(25));

        // Without adjusted cyclomatic
        let debt = DebtType::ComplexityHotspot {
            cyclomatic: 25,
            cognitive: 15,
        };
        assert_eq!(extract_effective_cyclomatic(&debt), Some(25));

        // Not a complexity hotspot
        let debt = DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 10,
        };
        assert_eq!(extract_effective_cyclomatic(&debt), None);
    }

    #[test]
    fn test_extract_cognitive() {
        let debt = DebtType::ComplexityHotspot {
            cyclomatic: 25,
            cognitive: 15,
        };
        assert_eq!(extract_cognitive(&debt), Some(15));

        let debt = DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 10,
        };
        assert_eq!(extract_cognitive(&debt), None);
    }
}
