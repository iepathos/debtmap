//! Coverage file diagnostic and validation tool
//!
//! This module provides tools for diagnosing and validating LCOV coverage files
//! to help users understand coverage data quality and troubleshoot matching issues.

use crate::risk::lcov::parse_lcov_file;
use anyhow::Result;
use std::path::Path;

/// Diagnose and validate an LCOV coverage file
///
/// Provides statistics, samples, and validation for coverage files to help
/// users understand their coverage data and troubleshoot matching issues.
///
/// # Examples
///
/// ```no_run
/// use debtmap::commands::diagnose_coverage::diagnose_coverage_file;
/// use std::path::Path;
///
/// diagnose_coverage_file(Path::new("coverage.lcov")).unwrap();
/// ```
pub fn diagnose_coverage_file(lcov_path: &Path) -> Result<()> {
    println!("Analyzing coverage file: {}", lcov_path.display());
    println!();

    let lcov_data = parse_lcov_file(lcov_path)?;

    // Basic statistics
    let total_files = lcov_data.functions.len();
    let total_functions: usize = lcov_data.functions.values().map(|funcs| funcs.len()).sum();
    let overall_coverage = lcov_data.get_overall_coverage();

    println!("📊 Coverage Statistics:");
    println!("   Files: {}", total_files);
    println!("   Functions: {}", total_functions);
    println!("   Overall Coverage: {:.1}%", overall_coverage);
    println!();

    // File samples
    println!("📁 Sample Paths (first 10):");
    for (i, path) in lcov_data.functions.keys().take(10).enumerate() {
        println!("   {}. {}", i + 1, path.display());
    }
    if total_files > 10 {
        println!("   ... and {} more", total_files - 10);
    }
    println!();

    // Function samples
    println!("🔧 Sample Functions (first 10):");
    let mut count = 0;
    for (file, funcs) in lcov_data.functions.iter() {
        if count >= 10 {
            break;
        }
        if let Some(func) = funcs.first() {
            println!(
                "   {}. {}::{} ({:.1}%)",
                count + 1,
                file.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                func.name,
                func.coverage_percentage
            );
            count += 1;
        }
    }
    println!();

    // Coverage distribution
    println!("📈 Coverage Distribution:");
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

    println!("   Uncovered (0%): {}", uncovered);
    println!("   Low (1-50%): {}", low);
    println!("   Medium (50-80%): {}", medium);
    println!("   High (80-100%): {}", high);
    println!();

    println!("✓ Coverage file appears valid and can be used with debtmap");
    println!();
    println!("💡 Tips:");
    println!("   • Enable diagnostic mode: DEBTMAP_COVERAGE_DEBUG=1 debtmap analyze ...");
    println!("   • Explain specific function: debtmap explain-coverage --function <name> ...");

    Ok(())
}
