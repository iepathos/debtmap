//! Sink Effect for Report Streaming
//!
//! This module provides the Sink Effect pattern for streaming analysis reports
//! with O(1) memory overhead. Unlike accumulating all report data before writing,
//! the Sink Effect streams output as it's generated.
//!
//! # Overview
//!
//! When analyzing large codebases, debtmap can generate substantial reports.
//! The traditional approach accumulates all data in memory before writing,
//! which can cause memory pressure. The Sink Effect pattern enables:
//!
//! - Real-time report generation as analysis progresses
//! - Constant memory usage regardless of codebase size
//! - Progressive feedback to users during long analyses
//!
//! # Example
//!
//! ```rust,ignore
//! use debtmap::effects::sink::{emit_report_line, ReportLine, run_with_file_sink};
//! use stillwater::effect::sink::prelude::*;
//!
//! // Stream file metrics as they're analyzed
//! let effect = analyze_file(&path)
//!     .and_then(|metrics| emit_report_line(ReportLine::json_line(&metrics)));
//!
//! // Execute with file sink
//! run_with_file_sink(effect, output_path).await?;
//! ```
//!
//! # Testing
//!
//! Use `run_collecting` to capture streamed output in tests:
//!
//! ```rust,ignore
//! let (result, lines) = effect.run_collecting(&env).await;
//! assert!(!lines.is_empty());
//! ```

use std::path::Path;

use serde::Serialize;
use stillwater::effect::sink::{emit, SinkEffect};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::errors::AnalysisError;

/// Report line types for streaming output.
///
/// These represent the different kinds of content that can be streamed
/// to a report output. Each variant corresponds to a specific format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportLine {
    /// Single JSON object as a line (JSON Lines format).
    JsonLine(String),
    /// Text report line.
    TextLine(String),
    /// Section separator.
    Separator,
    /// Section header.
    Header(String),
}

impl ReportLine {
    /// Create a JSON line from a serializable value.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let line = ReportLine::json_line(&file_metrics);
    /// ```
    pub fn json_line<T: Serialize>(value: &T) -> Self {
        match serde_json::to_string(value) {
            Ok(json) => Self::JsonLine(json),
            Err(e) => Self::TextLine(format!("{{\"error\": \"{}\"}}", e)),
        }
    }

    /// Create a text line.
    pub fn text(content: impl Into<String>) -> Self {
        Self::TextLine(content.into())
    }

    /// Create a section header.
    pub fn header(title: impl Into<String>) -> Self {
        Self::Header(title.into())
    }

    /// Create a separator line.
    pub fn separator() -> Self {
        Self::Separator
    }

    /// Convert to output string format.
    pub fn to_output_string(&self) -> String {
        match self {
            Self::JsonLine(json) => format!("{}\n", json),
            Self::TextLine(text) => format!("{}\n", text),
            Self::Separator => "---\n".to_string(),
            Self::Header(h) => format!("\n## {}\n\n", h),
        }
    }
}

/// Sink configuration for report output.
#[derive(Debug, Clone)]
pub struct SinkConfig {
    /// The output format.
    pub format: ReportFormat,
    /// The output destination.
    pub destination: SinkDestination,
    /// Buffer size for file output (in bytes).
    pub buffer_size: usize,
}

impl Default for SinkConfig {
    fn default() -> Self {
        Self {
            format: ReportFormat::JsonLines,
            destination: SinkDestination::Stdout,
            buffer_size: 8192,
        }
    }
}

/// Output format for streaming reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// One JSON object per line (JSONL).
    JsonLines,
    /// Human-readable text format.
    Text,
    /// Markdown-formatted report.
    Markdown,
}

/// Destination for sink output.
#[derive(Debug, Clone)]
pub enum SinkDestination {
    /// Write to stdout.
    Stdout,
    /// Write to a file.
    File(std::path::PathBuf),
}

/// Emit a single report line to the sink.
///
/// This is the primary helper for streaming report output.
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::sink::{emit_report_line, ReportLine};
///
/// let effect = emit_report_line(ReportLine::header("Analysis Results"));
/// ```
pub fn emit_report_line<E, Env>(
    line: ReportLine,
) -> impl SinkEffect<Output = (), Error = E, Env = Env, Item = ReportLine>
where
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
{
    emit(line)
}

/// Emit multiple report lines to the sink.
///
/// Use this when you have several lines to emit at once.
pub fn emit_report_lines<E, Env>(
    lines: Vec<ReportLine>,
) -> impl SinkEffect<Output = (), Error = E, Env = Env, Item = ReportLine>
where
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
{
    stillwater::effect::sink::emit_many(lines)
}

/// Emit a JSON line for a serializable value.
///
/// Convenience function that serializes the value to JSON Lines format.
pub fn emit_json_line<T, E, Env>(
    value: &T,
) -> impl SinkEffect<Output = (), Error = E, Env = Env, Item = ReportLine>
where
    T: Serialize,
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
{
    emit_report_line(ReportLine::json_line(value))
}

/// Emit a text line.
pub fn emit_text_line<E, Env>(
    text: impl Into<String>,
) -> impl SinkEffect<Output = (), Error = E, Env = Env, Item = ReportLine>
where
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
{
    emit_report_line(ReportLine::text(text))
}

/// Emit a section header.
pub fn emit_header<E, Env>(
    title: impl Into<String>,
) -> impl SinkEffect<Output = (), Error = E, Env = Env, Item = ReportLine>
where
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
{
    emit_report_line(ReportLine::header(title))
}

/// Emit a separator.
pub fn emit_separator<E, Env>(
) -> impl SinkEffect<Output = (), Error = E, Env = Env, Item = ReportLine>
where
    E: Send + 'static,
    Env: Clone + Send + Sync + 'static,
{
    emit_report_line(ReportLine::separator())
}

/// Execute a sink effect, streaming output to a file.
///
/// This function creates an async buffered writer and streams all
/// emitted report lines to the specified file.
///
/// # Arguments
///
/// * `effect` - The sink effect to execute
/// * `output_path` - Path to the output file
/// * `env` - The environment for the effect
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::sink::{emit_header, emit_text_line, run_with_file_sink};
/// use stillwater::effect::sink::prelude::*;
///
/// let effect = emit_header::<AnalysisError, ()>("Results")
///     .and_then(|_| emit_text_line("Line 1"));
///
/// run_with_file_sink(effect, Path::new("output.txt"), &()).await?;
/// ```
pub async fn run_with_file_sink<T, Env, E>(
    effect: E,
    output_path: &Path,
    env: &Env,
) -> Result<T, AnalysisError>
where
    T: Send,
    Env: Clone + Send + Sync + 'static,
    E: SinkEffect<Output = T, Error = AnalysisError, Env = Env, Item = ReportLine>,
{
    let file = File::create(output_path)
        .await
        .map_err(|e| AnalysisError::io(format!("Failed to create output file: {}", e)))?;

    let writer = std::sync::Arc::new(tokio::sync::Mutex::new(BufWriter::new(file)));

    let result = effect
        .run_with_sink(env, |line| {
            let writer = writer.clone();
            async move {
                let text = line.to_output_string();
                let mut w = writer.lock().await;
                let _ = w.write_all(text.as_bytes()).await;
            }
        })
        .await?;

    // Flush the buffer
    let mut w = writer.lock().await;
    w.flush()
        .await
        .map_err(|e| AnalysisError::io(format!("Failed to flush output file: {}", e)))?;

    Ok(result)
}

/// Execute a sink effect, streaming output to stdout.
///
/// This function writes all emitted report lines to stdout immediately.
///
/// # Arguments
///
/// * `effect` - The sink effect to execute
/// * `env` - The environment for the effect
///
/// # Example
///
/// ```rust,ignore
/// use debtmap::effects::sink::{emit_text_line, run_with_stdout_sink};
///
/// let effect = emit_text_line::<AnalysisError, ()>("Hello, world!");
/// run_with_stdout_sink(effect, &()).await?;
/// ```
pub async fn run_with_stdout_sink<T, Env, E>(effect: E, env: &Env) -> Result<T, AnalysisError>
where
    T: Send,
    Env: Clone + Send + Sync + 'static,
    E: SinkEffect<Output = T, Error = AnalysisError, Env = Env, Item = ReportLine>,
{
    let writer = std::sync::Arc::new(tokio::sync::Mutex::new(tokio::io::stdout()));

    let result = effect
        .run_with_sink(env, |line| {
            let writer = writer.clone();
            async move {
                let text = line.to_output_string();
                let mut w = writer.lock().await;
                let _ = w.write_all(text.as_bytes()).await;
                let _ = w.flush().await;
            }
        })
        .await?;

    Ok(result)
}

/// Execute a sink effect with the configured destination.
///
/// This is a convenience function that dispatches to either
/// `run_with_file_sink` or `run_with_stdout_sink` based on config.
pub async fn run_with_sink<T, Env, E>(
    effect: E,
    config: &SinkConfig,
    env: &Env,
) -> Result<T, AnalysisError>
where
    T: Send,
    Env: Clone + Send + Sync + 'static,
    E: SinkEffect<Output = T, Error = AnalysisError, Env = Env, Item = ReportLine>,
{
    match &config.destination {
        SinkDestination::Stdout => run_with_stdout_sink(effect, env).await,
        SinkDestination::File(path) => run_with_file_sink(effect, path, env).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stillwater::effect::sink::SinkEffectExt;

    #[test]
    fn report_line_json_line_serializes() {
        let data = serde_json::json!({"key": "value", "count": 42});
        let line = ReportLine::json_line(&data);
        match line {
            ReportLine::JsonLine(json) => {
                assert!(json.contains("key"));
                assert!(json.contains("value"));
                assert!(json.contains("42"));
            }
            _ => panic!("Expected JsonLine"),
        }
    }

    #[test]
    fn report_line_text_creates_text_line() {
        let line = ReportLine::text("Hello, world!");
        assert_eq!(line, ReportLine::TextLine("Hello, world!".to_string()));
    }

    #[test]
    fn report_line_header_creates_header() {
        let line = ReportLine::header("Analysis Results");
        assert_eq!(line, ReportLine::Header("Analysis Results".to_string()));
    }

    #[test]
    fn report_line_separator_creates_separator() {
        let line = ReportLine::separator();
        assert_eq!(line, ReportLine::Separator);
    }

    #[test]
    fn report_line_to_output_string_json() {
        let line = ReportLine::JsonLine(r#"{"key":"value"}"#.to_string());
        assert_eq!(line.to_output_string(), "{\"key\":\"value\"}\n");
    }

    #[test]
    fn report_line_to_output_string_text() {
        let line = ReportLine::TextLine("Hello".to_string());
        assert_eq!(line.to_output_string(), "Hello\n");
    }

    #[test]
    fn report_line_to_output_string_separator() {
        let line = ReportLine::Separator;
        assert_eq!(line.to_output_string(), "---\n");
    }

    #[test]
    fn report_line_to_output_string_header() {
        let line = ReportLine::Header("Title".to_string());
        assert_eq!(line.to_output_string(), "\n## Title\n\n");
    }

    #[test]
    fn sink_config_default() {
        let config = SinkConfig::default();
        assert_eq!(config.format, ReportFormat::JsonLines);
        assert!(matches!(config.destination, SinkDestination::Stdout));
        assert_eq!(config.buffer_size, 8192);
    }

    #[tokio::test]
    async fn emit_report_line_collects_single_line() {
        let effect = emit_report_line::<AnalysisError, ()>(ReportLine::text("test line"));
        let (result, lines) = effect.run_collecting(&()).await;

        assert!(result.is_ok());
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], ReportLine::TextLine("test line".to_string()));
    }

    #[tokio::test]
    async fn emit_report_lines_collects_multiple() {
        let lines_to_emit = vec![
            ReportLine::header("Section"),
            ReportLine::text("Line 1"),
            ReportLine::text("Line 2"),
            ReportLine::separator(),
        ];

        let effect = emit_report_lines::<AnalysisError, ()>(lines_to_emit.clone());
        let (result, collected) = effect.run_collecting(&()).await;

        assert!(result.is_ok());
        assert_eq!(collected.len(), 4);
        assert_eq!(collected[0], ReportLine::Header("Section".to_string()));
    }

    #[tokio::test]
    async fn chained_emissions_collect_in_order() {
        let effect = emit_header::<AnalysisError, ()>("Start")
            .and_then(|_| emit_text_line("Middle"))
            .and_then(|_| emit_separator())
            .and_then(|_| emit_text_line("End"));

        let (result, lines) = effect.run_collecting(&()).await;

        assert!(result.is_ok());
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], ReportLine::Header("Start".to_string()));
        assert_eq!(lines[1], ReportLine::TextLine("Middle".to_string()));
        assert_eq!(lines[2], ReportLine::Separator);
        assert_eq!(lines[3], ReportLine::TextLine("End".to_string()));
    }

    #[tokio::test]
    async fn sink_effect_with_computation_result() {
        let effect = emit_text_line::<AnalysisError, ()>("Starting")
            .and_then(|_| emit_text_line("Processing"))
            .map(|_| 42);

        let (result, lines) = effect.run_collecting(&()).await;

        assert_eq!(result, Ok(42));
        assert_eq!(lines.len(), 2);
    }

    #[tokio::test]
    async fn run_with_file_sink_writes_to_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("test_output.txt");

        let effect = emit_header::<AnalysisError, ()>("Test Report")
            .and_then(|_| emit_text_line("Line 1"))
            .and_then(|_| emit_separator())
            .and_then(|_| emit_text_line("Line 2"));

        let result = run_with_file_sink(effect, &output_path, &()).await;
        assert!(result.is_ok());

        // Verify file contents
        let contents = std::fs::read_to_string(&output_path).unwrap();
        assert!(contents.contains("## Test Report"));
        assert!(contents.contains("Line 1"));
        assert!(contents.contains("---"));
        assert!(contents.contains("Line 2"));
    }

    #[tokio::test]
    async fn json_line_serialization_handles_complex_types() {
        #[derive(Serialize)]
        struct TestData {
            name: String,
            count: u32,
            nested: NestedData,
        }

        #[derive(Serialize)]
        struct NestedData {
            value: f64,
        }

        let data = TestData {
            name: "test".to_string(),
            count: 42,
            nested: NestedData { value: 99.5 },
        };

        let line = ReportLine::json_line(&data);
        if let ReportLine::JsonLine(json) = line {
            assert!(json.contains("test"));
            assert!(json.contains("42"));
            assert!(json.contains("99.5"));
        } else {
            panic!("Expected JsonLine");
        }
    }

    #[test]
    fn json_line_handles_serialization_error() {
        // f64::NAN is not valid JSON
        let data = f64::NAN;
        let line = ReportLine::json_line(&data);

        // Should produce an error message, not crash
        if let ReportLine::TextLine(text) = line {
            assert!(text.contains("error"));
        } else if let ReportLine::JsonLine(json) = line {
            // serde_json might serialize NaN as null or produce an error
            // Either is acceptable behavior
            assert!(!json.is_empty());
        }
    }
}
