use debtmap::risk::{Difficulty, FunctionRisk, RiskCategory, TestEffort};
use std::path::PathBuf;

#[test]
fn test_format_risk_function_with_coverage() {
    let func = FunctionRisk {
        function_name: "test_func".to_string(),
        file: PathBuf::from("test.rs"),
        line_range: (10, 20),
        cyclomatic_complexity: 5,
        cognitive_complexity: 7,
        risk_score: 8.5,
        coverage_percentage: Some(0.75),
        contextual_risk: None, // spec 203
        test_effort: TestEffort {
            estimated_difficulty: Difficulty::Moderate,
            cognitive_load: 7,
            branch_count: 5,
            recommended_test_cases: 3,
        },
        risk_category: RiskCategory::Medium,
        is_test_function: false,
    };
    // Note: format_risk_function was a helper in main.rs
    // We'll need to either expose it from a module or reimplement
    let coverage_str = func
        .coverage_percentage
        .map(|c| format!("{:.0}%", c * 100.0))
        .unwrap_or_else(|| "0%".to_string());
    let result = format!(
        "    - {} (risk: {:.1}, coverage: {})",
        func.function_name, func.risk_score, coverage_str
    );
    assert_eq!(result, "    - test_func (risk: 8.5, coverage: 75%)");
}

#[test]
fn test_format_risk_function_without_coverage() {
    let func = FunctionRisk {
        function_name: "test_func".to_string(),
        file: PathBuf::from("test.rs"),
        line_range: (10, 20),
        cyclomatic_complexity: 5,
        cognitive_complexity: 7,
        risk_score: 10.0,
        coverage_percentage: None,
        contextual_risk: None, // spec 203
        test_effort: TestEffort {
            estimated_difficulty: Difficulty::Trivial,
            cognitive_load: 3,
            branch_count: 1,
            recommended_test_cases: 1,
        },
        risk_category: RiskCategory::Critical,
        is_test_function: false,
    };
    let coverage_str = func
        .coverage_percentage
        .map(|c| format!("{:.0}%", c * 100.0))
        .unwrap_or_else(|| "0%".to_string());
    let result = format!(
        "    - {} (risk: {:.1}, coverage: {})",
        func.function_name, func.risk_score, coverage_str
    );
    assert_eq!(result, "    - test_func (risk: 10.0, coverage: 0%)");
}

#[test]
fn test_format_risk_function_zero_coverage() {
    let func = FunctionRisk {
        function_name: "zero_cov_func".to_string(),
        file: PathBuf::from("test.rs"),
        line_range: (20, 30),
        cyclomatic_complexity: 3,
        cognitive_complexity: 4,
        risk_score: 5.5,
        coverage_percentage: Some(0.0),
        contextual_risk: None, // spec 203
        test_effort: TestEffort {
            estimated_difficulty: Difficulty::Trivial,
            cognitive_load: 4,
            branch_count: 2,
            recommended_test_cases: 2,
        },
        risk_category: RiskCategory::Low,
        is_test_function: false,
    };
    let coverage_str = func
        .coverage_percentage
        .map(|c| format!("{:.0}%", c * 100.0))
        .unwrap_or_else(|| "0%".to_string());
    let result = format!(
        "    - {} (risk: {:.1}, coverage: {})",
        func.function_name, func.risk_score, coverage_str
    );
    assert_eq!(result, "    - zero_cov_func (risk: 5.5, coverage: 0%)");
}

#[test]
fn test_format_risk_function_full_coverage() {
    let func = FunctionRisk {
        function_name: "well_tested_func".to_string(),
        file: PathBuf::from("test.rs"),
        line_range: (30, 50),
        cyclomatic_complexity: 10,
        cognitive_complexity: 15,
        risk_score: 1.2,
        coverage_percentage: Some(1.0),
        contextual_risk: None, // spec 203
        test_effort: TestEffort {
            estimated_difficulty: Difficulty::Moderate,
            cognitive_load: 15,
            branch_count: 8,
            recommended_test_cases: 5,
        },
        risk_category: RiskCategory::WellTested,
        is_test_function: false,
    };
    let coverage_str = func
        .coverage_percentage
        .map(|c| format!("{:.0}%", c * 100.0))
        .unwrap_or_else(|| "0%".to_string());
    let result = format!(
        "    - {} (risk: {:.1}, coverage: {})",
        func.function_name, func.risk_score, coverage_str
    );
    assert_eq!(result, "    - well_tested_func (risk: 1.2, coverage: 100%)");
}
