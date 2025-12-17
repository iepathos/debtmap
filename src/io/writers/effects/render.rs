//! Pure rendering functions for analysis output.
//!
//! This module contains pure transformation functions that convert analysis
//! results into various string formats. These functions have no side effects
//! and are easily testable.
//!
//! # Stillwater Philosophy
//!
//! These are the "still water" core - pure, predictable, testable functions
//! that transform data without any I/O.

use crate::core::AnalysisResults;
use crate::errors::AnalysisError;
use crate::io::output::OutputWriter;
use crate::io::writers::{HtmlWriter, JsonWriter, MarkdownWriter};
use crate::risk::RiskInsight;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        ComplexityReport, ComplexitySummary, DebtItem, DebtType, DependencyReport, FunctionMetrics,
        Priority, TechnicalDebtReport,
    };
    use crate::env::AnalysisEnv;
    use crate::testkit::DebtmapTestEnv;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

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
}
