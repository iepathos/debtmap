//! Coverage file diagnostic and validation tool
//!
//! This module provides tools for diagnosing and validating LCOV coverage files
//! to help users understand coverage data quality and troubleshoot matching issues.
//!
//! Architecture follows Stillwater "Pure Core, Imperative Shell" pattern:
//! - Pure functions: data collection and transformation
//! - I/O functions: output formatting at the boundaries

use crate::risk::lcov::{parse_lcov_file, LcovData};
use anyhow::{bail, Result};
use serde::Serialize;
use std::path::Path;

/// Coverage diagnostics result for JSON output (Spec 203 NFR2)
#[derive(Debug, Serialize)]
pub struct CoverageDiagnostics {
    pub file: String,
    pub statistics: Statistics,
    pub sample_paths: Vec<String>,
    pub sample_functions: Vec<FunctionSample>,
    pub distribution: CoverageDistribution,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Statistics {
    pub total_files: usize,
    pub total_functions: usize,
    pub overall_coverage: f64,
}

#[derive(Debug, Serialize)]
pub struct FunctionSample {
    pub file: String,
    pub name: String,
    pub coverage: f64,
}

#[derive(Debug, Serialize)]
pub struct CoverageDistribution {
    pub uncovered: usize,
    pub low: usize,
    pub medium: usize,
    pub high: usize,
}

// ============================================================================
// Pure Core: Data Collection Functions
// ============================================================================

/// Collect statistics from LCOV data (pure function)
fn collect_statistics(lcov_data: &LcovData) -> Statistics {
    Statistics {
        total_files: lcov_data.functions.len(),
        total_functions: lcov_data.functions.values().map(|f| f.len()).sum(),
        overall_coverage: lcov_data.get_overall_coverage(),
    }
}

/// Collect sample paths from LCOV data (pure function)
fn collect_sample_paths(lcov_data: &LcovData, limit: usize) -> Vec<String> {
    lcov_data
        .functions
        .keys()
        .take(limit)
        .map(|p| p.display().to_string())
        .collect()
}

/// Collect sample functions from LCOV data (pure function)
fn collect_sample_functions(lcov_data: &LcovData, limit: usize) -> Vec<FunctionSample> {
    lcov_data
        .functions
        .iter()
        .take(limit)
        .filter_map(|(file, funcs)| {
            funcs.first().map(|func| FunctionSample {
                file: file
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string(),
                name: func.name.clone(),
                coverage: func.coverage_percentage,
            })
        })
        .collect()
}

/// Calculate coverage distribution across all functions (pure function)
fn calculate_distribution(lcov_data: &LcovData) -> CoverageDistribution {
    lcov_data.functions.values().flatten().fold(
        CoverageDistribution {
            uncovered: 0,
            low: 0,
            medium: 0,
            high: 0,
        },
        |mut dist, func| {
            #[allow(clippy::float_cmp)]
            if func.coverage_percentage == 0.0 {
                dist.uncovered += 1;
            } else if func.coverage_percentage < 50.0 {
                dist.low += 1;
            } else if func.coverage_percentage < 80.0 {
                dist.medium += 1;
            } else {
                dist.high += 1;
            }
            dist
        },
    )
}

/// Build complete diagnostics from LCOV data (pure function)
fn build_diagnostics(lcov_path: &Path, lcov_data: &LcovData) -> CoverageDiagnostics {
    let statistics = collect_statistics(lcov_data);
    let distribution = calculate_distribution(lcov_data);
    let suggestions = generate_suggestions(
        distribution.uncovered,
        statistics.total_functions,
        statistics.total_files,
    );

    CoverageDiagnostics {
        file: lcov_path.display().to_string(),
        statistics,
        sample_paths: collect_sample_paths(lcov_data, 10),
        sample_functions: collect_sample_functions(lcov_data, 10),
        distribution,
        suggestions,
    }
}

/// Generate actionable suggestions based on coverage patterns (Spec 203 FR4)
fn generate_suggestions(
    uncovered: usize,
    total_functions: usize,
    total_files: usize,
) -> Vec<String> {
    if total_functions == 0 {
        return vec![
            "No functions found in LCOV file. Check that the coverage tool generated valid output."
                .to_string(),
        ];
    }

    let uncovered_percent = (uncovered as f64 / total_functions as f64) * 100.0;

    let mut suggestions: Vec<String> = Vec::new();

    if uncovered_percent > 50.0 {
        suggestions.push(format!(
            "Many functions ({}%) show 0% coverage. If this seems wrong, check that paths in LCOV match your project structure.",
            uncovered_percent as usize
        ));
        suggestions.push(
            "Use DEBTMAP_COVERAGE_DEBUG=1 to see detailed matching logs and identify path mismatches.".to_string(),
        );
    } else if uncovered_percent > 20.0 {
        suggestions.push(
            "Moderate number of functions with 0% coverage. Check function name matching with explain-coverage.".to_string(),
        );
    }

    if total_files < 5 {
        suggestions.push(
            "Very few files in coverage report. Ensure coverage tool is scanning the entire project.".to_string(),
        );
    }

    if suggestions.is_empty() {
        suggestions.push(
            "Coverage data looks good! Most functions have coverage information.".to_string(),
        );
    }

    suggestions
}

// ============================================================================
// Imperative Shell: I/O Output Functions
// ============================================================================

/// Output diagnostics as JSON (I/O boundary)
fn output_json(diagnostics: &CoverageDiagnostics) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(diagnostics)?);
    Ok(())
}

/// Output diagnostics as formatted text (I/O boundary)
fn output_text(diagnostics: &CoverageDiagnostics) {
    println!("Analyzing coverage file: {}", diagnostics.file);
    println!();

    print_statistics(&diagnostics.statistics);
    print_sample_paths(
        &diagnostics.sample_paths,
        diagnostics.statistics.total_files,
    );
    print_sample_functions(&diagnostics.sample_functions);
    print_distribution(&diagnostics.distribution);
    print_suggestions(&diagnostics.suggestions);
    print_tips();
}

fn print_statistics(stats: &Statistics) {
    println!("📊 Coverage Statistics:");
    println!("   Files: {}", stats.total_files);
    println!("   Functions: {}", stats.total_functions);
    println!("   Overall Coverage: {:.1}%", stats.overall_coverage);
    println!();
}

fn print_sample_paths(paths: &[String], total_files: usize) {
    println!("📁 Sample Paths (first 10):");
    for (i, path) in paths.iter().enumerate() {
        println!("   {}. {}", i + 1, path);
    }
    if total_files > 10 {
        println!("   ... and {} more", total_files - 10);
    }
    println!();
}

fn print_sample_functions(samples: &[FunctionSample]) {
    println!("🔧 Sample Functions (first 10):");
    for (i, sample) in samples.iter().enumerate() {
        println!(
            "   {}. {}::{} ({:.1}%)",
            i + 1,
            sample.file,
            sample.name,
            sample.coverage
        );
    }
    println!();
}

fn print_distribution(dist: &CoverageDistribution) {
    println!("📈 Coverage Distribution:");
    println!("   Uncovered (0%): {}", dist.uncovered);
    println!("   Low (1-50%): {}", dist.low);
    println!("   Medium (50-80%): {}", dist.medium);
    println!("   High (80-100%): {}", dist.high);
    println!();
    println!("✓ Coverage file appears valid and can be used with debtmap");
}

fn print_suggestions(suggestions: &[String]) {
    if !suggestions.is_empty() {
        println!();
        println!("💡 Suggestions:");
        for suggestion in suggestions {
            println!("   • {}", suggestion);
        }
    }
}

fn print_tips() {
    println!();
    println!("Additional Tips:");
    println!("   • Enable diagnostic mode: DEBTMAP_COVERAGE_DEBUG=1 debtmap analyze ...");
    println!("   • Explain specific function: debtmap explain-coverage --function <name> ...");
}

// ============================================================================
// Public API: Thin Orchestration Layer
// ============================================================================

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
    if format != "text" && format != "json" {
        bail!("Invalid format '{}'. Must be 'text' or 'json'", format);
    }

    let lcov_data = parse_lcov_file(lcov_path)?;
    let diagnostics = build_diagnostics(lcov_path, &lcov_data);

    match format {
        "json" => output_json(&diagnostics),
        _ => {
            output_text(&diagnostics);
            Ok(())
        }
    }
}
