//! # God Object Detector (Orchestration)
//!
//! Composes pure core functions into the detection pipeline.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Imperative Shell** - orchestration and I/O boundary.
//! It composes pure functions from the core modules into the analysis pipeline.

use super::classification_types::*;
use super::core_types::GodObjectAnalysis;
use crate::common::UnifiedLocationExtractor;
use crate::organization::{MaintainabilityImpact, OrganizationAntiPattern, OrganizationDetector};
use std::collections::{HashMap, HashSet};
use std::path::Path;

// Import clustering types for improved responsibility detection
use crate::organization::clustering::{CallGraphProvider, FieldAccessProvider};

/// Adapter for call graph adjacency matrix to CallGraphProvider trait.
/// This will be used in future phases when composing pure functions.
#[allow(dead_code)]
struct CallGraphAdapter {
    adjacency: std::collections::BTreeMap<(String, String), usize>,
}

#[allow(dead_code)]
impl CallGraphAdapter {
    fn from_adjacency_matrix(adjacency: HashMap<(String, String), usize>) -> Self {
        // Convert HashMap to BTreeMap for deterministic iteration order
        Self {
            adjacency: adjacency.into_iter().collect(),
        }
    }
}

impl CallGraphProvider for CallGraphAdapter {
    fn call_count(&self, from: &str, to: &str) -> usize {
        *self
            .adjacency
            .get(&(from.to_string(), to.to_string()))
            .unwrap_or(&0)
    }

    fn callees(&self, method: &str) -> HashSet<String> {
        // BTreeMap provides deterministic iteration order
        self.adjacency
            .keys()
            .filter(|(caller, _)| caller == method)
            .map(|(_, callee)| callee.clone())
            .collect()
    }

    fn callers(&self, method: &str) -> HashSet<String> {
        // BTreeMap provides deterministic iteration order
        self.adjacency
            .keys()
            .filter(|(_, callee)| callee == method)
            .map(|(caller, _)| caller.clone())
            .collect()
    }
}

/// Adapter for FieldAccessTracker to FieldAccessProvider trait.
/// This will be used in future phases when composing pure functions.
#[allow(dead_code)]
struct FieldAccessAdapter<'a> {
    tracker: &'a crate::organization::FieldAccessTracker,
}

#[allow(dead_code)]
impl<'a> FieldAccessAdapter<'a> {
    fn new(tracker: &'a crate::organization::FieldAccessTracker) -> Self {
        Self { tracker }
    }
}

impl<'a> FieldAccessProvider for FieldAccessAdapter<'a> {
    fn fields_accessed_by(&self, method: &str) -> HashSet<String> {
        self.tracker.fields_for_method(method).unwrap_or_default()
    }

    fn writes_to_field(&self, method: &str, field: &str) -> bool {
        self.tracker.method_writes_to_field(method, field)
    }
}

/// God object detector that orchestrates analysis.
pub struct GodObjectDetector {
    pub(crate) max_methods: usize,
    pub(crate) max_fields: usize,
    pub(crate) max_responsibilities: usize,
    pub(crate) location_extractor: Option<UnifiedLocationExtractor>,
    pub(crate) source_content: Option<String>,
}

impl Default for GodObjectDetector {
    fn default() -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_responsibilities: 3,
            location_extractor: None,
            source_content: None,
        }
    }
}

impl GodObjectDetector {
    /// Create a new detector with default thresholds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a detector with source content for enhanced analysis.
    pub fn with_source_content(source_content: &str) -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_responsibilities: 3,
            location_extractor: Some(UnifiedLocationExtractor::new(source_content)),
            source_content: Some(source_content.to_string()),
        }
    }

    /// Main analysis pipeline - composes pure functions.
    ///
    /// For now, this uses the comprehensive implementation from god_object_detector.rs module.
    /// This will be further refactored in Phase 8 to compose pure functions directly.
    #[allow(deprecated)]
    pub fn analyze_enhanced(&self, path: &Path, ast: &syn::File) -> EnhancedGodObjectAnalysis {
        // Import the comprehensive analysis from the legacy god_object_detector module
        // This avoids circular dependencies during the transition
        use crate::organization::god_object_detector as legacy;
        legacy::analyze_enhanced_for_detector(self, path, ast)
    }

    /// Get the max_methods threshold configured for this detector.
    pub fn max_methods(&self) -> usize {
        self.max_methods
    }

    /// Get the max_fields threshold configured for this detector.
    pub fn max_fields(&self) -> usize {
        self.max_fields
    }

    /// Get the max_responsibilities threshold configured for this detector.
    pub fn max_responsibilities(&self) -> usize {
        self.max_responsibilities
    }

    /// Comprehensive analysis returning GodObjectAnalysis.
    ///
    /// This is a simpler analysis compared to analyze_enhanced, used by enhanced_analyzer.
    #[allow(deprecated)]
    pub fn analyze_comprehensive(&self, path: &Path, ast: &syn::File) -> GodObjectAnalysis {
        use crate::organization::god_object_detector as legacy;
        legacy::analyze_comprehensive_for_detector(self, path, ast)
    }
}

impl OrganizationDetector for GodObjectDetector {
    #[allow(deprecated)]
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        // Import the comprehensive analysis from the legacy god_object_detector module
        use crate::organization::god_object_detector as legacy;
        legacy::detect_anti_patterns_for_detector(self, file)
    }

    fn detector_name(&self) -> &'static str {
        "GodObjectDetector"
    }

    #[allow(deprecated)]
    fn estimate_maintainability_impact(
        &self,
        pattern: &OrganizationAntiPattern,
    ) -> MaintainabilityImpact {
        match pattern {
            OrganizationAntiPattern::GodObject {
                method_count,
                field_count,
                ..
            } => {
                // Use the legacy implementation
                use crate::organization::god_object_detector as legacy;
                legacy::classify_god_object_impact(*method_count, *field_count)
            }
            _ => MaintainabilityImpact::Low,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        let detector = GodObjectDetector::new();
        assert_eq!(detector.max_methods, 15);
        assert_eq!(detector.max_fields, 10);
        assert_eq!(detector.max_responsibilities, 3);
    }

    #[test]
    fn test_detector_with_source_content() {
        let content = "struct Foo {}";
        let detector = GodObjectDetector::with_source_content(content);
        assert!(detector.source_content.is_some());
        assert!(detector.location_extractor.is_some());
    }

    #[test]
    fn test_detector_thresholds() {
        let detector = GodObjectDetector::new();
        assert_eq!(detector.max_methods(), 15);
        assert_eq!(detector.max_fields(), 10);
        assert_eq!(detector.max_responsibilities(), 3);
    }

    #[test]
    fn test_detector_name() {
        let detector = GodObjectDetector::new();
        assert_eq!(detector.detector_name(), "GodObjectDetector");
    }
}
