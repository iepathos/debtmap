//! Architecture recognition for stable core detection (spec 269).
//!
//! This module provides architectural pattern recognition based on the
//! Stable Dependencies Principle from Clean Architecture. It identifies
//! modules that are intentionally stable foundations vs. actual technical debt.
//!
//! # Background
//!
//! The Stable Dependencies Principle states that dependencies should flow
//! toward stability. A stable module (low instability) with many dependents
//! is architecturally correct - it's a foundation that others build upon.
//!
//! Flagging such modules as "critical blast radius" debt is a false positive.
//! This module provides the logic to recognize and properly classify these
//! architectural patterns.
//!
//! # Key Metrics
//!
//! - **Instability**: I = Ce/(Ca+Ce), where Ce = efferent (outgoing) coupling,
//!   Ca = afferent (incoming) coupling. Range [0.0, 1.0].
//!   - I < 0.3: Stable (depended upon by many, depends on few)
//!   - I > 0.7: Unstable (depends on many, depended upon by few)
//!
//! - **Test Caller Ratio**: Percentage of callers that are test functions.
//!   High ratio (> 0.7) indicates well-tested code, reducing risk.
//!
//! # Usage
//!
//! ```rust,ignore
//! use debtmap::priority::architecture_recognition::*;
//!
//! let instability = calculate_instability(incoming, outgoing);
//! let classification = classify_coupling_pattern(
//!     instability,
//!     production_callers,
//!     test_callers,
//!     callees,
//! );
//!
//! if classification.is_stable_by_design() {
//!     // This is intentional architecture, not debt
//! }
//! ```

// Re-export core types and functions from the unified output module
pub use crate::output::unified::{
    calculate_architectural_dependency_factor, calculate_instability, classify_coupling_pattern,
    CouplingClassification,
};

/// Low confidence threshold for filtering uncertain items (spec 269).
///
/// Items with completeness_confidence below this threshold are considered
/// unreliable and should be excluded from top priority lists by default.
pub const LOW_CONFIDENCE_THRESHOLD: f64 = 0.5;

/// Determine if an item should be filtered due to low confidence.
///
/// Items with confidence below the threshold are unreliable due to
/// incomplete analysis (e.g., missing call graph data, incomplete parsing).
///
/// # Arguments
///
/// * `completeness_confidence` - The confidence score from context analysis
///
/// # Returns
///
/// `true` if the item should be filtered (low confidence), `false` otherwise.
pub fn is_low_confidence(completeness_confidence: f64) -> bool {
    completeness_confidence < LOW_CONFIDENCE_THRESHOLD
}

/// Generate a confidence note for low-confidence items.
///
/// This provides a user-friendly explanation of why an item may have
/// unreliable metrics.
///
/// # Arguments
///
/// * `completeness_confidence` - The confidence score from context analysis
///
/// # Returns
///
/// `Some(note)` if confidence is low, `None` otherwise.
pub fn generate_confidence_note(completeness_confidence: f64) -> Option<String> {
    if completeness_confidence < LOW_CONFIDENCE_THRESHOLD {
        Some(format!(
            "Low confidence ({:.0}%) - metrics may be incomplete",
            completeness_confidence * 100.0
        ))
    } else {
        None
    }
}

/// Summary of architectural analysis for a codebase.
///
/// Contains counts and examples of different architectural classifications,
/// useful for generating the "Architectural Analysis" section in reports.
#[derive(Debug, Clone, Default)]
pub struct ArchitecturalSummary {
    /// Count of WellTestedCore, StableFoundation, StableCore items
    pub stable_count: usize,
    /// Count of UnstableHighCoupling, ArchitecturalHub, HighlyCoupled items
    pub concern_count: usize,
    /// Top stable items (file paths with metrics)
    pub top_stable: Vec<ArchitecturalItem>,
    /// Top concern items (file paths with metrics)
    pub top_concerns: Vec<ArchitecturalItem>,
}

/// A single item in the architectural summary.
#[derive(Debug, Clone)]
pub struct ArchitecturalItem {
    /// File path
    pub file: String,
    /// Classification
    pub classification: CouplingClassification,
    /// Instability metric
    pub instability: f64,
    /// Production caller count
    pub production_callers: usize,
    /// Test caller count
    pub test_callers: usize,
}

impl ArchitecturalSummary {
    /// Create a new empty summary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an item to the summary based on its classification.
    pub fn add_item(&mut self, item: ArchitecturalItem) {
        if item.classification.is_stable_by_design() {
            self.stable_count += 1;
            if self.top_stable.len() < 10 {
                self.top_stable.push(item);
            }
        } else if item.classification.is_architectural_concern() {
            self.concern_count += 1;
            if self.top_concerns.len() < 10 {
                self.top_concerns.push(item);
            }
        }
    }

    /// Check if there are any stable-by-design items.
    pub fn has_stable_items(&self) -> bool {
        self.stable_count > 0
    }

    /// Check if there are any architectural concerns.
    pub fn has_concerns(&self) -> bool {
        self.concern_count > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_confidence_threshold() {
        assert!(is_low_confidence(0.0));
        assert!(is_low_confidence(0.49));
        assert!(!is_low_confidence(0.5));
        assert!(!is_low_confidence(0.9));
        assert!(!is_low_confidence(1.0));
    }

    #[test]
    fn test_generate_confidence_note() {
        assert!(generate_confidence_note(0.3).is_some());
        assert!(generate_confidence_note(0.3).unwrap().contains("30%"));
        assert!(generate_confidence_note(0.6).is_none());
    }

    #[test]
    fn test_architectural_summary_add_stable() {
        let mut summary = ArchitecturalSummary::new();

        summary.add_item(ArchitecturalItem {
            file: "src/core.rs".to_string(),
            classification: CouplingClassification::WellTestedCore,
            instability: 0.2,
            production_callers: 5,
            test_callers: 85,
        });

        assert_eq!(summary.stable_count, 1);
        assert_eq!(summary.concern_count, 0);
        assert_eq!(summary.top_stable.len(), 1);
        assert!(summary.has_stable_items());
        assert!(!summary.has_concerns());
    }

    #[test]
    fn test_architectural_summary_add_concern() {
        let mut summary = ArchitecturalSummary::new();

        summary.add_item(ArchitecturalItem {
            file: "src/unstable.rs".to_string(),
            classification: CouplingClassification::UnstableHighCoupling,
            instability: 0.8,
            production_callers: 15,
            test_callers: 2,
        });

        assert_eq!(summary.stable_count, 0);
        assert_eq!(summary.concern_count, 1);
        assert_eq!(summary.top_concerns.len(), 1);
        assert!(!summary.has_stable_items());
        assert!(summary.has_concerns());
    }

    #[test]
    fn test_architectural_summary_limits_to_10() {
        let mut summary = ArchitecturalSummary::new();

        for i in 0..15 {
            summary.add_item(ArchitecturalItem {
                file: format!("src/core_{}.rs", i),
                classification: CouplingClassification::StableCore,
                instability: 0.2,
                production_callers: 6,
                test_callers: 2,
            });
        }

        assert_eq!(summary.stable_count, 15);
        assert_eq!(summary.top_stable.len(), 10);
    }
}
