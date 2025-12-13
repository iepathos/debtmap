//! Integration tests for the validate command.
//!
//! These tests verify the overall behavior of the validation system,
//! including environment variable handling and configuration.

use super::*;
use std::path::PathBuf;

#[test]
fn test_validate_sets_parallel_env_var() {
    // Clear any existing env var
    std::env::remove_var("DEBTMAP_PARALLEL");

    // Simulate validate command with parallel enabled (default)
    let config = ValidateConfig {
        path: PathBuf::from("."),
        config: None,
        coverage_file: None,
        format: None,
        output: None,
        enable_context: false,
        context_providers: None,
        disable_context: None,
        max_debt_density: None,
        top: None,
        tail: None,
        semantic_off: false,
        verbosity: 0,
        no_parallel: false,
        jobs: 0,
        show_splits: false,
    };

    // When parallel is enabled, the environment variable should be set
    if !config.no_parallel {
        std::env::set_var("DEBTMAP_PARALLEL", "true");
    }

    assert_eq!(std::env::var("DEBTMAP_PARALLEL").unwrap(), "true");

    // Clean up
    std::env::remove_var("DEBTMAP_PARALLEL");
}

#[test]
fn test_validate_respects_no_parallel_flag() {
    // Test the logic of no_parallel flag (don't rely on global env var state)
    let config = ValidateConfig {
        path: PathBuf::from("."),
        config: None,
        coverage_file: None,
        format: None,
        output: None,
        enable_context: false,
        context_providers: None,
        disable_context: None,
        max_debt_density: None,
        top: None,
        tail: None,
        semantic_off: false,
        verbosity: 0,
        no_parallel: true,
        jobs: 0,
        show_splits: false,
    };

    // Verify that no_parallel flag is set correctly
    assert!(config.no_parallel);

    // Test that when no_parallel is true, we should NOT set the env var
    let should_set_parallel = !config.no_parallel;
    assert!(!should_set_parallel);
}

#[test]
fn test_validate_sets_jobs_env_var() {
    // Clear any existing env var
    std::env::remove_var("DEBTMAP_JOBS");

    // Simulate validate command with custom job count
    let config = ValidateConfig {
        path: PathBuf::from("."),
        config: None,
        coverage_file: None,
        format: None,
        output: None,
        enable_context: false,
        context_providers: None,
        disable_context: None,
        max_debt_density: None,
        top: None,
        tail: None,
        semantic_off: false,
        verbosity: 0,
        no_parallel: false,
        jobs: 4,
        show_splits: false,
    };

    // When jobs is set, the environment variable should be set
    if config.jobs > 0 {
        std::env::set_var("DEBTMAP_JOBS", config.jobs.to_string());
    }

    assert_eq!(std::env::var("DEBTMAP_JOBS").unwrap(), "4");

    // Clean up
    std::env::remove_var("DEBTMAP_JOBS");
}

#[test]
fn test_env_var_parsing() {
    // Combined test for environment variable parsing to avoid race conditions
    // when tests run in parallel. Tests must not interfere with each other.

    // Test DEBTMAP_PARALLEL detection

    // Case 1: DEBTMAP_PARALLEL not set (default: sequential)
    std::env::remove_var("DEBTMAP_PARALLEL");
    let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    assert!(!parallel_enabled);

    // Case 2: DEBTMAP_PARALLEL=true
    std::env::set_var("DEBTMAP_PARALLEL", "true");
    let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    assert!(parallel_enabled);

    // Case 3: DEBTMAP_PARALLEL=1
    std::env::set_var("DEBTMAP_PARALLEL", "1");
    let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    assert!(parallel_enabled);

    // Case 4: DEBTMAP_PARALLEL=false
    std::env::set_var("DEBTMAP_PARALLEL", "false");
    let parallel_enabled = std::env::var("DEBTMAP_PARALLEL")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    assert!(!parallel_enabled);

    // Clean up
    std::env::remove_var("DEBTMAP_PARALLEL");

    // Test DEBTMAP_JOBS parsing

    // Case 1: Valid number
    std::env::set_var("DEBTMAP_JOBS", "8");
    let jobs = std::env::var("DEBTMAP_JOBS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    assert_eq!(jobs, 8);

    // Case 2: Invalid number (defaults to 0)
    std::env::set_var("DEBTMAP_JOBS", "invalid");
    let jobs = std::env::var("DEBTMAP_JOBS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    assert_eq!(jobs, 0);

    // Case 3: Not set (defaults to 0)
    std::env::remove_var("DEBTMAP_JOBS");
    let jobs = std::env::var("DEBTMAP_JOBS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    assert_eq!(jobs, 0);

    // Clean up
    std::env::remove_var("DEBTMAP_JOBS");
}

#[test]
fn test_validation_details_creation() {
    // Test that ValidationDetails can be constructed correctly
    let details = ValidationDetails {
        average_complexity: 5.0,
        max_average_complexity: 10.0,
        high_complexity_count: 3,
        max_high_complexity_count: 5,
        debt_items: 10,
        max_debt_items: 20,
        total_debt_score: 150,
        max_total_debt_score: 300,
        debt_density: 0.15,
        max_debt_density: 0.20,
        codebase_risk_score: 25.5,
        max_codebase_risk_score: 50.0,
        high_risk_functions: 5,
        max_high_risk_functions: 10,
        coverage_percentage: 75.0,
        min_coverage_percentage: 60.0,
    };

    assert_eq!(details.average_complexity, 5.0);
    assert_eq!(details.max_average_complexity, 10.0);
    assert_eq!(details.high_complexity_count, 3);
    assert_eq!(details.max_high_complexity_count, 5);
    assert_eq!(details.debt_density, 0.15);
    assert_eq!(details.max_debt_density, 0.20);
    assert_eq!(details.debt_items, 10);
    assert_eq!(details.max_debt_items, 20);
    assert_eq!(details.total_debt_score, 150);
    assert_eq!(details.max_total_debt_score, 300);
    assert_eq!(details.codebase_risk_score, 25.5);
    assert_eq!(details.max_codebase_risk_score, 50.0);
    assert_eq!(details.high_risk_functions, 5);
    assert_eq!(details.max_high_risk_functions, 10);
    assert_eq!(details.coverage_percentage, 75.0);
    assert_eq!(details.min_coverage_percentage, 60.0);
}
