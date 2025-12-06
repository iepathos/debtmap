// Helper module to convert subprocess-based tests to library API tests
// This module provides utilities for tests that were migrated from Command::new("cargo")

use anyhow::Result;
use debtmap::config::DebtmapConfig;
use debtmap::core::DebtType;
use debtmap::io::walker::find_project_files_with_config;
use serde_json::Value;
use std::path::Path;

/// Simulates running `cargo run -- analyze <path> --format json`
/// Returns JSON-like structure similar to CLI output
pub fn analyze_as_json(path: &Path) -> Result<Value> {
    let _config = DebtmapConfig::default();
    let results = super::analyze_file_directly(path)?;

    // Convert results to JSON format similar to CLI output
    let json = serde_json::json!({
        "path": path.to_str().unwrap_or(""),
        "items": results.technical_debt.items.iter().map(|item| {
            serde_json::json!({
                "debt_type": format!("{:?}", item.debt_type),
                "file_path": item.file.to_str().unwrap_or(""),
                "line": item.line,
                "column": item.column,
                "message": item.message,
                "priority": format!("{:?}", item.priority),
            })
        }).collect::<Vec<_>>(),
        "total_debt_items": results.technical_debt.items.len(),
    });

    Ok(json)
}

/// Simulates running `cargo run -- analyze <path>` with text output
/// Returns string output similar to terminal format
pub fn analyze_as_text(path: &Path, context_aware: bool) -> Result<String> {
    // Set context-aware flag
    if context_aware {
        std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
    } else {
        std::env::set_var("DEBTMAP_CONTEXT_AWARE", "false");
    }

    let results = super::analyze_file_directly(path)?;

    // Clean up env var
    std::env::remove_var("DEBTMAP_CONTEXT_AWARE");

    // Format results as text similar to CLI output
    let mut output = String::new();

    for item in &results.technical_debt.items {
        let type_str = match &item.debt_type {
            DebtType::Complexity { .. } => "COMPLEXITY",
            DebtType::Todo { .. } => "TODO",
            DebtType::CodeSmell { .. } => "SMELL",
            DebtType::Duplication { .. } => "DUPLICATION",
            _ => "DEBT",
        };

        output.push_str(&format!(
            "{}: {} at {}:{}\n",
            type_str,
            item.message,
            item.file.display(),
            item.line
        ));
    }

    if results.technical_debt.items.is_empty() {
        output.push_str("No issues found.\n");
    } else {
        output.push_str(&format!(
            "\nTOTAL DEBT SCORE: {}\n",
            results.technical_debt.items.len()
        ));
    }

    Ok(output)
}

/// Analyze multiple files in a directory
pub fn analyze_directory(dir: &Path, context_aware: bool) -> Result<String> {
    let config = DebtmapConfig::default();

    // Set context-aware flag
    if context_aware {
        std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
    } else {
        std::env::set_var("DEBTMAP_CONTEXT_AWARE", "false");
    }

    let files = find_project_files_with_config(dir, vec![], &config)?;
    let mut total_output = String::new();
    let mut total_debt_items = 0;

    for file in files.iter().take(10) {
        // Limit for performance in tests
        if let Ok(results) = super::analyze_file_directly(file) {
            total_debt_items += results.technical_debt.items.len();

            for item in &results.technical_debt.items {
                let type_str = match &item.debt_type {
                    DebtType::Complexity { .. } => "COMPLEXITY",
                    _ => "DEBT",
                };

                total_output.push_str(&format!(
                    "{}: {} at {}:{}\n",
                    type_str,
                    item.message,
                    item.file.display(),
                    item.line
                ));
            }
        }
    }

    // Clean up env var
    std::env::remove_var("DEBTMAP_CONTEXT_AWARE");

    total_output.push_str(&format!("\nTOTAL DEBT ITEMS: {}\n", total_debt_items));
    Ok(total_output)
}
