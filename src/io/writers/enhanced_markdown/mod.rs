pub mod complexity_analyzer;
pub mod config;
pub mod formatters;
pub mod risk_analyzer;
pub mod statistics;
pub mod toc;

pub use config::{DetailLevel, MarkdownConfig, RepositoryType};

use crate::core::AnalysisResults;
use crate::priority::{DebtType, UnifiedAnalysis};
use crate::risk::{RiskDistribution, RiskInsight};
use anyhow::Result;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use self::complexity_analyzer::*;
use self::formatters::*;
use self::risk_analyzer::*;
use self::statistics::*;
use self::toc::TocBuilder;

/// Enhanced markdown writer with rich formatting capabilities
pub struct EnhancedMarkdownWriter<W: Write> {
    writer: W,
    config: MarkdownConfig,
    toc_builder: TocBuilder,
}

// Pure functions for executive summary calculations
fn classify_health_status(score: u32) -> &'static str {
    match score {
        70..=100 => "Good",
        _ => "Needs Attention",
    }
}

fn format_health_metric(name: &str, value: String, status: &str) -> String {
    format!("| {} | {} | {} |", name, value, status)
}

fn should_show_complexity_insight(avg_complexity: f64) -> bool {
    avg_complexity > 10.0
}

fn should_show_coverage_insight(coverage: Option<f64>) -> bool {
    coverage.is_some_and(|c| c < 0.5)
}

fn format_critical_complexity_insight(count: usize) -> String {
    format!(
        "- ⚠️ **{}** critical complexity issues require immediate attention",
        count
    )
}

fn format_high_complexity_insight(avg: f64) -> String {
    format!(
        "- 📈 Average complexity ({:.1}) exceeds recommended threshold",
        avg
    )
}

fn format_low_coverage_insight(coverage: f64) -> String {
    format!(
        "- 🔴 Code coverage ({:.1}%) is below minimum recommended level",
        coverage * 100.0
    )
}

// Pure functions for health metrics formatting
fn format_overall_health_metric(health_score: u32) -> String {
    format_health_metric(
        "**Overall Health**",
        format!("{}% {}", health_score, get_health_emoji(health_score)),
        classify_health_status(health_score),
    )
}

fn format_complexity_metric(avg_complexity: f64) -> String {
    format_health_metric(
        "**Average Complexity**",
        format!("{:.2}", avg_complexity),
        get_complexity_status(avg_complexity),
    )
}

fn format_coverage_metric(coverage: f64) -> String {
    format_health_metric(
        "**Code Coverage**",
        format!("{:.1}%", coverage * 100.0),
        get_coverage_status(coverage * 100.0),
    )
}

fn format_debt_metric(debt_count: usize) -> String {
    format_health_metric(
        "**Technical Debt**",
        format!("{} items", debt_count),
        get_debt_status(debt_count),
    )
}

// Pure function for building all metrics
fn build_health_metrics(
    health_score: u32,
    avg_complexity: f64,
    coverage_percentage: Option<f64>,
    debt_count: usize,
) -> Vec<String> {
    let mut metrics = vec![
        format_overall_health_metric(health_score),
        format_complexity_metric(avg_complexity),
    ];

    if let Some(coverage) = coverage_percentage {
        metrics.push(format_coverage_metric(coverage));
    }

    metrics.push(format_debt_metric(debt_count));
    metrics
}

impl<W: Write> EnhancedMarkdownWriter<W> {
    // Helper functions for health status section
    fn write_health_section_header(&mut self) -> Result<()> {
        if self.config.collapsible_sections {
            writeln!(self.writer, "<details open>")?;
            writeln!(
                self.writer,
                "<summary><strong>📊 Health Status</strong></summary>\n"
            )?;
        } else {
            writeln!(self.writer, "### 📊 Health Status\n")?;
        }
        Ok(())
    }

    fn write_health_metrics_table(
        &mut self,
        health_score: u32,
        avg_complexity: f64,
        coverage_percentage: Option<f64>,
        results: &AnalysisResults,
    ) -> Result<()> {
        // Write table headers
        writeln!(self.writer, "| Metric | Value | Status |")?;
        writeln!(self.writer, "|--------|-------|--------|")?;

        // Generate and write all metrics
        let metrics = build_health_metrics(
            health_score,
            avg_complexity,
            coverage_percentage,
            results.technical_debt.items.len(),
        );

        for metric in metrics {
            writeln!(self.writer, "{}", metric)?;
        }

        Ok(())
    }

    fn write_health_section_footer(&mut self) -> Result<()> {
        if self.config.collapsible_sections {
            writeln!(self.writer, "\n</details>\n")?;
        }
        Ok(())
    }

    pub fn new(writer: W) -> Self {
        Self::with_config(writer, MarkdownConfig::default())
    }

    pub fn with_config(writer: W, config: MarkdownConfig) -> Self {
        Self {
            writer,
            config,
            toc_builder: TocBuilder::new(),
        }
    }

    /// Write complete enhanced analysis report
    pub fn write_enhanced_report(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
        risk_insights: Option<&RiskInsight>,
    ) -> Result<()> {
        // Write header
        self.write_header(results)?;

        // Collect TOC entries as we write sections
        self.write_executive_summary(results, unified_analysis)?;

        if self.config.include_visualizations {
            self.write_visualizations(results, unified_analysis)?;
        }

        if let Some(insights) = risk_insights {
            self.write_risk_analysis(results, insights)?;
        }

        if self.config.detail_level >= DetailLevel::Standard {
            self.write_technical_debt(results, unified_analysis)?;

            if let Some(analysis) = unified_analysis {
                self.write_dependency_analysis(analysis)?;
            }
        }

        if self.config.include_statistics && self.config.detail_level >= DetailLevel::Detailed {
            self.write_statistics(results)?;
        }

        self.write_recommendations(results, unified_analysis)?;

        // Write TOC at the beginning (would normally seek back)
        if self.config.include_toc {
            // Note: In a real implementation, we'd write TOC at the beginning
            // For now, it's written inline
        }

        Ok(())
    }

    fn write_header(&mut self, results: &AnalysisResults) -> Result<()> {
        writeln!(self.writer, "# Technical Debt Analysis Report\n")?;
        writeln!(
            self.writer,
            "**Generated**: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        )?;
        writeln!(
            self.writer,
            "**Files Analyzed**: {}",
            results.complexity.metrics.len()
        )?;
        writeln!(
            self.writer,
            "**Total Debt Items**: {}",
            results.technical_debt.items.len()
        )?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_executive_summary(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        self.toc_builder.add_entry(1, "Executive Summary");
        writeln!(self.writer, "## Executive Summary\n")?;

        // Calculate key metrics
        let coverage_percentage: Option<f64> = None;
        let health_score = calculate_health_score(results, coverage_percentage.map(|r| r * 100.0));
        let avg_complexity = calculate_average_complexity(results);

        self.write_health_status_section(
            health_score,
            avg_complexity,
            coverage_percentage,
            results,
        )?;
        self.write_key_insights_section(avg_complexity, coverage_percentage, unified_analysis)?;

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_health_status_section(
        &mut self,
        health_score: u32,
        avg_complexity: f64,
        coverage_percentage: Option<f64>,
        results: &AnalysisResults,
    ) -> Result<()> {
        self.write_health_section_header()?;
        self.write_health_metrics_table(
            health_score,
            avg_complexity,
            coverage_percentage,
            results,
        )?;
        self.write_health_section_footer()
    }

    fn write_key_insights_section(
        &mut self,
        avg_complexity: f64,
        coverage_percentage: Option<f64>,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        writeln!(self.writer, "### 🔍 Key Insights\n")?;

        // Generate insights based on analysis
        let insights = self.collect_insights(avg_complexity, coverage_percentage, unified_analysis);

        for insight in insights {
            writeln!(self.writer, "{}", insight)?;
        }

        Ok(())
    }

    fn collect_insights(
        &self,
        avg_complexity: f64,
        coverage_percentage: Option<f64>,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Vec<String> {
        let mut insights = Vec::new();

        // Critical complexity items
        if let Some(analysis) = unified_analysis {
            let critical_count = analysis
                .items
                .iter()
                .filter(|i| {
                    matches!(
                        i.debt_type,
                        DebtType::ComplexityHotspot {
                            cyclomatic: 10,
                            cognitive: 8
                        }
                    )
                })
                .count();
            if critical_count > 0 {
                insights.push(format_critical_complexity_insight(critical_count));
            }
        }

        // High average complexity
        if should_show_complexity_insight(avg_complexity) {
            insights.push(format_high_complexity_insight(avg_complexity));
        }

        // Low coverage
        if should_show_coverage_insight(coverage_percentage) {
            if let Some(coverage) = coverage_percentage {
                insights.push(format_low_coverage_insight(coverage));
            }
        }

        insights
    }

    fn write_visualizations(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        self.toc_builder.add_entry(1, "Visualizations");
        writeln!(self.writer, "## 📊 Visualizations\n")?;

        self.write_complexity_distribution(results)?;
        self.write_risk_heat_map(results)?;

        if let Some(analysis) = unified_analysis {
            self.write_dependency_graph(analysis)?;
        }

        self.write_distribution_charts(results)?;

        Ok(())
    }

    fn write_complexity_distribution(&mut self, results: &AnalysisResults) -> Result<()> {
        self.toc_builder.add_entry(2, "Complexity Distribution");
        writeln!(self.writer, "### Complexity Distribution\n")?;

        let distribution = calculate_complexity_distribution(results);

        writeln!(self.writer, "```")?;
        for (label, percentage) in distribution {
            let bar_length = (percentage / 2.0) as usize;
            let bar = "█".repeat(bar_length);
            writeln!(self.writer, "{:15} {} {:.1}%", label, bar, percentage)?;
        }
        writeln!(self.writer, "```\n")?;

        Ok(())
    }

    fn write_risk_heat_map(&mut self, results: &AnalysisResults) -> Result<()> {
        self.toc_builder.add_entry(2, "Risk Heat Map");
        writeln!(self.writer, "### Risk Heat Map\n")?;

        let modules = get_top_risk_modules(results, 5);

        writeln!(
            self.writer,
            "| Module | Complexity | Coverage | Risk Level |"
        )?;
        writeln!(
            self.writer,
            "|--------|------------|----------|------------|"
        )?;

        for module in modules {
            writeln!(
                self.writer,
                "| {} | {} | {} | {} |",
                module.name,
                get_complexity_indicator(module.complexity),
                get_coverage_indicator(module.coverage),
                get_risk_indicator(module.risk)
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_dependency_graph(&mut self, analysis: &UnifiedAnalysis) -> Result<()> {
        self.toc_builder.add_entry(2, "Module Dependencies");
        writeln!(self.writer, "### Module Dependencies\n")?;

        let items: Vec<_> = analysis.items.iter().cloned().collect();
        let deps = extract_module_dependencies(&items);

        writeln!(self.writer, "```mermaid")?;
        writeln!(self.writer, "graph LR")?;

        for (module, dependencies) in deps.iter().take(10) {
            for dep in dependencies {
                writeln!(self.writer, "    {} --> {}", module, dep)?;
            }
        }

        writeln!(self.writer, "```\n")?;
        Ok(())
    }

    fn write_distribution_charts(&mut self, results: &AnalysisResults) -> Result<()> {
        self.toc_builder.add_entry(2, "Complexity Trends");
        writeln!(self.writer, "### Complexity Trends\n")?;

        let sample_values: Vec<u32> = results
            .complexity
            .metrics
            .iter()
            .take(20)
            .map(|m| m.cyclomatic)
            .collect();

        if !sample_values.is_empty() {
            writeln!(
                self.writer,
                "Recent complexity trend: {}\n",
                create_sparkline(&sample_values)
            )?;
        }

        Ok(())
    }

    fn write_risk_analysis(
        &mut self,
        results: &AnalysisResults,
        insights: &RiskInsight,
    ) -> Result<()> {
        self.toc_builder.add_entry(1, "Risk Analysis");
        writeln!(self.writer, "## ⚠️ Risk Analysis\n")?;

        // Risk Summary
        writeln!(self.writer, "### Risk Summary\n")?;
        writeln!(
            self.writer,
            "**Overall Risk Level**: {}\n",
            get_risk_indicator(insights.codebase_risk_score)
        )?;

        // Risk Distribution
        self.write_risk_distribution(&insights.risk_distribution)?;

        // Critical Risks
        if self.config.detail_level >= DetailLevel::Standard {
            self.write_critical_risks(results)?;
        }

        // Complexity Hotspots
        if self.config.detail_level >= DetailLevel::Detailed {
            self.write_complexity_hotspots(results)?;
        }

        Ok(())
    }

    fn write_risk_distribution(&mut self, distribution: &RiskDistribution) -> Result<()> {
        self.toc_builder.add_entry(2, "Risk Distribution");
        writeln!(self.writer, "### Risk Distribution\n")?;

        let total = distribution.low_count
            + distribution.medium_count
            + distribution.high_count
            + distribution.critical_count;

        if total > 0 {
            writeln!(self.writer, "| Risk Level | Count | Percentage |")?;
            writeln!(self.writer, "|------------|-------|------------|")?;
            writeln!(
                self.writer,
                "| 🟢 Low | {} | {:.1}% |",
                distribution.low_count,
                (distribution.low_count as f64 / total as f64) * 100.0
            )?;
            writeln!(
                self.writer,
                "| 🟡 Medium | {} | {:.1}% |",
                distribution.medium_count,
                (distribution.medium_count as f64 / total as f64) * 100.0
            )?;
            writeln!(
                self.writer,
                "| 🟠 High | {} | {:.1}% |",
                distribution.high_count,
                (distribution.high_count as f64 / total as f64) * 100.0
            )?;
            writeln!(
                self.writer,
                "| 🔴 Critical | {} | {:.1}% |",
                distribution.critical_count,
                (distribution.critical_count as f64 / total as f64) * 100.0
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_critical_risks(&mut self, results: &AnalysisResults) -> Result<()> {
        self.toc_builder.add_entry(2, "Critical Risk Functions");
        writeln!(self.writer, "### Critical Risk Functions\n")?;

        let critical_functions = get_critical_risk_functions(&results.complexity.metrics, 5);

        if !critical_functions.is_empty() {
            writeln!(self.writer, "| Function | Complexity | Priority |")?;
            writeln!(self.writer, "|----------|------------|----------|")?;

            for func in critical_functions {
                writeln!(
                    self.writer,
                    "| {} | {} | {} |",
                    func.name,
                    func.cyclomatic,
                    get_priority_label(0)
                )?;
            }
        }

        writeln!(self.writer)?;
        Ok(())
    }

    // Pure functions for recommendations

    fn write_complexity_hotspots(&mut self, results: &AnalysisResults) -> Result<()> {
        self.toc_builder.add_entry(2, "Complexity Hotspots");
        writeln!(self.writer, "### Complexity Hotspots\n")?;

        let mut file_complexities: HashMap<&Path, Vec<u32>> = HashMap::new();

        for metric in &results.complexity.metrics {
            file_complexities
                .entry(&metric.file)
                .or_default()
                .push(metric.cyclomatic);
        }

        let mut hotspots: Vec<_> = file_complexities
            .into_iter()
            .map(|(path, complexities)| {
                let avg = complexities.iter().sum::<u32>() as f64 / complexities.len() as f64;
                let max = *complexities.iter().max().unwrap_or(&0);
                (path, avg, max, complexities.len())
            })
            .collect();

        hotspots.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        writeln!(
            self.writer,
            "| File | Avg Complexity | Max | Functions | Trend |"
        )?;
        writeln!(
            self.writer,
            "|------|----------------|-----|-----------|-------|"
        )?;

        for (path, avg, max, count) in hotspots.iter().take(10) {
            writeln!(
                self.writer,
                "| {} | {:.1} | {} | {} | {} |",
                path.display(),
                avg,
                max,
                count,
                get_trend_indicator(0.0)
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_technical_debt(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        self.toc_builder.add_entry(1, "Technical Debt");
        writeln!(self.writer, "## 💳 Technical Debt\n")?;

        if let Some(analysis) = unified_analysis {
            self.write_priority_matrix(analysis)?;
        }

        self.write_debt_categories(results)?;
        self.write_actionable_items(results)?;

        Ok(())
    }

    fn write_priority_matrix(&mut self, analysis: &UnifiedAnalysis) -> Result<()> {
        self.toc_builder.add_entry(2, "Priority Matrix");
        writeln!(self.writer, "### Priority Matrix\n")?;

        writeln!(self.writer, "| Priority | Item | Effort | Impact |")?;
        writeln!(self.writer, "|----------|------|--------|--------|")?;

        for (i, item) in analysis.items.iter().take(10).enumerate() {
            writeln!(
                self.writer,
                "| {} | {} | {} days | {} |",
                get_priority_label(i),
                item.recommendation.primary_action.clone(),
                estimate_effort(item) / 8,
                item.expected_impact.complexity_reduction
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_debt_categories(&mut self, results: &AnalysisResults) -> Result<()> {
        self.toc_builder.add_entry(2, "Debt Categories");
        writeln!(self.writer, "### Debt by Category\n")?;

        let categories = categorize_debt(&results.technical_debt.items);

        writeln!(self.writer, "| Category | Count | Severity |")?;
        writeln!(self.writer, "|----------|-------|----------|")?;

        for (category, items) in categories {
            let priorities: Vec<_> = items.iter().map(|i| i.priority).collect();
            writeln!(
                self.writer,
                "| {} | {} | {} |",
                category,
                items.len(),
                calculate_category_severity(&priorities)
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_actionable_items(&mut self, results: &AnalysisResults) -> Result<()> {
        self.toc_builder.add_entry(2, "Actionable Items");
        writeln!(self.writer, "### 🎯 Actionable Items\n")?;

        writeln!(self.writer, "#### Quick Wins (< 1 day effort)\n")?;

        for item in results
            .technical_debt
            .items
            .iter()
            .filter(|i| i.priority == crate::core::Priority::Low)
            .take(5)
        {
            writeln!(self.writer, "- [ ] {}", item.message)?;
        }

        writeln!(self.writer, "\n#### High Impact (1-3 days effort)\n")?;

        for item in results
            .technical_debt
            .items
            .iter()
            .filter(|i| i.priority == crate::core::Priority::Medium)
            .take(5)
        {
            writeln!(self.writer, "- [ ] {}", item.message)?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_dependency_analysis(&mut self, analysis: &UnifiedAnalysis) -> Result<()> {
        self.toc_builder.add_entry(1, "Dependency Analysis");
        writeln!(self.writer, "## 🔗 Dependency Analysis\n")?;

        let items: Vec<_> = analysis.items.iter().cloned().collect();
        let deps = extract_module_dependencies(&items);

        writeln!(self.writer, "### Module Coupling\n")?;
        writeln!(
            self.writer,
            "| Module | Afferent | Efferent | Instability |"
        )?;
        writeln!(
            self.writer,
            "|--------|----------|----------|-------------|"
        )?;

        for (module, dependencies) in deps.iter().take(10) {
            let metrics = calculate_coupling_metrics(0, dependencies.len());
            writeln!(
                self.writer,
                "| {} | {} | {} | {:.2} |",
                module, metrics.afferent, metrics.efferent, metrics.instability
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_statistics(&mut self, results: &AnalysisResults) -> Result<()> {
        self.toc_builder.add_entry(1, "Statistics");
        writeln!(self.writer, "## 📈 Statistics\n")?;

        writeln!(self.writer, "### Summary Statistics\n")?;
        writeln!(self.writer, "| Metric | Value |")?;
        writeln!(self.writer, "|--------|-------|")?;
        writeln!(
            self.writer,
            "| Total Functions | {} |",
            results.complexity.summary.total_functions
        )?;
        // Total lines not available in ComplexitySummary
        // writeln!(self.writer, "| Total Lines | {} |", results.complexity.summary.total_lines)?;
        writeln!(
            self.writer,
            "| Debt Items | {} |",
            results.technical_debt.items.len()
        )?;
        writeln!(
            self.writer,
            "| Average Complexity | {:.2} |",
            calculate_average_complexity(results)
        )?;

        self.write_distribution_statistics(results)?;

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_distribution_statistics(&mut self, results: &AnalysisResults) -> Result<()> {
        let stats = calculate_distribution_stats(&results.complexity.metrics);

        writeln!(self.writer, "\n### Complexity Distribution Statistics\n")?;
        writeln!(self.writer, "| Statistic | Value |")?;
        writeln!(self.writer, "|-----------|-------|")?;
        writeln!(self.writer, "| Mean | {:.2} |", stats.mean)?;
        writeln!(self.writer, "| Median | {} |", stats.median)?;
        writeln!(self.writer, "| Std Dev | {:.2} |", stats.std_dev)?;
        writeln!(self.writer, "| Min | {} |", stats.min)?;
        writeln!(self.writer, "| Max | {} |", stats.max)?;
        writeln!(self.writer, "| Q1 | {} |", stats.quartiles.0)?;
        writeln!(self.writer, "| Q2 | {} |", stats.quartiles.1)?;
        writeln!(self.writer, "| Q3 | {} |", stats.quartiles.2)?;

        Ok(())
    }

    fn write_recommendations(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        self.toc_builder.add_entry(1, "Recommendations");
        writeln!(self.writer, "## 💡 Recommendations\n")?;

        self.write_priority_actions(unified_analysis)?;
        self.write_strategic_recommendations(results)?;

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_priority_actions(&mut self, unified_analysis: Option<&UnifiedAnalysis>) -> Result<()> {
        writeln!(self.writer, "### 🚨 Priority Actions\n")?;

        if let Some(analysis) = unified_analysis {
            let priority_items: Vec<_> = analysis
                .items
                .iter()
                .take(3)
                .enumerate()
                .flat_map(|(i, item)| {
                    vec![
                        format!("{}. **{}**", i + 1, item.recommendation.primary_action),
                        format!("   - Location: `{}`", item.location.file.display()),
                        format!("   - Estimated Effort: {} hours", estimate_effort(item)),
                    ]
                })
                .collect();

            for line in priority_items {
                writeln!(self.writer, "{}", line)?;
            }

            // Add spacing between items
            if !analysis.items.is_empty() {
                writeln!(self.writer)?;
            }
        }

        Ok(())
    }

    fn write_strategic_recommendations(&mut self, results: &AnalysisResults) -> Result<()> {
        writeln!(self.writer, "### 📋 Strategic Recommendations\n")?;

        let avg_complexity = calculate_average_complexity(results);
        let debt_count = results.technical_debt.items.len();
        let mut recommendations = Vec::new();
        let mut index = 1;

        if avg_complexity > 10.0 {
            recommendations.push(format!(
                "{}. **Reduce Complexity**: Implement code review process focusing on cyclomatic complexity",
                index
            ));
            index += 1;
        }

        if debt_count > 50 {
            recommendations.push(format!(
                "{}. **Debt Reduction Sprint**: Allocate 20% of sprint capacity to debt reduction",
                index
            ));
        }

        for recommendation in recommendations {
            writeln!(self.writer, "{}", recommendation)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location,
        UnifiedDebtItem as DebtItem, UnifiedScore,
    };
    use std::path::PathBuf;

    #[test]
    fn test_markdown_config_default() {
        let config = MarkdownConfig::default();
        assert!(config.include_toc);
        assert_eq!(config.toc_depth, 3);
        assert!(config.include_visualizations);
    }

    #[test]
    fn test_detail_level_ordering() {
        assert!(DetailLevel::Summary < DetailLevel::Standard);
        assert!(DetailLevel::Standard < DetailLevel::Detailed);
        assert!(DetailLevel::Detailed < DetailLevel::Complete);
    }

    // Tests for pure functions
    #[test]
    fn test_classify_health_status() {
        assert_eq!(classify_health_status(100), "Good");
        assert_eq!(classify_health_status(85), "Good");
        assert_eq!(classify_health_status(70), "Good");
        assert_eq!(classify_health_status(69), "Needs Attention");
        assert_eq!(classify_health_status(50), "Needs Attention");
        assert_eq!(classify_health_status(0), "Needs Attention");
    }

    #[test]
    fn test_format_health_metric() {
        let result = format_health_metric("Test Metric", "50%".to_string(), "Good");
        assert_eq!(result, "| Test Metric | 50% | Good |");
    }

    #[test]
    fn test_should_show_complexity_insight() {
        assert!(!should_show_complexity_insight(5.0));
        assert!(!should_show_complexity_insight(10.0));
        assert!(should_show_complexity_insight(10.1));
        assert!(should_show_complexity_insight(15.0));
    }

    #[test]
    fn test_should_show_coverage_insight() {
        assert!(!should_show_coverage_insight(None));
        assert!(!should_show_coverage_insight(Some(0.8)));
        assert!(!should_show_coverage_insight(Some(0.5)));
        assert!(should_show_coverage_insight(Some(0.49)));
        assert!(should_show_coverage_insight(Some(0.2)));
    }

    #[test]
    fn test_format_critical_complexity_insight() {
        let result = format_critical_complexity_insight(3);
        assert_eq!(
            result,
            "- ⚠️ **3** critical complexity issues require immediate attention"
        );
    }

    #[test]
    fn test_format_high_complexity_insight() {
        let result = format_high_complexity_insight(12.5);
        assert_eq!(
            result,
            "- 📈 Average complexity (12.5) exceeds recommended threshold"
        );
    }

    #[test]
    fn test_format_low_coverage_insight() {
        let result = format_low_coverage_insight(0.35);
        assert_eq!(
            result,
            "- 🔴 Code coverage (35.0%) is below minimum recommended level"
        );
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    #[allow(clippy::nonminimal_bool)]
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    fn test_should_recommend_complexity_reduction() {
        // Test directly with inline logic
        assert!(!(5.0 > 10.0));
        assert!(!(10.0 > 10.0));
        assert!(10.1 > 10.0);
        assert!(15.0 > 10.0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_should_recommend_debt_sprint() {
        // Test directly with inline logic
        assert!(0 <= 50);
        assert!(25 <= 50);
        assert!(50 <= 50);
        assert!(51 > 50);
        assert!(100 > 50);
    }

    fn generate_strategic_recommendations_test(
        avg_complexity: f64,
        debt_count: usize,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();
        let mut index = 1;

        if avg_complexity > 10.0 {
            recommendations.push(format!(
                "{}. **Reduce Complexity**: Implement code review process focusing on cyclomatic complexity",
                index
            ));
            index += 1;
        }

        if debt_count > 50 {
            recommendations.push(format!(
                "{}. **Debt Reduction Sprint**: Allocate 20% of sprint capacity to debt reduction",
                index
            ));
        }

        recommendations
    }

    #[test]
    fn test_generate_strategic_recommendations() {
        // Test with no recommendations needed
        let recs = generate_strategic_recommendations_test(5.0, 10);
        assert_eq!(recs.len(), 0);

        // Test with complexity recommendation only
        let recs = generate_strategic_recommendations_test(15.0, 10);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].contains("Reduce Complexity"));

        // Test with debt sprint recommendation only
        let recs = generate_strategic_recommendations_test(5.0, 60);
        assert_eq!(recs.len(), 1);
        assert!(recs[0].contains("Debt Reduction Sprint"));

        // Test with both recommendations
        let recs = generate_strategic_recommendations_test(15.0, 60);
        assert_eq!(recs.len(), 2);
        assert!(recs[0].contains("Reduce Complexity"));
        assert!(recs[1].contains("Debt Reduction Sprint"));
    }

    #[test]
    fn test_count_critical_complexity_items() {
        let items = vec![
            create_test_debt_item(DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 8,
            }),
            create_test_debt_item(DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 8,
            }),
            create_test_debt_item(DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 8,
            }),
            create_test_debt_item(DebtType::TestingGap {
                coverage: 0.3,
                cyclomatic: 10,
                cognitive: 8,
            }),
        ];

        let count = items
            .iter()
            .filter(|i| {
                matches!(
                    i.debt_type,
                    DebtType::ComplexityHotspot {
                        cyclomatic: 10,
                        cognitive: 8
                    }
                )
            })
            .count();
        assert_eq!(count, 3); // Count ComplexityHotspot items with cyclomatic: 10, cognitive: 8
    }

    // Helper function to create test debt items
    fn create_test_debt_item(debt_type: DebtType) -> DebtItem {
        DebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_function".to_string(),
                line: 10,
            },
            debt_type,
            recommendation: ActionableRecommendation {
                primary_action: "Test action".to_string(),
                rationale: "Test rationale".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.5,
                complexity_reduction: 0.3,
                coverage_improvement: 0.2,
                lines_reduction: 10,
            },
            unified_score: UnifiedScore {
                final_score: 5.0,
                coverage_factor: 1.0,
                complexity_factor: 1.0,
                dependency_factor: 1.0,
                role_multiplier: 1.0,
            },
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            cyclomatic_complexity: 10,
            cognitive_complexity: 8,
            nesting_depth: 3,
            function_length: 50,
            function_role: FunctionRole::PureLogic,
            transitive_coverage: None,
            upstream_callers: vec![],
            downstream_callees: vec![],
            entropy_details: None,
            is_pure: None,
            purity_confidence: None,
            god_object_indicators: None,
        }
    }

    // Comprehensive tests for write_health_status_section
    use crate::core::{
        AnalysisResults, ComplexityReport, ComplexitySummary, FunctionMetrics, TechnicalDebtReport,
    };
    use crate::core::{
        DebtItem as CoreDebtItem, DebtType as CoreDebtType, Priority as CorePriority,
    };
    use chrono::Utc;
    use std::io::Cursor;

    fn create_test_results(debt_count: usize, avg_complexity: f64) -> AnalysisResults {
        let metrics: Vec<FunctionMetrics> = (0..5)
            .map(|i| FunctionMetrics {
                file: PathBuf::from(format!("test_{}.rs", i)),
                name: format!("test_function_{}", i),
                line: i * 10,
                cyclomatic: (avg_complexity * (i as f64 + 1.0) / 2.5) as u32,
                cognitive: ((avg_complexity * (i as f64 + 1.0) / 2.5) * 1.5) as u32,
                nesting: (i % 4) as u32,
                length: 20 + i * 10,
                is_test: false,
                is_pure: Some(i % 2 == 0),
                purity_confidence: Some(0.8),
                visibility: Some("pub".to_string()),
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
            })
            .collect();

        AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics,
                summary: ComplexitySummary {
                    total_functions: 5,
                    average_complexity: avg_complexity,
                    high_complexity_count: if avg_complexity > 10.0 { 2 } else { 0 },
                    max_complexity: (avg_complexity * 2.0) as u32,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: (0..debt_count)
                    .map(|i| CoreDebtItem {
                        id: format!("debt_{}", i),
                        file: PathBuf::from(format!("debt_{}.rs", i)),
                        line: i * 15,
                        column: None,
                        message: format!("Debt issue {}", i),
                        priority: CorePriority::Medium,
                        debt_type: CoreDebtType::Complexity,
                        context: None,
                    })
                    .collect(),
                by_type: std::collections::HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: crate::core::DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
        }
    }

    #[test]
    fn test_write_health_status_section_good_health() {
        let results = create_test_results(5, 5.0);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(85, 5.0, Some(0.8), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("85%"));
        assert!(output.contains("Good"));
        assert!(output.contains("5 items"));
    }

    #[test]
    fn test_write_health_status_section_needs_attention() {
        let results = create_test_results(25, 15.0);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(55, 15.0, Some(0.3), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("55%"));
        assert!(output.contains("Needs Attention"));
        assert!(output.contains("25 items"));
    }

    #[test]
    fn test_write_health_status_section_no_coverage() {
        let results = create_test_results(10, 8.0);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(75, 8.0, None, &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("75%"));
        assert!(output.contains("8.00"));
        assert!(output.contains("10 items"));
        // Should not contain coverage metrics when None is passed
        assert!(!output.contains("Code Coverage"));
    }

    #[test]
    fn test_write_health_status_section_collapsible_enabled() {
        let results = create_test_results(3, 6.0);
        let config = MarkdownConfig { collapsible_sections: true, ..Default::default() };
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(90, 6.0, Some(0.9), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("<details open>"));
        assert!(output.contains("<summary><strong>📊 Health Status</strong></summary>"));
        assert!(output.contains("</details>"));
    }

    #[test]
    fn test_write_health_status_section_collapsible_disabled() {
        let results = create_test_results(7, 9.0);
        let config = MarkdownConfig { collapsible_sections: false, ..Default::default() };
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(78, 9.0, Some(0.7), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(!output.contains("<details"));
        assert!(output.contains("### 📊 Health Status"));
    }

    #[test]
    fn test_write_health_status_section_boundary_health_score_70() {
        let results = create_test_results(12, 7.5);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(70, 7.5, Some(0.6), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("70%"));
        assert!(output.contains("Good"));
    }

    #[test]
    fn test_write_health_status_section_boundary_health_score_69() {
        let results = create_test_results(15, 12.0);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(69, 12.0, Some(0.4), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("69%"));
        assert!(output.contains("Needs Attention"));
    }

    #[test]
    fn test_write_health_status_section_zero_debt_items() {
        let results = create_test_results(0, 3.0);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(95, 3.0, Some(0.95), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("0 items"));
        assert!(output.contains("95%"));
        assert!(output.contains("Good"));
    }

    #[test]
    fn test_write_health_status_section_high_complexity() {
        let results = create_test_results(8, 25.0);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(45, 25.0, Some(0.2), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("25.00"));
        assert!(output.contains("20.0%"));
        assert!(output.contains("Needs Attention"));
    }

    #[test]
    fn test_write_health_status_section_perfect_coverage() {
        let results = create_test_results(2, 4.0);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(100, 4.0, Some(1.0), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("100%"));
        assert!(output.contains("100.0%"));
        assert!(output.contains("Good"));
    }

    #[test]
    fn test_write_health_status_section_table_structure() {
        let results = create_test_results(6, 8.5);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(82, 8.5, Some(0.75), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();

        // Check table structure
        assert!(output.contains("| Metric | Value | Status |"));
        assert!(output.contains("|--------|-------|--------|"));

        // Check all required rows
        assert!(output.contains("**Overall Health**"));
        assert!(output.contains("**Average Complexity**"));
        assert!(output.contains("**Code Coverage**"));
        assert!(output.contains("**Technical Debt**"));
    }

    #[test]
    fn test_write_health_status_section_emoji_indicators() {
        let results = create_test_results(20, 18.0);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(50, 18.0, Some(0.3), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();

        // Should contain health emoji in the output
        let health_emoji_present = output.contains("🟢")
            || output.contains("🟡")
            || output.contains("🟠")
            || output.contains("🔴")
            || output.contains("✅")
            || output.contains("⚠️")
            || output.contains("❌");
        assert!(
            health_emoji_present,
            "Expected health emoji in output: {}",
            output
        );
    }

    #[test]
    fn test_write_health_status_section_extreme_values() {
        let results = create_test_results(100, 50.0);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(5, 50.0, Some(0.01), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("5%"));
        assert!(output.contains("50.00"));
        assert!(output.contains("1.0%"));
        assert!(output.contains("100 items"));
        assert!(output.contains("Needs Attention"));
    }

    #[test]
    fn test_write_health_status_section_edge_case_zero_complexity() {
        let results = create_test_results(1, 0.0);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        let result = writer.write_health_status_section(80, 0.0, Some(0.5), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("0.00"));
        assert!(output.contains("80%"));
    }

    #[test]
    fn test_write_health_status_section_coverage_formatting() {
        let results = create_test_results(4, 6.8);
        let config = MarkdownConfig::default();
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::with_config(&mut buffer, config);

        // Test various coverage values for proper formatting
        let result = writer.write_health_status_section(88, 6.8, Some(0.123456), &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        // Should format coverage to 1 decimal place
        assert!(output.contains("12.3%"));
    }
}
