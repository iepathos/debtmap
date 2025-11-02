//! Pattern Analysis Data Structures
//!
//! This module defines data structures for aggregating and displaying pattern analysis results
//! from purity analysis (Spec 143), framework pattern detection (Spec 144), and Rust-specific
//! pattern detection (Spec 146).
//!
//! # Architecture
//!
//! - **Data structures are pure data** - no formatting logic, only calculations
//! - **All types implement Serialize/Deserialize** - for potential JSON output
//! - **Default implementations** - all types have sensible defaults
//! - **Helper methods for queries** - has_patterns(), repetitive_traits(), etc.
//!
//! # Example
//!
//! ```ignore
//! use debtmap::output::pattern_analysis::{PatternAnalysis, PurityMetrics};
//!
//! // Create pattern analysis from function analyses
//! let pattern_analysis = PatternAnalysis::from_functions(&functions);
//!
//! // Query the data
//! if pattern_analysis.purity.purity_percentage() > 0.5 {
//!     println!("Codebase is more than 50% pure!");
//! }
//! ```

use crate::analysis::purity_analysis::PurityViolation;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Constants for thresholds (configurable via settings)
pub const REPETITIVE_TRAIT_THRESHOLD: usize = 5;
pub const MAX_DISPLAYED_EXAMPLES: usize = 3;
pub const MAX_ALMOST_PURE_VIOLATIONS: usize = 2;

/// Purity metrics aggregated from function analyses
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PurityMetrics {
    pub strictly_pure: usize,
    pub locally_pure: usize,
    pub read_only: usize,
    pub impure: usize,
    pub almost_pure: Vec<AlmostPureFunction>,
}

impl PurityMetrics {
    /// Pure calculation - returns total number of functions analyzed
    pub fn total_functions(&self) -> usize {
        self.strictly_pure + self.locally_pure + self.read_only + self.impure
    }

    /// Returns purity percentage as a value between 0.0 and 1.0
    pub fn purity_percentage(&self) -> f64 {
        let total = self.total_functions();
        if total == 0 {
            0.0
        } else {
            (self.strictly_pure + self.locally_pure) as f64 / total as f64
        }
    }

    /// Returns the top N almost-pure functions by refactoring impact
    pub fn top_refactoring_opportunities(&self, limit: usize) -> &[AlmostPureFunction] {
        let end = self.almost_pure.len().min(limit);
        &self.almost_pure[..end]
    }

    /// Returns true if there are any functions analyzed
    pub fn has_functions(&self) -> bool {
        self.total_functions() > 0
    }

    /// Returns true if there are refactoring opportunities
    pub fn has_opportunities(&self) -> bool {
        !self.almost_pure.is_empty()
    }
}

/// Function that is "almost pure" - has only 1-2 purity violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmostPureFunction {
    pub name: String,
    pub violations: Vec<PurityViolation>,
    pub refactoring_suggestion: String,
}

/// Framework pattern metrics aggregated from function analyses
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FrameworkPatternMetrics {
    pub patterns: Vec<DetectedPattern>,
}

impl FrameworkPatternMetrics {
    /// Returns patterns sorted by count (most frequent first)
    pub fn sorted_by_frequency(&self) -> Vec<&DetectedPattern> {
        let mut sorted: Vec<_> = self.patterns.iter().collect();
        sorted.sort_by(|a, b| b.count.cmp(&a.count));
        sorted
    }

    /// Returns true if any framework patterns were detected
    pub fn has_patterns(&self) -> bool {
        !self.patterns.is_empty()
    }
}

/// Detected framework pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub framework: String,
    pub pattern_type: String,
    pub count: usize,
    pub examples: Vec<String>,
    pub recommendation: String,
}

/// Rust-specific pattern metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RustPatternMetrics {
    pub trait_impls: Vec<TraitImplementation>,
    pub async_patterns: AsyncPatternSummary,
    pub error_handling: ErrorHandlingSummary,
    pub builder_candidates: Vec<String>,
}

impl RustPatternMetrics {
    /// Returns trait implementations that appear repetitively
    pub fn repetitive_traits(&self) -> Vec<&TraitImplementation> {
        self.trait_impls
            .iter()
            .filter(|t| t.count >= REPETITIVE_TRAIT_THRESHOLD)
            .collect()
    }

    /// Returns true if async patterns are present
    pub fn has_async_patterns(&self) -> bool {
        self.async_patterns.async_functions > 0
    }

    /// Returns true if error handling patterns are present
    pub fn has_error_handling(&self) -> bool {
        self.error_handling.question_mark_density > 0.0 || self.error_handling.unwrap_count > 0
    }

    /// Returns true if builder patterns are present
    pub fn has_builder_candidates(&self) -> bool {
        !self.builder_candidates.is_empty()
    }

    /// Returns true if any Rust patterns are present
    pub fn has_patterns(&self) -> bool {
        !self.trait_impls.is_empty()
            || self.has_async_patterns()
            || self.has_error_handling()
            || self.has_builder_candidates()
    }
}

/// Trait implementation pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitImplementation {
    pub trait_name: String,
    pub count: usize,
    pub types: Vec<String>,
}

/// Async pattern summary
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AsyncPatternSummary {
    pub async_functions: usize,
    pub spawn_calls: usize,
    pub channel_usage: bool,
    pub mutex_usage: bool,
}

/// Error handling summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandlingSummary {
    /// Average ? operators per function
    pub question_mark_density: f64,
    pub custom_error_types: Vec<String>,
    /// Anti-pattern: unwrap() call count
    pub unwrap_count: usize,
}

impl Default for ErrorHandlingSummary {
    fn default() -> Self {
        Self {
            question_mark_density: 0.0,
            custom_error_types: vec![],
            unwrap_count: 0,
        }
    }
}

/// Container for all pattern analysis results
/// Attached to recommendations for display
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatternAnalysis {
    pub purity: PurityMetrics,
    pub frameworks: FrameworkPatternMetrics,
    pub rust_patterns: RustPatternMetrics,
}

impl PatternAnalysis {
    /// Create a new empty pattern analysis
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if any patterns were detected
    pub fn has_patterns(&self) -> bool {
        self.purity.has_functions()
            || self.frameworks.has_patterns()
            || self.rust_patterns.has_patterns()
    }
}

/// Display implementation for PurityViolation (used in formatting)
impl fmt::Display for PurityViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}
