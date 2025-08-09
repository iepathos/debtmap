use crate::core::{AnalysisResults, FunctionMetrics, Priority};
use crate::debt::total_debt_score;
use colored::*;
use serde_json;
use std::io::Write;

#[derive(Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Markdown,
    Terminal,
}

pub trait OutputWriter {
    fn write_results(&mut self, results: &AnalysisResults) -> anyhow::Result<()>;
}

pub struct JsonWriter<W: Write> {
    writer: W,
}

impl<W: Write> JsonWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: Write> OutputWriter for JsonWriter<W> {
    fn write_results(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(results)?;
        self.writer.write_all(json.as_bytes())?;
        Ok(())
    }
}

pub struct MarkdownWriter<W: Write> {
    writer: W,
}

impl<W: Write> MarkdownWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W: Write> OutputWriter for MarkdownWriter<W> {
    fn write_results(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        self.write_header(results)?;
        self.write_summary(results)?;
        self.write_complexity_analysis(results)?;
        self.write_technical_debt(results)?;
        self.write_recommendations()?;
        Ok(())
    }
}

impl<W: Write> MarkdownWriter<W> {
    fn write_header(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        writeln!(self.writer, "# Debtmap Analysis Report")?;
        writeln!(self.writer)?;
        writeln!(
            self.writer,
            "Generated: {}",
            results.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )?;
        writeln!(self.writer, "Version: 0.1.0")?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_summary(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        let debt_score = total_debt_score(&results.technical_debt.items);
        let debt_threshold = 100; // Default threshold, can be made configurable

        writeln!(self.writer, "## Executive Summary")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "| Metric | Value | Status |")?;
        writeln!(self.writer, "|--------|-------|--------|")?;

        self.write_summary_row(
            "Files Analyzed",
            &results.complexity.metrics.len().to_string(),
            "-",
        )?;
        self.write_summary_row(
            "Total Functions",
            &results.complexity.summary.total_functions.to_string(),
            "-",
        )?;
        self.write_summary_row(
            "Average Complexity",
            &format!("{:.1}", results.complexity.summary.average_complexity),
            complexity_status(results.complexity.summary.average_complexity),
        )?;
        self.write_summary_row(
            "High Complexity Functions",
            &results.complexity.summary.high_complexity_count.to_string(),
            if results.complexity.summary.high_complexity_count > 10 {
                "⚠️ Warning"
            } else {
                "✅ Good"
            },
        )?;
        self.write_summary_row(
            "Technical Debt Items",
            &results.technical_debt.items.len().to_string(),
            debt_status(results.technical_debt.items.len()),
        )?;
        self.write_summary_row(
            "Total Debt Score",
            &format!("{debt_score} / {debt_threshold}"),
            if debt_score > debt_threshold {
                "❌ Exceeds threshold"
            } else if debt_score > debt_threshold / 2 {
                "⚠️ Medium"
            } else {
                "✅ Good"
            },
        )?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_summary_row(&mut self, metric: &str, value: &str, status: &str) -> anyhow::Result<()> {
        writeln!(self.writer, "| {metric} | {value} | {status} |")?;
        Ok(())
    }

    fn write_complexity_analysis(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        if results.complexity.metrics.is_empty() {
            return Ok(());
        }

        writeln!(self.writer, "## Complexity Analysis")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "### Hotspots Requiring Attention")?;
        writeln!(self.writer)?;
        writeln!(
            self.writer,
            "| File:Line | Function | Cyclomatic | Cognitive | Recommendation |"
        )?;
        writeln!(
            self.writer,
            "|-----------|----------|------------|-----------|----------------|"
        )?;

        let mut top_complex: Vec<_> = results.complexity.metrics.iter().collect();
        top_complex.sort_by(|a, b| b.cyclomatic.cmp(&a.cyclomatic));

        for func in top_complex.iter().take(5) {
            writeln!(
                self.writer,
                "| {}:{} | {} | {} | {} | {} |",
                func.file.display(),
                func.line,
                func.name,
                func.cyclomatic,
                func.cognitive,
                get_recommendation(func)
            )?;
        }
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_technical_debt(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        if results.technical_debt.items.is_empty() {
            return Ok(());
        }

        writeln!(self.writer, "## Technical Debt")?;
        writeln!(self.writer)?;

        let high_priority: Vec<_> = results
            .technical_debt
            .items
            .iter()
            .filter(|item| matches!(item.priority, Priority::High | Priority::Critical))
            .collect();

        if !high_priority.is_empty() {
            writeln!(
                self.writer,
                "### High Priority ({} items)",
                high_priority.len()
            )?;
            for item in high_priority.iter().take(10) {
                writeln!(
                    self.writer,
                    "- [ ] `{}:{}` - {}",
                    item.file.display(),
                    item.line,
                    item.message
                )?;
            }
            writeln!(self.writer)?;
        }
        Ok(())
    }

    fn write_recommendations(&mut self) -> anyhow::Result<()> {
        writeln!(self.writer, "## Recommendations")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "1. **Immediate Action**: Address high-priority debt items and refactor top complexity hotspots")?;
        writeln!(
            self.writer,
            "2. **Short Term**: Reduce code duplication by extracting common functionality"
        )?;
        writeln!(
            self.writer,
            "3. **Long Term**: Establish complexity budget and monitor trends over time"
        )?;
        Ok(())
    }
}

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
        print_header();
        print_summary(results);
        print_complexity_hotspots(results);
        print_technical_debt(results);
        print_pass_fail_status(results);
        Ok(())
    }
}

fn print_header() {
    println!("{}", "Debtmap Analysis Report".bold().blue());
    println!("{}", "=======================".blue());
    println!();
}

fn print_summary(results: &AnalysisResults) {
    let debt_score = total_debt_score(&results.technical_debt.items);
    let debt_threshold = 100; // Default threshold, can be made configurable

    println!("{} Summary:", "📊".bold());
    println!("  Files analyzed: {}", results.complexity.metrics.len());
    println!(
        "  Total functions: {}",
        results.complexity.summary.total_functions
    );
    println!(
        "  Average complexity: {:.1}",
        results.complexity.summary.average_complexity
    );
    println!("  Debt items: {}", results.technical_debt.items.len());

    // Add debt score with color coding
    let score_display = if debt_score > debt_threshold {
        format!(
            "{} (threshold: {})",
            debt_score.to_string().red(),
            debt_threshold
        )
    } else if debt_score > debt_threshold / 2 {
        format!(
            "{} (threshold: {})",
            debt_score.to_string().yellow(),
            debt_threshold
        )
    } else {
        format!(
            "{} (threshold: {})",
            debt_score.to_string().green(),
            debt_threshold
        )
    };
    println!("  Total debt score: {score_display}");

    println!();
}

fn print_complexity_hotspots(results: &AnalysisResults) {
    if results.complexity.summary.high_complexity_count == 0 {
        return;
    }

    println!("{} Complexity Hotspots (top 5):", "⚠️".yellow());

    let mut top_complex: Vec<_> = results.complexity.metrics.iter().collect();
    top_complex.sort_by(|a, b| b.cyclomatic.cmp(&a.cyclomatic));

    top_complex
        .iter()
        .take(5)
        .enumerate()
        .for_each(|(i, func)| {
            println!(
                "  {}. {}:{} {}() - Cyclomatic: {}, Cognitive: {}",
                i + 1,
                func.file.display(),
                func.line,
                func.name.yellow(),
                func.cyclomatic.to_string().red(),
                func.cognitive.to_string().red()
            );
        });
    println!();
}

fn print_technical_debt(results: &AnalysisResults) {
    let high_priority: Vec<_> = results
        .technical_debt
        .items
        .iter()
        .filter(|item| matches!(item.priority, Priority::High | Priority::Critical))
        .collect();

    if high_priority.is_empty() {
        return;
    }

    println!(
        "{} Technical Debt ({} items):",
        "🔧".bold(),
        results.technical_debt.items.len()
    );
    println!("  {} ({}):", "High Priority".red(), high_priority.len());

    high_priority.iter().take(5).for_each(|item| {
        println!(
            "    - {}:{} - {}",
            item.file.display(),
            item.line,
            item.message
        );
    });
    println!();
}

fn print_pass_fail_status(results: &AnalysisResults) {
    let pass = is_passing(results);
    let (symbol, status, message) = if pass {
        (
            "✓".green(),
            "PASS".green().bold(),
            "all metrics within thresholds",
        )
    } else {
        (
            "✗".red(),
            "FAIL".red().bold(),
            "some metrics exceed thresholds",
        )
    };

    println!("{symbol} Pass/Fail: {status} ({message})");
}

fn is_passing(results: &AnalysisResults) -> bool {
    let debt_score = total_debt_score(&results.technical_debt.items);
    let debt_threshold = 100; // Default threshold, can be made configurable

    results.complexity.summary.average_complexity < 10.0
        && results.complexity.summary.high_complexity_count < 10
        && debt_score <= debt_threshold
}

fn complexity_status(avg: f64) -> &'static str {
    match avg {
        x if x < 5.0 => "✅ Good",
        x if x < 10.0 => "⚠️ Medium",
        _ => "❌ High",
    }
}

fn debt_status(count: usize) -> &'static str {
    match count {
        x if x < 20 => "✅ Low",
        x if x < 50 => "⚠️ Medium",
        _ => "❌ High",
    }
}

fn get_recommendation(func: &FunctionMetrics) -> &'static str {
    match func.cyclomatic {
        x if x > 20 => "Refactor: Split into smaller functions",
        x if x > 10 => "Review: Consider extracting complex logic",
        _ => "Monitor: Keep watching for increases",
    }
}

pub fn create_writer(format: OutputFormat) -> Box<dyn OutputWriter> {
    match format {
        OutputFormat::Json => Box::new(JsonWriter::new(std::io::stdout())),
        OutputFormat::Markdown => Box::new(MarkdownWriter::new(std::io::stdout())),
        OutputFormat::Terminal => Box::new(TerminalWriter::new()),
    }
}
