//! Multi-format output composition.
//!
//! This module provides functions to compose multiple output writers
//! into combined effects.

use crate::core::AnalysisResults;
use crate::effects::{effect_from_fn, effect_pure, AnalysisEffect};
use crate::env::RealEnv;
use stillwater::effect::prelude::*;

use super::config::{OutputConfig, OutputFormat, OutputResult};
use super::render::{render_html, render_json, render_markdown, render_terminal};
use super::writers::{
    write_html_effect, write_json_effect, write_markdown_effect, write_terminal_effect,
};

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

// ============================================================================
// Helper Functions
// ============================================================================

/// Sequence a vector of effects into a single effect that produces a vector.
pub(crate) fn sequence_effects(
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
    use std::path::PathBuf;
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
}
