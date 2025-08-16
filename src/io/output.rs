use crate::core::{AnalysisResults, FunctionMetrics};
use crate::io::writers::{JsonWriter, MarkdownWriter, TerminalWriter};
use crate::risk::RiskInsight;
use std::io;

#[derive(Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Markdown,
    Terminal,
}

pub trait OutputWriter {
    fn write_results(&mut self, results: &AnalysisResults) -> anyhow::Result<()>;
    fn write_risk_insights(&mut self, insights: &RiskInsight) -> anyhow::Result<()>;
}

pub fn create_writer(format: OutputFormat) -> Box<dyn OutputWriter> {
    match format {
        OutputFormat::Json => Box::new(JsonWriter::new(io::stdout())),
        OutputFormat::Markdown => Box::new(MarkdownWriter::new(io::stdout())),
        OutputFormat::Terminal => Box::new(TerminalWriter::new()),
    }
}

// Helper functions shared by multiple writers
pub fn complexity_status(avg: f64) -> &'static str {
    match avg {
        a if a > 15.0 => "❌ High",
        a if a > 10.0 => "⚠️ Medium",
        a if a > 5.0 => "🔶 Moderate",
        _ => "✅ Low",
    }
}

pub fn debt_status(count: usize) -> &'static str {
    match count {
        c if c > 50 => "❌ High",
        c if c > 20 => "⚠️ Medium",
        c if c > 10 => "🔶 Moderate",
        _ => "✅ Low",
    }
}

pub fn high_complexity_status(count: usize) -> &'static str {
    match count {
        0 => "✅ Good",
        1..=5 => "🔶 Fair",
        _ => "❌ Poor",
    }
}

pub fn debt_score_status(score: u32, threshold: u32) -> &'static str {
    match score {
        s if s > threshold => "❌ High",
        s if s > threshold / 2 => "⚠️ Medium",
        _ => "✅ Good",
    }
}

pub fn complexity_header_lines() -> Vec<&'static str> {
    vec![
        "## Complexity Analysis",
        "",
        "| Location | Function | Cyclomatic | Cognitive | Recommendation |",
        "|----------|----------|------------|-----------|----------------|",
    ]
}

pub fn build_summary_rows(
    results: &AnalysisResults,
    debt_score: u32,
    debt_threshold: u32,
) -> Vec<(&'static str, String, String)> {
    vec![
        (
            "Files Analyzed",
            results.complexity.metrics.len().to_string(),
            "-".to_string(),
        ),
        (
            "Total Functions",
            results.complexity.summary.total_functions.to_string(),
            "-".to_string(),
        ),
        (
            "Average Complexity",
            format!("{:.1}", results.complexity.summary.average_complexity),
            complexity_status(results.complexity.summary.average_complexity).to_string(),
        ),
        (
            "High Complexity Functions",
            results.complexity.summary.high_complexity_count.to_string(),
            high_complexity_status(results.complexity.summary.high_complexity_count).to_string(),
        ),
        (
            "Technical Debt Items",
            results.technical_debt.items.len().to_string(),
            debt_status(results.technical_debt.items.len()).to_string(),
        ),
        (
            "Total Debt Score",
            format!("{debt_score} / {debt_threshold}"),
            debt_score_status(debt_score, debt_threshold).to_string(),
        ),
    ]
}

pub fn get_top_complex_functions(
    metrics: &[FunctionMetrics],
    count: usize,
) -> Vec<&FunctionMetrics> {
    let mut sorted = metrics.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|m| std::cmp::Reverse(m.cyclomatic.max(m.cognitive)));
    sorted.into_iter().take(count).collect()
}

pub fn get_recommendation(func: &FunctionMetrics) -> &'static str {
    match func.cyclomatic.max(func.cognitive) {
        c if c > 20 => "Urgent refactoring needed",
        c if c > 15 => "Refactor recommended",
        c if c > 10 => "Consider simplifying",
        _ => "Acceptable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        AnalysisResults, ComplexityReport, ComplexitySummary, DebtItem, DebtType, DependencyReport,
        FunctionMetrics, Priority, TechnicalDebtReport,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_test_results() -> AnalysisResults {
        let items = vec![DebtItem {
            id: "test-1".to_string(),
            debt_type: DebtType::Todo,
            priority: Priority::Medium,
            file: PathBuf::from("test.rs"),
            line: 5,
            column: None,
            message: "TODO: Implement feature".to_string(),
            context: None,
        }];

        let metrics = vec![FunctionMetrics {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 7,
            nesting: 2,
            length: 25,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
        }];

        AnalysisResults {
            project_path: PathBuf::from("/test/project"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: metrics.clone(),
                summary: ComplexitySummary {
                    total_functions: 1,
                    average_complexity: 5.0,
                    max_complexity: 5,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items,
                by_type: HashMap::new(),
                priorities: vec![Priority::Medium],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        }
    }

    #[test]
    fn test_output_json_format() {
        let results = create_test_results();
        let mut buffer = Vec::new();
        let mut writer = crate::io::writers::JsonWriter::new(&mut buffer);
        writer.write_results(&results).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("test_func"));
        assert!(output.contains("TODO: Implement feature"));
    }

    #[test]
    fn test_output_markdown_format() {
        let results = create_test_results();
        let mut buffer = Vec::new();
        let mut writer = crate::io::writers::MarkdownWriter::new(&mut buffer);
        writer.write_results(&results).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("# Debtmap Analysis Report"));
        assert!(output.contains("Executive Summary"));
    }
}
