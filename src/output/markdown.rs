use crate::priority;
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn output_markdown(
    analysis: &priority::UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    verbosity: u8,
    output_file: Option<PathBuf>,
) -> Result<()> {
    let limit = calculate_markdown_limit(top, tail);
    let output = priority::format_priorities_markdown(analysis, limit, verbosity);

    if let Some(path) = output_file {
        let mut file = fs::File::create(path)?;
        file.write_all(output.as_bytes())?;
    } else {
        println!("{output}");
    }
    Ok(())
}

fn calculate_markdown_limit(top: Option<usize>, _tail: Option<usize>) -> usize {
    top.unwrap_or(10)
}
