use super::{FunctionRisk, RiskAnalyzer, TestingRecommendation};
use im::Vector;

pub fn prioritize_by_roi(
    functions: &Vector<FunctionRisk>,
    analyzer: &RiskAnalyzer,
) -> Vector<TestingRecommendation> {
    let mut recommendations = Vector::new();

    // Calculate total risk for normalization
    let total_risk: f64 = functions.iter().map(|f| f.risk_score).sum();
    if total_risk == 0.0 {
        return recommendations;
    }

    // Sort by ROI (risk reduction per unit of effort)
    let mut sorted_functions: Vec<&FunctionRisk> = functions.iter().collect();
    sorted_functions.sort_by(|a, b| {
        let roi_a = calculate_roi(a);
        let roi_b = calculate_roi(b);
        roi_b.partial_cmp(&roi_a).unwrap()
    });

    // Take top 5 functions with best ROI
    for func in sorted_functions.into_iter().take(5) {
        let risk_reduction = analyzer.calculate_risk_reduction(
            func.risk_score,
            func.cyclomatic_complexity,
            90.0, // Target coverage
        );

        let recommendation = TestingRecommendation {
            function: func.function_name.clone(),
            file: func.file.clone(),
            current_risk: func.risk_score,
            potential_risk_reduction: (risk_reduction / total_risk) * 100.0,
            test_effort_estimate: func.test_effort.clone(),
            rationale: generate_rationale(func),
        };

        recommendations.push_back(recommendation);
    }

    recommendations
}

fn calculate_roi(func: &FunctionRisk) -> f64 {
    // ROI = risk_score / effort
    // Higher risk and lower effort = higher ROI
    let effort = func.test_effort.cognitive_load as f64 + 1.0; // +1 to avoid division by zero
    func.risk_score / effort
}

fn generate_rationale(func: &FunctionRisk) -> String {
    match (&func.risk_category, func.coverage_percentage) {
        (super::RiskCategory::Critical, Some(cov)) if cov < 10.0 => {
            format!(
                "Critical complexity (cyclo={}, cognitive={}) with no test coverage",
                func.cyclomatic_complexity, func.cognitive_complexity
            )
        }
        (super::RiskCategory::Critical, None) => {
            format!(
                "Critical complexity (cyclo={}, cognitive={}) - should be tested first",
                func.cyclomatic_complexity, func.cognitive_complexity
            )
        }
        (super::RiskCategory::High, Some(cov)) => {
            format!(
                "High complexity with only {:.0}% coverage - significant risk reduction potential",
                cov
            )
        }
        (super::RiskCategory::High, None) => {
            format!("High complexity function - testing would significantly reduce risk")
        }
        _ => {
            format!(
                "Complexity score {} suggests {} test cases needed",
                func.cognitive_complexity, func.test_effort.recommended_test_cases
            )
        }
    }
}

pub fn identify_untested_complex_functions(
    functions: &Vector<FunctionRisk>,
    complexity_threshold: u32,
) -> Vector<FunctionRisk> {
    functions
        .iter()
        .filter(|f| {
            let avg_complexity = (f.cyclomatic_complexity + f.cognitive_complexity) / 2;
            match f.coverage_percentage {
                Some(cov) => avg_complexity > complexity_threshold && cov < 30.0,
                None => avg_complexity > complexity_threshold,
            }
        })
        .cloned()
        .collect()
}

pub fn identify_well_tested_complex_functions(
    functions: &Vector<FunctionRisk>,
    complexity_threshold: u32,
    coverage_threshold: f64,
) -> Vector<FunctionRisk> {
    functions
        .iter()
        .filter(|f| {
            let avg_complexity = (f.cyclomatic_complexity + f.cognitive_complexity) / 2;
            match f.coverage_percentage {
                Some(cov) => avg_complexity > complexity_threshold && cov >= coverage_threshold,
                None => false,
            }
        })
        .cloned()
        .collect()
}

pub fn calculate_dynamic_coverage_threshold(complexity: u32) -> f64 {
    // Dynamic threshold: more complex code needs higher coverage
    // Base: 50% + 2% per complexity point, max 100%
    let threshold = 50.0 + (complexity as f64 * 2.0);
    threshold.min(100.0)
}
