//! LLM-optimized markdown output (Spec 264)
//!
//! This module provides the output function for LLM-optimized markdown format,
//! designed for AI agent consumption.

use crate::io::writers::LlmMarkdownWriter;
use crate::output::unified::convert_to_unified_format;
use crate::priority::UnifiedAnalysis;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Output analysis as LLM-optimized markdown (Spec 264)
///
/// This function produces markdown designed for AI agent consumption:
/// - Hierarchical with consistent heading levels
/// - No decorative elements (emoji, boxes, separators)
/// - Complete with all available data
/// - Stable item IDs for reference
pub fn output_llm_markdown(analysis: &UnifiedAnalysis, output_file: Option<PathBuf>) -> Result<()> {
    output_llm_markdown_with_filters(analysis, None, None, output_file)
}

/// Output analysis as LLM-optimized markdown with filters (Spec 264)
pub fn output_llm_markdown_with_filters(
    analysis: &UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    output_file: Option<PathBuf>,
) -> Result<()> {
    output_llm_markdown_with_format(analysis, top, tail, output_file, false)
}

/// Output analysis as LLM-optimized markdown with full options (Spec 264)
pub fn output_llm_markdown_with_format(
    analysis: &UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    output_file: Option<PathBuf>,
    include_scoring_details: bool,
) -> Result<()> {
    // Convert to unified format (same as JSON for consistency)
    let unified_output = convert_to_unified_format(analysis, include_scoring_details);

    // Apply filtering
    let filtered = apply_filters(unified_output, top, tail);

    if let Some(path) = output_file {
        if let Some(parent) = path.parent() {
            crate::io::ensure_dir(parent)?;
        }
        let mut file = fs::File::create(path)?;
        let mut writer = LlmMarkdownWriter::new(&mut file);
        writer.write_unified_output(&filtered)?;
    } else {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let mut writer = LlmMarkdownWriter::new(&mut handle);
        writer.write_unified_output(&filtered)?;
    }
    Ok(())
}

/// Apply top/tail filters to unified output
fn apply_filters(
    mut output: crate::output::unified::UnifiedOutput,
    top: Option<usize>,
    tail: Option<usize>,
) -> crate::output::unified::UnifiedOutput {
    if let Some(n) = top {
        output.items.truncate(n);
    } else if let Some(n) = tail {
        let total = output.items.len();
        let skip = total.saturating_sub(n);
        output.items = output.items.into_iter().skip(skip).collect();
    }

    // Update summary to reflect filtered items
    output.summary.total_items = output.items.len();
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::call_graph::CallGraph;
    use tempfile::TempDir;

    #[test]
    fn test_output_llm_markdown_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        let call_graph = CallGraph::new();
        let analysis = UnifiedAnalysis::new(call_graph);

        let result = output_llm_markdown(&analysis, Some(output_path.clone()));
        assert!(
            result.is_ok(),
            "Failed to write LLM markdown: {:?}",
            result.err()
        );
        assert!(output_path.exists(), "Output file was not created");

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("# Debtmap Analysis Report"));
        assert!(content.contains("## Metadata"));
        assert!(content.contains("## Summary"));
        assert!(content.contains("## Debt Items"));
    }

    #[test]
    fn test_output_llm_markdown_with_filters() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("filtered.md");

        let call_graph = CallGraph::new();
        let analysis = UnifiedAnalysis::new(call_graph);

        // Test with top=5
        let result =
            output_llm_markdown_with_filters(&analysis, Some(5), None, Some(output_path.clone()));
        assert!(result.is_ok());

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("# Debtmap Analysis Report"));
    }

    #[test]
    fn test_output_llm_markdown_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("subdirs")
            .join("output.md");

        let call_graph = CallGraph::new();
        let analysis = UnifiedAnalysis::new(call_graph);

        let result = output_llm_markdown(&analysis, Some(nested_path.clone()));
        assert!(
            result.is_ok(),
            "Failed to write to nested path: {:?}",
            result.err()
        );
        assert!(
            nested_path.exists(),
            "Output file was not created at nested path"
        );
    }
}
