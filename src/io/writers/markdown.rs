use crate::core::{AnalysisResults, DebtItem, FunctionMetrics, Priority};
use crate::debt::total_debt_score;
use crate::io::output::{
    build_summary_rows, complexity_header_lines, get_recommendation, get_top_complex_functions,
    OutputWriter,
};
use crate::priority::{UnifiedAnalysis, UnifiedDebtItem};
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

// Additional trait for enhanced markdown output
pub trait EnhancedMarkdownWriter {
    fn write_unified_analysis(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()>;
    fn write_priority_section(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()>;
    fn write_dead_code_section(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()>;
    fn write_call_graph_insights(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()>;
    fn write_testing_recommendations(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()>;
}

impl<W: Write> EnhancedMarkdownWriter for MarkdownWriter<W> {
    fn write_unified_analysis(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()> {
        self.write_priority_section(analysis)?;
        self.write_dead_code_section(analysis)?;
        self.write_call_graph_insights(analysis)?;
        self.write_testing_recommendations(analysis)?;
        Ok(())
    }

    fn write_priority_section(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()> {
        writeln!(self.writer, "## Priority Technical Debt")?;
        writeln!(self.writer)?;

        // Get top 10 priorities
        let top_items = analysis.get_top_priorities(10);

        if top_items.is_empty() {
            writeln!(self.writer, "_No priority items found._")?;
            writeln!(self.writer)?;
            return Ok(());
        }

        writeln!(self.writer, "### Top {} Priority Items", top_items.len())?;
        writeln!(self.writer)?;
        writeln!(self.writer, "| Rank | Score | Function | Type | Issue |")?;
        writeln!(self.writer, "|------|-------|----------|------|-------|")?;

        for (idx, item) in top_items.iter().enumerate() {
            let rank = idx + 1;
            let score = format!("{:.1}", item.unified_score.final_score);
            let location = format!("{}:{}", item.location.file.display(), item.location.line);
            let debt_type = format_debt_type(&item.debt_type);
            let issue = format_debt_issue(&item.debt_type);

            writeln!(
                self.writer,
                "| {} | {} | `{}` | {} | {} |",
                rank, score, location, debt_type, issue
            )?;
        }
        writeln!(self.writer)?;

        // Add score breakdown if verbosity is enabled
        if self.verbosity > 0 {
            let items_vec: Vec<UnifiedDebtItem> = top_items.iter().cloned().collect();
            self.write_score_breakdown(&items_vec)?;
        }

        Ok(())
    }

    fn write_dead_code_section(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()> {
        use crate::priority::DebtType;

        let dead_code_items: Vec<&UnifiedDebtItem> = analysis
            .items
            .iter()
            .filter(|item| matches!(item.debt_type, DebtType::DeadCode { .. }))
            .collect();

        if dead_code_items.is_empty() {
            return Ok(());
        }

        writeln!(self.writer, "## Dead Code Detection")?;
        writeln!(self.writer)?;
        writeln!(
            self.writer,
            "### Unused Functions ({} found)",
            dead_code_items.len()
        )?;
        writeln!(self.writer)?;
        writeln!(
            self.writer,
            "| Function | Visibility | Complexity | Recommendation |"
        )?;
        writeln!(
            self.writer,
            "|----------|------------|------------|----------------|"
        )?;

        for item in dead_code_items.iter().take(20) {
            if let DebtType::DeadCode {
                visibility,
                cyclomatic,
                ..
            } = &item.debt_type
            {
                let vis_str = format_visibility(visibility);
                let recommendation = get_dead_code_recommendation(visibility, *cyclomatic);

                writeln!(
                    self.writer,
                    "| `{}` | {} | {} | {} |",
                    item.location.function, vis_str, cyclomatic, recommendation
                )?;
            }
        }
        writeln!(self.writer)?;

        Ok(())
    }

    fn write_call_graph_insights(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()> {
        if self.verbosity < 2 {
            return Ok(());
        }

        writeln!(self.writer, "## Call Graph Analysis")?;
        writeln!(self.writer)?;

        // Show module dependency statistics
        // The call graph structure uses different field names
        let total_functions = analysis.call_graph.node_count();
        let total_relationships = analysis
            .items
            .iter()
            .map(|item| item.downstream_dependencies)
            .sum::<usize>();

        writeln!(self.writer, "### Module Statistics")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "- Total Functions: {}", total_functions)?;
        writeln!(
            self.writer,
            "- Total Call Relationships: {}",
            total_relationships
        )?;
        writeln!(self.writer)?;

        Ok(())
    }

    fn write_testing_recommendations(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()> {
        writeln!(self.writer, "## Testing Recommendations")?;
        writeln!(self.writer)?;

        // Get top untested functions with high ROI
        let testing_gaps: Vec<&UnifiedDebtItem> = analysis
            .items
            .iter()
            .filter(|item| matches!(item.debt_type, crate::priority::DebtType::TestingGap { .. }))
            .take(10)
            .collect();

        if testing_gaps.is_empty() {
            writeln!(
                self.writer,
                "_All critical functions have adequate test coverage._"
            )?;
            writeln!(self.writer)?;
            return Ok(());
        }

        writeln!(self.writer, "### ROI-Based Testing Priorities")?;
        writeln!(self.writer)?;
        writeln!(
            self.writer,
            "| Function | ROI | Complexity | Coverage | Risk Reduction |"
        )?;
        writeln!(
            self.writer,
            "|----------|-----|------------|----------|----------------|"
        )?;

        for item in testing_gaps {
            if let crate::priority::DebtType::TestingGap {
                coverage,
                cyclomatic,
                cognitive: _,
            } = &item.debt_type
            {
                let roi = calculate_roi(item);
                let risk_reduction = estimate_risk_reduction(*coverage);

                writeln!(
                    self.writer,
                    "| `{}` | {:.1} | {} | {:.0}% | {:.0}% |",
                    item.location.function,
                    roi,
                    cyclomatic,
                    coverage * 100.0,
                    risk_reduction * 100.0
                )?;
            }
        }
        writeln!(self.writer)?;

        Ok(())
    }
}

impl<W: Write> MarkdownWriter<W> {
    fn write_score_breakdown(&mut self, items: &[UnifiedDebtItem]) -> anyhow::Result<()> {
        writeln!(self.writer, "<details>")?;
        writeln!(
            self.writer,
            "<summary>Score Breakdown (click to expand)</summary>"
        )?;
        writeln!(self.writer)?;

        for (idx, item) in items.iter().enumerate().take(3) {
            writeln!(self.writer, "#### {}. {}", idx + 1, item.location.function)?;
            writeln!(self.writer)?;
            writeln!(
                self.writer,
                "- **Priority Score**: {:.2}",
                item.unified_score.final_score
            )?;
            writeln!(
                self.writer,
                "- **Complexity Factor**: {:.2}",
                item.unified_score.complexity_factor
            )?;
            writeln!(
                self.writer,
                "- **Coverage Factor**: {:.2}",
                item.unified_score.coverage_factor
            )?;
            writeln!(
                self.writer,
                "- **ROI Factor**: {:.2}",
                item.unified_score.roi_factor
            )?;
            writeln!(
                self.writer,
                "- **Semantic Factor**: {:.2}",
                item.unified_score.semantic_factor
            )?;
            writeln!(self.writer)?;
        }

        writeln!(self.writer, "</details>")?;
        writeln!(self.writer)?;
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
            "| {}:{} | {} | {} | {} | {} |",
            func.file.display(),
            func.line,
            func.name,
            func.cyclomatic,
            func.cognitive,
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

// Helper functions for formatting
fn format_debt_type(debt_type: &crate::priority::DebtType) -> &'static str {
    use crate::priority::DebtType;
    match debt_type {
        DebtType::TestingGap { .. } => "Testing Gap",
        DebtType::ComplexityHotspot { .. } => "Complexity",
        DebtType::DeadCode { .. } => "Dead Code",
        DebtType::Orchestration { .. } => "Orchestration",
        DebtType::Duplication { .. } => "Duplication",
        DebtType::Risk { .. } => "Risk",
        DebtType::TestComplexityHotspot { .. } => "Test Complexity",
        DebtType::TestTodo { .. } => "Test TODO",
        DebtType::TestDuplication { .. } => "Test Duplication",
    }
}

fn format_debt_issue(debt_type: &crate::priority::DebtType) -> String {
    use crate::priority::DebtType;
    match debt_type {
        DebtType::TestingGap {
            coverage,
            cyclomatic,
            ..
        } => {
            format!(
                "{:.0}% coverage, complexity {}",
                coverage * 100.0,
                cyclomatic
            )
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => {
            format!("Cyclomatic: {}, Cognitive: {}", cyclomatic, cognitive)
        }
        DebtType::DeadCode { visibility, .. } => {
            format!("Unused {:?} function", visibility)
        }
        DebtType::Orchestration { delegates_to } => {
            format!("Delegates to {} functions", delegates_to.len())
        }
        DebtType::Duplication {
            instances,
            total_lines,
        } => {
            format!("{} instances, {} lines", instances, total_lines)
        }
        DebtType::Risk { risk_score, .. } => {
            format!("Risk score: {:.1}", risk_score)
        }
        DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive,
            ..
        } => {
            format!("Test complexity: {} / {}", cyclomatic, cognitive)
        }
        DebtType::TestTodo { priority, reason } => {
            let reason_str = reason.as_deref().unwrap_or("No reason provided");
            format!("{:?} priority: {}", priority, reason_str)
        }
        DebtType::TestDuplication {
            instances,
            similarity,
            ..
        } => {
            format!(
                "{} instances, {:.0}% similar",
                instances,
                similarity * 100.0
            )
        }
    }
}

fn format_visibility(visibility: &crate::priority::FunctionVisibility) -> &'static str {
    use crate::priority::FunctionVisibility;
    match visibility {
        FunctionVisibility::Public => "public",
        FunctionVisibility::Private => "private",
        FunctionVisibility::Crate => "crate",
    }
}

fn get_dead_code_recommendation(
    visibility: &crate::priority::FunctionVisibility,
    complexity: u32,
) -> &'static str {
    use crate::priority::FunctionVisibility;
    match (visibility, complexity) {
        (FunctionVisibility::Private, c) if c < 5 => "Safe to remove",
        (FunctionVisibility::Private, _) => "Review and remove if unused",
        (FunctionVisibility::Crate, _) => "Check module usage",
        (FunctionVisibility::Public, _) => "Check external usage",
    }
}

fn calculate_roi(item: &crate::priority::UnifiedDebtItem) -> f64 {
    // Simple ROI calculation based on score components
    item.unified_score.roi_factor * 10.0
}

fn estimate_risk_reduction(coverage: f64) -> f64 {
    // Estimate risk reduction from improving coverage
    (1.0 - coverage) * 0.3
}
