//! Project validation command module.
//!
//! This module validates a project against configured quality thresholds.
//! It follows the Stillwater philosophy of separating pure computation
//! from I/O side effects.
//!
//! # Module Structure
//!
//! - `types` - Configuration and result data structures
//! - `thresholds` - Pure validation logic for checking metrics
//! - `analysis` - Unified analysis computation
//! - `output` - I/O operations (timing display, reporting, warnings)
//!
//! # Architecture
//!
//! The main entry point `validate_project` is a thin I/O shell that:
//! 1. Sets up parallel processing (I/O: env vars)
//! 2. Runs project analysis (I/O: file system)
//! 3. Gets risk insights if coverage provided (I/O)
//! 4. Generates reports if requested (I/O)
//! 5. Delegates to pure validation functions
//! 6. Reports results (I/O: console output)

mod analysis;
mod output;
mod thresholds;
pub mod types;

// Re-export public types
pub use types::{ValidateConfig, ValidationDetails};

use crate::commands::analyze;
use crate::core::{AnalysisResults, Language};
use crate::formatting::FormattingConfig;
use crate::io;
use crate::progress::{ProgressConfig, ProgressManager};
use crate::utils::risk_analyzer;
use crate::{config, risk};
use analysis::{calculate_unified_analysis, read_parallel_options_from_env, ValidationAnalysisOptions};
use anyhow::Result;
use output::{
    display_timing_information, generate_report_if_requested, print_parallel_status,
    print_validation_failure, print_validation_success, warn_deprecated_thresholds,
};
use thresholds::{find_deprecated_thresholds, validate_basic, validate_with_risk};

// =============================================================================
// Public API
// =============================================================================

/// I/O Shell: Main entry point for project validation.
///
/// This orchestrates the validation process:
/// 1. Configures parallel processing
/// 2. Sets up progress display (same as analyze command)
/// 3. Analyzes the project
/// 4. Cleans up progress display
/// 5. Optionally generates risk insights
/// 6. Validates against thresholds
/// 7. Reports results
pub fn validate_project(config: ValidateConfig) -> Result<()> {
    let complexity_threshold = 10;
    let duplication_threshold = 50;

    // I/O: Configure parallel processing
    let parallel_enabled = !config.no_parallel;
    let jobs = config.jobs;
    setup_parallel_processing(parallel_enabled, jobs, config.verbosity);

    // I/O: Set up progress manager (same as analyze command)
    setup_progress_manager(config.verbosity);

    // I/O: Run project analysis
    let results = analyze::analyze_project(
        config.path.clone(),
        vec![Language::Rust, Language::Python],
        complexity_threshold,
        duplication_threshold,
        parallel_enabled,
        FormattingConfig::default(),
    )?;

    // I/O: Get risk insights if coverage provided
    let risk_insights = get_risk_insights(&config, &results)?;

    // I/O: Generate report if requested
    generate_report_if_requested(&config, &results, &risk_insights)?;

    // Perform validation and report results (includes unified analysis with call graph)
    validate_and_report(&config, &results, &risk_insights)
}

// =============================================================================
// I/O Setup Functions
// =============================================================================

/// I/O: Set up parallel processing environment.
fn setup_parallel_processing(enabled: bool, jobs: usize, verbosity: u8) {
    if enabled {
        std::env::set_var("DEBTMAP_PARALLEL", "true");
        print_parallel_status(enabled, jobs, verbosity);
    }

    if jobs > 0 {
        std::env::set_var("DEBTMAP_JOBS", jobs.to_string());
    }
}

/// I/O: Initialize progress manager with TUI support.
///
/// This uses the same progress setup as the analyze command to ensure
/// consistent progress display during analysis.
fn setup_progress_manager(verbosity: u8) {
    let quiet = std::env::var("DEBTMAP_QUIET").is_ok();
    let progress_config = ProgressConfig::from_env(quiet, verbosity);
    ProgressManager::init_global(progress_config);

    // Start TUI rendering if available
    if let Some(manager) = ProgressManager::global() {
        manager.tui_start_stage(0);
    }
}

/// I/O: Clean up progress display after analysis.
fn cleanup_progress() {
    if let Some(manager) = ProgressManager::global() {
        manager.tui_set_progress(1.0);
        manager.tui_cleanup();
    }
    io::progress::AnalysisProgress::with_global(|p| p.finish());
}

/// I/O: Get risk insights based on configuration.
fn get_risk_insights(
    config: &ValidateConfig,
    results: &AnalysisResults,
) -> Result<Option<risk::RiskInsight>> {
    match (&config.coverage_file, config.enable_context) {
        (Some(lcov_path), _) => risk_analyzer::analyze_risk_with_coverage(
            results,
            lcov_path,
            &config.path,
            config.enable_context,
            config.context_providers.clone(),
            config.disable_context.clone(),
        ),
        (None, true) => risk_analyzer::analyze_risk_without_coverage(
            results,
            config.enable_context,
            config.context_providers.clone(),
            config.disable_context.clone(),
            &config.path,
        ),
        _ => Ok(None),
    }
}

// =============================================================================
// Validation Orchestration
// =============================================================================

/// Orchestrate validation and report results.
fn validate_and_report(
    config: &ValidateConfig,
    results: &AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
) -> Result<()> {
    // I/O: Parse lcov if available
    let lcov_data = config
        .coverage_file
        .as_ref()
        .and_then(|path| risk::lcov::parse_lcov_file(path).ok());

    // I/O: Get thresholds and warn about deprecated ones
    let validation_thresholds = config::get_validation_thresholds();
    let deprecated = find_deprecated_thresholds(&validation_thresholds);
    warn_deprecated_thresholds(&deprecated);

    // Calculate unified analysis metrics with same options as analyze command
    let parallel_options = read_parallel_options_from_env();
    let options = ValidationAnalysisOptions {
        parallel: parallel_options.parallel,
        jobs: parallel_options.jobs,
        enable_context: config.enable_context,
        context_providers: config.context_providers.clone(),
        disable_context: config.disable_context.clone(),
    };
    let unified = calculate_unified_analysis(results, config.coverage_file.as_ref(), &options);

    // I/O: Clean up progress display before printing results
    cleanup_progress();

    // I/O: Display timing if verbosity enabled
    display_timing_information(&unified, config.verbosity);

    // Extract metrics from unified analysis
    let total_debt_score = unified.total_debt_score as u32;
    let debt_density = unified.debt_density;

    // Pure: Perform validation
    let (pass, details) = perform_validation(
        results,
        risk_insights,
        lcov_data.as_ref(),
        total_debt_score,
        debt_density,
        &validation_thresholds,
        config.max_debt_density,
    );

    // I/O: Report results
    if pass {
        print_validation_success(&details, config.verbosity);
        Ok(())
    } else {
        print_validation_failure(&details, risk_insights, config.verbosity);
        anyhow::bail!("Validation failed")
    }
}

/// Pure: Dispatch to appropriate validation function based on available data.
fn perform_validation(
    results: &AnalysisResults,
    risk_insights: &Option<risk::RiskInsight>,
    lcov_data: Option<&risk::lcov::LcovData>,
    total_debt_score: u32,
    debt_density: f64,
    thresholds: &config::ValidationThresholds,
    max_debt_density_override: Option<f64>,
) -> (bool, ValidationDetails) {
    match risk_insights {
        Some(insights) => {
            let coverage_percentage = lcov_data
                .map(|lcov| lcov.get_overall_coverage())
                .unwrap_or(0.0);

            validate_with_risk(
                results,
                insights,
                coverage_percentage,
                total_debt_score,
                debt_density,
                thresholds,
                max_debt_density_override,
            )
        }
        None => validate_basic(
            results,
            total_debt_score,
            debt_density,
            thresholds,
            max_debt_density_override,
        ),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests;
