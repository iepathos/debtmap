//! Effect-based wrappers for diagnostic generation (Spec 207).
//!
//! This module provides effect-based interfaces for diagnostic generation,
//! enabling configuration access via the Reader pattern and supporting
//! testability with `DebtmapTestEnv`.
//!
//! # Pure Functions vs Effects
//!
//! The diagnostics module has a clean separation:
//!
//! - **Pure functions**: `generate_summary`, `generate_detailed_attribution`
//! - **Effect wrappers**: Functions that need config access (thresholds, formats)
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::analysis::diagnostics::effects::generate_diagnostics_effect;
//!
//! let effect = generate_diagnostics_effect(&analysis_results);
//! let diagnostics = run_effect(effect, config)?;
//! ```

use super::{
    generate_detailed_attribution, generate_summary, DetailLevel, DiagnosticReport,
    DiagnosticReporter, OutputFormat,
};
use crate::analysis::effects::{analyze_with_config, lift_pure, query_config};
use crate::analysis::multi_pass::MultiPassResult;
use crate::effects::AnalysisEffect;
use crate::env::RealEnv;
use crate::errors::AnalysisError;
use stillwater::Effect;

/// Generate a diagnostic report from multi-pass results using effect pattern.
///
/// This effect wrapper queries the configuration for output settings and
/// generates the report accordingly.
pub fn generate_report_effect(result: MultiPassResult) -> AnalysisEffect<DiagnosticReport> {
    analyze_with_config(move |config| {
        // Get output preferences from config or use defaults
        let detail_level = get_detail_level_from_config(config);
        let output_format = get_output_format_from_config(config);

        let reporter = DiagnosticReporter::new(output_format, detail_level);
        Ok(reporter.generate_report(&result))
    })
}

/// Generate a formatted diagnostic report as a string.
pub fn format_report_effect(result: MultiPassResult) -> AnalysisEffect<String> {
    analyze_with_config(move |config| {
        let detail_level = get_detail_level_from_config(config);
        let output_format = get_output_format_from_config(config);

        let reporter = DiagnosticReporter::new(output_format, detail_level);
        let report = reporter.generate_report(&result);
        Ok(reporter.format_report(&report))
    })
}

/// Generate diagnostics with custom detail level.
///
/// This effect allows overriding the config-based detail level.
pub fn generate_report_with_detail(
    result: MultiPassResult,
    detail_level: DetailLevel,
) -> AnalysisEffect<DiagnosticReport> {
    analyze_with_config(move |config| {
        let output_format = get_output_format_from_config(config);
        let reporter = DiagnosticReporter::new(output_format, detail_level.clone());
        Ok(reporter.generate_report(&result))
    })
}

/// Generate diagnostics with explicit formatting options.
///
/// This is a pure function lifted into an effect for API consistency.
pub fn generate_report_with_options(
    result: MultiPassResult,
    detail_level: DetailLevel,
    output_format: OutputFormat,
) -> AnalysisEffect<DiagnosticReport> {
    let reporter = DiagnosticReporter::new(output_format, detail_level);
    let report = reporter.generate_report(&result);
    lift_pure(report)
}

/// Generate summary from analysis results as an effect.
///
/// This wraps the pure `generate_summary` function in an effect for
/// composition with other effect-based operations.
pub fn generate_summary_effect(
    result: MultiPassResult,
) -> AnalysisEffect<super::ComplexitySummary> {
    let summary = generate_summary(&result);
    lift_pure(summary)
}

/// Generate detailed attribution as an effect.
///
/// Wraps the pure `generate_detailed_attribution` function.
pub fn generate_detailed_attribution_effect(
    result: MultiPassResult,
) -> AnalysisEffect<super::DetailedAttribution> {
    let attribution = generate_detailed_attribution(&result.attribution);
    lift_pure(attribution)
}

/// Query the detail level from configuration.
///
/// Returns the configured detail level or `DetailLevel::Standard` as default.
pub fn get_detail_level_effect(
) -> impl Effect<Output = DetailLevel, Error = AnalysisError, Env = RealEnv> {
    query_config(get_detail_level_from_config)
}

/// Query the output format from configuration.
///
/// Returns the configured output format or `OutputFormat::Json` as default.
pub fn get_output_format_effect(
) -> impl Effect<Output = OutputFormat, Error = AnalysisError, Env = RealEnv> {
    query_config(get_output_format_from_config)
}

// Helper functions to extract config values

fn get_detail_level_from_config(config: &crate::config::DebtmapConfig) -> DetailLevel {
    config
        .output
        .as_ref()
        .and_then(|o| o.detail_level.as_ref())
        .map(|s| match s.as_str() {
            "summary" => DetailLevel::Summary,
            "comprehensive" => DetailLevel::Comprehensive,
            "debug" => DetailLevel::Debug,
            _ => DetailLevel::Standard,
        })
        .unwrap_or(DetailLevel::Standard)
}

fn get_output_format_from_config(config: &crate::config::DebtmapConfig) -> OutputFormat {
    config
        .output
        .as_ref()
        .and_then(|o| o.format.as_ref().or(o.default_format.as_ref()))
        .map(|s| match s.as_str() {
            "yaml" => OutputFormat::Yaml,
            "markdown" | "md" => OutputFormat::Markdown,
            "html" => OutputFormat::Html,
            "text" | "txt" => OutputFormat::Text,
            _ => OutputFormat::Json,
        })
        .unwrap_or(OutputFormat::Json)
}

// =============================================================================
// Backwards Compatibility Wrappers
// =============================================================================

/// Generate a diagnostic report (backwards-compatible wrapper).
///
/// This function maintains the existing API while using effects internally.
pub fn generate_report_result(
    result: &MultiPassResult,
    config: &crate::config::DebtmapConfig,
) -> DiagnosticReport {
    let detail_level = get_detail_level_from_config(config);
    let output_format = get_output_format_from_config(config);
    let reporter = DiagnosticReporter::new(output_format, detail_level);
    reporter.generate_report(result)
}

/// Format a diagnostic report to string (backwards-compatible wrapper).
pub fn format_report_result(
    result: &MultiPassResult,
    config: &crate::config::DebtmapConfig,
) -> String {
    let detail_level = get_detail_level_from_config(config);
    let output_format = get_output_format_from_config(config);
    let reporter = DiagnosticReporter::new(output_format, detail_level);
    let report = reporter.generate_report(result);
    reporter.format_report(&report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::attribution::{AttributedComplexity, ComplexityAttribution};
    use crate::analysis::multi_pass::{AnalysisType, ComplexityResult};
    use crate::config::{DebtmapConfig, OutputConfig};
    use crate::env::RealEnv;

    fn create_test_result() -> MultiPassResult {
        MultiPassResult {
            raw_complexity: ComplexityResult {
                total_complexity: 20,
                cognitive_complexity: 15,
                functions: vec![],
                analysis_type: AnalysisType::Raw,
            },
            normalized_complexity: ComplexityResult {
                total_complexity: 15,
                cognitive_complexity: 12,
                functions: vec![],
                analysis_type: AnalysisType::Normalized,
            },
            attribution: ComplexityAttribution {
                logical_complexity: AttributedComplexity {
                    total: 12,
                    breakdown: vec![],
                    confidence: 0.9,
                },
                formatting_artifacts: AttributedComplexity {
                    total: 5,
                    breakdown: vec![],
                    confidence: 0.8,
                },
                pattern_complexity: AttributedComplexity {
                    total: 3,
                    breakdown: vec![],
                    confidence: 0.7,
                },
                source_mappings: vec![],
            },
            insights: vec![],
            recommendations: vec![],
            performance_metrics: None,
        }
    }

    #[tokio::test]
    async fn test_generate_report_effect() {
        let env = RealEnv::default();
        let result = create_test_result();

        let effect = generate_report_effect(result);
        let report = effect.run(&env).await.unwrap();

        assert_eq!(report.summary.raw_complexity, 20);
        assert_eq!(report.summary.normalized_complexity, 15);
    }

    #[tokio::test]
    async fn test_format_report_effect() {
        let env = RealEnv::default();
        let result = create_test_result();

        let effect = format_report_effect(result);
        let formatted = effect.run(&env).await.unwrap();

        // Default format is JSON
        assert!(formatted.contains("\"raw_complexity\""));
    }

    #[tokio::test]
    async fn test_generate_report_with_detail() {
        let env = RealEnv::default();
        let result = create_test_result();

        let effect = generate_report_with_detail(result, DetailLevel::Debug);
        let report = effect.run(&env).await.unwrap();

        // Debug level includes more details
        assert_eq!(report.summary.raw_complexity, 20);
    }

    #[tokio::test]
    async fn test_generate_summary_effect() {
        let env = RealEnv::default();
        let result = create_test_result();

        let effect = generate_summary_effect(result);
        let summary = effect.run(&env).await.unwrap();

        assert_eq!(summary.raw_complexity, 20);
        assert_eq!(summary.normalized_complexity, 15);
    }

    #[tokio::test]
    async fn test_get_detail_level_effect_default() {
        let env = RealEnv::default();

        let effect = get_detail_level_effect();
        let level = effect.run(&env).await.unwrap();

        assert!(matches!(level, DetailLevel::Standard));
    }

    #[tokio::test]
    async fn test_get_detail_level_effect_custom() {
        let config = DebtmapConfig {
            output: Some(OutputConfig {
                detail_level: Some("comprehensive".to_string()),
                format: None,
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect = get_detail_level_effect();
        let level = effect.run(&env).await.unwrap();

        assert!(matches!(level, DetailLevel::Comprehensive));
    }

    #[tokio::test]
    async fn test_get_output_format_effect_custom() {
        let config = DebtmapConfig {
            output: Some(OutputConfig {
                format: Some("markdown".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let env = RealEnv::new(config);

        let effect = get_output_format_effect();
        let format = effect.run(&env).await.unwrap();

        assert!(matches!(format, OutputFormat::Markdown));
    }

    #[test]
    fn test_backwards_compat_generate_report_result() {
        let result = create_test_result();
        let config = DebtmapConfig::default();

        let report = generate_report_result(&result, &config);

        assert_eq!(report.summary.raw_complexity, 20);
    }

    #[test]
    fn test_backwards_compat_format_report_result() {
        let result = create_test_result();
        let config = DebtmapConfig::default();

        let formatted = format_report_result(&result, &config);

        assert!(formatted.contains("raw_complexity"));
    }
}
