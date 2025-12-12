//! Coverage matching diagnostics (Spec 203).
//!
//! This module provides global statistics for debug mode to help diagnose
//! coverage matching issues. When `DEBTMAP_COVERAGE_DEBUG=1` is set,
//! detailed statistics are tracked and printed at the end of analysis.
//!
//! # Stillwater Philosophy
//!
//! While this module contains global state (atomic counters), the state is
//! isolated and used only for diagnostic purposes. The counters are at the
//! boundary of the system - "flowing water" that tracks the system's behavior.
//!
//! # Usage
//!
//! Enable diagnostics by setting the environment variable:
//! ```bash
//! DEBTMAP_COVERAGE_DEBUG=1 debtmap analyze
//! ```
//!
//! At the end of analysis, summary statistics are printed:
//! ```text
//! [COVERAGE] ═══════════════════════════════════════════════════
//! [COVERAGE] Match Statistics Summary
//! [COVERAGE] ═══════════════════════════════════════════════════
//! [COVERAGE]   Total functions: 1234
//! [COVERAGE]   Matched: 1100 (89.1%)
//! [COVERAGE]   Unmatched (0%): 134 (10.9%)
//! [COVERAGE] ═══════════════════════════════════════════════════
//! ```

use std::sync::atomic::{AtomicUsize, Ordering};

// Global statistics for diagnostic mode (Spec 203 FR3)
static COVERAGE_MATCH_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
static COVERAGE_MATCH_SUCCESS: AtomicUsize = AtomicUsize::new(0);
static COVERAGE_MATCH_ZERO: AtomicUsize = AtomicUsize::new(0);

/// Print aggregate coverage matching statistics (Spec 203 FR3).
///
/// Called at end of analysis when `DEBTMAP_COVERAGE_DEBUG=1` to show
/// summary of match success rates. Only prints if any matches were attempted.
///
/// # Example Output
///
/// ```text
/// [COVERAGE] ═══════════════════════════════════════════════════
/// [COVERAGE] Match Statistics Summary
/// [COVERAGE] ═══════════════════════════════════════════════════
/// [COVERAGE]   Total functions: 1234
/// [COVERAGE]   Matched: 1100 (89.1%)
/// [COVERAGE]   Unmatched (0%): 134 (10.9%)
/// [COVERAGE] ═══════════════════════════════════════════════════
/// ```
pub fn print_coverage_statistics() {
    let attempts = COVERAGE_MATCH_ATTEMPTS.load(Ordering::Relaxed);
    if attempts == 0 {
        return; // No matches attempted
    }

    let success = COVERAGE_MATCH_SUCCESS.load(Ordering::Relaxed);
    let zero = COVERAGE_MATCH_ZERO.load(Ordering::Relaxed);
    let success_rate = (success as f64 / attempts as f64) * 100.0;
    let zero_rate = (zero as f64 / attempts as f64) * 100.0;

    eprintln!();
    eprintln!("[COVERAGE] ═══════════════════════════════════════════════════");
    eprintln!("[COVERAGE] Match Statistics Summary");
    eprintln!("[COVERAGE] ═══════════════════════════════════════════════════");
    eprintln!("[COVERAGE]   Total functions: {}", attempts);
    eprintln!("[COVERAGE]   Matched: {} ({:.1}%)", success, success_rate);
    eprintln!("[COVERAGE]   Unmatched (0%): {} ({:.1}%)", zero, zero_rate);
    eprintln!("[COVERAGE] ═══════════════════════════════════════════════════");
}

/// Track a match attempt.
///
/// Called when a coverage lookup is attempted. This increments the
/// total attempts counter used for statistics.
pub fn track_match_attempt() {
    COVERAGE_MATCH_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
}

/// Track a successful match.
///
/// Called when a coverage lookup successfully finds coverage data
/// with a non-zero percentage.
pub fn track_match_success() {
    COVERAGE_MATCH_SUCCESS.fetch_add(1, Ordering::Relaxed);
}

/// Track a zero-coverage match.
///
/// Called when a coverage lookup either fails to find the function
/// or finds it with 0% coverage.
pub fn track_match_zero() {
    COVERAGE_MATCH_ZERO.fetch_add(1, Ordering::Relaxed);
}

/// Reset all statistics counters.
///
/// Primarily used in tests to ensure clean state between test runs.
#[cfg(test)]
pub fn reset_statistics() {
    COVERAGE_MATCH_ATTEMPTS.store(0, Ordering::Relaxed);
    COVERAGE_MATCH_SUCCESS.store(0, Ordering::Relaxed);
    COVERAGE_MATCH_ZERO.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_match_attempt() {
        reset_statistics();

        track_match_attempt();
        track_match_attempt();
        track_match_attempt();

        let attempts = COVERAGE_MATCH_ATTEMPTS.load(Ordering::Relaxed);
        assert_eq!(attempts, 3);

        reset_statistics();
    }

    #[test]
    fn test_track_match_success() {
        reset_statistics();

        track_match_success();
        track_match_success();

        let success = COVERAGE_MATCH_SUCCESS.load(Ordering::Relaxed);
        assert_eq!(success, 2);

        reset_statistics();
    }

    #[test]
    fn test_track_match_zero() {
        reset_statistics();

        track_match_zero();

        let zero = COVERAGE_MATCH_ZERO.load(Ordering::Relaxed);
        assert_eq!(zero, 1);

        reset_statistics();
    }

    #[test]
    fn test_reset_statistics() {
        track_match_attempt();
        track_match_success();
        track_match_zero();

        reset_statistics();

        assert_eq!(COVERAGE_MATCH_ATTEMPTS.load(Ordering::Relaxed), 0);
        assert_eq!(COVERAGE_MATCH_SUCCESS.load(Ordering::Relaxed), 0);
        assert_eq!(COVERAGE_MATCH_ZERO.load(Ordering::Relaxed), 0);
    }
}
