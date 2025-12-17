//! Complete report generation effects.
//!
//! This module provides the full report generation workflow, including
//! writing analysis results and optional risk insights to multiple formats.

use crate::core::AnalysisResults;
use crate::effects::{effect_from_fn, AnalysisEffect};
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use crate::io::output::OutputWriter;
use crate::io::writers::TerminalWriter;
use crate::risk::RiskInsight;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::config::{OutputConfig, OutputResult};
use super::render::{
    render_html, render_json, render_markdown, render_risk_json, render_risk_markdown,
};

// ============================================================================
// Report Configuration
// ============================================================================

/// Configuration for full analysis report generation.
///
/// This provides all options needed to generate a complete analysis report
/// including multiple output formats and risk insights.
#[derive(Debug, Clone, Default)]
pub struct ReportConfig {
    /// Output configuration for file formats.
    pub output: OutputConfig,

    /// Optional risk insights to include in the report.
    pub risk_insights: Option<RiskInsight>,

    /// Path for risk-specific markdown output.
    pub risk_markdown_path: Option<PathBuf>,

    /// Path for risk-specific JSON output.
    pub risk_json_path: Option<PathBuf>,
}

impl ReportConfig {
    /// Create a new report config builder.
    pub fn builder() -> ReportConfigBuilder {
        ReportConfigBuilder::default()
    }
}

/// Builder for ReportConfig.
#[derive(Debug, Clone, Default)]
pub struct ReportConfigBuilder {
    config: ReportConfig,
}

impl ReportConfigBuilder {
    /// Set the output configuration.
    pub fn output(mut self, output: OutputConfig) -> Self {
        self.config.output = output;
        self
    }

    /// Set markdown output path.
    pub fn markdown(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.output.markdown_path = Some(path.into());
        self
    }

    /// Set JSON output path.
    pub fn json(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.output.json_path = Some(path.into());
        self
    }

    /// Set HTML output path.
    pub fn html(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.output.html_path = Some(path.into());
        self
    }

    /// Enable terminal output.
    pub fn terminal(mut self, enabled: bool) -> Self {
        self.config.output.terminal_output = enabled;
        self
    }

    /// Set risk insights to include in the report.
    pub fn risk_insights(mut self, insights: RiskInsight) -> Self {
        self.config.risk_insights = Some(insights);
        self
    }

    /// Set risk-specific markdown output path.
    pub fn risk_markdown(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.risk_markdown_path = Some(path.into());
        self
    }

    /// Set risk-specific JSON output path.
    pub fn risk_json(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.risk_json_path = Some(path.into());
        self
    }

    /// Build the report config.
    pub fn build(self) -> ReportConfig {
        self.config
    }
}

// ============================================================================
// Report Result
// ============================================================================

/// Result of a full report generation operation.
#[derive(Debug, Clone)]
pub struct ReportResult {
    /// Results from individual output operations.
    pub outputs: Vec<OutputResult>,

    /// Total bytes written across all outputs.
    pub total_bytes: usize,

    /// Total time taken for all output operations.
    pub total_duration: Duration,
}

// ============================================================================
// Report Generation Effect
// ============================================================================

/// Write a complete analysis report as an Effect.
///
/// This effect wraps the full report generation workflow, writing analysis
/// results to all configured output formats and optionally including risk
/// insights.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::io::writers::effects::{write_analysis_report_effect, ReportConfig};
/// use debtmap::effects::run_effect;
///
/// let config = ReportConfig::builder()
///     .markdown("report.md")
///     .json("report.json")
///     .terminal(true)
///     .build();
///
/// let effect = write_analysis_report_effect(results, config);
/// let report_result = run_effect(effect, debtmap_config)?;
///
/// println!("Wrote {} bytes in {:?}",
///     report_result.total_bytes,
///     report_result.total_duration);
/// ```
pub fn write_analysis_report_effect(
    results: AnalysisResults,
    config: ReportConfig,
) -> AnalysisEffect<ReportResult> {
    effect_from_fn(move |env: &RealEnv| {
        let start = Instant::now();
        let mut outputs: Vec<OutputResult> = Vec::new();

        // Write main analysis results
        if let Some(ref md_path) = config.output.markdown_path {
            let content = render_markdown(&results)?;
            env.file_system().write(md_path, &content).map_err(|e| {
                AnalysisError::io_with_path(
                    format!("Failed to write markdown: {}", e.message()),
                    md_path,
                )
            })?;
            outputs.push(OutputResult {
                destination: md_path.display().to_string(),
                bytes_written: content.len(),
                duration: start.elapsed(),
            });
        }

        if let Some(ref json_path) = config.output.json_path {
            let content = render_json(&results)?;
            env.file_system().write(json_path, &content).map_err(|e| {
                AnalysisError::io_with_path(
                    format!("Failed to write JSON: {}", e.message()),
                    json_path,
                )
            })?;
            outputs.push(OutputResult {
                destination: json_path.display().to_string(),
                bytes_written: content.len(),
                duration: start.elapsed(),
            });
        }

        if let Some(ref html_path) = config.output.html_path {
            let content = render_html(&results)?;
            env.file_system().write(html_path, &content).map_err(|e| {
                AnalysisError::io_with_path(
                    format!("Failed to write HTML: {}", e.message()),
                    html_path,
                )
            })?;
            outputs.push(OutputResult {
                destination: html_path.display().to_string(),
                bytes_written: content.len(),
                duration: start.elapsed(),
            });
        }

        if config.output.terminal_output {
            let mut writer = TerminalWriter::default();
            writer
                .write_results(&results)
                .map_err(|e| AnalysisError::io(format!("Failed to write to terminal: {}", e)))?;
            outputs.push(OutputResult {
                destination: "terminal".to_string(),
                bytes_written: 0,
                duration: start.elapsed(),
            });
        }

        // Write risk insights if configured
        if let Some(ref insights) = config.risk_insights {
            if let Some(ref risk_md_path) = config.risk_markdown_path {
                let content = render_risk_markdown(insights)?;
                env.file_system()
                    .write(risk_md_path, &content)
                    .map_err(|e| {
                        AnalysisError::io_with_path(
                            format!("Failed to write risk markdown: {}", e.message()),
                            risk_md_path,
                        )
                    })?;
                outputs.push(OutputResult {
                    destination: risk_md_path.display().to_string(),
                    bytes_written: content.len(),
                    duration: start.elapsed(),
                });
            }

            if let Some(ref risk_json_path) = config.risk_json_path {
                let content = render_risk_json(insights)?;
                env.file_system()
                    .write(risk_json_path, &content)
                    .map_err(|e| {
                        AnalysisError::io_with_path(
                            format!("Failed to write risk JSON: {}", e.message()),
                            risk_json_path,
                        )
                    })?;
                outputs.push(OutputResult {
                    destination: risk_json_path.display().to_string(),
                    bytes_written: content.len(),
                    duration: start.elapsed(),
                });
            }
        }

        let total_bytes = outputs.iter().map(|o| o.bytes_written).sum();
        let total_duration = start.elapsed();

        Ok(ReportResult {
            outputs,
            total_bytes,
            total_duration,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::core::{
        ComplexityReport, ComplexitySummary, DebtItem, DebtType, DependencyReport, FunctionMetrics,
        Priority, TechnicalDebtReport,
    };
    use crate::effects::run_effect;
    use crate::env::AnalysisEnv;
    use crate::testkit::DebtmapTestEnv;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_results() -> AnalysisResults {
        let items = vec![DebtItem {
            id: "test-1".to_string(),
            debt_type: DebtType::Todo { reason: None },
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
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        }];

        AnalysisResults {
            project_path: PathBuf::from("/test/project"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics,
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
            file_contexts: HashMap::new(),
        }
    }

    #[test]
    fn test_report_config_builder() {
        let config = ReportConfig::builder()
            .markdown("report.md")
            .json("report.json")
            .html("report.html")
            .terminal(false)
            .risk_markdown("risk.md")
            .risk_json("risk.json")
            .build();

        assert_eq!(
            config.output.markdown_path,
            Some(PathBuf::from("report.md"))
        );
        assert_eq!(config.output.json_path, Some(PathBuf::from("report.json")));
        assert_eq!(config.output.html_path, Some(PathBuf::from("report.html")));
        assert!(!config.output.terminal_output);
        assert_eq!(config.risk_markdown_path, Some(PathBuf::from("risk.md")));
        assert_eq!(config.risk_json_path, Some(PathBuf::from("risk.json")));
    }

    #[test]
    fn test_report_config_with_output() {
        let output = OutputConfig::builder().markdown("output.md").build();

        let config = ReportConfig::builder().output(output).build();

        assert_eq!(
            config.output.markdown_path,
            Some(PathBuf::from("output.md"))
        );
    }

    #[test]
    fn test_write_analysis_report_effect_basic() {
        let temp_dir = TempDir::new().unwrap();
        let results = create_test_results();

        let config = ReportConfig::builder()
            .markdown(temp_dir.path().join("report.md"))
            .json(temp_dir.path().join("report.json"))
            .build();

        let effect = write_analysis_report_effect(results, config);
        let report_result = run_effect(effect, DebtmapConfig::default()).unwrap();

        // Verify outputs were generated
        assert_eq!(report_result.outputs.len(), 2);
        assert!(report_result.total_bytes > 0);

        // Verify files exist
        assert!(temp_dir.path().join("report.md").exists());
        assert!(temp_dir.path().join("report.json").exists());

        // Verify content
        let md_content = std::fs::read_to_string(temp_dir.path().join("report.md")).unwrap();
        assert!(md_content.contains("# Debtmap Analysis Report"));

        let json_content = std::fs::read_to_string(temp_dir.path().join("report.json")).unwrap();
        let _: serde_json::Value = serde_json::from_str(&json_content).unwrap();
    }

    #[test]
    fn test_write_analysis_report_effect_empty_config() {
        let results = create_test_results();
        let config = ReportConfig::default();

        let effect = write_analysis_report_effect(results, config);
        let report_result = run_effect(effect, DebtmapConfig::default()).unwrap();

        // No outputs when config is empty
        assert!(report_result.outputs.is_empty());
        assert_eq!(report_result.total_bytes, 0);
    }

    #[test]
    fn test_write_analysis_report_effect_all_formats() {
        let temp_dir = TempDir::new().unwrap();
        let results = create_test_results();

        let config = ReportConfig::builder()
            .markdown(temp_dir.path().join("report.md"))
            .json(temp_dir.path().join("report.json"))
            .html(temp_dir.path().join("report.html"))
            .build();

        let effect = write_analysis_report_effect(results, config);
        let report_result = run_effect(effect, DebtmapConfig::default()).unwrap();

        // Verify all three outputs
        assert_eq!(report_result.outputs.len(), 3);
        assert!(temp_dir.path().join("report.md").exists());
        assert!(temp_dir.path().join("report.json").exists());
        assert!(temp_dir.path().join("report.html").exists());
    }

    /// Verify rendered content can be round-tripped through mock file system.
    #[test]
    fn test_render_content_roundtrip_with_mock_env() {
        use super::super::render::{render_json, render_markdown};

        let results = create_test_results();
        let env = DebtmapTestEnv::new();

        // Simulate what write_analysis_report_effect does
        let md_content = render_markdown(&results).unwrap();
        let json_content = render_json(&results).unwrap();

        // Write to mock env
        env.file_system()
            .write(Path::new("output/report.md"), &md_content)
            .unwrap();
        env.file_system()
            .write(Path::new("output/report.json"), &json_content)
            .unwrap();

        // Verify files exist in mock env
        assert!(env.has_file("output/report.md"));
        assert!(env.has_file("output/report.json"));

        // Read back and verify content matches
        let read_md = env
            .file_system()
            .read_to_string(Path::new("output/report.md"))
            .unwrap();
        assert_eq!(read_md, md_content);

        let read_json = env
            .file_system()
            .read_to_string(Path::new("output/report.json"))
            .unwrap();
        assert_eq!(read_json, json_content);
    }

    /// Test that the mock environment correctly tracks multiple output files.
    #[test]
    fn test_mock_env_multi_file_tracking() {
        use super::super::render::{render_html, render_json, render_markdown};

        let results = create_test_results();
        let env = DebtmapTestEnv::new();

        // Write multiple files like write_analysis_report_effect would
        let formats = vec![
            ("report.md", render_markdown(&results).unwrap()),
            ("report.json", render_json(&results).unwrap()),
            ("report.html", render_html(&results).unwrap()),
        ];

        for (path, content) in &formats {
            env.file_system().write(Path::new(path), content).unwrap();
        }

        // Verify all files tracked
        assert_eq!(env.file_paths().len(), 3);
        for (path, expected_content) in &formats {
            let content = env.file_system().read_to_string(Path::new(path)).unwrap();
            assert_eq!(&content, expected_content);
        }
    }
}
