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

impl<W: Write> EnhancedMarkdownWriter<W> {
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
        // Coverage not directly available in results - would need to be passed separately
        let coverage_percentage: Option<f64> = None;
        let health_score = calculate_health_score(results, coverage_percentage.map(|r| r * 100.0));
        let avg_complexity = calculate_average_complexity(results);

        // Health Status Card
        if self.config.collapsible_sections {
            writeln!(self.writer, "<details open>")?;
            writeln!(
                self.writer,
                "<summary><strong>📊 Health Status</strong></summary>\n"
            )?;
        } else {
            writeln!(self.writer, "### 📊 Health Status\n")?;
        }

        writeln!(self.writer, "| Metric | Value | Status |")?;
        writeln!(self.writer, "|--------|-------|--------|")?;
        writeln!(
            self.writer,
            "| **Overall Health** | {}% {} | {} |",
            health_score,
            get_health_emoji(health_score),
            if health_score >= 70 {
                "Good"
            } else {
                "Needs Attention"
            }
        )?;
        writeln!(
            self.writer,
            "| **Average Complexity** | {:.2} | {} |",
            avg_complexity,
            get_complexity_status(avg_complexity)
        )?;

        if let Some(coverage) = coverage_percentage {
            writeln!(
                self.writer,
                "| **Code Coverage** | {:.1}% | {} |",
                coverage * 100.0,
                get_coverage_status(coverage * 100.0)
            )?;
        }

        writeln!(
            self.writer,
            "| **Technical Debt** | {} items | {} |",
            results.technical_debt.items.len(),
            get_debt_status(results.technical_debt.items.len())
        )?;

        if self.config.collapsible_sections {
            writeln!(self.writer, "\n</details>\n")?;
        }

        // Key Insights
        writeln!(self.writer, "### 🔍 Key Insights\n")?;

        if let Some(analysis) = unified_analysis {
            let critical_items = analysis.items.iter()
                .filter(|i| matches!(i.debt_type, DebtType::ComplexityHotspot { cyclomatic, .. } if cyclomatic > 20))
                .count();

            if critical_items > 0 {
                writeln!(
                    self.writer,
                    "- ⚠️ **{}** critical complexity issues require immediate attention",
                    critical_items
                )?;
            }
        }

        if avg_complexity > 10.0 {
            writeln!(
                self.writer,
                "- 📈 Average complexity ({:.1}) exceeds recommended threshold",
                avg_complexity
            )?;
        }

        if let Some(coverage) = coverage_percentage {
            if coverage < 0.5 {
                writeln!(
                    self.writer,
                    "- 🔴 Code coverage ({:.1}%) is below minimum recommended level",
                    coverage * 100.0
                )?;
            }
        }

        writeln!(self.writer)?;
        Ok(())
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

        // Priority Actions
        writeln!(self.writer, "### 🚨 Priority Actions\n")?;

        if let Some(analysis) = unified_analysis {
            for (i, item) in analysis.items.iter().take(3).enumerate() {
                writeln!(
                    self.writer,
                    "{}. **{}**",
                    i + 1,
                    item.recommendation.primary_action
                )?;
                writeln!(
                    self.writer,
                    "   - Location: `{}`",
                    item.location.file.display()
                )?;
                writeln!(
                    self.writer,
                    "   - Estimated Effort: {} hours",
                    estimate_effort(item)
                )?;
                writeln!(self.writer)?;
            }
        }

        // Strategic Recommendations
        writeln!(self.writer, "### 📋 Strategic Recommendations\n")?;

        let avg_complexity = calculate_average_complexity(results);
        if avg_complexity > 10.0 {
            writeln!(self.writer, "1. **Reduce Complexity**: Implement code review process focusing on cyclomatic complexity")?;
        }

        // Coverage check removed as it's not in results structure
        if false {
            // if coverage < 0.6 {
            writeln!(
                self.writer,
                "2. **Improve Test Coverage**: Set up coverage gates in CI/CD pipeline"
            )?;
            // }
        }

        if results.technical_debt.items.len() > 50 {
            writeln!(
                self.writer,
                "3. **Debt Reduction Sprint**: Allocate 20% of sprint capacity to debt reduction"
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
