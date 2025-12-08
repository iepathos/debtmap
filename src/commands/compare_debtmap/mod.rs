//! Debtmap comparison and validation module.
//!
//! This module compares before and after debtmap analysis results to validate
//! that technical debt has been addressed. It follows the Stillwater philosophy
//! of separating pure computation from I/O side effects.
//!
//! # Module Structure
//!
//! - `types` - All data types used in comparison
//! - `io` - I/O operations (file reading/writing, console output)
//! - `analysis` - Pure analysis functions for identifying changes
//! - `messages` - Pure message builder functions
//! - `scoring` - Pure scoring functions
//!
//! # Architecture
//!
//! The main entry point `compare_debtmaps` is a thin I/O shell that:
//! 1. Reads input files (I/O)
//! 2. Delegates to pure validation functions
//! 3. Writes output (I/O)
//! 4. Prints summary (I/O)

mod analysis;
mod io;
mod messages;
mod scoring;
pub mod types;

// Re-export public types
pub use types::{AnalysisSummary, CompareConfig, DebtmapJsonInput, GapDetail, ValidationResult};

use analysis::{create_summary, identify_all_changes};
use anyhow::Result;
use io::{load_both_debtmaps, print_summary, read_automation_mode, write_validation_result};
use messages::{build_all_gaps, build_all_improvement_messages, build_all_issue_messages};
use scoring::{calculate_improvement_score, determine_status};
use types::DebtmapJsonInput as Input;

// =============================================================================
// Public API
// =============================================================================

/// I/O Shell: Main entry point - orchestrates I/O and delegates to pure validation
pub fn compare_debtmaps(config: CompareConfig) -> Result<()> {
    let is_automation = read_automation_mode();

    if !is_automation {
        println!("Loading debtmap data from before and after states...");
    }

    let (before, after) = load_both_debtmaps(&config)?;
    let result = perform_validation(&before, &after)?; // Pure!

    write_validation_result(&config.output_path, &result)?;

    if !is_automation {
        print_summary(&result);
    }

    Ok(())
}

// =============================================================================
// Pure Validation Core
// =============================================================================

/// Pure: Perform validation by comparing before and after states
fn perform_validation(before: &Input, after: &Input) -> Result<ValidationResult> {
    let before_summary = create_summary(before);
    let after_summary = create_summary(after);
    let changes = identify_all_changes(before, after);

    let improvements = build_all_improvement_messages(&changes.resolved, &changes.improved);
    let remaining_issues =
        build_all_issue_messages(&changes.unchanged_critical, &changes.new_items);
    let gaps = build_all_gaps(&changes.unchanged_critical, &changes.new_items);

    let completion = calculate_improvement_score(
        &changes.resolved,
        &changes.improved,
        &changes.new_items,
        &changes.unchanged_critical,
        &before_summary,
        &after_summary,
    );
    let status = determine_status(
        completion,
        &changes.new_items,
        &before_summary,
        &after_summary,
    );

    Ok(ValidationResult {
        completion_percentage: completion,
        status,
        improvements,
        remaining_issues,
        gaps,
        before_summary,
        after_summary,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests;
