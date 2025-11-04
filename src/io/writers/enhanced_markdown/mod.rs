pub mod complexity_analyzer;
pub mod config;
pub mod debt_writer;
pub mod executive_summary;
pub mod formatters;
pub mod health_writer;
pub mod recommendation_writer;
pub mod risk_analyzer;
pub mod risk_writer;
pub mod statistics;
pub mod toc;
pub mod visualization_writer;

pub use config::{DetailLevel, MarkdownConfig, RepositoryType};

use crate::core::AnalysisResults;
use crate::priority::UnifiedAnalysis;
use crate::risk::RiskInsight;
use anyhow::Result;
use std::io::Write;

use self::debt_writer::*;
use self::executive_summary::*;
use self::health_writer::*;
use self::recommendation_writer::*;
use self::risk_analyzer::calculate_health_score;
use self::risk_writer::*;
use self::statistics::*;
use self::toc::TocBuilder;
use self::visualization_writer::*;

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
        self.write_header(results)?;
        self.write_executive_summary(results, unified_analysis)?;
        self.write_optional_visualizations(results, unified_analysis)?;
        self.write_optional_risk_analysis(results, risk_insights)?;
        self.write_optional_technical_debt(results, unified_analysis)?;
        self.write_optional_statistics(results)?;
        self.write_recommendations(results, unified_analysis)?;
        Ok(())
    }

    /// Write visualizations section if enabled in configuration
    fn write_optional_visualizations(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        if !self.config.include_visualizations {
            return Ok(());
        }

        self.write_visualizations(results, unified_analysis)
    }

    /// Write risk analysis section if insights are available
    fn write_optional_risk_analysis(
        &mut self,
        results: &AnalysisResults,
        risk_insights: Option<&RiskInsight>,
    ) -> Result<()> {
        let Some(insights) = risk_insights else {
            return Ok(());
        };

        self.write_risk_analysis(results, insights)
    }

    /// Write technical debt section if detail level is sufficient
    fn write_optional_technical_debt(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        if self.config.detail_level < DetailLevel::Standard {
            return Ok(());
        }

        self.write_technical_debt(results, unified_analysis)?;

        let Some(analysis) = unified_analysis else {
            return Ok(());
        };

        self.write_dependency_analysis_section(analysis)
    }

    /// Write statistics section if enabled and detail level is sufficient
    fn write_optional_statistics(&mut self, results: &AnalysisResults) -> Result<()> {
        if !self.config.include_statistics {
            return Ok(());
        }

        match self.config.detail_level {
            DetailLevel::Detailed | DetailLevel::Complete => self.write_statistics_section(results),
            _ => Ok(()),
        }
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

        let coverage_percentage: Option<f64> = None;
        let health_score = calculate_health_score(results, coverage_percentage.map(|r| r * 100.0));
        let avg_complexity = calculate_average_complexity(results);

        let summary =
            generate_executive_summary(results, unified_analysis, health_score, avg_complexity);

        write_enhanced_health_dashboard(&mut self.writer, &summary.health_dashboard)?;

        if summary.quick_wins.count > 0 {
            write_quick_wins_section(&mut self.writer, &summary.quick_wins)?;
        }

        if !summary.strategic_priorities.is_empty() {
            write_strategic_priorities_section(&mut self.writer, &summary.strategic_priorities)?;
        }

        write_team_guidance_section(&mut self.writer, &summary.team_guidance)?;
        write_success_metrics_section(&mut self.writer, &summary.success_metrics)?;

        writeln!(self.writer)?;
        Ok(())
    }

    fn write_visualizations(
        &mut self,
        results: &AnalysisResults,
        unified_analysis: Option<&UnifiedAnalysis>,
    ) -> Result<()> {
        self.toc_builder.add_entry(1, "Visualizations");
        writeln!(self.writer, "## Visualizations\n")?;

        self.toc_builder.add_entry(2, "Complexity Distribution");
        write_complexity_distribution(&mut self.writer, results)?;

        self.toc_builder.add_entry(2, "Risk Heat Map");
        write_risk_heat_map(&mut self.writer, results)?;

        if let Some(analysis) = unified_analysis {
            self.toc_builder.add_entry(2, "Module Dependencies");
            write_dependency_graph(&mut self.writer, analysis)?;
        }

        self.toc_builder.add_entry(2, "Complexity Trends");
        write_distribution_charts(&mut self.writer, results)?;

        Ok(())
    }

    fn write_risk_analysis(
        &mut self,
        results: &AnalysisResults,
        insights: &RiskInsight,
    ) -> Result<()> {
        self.toc_builder.add_entry(1, "Risk Analysis");
        writeln!(self.writer, "## [WARNING] Risk Analysis\n")?;

        writeln!(self.writer, "### Risk Summary\n")?;
        writeln!(
            self.writer,
            "**Overall Risk Level**: {}\n",
            formatters::get_risk_indicator(insights.codebase_risk_score)
        )?;

        self.toc_builder.add_entry(2, "Risk Distribution");
        write_risk_distribution(&mut self.writer, &insights.risk_distribution)?;

        if self.config.detail_level >= DetailLevel::Standard {
            self.toc_builder.add_entry(2, "Critical Risk Functions");
            write_critical_risks(&mut self.writer, results)?;
        }

        if self.config.detail_level >= DetailLevel::Detailed {
            self.toc_builder.add_entry(2, "Complexity Hotspots");
            write_complexity_hotspots(&mut self.writer, results)?;
        }

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
            self.toc_builder.add_entry(2, "Priority Matrix");
            write_priority_matrix(&mut self.writer, analysis)?;
        }

        self.toc_builder.add_entry(2, "Debt Categories");
        write_debt_categories(&mut self.writer, results)?;

        self.toc_builder.add_entry(2, "Actionable Items");
        write_actionable_items(&mut self.writer, results)?;

        Ok(())
    }

    fn write_dependency_analysis_section(&mut self, analysis: &UnifiedAnalysis) -> Result<()> {
        self.toc_builder.add_entry(1, "Dependency Analysis");
        write_dependency_analysis(&mut self.writer, analysis)?;
        Ok(())
    }

    fn write_statistics_section(&mut self, results: &AnalysisResults) -> Result<()> {
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
        writeln!(self.writer, "## [TIP] Recommendations\n")?;

        write_priority_actions(&mut self.writer, unified_analysis)?;
        write_strategic_recommendations(&mut self.writer, results)?;

        writeln!(self.writer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        AnalysisResults, ComplexityReport, ComplexitySummary, DebtItem as CoreDebtItem,
        DebtType as CoreDebtType, DependencyReport, FunctionMetrics, Priority as CorePriority,
        TechnicalDebtReport,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use std::io::Cursor;
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
                purity_reason: None,
                call_dependencies: None,
                visibility: Some("pub".to_string()),
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
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
                by_type: HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
            file_contexts: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_write_enhanced_report() {
        let results = create_test_results(5, 8.0);
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = EnhancedMarkdownWriter::new(&mut buffer);

        let result = writer.write_enhanced_report(&results, None, None);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Technical Debt Analysis Report"));
        assert!(output.contains("Executive Summary"));
    }

    #[test]
    fn test_enhanced_markdown_writer_creation() {
        let buffer = Cursor::new(Vec::new());
        let writer = EnhancedMarkdownWriter::new(buffer);
        assert!(writer.config.include_toc);
    }

    #[test]
    fn test_enhanced_markdown_writer_with_config() {
        let config = MarkdownConfig {
            include_toc: false,
            toc_depth: 2,
            include_visualizations: false,
            collapsible_sections: true,
            detail_level: DetailLevel::Summary,
            include_statistics: false,
            repository_type: RepositoryType::Git,
            include_code_snippets: false,
            snippet_context_lines: 3,
            base_url: None,
        };

        let buffer = Cursor::new(Vec::new());
        let writer = EnhancedMarkdownWriter::with_config(buffer, config.clone());
        assert!(!writer.config.include_toc);
        assert_eq!(writer.config.toc_depth, 2);
        assert!(!writer.config.include_visualizations);
    }
}
