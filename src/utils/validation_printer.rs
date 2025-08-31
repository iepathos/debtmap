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
    if exceeds_max_threshold(details.average_complexity, details.max_average_complexity) {
        println!(
            "{}",
            format_threshold_failure(
                "Average complexity",
                &format!("{:.1}", details.average_complexity),
                &format!("{:.1}", details.max_average_complexity),
                ">"
            )
        );
    }
    if exceeds_max_threshold(
        details.high_complexity_count,
        details.max_high_complexity_count,
    ) {
        println!(
            "{}",
            format_threshold_failure(
                "High complexity functions",
                &details.high_complexity_count.to_string(),
                &details.max_high_complexity_count.to_string(),
                ">"
            )
        );
    }
    if exceeds_max_threshold(details.debt_items, details.max_debt_items) {
        println!(
            "{}",
            format_threshold_failure(
                "Technical debt items",
                &details.debt_items.to_string(),
                &details.max_debt_items.to_string(),
                ">"
            )
        );
    }
    if exceeds_max_threshold(details.total_debt_score, details.max_total_debt_score) {
        println!(
            "{}",
            format_threshold_failure(
                "Total debt score",
                &details.total_debt_score.to_string(),
                &details.max_total_debt_score.to_string(),
                ">"
            )
        );
    }
    if details.max_codebase_risk_score > 0.0
        && exceeds_max_threshold(details.codebase_risk_score, details.max_codebase_risk_score)
    {
        println!(
            "{}",
            format_threshold_failure(
                "Codebase risk score",
                &format!("{:.1}", details.codebase_risk_score),
                &format!("{:.1}", details.max_codebase_risk_score),
                ">"
            )
        );
    }
    if details.max_high_risk_functions > 0
        && exceeds_max_threshold(details.high_risk_functions, details.max_high_risk_functions)
    {
        println!(
            "{}",
            format_threshold_failure(
                "High-risk functions",
                &details.high_risk_functions.to_string(),
                &details.max_high_risk_functions.to_string(),
                ">"
            )
        );
    }
    if details.min_coverage_percentage > 0.0
        && below_min_threshold(details.coverage_percentage, details.min_coverage_percentage)
    {
        println!(
            "{}",
            format_threshold_failure(
                "Code coverage",
                &format!("{:.1}%", details.coverage_percentage),
                &format!("{:.1}%", details.min_coverage_percentage),
                "<"
            )
        );
    }
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
