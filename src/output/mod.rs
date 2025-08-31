pub mod formatters;
pub mod json;
pub mod markdown;
pub mod terminal;

use crate::{core::AnalysisResults, formatting::FormattingConfig, io, priority, risk};
use anyhow::Result;
use std::path::PathBuf;

pub use formatters::*;
pub use json::*;
pub use markdown::*;
pub use terminal::*;

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
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    output_file: Option<PathBuf>,
    output_format: Option<crate::cli::OutputFormat>,
    _results: &AnalysisResults,
    _coverage_file: Option<&PathBuf>,
    formatting_config: FormattingConfig,
) -> Result<()> {
    output_unified_priorities(
        analysis,
        top,
        tail,
        verbosity,
        output_file,
        output_format,
        formatting_config,
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
    match output_format {
        Some(crate::cli::OutputFormat::Json) => json::output_json(&analysis, output_file),
        Some(crate::cli::OutputFormat::Markdown) => {
            markdown::output_markdown(&analysis, top, tail, verbosity, output_file)
        }
        _ => {
            if is_markdown_file(&output_file) {
                markdown::output_markdown(&analysis, top, tail, verbosity, output_file)
            } else {
                terminal::output_terminal(
                    &analysis,
                    top,
                    tail,
                    verbosity,
                    output_file,
                    formatting_config,
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
