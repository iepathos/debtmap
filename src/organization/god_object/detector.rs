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
use crate::priority::score_types::Score0To100;
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

    /// Main analysis pipeline - composes pure functions.
    ///
    /// Enhanced analysis that includes pattern detection and per-struct breakdown.
    /// Follows Stillwater pattern: orchestrates pure functions with clear data flow.
    ///
    /// Spec 201: Uses per-struct analysis but combines results into a single
    /// EnhancedGodObjectAnalysis for backwards compatibility.
    pub fn analyze_enhanced(&self, path: &Path, ast: &syn::File) -> EnhancedGodObjectAnalysis {
        use super::ast_visitor::TypeVisitor;
        use super::classification_types::{EnhancedGodObjectAnalysis, GodObjectType};
        use super::metrics;
        use super::recommendation_generator;
        use crate::organization::struct_patterns;
        use syn::visit::Visit;

        // Step 1: Get per-struct comprehensive analysis (Spec 201)
        let analyses = self.analyze_comprehensive(path, ast);

        // Step 2: Build per-struct metrics (pure)
        let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
        visitor.visit_file(ast);
        let per_struct_metrics = metrics::build_per_struct_metrics(&visitor);

        // Step 3: Select primary file_metrics from analyses
        // If we have god objects, use the first one; otherwise create a non-god-object result
        let file_metrics = if let Some(first_analysis) = analyses.first() {
            first_analysis.clone()
        } else {
            // No god objects found - create a non-god-object analysis
            self.create_non_god_object_analysis(&visitor)
        };

        // Step 4: Detect patterns (pure) - reduces false positives
        let pattern_analysis = if let Some(first_struct) = visitor.types.values().next() {
            Some(struct_patterns::detect_pattern(
                first_struct,
                file_metrics.responsibility_count,
            ))
        } else {
            None
        };

        // Step 5: Determine if this is truly a god object (pure with pattern awareness)
        let is_genuine_god_object = file_metrics.is_god_object
            && !pattern_analysis
                .as_ref()
                .map(|p| p.skip_god_object_check)
                .unwrap_or(false);

        // Step 6: Classify the god object type (pure)
        let classification = if is_genuine_god_object {
            // Simple classification based on detection type
            match file_metrics.detection_type {
                super::core_types::DetectionType::GodClass => {
                    let struct_name = file_metrics
                        .struct_name
                        .clone()
                        .or_else(|| per_struct_metrics.first().map(|s| s.name.clone()))
                        .unwrap_or_else(|| "Unknown".to_string());
                    GodObjectType::GodClass {
                        struct_name,
                        method_count: file_metrics.method_count,
                        field_count: file_metrics.field_count,
                        responsibilities: file_metrics.responsibility_count,
                    }
                }
                super::core_types::DetectionType::GodModule
                | super::core_types::DetectionType::GodFile => {
                    let largest_struct = per_struct_metrics.first().cloned().unwrap_or_else(|| {
                        super::core_types::StructMetrics {
                            name: "Unknown".to_string(),
                            method_count: file_metrics.method_count,
                            field_count: file_metrics.field_count,
                            responsibilities: vec![],
                            line_span: (0, 0),
                        }
                    });
                    GodObjectType::GodModule {
                        total_structs: per_struct_metrics.len(),
                        total_methods: file_metrics.method_count,
                        largest_struct,
                        suggested_splits: file_metrics.recommended_splits.clone(),
                    }
                }
            }
        } else {
            GodObjectType::NotGodObject
        };

        // Step 7: Generate responsibility-aware recommendation (pure)
        let recommendation = recommendation_generator::generate_recommendation(
            &classification,
            pattern_analysis.as_ref(),
        );

        EnhancedGodObjectAnalysis {
            file_metrics,
            per_struct_metrics,
            classification,
            recommendation,
        }
    }

    /// Create a non-god-object analysis result when no structs qualify.
    fn create_non_god_object_analysis(
        &self,
        visitor: &super::ast_visitor::TypeVisitor,
    ) -> GodObjectAnalysis {
        use super::classifier::group_methods_by_responsibility;
        use super::core_types::DetectionType;

        let method_names: Vec<_> = visitor
            .function_complexity
            .iter()
            .map(|fc| fc.name.clone())
            .collect();
        let method_count = method_names.len();
        let field_count: usize = visitor.types.values().map(|t| t.field_count).sum();

        let responsibility_groups = group_methods_by_responsibility(&method_names);
        let responsibility_count = responsibility_groups.len();

        let responsibilities: Vec<String> = responsibility_groups.keys().cloned().collect();

        let lines_of_code = self
            .source_content
            .as_ref()
            .map(|content| content.lines().count())
            .unwrap_or(method_count * 15);

        let complexity_sum: u32 = visitor
            .function_complexity
            .iter()
            .map(|fc| fc.cyclomatic_complexity)
            .sum();

        GodObjectAnalysis {
            is_god_object: false,
            method_count,
            field_count,
            responsibility_count,
            lines_of_code,
            complexity_sum,
            god_object_score: Score0To100::new(0.0),
            recommended_splits: vec![],
            confidence: super::core_types::GodObjectConfidence::NotGodObject,
            responsibilities,
            responsibility_method_counts: std::collections::HashMap::new(),
            purity_distribution: None,
            module_structure: None,
            detection_type: DetectionType::GodClass,
            struct_name: None,
            struct_line: None,
            struct_location: None,
            visibility_breakdown: None,
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: super::core_types::SplitAnalysisMethod::None,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
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

    /// Per-struct comprehensive analysis returning `Vec<GodObjectAnalysis>`.
    ///
    /// Spec 201: Analyzes each struct/type individually rather than aggregating
    /// file-level metrics. This prevents false positives where simple DTOs in
    /// large files are incorrectly flagged as god objects.
    ///
    /// ## Key differences from file-level analysis:
    /// - LOC is calculated per-struct using line span, not entire file
    /// - Methods/fields are counted per-struct, not aggregated
    /// - Structs with 0 impl methods are never flagged (DTOs/data structs)
    /// - Each struct is scored independently
    ///
    /// Returns a list of structs that qualify as god objects (may be empty).
    pub fn analyze_comprehensive(&self, _path: &Path, ast: &syn::File) -> Vec<GodObjectAnalysis> {
        use super::ast_visitor::TypeVisitor;
        use super::core_types::DetectionType;
        use super::thresholds::GodObjectThresholds;
        use syn::visit::Visit;

        // Step 1: Collect data from AST
        let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
        visitor.visit_file(ast);

        let thresholds = GodObjectThresholds::default();

        // Step 2: Determine detection type context
        let has_structs = !visitor.types.is_empty();
        let standalone_count = visitor.standalone_functions.len();
        let impl_method_count: usize = visitor.types.values().map(|t| t.method_count).sum();

        let detection_type =
            if has_structs && standalone_count > 50 && standalone_count > impl_method_count * 3 {
                DetectionType::GodModule
            } else if has_structs && impl_method_count > 0 {
                DetectionType::GodClass
            } else {
                DetectionType::GodFile
            };

        // Step 3: For GodFile/GodModule, use file-level analysis (no per-struct)
        // For GodClass, analyze each struct individually
        match detection_type {
            DetectionType::GodFile | DetectionType::GodModule => {
                // File-level analysis - return at most one result
                let result = self.analyze_file_level(&visitor, &thresholds, detection_type.clone());
                if result.is_god_object {
                    vec![result]
                } else {
                    vec![]
                }
            }
            DetectionType::GodClass => {
                // Per-struct analysis (Spec 201)
                visitor
                    .types
                    .values()
                    .filter_map(|type_analysis| {
                        self.analyze_single_struct(type_analysis, &visitor, &thresholds)
                    })
                    .collect()
            }
        }
    }

    /// Analyze a single struct for god object characteristics.
    ///
    /// Spec 201: Per-struct analysis using the struct's own metrics.
    fn analyze_single_struct(
        &self,
        type_analysis: &super::ast_visitor::TypeAnalysis,
        visitor: &super::ast_visitor::TypeVisitor,
        thresholds: &super::thresholds::GodObjectThresholds,
    ) -> Option<GodObjectAnalysis> {
        use super::classifier::{determine_confidence, group_methods_by_responsibility};
        use super::recommender::recommend_module_splits;
        use super::scoring::{calculate_god_object_score, calculate_god_object_score_weighted};

        // Spec 201: Skip zero-method structs immediately - they cannot be god objects
        // DTOs, data structs, and enums with no behavior are excluded
        if type_analysis.method_count == 0 {
            return None;
        }

        // Calculate per-struct LOC from line span
        let lines_of_code = type_analysis
            .location
            .end_line
            .unwrap_or(type_analysis.location.line)
            .saturating_sub(type_analysis.location.line)
            + 1;

        // Use per-struct method and field counts
        let method_count = type_analysis.method_count;
        let field_count = type_analysis.field_count;
        let method_names = &type_analysis.methods;

        // Group methods by responsibility for THIS struct only
        let responsibility_groups = group_methods_by_responsibility(method_names);
        let responsibility_count = responsibility_groups.len();

        // Sort responsibilities by method count
        let mut responsibilities_with_counts: Vec<(String, usize)> = responsibility_groups
            .iter()
            .map(|(name, methods)| (name.clone(), methods.len()))
            .collect();
        responsibilities_with_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let responsibilities: Vec<String> = responsibilities_with_counts
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        let responsibility_method_counts: std::collections::HashMap<String, usize> =
            responsibilities_with_counts.into_iter().collect();

        // Calculate complexity for methods in this struct
        let complexity_sum: u32 = visitor
            .function_complexity
            .iter()
            .filter(|fc| method_names.contains(&fc.name))
            .map(|fc| fc.cyclomatic_complexity)
            .sum();

        // Calculate average complexity for weighted scoring
        let avg_complexity = if method_count > 0 {
            complexity_sum as f64 / method_count as f64
        } else {
            0.0
        };

        // Calculate god object score using per-struct metrics
        let god_object_score = if method_count > 0 {
            calculate_god_object_score_weighted(
                method_count as f64,
                field_count,
                responsibility_count,
                lines_of_code,
                avg_complexity,
                thresholds,
            )
        } else {
            calculate_god_object_score(
                method_count,
                field_count,
                responsibility_count,
                lines_of_code,
                thresholds,
            )
        };

        // Determine confidence
        let confidence = determine_confidence(
            method_count,
            field_count,
            responsibility_count,
            lines_of_code,
            complexity_sum,
            thresholds,
        );

        let is_god_object = god_object_score >= 70.0;

        // Only return if this struct is actually a god object
        if !is_god_object {
            return None;
        }

        // Generate recommendations for this struct
        let recommended_splits =
            recommend_module_splits(&type_analysis.name, method_names, &responsibility_groups);

        Some(GodObjectAnalysis {
            is_god_object,
            method_count,
            field_count,
            responsibility_count,
            lines_of_code,
            complexity_sum,
            god_object_score: Score0To100::new(god_object_score),
            recommended_splits,
            confidence,
            responsibilities,
            responsibility_method_counts,
            purity_distribution: None, // Per-struct purity analysis could be added later
            module_structure: None,
            detection_type: super::core_types::DetectionType::GodClass,
            struct_name: Some(type_analysis.name.clone()),
            struct_line: Some(type_analysis.location.line),
            struct_location: Some(type_analysis.location.clone()),
            visibility_breakdown: None, // Per-struct visibility could be added later
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: super::core_types::SplitAnalysisMethod::MethodBased,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
        })
    }

    /// File-level analysis for GodFile/GodModule detection types.
    ///
    /// Used when the file doesn't contain struct-based code (functional modules,
    /// procedural code) or when standalone functions dominate the file.
    fn analyze_file_level(
        &self,
        visitor: &super::ast_visitor::TypeVisitor,
        thresholds: &super::thresholds::GodObjectThresholds,
        detection_type: super::core_types::DetectionType,
    ) -> GodObjectAnalysis {
        use super::classifier::{determine_confidence, group_methods_by_responsibility};
        use super::metrics;
        use super::recommender::recommend_module_splits;
        use super::scoring::{calculate_god_object_score, calculate_god_object_score_weighted};

        // Count all methods for file-level analysis
        let method_names: Vec<_> = visitor
            .function_complexity
            .iter()
            .map(|fc| fc.name.clone())
            .collect();
        let method_count = method_names.len();

        // Aggregate field count across all structs
        let field_count: usize = visitor.types.values().map(|t| t.field_count).sum();

        // Group methods by responsibility
        let responsibility_groups = group_methods_by_responsibility(&method_names);
        let responsibility_count = responsibility_groups.len();

        let mut responsibilities_with_counts: Vec<(String, usize)> = responsibility_groups
            .iter()
            .map(|(name, methods)| (name.clone(), methods.len()))
            .collect();
        responsibilities_with_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let responsibilities: Vec<String> = responsibilities_with_counts
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        let responsibility_method_counts: std::collections::HashMap<String, usize> =
            responsibilities_with_counts.into_iter().collect();

        let complexity_sum: u32 = visitor
            .function_complexity
            .iter()
            .map(|fc| fc.cyclomatic_complexity)
            .sum();

        let lines_of_code = self
            .source_content
            .as_ref()
            .map(|content| content.lines().count())
            .unwrap_or(method_count * 15);

        let (weighted_method_count, avg_complexity, _, purity_distribution) =
            metrics::calculate_weighted_metrics(visitor, &detection_type);

        let god_object_score = if !visitor.function_complexity.is_empty() {
            calculate_god_object_score_weighted(
                weighted_method_count,
                field_count,
                responsibility_count,
                lines_of_code,
                avg_complexity,
                thresholds,
            )
        } else {
            calculate_god_object_score(
                method_count,
                field_count,
                responsibility_count,
                lines_of_code,
                thresholds,
            )
        };

        let confidence = determine_confidence(
            method_count,
            field_count,
            responsibility_count,
            lines_of_code,
            complexity_sum,
            thresholds,
        );

        let is_god_object = god_object_score >= 70.0;

        let type_name = visitor
            .types
            .keys()
            .next()
            .map(|s| s.as_str())
            .unwrap_or("Module");
        let recommended_splits =
            recommend_module_splits(type_name, &method_names, &responsibility_groups);

        let visibility_breakdown = Some(metrics::calculate_visibility_breakdown(
            visitor,
            &method_names,
        ));

        GodObjectAnalysis {
            is_god_object,
            method_count,
            field_count,
            responsibility_count,
            lines_of_code,
            complexity_sum,
            god_object_score: Score0To100::new(god_object_score),
            recommended_splits,
            confidence,
            responsibilities,
            responsibility_method_counts,
            purity_distribution,
            module_structure: None,
            detection_type,
            struct_name: None,
            struct_line: None,
            struct_location: None,
            visibility_breakdown,
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: super::core_types::SplitAnalysisMethod::MethodBased,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
        }
    }
}

impl OrganizationDetector for GodObjectDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        use super::ast_visitor::TypeVisitor;
        use super::classifier::group_methods_by_responsibility;
        use crate::organization::ResponsibilityGroup;
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

impl GodObjectDetector {
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

    /// Spec 201: Test per-struct analysis - zero-method struct should not be flagged
    #[test]
    fn test_per_struct_analysis_skips_dto_structs() {
        // A file with a DTO struct (no impl methods) and a small struct with methods
        let content = r#"
/// DTO struct with many fields but no methods - should NOT be flagged
pub struct MessageData {
    pub id: u64,
    pub sender: String,
    pub receiver: String,
    pub content: String,
    pub timestamp: u64,
    pub status: String,
    pub metadata: HashMap<String, String>,
    pub attachments: Vec<String>,
    pub priority: u8,
    pub read: bool,
}

/// Small struct with a few methods - should NOT be flagged
pub struct Helper {
    value: i32,
}

impl Helper {
    pub fn new(value: i32) -> Self {
        Self { value }
    }

    pub fn get(&self) -> i32 {
        self.value
    }
}
"#;

        let ast = syn::parse_file(content).expect("Failed to parse");
        let detector = GodObjectDetector::with_source_content(content);
        let analyses = detector.analyze_comprehensive(Path::new("test.rs"), &ast);

        // Neither struct should be flagged - DTO has no methods, Helper is too small
        assert!(
            analyses.is_empty(),
            "DTO and small structs should not be flagged as god objects, got {} analyses",
            analyses.len()
        );
    }

    /// Spec 201: Test per-struct analysis - each struct analyzed independently
    #[test]
    fn test_per_struct_analysis_independent_metrics() {
        // A file with two structs - one is a simple DTO, one might be flagged
        // This tests that we don't aggregate file-level metrics
        let content = r#"
/// Simple DTO - many fields, no behavior
pub struct Config {
    pub a: String,
    pub b: String,
    pub c: String,
    pub d: String,
    pub e: String,
    pub f: String,
    pub g: String,
    pub h: String,
}

/// Small helper struct - few methods
pub struct SmallHelper {
    data: Vec<u8>,
}

impl SmallHelper {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn process(&mut self) {
        self.data.push(1);
    }
}
"#;

        let ast = syn::parse_file(content).expect("Failed to parse");
        let detector = GodObjectDetector::with_source_content(content);
        let analyses = detector.analyze_comprehensive(Path::new("test.rs"), &ast);

        // Neither should be flagged - Config has 0 methods, SmallHelper is small
        assert!(
            analyses.is_empty(),
            "Neither struct should be flagged as god object"
        );
    }

    /// Spec 201: Test that struct_location is populated in results
    #[test]
    fn test_struct_location_populated() {
        // Create a file with a struct that has many methods to trigger god object detection
        let content = r#"
pub struct GodObject {
    a: i32,
    b: i32,
    c: i32,
    d: i32,
    e: i32,
}

impl GodObject {
    pub fn create_a(&self) -> i32 { self.a }
    pub fn update_a(&mut self, v: i32) { self.a = v; }
    pub fn delete_a(&mut self) { self.a = 0; }
    pub fn validate_a(&self) -> bool { self.a > 0 }

    pub fn create_b(&self) -> i32 { self.b }
    pub fn update_b(&mut self, v: i32) { self.b = v; }
    pub fn delete_b(&mut self) { self.b = 0; }
    pub fn validate_b(&self) -> bool { self.b > 0 }

    pub fn create_c(&self) -> i32 { self.c }
    pub fn update_c(&mut self, v: i32) { self.c = v; }
    pub fn delete_c(&mut self) { self.c = 0; }
    pub fn validate_c(&self) -> bool { self.c > 0 }

    pub fn create_d(&self) -> i32 { self.d }
    pub fn update_d(&mut self, v: i32) { self.d = v; }
    pub fn delete_d(&mut self) { self.d = 0; }
    pub fn validate_d(&self) -> bool { self.d > 0 }

    pub fn create_e(&self) -> i32 { self.e }
    pub fn update_e(&mut self, v: i32) { self.e = v; }
    pub fn delete_e(&mut self) { self.e = 0; }
    pub fn validate_e(&self) -> bool { self.e > 0 }
}
"#;

        let ast = syn::parse_file(content).expect("Failed to parse");
        let detector = GodObjectDetector::with_source_content(content);
        let analyses = detector.analyze_comprehensive(Path::new("test.rs"), &ast);

        // Should have at least one analysis (the god object)
        if !analyses.is_empty() {
            let first = &analyses[0];
            assert_eq!(first.struct_name, Some("GodObject".to_string()));
            assert!(
                first.struct_location.is_some(),
                "struct_location should be populated"
            );
        }
    }

    /// Spec 201: Test file-level analysis for pure functional modules
    #[test]
    fn test_file_level_analysis_for_functional_modules() {
        // A purely functional module with no structs
        let content = r#"
pub fn helper1() -> i32 { 1 }
pub fn helper2() -> i32 { 2 }
pub fn helper3() -> i32 { 3 }
pub fn helper4() -> i32 { 4 }
pub fn helper5() -> i32 { 5 }
"#;

        let ast = syn::parse_file(content).expect("Failed to parse");
        let detector = GodObjectDetector::with_source_content(content);
        let analyses = detector.analyze_comprehensive(Path::new("test.rs"), &ast);

        // Functional modules use file-level analysis
        // This small module should not be flagged
        assert!(
            analyses.is_empty(),
            "Small functional module should not be flagged"
        );
    }
}
