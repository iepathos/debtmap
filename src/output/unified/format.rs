//! Numeric precision functions for output formatting (spec 230)
//!
//! Provides rounding utilities to eliminate floating-point noise in output
//! and assertion helpers for validating output invariants.

// ============================================================================
// Numeric Precision Functions (spec 230)
// ============================================================================

/// Round score to 2 decimal places for clean output
///
/// Removes floating-point noise like 1.5697499999999998 -> 1.57
#[inline]
pub fn round_score(score: f64) -> f64 {
    (score * 100.0).round() / 100.0
}

/// Round percentage/ratio to 4 decimal places
///
/// Used for coverage, confidence, and other 0-1 ratios
#[inline]
pub fn round_ratio(ratio: f64) -> f64 {
    (ratio * 10000.0).round() / 10000.0
}

// ============================================================================
// Invariant Assertions (spec 230)
// ============================================================================

/// Maximum reasonable score value
pub const MAX_SCORE: f64 = 1000.0;

/// Assert score invariants: non-negative and within reasonable bounds
#[inline]
pub fn assert_score_invariants(score: f64, context: &str) {
    debug_assert!(
        score >= 0.0,
        "Score must be non-negative: {} = {}",
        context,
        score
    );
    debug_assert!(
        score <= MAX_SCORE,
        "Score exceeds maximum ({}): {} = {}",
        MAX_SCORE,
        context,
        score
    );
}

/// Assert ratio invariants: value in 0.0..=1.0
#[inline]
pub fn assert_ratio_invariants(ratio: f64, context: &str) {
    debug_assert!(
        (0.0..=1.0).contains(&ratio),
        "{} must be in range [0.0, 1.0]: {}",
        context,
        ratio
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_score_removes_noise() {
        // Typical floating-point noise
        assert_eq!(round_score(1.5697499999999998), 1.57);
        assert_eq!(round_score(42.999999999999), 43.0);
        assert_eq!(round_score(10.0000000001), 10.0);
    }

    #[test]
    fn test_round_score_preserves_valid_values() {
        assert_eq!(round_score(0.0), 0.0);
        assert_eq!(round_score(100.0), 100.0);
        assert_eq!(round_score(42.15), 42.15);
    }

    #[test]
    fn test_round_ratio_removes_noise() {
        // Typical floating-point noise in ratios
        assert_eq!(round_ratio(0.9999999999), 1.0);
        assert_eq!(round_ratio(0.0000000001), 0.0);
        assert_eq!(round_ratio(0.7499999999), 0.75);
    }

    #[test]
    fn test_round_ratio_preserves_valid_values() {
        assert_eq!(round_ratio(0.0), 0.0);
        assert_eq!(round_ratio(1.0), 1.0);
        assert_eq!(round_ratio(0.5), 0.5);
        assert_eq!(round_ratio(0.1234), 0.1234);
    }
}
