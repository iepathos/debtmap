use super::super::commands::validate::ValidationDetails;
use crate::risk;

pub fn print_validation_success(details: &ValidationDetails, verbosity: u8) {
    println!("✅ Validation PASSED - All metrics within thresholds");

    if verbosity > 0 {
        println!();
        print_validation_details(details);
    }
}

pub fn print_validation_failure_with_details(
    details: &ValidationDetails,
    risk_insights: &Option<risk::RiskInsight>,
    verbosity: u8,
) {
    println!("❌ Validation FAILED - Some metrics exceed thresholds");
    println!();

    print_validation_details(details);

    println!("\n  Failed checks:");
    print_failed_validation_checks(details);

    if verbosity > 1 && risk_insights.is_some() {
        if let Some(insights) = risk_insights {
            print_risk_metrics(insights);
        }
    }
}

pub fn print_validation_details(details: &ValidationDetails) {
    println!("  Metrics Summary:");
    println!(
        "    Average complexity: {:.1} (threshold: {:.1})",
        details.average_complexity, details.max_average_complexity
    );
    println!(
        "    High complexity functions: {} (threshold: {})",
        details.high_complexity_count, details.max_high_complexity_count
    );
    println!(
        "    Technical debt items: {} (threshold: {})",
        details.debt_items, details.max_debt_items
    );
    println!(
        "    Total debt score: {} (threshold: {})",
        details.total_debt_score, details.max_total_debt_score
    );

    if details.max_codebase_risk_score > 0.0 || details.codebase_risk_score > 0.0 {
        println!(
            "    Codebase risk score: {:.1} (threshold: {:.1})",
            details.codebase_risk_score, details.max_codebase_risk_score
        );
    }

    if details.max_high_risk_functions > 0 || details.high_risk_functions > 0 {
        println!(
            "    High-risk functions: {} (threshold: {})",
            details.high_risk_functions, details.max_high_risk_functions
        );
    }

    if details.min_coverage_percentage > 0.0 || details.coverage_percentage > 0.0 {
        println!(
            "    Code coverage: {:.1}% (minimum: {:.1}%)",
            details.coverage_percentage, details.min_coverage_percentage
        );
    }
}

fn print_failed_validation_checks(details: &ValidationDetails) {
    // Define validation checks as data
    let checks = create_validation_checks(details);

    // Process each check functionally
    checks
        .into_iter()
        .filter(|check| check.is_failed())
        .for_each(|check| println!("{}", check.format_failure()));
}

/// Represents a single validation check
struct ValidationCheck {
    metric_name: &'static str,
    actual: String,
    threshold: String,
    comparison: &'static str,
    failed: bool,
}

impl ValidationCheck {
    fn is_failed(&self) -> bool {
        self.failed
    }

    fn format_failure(&self) -> String {
        format_threshold_failure(
            self.metric_name,
            &self.actual,
            &self.threshold,
            self.comparison,
        )
    }
}

/// Creates all validation checks from details
fn create_validation_checks(details: &ValidationDetails) -> Vec<ValidationCheck> {
    vec![
        ValidationCheck {
            metric_name: "Average complexity",
            actual: format!("{:.1}", details.average_complexity),
            threshold: format!("{:.1}", details.max_average_complexity),
            comparison: ">",
            failed: exceeds_max_threshold(
                details.average_complexity,
                details.max_average_complexity,
            ),
        },
        ValidationCheck {
            metric_name: "High complexity functions",
            actual: details.high_complexity_count.to_string(),
            threshold: details.max_high_complexity_count.to_string(),
            comparison: ">",
            failed: exceeds_max_threshold(
                details.high_complexity_count,
                details.max_high_complexity_count,
            ),
        },
        ValidationCheck {
            metric_name: "Technical debt items",
            actual: details.debt_items.to_string(),
            threshold: details.max_debt_items.to_string(),
            comparison: ">",
            failed: exceeds_max_threshold(details.debt_items, details.max_debt_items),
        },
        ValidationCheck {
            metric_name: "Total debt score",
            actual: details.total_debt_score.to_string(),
            threshold: details.max_total_debt_score.to_string(),
            comparison: ">",
            failed: exceeds_max_threshold(details.total_debt_score, details.max_total_debt_score),
        },
        ValidationCheck {
            metric_name: "Codebase risk score",
            actual: format!("{:.1}", details.codebase_risk_score),
            threshold: format!("{:.1}", details.max_codebase_risk_score),
            comparison: ">",
            failed: details.max_codebase_risk_score > 0.0
                && exceeds_max_threshold(
                    details.codebase_risk_score,
                    details.max_codebase_risk_score,
                ),
        },
        ValidationCheck {
            metric_name: "High-risk functions",
            actual: details.high_risk_functions.to_string(),
            threshold: details.max_high_risk_functions.to_string(),
            comparison: ">",
            failed: details.max_high_risk_functions > 0
                && exceeds_max_threshold(
                    details.high_risk_functions,
                    details.max_high_risk_functions,
                ),
        },
        ValidationCheck {
            metric_name: "Code coverage",
            actual: format!("{:.1}%", details.coverage_percentage),
            threshold: format!("{:.1}%", details.min_coverage_percentage),
            comparison: "<",
            failed: details.min_coverage_percentage > 0.0
                && below_min_threshold(
                    details.coverage_percentage,
                    details.min_coverage_percentage,
                ),
        },
    ]
}

fn format_threshold_failure(
    metric_name: &str,
    actual: &str,
    threshold: &str,
    comparison: &str,
) -> String {
    format!(
        "    ❌ {}: {} {} {}",
        metric_name, actual, comparison, threshold
    )
}

fn exceeds_max_threshold<T: PartialOrd>(actual: T, threshold: T) -> bool {
    actual > threshold
}

fn below_min_threshold<T: PartialOrd>(actual: T, threshold: T) -> bool {
    actual < threshold
}

fn print_risk_metrics(insights: &risk::RiskInsight) {
    println!(
        "\n  Overall codebase risk score: {:.1}",
        insights.codebase_risk_score
    );

    if !insights.top_risks.is_empty() {
        println!("\n  Critical risk functions (high complexity + low/no coverage):");
        insights
            .top_risks
            .iter()
            .take(5)
            .for_each(print_risk_function);
    }
}

fn print_risk_function(func: &risk::FunctionRisk) {
    let formatted = format_risk_function(func);
    println!("{formatted}");
}

fn format_risk_function(func: &risk::FunctionRisk) -> String {
    let coverage_str = func
        .coverage_percentage
        .map(|c| format!("{:.0}%", c * 100.0))
        .unwrap_or_else(|| "0%".to_string());
    format!(
        "    - {} (risk: {:.1}, coverage: {})",
        func.function_name, func.risk_score, coverage_str
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::validate::ValidationDetails;

    #[test]
    fn test_exceeds_max_threshold() {
        assert!(exceeds_max_threshold(10, 5));
        assert!(!exceeds_max_threshold(5, 10));
        assert!(!exceeds_max_threshold(5, 5));
    }

    #[test]
    fn test_below_min_threshold() {
        assert!(below_min_threshold(5, 10));
        assert!(!below_min_threshold(10, 5));
        assert!(!below_min_threshold(5, 5));
    }

    #[test]
    fn test_validation_check_is_failed() {
        let check = ValidationCheck {
            metric_name: "Test metric",
            actual: "10".to_string(),
            threshold: "5".to_string(),
            comparison: ">",
            failed: true,
        };
        assert!(check.is_failed());

        let check_passed = ValidationCheck {
            metric_name: "Test metric",
            actual: "5".to_string(),
            threshold: "10".to_string(),
            comparison: ">",
            failed: false,
        };
        assert!(!check_passed.is_failed());
    }

    #[test]
    fn test_validation_check_format_failure() {
        let check = ValidationCheck {
            metric_name: "Test metric",
            actual: "10".to_string(),
            threshold: "5".to_string(),
            comparison: ">",
            failed: true,
        };
        let formatted = check.format_failure();
        assert!(formatted.contains("Test metric"));
        assert!(formatted.contains("10"));
        assert!(formatted.contains("5"));
        assert!(formatted.contains(">"));
        assert!(formatted.contains("❌"));
    }

    #[test]
    fn test_create_validation_checks_all_thresholds_passed() {
        let details = ValidationDetails {
            average_complexity: 5.0,
            max_average_complexity: 10.0,
            high_complexity_count: 2,
            max_high_complexity_count: 5,
            debt_items: 10,
            max_debt_items: 20,
            total_debt_score: 50,
            max_total_debt_score: 100,
            codebase_risk_score: 3.0,
            max_codebase_risk_score: 5.0,
            high_risk_functions: 1,
            max_high_risk_functions: 3,
            coverage_percentage: 80.0,
            min_coverage_percentage: 70.0,
        };

        let checks = create_validation_checks(&details);
        assert_eq!(checks.len(), 7);

        // All checks should pass
        let failed_count = checks.iter().filter(|c| c.is_failed()).count();
        assert_eq!(failed_count, 0);
    }

    #[test]
    fn test_create_validation_checks_some_thresholds_failed() {
        let details = ValidationDetails {
            average_complexity: 15.0, // Exceeds threshold
            max_average_complexity: 10.0,
            high_complexity_count: 10, // Exceeds threshold
            max_high_complexity_count: 5,
            debt_items: 10, // Within threshold
            max_debt_items: 20,
            total_debt_score: 50, // Within threshold
            max_total_debt_score: 100,
            codebase_risk_score: 3.0, // Within threshold
            max_codebase_risk_score: 5.0,
            high_risk_functions: 1, // Within threshold
            max_high_risk_functions: 3,
            coverage_percentage: 60.0, // Below threshold
            min_coverage_percentage: 70.0,
        };

        let checks = create_validation_checks(&details);

        // Count failed checks
        let failed_checks: Vec<_> = checks
            .iter()
            .filter(|c| c.is_failed())
            .map(|c| c.metric_name)
            .collect();

        assert_eq!(failed_checks.len(), 3);
        assert!(failed_checks.contains(&"Average complexity"));
        assert!(failed_checks.contains(&"High complexity functions"));
        assert!(failed_checks.contains(&"Code coverage"));
    }

    #[test]
    fn test_create_validation_checks_disabled_thresholds() {
        let details = ValidationDetails {
            average_complexity: 15.0,
            max_average_complexity: 10.0,
            high_complexity_count: 10,
            max_high_complexity_count: 5,
            debt_items: 10,
            max_debt_items: 20,
            total_debt_score: 50,
            max_total_debt_score: 100,
            codebase_risk_score: 10.0,    // Would exceed if enabled
            max_codebase_risk_score: 0.0, // Disabled (0.0)
            high_risk_functions: 10,      // Would exceed if enabled
            max_high_risk_functions: 0,   // Disabled (0)
            coverage_percentage: 30.0,    // Would fail if enabled
            min_coverage_percentage: 0.0, // Disabled (0.0)
        };

        let checks = create_validation_checks(&details);

        // These disabled checks should not be marked as failed
        let risk_check = checks
            .iter()
            .find(|c| c.metric_name == "Codebase risk score")
            .unwrap();
        assert!(!risk_check.is_failed());

        let high_risk_check = checks
            .iter()
            .find(|c| c.metric_name == "High-risk functions")
            .unwrap();
        assert!(!high_risk_check.is_failed());

        let coverage_check = checks
            .iter()
            .find(|c| c.metric_name == "Code coverage")
            .unwrap();
        assert!(!coverage_check.is_failed());
    }

    #[test]
    fn test_format_threshold_failure() {
        let result = format_threshold_failure("Test metric", "100", "50", ">");
        assert_eq!(result, "    ❌ Test metric: 100 > 50");

        let result = format_threshold_failure("Coverage", "40%", "60%", "<");
        assert_eq!(result, "    ❌ Coverage: 40% < 60%");
    }
}
