use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::risk::coverage_index::CoverageIndex;
use crate::risk::lcov::parse_lcov_file;

#[derive(Debug, Clone, Copy)]
pub enum DebugFormat {
    Text,
    Json,
}

#[derive(Debug)]
pub struct ExplainCoverageConfig {
    pub path: PathBuf,
    pub coverage_file: PathBuf,
    pub function_name: String,
    pub file_path: Option<PathBuf>,
    pub verbose: bool,
    pub format: DebugFormat,
}

#[derive(Debug, Serialize, Deserialize)]
struct StrategyAttempt {
    strategy: String,
    success: bool,
    matched_function: Option<String>,
    matched_file: Option<String>,
    coverage_percentage: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExplainCoverageResult {
    function_name: String,
    file_path: Option<String>,
    coverage_found: bool,
    coverage_percentage: Option<f64>,
    matched_by_strategy: Option<String>,
    attempts: Vec<StrategyAttempt>,
    available_functions: Vec<String>,
    available_files: Vec<String>,
}

/// Explain how coverage detection works for a specific function
pub fn explain_coverage(config: ExplainCoverageConfig) -> Result<()> {
    // Parse LCOV data
    let lcov_data = parse_lcov_file(&config.coverage_file).context("Failed to parse LCOV file")?;

    // Build coverage index
    let coverage_index = CoverageIndex::from_coverage(&lcov_data);

    let mut result = ExplainCoverageResult {
        function_name: config.function_name.clone(),
        file_path: config.file_path.as_ref().map(|p| p.display().to_string()),
        coverage_found: false,
        coverage_percentage: None,
        matched_by_strategy: None,
        attempts: Vec::new(),
        available_functions: Vec::new(),
        available_files: Vec::new(),
    };

    // Collect available functions and files for debugging
    for (file, functions) in lcov_data.functions.iter() {
        result.available_files.push(file.display().to_string());
        for func in functions.iter() {
            result
                .available_functions
                .push(format!("{}::{}", file.display(), func.name));
        }
    }

    // If file path is provided, try with that file
    if let Some(file_path) = &config.file_path {
        // Try exact match first
        let attempt = try_exact_match(&coverage_index, file_path, &config.function_name);
        let success = attempt.success;
        result.attempts.push(attempt);

        if success {
            let last_attempt = result.attempts.last().unwrap();
            result.matched_by_strategy = Some(last_attempt.strategy.clone());
            result.coverage_found = true;
            result.coverage_percentage = last_attempt.coverage_percentage;
        }

        // Try path matching strategies
        if !result.coverage_found && config.verbose {
            let attempts = try_path_strategies(
                &coverage_index,
                file_path,
                &config.function_name,
                &lcov_data,
            );
            for attempt in attempts {
                let success = attempt.success;
                result.attempts.push(attempt);
                if success && !result.coverage_found {
                    let last_attempt = result.attempts.last().unwrap();
                    result.matched_by_strategy = Some(last_attempt.strategy.clone());
                    result.coverage_found = true;
                    result.coverage_percentage = last_attempt.coverage_percentage;
                }
            }
        }
    } else {
        // No file path provided, search all files
        let attempts = search_all_files(&coverage_index, &config.function_name, &lcov_data);
        for attempt in attempts {
            let success = attempt.success;
            result.attempts.push(attempt);
            if success && !result.coverage_found {
                let last_attempt = result.attempts.last().unwrap();
                result.matched_by_strategy = Some(last_attempt.strategy.clone());
                result.coverage_found = true;
                result.coverage_percentage = last_attempt.coverage_percentage;
            }
        }
    }

    // Output results
    match config.format {
        DebugFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        DebugFormat::Text => {
            output_text_format(&result, config.verbose);
        }
    }

    Ok(())
}

fn try_exact_match(
    coverage_index: &CoverageIndex,
    file_path: &Path,
    function_name: &str,
) -> StrategyAttempt {
    let coverage = coverage_index.get_function_coverage(file_path, function_name);

    StrategyAttempt {
        strategy: "exact_match".to_string(),
        success: coverage.is_some(),
        matched_function: if coverage.is_some() {
            Some(function_name.to_string())
        } else {
            None
        },
        matched_file: if coverage.is_some() {
            Some(file_path.display().to_string())
        } else {
            None
        },
        coverage_percentage: coverage,
    }
}

fn try_path_strategies(
    _coverage_index: &CoverageIndex,
    file_path: &PathBuf,
    function_name: &str,
    lcov_data: &crate::risk::lcov::LcovData,
) -> Vec<StrategyAttempt> {
    let mut attempts = Vec::new();

    // Strategy: Suffix matching
    for (lcov_file, functions) in &lcov_data.functions {
        if file_path.ends_with(lcov_file) || lcov_file.ends_with(file_path) {
            // Search for exact function name match
            if let Some(coverage_data) = functions.iter().find(|f| f.name == function_name) {
                attempts.push(StrategyAttempt {
                    strategy: "suffix_match".to_string(),
                    success: true,
                    matched_function: Some(function_name.to_string()),
                    matched_file: Some(lcov_file.display().to_string()),
                    coverage_percentage: Some(coverage_data.coverage_percentage / 100.0),
                });
                return attempts; // Return early on first success
            }

            // Try method name matching
            for coverage_data in functions {
                if coverage_data.normalized.method_name == function_name {
                    attempts.push(StrategyAttempt {
                        strategy: "method_name_match".to_string(),
                        success: true,
                        matched_function: Some(coverage_data.name.clone()),
                        matched_file: Some(lcov_file.display().to_string()),
                        coverage_percentage: Some(coverage_data.coverage_percentage / 100.0),
                    });
                    return attempts; // Return early on first success
                }
            }
        }
    }

    // Strategy: Normalized path equality (component-based)
    let query_components = crate::risk::path_normalization::normalize_path_components(file_path);
    for (lcov_file, functions) in &lcov_data.functions {
        let lcov_components = crate::risk::path_normalization::normalize_path_components(lcov_file);
        if lcov_components == query_components {
            if let Some(coverage_data) = functions.iter().find(|f| f.name == function_name) {
                attempts.push(StrategyAttempt {
                    strategy: "normalized_path_match".to_string(),
                    success: true,
                    matched_function: Some(function_name.to_string()),
                    matched_file: Some(lcov_file.display().to_string()),
                    coverage_percentage: Some(coverage_data.coverage_percentage / 100.0),
                });
                return attempts;
            }
        }
    }

    attempts.push(StrategyAttempt {
        strategy: "all_path_strategies".to_string(),
        success: false,
        matched_function: None,
        matched_file: None,
        coverage_percentage: None,
    });

    attempts
}

fn search_all_files(
    _coverage_index: &CoverageIndex,
    function_name: &str,
    lcov_data: &crate::risk::lcov::LcovData,
) -> Vec<StrategyAttempt> {
    let mut attempts = Vec::new();

    // Search for exact function name matches in all files
    for (lcov_file, functions) in &lcov_data.functions {
        if let Some(coverage_data) = functions.iter().find(|f| f.name == function_name) {
            attempts.push(StrategyAttempt {
                strategy: "global_function_name_match".to_string(),
                success: true,
                matched_function: Some(function_name.to_string()),
                matched_file: Some(lcov_file.display().to_string()),
                coverage_percentage: Some(coverage_data.coverage_percentage / 100.0),
            });
            return attempts; // Return on first match
        }
    }

    // Try method name matching across all files
    for (lcov_file, functions) in &lcov_data.functions {
        for coverage_data in functions {
            if coverage_data.normalized.method_name == function_name {
                attempts.push(StrategyAttempt {
                    strategy: "global_method_name_match".to_string(),
                    success: true,
                    matched_function: Some(coverage_data.name.clone()),
                    matched_file: Some(lcov_file.display().to_string()),
                    coverage_percentage: Some(coverage_data.coverage_percentage / 100.0),
                });
                return attempts; // Return on first match
            }
        }
    }

    attempts.push(StrategyAttempt {
        strategy: "global_search".to_string(),
        success: false,
        matched_function: None,
        matched_file: None,
        coverage_percentage: None,
    });

    attempts
}

fn output_text_format(result: &ExplainCoverageResult, verbose: bool) {
    println!("Coverage Detection Explanation");
    println!("==============================");
    println!();
    println!("Function: {}", result.function_name);
    if let Some(ref file) = result.file_path {
        println!("File: {}", file);
    }
    println!();

    if result.coverage_found {
        println!("✓ Coverage Found!");
        println!(
            "  Strategy: {}",
            result.matched_by_strategy.as_ref().unwrap()
        );
        println!(
            "  Coverage: {:.1}%",
            result.coverage_percentage.unwrap() * 100.0
        );
        println!();
    } else {
        println!("✗ Coverage Not Found");
        println!();
    }

    if verbose || !result.coverage_found {
        println!("Matching Attempts:");
        println!("------------------");
        for attempt in &result.attempts {
            let status = if attempt.success { "✓" } else { "✗" };
            println!("  {} {}", status, attempt.strategy);
            if attempt.success {
                if let Some(ref matched_file) = attempt.matched_file {
                    println!("      File: {}", matched_file);
                }
                if let Some(ref matched_func) = attempt.matched_function {
                    println!("      Function: {}", matched_func);
                }
                if let Some(coverage) = attempt.coverage_percentage {
                    println!("      Coverage: {:.1}%", coverage * 100.0);
                }
            }
        }
        println!();
    }

    if !result.coverage_found {
        println!(
            "Available Files in LCOV ({} total):",
            result.available_files.len()
        );
        println!("----------------------------------");
        for (i, file) in result.available_files.iter().take(10).enumerate() {
            println!("  {}. {}", i + 1, file);
        }
        if result.available_files.len() > 10 {
            println!("  ... and {} more", result.available_files.len() - 10);
        }
        println!();

        println!(
            "Available Functions in LCOV ({} total):",
            result.available_functions.len()
        );
        println!("--------------------------------------");

        // Show functions that partially match the query
        let matching_functions: Vec<_> = result
            .available_functions
            .iter()
            .filter(|f| {
                f.to_lowercase()
                    .contains(&result.function_name.to_lowercase())
            })
            .collect();

        if !matching_functions.is_empty() {
            println!("  Functions containing '{}':", result.function_name);
            for func in matching_functions.iter().take(10) {
                println!("    - {}", func);
            }
            if matching_functions.len() > 10 {
                println!("    ... and {} more", matching_functions.len() - 10);
            }
        } else {
            println!("  No functions found containing '{}'", result.function_name);
            println!();
            println!("  First 10 available functions:");
            for func in result.available_functions.iter().take(10) {
                println!("    - {}", func);
            }
            if result.available_functions.len() > 10 {
                println!("    ... and {} more", result.available_functions.len() - 10);
            }
        }
    }
}

// Make normalize_path public for use in this module
mod coverage_index_helpers {
    use std::path::{Path, PathBuf};

    pub fn normalize_path(path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);
        PathBuf::from(cleaned)
    }
}

// Re-export for use in coverage_index
pub use coverage_index_helpers::normalize_path;
