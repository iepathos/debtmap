use crate::priority;
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn output_json(
    analysis: &priority::UnifiedAnalysis,
    output_file: Option<PathBuf>,
) -> Result<()> {
    let json = serde_json::to_string_pretty(analysis)?;
    if let Some(path) = output_file {
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
    } else {
        println!("{json}");
    }
    Ok(())
}
