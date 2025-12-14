//! Validation of technical debt improvements.
//!
//! This module validates that technical debt improvements have been made
//! by analyzing comparison output from `debtmap compare`.
//!
//! # Module Structure
//!
//! - `types` - Configuration and result data structures
//! - `scoring` - Pure scoring logic (composite scores, status)
//! - `processors` - Pure processors for targets, regressions, health
//! - `formatters` - Pure output formatting (JSON, Terminal, Markdown)
//! - `io` - I/O operations (file loading, writing, console)
//!
//! # Architecture
//!
//! Follows Stillwater philosophy: "Pure Core, Imperative Shell".
//! The main entry point `validate_improvement` is a thin I/O shell that:
//! 1. Loads inputs (I/O)
//! 2. Delegates to pure validation functions
//! 3. Writes outputs (I/O)
//!
//! # Scoring Algorithm
//!
//! The validation score is a composite of three components:
//! - Target improvement (50%): Did the specific target item improve?
//! - Project health (30%): Did overall project debt decrease?
//! - No regressions (20%): Were new critical items introduced?
//!
//! # Examples
//!
//! ```no_run
//! use debtmap::commands::validate_improvement::*;
//! use std::path::PathBuf;
//!
//! let config = ValidateImprovementConfig {
//!     comparison_path: PathBuf::from("comparison.json"),
//!     output_path: PathBuf::from("validation.json"),
//!     previous_validation: None,
//!     threshold: 75.0,
//!     format: OutputFormat::Json,
//!     quiet: false,
//! };
//!
//! validate_improvement(config).unwrap();
//! ```

mod formatters;
mod io;
mod processors;
mod scoring;
pub mod types;

// Re-export public types
pub use types::{OutputFormat, ValidateImprovementConfig, ValidationResult};

use anyhow::Result;
use std::collections::HashMap;

use crate::comparison::types::ComparisonResult;

use io::{
    load_comparison, load_previous_validation, print_validation_summary, write_validation_result,
};
use processors::{process_project_health, process_regressions, process_target_improvements};
use scoring::{
    build_project_summary, build_target_summary, calculate_composite_score,
    calculate_trend_analysis, determine_status,
};
use types::ValidationResult as VResult;

// =============================================================================
// Public API
// =============================================================================

/// I/O Shell: Main entry point for validation.
///
/// This orchestrates the validation process:
/// 1. Loads comparison and previous validation (I/O)
/// 2. Performs pure validation calculations
/// 3. Writes results to disk (I/O)
/// 4. Prints summary to console (I/O)
pub fn validate_improvement(config: ValidateImprovementConfig) -> Result<()> {
    // I/O: Load inputs
    let comparison = load_comparison(&config.comparison_path)?;
    let previous = config
        .previous_validation
        .as_ref()
        .map(|path| load_previous_validation(path))
        .transpose()?;

    // Pure: Perform calculations
    let result = validate_improvement_internal(&comparison, previous.as_ref())?;

    // I/O: Write outputs
    write_validation_result(&config.output_path, &result, config.format)?;

    // I/O: Print to console (if not quiet)
    if !config.quiet {
        print_validation_summary(&result);
    }

    Ok(())
}

// =============================================================================
// Pure Validation Logic
// =============================================================================

/// Pure: Internal validation logic.
///
/// Composes pure processors and scoring functions to produce
/// a complete validation result.
fn validate_improvement_internal(
    comparison: &ComparisonResult,
    previous: Option<&VResult>,
) -> Result<VResult> {
    // Process each component
    let target_result = process_target_improvements(comparison);
    let regression_result = process_regressions(comparison);
    let health_result = process_project_health(comparison);

    // Merge improvements and issues
    let improvements = merge_improvements(&target_result.improvements, &health_result.improvements);
    let remaining_issues = merge_issues(
        &target_result.remaining_issues,
        &regression_result.remaining_issues,
    );
    let gaps = merge_gaps(target_result.gaps, regression_result.gaps);

    // Calculate composite score
    let improvement_score = calculate_composite_score(
        target_result.component_score,
        health_result.component_score,
        regression_result.component_score,
    );

    // Build summaries
    let status = determine_status(improvement_score);
    let target_summary = build_target_summary(&comparison.target_item);
    let project_summary = build_project_summary(comparison);

    // Calculate trend analysis if previous validation is provided
    let trend_analysis = previous.map(|prev| calculate_trend_analysis(prev, improvement_score));
    let attempt_number = previous.map(|prev| prev.attempt_number.unwrap_or(1) + 1);

    Ok(VResult {
        completion_percentage: improvement_score,
        status,
        improvements,
        remaining_issues,
        gaps,
        target_summary,
        project_summary,
        trend_analysis,
        attempt_number,
    })
}

// =============================================================================
// Pure Helper Functions
// =============================================================================

/// Pure: Merge improvement lists.
fn merge_improvements(target: &[String], health: &[String]) -> Vec<String> {
    target.iter().chain(health.iter()).cloned().collect()
}

/// Pure: Merge issue lists.
fn merge_issues(target: &[String], regression: &[String]) -> Vec<String> {
    target.iter().chain(regression.iter()).cloned().collect()
}

/// Pure: Merge gap maps.
fn merge_gaps(
    mut target: HashMap<String, types::GapDetail>,
    regression: HashMap<String, types::GapDetail>,
) -> HashMap<String, types::GapDetail> {
    target.extend(regression);
    target
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_improvements() {
        let target = vec!["A".to_string()];
        let health = vec!["B".to_string(), "C".to_string()];

        let merged = merge_improvements(&target, &health);

        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0], "A");
        assert_eq!(merged[1], "B");
        assert_eq!(merged[2], "C");
    }

    #[test]
    fn test_merge_gaps() {
        use types::GapDetail;

        let mut target = HashMap::new();
        target.insert(
            "target".to_string(),
            GapDetail {
                description: "Target gap".to_string(),
                location: "a.rs".to_string(),
                severity: "high".to_string(),
                suggested_fix: "Fix".to_string(),
                score_before: None,
                score_after: None,
                current_score: None,
            },
        );

        let mut regression = HashMap::new();
        regression.insert(
            "regression".to_string(),
            GapDetail {
                description: "Regression gap".to_string(),
                location: "b.rs".to_string(),
                severity: "high".to_string(),
                suggested_fix: "Fix".to_string(),
                score_before: None,
                score_after: None,
                current_score: None,
            },
        );

        let merged = merge_gaps(target, regression);

        assert_eq!(merged.len(), 2);
        assert!(merged.contains_key("target"));
        assert!(merged.contains_key("regression"));
    }
}
