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
    pub fn analyze_enhanced(&self, path: &Path, ast: &syn::File) -> EnhancedGodObjectAnalysis {
        use super::ast_visitor::TypeVisitor;
        use super::classification_types::{EnhancedGodObjectAnalysis, GodObjectType};
        use super::metrics;
        use super::recommendation_generator;
        use crate::organization::struct_patterns;
        use syn::visit::Visit;

        // Step 1: Get comprehensive analysis (pure)
        let file_metrics = self.analyze_comprehensive(path, ast);

        // Step 2: Build per-struct metrics (pure)
        let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
        visitor.visit_file(ast);
        let per_struct_metrics = metrics::build_per_struct_metrics(&visitor);

        // Step 3: Detect patterns (pure) - reduces false positives
        let pattern_analysis = if let Some(first_struct) = visitor.types.values().next() {
            Some(struct_patterns::detect_pattern(
                first_struct,
                file_metrics.responsibility_count,
            ))
        } else {
            None
        };

        // Step 4: Determine if this is truly a god object (pure with pattern awareness)
        let is_genuine_god_object = file_metrics.is_god_object
            && !pattern_analysis
                .as_ref()
                .map(|p| p.skip_god_object_check)
                .unwrap_or(false);

        // Step 5: Classify the god object type (pure)
        let classification = if is_genuine_god_object {
            // Simple classification based on detection type
            match file_metrics.detection_type {
                super::core_types::DetectionType::GodClass => {
                    let struct_name = per_struct_metrics
                        .first()
                        .map(|s| s.name.clone())
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

        // Step 6: Generate responsibility-aware recommendation (pure)
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
    pub fn analyze_comprehensive(&self, _path: &Path, ast: &syn::File) -> GodObjectAnalysis {
        use super::ast_visitor::TypeVisitor;
        use super::classifier::{determine_confidence, group_methods_by_responsibility};
        use super::core_types::{DetectionType, GodObjectAnalysis};
        use super::metrics;
        use super::recommender::recommend_module_splits;
        use super::scoring::{calculate_god_object_score, calculate_god_object_score_weighted};
        use super::thresholds::GodObjectThresholds;
        use syn::visit::Visit;

        // Step 1: Collect data from AST
        let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
        visitor.visit_file(ast);

        let thresholds = GodObjectThresholds::default();

        // Step 2: Determine detection type (GodClass vs GodFile vs GodModule)
        let has_structs = !visitor.types.is_empty();
        let standalone_count = visitor.standalone_functions.len();
        let impl_method_count: usize = visitor.types.values().map(|t| t.method_count).sum();

        // Determine detection type based on code structure:
        // - GodModule: Many standalone functions dominating the file (>50 and >3x impl methods)
        // - GodClass: Structs with meaningful impl methods (impl methods >= standalone or impl > 0 with few standalone)
        // - GodFile: Primarily standalone functions, or structs without impl methods (data-only structs)
        let detection_type =
            if has_structs && standalone_count > 50 && standalone_count > impl_method_count * 3 {
                DetectionType::GodModule
            } else if has_structs && impl_method_count > 0 && impl_method_count >= standalone_count
            {
                // True god class: has structs with impl methods that dominate the file
                DetectionType::GodClass
            } else if has_structs && impl_method_count > 0 && standalone_count < 10 {
                // Small file with struct + impl methods - treat as god class
                DetectionType::GodClass
            } else {
                // Default to GodFile for:
                // - No structs (pure functional module)
                // - Structs with no impl methods (data-only structs)
                // - Structs with few impl methods but many standalone functions
                DetectionType::GodFile
            };

        // Step 3: Count methods (exclude tests for GodClass, include for GodFile/GodModule)
        let (method_count, method_names) = match detection_type {
            DetectionType::GodClass => {
                // For GodClass: only count impl methods, not standalone functions (Spec 118)
                // This prevents false positives for functional/procedural modules
                let standalone_set: std::collections::HashSet<_> =
                    visitor.standalone_functions.iter().collect();
                let impl_methods: Vec<_> = visitor
                    .function_complexity
                    .iter()
                    .filter(|fc| !fc.is_test && !standalone_set.contains(&fc.name))
                    .map(|fc| fc.name.clone())
                    .collect();
                (impl_methods.len(), impl_methods)
            }
            DetectionType::GodFile | DetectionType::GodModule => {
                // All functions
                let all_methods: Vec<_> = visitor
                    .function_complexity
                    .iter()
                    .map(|fc| fc.name.clone())
                    .collect();
                (all_methods.len(), all_methods)
            }
        };

        // Step 4: Calculate metrics
        let field_count: usize = visitor.types.values().map(|t| t.field_count).sum();

        // Group methods by responsibility
        let responsibility_groups = group_methods_by_responsibility(&method_names);
        let responsibility_count = responsibility_groups.len();

        // Sort responsibilities by method count (descending) so primary responsibility is most common
        let mut responsibilities_with_counts: Vec<(String, usize)> = responsibility_groups
            .iter()
            .map(|(name, methods)| (name.clone(), methods.len()))
            .collect();
        responsibilities_with_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let responsibilities: Vec<String> = responsibilities_with_counts
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        // Build method count map for all responsibilities (not just those in recommended_splits)
        let responsibility_method_counts: std::collections::HashMap<String, usize> =
            responsibilities_with_counts.into_iter().collect();

        // Calculate complexity
        let complexity_sum: u32 = visitor
            .function_complexity
            .iter()
            .filter(|fc| method_names.contains(&fc.name))
            .map(|fc| fc.cyclomatic_complexity)
            .sum();

        // Calculate actual lines of code from source content
        let lines_of_code = self
            .source_content
            .as_ref()
            .map(|content| content.lines().count())
            .unwrap_or(method_count * 15); // Fallback to estimate if no source

        // Step 5: Calculate weighted metrics
        let (weighted_method_count, avg_complexity, _, purity_distribution) =
            metrics::calculate_weighted_metrics(&visitor, &detection_type);

        // Step 6: Calculate god object score
        let god_object_score = if !visitor.function_complexity.is_empty() {
            calculate_god_object_score_weighted(
                weighted_method_count,
                field_count,
                responsibility_count,
                lines_of_code,
                avg_complexity,
                &thresholds,
            )
        } else {
            calculate_god_object_score(
                method_count,
                field_count,
                responsibility_count,
                lines_of_code,
                &thresholds,
            )
        };

        // Step 7: Determine confidence and if it's a god object
        let confidence = determine_confidence(
            method_count,
            field_count,
            responsibility_count,
            lines_of_code,
            complexity_sum,
            &thresholds,
        );

        let is_god_object = god_object_score >= 70.0;

        // Step 8: Generate recommendations
        let type_name = visitor
            .types
            .keys()
            .next()
            .map(|s| s.as_str())
            .unwrap_or("Module");
        let recommended_splits =
            recommend_module_splits(type_name, &method_names, &responsibility_groups);

        // Step 9: Calculate visibility breakdown
        let visibility_breakdown = Some(metrics::calculate_visibility_breakdown(
            &visitor,
            &method_names,
        ));

        // Step 10: Extract struct name and line for GodClass
        let (struct_name, struct_line) = match detection_type {
            DetectionType::GodClass => {
                // Get the first (dominant) struct's name and location
                visitor
                    .types
                    .values()
                    .next()
                    .map(|type_analysis| {
                        (
                            Some(type_analysis.name.clone()),
                            Some(type_analysis.location.line),
                        )
                    })
                    .unwrap_or((None, None))
            }
            DetectionType::GodFile | DetectionType::GodModule => {
                // For file/module level, no specific struct
                (None, None)
            }
        };

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
            struct_name,
            struct_line,
            visibility_breakdown,
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: super::core_types::SplitAnalysisMethod::MethodBased,
            cross_domain_severity: None,
            domain_diversity_metrics: None,
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
}
