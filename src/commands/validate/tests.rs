//! Integration tests for the validate command.
//!
//! These tests verify the overall behavior of the validation system.

use super::*;

#[test]
fn test_validation_details_creation() {
    // Test that ValidationDetails can be constructed correctly
    let details = ValidationDetails {
        average_complexity: 5.0,
        max_average_complexity: 10.0,
        high_complexity_count: 3,
        max_high_complexity_count: 5,
        debt_items: 10,
        max_debt_items: 20,
        total_debt_score: 150,
        max_total_debt_score: 300,
        debt_density: 0.15,
        max_debt_density: 0.20,
        codebase_risk_score: 25.5,
        max_codebase_risk_score: 50.0,
        high_risk_functions: 5,
        max_high_risk_functions: 10,
        coverage_percentage: 75.0,
        min_coverage_percentage: 60.0,
    };

    assert_eq!(details.average_complexity, 5.0);
    assert_eq!(details.max_average_complexity, 10.0);
    assert_eq!(details.high_complexity_count, 3);
    assert_eq!(details.max_high_complexity_count, 5);
    assert_eq!(details.debt_density, 0.15);
    assert_eq!(details.max_debt_density, 0.20);
    assert_eq!(details.debt_items, 10);
    assert_eq!(details.max_debt_items, 20);
    assert_eq!(details.total_debt_score, 150);
    assert_eq!(details.max_total_debt_score, 300);
    assert_eq!(details.codebase_risk_score, 25.5);
    assert_eq!(details.max_codebase_risk_score, 50.0);
    assert_eq!(details.high_risk_functions, 5);
    assert_eq!(details.max_high_risk_functions, 10);
    assert_eq!(details.coverage_percentage, 75.0);
    assert_eq!(details.min_coverage_percentage, 60.0);
}
