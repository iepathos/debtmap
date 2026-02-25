//! I/O operations for validation output.
//!
//! This module contains all I/O-related functions for the validate command,
//! including timing display, report generation, and warning output.
//! Following Stillwater philosophy, these are the "water" functions that
//! perform side effects at the boundaries.

use super::types::{ValidateConfig, ValidationDetails};
use crate::cli;
use crate::core::AnalysisResults;
use crate::output;
use crate::priority::UnifiedAnalysis;
use crate::risk;
use crate::utils::validation_printer;
use anyhow::Result;

/// I/O: Display timing information for analysis phases.
///
/// Only displays if verbosity > 0 and quiet mode is not enabled.
pub fn display_timing_information(unified: &UnifiedAnalysis, verbosity: u8) {
    if std::env::var("DEBTMAP_QUIET").is_ok() || verbosity == 0 {
        return;
    }

    let Some(timings) = &unified.timings else {
        return;
    };

    eprintln!("\nTiming information:");
    eprintln!("  Total analysis time: {:?}", timings.total);

    if verbosity >= 1 {
        eprintln!("  - Call graph building: {:?}", timings.call_graph_building);
        eprintln!("  - Trait resolution: {:?}", timings.trait_resolution);
        eprintln!("  - Coverage loading: {:?}", timings.coverage_loading);

        let optional_phases = [
            ("Data flow", timings.data_flow_creation),
            ("Purity", timings.purity_analysis),
            ("Test detection", timings.test_detection),
            ("Debt aggregation", timings.debt_aggregation),
            ("Function analysis", timings.function_analysis),
            ("File analysis", timings.file_analysis),
            ("Sorting", timings.sorting),
        ];

        for (n, d) in optional_phases {
            if d.as_millis() > 0 {
                eprintln!("  - {}: {:?}", n, d);
            }
        }
    }
    eprintln!();
}

/// I/O: Print deprecation warning for deprecated threshold settings.
pub fn warn_deprecated_thresholds(deprecated: &[&str]) {
    if deprecated.is_empty() {
        return;
    }

    eprintln!("\n[WARN] DEPRECATION WARNING:");
    eprintln!("   The following validation thresholds are deprecated:");
    for metric in deprecated {
        eprintln!("   - {}", metric);
    }
    eprintln!("\n   These scale-dependent metrics will be removed in v1.0.");
    eprintln!("   Please migrate to density-based validation:");
    eprintln!("     - Use 'max_debt_density' instead of absolute counts");
    eprintln!("     - Density metrics remain stable as your codebase grows");
    eprintln!("     - See: https://github.com/your-repo/debtmap#density-based-validation\n");
}

/// I/O: Print parallel processing status message.
pub fn print_parallel_status(enabled: bool, jobs: usize, verbosity: u8) {
    if !enabled || verbosity == 0 {
        return;
    }

    let thread_msg = if jobs == 0 {
        "all available cores".to_string()
    } else {
        format!("{} threads", jobs)
    };
    eprintln!("Building call graph using {}...", thread_msg);
}

/// Pure: Determine output format from config.
pub fn determine_output_format(config: &ValidateConfig) -> Option<cli::OutputFormat> {
    config
        .format
        .or(config.output.as_ref().map(|_| cli::OutputFormat::Terminal))
}

/// I/O: Generate and write report if output is requested.
pub fn generate_report_if_requested(
    config: &ValidateConfig,
    results: &AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
) -> Result<()> {
    determine_output_format(config)
        .map(|format| {
            output::output_results_with_risk(
                results.clone(),
                risk_insights.clone(),
                format.into(),
                config.output.clone(),
            )
        })
        .unwrap_or(Ok(()))
}

/// I/O: Print validation success message.
pub fn print_validation_success(details: &ValidationDetails, verbosity: u8) {
    validation_printer::print_validation_success(details, verbosity);
}

/// I/O: Print validation failure message with details.
pub fn print_validation_failure(
    details: &ValidationDetails,
    risk_insights: &Option<risk::RiskInsight>,
    verbosity: u8,
) {
    validation_printer::print_validation_failure_with_details(details, risk_insights, verbosity);
}
