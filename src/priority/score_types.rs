//! Type-safe score scales for debt scoring system.
//!
//! This module provides newtype wrappers for different score scales used
//! throughout the analysis. By encoding the scale in the type system, we
//! prevent bugs caused by mixing incompatible scales.
//!
//! # Score Scales
//!
//! - `Score0To100`: Standard 0-100 scale for most debt scores
//! - `Score0To1`: Normalized 0-1 scale for certain calculations
//!
//! # Examples
//!
//! ```rust
//! use debtmap::priority::score_types::{Score0To100, Score0To1};
//!
//! // Create scores with automatic bounds enforcement
//! let score = Score0To100::new(85.0);
//! assert_eq!(score.value(), 85.0);
//!
//! // Out-of-bounds values are clamped
//! let clamped = Score0To100::new(150.0);
//! assert_eq!(clamped.value(), 100.0);
//!
//! // Explicit conversion between scales
//! let normalized = score.normalize();
//! assert_eq!(normalized.value(), 0.85);
//!
//! // Roundtrip conversion is identity
//! assert_eq!(score, normalized.denormalize());
//! ```

use serde::{Deserialize, Serialize};

/// Score on 0-100 scale.
///
/// This is the standard scale for debt scores throughout the system.
/// God object scores, unified scores, and threshold configurations
/// all use this scale.
///
/// Values are automatically clamped to the [0.0, 100.0] range.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Score0To100(f64);

impl Score0To100 {
    /// Create a new score, clamping to [0.0, 100.0].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use debtmap::priority::score_types::Score0To100;
    /// let score = Score0To100::new(85.0);
    /// assert_eq!(score.value(), 85.0);
    ///
    /// let clamped = Score0To100::new(150.0);
    /// assert_eq!(clamped.value(), 100.0);
    /// ```
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 100.0))
    }

    /// Get the raw score value.
    pub fn value(self) -> f64 {
        self.0
    }

    /// Normalize to 0-1 scale by dividing by 100.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use debtmap::priority::score_types::Score0To100;
    /// let score = Score0To100::new(85.0);
    /// let normalized = score.normalize();
    /// assert_eq!(normalized.value(), 0.85);
    /// ```
    pub fn normalize(self) -> Score0To1 {
        Score0To1(self.0 / 100.0)
    }
}

/// Score on 0-1 scale (normalized).
///
/// This scale is used for certain calculations where normalized
/// values are preferred. Most code should use `Score0To100`.
///
/// Values are automatically clamped to the [0.0, 1.0] range.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Score0To1(f64);

impl Score0To1 {
    /// Create a new normalized score, clamping to [0.0, 1.0].
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the raw score value.
    pub fn value(self) -> f64 {
        self.0
    }

    /// Denormalize to 0-100 scale by multiplying by 100.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use debtmap::priority::score_types::Score0To1;
    /// let normalized = Score0To1::new(0.85);
    /// let score = normalized.denormalize();
    /// assert_eq!(score.value(), 85.0);
    /// ```
    pub fn denormalize(self) -> Score0To100 {
        Score0To100(self.0 * 100.0)
    }
}

// Implement Display for user-facing output
impl std::fmt::Display for Score0To100 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

impl std::fmt::Display for Score0To1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_0_to_100_clamps_upper_bound() {
        let score = Score0To100::new(150.0);
        assert_eq!(score.value(), 100.0);
    }

    #[test]
    fn score_0_to_100_clamps_lower_bound() {
        let score = Score0To100::new(-10.0);
        assert_eq!(score.value(), 0.0);
    }

    #[test]
    fn score_0_to_1_clamps_upper_bound() {
        let score = Score0To1::new(1.5);
        assert_eq!(score.value(), 1.0);
    }

    #[test]
    fn score_0_to_1_clamps_lower_bound() {
        let score = Score0To1::new(-0.5);
        assert_eq!(score.value(), 0.0);
    }

    #[test]
    fn normalization_divides_by_100() {
        let score = Score0To100::new(85.0);
        let normalized = score.normalize();
        assert_eq!(normalized.value(), 0.85);
    }

    #[test]
    fn denormalization_multiplies_by_100() {
        let normalized = Score0To1::new(0.85);
        let score = normalized.denormalize();
        assert_eq!(score.value(), 85.0);
    }

    #[test]
    fn roundtrip_conversion_is_identity() {
        let original = Score0To100::new(75.5);
        let roundtrip = original.normalize().denormalize();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn comparison_works_correctly() {
        let score1 = Score0To100::new(50.0);
        let score2 = Score0To100::new(75.0);

        assert!(score1 < score2);
        assert!(score2 > score1);
        assert_eq!(score1, Score0To100::new(50.0));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn score_0_to_100_always_in_bounds(value in -1000.0..1000.0f64) {
            let score = Score0To100::new(value);
            assert!(score.value() >= 0.0 && score.value() <= 100.0);
        }

        #[test]
        fn score_0_to_1_always_in_bounds(value in -10.0..10.0f64) {
            let score = Score0To1::new(value);
            assert!(score.value() >= 0.0 && score.value() <= 1.0);
        }

        #[test]
        fn normalization_preserves_ordering(a in 0.0..100.0f64, b in 0.0..100.0f64) {
            let score_a = Score0To100::new(a);
            let score_b = Score0To100::new(b);

            if a < b {
                assert!(score_a.normalize() < score_b.normalize());
            } else if a > b {
                assert!(score_a.normalize() > score_b.normalize());
            } else {
                assert_eq!(score_a.normalize(), score_b.normalize());
            }
        }

        #[test]
        fn roundtrip_conversion_exact(value in 0.0..100.0f64) {
            let original = Score0To100::new(value);
            let roundtrip = original.normalize().denormalize();
            // Use approximate equality for floating point
            assert!((original.value() - roundtrip.value()).abs() < 1e-10);
        }
    }
}
