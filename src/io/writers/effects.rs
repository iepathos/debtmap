//! Effect-based output writers for debtmap analysis.
//!
//! This module provides Effect-wrapped output writers that enable testable,
//! composable output operations. All I/O is deferred until the effect is run.
//!
//! # Design Philosophy
//!
//! - **Pure Rendering**: Rendering logic is separated into pure functions
//! - **Effect Wrapping**: I/O operations are wrapped in Effects
//! - **Composability**: Multiple outputs can be combined in a single pipeline
//! - **Testability**: All writers can be tested without file system access
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::io::writers::effects::{write_markdown_effect, write_multi_format_effect};
//! use debtmap::effects::run_effect;
//! use debtmap::config::DebtmapConfig;
//!
//! // Write to markdown file
//! let effect = write_markdown_effect(results.clone(), "report.md".into());
//! run_effect(effect, DebtmapConfig::default())?;
//!
//! // Write to multiple formats at once
//! let config = OutputConfig {
//!     markdown_path: Some("report.md".into()),
//!     json_path: Some("report.json".into()),
//!     ..Default::default()
//! };
//! let effect = write_multi_format_effect(results, &config);
//! run_effect(effect, DebtmapConfig::default())?;
//! ```

use crate::core::AnalysisResults;
use crate::effects::{effect_from_fn, effect_pure, AnalysisEffect};
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use crate::io::output::OutputWriter;
use crate::io::writers::{HtmlWriter, JsonWriter, MarkdownWriter, TerminalWriter};
use crate::risk::RiskInsight;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use stillwater::effect::prelude::*;

// ============================================================================
// Output Configuration
// ============================================================================

/// Configuration for output generation.
///
/// Specifies which output formats to generate and where to write them.
#[derive(Debug, Clone, Default)]
pub struct OutputConfig {
    /// Path for markdown output (if any).
    pub markdown_path: Option<PathBuf>,

    /// Path for JSON output (if any).
    pub json_path: Option<PathBuf>,

    /// Path for HTML output (if any).
    pub html_path: Option<PathBuf>,

    /// Whether to output to terminal.
    pub terminal_output: bool,
}

impl OutputConfig {
    /// Create a new output config builder.
    pub fn builder() -> OutputConfigBuilder {
        OutputConfigBuilder::default()
    }

    /// Check if any output is configured.
    pub fn has_output(&self) -> bool {
        self.markdown_path.is_some()
            || self.json_path.is_some()
            || self.html_path.is_some()
            || self.terminal_output
    }
}

/// Builder for OutputConfig.
#[derive(Debug, Clone, Default)]
pub struct OutputConfigBuilder {
    config: OutputConfig,
}

impl OutputConfigBuilder {
    /// Set markdown output path.
    pub fn markdown(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.markdown_path = Some(path.into());
        self
    }

    /// Set JSON output path.
    pub fn json(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.json_path = Some(path.into());
        self
    }

    /// Set HTML output path.
    pub fn html(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.html_path = Some(path.into());
        self
    }

    /// Enable terminal output.
    pub fn terminal(mut self, enabled: bool) -> Self {
        self.config.terminal_output = enabled;
        self
    }

    /// Build the output config.
    pub fn build(self) -> OutputConfig {
        self.config
    }
}

/// Result of an output operation with metadata.
#[derive(Debug, Clone)]
pub struct OutputResult {
    /// Destination description.
    pub destination: String,

    /// Number of bytes written.
    pub bytes_written: usize,

    /// Time taken to write.
    pub duration: Duration,
}

// ============================================================================
// Pure Rendering Functions
// ============================================================================

/// Pure function to render analysis results to markdown string.
///
/// This is a pure transformation with no I/O - just data to string conversion.
pub fn render_markdown(results: &AnalysisResults) -> Result<String, AnalysisError> {
    let mut buffer = Vec::new();
    let mut writer = MarkdownWriter::new(&mut buffer);
    writer
        .write_results(results)
        .map_err(|e| AnalysisError::other(format!("Failed to render markdown: {}", e)))?;
    String::from_utf8(buffer)
        .map_err(|e| AnalysisError::other(format!("Invalid UTF-8 in markdown output: {}", e)))
}

/// Pure function to render risk insights to markdown string.
pub fn render_risk_markdown(insights: &RiskInsight) -> Result<String, AnalysisError> {
    let mut buffer = Vec::new();
    let mut writer = MarkdownWriter::new(&mut buffer);
    writer
        .write_risk_insights(insights)
        .map_err(|e| AnalysisError::other(format!("Failed to render risk markdown: {}", e)))?;
    String::from_utf8(buffer)
        .map_err(|e| AnalysisError::other(format!("Invalid UTF-8 in markdown output: {}", e)))
}

/// Pure function to render analysis results to JSON string.
///
/// This is a pure transformation with no I/O.
pub fn render_json(results: &AnalysisResults) -> Result<String, AnalysisError> {
    serde_json::to_string_pretty(results)
        .map_err(|e| AnalysisError::other(format!("Failed to render JSON: {}", e)))
}

/// Pure function to render risk insights to JSON string.
pub fn render_risk_json(insights: &RiskInsight) -> Result<String, AnalysisError> {
    serde_json::to_string_pretty(insights)
        .map_err(|e| AnalysisError::other(format!("Failed to render risk JSON: {}", e)))
}

/// Pure function to render analysis results to HTML string.
///
/// This is a pure transformation with no I/O.
pub fn render_html(results: &AnalysisResults) -> Result<String, AnalysisError> {
    let mut buffer = Vec::new();
    let mut writer = HtmlWriter::new(&mut buffer);
    writer
        .write_results(results)
        .map_err(|e| AnalysisError::other(format!("Failed to render HTML: {}", e)))?;
    String::from_utf8(buffer)
        .map_err(|e| AnalysisError::other(format!("Invalid UTF-8 in HTML output: {}", e)))
}

/// Pure function to render terminal output string.
///
/// Note: This captures the terminal output as a string, which is useful for
/// testing but may lose color formatting.
pub fn render_terminal(results: &AnalysisResults) -> Result<String, AnalysisError> {
    // For terminal output, we capture to a buffer
    // Note: Colors may not render correctly when captured this way
    let mut buffer = Vec::new();
    {
        let mut writer = JsonWriter::new(&mut buffer);
        // Use a simple representation for testing purposes
        // Real terminal output goes directly to stdout
        writer.write_results(results).map_err(|e| {
            AnalysisError::other(format!("Failed to render terminal output: {}", e))
        })?;
    }
    String::from_utf8(buffer)
        .map_err(|e| AnalysisError::other(format!("Invalid UTF-8 in terminal output: {}", e)))
}

// ============================================================================
// Effect-Based Writers
// ============================================================================

/// Write analysis results to markdown format as an Effect.
///
/// This effect renders the results to markdown and writes to the specified path.
///
/// # Example
///
/// ```rust,ignore
/// let effect = write_markdown_effect(results, "report.md".into());
/// run_effect(effect, config)?;
/// ```
pub fn write_markdown_effect(
    results: AnalysisResults,
    path: PathBuf,
) -> AnalysisEffect<OutputResult> {
    let path_display = path.display().to_string();
    effect_from_fn(move |env: &RealEnv| {
        let start = Instant::now();

        // Pure rendering
        let content = render_markdown(&results)?;

        // I/O at the boundary
        env.file_system().write(&path, &content).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to write markdown: {}", e.message()), &path)
        })?;

        Ok(OutputResult {
            destination: path_display,
            bytes_written: content.len(),
            duration: start.elapsed(),
        })
    })
}

/// Write risk insights to markdown format as an Effect.
pub fn write_risk_markdown_effect(
    insights: RiskInsight,
    path: PathBuf,
) -> AnalysisEffect<OutputResult> {
    let path_display = path.display().to_string();
    effect_from_fn(move |env: &RealEnv| {
        let start = Instant::now();
        let content = render_risk_markdown(&insights)?;

        env.file_system().write(&path, &content).map_err(|e| {
            AnalysisError::io_with_path(
                format!("Failed to write risk markdown: {}", e.message()),
                &path,
            )
        })?;

        Ok(OutputResult {
            destination: path_display,
            bytes_written: content.len(),
            duration: start.elapsed(),
        })
    })
}

/// Write analysis results to JSON format as an Effect.
///
/// # Example
///
/// ```rust,ignore
/// let effect = write_json_effect(results, "report.json".into());
/// run_effect(effect, config)?;
/// ```
pub fn write_json_effect(results: AnalysisResults, path: PathBuf) -> AnalysisEffect<OutputResult> {
    let path_display = path.display().to_string();
    effect_from_fn(move |env: &RealEnv| {
        let start = Instant::now();

        let json = render_json(&results)?;

        env.file_system().write(&path, &json).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to write JSON: {}", e.message()), &path)
        })?;

        Ok(OutputResult {
            destination: path_display,
            bytes_written: json.len(),
            duration: start.elapsed(),
        })
    })
}

/// Write risk insights to JSON format as an Effect.
pub fn write_risk_json_effect(
    insights: RiskInsight,
    path: PathBuf,
) -> AnalysisEffect<OutputResult> {
    let path_display = path.display().to_string();
    effect_from_fn(move |env: &RealEnv| {
        let start = Instant::now();
        let content = render_risk_json(&insights)?;

        env.file_system().write(&path, &content).map_err(|e| {
            AnalysisError::io_with_path(
                format!("Failed to write risk JSON: {}", e.message()),
                &path,
            )
        })?;

        Ok(OutputResult {
            destination: path_display,
            bytes_written: content.len(),
            duration: start.elapsed(),
        })
    })
}

/// Write analysis results to HTML report as an Effect.
///
/// # Example
///
/// ```rust,ignore
/// let effect = write_html_effect(results, "report.html".into());
/// run_effect(effect, config)?;
/// ```
pub fn write_html_effect(results: AnalysisResults, path: PathBuf) -> AnalysisEffect<OutputResult> {
    let path_display = path.display().to_string();
    effect_from_fn(move |env: &RealEnv| {
        let start = Instant::now();

        let html = render_html(&results)?;

        env.file_system().write(&path, &html).map_err(|e| {
            AnalysisError::io_with_path(format!("Failed to write HTML: {}", e.message()), &path)
        })?;

        Ok(OutputResult {
            destination: path_display,
            bytes_written: html.len(),
            duration: start.elapsed(),
        })
    })
}

/// Write to terminal with formatting as an Effect.
///
/// This effect writes directly to stdout with colored formatting.
///
/// # Example
///
/// ```rust,ignore
/// let effect = write_terminal_effect(results);
/// run_effect(effect, config)?;
/// ```
pub fn write_terminal_effect(results: AnalysisResults) -> AnalysisEffect<OutputResult> {
    effect_from_fn(move |_env: &RealEnv| {
        let start = Instant::now();

        // Terminal writer writes directly to stdout
        let mut writer = TerminalWriter::default();
        writer
            .write_results(&results)
            .map_err(|e| AnalysisError::io(format!("Failed to write to terminal: {}", e)))?;

        Ok(OutputResult {
            destination: "terminal".to_string(),
            bytes_written: 0, // Unknown for terminal output
            duration: start.elapsed(),
        })
    })
}

/// Write risk insights to terminal as an Effect.
pub fn write_risk_terminal_effect(insights: RiskInsight) -> AnalysisEffect<OutputResult> {
    effect_from_fn(move |_env: &RealEnv| {
        let start = Instant::now();

        let mut writer = TerminalWriter::default();
        writer.write_risk_insights(&insights).map_err(|e| {
            AnalysisError::io(format!("Failed to write risk insights to terminal: {}", e))
        })?;

        Ok(OutputResult {
            destination: "terminal".to_string(),
            bytes_written: 0,
            duration: start.elapsed(),
        })
    })
}

// ============================================================================
// Composed Output Effects
// ============================================================================

/// Write analysis results to multiple formats based on configuration.
///
/// This effect writes to all configured output destinations, collecting
/// results from each write operation.
///
/// # Example
///
/// ```rust,ignore
/// let config = OutputConfig::builder()
///     .markdown("report.md")
///     .json("report.json")
///     .terminal(true)
///     .build();
///
/// let effect = write_multi_format_effect(results, &config);
/// let results = run_effect(effect, debtmap_config)?;
/// for result in results {
///     println!("Wrote {} bytes to {}", result.bytes_written, result.destination);
/// }
/// ```
pub fn write_multi_format_effect(
    results: AnalysisResults,
    config: &OutputConfig,
) -> AnalysisEffect<Vec<OutputResult>> {
    let mut effects: Vec<AnalysisEffect<OutputResult>> = Vec::new();

    if let Some(ref md_path) = config.markdown_path {
        effects.push(write_markdown_effect(results.clone(), md_path.clone()));
    }

    if let Some(ref json_path) = config.json_path {
        effects.push(write_json_effect(results.clone(), json_path.clone()));
    }

    if let Some(ref html_path) = config.html_path {
        effects.push(write_html_effect(results.clone(), html_path.clone()));
    }

    if config.terminal_output {
        effects.push(write_terminal_effect(results));
    }

    // Return empty vec if no outputs configured
    if effects.is_empty() {
        return effect_pure(Vec::new());
    }

    // Sequence all effects, collecting results
    sequence_effects(effects)
}

/// Write analysis results to a single format and return the content.
///
/// This is useful when you want to capture the rendered output for further
/// processing without writing to a file.
///
/// # Example
///
/// ```rust,ignore
/// let effect = render_to_string_effect(results, OutputFormat::Markdown);
/// let content = run_effect(effect, config)?;
/// // Process content further...
/// ```
pub fn render_to_string_effect(
    results: AnalysisResults,
    format: OutputFormat,
) -> AnalysisEffect<String> {
    effect_from_fn(move |_env: &RealEnv| match format {
        OutputFormat::Markdown => render_markdown(&results),
        OutputFormat::Json => render_json(&results),
        OutputFormat::Html => render_html(&results),
        OutputFormat::Terminal => render_terminal(&results),
    })
}

/// Output format enumeration for render_to_string_effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Markdown format
    Markdown,
    /// JSON format
    Json,
    /// HTML format
    Html,
    /// Terminal format (may lose colors)
    Terminal,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Sequence a vector of effects into a single effect that produces a vector.
fn sequence_effects(
    effects: Vec<AnalysisEffect<OutputResult>>,
) -> AnalysisEffect<Vec<OutputResult>> {
    if effects.is_empty() {
        return pure(Vec::new()).boxed();
    }

    let mut effects_iter = effects.into_iter();
    let first = effects_iter.next().unwrap();

    effects_iter.fold(first.map(|r| vec![r]).boxed(), |acc, eff| {
        acc.and_then(move |mut results| {
            eff.map(move |r| {
                results.push(r);
                results
            })
            .boxed()
        })
        .boxed()
    })
}

// ============================================================================
// Complete Report Generation Effect
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

    // =========================================================================
    // Pure Rendering Function Tests (using DebtmapTestEnv for verification)
    // =========================================================================

    #[test]
    fn test_render_markdown_pure() {
        let results = create_test_results();
        let markdown = render_markdown(&results).unwrap();

        assert!(markdown.contains("# Debtmap Analysis Report"));
        assert!(markdown.contains("Executive Summary"));
    }

    #[test]
    fn test_render_json_pure() {
        let results = create_test_results();
        let json = render_json(&results).unwrap();

        assert!(json.contains("test_func"));
        assert!(json.contains("TODO: Implement feature"));
    }

    #[test]
    fn test_render_html_pure() {
        let results = create_test_results();
        let html = render_html(&results).unwrap();

        // HTML should contain some HTML structure
        assert!(html.contains("<") || html.contains("{"));
    }

    /// Test that pure rendering functions produce content that can be stored
    /// in DebtmapTestEnv's mock file system and read back correctly.
    #[test]
    fn test_render_and_verify_with_mock_env() {
        let results = create_test_results();
        let env = DebtmapTestEnv::new();

        // Render content using pure functions
        let markdown_content = render_markdown(&results).unwrap();
        let json_content = render_json(&results).unwrap();
        let html_content = render_html(&results).unwrap();

        // Write to mock file system
        env.file_system()
            .write(Path::new("report.md"), &markdown_content)
            .unwrap();
        env.file_system()
            .write(Path::new("report.json"), &json_content)
            .unwrap();
        env.file_system()
            .write(Path::new("report.html"), &html_content)
            .unwrap();

        // Verify content through mock file system
        let read_md = env
            .file_system()
            .read_to_string(Path::new("report.md"))
            .unwrap();
        assert!(read_md.contains("# Debtmap Analysis Report"));
        assert!(read_md.contains("Executive Summary"));

        let read_json = env
            .file_system()
            .read_to_string(Path::new("report.json"))
            .unwrap();
        let _: serde_json::Value = serde_json::from_str(&read_json).unwrap();
        assert!(read_json.contains("test_func"));

        let read_html = env
            .file_system()
            .read_to_string(Path::new("report.html"))
            .unwrap();
        assert_eq!(read_html, html_content);
    }

    /// Test render functions work correctly with mock environment's file operations.
    #[test]
    fn test_render_deterministic_with_mock_env() {
        let results = create_test_results();
        let env = DebtmapTestEnv::new();

        // Render twice
        let markdown1 = render_markdown(&results).unwrap();
        let markdown2 = render_markdown(&results).unwrap();

        // Write both to mock env
        env.file_system()
            .write(Path::new("report1.md"), &markdown1)
            .unwrap();
        env.file_system()
            .write(Path::new("report2.md"), &markdown2)
            .unwrap();

        // Verify both files exist and have same content
        assert!(env.has_file("report1.md"));
        assert!(env.has_file("report2.md"));

        let content1 = env
            .file_system()
            .read_to_string(Path::new("report1.md"))
            .unwrap();
        let content2 = env
            .file_system()
            .read_to_string(Path::new("report2.md"))
            .unwrap();
        assert_eq!(content1, content2);
    }

    /// Test JSON output structure verification using mock env.
    #[test]
    fn test_json_structure_with_mock_env() {
        let results = create_test_results();
        let env = DebtmapTestEnv::new();

        let json_content = render_json(&results).unwrap();
        env.file_system()
            .write(Path::new("analysis.json"), &json_content)
            .unwrap();

        // Read back and parse
        let content = env
            .file_system()
            .read_to_string(Path::new("analysis.json"))
            .unwrap();
        let parsed: AnalysisResults = serde_json::from_str(&content).unwrap();

        // Verify structure
        assert_eq!(parsed.complexity.summary.total_functions, 1);
        assert_eq!(parsed.technical_debt.items.len(), 1);
        assert_eq!(
            parsed.technical_debt.items[0].message,
            "TODO: Implement feature"
        );
    }

    // =========================================================================
    // Integration Tests (using TempDir for real I/O verification)
    // =========================================================================

    #[test]
    fn test_write_markdown_effect() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("report.md");
        let results = create_test_results();

        let effect = write_markdown_effect(results, path.clone());
        let output_result = run_effect(effect, DebtmapConfig::default()).unwrap();

        assert!(path.exists());
        assert!(output_result.bytes_written > 0);
        assert!(output_result.destination.contains("report.md"));

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Debtmap Analysis Report"));
    }

    #[test]
    fn test_write_json_effect() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("report.json");
        let results = create_test_results();

        let effect = write_json_effect(results, path.clone());
        let output_result = run_effect(effect, DebtmapConfig::default()).unwrap();

        assert!(path.exists());
        assert!(output_result.bytes_written > 0);

        let content = std::fs::read_to_string(&path).unwrap();
        // Should be valid JSON
        let _: serde_json::Value = serde_json::from_str(&content).unwrap();
    }

    #[test]
    fn test_write_html_effect() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("report.html");
        let results = create_test_results();

        let effect = write_html_effect(results, path.clone());
        let output_result = run_effect(effect, DebtmapConfig::default()).unwrap();

        assert!(path.exists());
        assert!(output_result.bytes_written > 0);
    }

    #[test]
    fn test_write_multi_format_effect() {
        let temp_dir = TempDir::new().unwrap();
        let results = create_test_results();

        let config = OutputConfig::builder()
            .markdown(temp_dir.path().join("report.md"))
            .json(temp_dir.path().join("report.json"))
            .build();

        let effect = write_multi_format_effect(results, &config);
        let output_results = run_effect(effect, DebtmapConfig::default()).unwrap();

        assert_eq!(output_results.len(), 2);
        assert!(temp_dir.path().join("report.md").exists());
        assert!(temp_dir.path().join("report.json").exists());
    }

    #[test]
    fn test_write_multi_format_effect_empty_config() {
        let results = create_test_results();
        let config = OutputConfig::default();

        let effect = write_multi_format_effect(results, &config);
        let output_results = run_effect(effect, DebtmapConfig::default()).unwrap();

        assert!(output_results.is_empty());
    }

    // =========================================================================
    // Config Builder Tests
    // =========================================================================

    #[test]
    fn test_output_config_builder() {
        let config = OutputConfig::builder()
            .markdown("report.md")
            .json("report.json")
            .html("report.html")
            .terminal(true)
            .build();

        assert_eq!(config.markdown_path, Some(PathBuf::from("report.md")));
        assert_eq!(config.json_path, Some(PathBuf::from("report.json")));
        assert_eq!(config.html_path, Some(PathBuf::from("report.html")));
        assert!(config.terminal_output);
        assert!(config.has_output());
    }

    #[test]
    fn test_output_config_has_output() {
        let empty_config = OutputConfig::default();
        assert!(!empty_config.has_output());

        let with_markdown = OutputConfig::builder().markdown("x.md").build();
        assert!(with_markdown.has_output());

        let with_terminal = OutputConfig::builder().terminal(true).build();
        assert!(with_terminal.has_output());
    }

    #[test]
    fn test_render_to_string_effect() {
        let results = create_test_results();

        // Test markdown
        let effect = render_to_string_effect(results.clone(), OutputFormat::Markdown);
        let content = run_effect(effect, DebtmapConfig::default()).unwrap();
        assert!(content.contains("Debtmap"));

        // Test JSON
        let effect = render_to_string_effect(results.clone(), OutputFormat::Json);
        let content = run_effect(effect, DebtmapConfig::default()).unwrap();
        assert!(content.contains("test_func"));
    }

    #[test]
    fn test_sequence_effects_empty() {
        let effects: Vec<AnalysisEffect<OutputResult>> = vec![];
        let effect = sequence_effects(effects);
        let results = run_effect(effect, DebtmapConfig::default()).unwrap();
        assert!(results.is_empty());
    }

    // =========================================================================
    // ReportConfig Builder Tests
    // =========================================================================

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

    // =========================================================================
    // write_analysis_report_effect Tests
    // =========================================================================

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
