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
    let format = determine_priority_output_format(top, tail);
    let output = priority::formatter::format_priorities_with_config(
        analysis,
        format,
        verbosity,
        formatting_config,
    );

    if let Some(path) = output_file {
        let mut file = fs::File::create(path)?;
        file.write_all(output.as_bytes())?;
    } else {
        println!("{output}");
    }
    Ok(())
}
