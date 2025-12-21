//! Refined types for domain invariants
//!
//! This module implements the "parse, don't validate" pattern using stillwater's
//! refined types. Domain invariants are encoded in the type system, validated once
//! at system boundaries, and guaranteed throughout the codebase.
//!
//! # Philosophy
//!
//! Instead of scattering validation checks like:
//! ```ignore
//! fn set_threshold(value: u32) {
//!     assert!(value >= 1 && value <= 1000, "threshold out of range");
//!     // ...
//! }
//! ```
//!
//! We validate once at construction:
//! ```ignore
//! fn set_threshold(value: ComplexityThreshold) {
//!     // Type guarantees 1 <= value <= 1000
//!     // ...
//! }
//! ```
//!
//! # Available Types
//!
//! ## Threshold Types
//! - [`ComplexityThreshold`]: Cyclomatic complexity threshold (1-1000)
//! - [`CognitiveThreshold`]: Cognitive complexity threshold (1-500)
//! - [`NestingThreshold`]: Max nesting depth threshold (1-50)
//!
//! ## Score Types
//! - [`NormalizedScore`]: Percentage score (0-100)
//! - [`RiskScore`]: Risk factor in unit interval (0.0-1.0)
//!
//! ## Weight Types
//! - [`WeightFactor`]: Scoring weight in unit interval (0.0-1.0)
//! - [`Percentage`]: Integer percentage (0-100)
//!
//! # Example
//!
//! ```ignore
//! use debtmap::core::refined::{ComplexityThreshold, WeightFactor};
//!
//! // Validate at construction - returns Result
//! let threshold = ComplexityThreshold::new(25)?;
//!
//! // Use like the inner type via Deref
//! let doubled = *threshold * 2;
//!
//! // Extract inner value explicitly
//! let raw: u32 = threshold.into_inner();
//! ```

use serde::{Deserialize, Serialize};
use stillwater::refined::{InRange, Predicate, Refined};

// ============================================================================
// Custom Predicates
// ============================================================================

/// Predicate for floating-point values in the unit interval [0.0, 1.0].
///
/// Used for weights, probabilities, and normalized scores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitInterval;

impl Predicate<f64> for UnitInterval {
    type Error = &'static str;

    fn check(value: &f64) -> Result<(), Self::Error> {
        if *value >= 0.0 && *value <= 1.0 {
            Ok(())
        } else {
            Err("value must be in range [0.0, 1.0]")
        }
    }
}

/// Predicate for positive f64 values (> 0.0).
///
/// Used for multipliers and factors that must be positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositiveF64;

impl Predicate<f64> for PositiveF64 {
    type Error = &'static str;

    fn check(value: &f64) -> Result<(), Self::Error> {
        if *value > 0.0 {
            Ok(())
        } else {
            Err("value must be positive (> 0.0)")
        }
    }
}

/// Predicate for non-negative f64 values (>= 0.0).
///
/// Used for scores and metrics that cannot be negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonNegativeF64;

impl Predicate<f64> for NonNegativeF64 {
    type Error = &'static str;

    fn check(value: &f64) -> Result<(), Self::Error> {
        if *value >= 0.0 {
            Ok(())
        } else {
            Err("value must be non-negative (>= 0.0)")
        }
    }
}

// ============================================================================
// Threshold Types
// ============================================================================

/// Cyclomatic complexity threshold.
///
/// Valid range: 1-1000. Cyclomatic complexity measures the number of
/// linearly independent paths through a function. Values above this
/// threshold indicate functions that may be difficult to test.
pub type ComplexityThreshold = Refined<u32, InRange<1, 1000>>;

/// Cognitive complexity threshold.
///
/// Valid range: 1-500. Cognitive complexity measures how difficult
/// code is to understand. Values above this threshold suggest the
/// code should be refactored for readability.
pub type CognitiveThreshold = Refined<u32, InRange<1, 500>>;

/// Maximum nesting depth threshold.
///
/// Valid range: 1-50. Deeply nested code is harder to understand
/// and maintain. This threshold flags excessive nesting.
pub type NestingThreshold = Refined<u32, InRange<1, 50>>;

/// Maximum function length threshold in lines.
///
/// Valid range: 1-1000. Long functions are harder to understand
/// and test. This threshold flags oversized functions.
pub type FunctionLengthThreshold = Refined<usize, InRange<1, 1000>>;

/// Maximum file length threshold in lines.
///
/// Valid range: 1-10000. Large files can indicate poor module
/// organization. This threshold flags oversized files.
pub type FileLengthThreshold = Refined<usize, InRange<1, 10000>>;

// ============================================================================
// Score Types
// ============================================================================

/// Normalized score as percentage (0-100).
///
/// Used for coverage percentages, quality scores, and other
/// metrics that are expressed as whole percentages.
pub type NormalizedScore = Refined<u32, InRange<0, 100>>;

/// Risk score in the unit interval [0.0, 1.0].
///
/// A floating-point score representing risk level where:
/// - 0.0 = no risk
/// - 1.0 = maximum risk
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RiskScore(f64);

impl RiskScore {
    /// Create a new risk score, validating it's in [0.0, 1.0].
    pub fn new(value: f64) -> Result<Self, &'static str> {
        UnitInterval::check(&value)?;
        Ok(Self(value))
    }

    /// Create a risk score without validation (for trusted sources).
    ///
    /// # Safety
    /// Caller must ensure value is in [0.0, 1.0].
    #[allow(dead_code)]
    pub fn new_unchecked(value: f64) -> Self {
        Self(value)
    }

    /// Get the inner value.
    pub fn into_inner(self) -> f64 {
        self.0
    }

    /// Get the inner value by reference.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl std::ops::Deref for RiskScore {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for RiskScore {
    fn default() -> Self {
        Self(0.0)
    }
}

// ============================================================================
// Weight and Configuration Types
// ============================================================================

/// Weight factor for scoring calculations.
///
/// Valid range: [0.0, 1.0]. Used for configuring the relative
/// importance of different factors in scoring algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeightFactor(f64);

impl WeightFactor {
    /// Create a new weight factor, validating it's in [0.0, 1.0].
    pub fn new(value: f64) -> Result<Self, &'static str> {
        UnitInterval::check(&value)?;
        Ok(Self(value))
    }

    /// Create a weight factor without validation (for trusted sources).
    ///
    /// # Safety
    /// Caller must ensure value is in [0.0, 1.0].
    #[allow(dead_code)]
    pub fn new_unchecked(value: f64) -> Self {
        Self(value)
    }

    /// Get the inner value.
    pub fn into_inner(self) -> f64 {
        self.0
    }

    /// Get the inner value by reference.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl std::ops::Deref for WeightFactor {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for WeightFactor {
    fn default() -> Self {
        Self(0.0)
    }
}

/// Integer percentage (0-100).
///
/// Used for configuration values that represent percentages
/// as whole numbers.
pub type Percentage = Refined<u32, InRange<0, 100>>;

/// Multiplier factor for scoring adjustments.
///
/// Valid range: (0.0, +inf). Must be positive. Used for role
/// multipliers and other scaling factors.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MultiplierFactor(f64);

impl MultiplierFactor {
    /// Create a new multiplier, validating it's positive.
    pub fn new(value: f64) -> Result<Self, &'static str> {
        PositiveF64::check(&value)?;
        Ok(Self(value))
    }

    /// Create a multiplier without validation (for trusted sources).
    ///
    /// # Safety
    /// Caller must ensure value is positive.
    #[allow(dead_code)]
    pub fn new_unchecked(value: f64) -> Self {
        Self(value)
    }

    /// Get the inner value.
    pub fn into_inner(self) -> f64 {
        self.0
    }

    /// Get the inner value by reference.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl std::ops::Deref for MultiplierFactor {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for MultiplierFactor {
    fn default() -> Self {
        Self(1.0)
    }
}

// ============================================================================
// Metric Types
// ============================================================================

/// Predicate for positive line counts (>= 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositiveLineCount;

impl Predicate<usize> for PositiveLineCount {
    type Error = &'static str;

    fn check(value: &usize) -> Result<(), Self::Error> {
        if *value >= 1 && *value <= 1_000_000 {
            Ok(())
        } else {
            Err("line count must be in range [1, 1000000]")
        }
    }
}

/// Predicate for valid function counts (0-10000).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidFunctionCount;

impl Predicate<usize> for ValidFunctionCount {
    type Error = &'static str;

    fn check(value: &usize) -> Result<(), Self::Error> {
        if *value <= 10_000 {
            Ok(())
        } else {
            Err("function count must be <= 10000")
        }
    }
}

/// Line count for files and functions.
///
/// Valid range: 1-1000000. Represents the number of lines in a
/// code unit. Must be at least 1 (empty files are not analyzed).
pub type LineCount = Refined<usize, PositiveLineCount>;

/// Nesting depth of code blocks.
///
/// Valid range: 0-100. Represents how deeply nested the code is.
pub type NestingDepth = Refined<u32, InRange<0, 100>>;

/// Function count in a module or file.
///
/// Valid range: 0-10000. Represents the number of functions
/// in a code unit.
pub type FunctionCount = Refined<usize, ValidFunctionCount>;

/// Debt density per 1000 lines of code.
///
/// Valid range: [0.0, +inf). Used for scale-independent
/// quality metrics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DebtDensity(f64);

impl DebtDensity {
    /// Create a new debt density, validating it's non-negative.
    pub fn new(value: f64) -> Result<Self, &'static str> {
        NonNegativeF64::check(&value)?;
        Ok(Self(value))
    }

    /// Create a debt density without validation (for trusted sources).
    ///
    /// # Safety
    /// Caller must ensure value is non-negative.
    #[allow(dead_code)]
    pub fn new_unchecked(value: f64) -> Self {
        Self(value)
    }

    /// Get the inner value.
    pub fn into_inner(self) -> f64 {
        self.0
    }

    /// Get the inner value by reference.
    pub fn get(&self) -> f64 {
        self.0
    }
}

impl std::ops::Deref for DebtDensity {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for DebtDensity {
    fn default() -> Self {
        Self(0.0)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod threshold_types {
        use super::*;

        #[test]
        fn complexity_threshold_valid_range() {
            // Valid values
            assert!(ComplexityThreshold::new(1).is_ok());
            assert!(ComplexityThreshold::new(500).is_ok());
            assert!(ComplexityThreshold::new(1000).is_ok());
        }

        #[test]
        fn complexity_threshold_invalid_range() {
            // Invalid values
            assert!(ComplexityThreshold::new(0).is_err());
            assert!(ComplexityThreshold::new(1001).is_err());
        }

        #[test]
        fn cognitive_threshold_valid_range() {
            assert!(CognitiveThreshold::new(1).is_ok());
            assert!(CognitiveThreshold::new(250).is_ok());
            assert!(CognitiveThreshold::new(500).is_ok());
        }

        #[test]
        fn cognitive_threshold_invalid_range() {
            assert!(CognitiveThreshold::new(0).is_err());
            assert!(CognitiveThreshold::new(501).is_err());
        }

        #[test]
        fn nesting_threshold_valid_range() {
            assert!(NestingThreshold::new(1).is_ok());
            assert!(NestingThreshold::new(25).is_ok());
            assert!(NestingThreshold::new(50).is_ok());
        }

        #[test]
        fn nesting_threshold_invalid_range() {
            assert!(NestingThreshold::new(0).is_err());
            assert!(NestingThreshold::new(51).is_err());
        }

        #[test]
        fn threshold_deref_access() {
            let threshold = ComplexityThreshold::new(25).unwrap();
            // Deref allows transparent access
            assert_eq!(*threshold, 25);
        }

        #[test]
        fn threshold_into_inner() {
            let threshold = ComplexityThreshold::new(30).unwrap();
            let raw: u32 = threshold.into_inner();
            assert_eq!(raw, 30);
        }
    }

    mod score_types {
        use super::*;

        #[test]
        fn normalized_score_valid_range() {
            assert!(NormalizedScore::new(0).is_ok());
            assert!(NormalizedScore::new(50).is_ok());
            assert!(NormalizedScore::new(100).is_ok());
        }

        #[test]
        fn normalized_score_invalid_range() {
            assert!(NormalizedScore::new(101).is_err());
        }

        #[test]
        fn risk_score_valid_range() {
            assert!(RiskScore::new(0.0).is_ok());
            assert!(RiskScore::new(0.5).is_ok());
            assert!(RiskScore::new(1.0).is_ok());
        }

        #[test]
        fn risk_score_invalid_range() {
            assert!(RiskScore::new(-0.1).is_err());
            assert!(RiskScore::new(1.1).is_err());
        }

        #[test]
        fn risk_score_deref() {
            let score = RiskScore::new(0.75).unwrap();
            assert_eq!(*score, 0.75);
        }
    }

    mod weight_types {
        use super::*;

        #[test]
        fn weight_factor_valid_range() {
            assert!(WeightFactor::new(0.0).is_ok());
            assert!(WeightFactor::new(0.5).is_ok());
            assert!(WeightFactor::new(1.0).is_ok());
        }

        #[test]
        fn weight_factor_invalid_range() {
            assert!(WeightFactor::new(-0.1).is_err());
            assert!(WeightFactor::new(1.1).is_err());
        }

        #[test]
        fn percentage_valid_range() {
            assert!(Percentage::new(0).is_ok());
            assert!(Percentage::new(50).is_ok());
            assert!(Percentage::new(100).is_ok());
        }

        #[test]
        fn percentage_invalid_range() {
            assert!(Percentage::new(101).is_err());
        }

        #[test]
        fn multiplier_valid_range() {
            assert!(MultiplierFactor::new(0.1).is_ok());
            assert!(MultiplierFactor::new(1.0).is_ok());
            assert!(MultiplierFactor::new(2.5).is_ok());
        }

        #[test]
        fn multiplier_invalid_range() {
            assert!(MultiplierFactor::new(0.0).is_err());
            assert!(MultiplierFactor::new(-0.5).is_err());
        }
    }

    mod metric_types {
        use super::*;

        #[test]
        fn line_count_valid_range() {
            assert!(LineCount::new(1).is_ok());
            assert!(LineCount::new(1000).is_ok());
            assert!(LineCount::new(1000000).is_ok());
        }

        #[test]
        fn line_count_invalid_range() {
            assert!(LineCount::new(0).is_err());
        }

        #[test]
        fn nesting_depth_valid_range() {
            assert!(NestingDepth::new(0).is_ok());
            assert!(NestingDepth::new(50).is_ok());
            assert!(NestingDepth::new(100).is_ok());
        }

        #[test]
        fn nesting_depth_invalid_range() {
            assert!(NestingDepth::new(101).is_err());
        }

        #[test]
        fn function_count_valid_range() {
            assert!(FunctionCount::new(0).is_ok());
            assert!(FunctionCount::new(500).is_ok());
            assert!(FunctionCount::new(10000).is_ok());
        }

        #[test]
        fn function_count_invalid_range() {
            assert!(FunctionCount::new(10001).is_err());
        }

        #[test]
        fn debt_density_valid_range() {
            assert!(DebtDensity::new(0.0).is_ok());
            assert!(DebtDensity::new(50.0).is_ok());
            assert!(DebtDensity::new(1000.0).is_ok());
        }

        #[test]
        fn debt_density_invalid_range() {
            assert!(DebtDensity::new(-0.1).is_err());
        }
    }

    mod custom_predicates {
        use super::*;

        #[test]
        fn unit_interval_boundaries() {
            assert!(UnitInterval::check(&0.0).is_ok());
            assert!(UnitInterval::check(&0.5).is_ok());
            assert!(UnitInterval::check(&1.0).is_ok());
            assert!(UnitInterval::check(&-0.001).is_err());
            assert!(UnitInterval::check(&1.001).is_err());
        }

        #[test]
        fn positive_f64_boundaries() {
            assert!(PositiveF64::check(&0.001).is_ok());
            assert!(PositiveF64::check(&1.0).is_ok());
            assert!(PositiveF64::check(&f64::MAX).is_ok());
            assert!(PositiveF64::check(&0.0).is_err());
            assert!(PositiveF64::check(&-0.001).is_err());
        }

        #[test]
        fn non_negative_f64_boundaries() {
            assert!(NonNegativeF64::check(&0.0).is_ok());
            assert!(NonNegativeF64::check(&0.001).is_ok());
            assert!(NonNegativeF64::check(&f64::MAX).is_ok());
            assert!(NonNegativeF64::check(&-0.001).is_err());
        }

        #[test]
        fn positive_line_count_boundaries() {
            assert!(PositiveLineCount::check(&1).is_ok());
            assert!(PositiveLineCount::check(&1000).is_ok());
            assert!(PositiveLineCount::check(&1_000_000).is_ok());
            assert!(PositiveLineCount::check(&0).is_err());
            assert!(PositiveLineCount::check(&1_000_001).is_err());
        }

        #[test]
        fn valid_function_count_boundaries() {
            assert!(ValidFunctionCount::check(&0).is_ok());
            assert!(ValidFunctionCount::check(&5000).is_ok());
            assert!(ValidFunctionCount::check(&10_000).is_ok());
            assert!(ValidFunctionCount::check(&10_001).is_err());
        }
    }

    mod serde_roundtrip {
        use super::*;

        #[test]
        fn risk_score_serde() {
            let score = RiskScore::new(0.75).unwrap();
            let json = serde_json::to_string(&score).unwrap();
            let restored: RiskScore = serde_json::from_str(&json).unwrap();
            assert_eq!(*score, *restored);
        }

        #[test]
        fn weight_factor_serde() {
            let weight = WeightFactor::new(0.35).unwrap();
            let json = serde_json::to_string(&weight).unwrap();
            let restored: WeightFactor = serde_json::from_str(&json).unwrap();
            assert_eq!(*weight, *restored);
        }

        #[test]
        fn multiplier_factor_serde() {
            let mult = MultiplierFactor::new(1.5).unwrap();
            let json = serde_json::to_string(&mult).unwrap();
            let restored: MultiplierFactor = serde_json::from_str(&json).unwrap();
            assert_eq!(*mult, *restored);
        }

        #[test]
        fn debt_density_serde() {
            let density = DebtDensity::new(45.5).unwrap();
            let json = serde_json::to_string(&density).unwrap();
            let restored: DebtDensity = serde_json::from_str(&json).unwrap();
            assert_eq!(*density, *restored);
        }
    }
}
