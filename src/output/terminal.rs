use super::formatters::determine_priority_output_format;
use crate::{formatting::FormattingConfig, priority};
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn output_terminal(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    output_file: Option<PathBuf>,
    formatting_config: FormattingConfig,
) -> Result<()> {
    output_terminal_with_mode(
        analysis,
        top,
        tail,
        verbosity,
        output_file,
        formatting_config,
        false,
    )
}

pub fn output_terminal_with_mode(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    output_file: Option<PathBuf>,
    formatting_config: FormattingConfig,
    summary_mode: bool,
) -> Result<()> {
    let output = if summary_mode {
        // Use tiered summary display
        let limit = top.unwrap_or(10);
        priority::formatter::format_summary_terminal(analysis, limit, verbosity)
    } else {
        // Use detailed display
        let format = determine_priority_output_format(top, tail);
        priority::formatter::format_priorities_with_config(
            analysis,
            format,
            verbosity,
            formatting_config,
        )
    };

    if let Some(path) = output_file {
        if let Some(parent) = path.parent() {
            crate::io::ensure_dir(parent)?;
        }
        let mut file = fs::File::create(path)?;
        file.write_all(output.as_bytes())?;
    } else {
        println!("{output}");
    }
    Ok(())
}
