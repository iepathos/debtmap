use crate::core::{AnalysisResults, FunctionMetrics, Priority};
use crate::debt::total_debt_score;
use crate::io::output::OutputWriter;
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
            format_risk_matrix_terminal,
        };

        // Print risk header
        println!();
        println!("{}", "═══════════════════════════════════════════".cyan());
        println!("{}", "           RISK ANALYSIS REPORT".bold().cyan());
        println!("{}", "═══════════════════════════════════════════".cyan());
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

        // Print risk matrix
        if insights.complexity_coverage_correlation.is_some() {
            print!("{}", format_risk_matrix_terminal());
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
    println!("{}", "═══════════════════════════════════════════".blue());
    println!("{}", "           DEBTMAP ANALYSIS REPORT".bold().blue());
    println!("{}", "═══════════════════════════════════════════".blue());
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

fn print_complexity_hotspots(results: &AnalysisResults) {
    if results.complexity.metrics.is_empty() {
        return;
    }

    println!("⚠️  {} (Top 5)", "COMPLEXITY HOTSPOTS".bold());
    println!("───────────────────────────────────────────");
    let top_complex = get_top_complex_functions(&results.complexity.metrics, 5);

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
