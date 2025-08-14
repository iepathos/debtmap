use crate::core::{AnalysisResults, FunctionMetrics, Priority};
use crate::debt::total_debt_score;
use crate::io::output::OutputWriter;
use crate::refactoring::{ComplexityLevel, PatternRecognitionEngine};
use crate::risk::RiskInsight;
use colored::*;

pub struct TerminalWriter;

impl Default for TerminalWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalWriter {
    pub fn new() -> Self {
        Self
    }
}

impl OutputWriter for TerminalWriter {
    fn write_results(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        let printers: Vec<fn(&AnalysisResults)> = vec![
            |_| print_header(),
            print_summary,
            print_complexity_hotspots,
            print_technical_debt,
            print_pass_fail_status,
        ];

        printers.iter().for_each(|printer| printer(results));
        Ok(())
    }

    fn write_risk_insights(&mut self, insights: &RiskInsight) -> anyhow::Result<()> {
        use crate::risk::insights::{
            format_actionable_insights, format_critical_risks, format_recommendations,
        };

        // Print risk header
        println!();
        let divider = "═══════════════════════════════════════════".cyan();
        let title = "           RISK ANALYSIS REPORT".bold().cyan();
        println!("{divider}");
        println!("{title}");
        println!("{divider}");
        println!();

        // Print risk summary
        println!("📈 {} Summary", "RISK".bold());
        println!("───────────────────────────────────────────");
        println!(
            "Codebase Risk Score: {:.1} ({})",
            insights.codebase_risk_score,
            if insights.codebase_risk_score < 30.0 {
                "LOW".green()
            } else if insights.codebase_risk_score < 60.0 {
                "MEDIUM".yellow()
            } else {
                "HIGH".red()
            }
        );

        if let Some(correlation) = insights.complexity_coverage_correlation {
            println!("Complexity-Coverage Correlation: {correlation:.2}");
        }
        println!();

        // Print risk distribution
        println!("Risk Distribution:");
        println!(
            "  Critical: {} functions",
            insights.risk_distribution.critical_count.to_string().red()
        );
        println!(
            "  High: {} functions",
            insights.risk_distribution.high_count.to_string().yellow()
        );
        println!(
            "  Medium: {} functions",
            insights.risk_distribution.medium_count
        );
        println!(
            "  Low: {} functions",
            insights.risk_distribution.low_count.to_string().green()
        );
        println!(
            "  Well Tested: {} functions",
            insights
                .risk_distribution
                .well_tested_count
                .to_string()
                .cyan()
        );
        println!();

        // Print critical risks
        let critical_risks_output = format_critical_risks(&insights.top_risks);
        if !critical_risks_output.is_empty() {
            print!("{critical_risks_output}");
        }

        // Print recommendations
        let recommendations_output = format_recommendations(&insights.risk_reduction_opportunities);
        if !recommendations_output.is_empty() {
            print!("{recommendations_output}");
        }

        // Print actionable insights
        let insights_output = format_actionable_insights(insights);
        if !insights_output.is_empty() {
            print!("{insights_output}");
        }

        Ok(())
    }
}

fn print_header() {
    println!();
    let divider = "═══════════════════════════════════════════".blue();
    let title = "           DEBTMAP ANALYSIS REPORT".bold().blue();
    println!("{divider}");
    println!("{title}");
    println!("{divider}");
    println!();
}

fn format_debt_score(score: u32, threshold: u32) -> String {
    let colored_score = match score {
        s if s > threshold => s.to_string().red(),
        s if s > threshold / 2 => s.to_string().yellow(),
        s => s.to_string().green(),
    };
    format!("{colored_score} (threshold: {threshold})")
}

fn print_summary(results: &AnalysisResults) {
    let debt_score = total_debt_score(&results.technical_debt.items);
    let debt_threshold = 100;

    // Count unique files from function metrics
    let unique_files: std::collections::HashSet<_> =
        results.complexity.metrics.iter().map(|m| &m.file).collect();
    let file_count = unique_files.len();

    println!("📊 {} Summary", "CODEBASE".bold());
    println!("───────────────────────────────────────────");
    println!("  Files analyzed:      {file_count}");
    println!(
        "  Total functions:     {}",
        results.complexity.summary.total_functions
    );
    println!(
        "  Average complexity:  {:.1}",
        results.complexity.summary.average_complexity
    );
    println!(
        "  Debt items:          {}",
        results.technical_debt.items.len()
    );
    println!(
        "  Total debt score:    {}",
        format_debt_score(debt_score, debt_threshold)
    );
    println!();
}

/// Classify complexity level based on cyclomatic complexity value
fn classify_complexity_level(cyclomatic: u32) -> ComplexityLevel {
    match cyclomatic {
        0..=5 => ComplexityLevel::Low,
        6..=10 => ComplexityLevel::Moderate,
        11..=15 => ComplexityLevel::High,
        _ => ComplexityLevel::Severe,
    }
}

/// Get refactoring action message for a given complexity level
fn get_refactoring_action_message(level: &ComplexityLevel) -> Option<&'static str> {
    match level {
        ComplexityLevel::Low => None,
        ComplexityLevel::Moderate => {
            Some("     ACTION: Extract 2-3 pure functions using direct functional transformation")
        }
        ComplexityLevel::High => {
            Some("     ACTION: Extract 3-5 pure functions using decompose-then-transform strategy")
        }
        ComplexityLevel::Severe => {
            Some("     ACTION: Extract 5+ pure functions into modules with functional core/imperative shell")
        }
    }
}

/// Get refactoring patterns for a given complexity level
fn get_refactoring_patterns(level: &ComplexityLevel) -> &'static str {
    match level {
        ComplexityLevel::Low => "",
        ComplexityLevel::Moderate => "Replace loops with map/filter/fold, extract predicates",
        ComplexityLevel::High => "Decompose into logical units, then apply functional patterns",
        ComplexityLevel::Severe => "Architectural refactoring with monadic patterns and pipelines",
    }
}

fn print_complexity_hotspots(results: &AnalysisResults) {
    if results.complexity.metrics.is_empty() {
        return;
    }

    println!("⚠️  {} (Top 5)", "COMPLEXITY HOTSPOTS".bold());
    println!("───────────────────────────────────────────");
    let top_complex = get_top_complex_functions(&results.complexity.metrics, 5);

    let _refactoring_engine = PatternRecognitionEngine::new();

    for (i, func) in top_complex.iter().enumerate() {
        println!(
            "  {}. {}:{} {}() - Cyclomatic: {}, Cognitive: {}",
            i + 1,
            func.file.display(),
            func.line,
            func.name,
            func.cyclomatic,
            func.cognitive
        );

        // Generate refactoring guidance for high complexity functions
        if func.cyclomatic > 5 {
            let complexity_level = classify_complexity_level(func.cyclomatic);

            if let Some(action_msg) = get_refactoring_action_message(&complexity_level) {
                println!("{}", action_msg.yellow());

                // Add patterns to apply
                let patterns = get_refactoring_patterns(&complexity_level);
                if !patterns.is_empty() {
                    println!("     PATTERNS: {}", patterns.cyan());
                }

                println!("     BENEFIT: Pure functions are easily testable and composable");
            }
        }
    }
    println!();
}

fn get_top_complex_functions(metrics: &[FunctionMetrics], count: usize) -> Vec<&FunctionMetrics> {
    let mut sorted = metrics.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|m| std::cmp::Reverse(m.cyclomatic.max(m.cognitive)));
    sorted.into_iter().take(count).collect()
}

fn print_technical_debt(results: &AnalysisResults) {
    if results.technical_debt.items.is_empty() {
        return;
    }

    println!(
        "🔧 {} ({} items)",
        "TECHNICAL DEBT".bold(),
        results.technical_debt.items.len()
    );
    println!("───────────────────────────────────────────");

    let high_priority: Vec<_> = results
        .technical_debt
        .items
        .iter()
        .filter(|item| matches!(item.priority, Priority::High | Priority::Critical))
        .collect();

    if !high_priority.is_empty() {
        println!("  {} ({}):", "High Priority".red(), high_priority.len());
        for item in high_priority.iter().take(5) {
            println!(
                "    - {}:{} - {}",
                item.file.display(),
                item.line,
                item.message
            );
        }
    }
    println!();
}

fn print_pass_fail_status(results: &AnalysisResults) {
    let passing = is_passing(results);
    let status = if passing {
        "✓ Pass/Fail: PASS".green()
    } else {
        "✗ Pass/Fail: FAIL (some metrics exceed thresholds)".red()
    };
    println!("{status}");
}

fn is_passing(results: &AnalysisResults) -> bool {
    let debt_score = total_debt_score(&results.technical_debt.items);
    let debt_threshold = 100;

    results.complexity.summary.average_complexity <= 10.0
        && results.complexity.summary.high_complexity_count <= 5
        && debt_score <= debt_threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::{
        Difficulty, FunctionRisk, RiskCategory, RiskDistribution, TestEffort, TestingRecommendation,
    };
    use im::Vector;
    use std::path::PathBuf;

    fn create_test_risk_insight() -> RiskInsight {
        RiskInsight {
            top_risks: Vector::from(vec![FunctionRisk {
                function_name: "high_risk_func".to_string(),
                file: PathBuf::from("src/main.rs"),
                line_range: (42, 50),
                risk_score: 85.0,
                cyclomatic_complexity: 15,
                cognitive_complexity: 20,
                coverage_percentage: Some(0.0),
                test_effort: TestEffort {
                    estimated_difficulty: Difficulty::Complex,
                    cognitive_load: 20,
                    branch_count: 10,
                    recommended_test_cases: 5,
                },
                risk_category: RiskCategory::Critical,
                is_test_function: false,
            }]),
            risk_reduction_opportunities: Vector::from(vec![TestingRecommendation {
                function: "test_me".to_string(),
                file: PathBuf::from("src/lib.rs"),
                line: 100,
                current_risk: 75.0,
                potential_risk_reduction: 40.0,
                test_effort_estimate: TestEffort {
                    estimated_difficulty: Difficulty::Moderate,
                    cognitive_load: 8,
                    branch_count: 5,
                    recommended_test_cases: 3,
                },
                rationale: "High risk function with low coverage".to_string(),
                roi: Some(5.0),
                dependencies: vec![],
                dependents: vec![],
            }]),
            codebase_risk_score: 45.5,
            complexity_coverage_correlation: Some(-0.65),
            risk_distribution: RiskDistribution {
                critical_count: 2,
                high_count: 5,
                medium_count: 10,
                low_count: 15,
                well_tested_count: 20,
                total_functions: 52,
            },
        }
    }

    #[test]
    fn test_write_risk_insights_complete_output() {
        let mut writer = TerminalWriter::new();
        let insights = create_test_risk_insight();

        // Test that the method completes without error
        let result = writer.write_risk_insights(&insights);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_risk_insights_low_risk() {
        let mut writer = TerminalWriter::new();
        let mut insights = create_test_risk_insight();
        insights.codebase_risk_score = 25.0;

        let result = writer.write_risk_insights(&insights);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_risk_insights_high_risk() {
        let mut writer = TerminalWriter::new();
        let mut insights = create_test_risk_insight();
        insights.codebase_risk_score = 75.0;

        let result = writer.write_risk_insights(&insights);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_risk_insights_no_correlation() {
        let mut writer = TerminalWriter::new();
        let mut insights = create_test_risk_insight();
        insights.complexity_coverage_correlation = None;

        let result = writer.write_risk_insights(&insights);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_risk_insights_empty_recommendations() {
        let mut writer = TerminalWriter::new();
        let mut insights = create_test_risk_insight();
        insights.risk_reduction_opportunities = Vector::new();
        insights.top_risks = Vector::new();

        let result = writer.write_risk_insights(&insights);
        assert!(result.is_ok());
    }
}
