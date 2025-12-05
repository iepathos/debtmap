use super::{FunctionRisk, RiskCategory, RiskDistribution, RiskInsight};
use im::Vector;

pub fn calculate_complexity_coverage_correlation(functions: &Vector<FunctionRisk>) -> Option<f64> {
    // Only calculate correlation if we have coverage data
    let with_coverage: Vec<&FunctionRisk> = functions
        .iter()
        .filter(|f| f.coverage_percentage.is_some())
        .collect();

    if with_coverage.len() < 2 {
        return None;
    }

    // Extract complexity and coverage values
    let complexities: Vec<f64> = with_coverage
        .iter()
        .map(|f| (f.cyclomatic_complexity + f.cognitive_complexity) as f64 / 2.0)
        .collect();

    let coverages: Vec<f64> = with_coverage
        .iter()
        .map(|f| f.coverage_percentage.unwrap())
        .collect();

    calculate_pearson_correlation(&complexities, &coverages)
}

fn calculate_pearson_correlation(x_values: &[f64], y_values: &[f64]) -> Option<f64> {
    let n = x_values.len() as f64;

    let mean_x = x_values.iter().sum::<f64>() / n;
    let mean_y = y_values.iter().sum::<f64>() / n;

    // Calculate statistics using functional approach
    let (covariance, variance_x, variance_y) = x_values
        .iter()
        .zip(y_values.iter())
        .map(|(x, y)| {
            let diff_x = x - mean_x;
            let diff_y = y - mean_y;
            (diff_x * diff_y, diff_x * diff_x, diff_y * diff_y)
        })
        .fold((0.0, 0.0, 0.0), |acc, (cov, var_x, var_y)| {
            (acc.0 + cov, acc.1 + var_x, acc.2 + var_y)
        });

    let std_dev_x = (variance_x / n).sqrt();
    let std_dev_y = (variance_y / n).sqrt();

    if std_dev_x == 0.0 || std_dev_y == 0.0 {
        return None;
    }

    Some(covariance / (n * std_dev_x * std_dev_y))
}

pub fn calculate_codebase_risk_score(functions: &Vector<FunctionRisk>) -> f64 {
    if functions.is_empty() {
        return 0.0;
    }

    let total_risk: f64 = functions.iter().map(|f| f.risk_score).sum();
    let max_possible_risk = functions.len() as f64 * 50.0; // Assuming max risk of 50 per function

    (total_risk / max_possible_risk) * 100.0
}

pub fn build_risk_distribution(functions: &Vector<FunctionRisk>) -> RiskDistribution {
    let mut dist = RiskDistribution {
        critical_count: 0,
        high_count: 0,
        medium_count: 0,
        low_count: 0,
        well_tested_count: 0,
        total_functions: functions.len(),
    };

    for func in functions {
        match func.risk_category {
            RiskCategory::Critical => dist.critical_count += 1,
            RiskCategory::High => dist.high_count += 1,
            RiskCategory::Medium => dist.medium_count += 1,
            RiskCategory::Low => dist.low_count += 1,
            RiskCategory::WellTested => dist.well_tested_count += 1,
        }
    }

    dist
}

pub fn analyze_risk_insights(functions: Vector<FunctionRisk>) -> RiskInsight {
    let codebase_risk_score = calculate_codebase_risk_score(&functions);
    let complexity_coverage_correlation = calculate_complexity_coverage_correlation(&functions);
    let risk_distribution = build_risk_distribution(&functions);

    // Sort functions by risk score to get top risks
    let mut sorted_functions = functions.clone();
    sorted_functions.sort_by(|a, b| b.risk_score.partial_cmp(&a.risk_score).unwrap());

    let top_risks = sorted_functions.into_iter().take(10).collect();

    RiskInsight {
        top_risks,
        risk_reduction_opportunities: Vector::new(), // Will be populated by priority module
        codebase_risk_score,
        complexity_coverage_correlation,
        risk_distribution,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::{Difficulty, TestEffort};
    use std::path::PathBuf;

    fn create_test_function_risk(risk_category: RiskCategory) -> FunctionRisk {
        FunctionRisk {
            file: PathBuf::from("test.rs"),
            function_name: "test_function".to_string(),
            line_range: (1, 10),
            cyclomatic_complexity: 5,
            cognitive_complexity: 5,
            coverage_percentage: Some(50.0),
            risk_score: 10.0,
            contextual_risk: None,
            test_effort: TestEffort {
                estimated_difficulty: Difficulty::Simple,
                cognitive_load: 5,
                branch_count: 2,
                recommended_test_cases: 3,
            },
            risk_category,
            is_test_function: false,
        }
    }

    #[test]
    fn test_build_risk_distribution_empty() {
        let functions = Vector::new();
        let dist = build_risk_distribution(&functions);

        assert_eq!(dist.critical_count, 0);
        assert_eq!(dist.high_count, 0);
        assert_eq!(dist.medium_count, 0);
        assert_eq!(dist.low_count, 0);
        assert_eq!(dist.well_tested_count, 0);
        assert_eq!(dist.total_functions, 0);
    }

    #[test]
    fn test_build_risk_distribution_single_critical() {
        let mut functions = Vector::new();
        functions.push_back(create_test_function_risk(RiskCategory::Critical));

        let dist = build_risk_distribution(&functions);

        assert_eq!(dist.critical_count, 1);
        assert_eq!(dist.high_count, 0);
        assert_eq!(dist.medium_count, 0);
        assert_eq!(dist.low_count, 0);
        assert_eq!(dist.well_tested_count, 0);
        assert_eq!(dist.total_functions, 1);
    }

    #[test]
    fn test_build_risk_distribution_mixed_categories() {
        let mut functions = Vector::new();
        functions.push_back(create_test_function_risk(RiskCategory::Critical));
        functions.push_back(create_test_function_risk(RiskCategory::High));
        functions.push_back(create_test_function_risk(RiskCategory::Medium));
        functions.push_back(create_test_function_risk(RiskCategory::Low));
        functions.push_back(create_test_function_risk(RiskCategory::WellTested));

        let dist = build_risk_distribution(&functions);

        assert_eq!(dist.critical_count, 1);
        assert_eq!(dist.high_count, 1);
        assert_eq!(dist.medium_count, 1);
        assert_eq!(dist.low_count, 1);
        assert_eq!(dist.well_tested_count, 1);
        assert_eq!(dist.total_functions, 5);
    }

    #[test]
    fn test_build_risk_distribution_multiple_same_category() {
        let mut functions = Vector::new();
        functions.push_back(create_test_function_risk(RiskCategory::High));
        functions.push_back(create_test_function_risk(RiskCategory::High));
        functions.push_back(create_test_function_risk(RiskCategory::High));

        let dist = build_risk_distribution(&functions);

        assert_eq!(dist.critical_count, 0);
        assert_eq!(dist.high_count, 3);
        assert_eq!(dist.medium_count, 0);
        assert_eq!(dist.low_count, 0);
        assert_eq!(dist.well_tested_count, 0);
        assert_eq!(dist.total_functions, 3);
    }

    #[test]
    fn test_build_risk_distribution_all_well_tested() {
        let mut functions = Vector::new();
        for _ in 0..10 {
            functions.push_back(create_test_function_risk(RiskCategory::WellTested));
        }

        let dist = build_risk_distribution(&functions);

        assert_eq!(dist.critical_count, 0);
        assert_eq!(dist.high_count, 0);
        assert_eq!(dist.medium_count, 0);
        assert_eq!(dist.low_count, 0);
        assert_eq!(dist.well_tested_count, 10);
        assert_eq!(dist.total_functions, 10);
    }

    #[test]
    fn test_build_risk_distribution_large_dataset() {
        let mut functions = Vector::new();

        // Add 100 functions with various risk categories
        for i in 0..100 {
            let category = match i % 5 {
                0 => RiskCategory::Critical,
                1 => RiskCategory::High,
                2 => RiskCategory::Medium,
                3 => RiskCategory::Low,
                _ => RiskCategory::WellTested,
            };
            functions.push_back(create_test_function_risk(category));
        }

        let dist = build_risk_distribution(&functions);

        assert_eq!(dist.critical_count, 20);
        assert_eq!(dist.high_count, 20);
        assert_eq!(dist.medium_count, 20);
        assert_eq!(dist.low_count, 20);
        assert_eq!(dist.well_tested_count, 20);
        assert_eq!(dist.total_functions, 100);
    }
}
