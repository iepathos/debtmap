//! Configuration types for effect-based output writers.
//!
//! This module contains all configuration structures and their builders
//! for controlling output generation.

use std::path::PathBuf;
use std::time::Duration;

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

// ============================================================================
// Output Result
// ============================================================================

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
// Output Format
// ============================================================================

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
