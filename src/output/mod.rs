pub mod evidence_formatter;
pub mod formatters;
pub mod json;
pub mod markdown;
pub mod pattern_analysis;
pub mod pattern_formatter;
pub mod terminal;
pub mod unified;

use crate::{core::AnalysisResults, formatting::FormattingConfig, io, priority, risk};
use anyhow::Result;
use std::path::PathBuf;

pub struct OutputConfig {
    pub top: Option<usize>,
    pub tail: Option<usize>,
    pub summary: bool,
    pub verbosity: u8,
    pub output_file: Option<PathBuf>,
    pub output_format: Option<crate::cli::OutputFormat>,
    pub json_format: crate::cli::JsonFormat,
    pub formatting_config: FormattingConfig,
}

pub use evidence_formatter::*;
pub use formatters::*;
pub use json::*;
pub use markdown::*;
pub use pattern_analysis::*;
pub use pattern_formatter::*;
pub use terminal::*;
pub use unified::*;

pub fn output_results_with_risk(
    results: AnalysisResults,
    risk_insights: Option<risk::RiskInsight>,
    format: io::output::OutputFormat,
    output_file: Option<PathBuf>,
) -> Result<()> {
    match output_file {
        Some(path) => {
            let content = format_results_to_string(&results, &risk_insights, format)?;
            io::write_file(&path, &content)?;
        }
        None => {
            let mut writer = io::output::create_writer(format);
            writer.write_results(&results)?;
            if let Some(insights) = risk_insights {
                writer.write_risk_insights(&insights)?;
            }
        }
    }
    Ok(())
}

pub fn output_unified_priorities_with_config(
    analysis: priority::UnifiedAnalysis,
    config: OutputConfig,
    _results: &AnalysisResults,
    _coverage_file: Option<&PathBuf>,
) -> Result<()> {
    output_unified_priorities_with_summary(
        analysis,
        config.top,
        config.tail,
        config.summary,
        config.verbosity,
        config.output_file,
        config.output_format,
        config.json_format,
        config.formatting_config,
    )
}

pub fn output_unified_priorities(
    analysis: priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    output_file: Option<PathBuf>,
    output_format: Option<crate::cli::OutputFormat>,
    formatting_config: FormattingConfig,
) -> Result<()> {
    output_unified_priorities_with_summary(
        analysis,
        top,
        tail,
        false, // default to detailed format
        verbosity,
        output_file,
        output_format,
        crate::cli::JsonFormat::Legacy, // default to legacy for backward compatibility
        formatting_config,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn output_unified_priorities_with_summary(
    analysis: priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    summary: bool,
    verbosity: u8,
    output_file: Option<PathBuf>,
    output_format: Option<crate::cli::OutputFormat>,
    json_format: crate::cli::JsonFormat,
    formatting_config: FormattingConfig,
) -> Result<()> {
    match output_format {
        Some(crate::cli::OutputFormat::Json) => {
            let include_scoring_details = verbosity >= 2;
            json::output_json_with_format(
                &analysis,
                top,
                tail,
                output_file,
                json_format,
                include_scoring_details,
            )
        }
        Some(crate::cli::OutputFormat::Markdown) => {
            markdown::output_markdown(&analysis, top, tail, verbosity, output_file)
        }
        _ => {
            if is_markdown_file(&output_file) {
                markdown::output_markdown(&analysis, top, tail, verbosity, output_file)
            } else {
                terminal::output_terminal_with_mode(
                    &analysis,
                    top,
                    tail,
                    verbosity,
                    output_file,
                    formatting_config,
                    summary,
                )
            }
        }
    }
}

fn is_markdown_file(output_file: &Option<PathBuf>) -> bool {
    output_file
        .as_ref()
        .and_then(|p| p.extension())
        .map(|ext| ext == "md")
        .unwrap_or(false)
}
