//! I/O operations for debtmap comparison.
//!
//! This module contains all side-effecting operations: file reading,
//! writing, environment variable access, and console output.

use super::types::{CompareConfig, DebtmapJsonInput, ValidationResult};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

// =============================================================================
// Environment I/O
// =============================================================================

/// I/O: Read automation mode from environment variables
pub fn read_automation_mode() -> bool {
    std::env::var("PRODIGY_AUTOMATION")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true")
        || std::env::var("PRODIGY_VALIDATION")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true")
}

// =============================================================================
// File I/O
// =============================================================================

/// I/O: Load both debtmap files
pub fn load_both_debtmaps(config: &CompareConfig) -> Result<(DebtmapJsonInput, DebtmapJsonInput)> {
    let before = load_debtmap(&config.before_path)?;
    let after = load_debtmap(&config.after_path)?;
    Ok((before, after))
}

/// I/O: Load single debtmap file
pub fn load_debtmap(path: &Path) -> Result<DebtmapJsonInput> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read debtmap file: {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse debtmap JSON from: {}", path.display()))
}

/// I/O: Write validation result to file
pub fn write_validation_result(path: &Path, result: &ValidationResult) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(result)?;
    fs::write(path, json)
        .with_context(|| format!("Failed to write validation result to: {}", path.display()))?;

    Ok(())
}

// =============================================================================
// Console Output
// =============================================================================

/// I/O: Print validation summary to console
pub fn print_summary(result: &ValidationResult) {
    println!("\n=== Debtmap Validation Results ===");
    println!("Completion: {:.1}%", result.completion_percentage);
    println!("Status: {}", result.status);

    if !result.improvements.is_empty() {
        println!("\nImprovements:");
        for improvement in &result.improvements {
            println!("  ✓ {}", improvement);
        }
    }

    if !result.remaining_issues.is_empty() {
        println!("\nRemaining Issues:");
        for issue in &result.remaining_issues {
            println!("  ✗ {}", issue);
        }
    }

    println!(
        "\nBefore: {} items (avg score: {:.1})",
        result.before_summary.total_items, result.before_summary.average_score
    );
    println!(
        "After: {} items (avg score: {:.1})",
        result.after_summary.total_items, result.after_summary.average_score
    );
}
