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

    // Calculate Pearson correlation coefficient
    let n = with_coverage.len() as f64;

    // Calculate means
    let mean_complexity: f64 = with_coverage
        .iter()
        .map(|f| (f.cyclomatic_complexity + f.cognitive_complexity) as f64 / 2.0)
        .sum::<f64>()
        / n;

    let mean_coverage: f64 = with_coverage
        .iter()
        .map(|f| f.coverage_percentage.unwrap())
        .sum::<f64>()
        / n;

    // Calculate covariance and standard deviations
    let mut covariance = 0.0;
    let mut std_dev_complexity = 0.0;
    let mut std_dev_coverage = 0.0;

    for func in &with_coverage {
        let complexity = (func.cyclomatic_complexity + func.cognitive_complexity) as f64 / 2.0;
        let coverage = func.coverage_percentage.unwrap();

        let diff_complexity = complexity - mean_complexity;
        let diff_coverage = coverage - mean_coverage;

        covariance += diff_complexity * diff_coverage;
        std_dev_complexity += diff_complexity * diff_complexity;
        std_dev_coverage += diff_coverage * diff_coverage;
    }

    let std_dev_complexity = (std_dev_complexity / n).sqrt();
    let std_dev_coverage = (std_dev_coverage / n).sqrt();

    if std_dev_complexity == 0.0 || std_dev_coverage == 0.0 {
        return None;
    }

    let correlation = covariance / (n * std_dev_complexity * std_dev_coverage);
    Some(correlation)
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
