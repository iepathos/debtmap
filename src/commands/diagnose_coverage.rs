//! Coverage file diagnostic and validation tool
//!
//! This module provides tools for diagnosing and validating LCOV coverage files
//! to help users understand coverage data quality and troubleshoot matching issues.

use crate::risk::lcov::parse_lcov_file;
use anyhow::{bail, Result};
use serde::Serialize;
use std::path::Path;

/// Coverage diagnostics result for JSON output (Spec 203 NFR2)
#[derive(Debug, Serialize)]
struct CoverageDiagnostics {
    file: String,
    statistics: Statistics,
    sample_paths: Vec<String>,
    sample_functions: Vec<FunctionSample>,
    distribution: CoverageDistribution,
    suggestions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Statistics {
    total_files: usize,
    total_functions: usize,
    overall_coverage: f64,
}

#[derive(Debug, Serialize)]
struct FunctionSample {
    file: String,
    name: String,
    coverage: f64,
}

#[derive(Debug, Serialize)]
struct CoverageDistribution {
    uncovered: usize,
    low: usize,
    medium: usize,
    high: usize,
}

/// Diagnose and validate an LCOV coverage file
///
/// Provides statistics, samples, and validation for coverage files to help
/// users understand their coverage data and troubleshoot matching issues.
///
/// # Arguments
///
/// * `lcov_path` - Path to the LCOV coverage file
/// * `format` - Output format: "text" or "json" (Spec 203 NFR2)
///
/// # Examples
///
/// ```no_run
/// use debtmap::commands::diagnose_coverage::diagnose_coverage_file;
/// use std::path::Path;
///
/// diagnose_coverage_file(Path::new("coverage.lcov"), "text").unwrap();
/// diagnose_coverage_file(Path::new("coverage.lcov"), "json").unwrap();
/// ```
pub fn diagnose_coverage_file(lcov_path: &Path, format: &str) -> Result<()> {
    // Validate format parameter
    if format != "text" && format != "json" {
        bail!("Invalid format '{}'. Must be 'text' or 'json'", format);
    }

    let lcov_data = parse_lcov_file(lcov_path)?;

    // Collect statistics
    let total_files = lcov_data.functions.len();
    let total_functions: usize = lcov_data.functions.values().map(|funcs| funcs.len()).sum();
    let overall_coverage = lcov_data.get_overall_coverage();

    // Collect sample paths
    let sample_paths: Vec<String> = lcov_data
        .functions
        .keys()
        .take(10)
        .map(|p| p.display().to_string())
        .collect();

    // Collect sample functions
    let mut sample_functions = Vec::new();
    for (file, funcs) in lcov_data.functions.iter().take(10) {
        if let Some(func) = funcs.first() {
            sample_functions.push(FunctionSample {
                file: file
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string(),
                name: func.name.clone(),
                coverage: func.coverage_percentage,
            });
        }
    }

    // Calculate coverage distribution
    let mut uncovered = 0;
    let mut low = 0; // 1-50%
    let mut medium = 0; // 50-80%
    let mut high = 0; // 80-100%

    for funcs in lcov_data.functions.values() {
        for func in funcs {
            if func.coverage_percentage == 0.0 {
                uncovered += 1;
            } else if func.coverage_percentage < 50.0 {
                low += 1;
            } else if func.coverage_percentage < 80.0 {
                medium += 1;
            } else {
                high += 1;
            }
        }
    }

    // Generate suggestions (Spec 203 FR4)
    let suggestions = generate_suggestions(uncovered, total_functions, total_files);

    // Output in requested format
    if format == "json" {
        let diagnostics = CoverageDiagnostics {
            file: lcov_path.display().to_string(),
            statistics: Statistics {
                total_files,
                total_functions,
                overall_coverage,
            },
            sample_paths,
            sample_functions,
            distribution: CoverageDistribution {
                uncovered,
                low,
                medium,
                high,
            },
            suggestions,
        };
        println!("{}", serde_json::to_string_pretty(&diagnostics)?);
    } else {
        // Text output
        println!("Analyzing coverage file: {}", lcov_path.display());
        println!();
        println!("📊 Coverage Statistics:");
        println!("   Files: {}", total_files);
        println!("   Functions: {}", total_functions);
        println!("   Overall Coverage: {:.1}%", overall_coverage);
        println!();

        println!("📁 Sample Paths (first 10):");
        for (i, path) in sample_paths.iter().enumerate() {
            println!("   {}. {}", i + 1, path);
        }
        if total_files > 10 {
            println!("   ... and {} more", total_files - 10);
        }
        println!();

        println!("🔧 Sample Functions (first 10):");
        for (i, sample) in sample_functions.iter().enumerate() {
            println!(
                "   {}. {}::{} ({:.1}%)",
                i + 1,
                sample.file,
                sample.name,
                sample.coverage
            );
        }
        println!();

        println!("📈 Coverage Distribution:");
        println!("   Uncovered (0%): {}", uncovered);
        println!("   Low (1-50%): {}", low);
        println!("   Medium (50-80%): {}", medium);
        println!("   High (80-100%): {}", high);
        println!();

        println!("✓ Coverage file appears valid and can be used with debtmap");

        // Print suggestions (Spec 203 FR4)
        if !suggestions.is_empty() {
            println!();
            println!("💡 Suggestions:");
            for suggestion in &suggestions {
                println!("   • {}", suggestion);
            }
        }

        println!();
        println!("Additional Tips:");
        println!("   • Enable diagnostic mode: DEBTMAP_COVERAGE_DEBUG=1 debtmap analyze ...");
        println!("   • Explain specific function: debtmap explain-coverage --function <name> ...");
    }

    Ok(())
}

/// Generate actionable suggestions based on coverage patterns (Spec 203 FR4)
fn generate_suggestions(
    uncovered: usize,
    total_functions: usize,
    total_files: usize,
) -> Vec<String> {
    let mut suggestions = Vec::new();

    if total_functions == 0 {
        suggestions.push(
            "No functions found in LCOV file. Check that the coverage tool generated valid output."
                .to_string(),
        );
        return suggestions;
    }

    let uncovered_percent = (uncovered as f64 / total_functions as f64) * 100.0;

    if uncovered_percent > 50.0 {
        suggestions.push(format!(
            "Many functions ({}%) show 0% coverage. If this seems wrong, check that paths in LCOV match your project structure.",
            uncovered_percent as usize
        ));
        suggestions.push(
            "Use DEBTMAP_COVERAGE_DEBUG=1 to see detailed matching logs and identify path mismatches.".to_string(),
        );
    }

    if total_files < 5 {
        suggestions.push(
            "Very few files in coverage report. Ensure coverage tool is scanning the entire project.".to_string(),
        );
    }

    if uncovered_percent > 20.0 && uncovered_percent <= 50.0 {
        suggestions.push(
            "Moderate number of functions with 0% coverage. Check function name matching with explain-coverage.".to_string(),
        );
    }

    if suggestions.is_empty() {
        suggestions.push(
            "Coverage data looks good! Most functions have coverage information.".to_string(),
        );
    }

    suggestions
}
