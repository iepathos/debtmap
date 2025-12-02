//! Core markdown writer implementation
//!
//! Contains the main MarkdownWriter struct and OutputWriter trait implementation,
//! along with basic formatting and output methods.

use crate::core::{AnalysisResults, DebtItem, FunctionMetrics, Priority};
use crate::debt::total_debt_score;
use crate::io::output::{
    build_summary_rows, complexity_header_lines, get_recommendation, get_top_complex_functions,
    OutputWriter,
};
use crate::io::writers::pattern_display::{format_pattern_confidence, format_pattern_type};
use crate::risk::{RiskDistribution, RiskInsight};
use std::io::Write;

pub struct MarkdownWriter<W: Write> {
    writer: W,
    verbosity: u8,
}

impl<W: Write> MarkdownWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            verbosity: 0,
        }
    }

    pub fn with_verbosity(writer: W, verbosity: u8) -> Self {
        Self { writer, verbosity }
    }

    // Getter methods for use in enhanced module
    pub(super) fn writer(&mut self) -> &mut W {
        &mut self.writer
    }

    pub(super) fn verbosity(&self) -> u8 {
        self.verbosity
    }
}

impl<W: Write> OutputWriter for MarkdownWriter<W> {
    fn write_results(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        let writers: Vec<fn(&mut Self, &AnalysisResults) -> anyhow::Result<()>> = vec![
            |w, r| w.write_header(r),
            |w, r| w.write_summary(r),
            |w, r| w.write_complexity_analysis(r),
            |w, r| w.write_technical_debt(r),
            |w, _| w.write_recommendations(),
        ];

        writers.iter().try_for_each(|writer| writer(self, results))
    }

    fn write_risk_insights(&mut self, insights: &RiskInsight) -> anyhow::Result<()> {
        self.write_risk_header()?;
        self.write_risk_summary(insights)?;
        self.write_risk_distribution(&insights.risk_distribution)?;
        Ok(())
    }
}

impl<W: Write> MarkdownWriter<W> {
    fn write_header(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        let header_lines = [
            "# Debtmap Analysis Report".to_string(),
            String::new(),
            format!(
                "Generated: {}",
                results.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
            ),
            "Version: 0.1.0".to_string(),
            String::new(),
        ];

        header_lines
            .iter()
            .try_for_each(|line| writeln!(self.writer, "{line}"))?;
        Ok(())
    }

    fn write_summary(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        self.write_summary_header()?;
        self.write_summary_metrics(results)?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_summary_header(&mut self) -> anyhow::Result<()> {
        writeln!(self.writer, "## Executive Summary")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "| Metric | Value | Status |")?;
        writeln!(self.writer, "|--------|-------|--------|")?;
        Ok(())
    }

    fn write_summary_metrics(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        let debt_score = total_debt_score(&results.technical_debt.items);
        let debt_threshold = 100;

        build_summary_rows(results, debt_score, debt_threshold)
            .into_iter()
            .try_for_each(|(metric, value, status)| self.write_summary_row(metric, &value, &status))
    }

    fn write_summary_row(&mut self, metric: &str, value: &str, status: &str) -> anyhow::Result<()> {
        writeln!(self.writer, "| {metric} | {value} | {status} |")?;
        Ok(())
    }

    fn write_complexity_analysis(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        if results.complexity.metrics.is_empty() {
            return Ok(());
        }

        self.write_complexity_header()?;
        self.write_complexity_table(results)?;
        Ok(())
    }

    fn write_complexity_header(&mut self) -> anyhow::Result<()> {
        complexity_header_lines()
            .iter()
            .try_for_each(|line| writeln!(self.writer, "{line}"))?;
        Ok(())
    }

    fn write_complexity_table(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        let top_complex = get_top_complex_functions(&results.complexity.metrics, 5);

        for func in top_complex {
            self.write_complexity_row(func)?;
        }
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_complexity_row(&mut self, func: &FunctionMetrics) -> anyhow::Result<()> {
        writeln!(
            self.writer,
            "| {}:{} | {} | {} | {} | {} | {} | {} |",
            func.file.display(),
            func.line,
            func.name,
            func.cyclomatic,
            func.cognitive,
            format_pattern_type(func),
            format_pattern_confidence(func),
            get_recommendation(func)
        )?;
        Ok(())
    }

    fn write_technical_debt(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        if results.technical_debt.items.is_empty() {
            return Ok(());
        }

        self.write_technical_debt_header()?;
        self.write_high_priority_items(&results.technical_debt.items)?;
        Ok(())
    }

    fn write_technical_debt_header(&mut self) -> anyhow::Result<()> {
        writeln!(self.writer, "## Technical Debt")?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_risk_header(&mut self) -> anyhow::Result<()> {
        writeln!(self.writer, "## Risk Analysis")?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_risk_summary(&mut self, insights: &RiskInsight) -> anyhow::Result<()> {
        writeln!(self.writer, "### Risk Summary")?;
        writeln!(
            self.writer,
            "- Codebase Risk Score: {:.1}",
            insights.codebase_risk_score
        )?;

        if let Some(correlation) = insights.complexity_coverage_correlation {
            writeln!(
                self.writer,
                "- Complexity-Coverage Correlation: {correlation:.2}"
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_risk_distribution(&mut self, distribution: &RiskDistribution) -> anyhow::Result<()> {
        writeln!(self.writer, "### Risk Distribution")?;

        let distribution_items = [
            ("Critical", distribution.critical_count),
            ("High", distribution.high_count),
            ("Medium", distribution.medium_count),
            ("Low", distribution.low_count),
            ("Well Tested", distribution.well_tested_count),
        ];

        distribution_items
            .iter()
            .try_for_each(|(label, count)| writeln!(self.writer, "- {label}: {count}"))?;

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_high_priority_items(&mut self, items: &[DebtItem]) -> anyhow::Result<()> {
        let high_priority: Vec<_> = items
            .iter()
            .filter(|item| self.is_high_priority(item))
            .collect();

        if high_priority.is_empty() {
            return Ok(());
        }

        writeln!(
            self.writer,
            "### High Priority ({} items)",
            high_priority.len()
        )?;

        high_priority
            .iter()
            .take(10)
            .try_for_each(|item| self.write_debt_item(item))?;

        writeln!(self.writer)?;
        Ok(())
    }

    fn is_high_priority(&self, item: &DebtItem) -> bool {
        matches!(item.priority, Priority::High | Priority::Critical)
    }

    fn write_debt_item(&mut self, item: &DebtItem) -> anyhow::Result<()> {
        writeln!(
            self.writer,
            "- [ ] `{}:{}` - {}",
            item.file.display(),
            item.line,
            item.message
        )?;
        Ok(())
    }

    fn write_recommendations(&mut self) -> anyhow::Result<()> {
        writeln!(self.writer, "## Recommendations")?;
        writeln!(self.writer)?;

        let recommendations = [
            "1. **Immediate Action**: Address high-priority debt items and refactor top complexity hotspots",
            "2. **Short Term**: Reduce code duplication by extracting common functionality",
            "3. **Long Term**: Establish complexity budget and monitor trends over time",
        ];

        recommendations
            .iter()
            .try_for_each(|rec| writeln!(self.writer, "{rec}"))?;

        Ok(())
    }
}
