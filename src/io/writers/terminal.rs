use crate::core::{AnalysisResults, FunctionMetrics, Priority};
use crate::debt::total_debt_score;
use crate::formatting::{ColoredFormatter, FormattingConfig};
use crate::io::output::OutputWriter;
use crate::refactoring::{ComplexityLevel, PatternRecognitionEngine};
use crate::risk::{RiskDistribution, RiskInsight};
use colored::*;

pub struct TerminalWriter {
    #[allow(dead_code)]
    formatter: ColoredFormatter,
}

impl Default for TerminalWriter {
    fn default() -> Self {
        Self::new(FormattingConfig::default())
    }
}

impl TerminalWriter {
    pub fn new(config: FormattingConfig) -> Self {
        Self {
            formatter: ColoredFormatter::new(config),
        }
    }

    pub fn with_formatting(config: FormattingConfig) -> Self {
        Self::new(config)
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

        print_risk_header();
        print_risk_summary(insights);
        print_risk_distribution(&insights.risk_distribution);

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

fn print_risk_header() {
    println!();
    let divider = "═══════════════════════════════════════════".cyan();
    let title = "           RISK ANALYSIS REPORT".bold().cyan();
    println!("{divider}");
    println!("{title}");
    println!("{divider}");
    println!();
}

fn classify_risk_level(score: f64) -> ColoredString {
    match score {
        s if s < 30.0 => "LOW".green(),
        s if s < 60.0 => "MEDIUM".yellow(),
        _ => "HIGH".red(),
    }
}

fn print_risk_summary(insights: &RiskInsight) {
    println!("📈 {} Summary", "RISK".bold());
    println!("───────────────────────────────────────────");
    println!(
        "Codebase Risk Score: {:.1} ({})",
        insights.codebase_risk_score,
        classify_risk_level(insights.codebase_risk_score)
    );

    if let Some(correlation) = insights.complexity_coverage_correlation {
        println!("Complexity-Coverage Correlation: {correlation:.2}");
    }
    println!();
}

fn print_risk_distribution(distribution: &RiskDistribution) {
    println!("Risk Distribution:");
    println!(
        "  Critical: {} functions",
        distribution.critical_count.to_string().red()
    );
    println!(
        "  High: {} functions",
        distribution.high_count.to_string().yellow()
    );
    println!("  Medium: {} functions", distribution.medium_count);
    println!(
        "  Low: {} functions",
        distribution.low_count.to_string().green()
    );
    println!(
        "  Well Tested: {} functions",
        distribution.well_tested_count.to_string().cyan()
    );
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

/// Format entropy information for display if dampening is applied
fn format_entropy_info(entropy_details: &crate::core::EntropyDetails) -> Option<Vec<String>> {
    if !entropy_details.dampening_applied {
        return None;
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "     {} Entropy: {:.2}, Repetition: {:.0}%, Effective: {:.1}x",
        "↓".green(),
        entropy_details.token_entropy,
        entropy_details.pattern_repetition * 100.0,
        entropy_details.effective_complexity
    ));

    for reason in entropy_details.reasoning.iter().take(1) {
        lines.push(format!("       {}", reason.dimmed()));
    }

    Some(lines)
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

        // Display entropy information if available
        if let Some(entropy_details) = func.get_entropy_details() {
            if let Some(entropy_lines) = format_entropy_info(entropy_details) {
                for line in entropy_lines {
                    println!("{line}");
                }
            }
        }

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
        let mut writer = TerminalWriter::default();
        let insights = create_test_risk_insight();

        // Test that the method completes without error
        let result = writer.write_risk_insights(&insights);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_risk_insights_low_risk() {
        let mut writer = TerminalWriter::default();
        let mut insights = create_test_risk_insight();
        insights.codebase_risk_score = 25.0;

        let result = writer.write_risk_insights(&insights);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_risk_insights_high_risk() {
        let mut writer = TerminalWriter::default();
        let mut insights = create_test_risk_insight();
        insights.codebase_risk_score = 75.0;

        let result = writer.write_risk_insights(&insights);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_risk_insights_no_correlation() {
        let mut writer = TerminalWriter::default();
        let mut insights = create_test_risk_insight();
        insights.complexity_coverage_correlation = None;

        let result = writer.write_risk_insights(&insights);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_risk_insights_empty_recommendations() {
        let mut writer = TerminalWriter::default();
        let mut insights = create_test_risk_insight();
        insights.risk_reduction_opportunities = Vector::new();
        insights.top_risks = Vector::new();

        let result = writer.write_risk_insights(&insights);
        assert!(result.is_ok());
    }

    // Helper functions for creating test data

    fn create_entropy_details_with_dampening() -> crate::core::EntropyDetails {
        crate::core::EntropyDetails {
            token_entropy: 3.5,
            pattern_repetition: 0.65,
            branch_similarity: 0.8,
            effective_complexity: 8.2,
            dampening_applied: true,
            dampening_factor: 0.75,
            reasoning: vec![
                "High pattern repetition detected".to_string(),
                "Similar branch structures found".to_string(),
            ],
        }
    }

    fn create_entropy_details_without_dampening() -> crate::core::EntropyDetails {
        crate::core::EntropyDetails {
            token_entropy: 1.2,
            pattern_repetition: 0.1,
            branch_similarity: 0.2,
            effective_complexity: 5.0,
            dampening_applied: false,
            dampening_factor: 1.0,
            reasoning: vec![],
        }
    }

    fn create_entropy_details_empty_reasoning() -> crate::core::EntropyDetails {
        crate::core::EntropyDetails {
            token_entropy: 2.8,
            pattern_repetition: 0.45,
            branch_similarity: 0.6,
            effective_complexity: 6.5,
            dampening_applied: true,
            dampening_factor: 0.85,
            reasoning: vec![],
        }
    }

    // Phase 1: Tests for existing pure functions

    #[test]
    fn test_classify_complexity_level_boundaries() {
        assert_eq!(classify_complexity_level(0), ComplexityLevel::Low);
        assert_eq!(classify_complexity_level(5), ComplexityLevel::Low);
        assert_eq!(classify_complexity_level(6), ComplexityLevel::Moderate);
        assert_eq!(classify_complexity_level(10), ComplexityLevel::Moderate);
        assert_eq!(classify_complexity_level(11), ComplexityLevel::High);
        assert_eq!(classify_complexity_level(15), ComplexityLevel::High);
        assert_eq!(classify_complexity_level(16), ComplexityLevel::Severe);
        assert_eq!(classify_complexity_level(100), ComplexityLevel::Severe);
    }

    #[test]
    fn test_get_refactoring_action_message() {
        assert!(get_refactoring_action_message(&ComplexityLevel::Low).is_none());
        assert!(get_refactoring_action_message(&ComplexityLevel::Moderate).is_some());
        assert!(get_refactoring_action_message(&ComplexityLevel::High).is_some());
        assert!(get_refactoring_action_message(&ComplexityLevel::Severe).is_some());

        // Verify messages contain expected guidance
        let moderate_msg = get_refactoring_action_message(&ComplexityLevel::Moderate).unwrap();
        assert!(moderate_msg.contains("2-3 pure functions"));

        let high_msg = get_refactoring_action_message(&ComplexityLevel::High).unwrap();
        assert!(high_msg.contains("3-5 pure functions"));

        let severe_msg = get_refactoring_action_message(&ComplexityLevel::Severe).unwrap();
        assert!(severe_msg.contains("5+ pure functions"));
    }

    #[test]
    fn test_get_refactoring_patterns() {
        assert_eq!(get_refactoring_patterns(&ComplexityLevel::Low), "");
        assert!(!get_refactoring_patterns(&ComplexityLevel::Moderate).is_empty());
        assert!(!get_refactoring_patterns(&ComplexityLevel::High).is_empty());
        assert!(!get_refactoring_patterns(&ComplexityLevel::Severe).is_empty());

        // Verify patterns contain expected keywords
        let moderate_patterns = get_refactoring_patterns(&ComplexityLevel::Moderate);
        assert!(moderate_patterns.contains("map/filter/fold"));

        let severe_patterns = get_refactoring_patterns(&ComplexityLevel::Severe);
        assert!(severe_patterns.contains("monadic"));
    }

    // Phase 2: Tests for format_entropy_info function

    #[test]
    fn test_format_entropy_info_dampening_applied() {
        let details = create_entropy_details_with_dampening();
        let result = format_entropy_info(&details);
        assert!(result.is_some());

        let lines = result.unwrap();
        assert_eq!(lines.len(), 2); // Header + one reason

        // Verify header line contains expected values
        assert!(lines[0].contains("Entropy: 3.50"));
        assert!(lines[0].contains("Repetition: 65%"));
        assert!(lines[0].contains("Effective: 8.2x"));

        // Verify reasoning line
        assert!(lines[1].contains("High pattern repetition detected"));
    }

    #[test]
    fn test_format_entropy_info_no_dampening() {
        let details = create_entropy_details_without_dampening();
        let result = format_entropy_info(&details);
        assert!(result.is_none());
    }

    #[test]
    fn test_format_entropy_info_no_reasoning() {
        let details = create_entropy_details_empty_reasoning();
        let result = format_entropy_info(&details);
        assert!(result.is_some());

        let lines = result.unwrap();
        assert_eq!(lines.len(), 1); // Only header, no reasoning

        // Verify header contains expected values
        assert!(lines[0].contains("Entropy: 2.80"));
        assert!(lines[0].contains("Repetition: 45%"));
        assert!(lines[0].contains("Effective: 6.5x"));
    }

    #[test]
    fn test_format_entropy_info_limits_reasoning_to_one() {
        let mut details = create_entropy_details_with_dampening();
        details.reasoning = vec![
            "Reason 1".to_string(),
            "Reason 2".to_string(),
            "Reason 3".to_string(),
        ];

        let result = format_entropy_info(&details);
        assert!(result.is_some());

        let lines = result.unwrap();
        assert_eq!(lines.len(), 2); // Header + only first reason
        assert!(lines[1].contains("Reason 1"));
        assert!(!lines.iter().any(|l| l.contains("Reason 2")));
    }
}
