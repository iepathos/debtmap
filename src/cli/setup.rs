//! Setup and initialization functions for CLI
//!
//! This module contains functions for initializing the runtime environment,
//! including thread pool configuration and logging setup.

use anyhow::Result;

/// Rayon thread stack size (8MB for handling deeply nested AST traversals)
const RAYON_STACK_SIZE: usize = 8 * 1024 * 1024;

/// Main thread stack size (16MB for recursive syn::visit patterns)
pub const MAIN_STACK_SIZE: usize = 16 * 1024 * 1024;

/// Configure rayon global thread pool once at startup
pub fn configure_thread_pool(jobs: usize) {
    let mut builder = rayon::ThreadPoolBuilder::new().stack_size(RAYON_STACK_SIZE);

    if jobs > 0 {
        builder = builder.num_threads(jobs);
    }

    if let Err(e) = builder.build_global() {
        // Already configured - this is fine, just ignore
        eprintln!("Note: Thread pool already configured: {}", e);
    }
}

/// Get the number of worker threads to use
pub fn get_worker_count(jobs: usize) -> usize {
    if jobs == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    } else {
        jobs
    }
}

/// Check if running in automation mode (Prodigy workflow)
pub fn is_automation_mode() -> bool {
    std::env::var("PRODIGY_AUTOMATION")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true")
        || std::env::var("PRODIGY_VALIDATION")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true")
}

/// Apply environment setup for analysis (side effect function, I/O at edges)
pub fn apply_environment_setup(no_context_aware: bool) -> Result<()> {
    if !no_context_aware {
        std::env::set_var("DEBTMAP_CONTEXT_AWARE", "true");
    }

    Ok(())
}

/// Print explanation of metric definitions and formulas
pub fn print_metrics_explanation() {
    println!("\n=== Debtmap Metrics Reference ===\n");

    println!("## Metric Categories (Spec 118)\n");
    println!("Debtmap distinguishes between two types of metrics:\n");

    println!("### Measured Metrics");
    println!("These metrics are directly computed from the AST (Abstract Syntax Tree):");
    println!("  - cyclomatic_complexity: Count of decision points (if, match, while, etc.)");
    println!("  - cognitive_complexity: Weighted measure of code understandability");
    println!("  - nesting_depth: Maximum levels of nested control structures");
    println!("  - loc: Lines of code in the function");
    println!("  - parameter_count: Number of function parameters\n");

    println!("### Estimated Metrics");
    println!("These metrics are heuristic estimates, not precise AST measurements:");
    println!("  - est_branches: Estimated execution paths (formula-based approximation)");
    println!("    Formula: max(nesting_depth, 1) x cyclomatic_complexity / 3");
    println!("    Purpose: Estimate test cases needed for branch coverage");
    println!("    Note: This is an ESTIMATE, not a count from the AST\n");

    println!("## Why the Distinction Matters\n");
    println!("- Measured metrics: Precise, repeatable, suitable for thresholds");
    println!("- Estimated metrics: Approximate, useful for prioritization and heuristics");
    println!("- Use cyclomatic_complexity for code quality gates");
    println!("- Use est_branches for estimating testing effort\n");

    println!("## Terminology Change (Spec 118)\n");
    println!("Previously called 'branches', now renamed to 'est_branches' to:");
    println!("  1. Make it clear this is an estimate, not a measured value");
    println!("  2. Avoid confusion with cyclomatic complexity (actual branches)");
    println!("  3. Set accurate expectations for users\n");

    println!("## Example Usage\n");
    println!("  debtmap analyze . --threshold-complexity 15    # Use measured cyclomatic");
    println!(
        "  debtmap analyze . --top 10                      # Uses est_branches for prioritization"
    );
    println!("  debtmap analyze . --lcov coverage.info          # Coverage vs complexity\n");

    println!("For more details, see: https://docs.debtmap.dev/metrics-reference\n");
}

/// Display configuration sources (spec 201)
pub fn show_config_sources() -> Result<()> {
    use crate::config::multi_source::{display_config_sources, load_multi_source_config};

    match load_multi_source_config() {
        Ok(traced) => {
            display_config_sources(&traced);
            Ok(())
        }
        Err(errors) => {
            eprintln!("Error loading configuration:");
            for error in errors {
                eprintln!("  - {}", error);
            }
            Err(anyhow::anyhow!("Configuration loading failed"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_worker_count_explicit() {
        assert_eq!(get_worker_count(4), 4);
        assert_eq!(get_worker_count(8), 8);
    }

    #[test]
    fn test_get_worker_count_auto() {
        let count = get_worker_count(0);
        assert!(count > 0);
    }

    // Note: These tests are inherently flaky due to environment variable races
    // in parallel test execution. We test the logic by checking each env var independently.
    #[test]
    fn test_automation_mode_with_automation_var() {
        // Save current values
        let automation_val = std::env::var("PRODIGY_AUTOMATION").ok();
        let validation_val = std::env::var("PRODIGY_VALIDATION").ok();

        // Test with PRODIGY_AUTOMATION=true
        std::env::set_var("PRODIGY_AUTOMATION", "true");
        std::env::remove_var("PRODIGY_VALIDATION");
        assert!(is_automation_mode());

        // Restore
        match automation_val {
            Some(v) => std::env::set_var("PRODIGY_AUTOMATION", v),
            None => std::env::remove_var("PRODIGY_AUTOMATION"),
        }
        match validation_val {
            Some(v) => std::env::set_var("PRODIGY_VALIDATION", v),
            None => std::env::remove_var("PRODIGY_VALIDATION"),
        }
    }

    #[test]
    fn test_automation_mode_with_validation_var() {
        // Save current values
        let automation_val = std::env::var("PRODIGY_AUTOMATION").ok();
        let validation_val = std::env::var("PRODIGY_VALIDATION").ok();

        // Test with PRODIGY_VALIDATION=true
        std::env::remove_var("PRODIGY_AUTOMATION");
        std::env::set_var("PRODIGY_VALIDATION", "true");
        assert!(is_automation_mode());

        // Restore
        match automation_val {
            Some(v) => std::env::set_var("PRODIGY_AUTOMATION", v),
            None => std::env::remove_var("PRODIGY_AUTOMATION"),
        }
        match validation_val {
            Some(v) => std::env::set_var("PRODIGY_VALIDATION", v),
            None => std::env::remove_var("PRODIGY_VALIDATION"),
        }
    }
}
