use super::super::commands::validate::ValidationDetails;
use crate::risk;

pub fn print_validation_success(details: &ValidationDetails, verbosity: u8) {
    println!("[OK] Validation PASSED - All metrics within thresholds");

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
    println!("[ERROR] Validation FAILED - Some metrics exceed thresholds");
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
    println!("  Primary Quality Metrics:");

    // Emphasize debt density as THE primary metric
    println!(
        "    📊 Debt Density: {:.1} per 1K LOC (threshold: {:.1})",
        details.debt_density, details.max_debt_density
    );

    // Show percentage of threshold used - helps users understand headroom
    let density_usage = if details.max_debt_density > 0.0 {
        (details.debt_density / details.max_debt_density) * 100.0
    } else {
        0.0
    };
    let density_headroom = 100.0 - density_usage;
    println!(
        "       └─ Using {:.0}% of max density ({:.0}% headroom)",
        density_usage, density_headroom
    );

    // Show other primary quality metrics
    println!(
        "    Average complexity: {:.1} (threshold: {:.1})",
        details.average_complexity, details.max_average_complexity
    );

    if details.max_codebase_risk_score > 0.0 || details.codebase_risk_score > 0.0 {
        println!(
            "    Codebase risk score: {:.1} (threshold: {:.1})",
            details.codebase_risk_score, details.max_codebase_risk_score
        );
    }

    if details.min_coverage_percentage > 0.0 || details.coverage_percentage > 0.0 {
        println!(
            "    Code coverage: {:.1}% (minimum: {:.1}%)",
            details.coverage_percentage, details.min_coverage_percentage
        );
    }

    // Show absolute counts as informational (not primary validation criteria)
    println!("\n  📈 Codebase Statistics (informational):");
    println!(
        "    High complexity functions: {}",
        details.high_complexity_count
    );
    println!("    Technical debt items: {}", details.debt_items);
    println!(
        "    Total debt score: {} (safety net threshold: {})",
        details.total_debt_score, details.max_total_debt_score
    );

    // Show deprecated metrics only if they are set (non-zero)
    if details.max_high_complexity_count > 0 || details.max_debt_items > 0 {
        println!("\n  ⚠️  Deprecated Thresholds (if configured):");
        if details.max_high_complexity_count > 0 {
            println!(
                "    High complexity functions: {} (threshold: {})",
                details.high_complexity_count, details.max_high_complexity_count
            );
        }
        if details.max_debt_items > 0 {
            println!(
                "    Technical debt items: {} (threshold: {})",
                details.debt_items, details.max_debt_items
            );
        }
        if details.max_high_risk_functions > 0 {
            println!(
                "    High-risk functions: {} (threshold: {})",
                details.high_risk_functions, details.max_high_risk_functions
            );
        }
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
    let mut checks = vec![
        // Primary quality metrics
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
            metric_name: "Debt density",
            actual: format!("{:.1} per 1K LOC", details.debt_density),
            threshold: format!("{:.1}", details.max_debt_density),
            comparison: ">",
            failed: exceeds_max_threshold(details.debt_density, details.max_debt_density),
        },
        ValidationCheck {
            metric_name: "Total debt score (safety net)",
            actual: details.total_debt_score.to_string(),
            threshold: details.max_total_debt_score.to_string(),
            comparison: ">",
            failed: exceeds_max_threshold(details.total_debt_score, details.max_total_debt_score),
        },
    ];

    // Add optional metrics if configured
    if details.max_codebase_risk_score > 0.0 || details.codebase_risk_score > 0.0 {
        checks.push(ValidationCheck {
            metric_name: "Codebase risk score",
            actual: format!("{:.1}", details.codebase_risk_score),
            threshold: format!("{:.1}", details.max_codebase_risk_score),
            comparison: ">",
            failed: details.max_codebase_risk_score > 0.0
                && exceeds_max_threshold(
                    details.codebase_risk_score,
                    details.max_codebase_risk_score,
                ),
        });
    }

    if details.min_coverage_percentage > 0.0 || details.coverage_percentage > 0.0 {
        checks.push(ValidationCheck {
            metric_name: "Code coverage",
            actual: format!("{:.1}%", details.coverage_percentage),
            threshold: format!("{:.1}%", details.min_coverage_percentage),
            comparison: "<",
            failed: details.min_coverage_percentage > 0.0
                && below_min_threshold(
                    details.coverage_percentage,
                    details.min_coverage_percentage,
                ),
        });
    }

    // Add deprecated metrics only if explicitly configured (non-zero)
    if details.max_high_complexity_count > 0 {
        checks.push(ValidationCheck {
            metric_name: "High complexity functions (deprecated)",
            actual: details.high_complexity_count.to_string(),
            threshold: details.max_high_complexity_count.to_string(),
            comparison: ">",
            failed: exceeds_max_threshold(
                details.high_complexity_count,
                details.max_high_complexity_count,
            ),
        });
    }

    if details.max_debt_items > 0 {
        checks.push(ValidationCheck {
            metric_name: "Technical debt items (deprecated)",
            actual: details.debt_items.to_string(),
            threshold: details.max_debt_items.to_string(),
            comparison: ">",
            failed: exceeds_max_threshold(details.debt_items, details.max_debt_items),
        });
    }

    if details.max_high_risk_functions > 0 {
        checks.push(ValidationCheck {
            metric_name: "High-risk functions (deprecated)",
            actual: details.high_risk_functions.to_string(),
            threshold: details.max_high_risk_functions.to_string(),
            comparison: ">",
            failed: exceeds_max_threshold(
                details.high_risk_functions,
                details.max_high_risk_functions,
            ),
        });
    }

    checks
}

fn format_threshold_failure(
    metric_name: &str,
    actual: &str,
    threshold: &str,
    comparison: &str,
) -> String {
    format!(
        "    [ERROR] {}: {} {} {}",
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
        assert!(formatted.contains("[ERROR]"));
    }

    #[test]
    fn test_create_validation_checks_all_thresholds_passed() {
        let details = ValidationDetails {
            average_complexity: 5.0,
            max_average_complexity: 10.0,
            high_complexity_count: 2,
            max_high_complexity_count: 0, // Not configured (deprecated)
            debt_items: 10,
            max_debt_items: 0, // Not configured (deprecated)
            total_debt_score: 50,
            max_total_debt_score: 100,
            debt_density: 10.0,
            max_debt_density: 50.0,
            codebase_risk_score: 3.0,
            max_codebase_risk_score: 5.0,
            high_risk_functions: 1,
            max_high_risk_functions: 0, // Not configured (deprecated)
            coverage_percentage: 80.0,
            min_coverage_percentage: 70.0,
        };

        let checks = create_validation_checks(&details);
        // Should have: avg complexity, debt density, total debt score, risk score, coverage
        // Deprecated metrics are NOT included when set to 0
        assert_eq!(checks.len(), 5);

        // All checks should pass
        let failed_count = checks.iter().filter(|c| c.is_failed()).count();
        assert_eq!(failed_count, 0);
    }

    #[test]
    fn test_create_validation_checks_some_thresholds_failed() {
        let details = ValidationDetails {
            average_complexity: 15.0, // Exceeds threshold
            max_average_complexity: 10.0,
            high_complexity_count: 10,    // Would exceed if configured
            max_high_complexity_count: 5, // Configured (deprecated)
            debt_items: 10,               // Within deprecated threshold
            max_debt_items: 20,           // Configured (deprecated)
            total_debt_score: 50,         // Within threshold
            max_total_debt_score: 100,
            debt_density: 25.0, // Within threshold
            max_debt_density: 50.0,
            codebase_risk_score: 3.0, // Within threshold
            max_codebase_risk_score: 5.0,
            high_risk_functions: 1,     // Within deprecated threshold
            max_high_risk_functions: 3, // Configured (deprecated)
            coverage_percentage: 60.0,  // Below threshold
            min_coverage_percentage: 70.0,
        };

        let checks = create_validation_checks(&details);

        // Count failed checks
        let failed_checks: Vec<_> = checks
            .iter()
            .filter(|c| c.is_failed())
            .map(|c| c.metric_name)
            .collect();

        // Should fail: avg complexity, high complexity (deprecated), coverage
        assert_eq!(failed_checks.len(), 3);
        assert!(failed_checks.contains(&"Average complexity"));
        assert!(failed_checks.contains(&"High complexity functions (deprecated)"));
        assert!(failed_checks.contains(&"Code coverage"));
    }

    #[test]
    fn test_create_validation_checks_disabled_thresholds() {
        let details = ValidationDetails {
            average_complexity: 15.0,
            max_average_complexity: 10.0,
            high_complexity_count: 10,
            max_high_complexity_count: 0, // Disabled (deprecated)
            debt_items: 10,
            max_debt_items: 0, // Disabled (deprecated)
            total_debt_score: 50,
            max_total_debt_score: 100,
            debt_density: 30.0,
            max_debt_density: 50.0,
            codebase_risk_score: 0.0,     // Disabled
            max_codebase_risk_score: 0.0, // Disabled (0.0)
            high_risk_functions: 0,       // Disabled
            max_high_risk_functions: 0,   // Disabled (deprecated)
            coverage_percentage: 0.0,     // Disabled
            min_coverage_percentage: 0.0, // Disabled (0.0)
        };

        let checks = create_validation_checks(&details);

        // Disabled checks should NOT be included when thresholds are 0/None
        // Should only have: avg complexity, debt density, total debt score
        assert_eq!(checks.len(), 3);

        // Risk check should not exist when both values are 0.0
        let risk_check = checks
            .iter()
            .find(|c| c.metric_name == "Codebase risk score");
        assert!(risk_check.is_none());

        // Deprecated high-risk functions check should not exist when threshold is 0
        let high_risk_check = checks
            .iter()
            .find(|c| c.metric_name.contains("High-risk functions"));
        assert!(high_risk_check.is_none());

        // Coverage check should not exist when both values are 0.0
        let coverage_check = checks
            .iter()
            .find(|c| c.metric_name == "Code coverage");
        assert!(coverage_check.is_none());

        // Verify that primary metrics are present
        assert!(checks
            .iter()
            .any(|c| c.metric_name == "Average complexity"));
        assert!(checks.iter().any(|c| c.metric_name == "Debt density"));
        assert!(checks
            .iter()
            .any(|c| c.metric_name == "Total debt score (safety net)"));
    }

    #[test]
    fn test_format_threshold_failure() {
        let result = format_threshold_failure("Test metric", "100", "50", ">");
        assert_eq!(result, "    [ERROR] Test metric: 100 > 50");

        let result = format_threshold_failure("Coverage", "40%", "60%", "<");
        assert_eq!(result, "    [ERROR] Coverage: 40% < 60%");
    }
}
