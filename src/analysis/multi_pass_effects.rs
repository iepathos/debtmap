//! Effect-based wrappers for multi-pass analysis (Spec 207).
//!
//! This module provides effect-based interfaces for multi-pass complexity analysis,
//! enabling configuration access via the Reader pattern and supporting testability
//! with `DebtmapTestEnv`.
//!
//! # Architecture
//!
//! The multi-pass module follows a "pure core, effects shell" pattern:
//!
//! - **Pure functions**: Complexity calculation, attribution, insight generation
//! - **Effect wrappers**: Operations that need config access for thresholds
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::analysis::multi_pass_effects::analyze_multi_pass_effect;
//!
//! let effect = analyze_multi_pass_effect(source, language);
//! let result = run_effect(effect, config)?;
//! ```

use super::multi_pass::{AnalysisUnit, MultiPassAnalyzer, MultiPassOptions, MultiPassResult};
use crate::analysis::diagnostics::{DetailLevel, DiagnosticReport, OutputFormat};
use crate::analysis::effects::{analyze_with_config, lift_pure, query_config};
use crate::core::Language;
use crate::effects::AnalysisEffect;
use crate::env::RealEnv;
use crate::errors::AnalysisError;
use std::path::PathBuf;
use stillwater::Effect;

/// Perform multi-pass complexity analysis as an effect.
///
/// This effect queries configuration for analysis options and runs
/// the full multi-pass analysis pipeline.
pub fn analyze_multi_pass_effect(
    source: String,
    language: Language,
    file_path: PathBuf,
) -> AnalysisEffect<MultiPassResult> {
    analyze_with_config(move |config| {
        let options = get_multi_pass_options_from_config(config, language);
        let analyzer = MultiPassAnalyzer::new(options);
        let unit = AnalysisUnit::new(&source, language, file_path);

        analyzer
            .analyze(&unit)
            .map_err(|e| AnalysisError::analysis(format!("Multi-pass analysis failed: {}", e)))
    })
}

/// Perform multi-pass analysis with explicit options.
///
/// Use this when you want to override config-based options.
pub fn analyze_with_options_effect(
    source: String,
    language: Language,
    file_path: PathBuf,
    options: MultiPassOptions,
) -> AnalysisEffect<MultiPassResult> {
    let result = {
        let analyzer = MultiPassAnalyzer::new(options);
        let unit = AnalysisUnit::new(&source, language, file_path);
        analyzer.analyze(&unit)
    };

    match result {
        Ok(r) => lift_pure(r),
        Err(e) => crate::effects::effect_fail(AnalysisError::analysis(format!(
            "Multi-pass analysis failed: {}",
            e
        ))),
    }
}

/// Generate a diagnostic report from multi-pass results as an effect.
pub fn generate_report_effect(result: MultiPassResult) -> AnalysisEffect<DiagnosticReport> {
    analyze_with_config(move |config| {
        let detail_level = get_detail_level_from_config(config);
        let output_format = get_output_format_from_config(config);
        let options = MultiPassOptions {
            output_format,
            detail_level,
            ..Default::default()
        };
        let analyzer = MultiPassAnalyzer::new(options);
        Ok(analyzer.generate_report(&result))
    })
}

/// Get multi-pass options from configuration.
pub fn get_multi_pass_options_effect(
    language: Language,
) -> impl Effect<Output = MultiPassOptions, Error = AnalysisError, Env = RealEnv> {
    query_config(move |config| get_multi_pass_options_from_config(config, language))
}

// Helper functions

fn get_multi_pass_options_from_config(
    config: &crate::config::DebtmapConfig,
    language: Language,
) -> MultiPassOptions {
    let detail_level = get_detail_level_from_config(config);
    let output_format = get_output_format_from_config(config);
    let performance_tracking = matches!(detail_level, DetailLevel::Debug);

    MultiPassOptions {
        language,
        detail_level,
        enable_recommendations: true,
        track_source_locations: true,
        generate_insights: true,
        output_format,
        performance_tracking,
    }
}

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

/// Perform multi-pass analysis (backwards-compatible wrapper).
pub fn analyze_multi_pass_result(
    source: &str,
    language: Language,
    config: &crate::config::DebtmapConfig,
) -> anyhow::Result<MultiPassResult> {
    let options = get_multi_pass_options_from_config(config, language);
    let analyzer = MultiPassAnalyzer::new(options);
    let unit = AnalysisUnit::new(source, language, PathBuf::from("source.rs"));
    analyzer.analyze(&unit)
}

/// Generate a report (backwards-compatible wrapper).
pub fn generate_report_result(
    result: &MultiPassResult,
    config: &crate::config::DebtmapConfig,
) -> DiagnosticReport {
    let detail_level = get_detail_level_from_config(config);
    let output_format = get_output_format_from_config(config);
    let options = MultiPassOptions {
        output_format,
        detail_level,
        ..Default::default()
    };
    let analyzer = MultiPassAnalyzer::new(options);
    analyzer.generate_report(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DebtmapConfig;
    use crate::env::RealEnv;

    #[tokio::test]
    async fn test_analyze_multi_pass_effect_simple_code() {
        let env = RealEnv::default();
        let source = "fn main() { println!(\"Hello\"); }".to_string();

        let effect = analyze_multi_pass_effect(source, Language::Rust, PathBuf::from("test.rs"));
        let result = effect.run(&env).await;

        assert!(result.is_ok());
        let analysis = result.unwrap();
        // Just verify we got a valid result with some complexity
        let _ = analysis.raw_complexity.total_complexity;
    }

    #[tokio::test]
    async fn test_analyze_with_options_effect() {
        let env = RealEnv::default();
        let source = "fn main() { if true { } }".to_string();
        let options = MultiPassOptions::default();

        let effect =
            analyze_with_options_effect(source, Language::Rust, PathBuf::from("test.rs"), options);
        let result = effect.run(&env).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_multi_pass_options_effect() {
        let env = RealEnv::default();

        let effect = get_multi_pass_options_effect(Language::Rust);
        let options = effect.run(&env).await.unwrap();

        assert_eq!(options.language, Language::Rust);
        assert!(options.enable_recommendations);
    }

    #[test]
    fn test_backwards_compat_analyze() {
        let source = "fn main() { }";
        let config = DebtmapConfig::default();

        let result = analyze_multi_pass_result(source, Language::Rust, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_detail_level_from_config() {
        // Default config
        let config = DebtmapConfig::default();
        let level = get_detail_level_from_config(&config);
        assert!(matches!(level, DetailLevel::Standard));

        // Custom config
        let config = DebtmapConfig {
            output: Some(crate::config::OutputConfig {
                detail_level: Some("debug".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let level = get_detail_level_from_config(&config);
        assert!(matches!(level, DetailLevel::Debug));
    }
}
