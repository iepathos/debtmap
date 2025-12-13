//! Data types for the validate command.
//!
//! This module contains all configuration and result types used
//! in project validation. These are pure data structures with no
//! behavior attached.

use crate::cli;
use std::path::PathBuf;

/// Configuration for project validation.
///
/// Contains all settings needed to run validation, including
/// paths, thresholds, and output options.
#[derive(Debug)]
pub struct ValidateConfig {
    pub path: PathBuf,
    pub config: Option<PathBuf>,
    pub coverage_file: Option<PathBuf>,
    pub format: Option<cli::OutputFormat>,
    pub output: Option<PathBuf>,
    pub enable_context: bool,
    pub context_providers: Option<Vec<String>>,
    pub disable_context: Option<Vec<String>>,
    pub max_debt_density: Option<f64>,
    pub top: Option<usize>,
    pub tail: Option<usize>,
    pub semantic_off: bool,
    pub verbosity: u8,
    pub no_parallel: bool,
    pub jobs: usize,
    /// Show detailed module split recommendations for god objects (Spec 208)
    pub show_splits: bool,
}

/// Details of validation results for reporting.
///
/// Contains all metrics checked during validation along with
/// their threshold values. Used for reporting pass/fail status
/// and generating detailed feedback.
pub struct ValidationDetails {
    pub average_complexity: f64,
    pub max_average_complexity: f64,
    pub high_complexity_count: usize,
    pub max_high_complexity_count: usize,
    pub debt_items: usize,
    pub max_debt_items: usize,
    pub total_debt_score: u32,
    pub max_total_debt_score: u32,
    pub debt_density: f64,
    pub max_debt_density: f64,
    pub codebase_risk_score: f64,
    pub max_codebase_risk_score: f64,
    pub high_risk_functions: usize,
    pub max_high_risk_functions: usize,
    pub coverage_percentage: f64,
    pub min_coverage_percentage: f64,
}

/// Result of threshold validation.
///
/// Contains the pass/fail status and detailed breakdown of
/// which checks passed or failed.
pub struct ThresholdCheckResult {
    /// Overall pass/fail status
    pub passed: bool,
    /// Individual check results for detailed reporting
    pub checks: Vec<CheckResult>,
}

/// Result of a single threshold check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: &'static str,
    pub passed: bool,
    pub actual: f64,
    pub threshold: f64,
    pub is_deprecated: bool,
}

impl ThresholdCheckResult {
    /// Create a new result from individual check results.
    pub fn from_checks(checks: Vec<CheckResult>) -> Self {
        let passed = checks.iter().all(|c| c.passed);
        Self { passed, checks }
    }
}
