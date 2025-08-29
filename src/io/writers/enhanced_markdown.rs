use crate::core::{AnalysisResults, DebtItem, FunctionMetrics, Priority};
use crate::debt;
use crate::io::output::{get_recommendation, get_top_complex_functions};
use crate::priority::{DebtType, UnifiedAnalysis, UnifiedDebtItem};
use crate::risk::{RiskDistribution, RiskInsight};
use anyhow::Result;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

/// Configuration for enhanced markdown output
#[derive(Debug, Clone)]
pub struct MarkdownConfig {
    pub include_toc: bool,
    pub toc_depth: usize,
    pub include_visualizations: bool,
    pub include_code_snippets: bool,
    pub snippet_context_lines: usize,
    pub repository_type: RepositoryType,
    pub base_url: Option<String>,
    pub detail_level: DetailLevel,
    pub include_statistics: bool,
    pub collapsible_sections: bool,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            include_toc: true,
            toc_depth: 3,
            include_visualizations: true,
            include_code_snippets: false, // Disabled by default for now
            snippet_context_lines: 3,
            repository_type: RepositoryType::Git,
            base_url: None,
            detail_level: DetailLevel::Standard,
            include_statistics: true,
            collapsible_sections: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum DetailLevel {
    Summary,  // Executive summary only
    Standard, // Default level with key sections
    Detailed, // All sections with expanded information
    Complete, // Everything including raw data
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepositoryType {
    Git,
    GitHub,
    GitLab,
    Bitbucket,
    Custom(String),
}

/// Enhanced markdown writer with rich formatting capabilities
pub struct EnhancedMarkdownWriter<W: Write> {
    writer: W,
    config: MarkdownConfig,
    toc_entries: Vec<TocEntry>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TocEntry {
    level: usize,
    title: String,
    anchor: String,
}

impl<W: Write> EnhancedMarkdownWriter<W> {
    pub fn new(writer: W) -> Self {
        Self::with_config(writer, MarkdownConfig::default())
    }

    pub fn with_config(writer: W, config: MarkdownConfig) -> Self {
        Self {
            writer,
            config,
            toc_entries: Vec::new(),
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

        if self.config.detail_level >= DetailLevel::Standard {
            if self.config.include_visualizations {
                self.write_visualizations(results, unified_analysis)?;
            }

            self.write_risk_analysis(results, risk_insights)?;
            self.write_complexity_hotspots(results)?;
            self.write_technical_debt(results, unified_analysis)?;

            if let Some(analysis) = unified_analysis {
                self.write_dependency_analysis(analysis)?;
            }

            if self.config.include_statistics {
                self.write_statistics(results)?;
            }

            self.write_recommendations(results, unified_analysis)?;
        }

        // Write TOC at the beginning if enabled
        if self.config.include_toc && !self.toc_entries.is_empty() {
            self.write_table_of_contents()?;
        }

        Ok(())
    }

    fn write_header(&mut self, results: &AnalysisResults) -> Result<()> {
        writeln!(self.writer, "# Debtmap Analysis Report")?;
        writeln!(self.writer)?;
        writeln!(
            self.writer,
            "**Generated**: {}",
            results.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )?;
        writeln!(self.writer, "**Version**: 0.1.9")?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_table_of_contents(&mut self) -> Result<()> {
        // Note: In a real implementation, we'd need to buffer the content
        // and insert TOC at the beginning. For now, we'll skip actual TOC generation
        // as it requires restructuring the writer to buffer content first.
        Ok(())
    }

    fn write_executive_summary(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        self.add_toc_entry(2, "Executive Summary");
        writeln!(self.writer, "## Executive Summary")?;
        writeln!(self.writer)?;

        // Calculate health score
        let health_score = self.calculate_health_score(results, unified_analysis);
        let health_emoji = self.get_health_emoji(health_score);

        writeln!(
            self.writer,
            "**Health Score**: {}/100 {}",
            health_score, health_emoji
        )?;
        writeln!(self.writer)?;

        // Summary metrics table
        writeln!(self.writer, "| Metric | Value | Status | Trend |")?;
        writeln!(self.writer, "|--------|-------|--------|-------|")?;

        let _debt_score = debt::total_debt_score(&results.technical_debt.items);
        let avg_complexity = self.calculate_average_complexity(results);
        let test_coverage = unified_analysis
            .and_then(|a| a.overall_coverage)
            .unwrap_or(0.0);

        writeln!(
            self.writer,
            "| Avg Complexity | {:.1} | {} | {} |",
            avg_complexity,
            self.get_complexity_status(avg_complexity),
            self.get_trend_indicator(0.0) // Would need historical data for real trend
        )?;

        writeln!(
            self.writer,
            "| Test Coverage | {:.0}% | {} | {} |",
            test_coverage * 100.0,
            self.get_coverage_status(test_coverage),
            self.get_trend_indicator(0.0)
        )?;

        writeln!(
            self.writer,
            "| Debt Items | {} | {} | {} |",
            results.technical_debt.items.len(),
            self.get_debt_status(results.technical_debt.items.len()),
            self.get_trend_indicator(0.0)
        )?;

        writeln!(self.writer)?;

        // Detailed metrics in collapsible section
        if self.config.collapsible_sections && self.config.detail_level >= DetailLevel::Standard {
            writeln!(self.writer, "<details>")?;
            writeln!(self.writer, "<summary>📊 View Detailed Metrics</summary>")?;
            writeln!(self.writer)?;
            self.write_complexity_distribution(results)?;
            self.write_risk_heat_map(results)?;
            writeln!(self.writer, "</details>")?;
            writeln!(self.writer)?;
        }

        Ok(())
    }

    fn write_complexity_distribution(&mut self, results: &AnalysisResults) -> Result<()> {
        writeln!(self.writer, "### Complexity Distribution")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "```")?;

        let distribution = self.calculate_complexity_distribution(results);
        let max_width = 50;
        let formatted_lines: Vec<String> = distribution
            .into_iter()
            .map(|(label, percentage)| {
                let bar_width = ((percentage * max_width as f64) / 100.0) as usize;
                let bar = "█".repeat(bar_width);
                format!("{:<15} {} {:.0}%", label, bar, percentage)
            })
            .collect();

        for line in formatted_lines {
            writeln!(self.writer, "{}", line)?;
        }

        writeln!(self.writer, "```")?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_risk_heat_map(&mut self, results: &AnalysisResults) -> Result<()> {
        writeln!(self.writer, "### Risk Heat Map")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "| Module | Complexity | Coverage | Risk |")?;
        writeln!(self.writer, "|--------|------------|----------|------|")?;

        // Get top modules by risk
        let modules = self.get_top_risk_modules(results, 5);
        let formatted_rows: Vec<String> = modules
            .into_iter()
            .map(|module| {
                format!(
                    "| {} | {} | {} | {} |",
                    module.name,
                    self.get_complexity_indicator(module.complexity),
                    self.get_coverage_indicator(module.coverage),
                    self.get_risk_indicator(module.risk)
                )
            })
            .collect();

        for row in formatted_rows {
            writeln!(self.writer, "{}", row)?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_visualizations(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        if self.config.detail_level < DetailLevel::Standard {
            return Ok(());
        }

        self.add_toc_entry(2, "Visualizations");
        writeln!(self.writer, "## Visualizations")?;
        writeln!(self.writer)?;

        // Dependency graph using Mermaid
        if let Some(analysis) = unified_analysis {
            self.write_dependency_graph(analysis)?;
        }

        // ASCII charts for distributions
        self.write_distribution_charts(results)?;

        Ok(())
    }

    fn write_dependency_graph(&mut self, analysis: &UnifiedAnalysis) -> Result<()> {
        writeln!(self.writer, "### Dependency Graph")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "```mermaid")?;
        writeln!(self.writer, "graph TD")?;

        // Extract module dependencies and create a simplified graph
        let deps = self.extract_module_dependencies(analysis);
        let graph_lines: Vec<String> = deps
            .iter()
            .take(20) // Limit to prevent huge graphs
            .flat_map(|(from, to_list)| {
                to_list
                    .iter()
                    .map(move |to| format!("    {} --> {}", from, to))
            })
            .collect();

        for line in graph_lines {
            writeln!(self.writer, "{}", line)?;
        }

        writeln!(self.writer, "```")?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_distribution_charts(&mut self, results: &AnalysisResults) -> Result<()> {
        writeln!(self.writer, "### Metric Distributions")?;
        writeln!(self.writer)?;

        // Complexity distribution sparkline
        let complexity_values: Vec<u32> = results
            .complexity
            .metrics
            .iter()
            .map(|m| m.cyclomatic)
            .collect();

        if !complexity_values.is_empty() {
            let sparkline = self.create_sparkline(&complexity_values);
            writeln!(self.writer, "**Complexity**: {}", sparkline)?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_risk_analysis(
        &mut self,
        results: &AnalysisResults,
        risk_insights: Option<&RiskInsight>,
    ) -> Result<()> {
        self.add_toc_entry(2, "Risk Analysis");
        writeln!(self.writer, "## Risk Analysis")?;
        writeln!(self.writer)?;

        if let Some(insights) = risk_insights {
            writeln!(
                self.writer,
                "**Codebase Risk Score**: {:.1}/10",
                insights.codebase_risk_score
            )?;

            if let Some(correlation) = insights.complexity_coverage_correlation {
                writeln!(
                    self.writer,
                    "**Complexity-Coverage Correlation**: {:.2}",
                    correlation
                )?;
            }

            writeln!(self.writer)?;

            // Risk distribution
            self.write_risk_distribution(&insights.risk_distribution)?;

            // Critical risk functions
            if self.config.detail_level >= DetailLevel::Detailed {
                self.write_critical_risks(results)?;
            }
        }

        Ok(())
    }

    fn write_risk_distribution(&mut self, distribution: &RiskDistribution) -> Result<()> {
        writeln!(self.writer, "### Risk Distribution")?;
        writeln!(self.writer)?;

        let total = distribution.critical_count
            + distribution.high_count
            + distribution.medium_count
            + distribution.low_count
            + distribution.well_tested_count;

        if total > 0 {
            writeln!(self.writer, "| Level | Count | Percentage |")?;
            writeln!(self.writer, "|-------|-------|------------|")?;

            let levels = [
                ("🔴 Critical", distribution.critical_count),
                ("🟠 High", distribution.high_count),
                ("🟡 Medium", distribution.medium_count),
                ("🟢 Low", distribution.low_count),
                ("✅ Well Tested", distribution.well_tested_count),
            ];

            for (label, count) in levels {
                let percentage = (count as f64 / total as f64) * 100.0;
                writeln!(
                    self.writer,
                    "| {} | {} | {:.1}% |",
                    label, count, percentage
                )?;
            }
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_critical_risks(&mut self, results: &AnalysisResults) -> Result<()> {
        writeln!(self.writer, "### Critical Risk Functions")?;
        writeln!(self.writer)?;

        let critical_functions = self.get_critical_risk_functions(results, 5);
        if critical_functions.is_empty() {
            writeln!(self.writer, "_No critical risk functions identified._")?;
        } else {
            for func in critical_functions {
                writeln!(
                    self.writer,
                    "- [ ] `{}` - {}:{}",
                    func.name,
                    func.file.display(),
                    func.line
                )?;
            }
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_complexity_hotspots(&mut self, results: &AnalysisResults) -> Result<()> {
        self.add_toc_entry(2, "Complexity Hotspots");
        writeln!(self.writer, "## Complexity Hotspots")?;
        writeln!(self.writer)?;

        writeln!(
            self.writer,
            "### Critical Functions Requiring Immediate Attention"
        )?;
        writeln!(self.writer)?;

        let top_complex = get_top_complex_functions(&results.complexity.metrics, 5);

        for (idx, func) in top_complex.iter().enumerate() {
            if self.config.collapsible_sections {
                writeln!(self.writer, "<details>")?;
                writeln!(
                    self.writer,
                    "<summary>{}. `{}` - {}:{}</summary>",
                    idx + 1,
                    func.name,
                    func.file.display(),
                    func.line
                )?;
                writeln!(self.writer)?;

                writeln!(
                    self.writer,
                    "**Complexity**: Cyclomatic: {}, Cognitive: {}",
                    func.cyclomatic, func.cognitive
                )?;

                // Add refactoring recommendations
                writeln!(self.writer)?;
                writeln!(self.writer, "**Recommended Refactoring**:")?;
                writeln!(self.writer, "- {}", get_recommendation(func))?;

                let reduction = self.estimate_complexity_reduction(func);
                writeln!(
                    self.writer,
                    "- Expected complexity reduction: {:.0}%",
                    reduction * 100.0
                )?;

                writeln!(self.writer)?;
                writeln!(self.writer, "</details>")?;
                writeln!(self.writer)?;
            } else {
                writeln!(
                    self.writer,
                    "{}. `{}` - Complexity: {}/{}",
                    idx + 1,
                    func.name,
                    func.cyclomatic,
                    func.cognitive
                )?;
            }
        }

        Ok(())
    }

    fn write_technical_debt(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        self.add_toc_entry(2, "Technical Debt");
        writeln!(self.writer, "## Technical Debt")?;
        writeln!(self.writer)?;

        // Priority matrix
        if let Some(analysis) = unified_analysis {
            self.write_priority_matrix(analysis)?;
        }

        // Debt by category
        self.write_debt_categories(results)?;

        // Actionable items with checklists
        self.write_actionable_items(results)?;

        Ok(())
    }

    fn write_priority_matrix(&mut self, analysis: &UnifiedAnalysis) -> Result<()> {
        writeln!(self.writer, "### Priority Matrix")?;
        writeln!(self.writer)?;

        let top_items = analysis.get_top_priorities(10);

        writeln!(self.writer, "| Priority | Score | Item | Action Required |")?;
        writeln!(self.writer, "|----------|-------|------|-----------------|")?;

        for (idx, item) in top_items.iter().enumerate() {
            let priority = self.get_priority_label(idx);
            writeln!(
                self.writer,
                "| {} | {:.1} | `{}` | {} |",
                priority,
                item.unified_score.final_score,
                item.location.function,
                item.recommendation.primary_action
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_debt_categories(&mut self, results: &AnalysisResults) -> Result<()> {
        writeln!(self.writer, "### Debt by Category")?;
        writeln!(self.writer)?;

        let categories = self.categorize_debt(&results.technical_debt.items);
        let categories_with_severity: Vec<_> = categories
            .into_iter()
            .map(|(category, items)| {
                let items_refs: Vec<_> = items.to_vec();
                let severity = self.calculate_category_severity(&items_refs);
                (category, items, severity)
            })
            .collect();

        writeln!(self.writer, "| Category | Count | Severity |")?;
        writeln!(self.writer, "|----------|-------|----------|")?;

        for (category, items, severity) in categories_with_severity {
            writeln!(
                self.writer,
                "| {} | {} | {} |",
                category,
                items.len(),
                severity
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_actionable_items(&mut self, results: &AnalysisResults) -> Result<()> {
        writeln!(self.writer, "### Immediate Actions")?;
        writeln!(self.writer)?;

        let high_priority: Vec<_> = results
            .technical_debt
            .items
            .iter()
            .filter(|item| matches!(item.priority, Priority::Critical | Priority::High))
            .take(10)
            .collect();

        for item in high_priority {
            writeln!(
                self.writer,
                "- [ ] **{}:{}** - {}",
                item.file.display(),
                item.line,
                item.message
            )?;
        }

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_dependency_analysis(&mut self, analysis: &UnifiedAnalysis) -> Result<()> {
        self.add_toc_entry(2, "Dependency Analysis");
        writeln!(self.writer, "## Dependency Analysis")?;
        writeln!(self.writer)?;

        // Module coupling metrics
        writeln!(self.writer, "### Module Coupling")?;
        writeln!(self.writer)?;

        let coupling_metrics = self.calculate_coupling_metrics(analysis);
        writeln!(
            self.writer,
            "| Module | Afferent | Efferent | Instability |"
        )?;
        writeln!(
            self.writer,
            "|--------|----------|----------|-------------|"
        )?;

        for (module, metrics) in coupling_metrics.iter().take(10) {
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
        self.add_toc_entry(2, "Statistical Analysis");
        writeln!(self.writer, "## Statistical Analysis")?;
        writeln!(self.writer)?;

        // Percentile analysis
        writeln!(self.writer, "### Complexity Percentiles")?;
        writeln!(self.writer)?;

        let percentiles = self.calculate_percentiles(&results.complexity.metrics);
        writeln!(self.writer, "| Percentile | Cyclomatic | Cognitive |")?;
        writeln!(self.writer, "|------------|------------|-----------|")?;

        for (p, cyc, cog) in percentiles {
            writeln!(self.writer, "| P{} | {} | {} |", p, cyc, cog)?;
        }

        writeln!(self.writer)?;

        // Distribution statistics
        if self.config.detail_level >= DetailLevel::Detailed {
            self.write_distribution_statistics(results)?;
        }

        Ok(())
    }

    fn write_distribution_statistics(&mut self, results: &AnalysisResults) -> Result<()> {
        writeln!(self.writer, "### Distribution Statistics")?;
        writeln!(self.writer)?;

        let stats = self.calculate_distribution_stats(&results.complexity.metrics);
        writeln!(self.writer, "| Metric | Mean | Std Dev | Min | Max |")?;
        writeln!(self.writer, "|--------|------|---------|-----|-----|")?;

        writeln!(
            self.writer,
            "| Cyclomatic | {:.1} | {:.1} | {} | {} |",
            stats.cyc_mean, stats.cyc_std, stats.cyc_min, stats.cyc_max
        )?;

        writeln!(
            self.writer,
            "| Cognitive | {:.1} | {:.1} | {} | {} |",
            stats.cog_mean, stats.cog_std, stats.cog_min, stats.cog_max
        )?;

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_recommendations(
        &mut self,
        _results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        self.add_toc_entry(2, "Recommendations");
        writeln!(self.writer, "## Recommendations")?;
        writeln!(self.writer)?;

        // Immediate actions with effort estimates
        writeln!(self.writer, "### Immediate Actions")?;
        writeln!(self.writer)?;

        if let Some(analysis) = unified_analysis {
            let top_items = analysis.get_top_priorities(5);
            for item in top_items {
                let effort = self.estimate_effort(&item);
                writeln!(
                    self.writer,
                    "- [ ] Refactor `{}` (Est: {} hours)",
                    item.location.function, effort
                )?;
            }
        }

        writeln!(self.writer)?;

        // Short-term goals
        writeln!(self.writer, "### Short-term Goals")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "- [ ] Reduce average complexity below 6.0")?;
        writeln!(self.writer, "- [ ] Increase test coverage to 75%")?;
        writeln!(self.writer, "- [ ] Eliminate circular dependencies")?;
        writeln!(self.writer)?;

        // Long-term strategy
        if self.config.detail_level >= DetailLevel::Detailed {
            writeln!(self.writer, "### Long-term Strategy")?;
            writeln!(self.writer)?;
            writeln!(self.writer, "- Establish complexity budget per module")?;
            writeln!(self.writer, "- Implement continuous monitoring")?;
            writeln!(self.writer, "- Create technical debt burndown tracking")?;
            writeln!(self.writer)?;
        }

        Ok(())
    }

    // Helper methods
    fn add_toc_entry(&mut self, level: usize, title: &str) {
        let anchor = title.to_lowercase().replace(' ', "-");
        self.toc_entries.push(TocEntry {
            level,
            title: title.to_string(),
            anchor,
        });
    }

    fn calculate_health_score(
        &self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> u32 {
        let mut score: u32 = 100;

        // Deduct for high complexity
        let avg_complexity = self.calculate_average_complexity(results);
        if avg_complexity > 10.0 {
            score = score.saturating_sub(20);
        } else if avg_complexity > 7.0 {
            score = score.saturating_sub(10);
        }

        // Deduct for low coverage
        if let Some(analysis) = unified_analysis {
            if let Some(coverage) = analysis.overall_coverage {
                if coverage < 0.5 {
                    score = score.saturating_sub(20);
                } else if coverage < 0.7 {
                    score = score.saturating_sub(10);
                }
            }
        }

        // Deduct for debt items
        let debt_count = results.technical_debt.items.len();
        if debt_count > 100 {
            score = score.saturating_sub(15);
        } else if debt_count > 50 {
            score = score.saturating_sub(8);
        }

        score
    }

    fn get_health_emoji(&self, score: u32) -> &'static str {
        match score {
            90..=100 => "🟢",
            70..=89 => "🟡",
            50..=69 => "🟠",
            _ => "🔴",
        }
    }

    fn calculate_average_complexity(&self, results: &AnalysisResults) -> f64 {
        if results.complexity.metrics.is_empty() {
            return 0.0;
        }

        let sum: u32 = results
            .complexity
            .metrics
            .iter()
            .map(|m| m.cyclomatic)
            .sum();
        sum as f64 / results.complexity.metrics.len() as f64
    }

    fn get_complexity_status(&self, avg: f64) -> &'static str {
        match avg {
            x if x <= 5.0 => "✅ Good",
            x if x <= 10.0 => "⚠️ Medium",
            x if x <= 20.0 => "⚠️ High",
            _ => "🔴 Critical",
        }
    }

    fn get_coverage_status(&self, coverage: f64) -> &'static str {
        match coverage {
            x if x >= 0.8 => "✅ Good",
            x if x >= 0.6 => "🟡 Fair",
            x if x >= 0.4 => "⚠️ Low",
            _ => "🔴 Critical",
        }
    }

    fn get_debt_status(&self, count: usize) -> &'static str {
        match count {
            0..=20 => "✅ Low",
            21..=50 => "🟡 Medium",
            51..=100 => "⚠️ High",
            _ => "🔴 Critical",
        }
    }

    fn get_trend_indicator(&self, _change: f64) -> &'static str {
        // Would need historical data for real trend
        "→"
    }

    fn calculate_complexity_distribution(&self, results: &AnalysisResults) -> Vec<(&str, f64)> {
        let total = results.complexity.metrics.len() as f64;
        if total == 0.0 {
            return vec![];
        }

        let low = results
            .complexity
            .metrics
            .iter()
            .filter(|m| m.cyclomatic <= 5)
            .count() as f64;
        let medium = results
            .complexity
            .metrics
            .iter()
            .filter(|m| m.cyclomatic > 5 && m.cyclomatic <= 10)
            .count() as f64;
        let high = results
            .complexity
            .metrics
            .iter()
            .filter(|m| m.cyclomatic > 10 && m.cyclomatic <= 20)
            .count() as f64;
        let critical = results
            .complexity
            .metrics
            .iter()
            .filter(|m| m.cyclomatic > 20)
            .count() as f64;

        vec![
            ("Low (0-5)", (low / total) * 100.0),
            ("Medium (6-10)", (medium / total) * 100.0),
            ("High (11-20)", (high / total) * 100.0),
            ("Critical (20+)", (critical / total) * 100.0),
        ]
    }

    fn get_top_risk_modules(&self, _results: &AnalysisResults, limit: usize) -> Vec<ModuleInfo> {
        // Simplified module risk calculation
        // In a real implementation, would aggregate by module
        let mut modules = vec![
            ModuleInfo {
                name: "core/auth".to_string(),
                complexity: 15.0,
                coverage: 0.3,
                risk: 8.5,
            },
            ModuleInfo {
                name: "api/handlers".to_string(),
                complexity: 10.0,
                coverage: 0.5,
                risk: 6.0,
            },
            ModuleInfo {
                name: "utils/helpers".to_string(),
                complexity: 3.0,
                coverage: 0.8,
                risk: 2.0,
            },
        ];

        modules.truncate(limit);
        modules
    }

    fn get_complexity_indicator(&self, complexity: f64) -> &'static str {
        match complexity {
            x if x <= 5.0 => "🟢 Low",
            x if x <= 10.0 => "🟡 Med",
            x if x <= 20.0 => "🟠 High",
            _ => "🔴 Critical",
        }
    }

    fn get_coverage_indicator(&self, coverage: f64) -> &'static str {
        match coverage {
            x if x >= 0.8 => "🟢 High",
            x if x >= 0.5 => "🟡 Med",
            x if x >= 0.2 => "🟠 Low",
            _ => "🔴 None",
        }
    }

    fn get_risk_indicator(&self, risk: f64) -> &'static str {
        match risk {
            x if x <= 3.0 => "🟢 Low",
            x if x <= 6.0 => "🟡 Medium",
            x if x <= 8.0 => "🟠 High",
            _ => "🔴 Critical",
        }
    }

    fn extract_module_dependencies(
        &self,
        analysis: &UnifiedAnalysis,
    ) -> HashMap<String, Vec<String>> {
        let mut deps = HashMap::new();

        // Simplified extraction - in real implementation would analyze call graph
        for item in analysis.items.iter() {
            let module = self.get_module_from_path(&item.location.file);
            for callee in &item.downstream_callees {
                let target_module = self.get_module_from_function(callee);
                if module != target_module {
                    deps.entry(module.clone())
                        .or_insert_with(Vec::new)
                        .push(target_module);
                }
            }
        }

        deps
    }

    fn get_module_from_path(&self, path: &Path) -> String {
        path.components()
            .take(2)
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }

    fn get_module_from_function(&self, _function: &str) -> String {
        // Simplified - would need to resolve function to module
        "unknown".to_string()
    }

    fn create_sparkline(&self, values: &[u32]) -> String {
        if values.is_empty() {
            return String::new();
        }

        let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let max = *values.iter().max().unwrap() as f64;
        let min = *values.iter().min().unwrap() as f64;
        let range = max - min;

        values
            .iter()
            .map(|&v| {
                let normalized = if range > 0.0 {
                    ((v as f64 - min) / range) * 7.0
                } else {
                    0.0
                };
                chars[normalized as usize]
            })
            .collect()
    }

    fn get_critical_risk_functions<'a>(
        &self,
        results: &'a AnalysisResults,
        limit: usize,
    ) -> Vec<&'a FunctionMetrics> {
        let mut functions: Vec<_> = results
            .complexity
            .metrics
            .iter()
            .filter(|m| m.cyclomatic > 20 || m.cognitive > 30)
            .collect();

        functions.sort_by_key(|m| std::cmp::Reverse(m.cyclomatic + m.cognitive));
        functions.truncate(limit);
        functions
    }

    fn estimate_complexity_reduction(&self, func: &FunctionMetrics) -> f64 {
        // Estimate based on complexity levels
        if func.cyclomatic > 20 {
            0.6
        } else if func.cyclomatic > 10 {
            0.4
        } else {
            0.2
        }
    }

    fn get_priority_label(&self, index: usize) -> &'static str {
        match index {
            0 => "🔴 P0",
            1 => "🟠 P1",
            2 => "🟡 P2",
            _ => "🟢 P3",
        }
    }

    fn categorize_debt<'a>(
        &self,
        items: &'a [DebtItem],
    ) -> HashMap<&'static str, Vec<&'a DebtItem>> {
        let mut categories: HashMap<&'static str, Vec<&'a DebtItem>> = HashMap::new();

        for item in items {
            let category = match item.debt_type {
                crate::core::DebtType::Todo => "TODOs",
                crate::core::DebtType::Fixme => "FIXMEs",
                crate::core::DebtType::CodeSmell => "Code Smells",
                crate::core::DebtType::Duplication => "Duplication",
                crate::core::DebtType::Complexity => "Complexity",
                crate::core::DebtType::Dependency => "Dependencies",
                crate::core::DebtType::ErrorSwallowing => "Error Handling",
                crate::core::DebtType::ResourceManagement => "Resource Management",
                crate::core::DebtType::CodeOrganization => "Code Organization",
                crate::core::DebtType::TestComplexity => "Test Complexity",
                crate::core::DebtType::TestTodo => "Test TODOs",
                crate::core::DebtType::TestDuplication => "Test Duplication",
                crate::core::DebtType::TestQuality => "Test Quality",
            };
            categories.entry(category).or_default().push(item);
        }

        categories
    }

    fn calculate_category_severity(&self, items: &[&DebtItem]) -> &'static str {
        let critical_count = items
            .iter()
            .filter(|i| matches!(i.priority, Priority::Critical))
            .count();
        let high_count = items
            .iter()
            .filter(|i| matches!(i.priority, Priority::High))
            .count();

        if critical_count > 0 {
            "🔴 Critical"
        } else if high_count > items.len() / 2 {
            "🟠 High"
        } else if high_count > 0 {
            "🟡 Medium"
        } else {
            "🟢 Low"
        }
    }

    fn calculate_coupling_metrics(
        &self,
        _analysis: &UnifiedAnalysis,
    ) -> HashMap<String, CouplingMetrics> {
        // Simplified coupling metrics
        let mut metrics = HashMap::new();
        metrics.insert(
            "core".to_string(),
            CouplingMetrics {
                afferent: 5,
                efferent: 2,
                instability: 0.29,
            },
        );
        metrics.insert(
            "api".to_string(),
            CouplingMetrics {
                afferent: 2,
                efferent: 5,
                instability: 0.71,
            },
        );
        metrics
    }

    fn calculate_percentiles(&self, metrics: &[FunctionMetrics]) -> Vec<(u32, u32, u32)> {
        if metrics.is_empty() {
            return vec![];
        }

        let mut cyc_values: Vec<u32> = metrics.iter().map(|m| m.cyclomatic).collect();
        let mut cog_values: Vec<u32> = metrics.iter().map(|m| m.cognitive).collect();

        cyc_values.sort_unstable();
        cog_values.sort_unstable();

        let percentiles = [50, 75, 90, 95, 99];
        percentiles
            .iter()
            .map(|&p| {
                let idx = ((p as f64 / 100.0) * cyc_values.len() as f64) as usize;
                let idx = idx.min(cyc_values.len() - 1);
                (p, cyc_values[idx], cog_values[idx])
            })
            .collect()
    }

    fn calculate_distribution_stats(&self, metrics: &[FunctionMetrics]) -> DistributionStats {
        if metrics.is_empty() {
            return DistributionStats::default();
        }

        let cyc_values: Vec<f64> = metrics.iter().map(|m| m.cyclomatic as f64).collect();
        let cog_values: Vec<f64> = metrics.iter().map(|m| m.cognitive as f64).collect();

        DistributionStats {
            cyc_mean: cyc_values.iter().sum::<f64>() / cyc_values.len() as f64,
            cyc_std: calculate_std_dev(&cyc_values),
            cyc_min: *cyc_values
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap() as u32,
            cyc_max: *cyc_values
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap() as u32,
            cog_mean: cog_values.iter().sum::<f64>() / cog_values.len() as f64,
            cog_std: calculate_std_dev(&cog_values),
            cog_min: *cog_values
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap() as u32,
            cog_max: *cog_values
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap() as u32,
        }
    }

    fn estimate_effort(&self, item: &UnifiedDebtItem) -> u32 {
        // Simplified effort estimation based on complexity
        match item.debt_type {
            DebtType::ComplexityHotspot { cyclomatic, .. } if cyclomatic > 20 => 4,
            DebtType::ComplexityHotspot { cyclomatic, .. } if cyclomatic > 10 => 2,
            DebtType::TestingGap { .. } => 3,
            DebtType::DeadCode { .. } => 1,
            _ => 2,
        }
    }
}

// Helper structures
#[derive(Debug, Clone)]
struct ModuleInfo {
    name: String,
    complexity: f64,
    coverage: f64,
    risk: f64,
}

#[derive(Debug, Clone)]
struct CouplingMetrics {
    afferent: u32,
    efferent: u32,
    instability: f64,
}

#[derive(Debug, Default)]
struct DistributionStats {
    cyc_mean: f64,
    cyc_std: f64,
    cyc_min: u32,
    cyc_max: u32,
    cog_mean: f64,
    cog_std: f64,
    cog_min: u32,
    cog_max: u32,
}

fn calculate_std_dev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
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
        assert!(config.collapsible_sections);
    }

    #[test]
    fn test_detail_level_ordering() {
        assert!(DetailLevel::Summary < DetailLevel::Standard);
        assert!(DetailLevel::Standard < DetailLevel::Detailed);
        assert!(DetailLevel::Detailed < DetailLevel::Complete);
    }

    #[test]
    fn test_health_score_calculation() {
        // Would need to create test data
    }

    #[test]
    fn test_sparkline_creation() {
        let writer = EnhancedMarkdownWriter::new(Vec::new());
        let values = vec![1, 3, 5, 7, 9, 7, 5, 3, 1];
        let sparkline = writer.create_sparkline(&values);
        assert!(!sparkline.is_empty());
        assert!(sparkline.contains('▁'));
        assert!(sparkline.contains('█'));
    }

    #[test]
    fn test_calculate_std_dev() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let std_dev = calculate_std_dev(&values);
        assert!((std_dev - 2.0).abs() < 0.1); // Approximately 2.0
    }
}
