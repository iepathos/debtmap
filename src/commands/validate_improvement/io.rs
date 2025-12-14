//! I/O operations for validation (the imperative shell).
//!
//! This module handles all file system interactions, keeping
//! side effects at the boundary per Stillwater philosophy.

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::comparison::types::ComparisonResult;

use super::formatters::{format_json, format_markdown, format_terminal};
use super::types::{OutputFormat, ValidationResult};

/// I/O: Load comparison file from disk.
pub fn load_comparison(path: &Path) -> Result<ComparisonResult> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read comparison file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse comparison JSON from: {}", path.display()))
}

/// I/O: Load previous validation result from disk.
pub fn load_previous_validation(path: &Path) -> Result<ValidationResult> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read validation file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse validation JSON from: {}", path.display()))
}

/// I/O: Write validation result to disk.
pub fn write_validation_result(
    path: &Path,
    result: &ValidationResult,
    format: OutputFormat,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let output = match format {
        OutputFormat::Json => format_json(result)?,
        OutputFormat::Terminal => format_terminal(result),
        OutputFormat::Markdown => format_markdown(result),
    };

    fs::write(path, output)
        .with_context(|| format!("Failed to write validation result to: {}", path.display()))?;

    Ok(())
}

/// I/O: Print validation summary to console.
pub fn print_validation_summary(result: &ValidationResult) {
    println!(
        "\nValidation complete: {:.1}% improvement",
        result.completion_percentage
    );
    println!("Status: {}", result.status);
}
