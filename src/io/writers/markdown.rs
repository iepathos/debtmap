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
        // Convert im::Vector to slice for compatibility
        let items_vec: Vec<UnifiedDebtItem> = analysis.items.iter().cloned().collect();
        let testing_gaps = collect_testing_gaps(&items_vec);

        // Format and write the recommendations
        let recommendations = format_testing_recommendations(&testing_gaps);
        write!(self.writer, "{}", recommendations)?;

        Ok(())
    }
}

impl<W: Write> MarkdownWriter<W> {
    fn write_score_breakdown(&mut self, items: &[UnifiedDebtItem]) -> anyhow::Result<()> {
        let breakdown = format_score_breakdown(items);
        write!(self.writer, "{}", breakdown)?;
        Ok(())
    }
}

// Pure functions for formatting score breakdown
fn format_score_breakdown(items: &[UnifiedDebtItem]) -> String {
    let mut output = String::new();
    output.push_str("<details>\n");
    output.push_str("<summary>Score Breakdown (click to expand)</summary>\n\n");

    for (idx, item) in items.iter().enumerate().take(3) {
        output.push_str(&format_item_breakdown(idx + 1, item));
    }

    output.push_str("</details>\n\n");
    output
}

fn format_item_breakdown(number: usize, item: &UnifiedDebtItem) -> String {
    format!(
        "#### {}. {}\n\n{}\n",
        number,
        item.location.function,
        format_score_factors(&item.unified_score)
    )
}

fn format_score_factors(score: &crate::priority::unified_scorer::UnifiedScore) -> String {
    format!(
        "- **Priority Score**: {:.2}\n\
         - **Complexity Factor**: {:.2}\n\
         - **Coverage Factor**: {:.2}\n\
         - **ROI Factor**: {:.2}\n\
         - **Semantic Factor**: {:.2}\n",
        score.final_score,
        score.complexity_factor,
        score.coverage_factor,
        score.roi_factor,
        score.semantic_factor
    )
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

// Pure functions for testing recommendations
fn collect_testing_gaps(items: &[UnifiedDebtItem]) -> Vec<&UnifiedDebtItem> {
    items
        .iter()
        .filter(|item| matches!(item.debt_type, crate::priority::DebtType::TestingGap { .. }))
        .take(10)
        .collect()
}

fn format_testing_table_header() -> String {
    "### ROI-Based Testing Priorities\n\n\
     | Function | ROI | Complexity | Coverage | Risk Reduction |\n\
     |----------|-----|------------|----------|----------------|\n"
        .to_string()
}

fn format_testing_gap_row(item: &UnifiedDebtItem) -> Option<String> {
    if let crate::priority::DebtType::TestingGap {
        coverage,
        cyclomatic,
        cognitive: _,
    } = &item.debt_type
    {
        let roi = calculate_roi(item);
        let risk_reduction = estimate_risk_reduction(*coverage);

        Some(format!(
            "| `{}` | {:.1} | {} | {:.0}% | {:.0}% |\n",
            item.location.function,
            roi,
            cyclomatic,
            coverage * 100.0,
            risk_reduction * 100.0
        ))
    } else {
        None
    }
}

fn format_testing_recommendations(testing_gaps: &[&UnifiedDebtItem]) -> String {
    if testing_gaps.is_empty() {
        return "_All critical functions have adequate test coverage._\n\n".to_string();
    }

    let mut output = format_testing_table_header();

    for item in testing_gaps {
        if let Some(row) = format_testing_gap_row(item) {
            output.push_str(&row);
        }
    }
    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::unified_scorer::{Location, UnifiedDebtItem, UnifiedScore};
    use crate::priority::{ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics};

    fn create_test_item(function_name: &str, final_score: f64) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: std::path::PathBuf::from("test.rs"),
                line: 10,
                function: function_name.to_string(),
            },
            unified_score: UnifiedScore {
                final_score,
                complexity_factor: 0.8,
                coverage_factor: 0.6,
                roi_factor: 0.7,
                semantic_factor: 0.5,
                dependency_factor: 0.5,
                role_multiplier: 1.0,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor to reduce complexity".to_string(),
                rationale: "Test recommendation".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 5.0,
                coverage_improvement: 0.1,
                lines_reduction: 10,
                risk_reduction: 0.2,
            },
            transitive_coverage: None,
            upstream_dependencies: 1,
            downstream_dependencies: 2,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 50,
            cyclomatic_complexity: 15,
            cognitive_complexity: 20,
        }
    }

    #[test]
    fn test_format_score_factors() {
        let score = UnifiedScore {
            final_score: 7.89,
            complexity_factor: 0.85,
            coverage_factor: 0.65,
            roi_factor: 0.75,
            semantic_factor: 0.55,
            dependency_factor: 0.45,
            role_multiplier: 1.0,
        };

        let result = format_score_factors(&score);

        assert!(result.contains("Priority Score**: 7.89"));
        assert!(result.contains("Complexity Factor**: 0.85"));
        assert!(result.contains("Coverage Factor**: 0.65"));
        assert!(result.contains("ROI Factor**: 0.75"));
        assert!(result.contains("Semantic Factor**: 0.55"));
    }

    #[test]
    fn test_format_item_breakdown() {
        let item = create_test_item("test_function", 8.5);
        let result = format_item_breakdown(1, &item);

        assert!(result.starts_with("#### 1. test_function\n"));
        assert!(result.contains("Priority Score**: 8.50"));
        assert!(result.contains("Complexity Factor**: 0.80"));
    }

    #[test]
    fn test_format_score_breakdown_empty() {
        let items: Vec<UnifiedDebtItem> = vec![];
        let result = format_score_breakdown(&items);

        assert!(result.starts_with("<details>\n"));
        assert!(result.contains("<summary>Score Breakdown (click to expand)</summary>"));
        assert!(result.ends_with("</details>\n\n"));
    }

    #[test]
    fn test_format_score_breakdown_single_item() {
        let items = vec![create_test_item("function_one", 9.0)];
        let result = format_score_breakdown(&items);

        assert!(result.contains("#### 1. function_one"));
        assert!(result.contains("Priority Score**: 9.00"));
        assert!(result.contains("<details>"));
        assert!(result.contains("</details>"));
    }

    #[test]
    fn test_format_score_breakdown_multiple_items() {
        let items = vec![
            create_test_item("function_one", 9.0),
            create_test_item("function_two", 7.5),
            create_test_item("function_three", 6.0),
        ];
        let result = format_score_breakdown(&items);

        assert!(result.contains("#### 1. function_one"));
        assert!(result.contains("#### 2. function_two"));
        assert!(result.contains("#### 3. function_three"));
        assert!(result.contains("Priority Score**: 9.00"));
        assert!(result.contains("Priority Score**: 7.50"));
        assert!(result.contains("Priority Score**: 6.00"));
    }

    #[test]
    fn test_format_score_breakdown_limits_to_three() {
        let items = vec![
            create_test_item("function_one", 9.0),
            create_test_item("function_two", 7.5),
            create_test_item("function_three", 6.0),
            create_test_item("function_four", 5.0),
            create_test_item("function_five", 4.0),
        ];
        let result = format_score_breakdown(&items);

        // Should only include first three
        assert!(result.contains("#### 1. function_one"));
        assert!(result.contains("#### 2. function_two"));
        assert!(result.contains("#### 3. function_three"));
        assert!(!result.contains("#### 4. function_four"));
        assert!(!result.contains("#### 5. function_five"));
    }

    #[test]
    fn test_format_score_factors_precision() {
        let score = UnifiedScore {
            final_score: 7.899999,
            complexity_factor: 0.855555,
            coverage_factor: 0.654321,
            roi_factor: 0.751234,
            semantic_factor: 0.559876,
            dependency_factor: 0.456789,
            role_multiplier: 1.0,
        };

        let result = format_score_factors(&score);

        // Check that all values are rounded to 2 decimal places
        assert!(result.contains("Priority Score**: 7.90"));
        assert!(result.contains("Complexity Factor**: 0.86"));
        assert!(result.contains("Coverage Factor**: 0.65"));
        assert!(result.contains("ROI Factor**: 0.75"));
        assert!(result.contains("Semantic Factor**: 0.56"));
    }

    #[test]
    fn test_format_item_breakdown_escapes_special_chars() {
        let mut item = create_test_item("test_function_with_<special>&_chars", 8.5);
        item.location.function = "test_function_with_<special>&_chars".to_string();
        let result = format_item_breakdown(1, &item);

        // The function name should be included as-is in markdown
        assert!(result.contains("test_function_with_<special>&_chars"));
    }

    fn create_testing_gap_item(
        function_name: &str,
        coverage: f64,
        cyclomatic: u32,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: std::path::PathBuf::from("test.rs"),
                line: 10,
                function: function_name.to_string(),
            },
            unified_score: UnifiedScore {
                final_score: 8.0,
                complexity_factor: 0.8,
                coverage_factor: 0.6,
                roi_factor: 0.7,
                semantic_factor: 0.5,
                dependency_factor: 0.5,
                role_multiplier: 1.0,
            },
            debt_type: DebtType::TestingGap {
                coverage,
                cyclomatic,
                cognitive: 20,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Add unit tests".to_string(),
                rationale: "Increase test coverage".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 0.0,
                coverage_improvement: 0.5,
                lines_reduction: 0,
                risk_reduction: 0.3,
            },
            transitive_coverage: None,
            upstream_dependencies: 1,
            downstream_dependencies: 2,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 50,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: 20,
        }
    }

    #[test]
    fn test_collect_testing_gaps() {
        let items = vec![
            create_testing_gap_item("func1", 0.2, 10),
            create_test_item("func2", 7.0), // Not a testing gap
            create_testing_gap_item("func3", 0.4, 15),
            create_testing_gap_item("func4", 0.1, 20),
        ];

        let gaps = collect_testing_gaps(&items);

        assert_eq!(gaps.len(), 3);
        assert_eq!(gaps[0].location.function, "func1");
        assert_eq!(gaps[1].location.function, "func3");
        assert_eq!(gaps[2].location.function, "func4");
    }

    #[test]
    fn test_collect_testing_gaps_empty() {
        let items = vec![
            create_test_item("func1", 7.0),
            create_test_item("func2", 8.0),
        ];

        let gaps = collect_testing_gaps(&items);

        assert!(gaps.is_empty());
    }

    #[test]
    fn test_collect_testing_gaps_limits_to_ten() {
        let mut items = Vec::new();
        for i in 0..15 {
            items.push(create_testing_gap_item(&format!("func{}", i), 0.2, 10));
        }

        let gaps = collect_testing_gaps(&items);

        assert_eq!(gaps.len(), 10);
    }

    #[test]
    fn test_format_testing_table_header() {
        let header = format_testing_table_header();

        assert!(header.contains("### ROI-Based Testing Priorities"));
        assert!(header.contains("| Function | ROI | Complexity | Coverage | Risk Reduction |"));
        assert!(header.contains("|----------|-----|------------|----------|----------------|"));
    }

    #[test]
    fn test_format_testing_gap_row() {
        let item = create_testing_gap_item("test_function", 0.3, 12);

        let row = format_testing_gap_row(&item).unwrap();

        assert!(row.contains("`test_function`"));
        assert!(row.contains("| 7.0 |")); // ROI = roi_factor * 10
        assert!(row.contains("| 12 |")); // Cyclomatic complexity
        assert!(row.contains("| 30% |")); // Coverage
        assert!(row.contains("| 21% |")); // Risk reduction = (1-0.3)*0.3
    }

    #[test]
    fn test_format_testing_gap_row_non_testing_gap() {
        let item = create_test_item("test_function", 8.0);

        let row = format_testing_gap_row(&item);

        assert!(row.is_none());
    }

    #[test]
    fn test_format_testing_recommendations_empty() {
        let gaps: Vec<&UnifiedDebtItem> = vec![];

        let result = format_testing_recommendations(&gaps);

        assert!(result.contains("_All critical functions have adequate test coverage._"));
    }

    #[test]
    fn test_format_testing_recommendations_with_gaps() {
        let items = vec![
            create_testing_gap_item("func1", 0.2, 10),
            create_testing_gap_item("func2", 0.5, 15),
        ];
        let gaps: Vec<&UnifiedDebtItem> = items.iter().collect();

        let result = format_testing_recommendations(&gaps);

        assert!(result.contains("### ROI-Based Testing Priorities"));
        assert!(result.contains("`func1`"));
        assert!(result.contains("`func2`"));
        assert!(result.contains("| 10 |")); // func1 complexity
        assert!(result.contains("| 15 |")); // func2 complexity
    }

    #[test]
    fn test_calculate_roi() {
        let item = create_testing_gap_item("test", 0.3, 10);

        let roi = calculate_roi(&item);

        assert_eq!(roi, 7.0); // roi_factor (0.7) * 10
    }

    #[test]
    fn test_estimate_risk_reduction() {
        assert_eq!(estimate_risk_reduction(0.0), 0.3); // (1.0 - 0.0) * 0.3
        assert_eq!(estimate_risk_reduction(0.5), 0.15); // (1.0 - 0.5) * 0.3
        assert_eq!(estimate_risk_reduction(1.0), 0.0); // (1.0 - 1.0) * 0.3
    }

    #[test]
    fn test_write_testing_recommendations_with_gaps() {
        use crate::priority::{CallGraph, ImpactMetrics, UnifiedAnalysis};
        use im::Vector;

        let items = vec![
            create_testing_gap_item("critical_func", 0.1, 20),
            create_testing_gap_item("important_func", 0.3, 15),
            create_test_item("other_func", 5.0),
        ];

        let analysis = UnifiedAnalysis {
            items: Vector::from(items),
            total_impact: ImpactMetrics {
                complexity_reduction: 10.0,
                coverage_improvement: 0.2,
                lines_reduction: 20,
                risk_reduction: 0.3,
            },
            total_debt_score: 100.0,
            call_graph: CallGraph::new(),
            overall_coverage: Some(0.75),
        };

        let mut buffer = Vec::new();
        let mut writer = MarkdownWriter::new(&mut buffer);

        writer.write_testing_recommendations(&analysis).unwrap();

        let output = String::from_utf8(buffer).unwrap();

        assert!(output.contains("## Testing Recommendations"));
        assert!(output.contains("### ROI-Based Testing Priorities"));
        assert!(output.contains("`critical_func`"));
        assert!(output.contains("`important_func`"));
        assert!(!output.contains("`other_func`")); // Not a testing gap
        assert!(output.contains("| 20 |")); // critical_func complexity
        assert!(output.contains("| 15 |")); // important_func complexity
    }

    #[test]
    fn test_write_testing_recommendations_no_gaps() {
        use crate::priority::{CallGraph, ImpactMetrics, UnifiedAnalysis};
        use im::Vector;

        let items = vec![
            create_test_item("func1", 5.0),
            create_test_item("func2", 6.0),
        ];

        let analysis = UnifiedAnalysis {
            items: Vector::from(items),
            total_impact: ImpactMetrics {
                complexity_reduction: 5.0,
                coverage_improvement: 0.1,
                lines_reduction: 10,
                risk_reduction: 0.2,
            },
            total_debt_score: 50.0,
            call_graph: CallGraph::new(),
            overall_coverage: Some(0.85),
        };

        let mut buffer = Vec::new();
        let mut writer = MarkdownWriter::new(&mut buffer);

        writer.write_testing_recommendations(&analysis).unwrap();

        let output = String::from_utf8(buffer).unwrap();

        assert!(output.contains("## Testing Recommendations"));
        assert!(output.contains("_All critical functions have adequate test coverage._"));
        assert!(!output.contains("### ROI-Based Testing Priorities"));
    }
}
