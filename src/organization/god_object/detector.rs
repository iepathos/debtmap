//! # God Object Detector (Orchestration)
//!
//! Composes pure core functions into the detection pipeline.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Imperative Shell** - orchestration and I/O boundary.
//! It composes pure functions from the core modules into the analysis pipeline.
//!
//! ## Migration Notice (Spec 212)
//!
//! The primary god object detection is now handled by the extraction adapter:
//! `crate::extraction::adapters::god_object::analyze_god_objects()`
//!
//! This module retains:
//! - `GodObjectDetector` struct for threshold configuration
//! - `OrganizationDetector` trait implementation for anti-pattern detection
//! - Helper functions for maintainability impact classification

use super::classifier::group_methods_by_responsibility;
use crate::common::UnifiedLocationExtractor;
use crate::organization::ResponsibilityGroup;
use crate::organization::{MaintainabilityImpact, OrganizationAntiPattern, OrganizationDetector};
use std::collections::HashSet;

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
    fn from_adjacency_matrix(
        adjacency: std::collections::HashMap<(String, String), usize>,
    ) -> Self {
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
///
/// ## Usage
///
/// For primary god object detection, use the extraction adapter instead:
/// ```ignore
/// use crate::extraction::adapters::god_object::analyze_god_objects;
/// let results = analyze_god_objects(&path, &extracted_data);
/// ```
///
/// This struct is retained for:
/// - Threshold configuration
/// - `OrganizationDetector` trait implementation (used by `rust.rs`)
pub struct GodObjectDetector {
    pub(crate) max_methods: usize,
    pub(crate) max_fields: usize,
    pub(crate) max_responsibilities: usize,
    pub(crate) location_extractor: Option<UnifiedLocationExtractor>,
    #[allow(dead_code)]
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

    /// Classify maintainability impact based on method and field counts.
    ///
    /// Pure function that maps god object metrics to impact severity.
    pub fn classify_god_object_impact(
        method_count: usize,
        field_count: usize,
    ) -> MaintainabilityImpact {
        match () {
            _ if method_count > 30 || field_count > 20 => MaintainabilityImpact::Critical,
            _ if method_count > 20 || field_count > 15 => MaintainabilityImpact::High,
            _ => MaintainabilityImpact::Medium,
        }
    }
}

impl OrganizationDetector for GodObjectDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        use super::ast_visitor::TypeVisitor;
        use syn::visit::Visit;

        let mut patterns = Vec::new();
        let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
        visitor.visit_file(file);

        // Analyze each struct found
        for (type_name, type_info) in visitor.types {
            // Check if it's a god object using thresholds
            let method_names = &type_info.methods;
            let responsibilities = group_methods_by_responsibility(method_names);
            let is_god = type_info.method_count > self.max_methods
                || type_info.field_count > self.max_fields
                || responsibilities.len() > self.max_responsibilities;

            if is_god {
                // Create responsibility groups
                let suggested_split: Vec<ResponsibilityGroup> = responsibilities
                    .into_iter()
                    .map(|(responsibility, methods)| ResponsibilityGroup {
                        name: responsibility.clone(),
                        methods,
                        fields: vec![], // Field grouping not implemented yet
                        responsibility,
                    })
                    .collect();

                patterns.push(OrganizationAntiPattern::GodObject {
                    type_name: type_name.clone(),
                    method_count: type_info.method_count,
                    field_count: type_info.field_count,
                    responsibility_count: suggested_split.len(),
                    suggested_split,
                    location: type_info.location,
                });
            }
        }

        patterns
    }

    fn detector_name(&self) -> &'static str {
        "GodObjectDetector"
    }

    fn estimate_maintainability_impact(
        &self,
        pattern: &OrganizationAntiPattern,
    ) -> MaintainabilityImpact {
        match pattern {
            OrganizationAntiPattern::GodObject {
                method_count,
                field_count,
                ..
            } => Self::classify_god_object_impact(*method_count, *field_count),
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

    /// Spec 206: Test that non-cohesive structs pass through the cohesion gate
    /// Note: Whether they get flagged depends on scoring, which is separate from cohesion
    #[test]
    fn test_cohesion_gate_allows_non_cohesive_structs() {
        use crate::organization::god_object::classifier::{
            calculate_domain_cohesion, is_cohesive_struct,
        };

        // A struct with unrelated method domains - should NOT be cohesive
        let struct_name = "ApplicationManager";
        let methods: Vec<String> = vec![
            "new",
            "parse_json",
            "parse_xml",
            "render_html",
            "render_pdf",
            "validate_email",
            "validate_phone",
            "send_email",
            "send_sms",
            "save_to_database",
            "load_from_database",
            "connect_to_server",
            "disconnect_from_server",
            "handle_request",
            "handle_error",
            "log_event",
            "track_metrics",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let cohesion = calculate_domain_cohesion(struct_name, &methods);
        let is_cohesive = is_cohesive_struct(struct_name, &methods);

        // Methods don't contain "application" or "manager" keywords
        // So cohesion should be very low (0 or close to 0)
        assert!(
            cohesion < 0.1,
            "ApplicationManager with unrelated methods should have near-zero cohesion, got {}",
            cohesion
        );

        // Should NOT be marked as cohesive
        assert!(
            !is_cohesive,
            "ApplicationManager with unrelated methods should NOT be cohesive"
        );

        // Contrast with a cohesive struct
        let cohesive_struct = "ModuleTracker";
        let cohesive_methods: Vec<String> = vec![
            "new",
            "analyze_module",
            "get_module_calls",
            "resolve_module_call",
            "track_module",
            "is_module_public",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let cohesive_cohesion = calculate_domain_cohesion(cohesive_struct, &cohesive_methods);
        let cohesive_is_cohesive = is_cohesive_struct(cohesive_struct, &cohesive_methods);

        // Methods contain "module" keyword - should be high cohesion
        assert!(
            cohesive_cohesion > 0.5,
            "ModuleTracker with module-related methods should have high cohesion, got {}",
            cohesive_cohesion
        );

        // Should be marked as cohesive
        assert!(
            cohesive_is_cohesive,
            "ModuleTracker with module-related methods SHOULD be cohesive"
        );
    }

    /// Spec 207: Test that LOC includes impl block lines
    #[test]
    fn test_loc_includes_impl_blocks() {
        use crate::organization::god_object::ast_visitor::TypeVisitor;
        use syn::visit::Visit;

        let content = r#"
pub struct Foo {
    a: i32,
    b: i32,
}

impl Foo {
    pub fn method1(&self) -> i32 {
        self.a + self.b
    }

    pub fn method2(&self) -> i32 {
        self.a * self.b
    }
}
"#;

        let ast = syn::parse_file(content).expect("parse");
        let detector = GodObjectDetector::with_source_content(content);
        let mut visitor = TypeVisitor::with_location_extractor(detector.location_extractor.clone());
        visitor.visit_file(&ast);

        let foo = visitor.types.get("Foo").expect("Foo should be found");

        // Verify impl_locations is populated
        assert!(
            !foo.impl_locations.is_empty(),
            "impl_locations should be populated"
        );

        // Calculate total LOC like the extraction adapter does
        let struct_loc = foo
            .location
            .end_line
            .unwrap_or(foo.location.line)
            .saturating_sub(foo.location.line)
            + 1;

        let impl_loc: usize = foo
            .impl_locations
            .iter()
            .map(|loc| loc.end_line.unwrap_or(loc.line).saturating_sub(loc.line) + 1)
            .sum();

        let total_loc = struct_loc + impl_loc;

        // Struct: lines 2-5 (~4 lines)
        // Impl: lines 7-15 (~9 lines)
        // Total should be at least 10 lines (struct + impl), not just 4 (struct only)
        assert!(
            total_loc >= 10,
            "LOC should include impl blocks, got {} (struct={}, impl={})",
            total_loc,
            struct_loc,
            impl_loc
        );
    }

    /// Spec 207: Test LOC with multiple impl blocks
    #[test]
    fn test_loc_with_multiple_impl_blocks() {
        use crate::organization::god_object::ast_visitor::TypeVisitor;
        use syn::visit::Visit;

        let content = r#"
pub struct Bar { a: i32 }

impl Bar {
    pub fn new() -> Self { Self { a: 0 } }
}

impl Default for Bar {
    fn default() -> Self { Self::new() }
}

impl Clone for Bar {
    fn clone(&self) -> Self { Self { a: self.a } }
}
"#;

        let ast = syn::parse_file(content).expect("parse");
        let detector = GodObjectDetector::with_source_content(content);
        let mut visitor = TypeVisitor::with_location_extractor(detector.location_extractor.clone());
        visitor.visit_file(&ast);

        let bar = visitor.types.get("Bar").expect("Bar should be found");

        // Should have all 3 impl blocks tracked
        assert!(
            bar.impl_locations.len() >= 3,
            "Should track all 3 impl blocks, got {}",
            bar.impl_locations.len()
        );

        // Verify total LOC includes all impl blocks
        let struct_loc = bar
            .location
            .end_line
            .unwrap_or(bar.location.line)
            .saturating_sub(bar.location.line)
            + 1;

        let impl_loc: usize = bar
            .impl_locations
            .iter()
            .map(|loc| loc.end_line.unwrap_or(loc.line).saturating_sub(loc.line) + 1)
            .sum();

        let total_loc = struct_loc + impl_loc;

        // Struct is 1 line, each impl is ~3 lines = ~10 total
        assert!(
            total_loc >= 8,
            "LOC should include all impl blocks, got {} (struct={}, impl={})",
            total_loc,
            struct_loc,
            impl_loc
        );
    }

    #[test]
    fn test_classify_god_object_impact() {
        // Critical: >30 methods or >20 fields
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(35, 5),
            MaintainabilityImpact::Critical
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(10, 25),
            MaintainabilityImpact::Critical
        );

        // High: >20 methods or >15 fields
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(25, 5),
            MaintainabilityImpact::High
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(10, 18),
            MaintainabilityImpact::High
        );

        // Medium: everything else
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(15, 10),
            MaintainabilityImpact::Medium
        );
    }
}
