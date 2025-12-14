//! Effect-based single-format output writers.
//!
//! This module provides Effect-wrapped output writers for individual formats.
//! Each writer composes pure rendering with I/O operations.
//!
//! # Stillwater Philosophy
//!
//! These are the "flowing water" shell - they wrap pure rendering functions
//! with effect-based I/O at the boundaries.

use crate::core::AnalysisResults;
use crate::effects::{effect_from_fn, AnalysisEffect};
use crate::env::{AnalysisEnv, RealEnv};
use crate::errors::AnalysisError;
use crate::io::output::OutputWriter;
use crate::io::writers::TerminalWriter;
use crate::risk::RiskInsight;
use std::path::PathBuf;
use std::time::Instant;

use super::config::OutputResult;
use super::render::{
    render_html, render_json, render_markdown, render_risk_json, render_risk_markdown,
};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::core::{
        ComplexityReport, ComplexitySummary, DebtItem, DebtType, DependencyReport, FunctionMetrics,
        Priority, TechnicalDebtReport,
    };
    use crate::effects::run_effect;
    use chrono::Utc;
    use std::collections::HashMap;
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
}
