//! Confidence thresholds for responsibility classification.
//!
//! This module defines the confidence thresholds used to determine when
//! a method's responsibility classification is reliable enough to act on.
//!
//! # Rationale
//!
//! Without confidence thresholds, the system tends to over-classify methods
//! as "utilities", leading to poor decomposition recommendations. By requiring
//! minimum confidence scores, we ensure only reliable classifications drive
//! architectural decisions.

/// Minimum confidence for any classification.
///
/// If a classification has confidence below this threshold, it is rejected
/// and the method is left in its original location rather than extracted.
///
/// **Value**: 0.50 (50%)
///
/// **Rationale**: Require more signal than noise before accepting a classification.
pub const MINIMUM_CONFIDENCE: f64 = 0.50;

/// Minimum confidence for "utilities" classification.
///
/// The "utilities" category requires higher confidence than other categories
/// because it was previously used as an unconditional fallback, leading to
/// over-classification (~30% of methods).
///
/// **Value**: 0.60 (60%)
///
/// **Rationale**: Higher bar prevents lazy fallback to "utilities" when
/// other signals are weak.
pub const UTILITIES_THRESHOLD: f64 = 0.60;

/// Minimum confidence for module split recommendation.
///
/// Module splits are structural changes that require high confidence.
/// Only recommend splits when aggregate confidence across all methods
/// in the cluster exceeds this threshold.
///
/// **Value**: 0.65 (65%)
///
/// **Rationale**: Structural changes need strong evidence to justify
/// the refactoring effort and risk.
pub const MODULE_SPLIT_CONFIDENCE: f64 = 0.65;

/// Minimum number of methods required for a module split.
///
/// **Value**: 5 methods
///
/// **Rationale**: Splitting off fewer methods creates unnecessary fragmentation.
pub const MIN_METHODS_FOR_SPLIT: usize = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_values() {
        // Document threshold values and relationships
        // These are compile-time constants, so we verify they exist
        let _min = MINIMUM_CONFIDENCE;
        let _util = UTILITIES_THRESHOLD;
        let _split = MODULE_SPLIT_CONFIDENCE;
        let _methods = MIN_METHODS_FOR_SPLIT;

        // Verify runtime behavior with dynamic values
        let test_confidence = 0.55;
        assert!(test_confidence >= MINIMUM_CONFIDENCE);
        assert!(test_confidence < UTILITIES_THRESHOLD);
    }
}
