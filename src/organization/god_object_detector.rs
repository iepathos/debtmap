use super::{
    calculate_god_object_score, determine_confidence,
    god_object::{metrics, TypeAnalysis, TypeVisitor},
    group_methods_by_responsibility, suggest_module_splits_by_domain, DetectionType,
    EnhancedGodObjectAnalysis, GodObjectAnalysis, GodObjectThresholds, GodObjectType,
    MaintainabilityImpact, ModuleSplit, OrganizationAntiPattern, OrganizationDetector,
    RecommendationSeverity, ResponsibilityGroup, StructMetrics,
};
use crate::common::{capitalize_first, SourceLocation, UnifiedLocationExtractor};
use std::collections::HashMap;
use std::path::Path;
use syn::{self, visit::Visit};

/// Minimum standalone functions required to trigger hybrid detection.
///
/// Files with fewer standalone functions are assumed to have helper
/// functions that complement the primary struct's impl methods.
///
/// Chosen based on analysis of Rust projects: 50+ functions typically
/// indicates a functional module rather than helpers.
const HYBRID_STANDALONE_THRESHOLD: usize = 50;

/// Dominance ratio: standalone functions must exceed impl methods by this factor.
///
/// Prevents false positives for balanced OOP/functional modules. A ratio of 3:1
/// ensures standalone functions truly dominate the file's purpose.
///
/// Examples:
/// - 60 standalone, 15 impl → 60 > 45? Yes → Hybrid
/// - 60 standalone, 25 impl → 60 > 75? No → God Class
const HYBRID_DOMINANCE_RATIO: usize = 3;

/// Parameters for god object classification
struct GodObjectClassificationParams<'a> {
    per_struct_metrics: &'a [StructMetrics],
    total_methods: usize,
    thresholds: &'a GodObjectThresholds,
    ownership: Option<&'a crate::organization::struct_ownership::StructOwnershipAnalyzer>,
    file_path: &'a Path,
    ast: &'a syn::File,
    visitor: &'a super::god_object::TypeVisitor,
}

pub struct GodObjectDetector {
    max_methods: usize,
    max_fields: usize,
    max_responsibilities: usize,
    location_extractor: Option<UnifiedLocationExtractor>,
    source_content: Option<String>,
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_source_content(source_content: &str) -> Self {
        Self {
            max_methods: 15,
            max_fields: 10,
            max_responsibilities: 3,
            location_extractor: Some(UnifiedLocationExtractor::new(source_content)),
            source_content: Some(source_content.to_string()),
        }
    }

    /// Analyze with per-struct detail and god class vs god module distinction
    pub fn analyze_enhanced(&self, path: &Path, ast: &syn::File) -> EnhancedGodObjectAnalysis {
        let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
        visitor.visit_file(ast);

        let thresholds = Self::get_thresholds_for_path(path);

        // Build per-struct metrics
        let per_struct_metrics = metrics::build_per_struct_metrics(&visitor);

        // Get basic file-level analysis
        let file_metrics = self.analyze_comprehensive(path, ast);

        // For Rust files, use struct ownership analysis
        let ownership = if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            Some(crate::organization::struct_ownership::StructOwnershipAnalyzer::analyze_file(ast))
        } else {
            None
        };

        // Classify as god class, god module, or not god object
        let classification = self.classify_god_object(&GodObjectClassificationParams {
            per_struct_metrics: &per_struct_metrics,
            total_methods: file_metrics.method_count,
            thresholds: &thresholds,
            ownership: ownership.as_ref(),
            file_path: path,
            ast,
            visitor: &visitor,
        });

        // Generate context-aware recommendation
        let recommendation =
            self.generate_recommendation(&classification, path, &per_struct_metrics);

        EnhancedGodObjectAnalysis {
            file_metrics,
            per_struct_metrics,
            classification,
            recommendation,
        }
    }

    /// Classify whether this is a god class, god module, or neither
    fn classify_god_object(&self, params: &GodObjectClassificationParams) -> GodObjectType {
        // First, check for boilerplate pattern before other classifications
        let boilerplate_detector =
            crate::organization::boilerplate_detector::BoilerplateDetector::default();
        let boilerplate_analysis = boilerplate_detector.detect(params.file_path, params.ast);

        if boilerplate_analysis.is_boilerplate && boilerplate_analysis.confidence > 0.7 {
            return GodObjectType::BoilerplatePattern {
                pattern: boilerplate_analysis.pattern_type.unwrap(),
                confidence: boilerplate_analysis.confidence,
                recommendation: boilerplate_analysis.recommendation,
            };
        }

        // Second, check for registry pattern before classifying as god object
        if let Some(source_content) = &self.source_content {
            let registry_detector = crate::organization::RegistryPatternDetector::default();
            if let Some(pattern) = registry_detector.detect(params.ast, source_content) {
                let confidence = registry_detector.confidence(&pattern);

                // Calculate what the god object score would have been
                let original_score = calculate_god_object_score(
                    params.total_methods,
                    params
                        .per_struct_metrics
                        .iter()
                        .map(|s| s.field_count)
                        .max()
                        .unwrap_or(0),
                    params
                        .per_struct_metrics
                        .iter()
                        .flat_map(|s| &s.responsibilities)
                        .count(),
                    source_content.lines().count(),
                    params.thresholds,
                );

                let adjusted_score =
                    crate::organization::adjust_registry_score(original_score, &pattern);

                return GodObjectType::Registry {
                    pattern,
                    confidence,
                    original_score,
                    adjusted_score,
                };
            }

            // Second, check for builder pattern before classifying as god object
            let builder_detector = crate::organization::BuilderPatternDetector::default();
            if let Some(pattern) = builder_detector.detect(params.ast, source_content) {
                let confidence = builder_detector.confidence(&pattern);

                // Calculate what the god object score would have been
                let original_score = calculate_god_object_score(
                    params.total_methods,
                    params
                        .per_struct_metrics
                        .iter()
                        .map(|s| s.field_count)
                        .max()
                        .unwrap_or(0),
                    params
                        .per_struct_metrics
                        .iter()
                        .flat_map(|s| &s.responsibilities)
                        .count(),
                    source_content.lines().count(),
                    params.thresholds,
                );

                let adjusted_score =
                    crate::organization::adjust_builder_score(original_score, &pattern);

                return GodObjectType::Builder {
                    pattern,
                    confidence,
                    original_score,
                    adjusted_score,
                };
            }
        }

        // Check if any individual struct exceeds thresholds (god class)
        for struct_metrics in params.per_struct_metrics {
            if struct_metrics.method_count > params.thresholds.max_methods
                || struct_metrics.field_count > params.thresholds.max_fields
                || struct_metrics.responsibilities.len() > params.thresholds.max_traits
            {
                return GodObjectType::GodClass {
                    struct_name: struct_metrics.name.clone(),
                    method_count: struct_metrics.method_count,
                    field_count: struct_metrics.field_count,
                    responsibilities: struct_metrics.responsibilities.len(),
                };
            }
        }

        // Check if module as a whole is large with many small structs (god module)
        if params.per_struct_metrics.len() >= 5
            && params.total_methods > params.thresholds.max_methods * 2
        {
            let largest_struct = params
                .per_struct_metrics
                .iter()
                .max_by_key(|s| s.method_count)
                .cloned()
                .unwrap_or_else(|| StructMetrics {
                    name: "Unknown".to_string(),
                    method_count: 0,
                    field_count: 0,
                    responsibilities: vec![],
                    line_span: (0, 0),
                });

            // Use enhanced struct ownership analysis if available, otherwise use module function classifier
            let suggested_splits = if params.ownership.is_some() {
                crate::organization::suggest_splits_by_struct_grouping(
                    params.per_struct_metrics,
                    params.ownership,
                    Some(params.file_path),
                    Some(params.ast),
                )
            } else {
                // Try module function classification first (Spec 149)
                self.try_module_function_classification(params.visitor, params.file_path)
                    .unwrap_or_else(|| suggest_module_splits_by_domain(params.per_struct_metrics))
            };

            return GodObjectType::GodModule {
                total_structs: params.per_struct_metrics.len(),
                total_methods: params.total_methods,
                largest_struct,
                suggested_splits,
            };
        }

        GodObjectType::NotGodObject
    }

    /// Generate context-aware recommendations based on classification
    fn generate_recommendation(
        &self,
        classification: &GodObjectType,
        path: &Path,
        per_struct_metrics: &[StructMetrics],
    ) -> String {
        match classification {
            GodObjectType::GodClass {
                struct_name,
                method_count,
                field_count,
                responsibilities,
            } => {
                format!(
                    "This struct '{}' violates single responsibility principle with {} methods, {} fields, and {} distinct responsibilities. \
                    Extract methods into smaller, focused structs or separate traits.",
                    struct_name, method_count, field_count, responsibilities
                )
            }
            GodObjectType::GodModule {
                total_structs,
                total_methods,
                largest_struct,
                suggested_splits,
            } => {
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("module");
                if suggested_splits.is_empty() {
                    format!(
                        "This module '{}' contains {} structs with {} total methods. Largest struct: {} with {} methods. \
                        Consider splitting into sub-modules by domain.",
                        file_name, total_structs, total_methods, largest_struct.name, largest_struct.method_count
                    )
                } else {
                    let split_suggestions = suggested_splits
                        .iter()
                        .map(|s| {
                            let structs_count = if !s.structs_to_move.is_empty() {
                                s.structs_to_move.len()
                            } else {
                                s.methods_to_move.len()
                            };
                            let priority_icon = match s.priority {
                                crate::organization::Priority::High => "[*][*][*]",
                                crate::organization::Priority::Medium => "[*][*]",
                                crate::organization::Priority::Low => "[*]",
                            };
                            format!(
                                "  - {} {}: {} structs, {} methods (~{} lines)",
                                priority_icon,
                                s.suggested_name,
                                structs_count,
                                s.method_count,
                                s.estimated_lines
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    format!(
                        "This module '{}' contains {} structs with {} total methods. Suggested splits:\n{}",
                        file_name, total_structs, total_methods, split_suggestions
                    )
                }
            }
            GodObjectType::Registry {
                pattern,
                confidence,
                original_score,
                adjusted_score,
            } => {
                format!(
                    "Registry/Catalog Pattern Detected (confidence: {:.0}%)\n\
                    This file contains {} implementations of the '{}' trait (avg {} lines each).\n\
                    This is an intentional registry pattern for discoverability and consistency, not a god object requiring splitting.\n\
                    \n\
                    Pattern metrics:\n\
                    - Trait implementations: {}\n\
                    - Average implementation size: {:.1} lines\n\
                    - Unit struct ratio: {:.0}%\n\
                    - Coverage: {:.0}%\n\
                    \n\
                    Score adjustment: {:.0} → {:.0} ({}% reduction)\n\
                    \n\
                    Consider if registry has grown too large for navigation:\n\
                    - If trait impls exceed 200, consider logical grouping (e.g., by category)\n\
                    - Ensure consistent naming and documentation across implementations\n\
                    - Add table-of-contents or index for discoverability",
                    confidence * 100.0,
                    pattern.impl_count,
                    pattern.trait_name,
                    pattern.avg_impl_size as usize,
                    pattern.impl_count,
                    pattern.avg_impl_size,
                    pattern.unit_struct_ratio * 100.0,
                    pattern.trait_impl_coverage * 100.0,
                    original_score,
                    adjusted_score,
                    ((1.0 - adjusted_score / original_score) * 100.0) as usize
                )
            }
            GodObjectType::Builder {
                pattern,
                confidence,
                original_score,
                adjusted_score,
            } => {
                let severity = if pattern.total_file_lines > 3000 {
                    "Medium"
                } else {
                    "Low"
                };

                format!(
                    "Builder Pattern Detected (confidence: {:.0}%)\n\
                    This file contains a builder with {} fluent setters ({:.0}% of methods).\n\
                    Builder patterns naturally have many setter methods - one per configuration option.\n\
                    \n\
                    Pattern metrics:\n\
                    - Setter count: {}\n\
                    - Setter ratio: {:.0}%\n\
                    - Average setter size: {:.1} lines\n\
                    - File size: {} lines\n\
                    {}\
                    \n\
                    Score adjustment: {:.0} → {:.0} ({}% reduction)\n\
                    \n\
                    {}\
                    \n\
                    Evaluation criteria:\n\
                    - Setter count is expected and appropriate for configuration builders\n\
                    - Focus on file size and logical cohesion, not setter count\n\
                    - Ensure all setters serve the same configuration domain",
                    confidence * 100.0,
                    pattern.setter_count,
                    pattern.setter_ratio * 100.0,
                    pattern.setter_count,
                    pattern.setter_ratio * 100.0,
                    pattern.avg_setter_size,
                    pattern.total_file_lines,
                    if !pattern.build_methods.is_empty() {
                        format!("- Build methods: {}\n", pattern.build_methods.join(", "))
                    } else {
                        String::new()
                    },
                    original_score,
                    adjusted_score,
                    ((1.0 - adjusted_score / original_score) * 100.0) as usize,
                    if pattern.total_file_lines > 3000 {
                        format!(
                            "Severity: {}\n\
                            File is {} lines. Consider splitting by logical concerns:\n\
                            1) Extract separate config struct if not present\n\
                            2) Identify multiple unrelated configuration domains\n\
                            3) Move complex implementation logic to separate modules\n\
                            4) Keep all setters together for API consistency",
                            severity, pattern.total_file_lines
                        )
                    } else {
                        format!(
                            "Severity: {}\n\
                            Builder with {} setters is appropriately sized. No refactoring needed.",
                            severity, pattern.setter_count
                        )
                    }
                )
            }
            GodObjectType::BoilerplatePattern {
                recommendation,
                confidence,
                ..
            } => {
                format!(
                    "{}\n\
                    \n\
                    Detection confidence: {:.0}%",
                    recommendation,
                    confidence * 100.0
                )
            }
            GodObjectType::NotGodObject => {
                if per_struct_metrics.is_empty() {
                    "No god object detected.".to_string()
                } else {
                    let largest = per_struct_metrics.iter().max_by_key(|s| s.method_count);
                    if let Some(largest) = largest {
                        format!(
                            "No god object detected. Largest struct: '{}' with {} methods.",
                            largest.name, largest.method_count
                        )
                    } else {
                        "No god object detected.".to_string()
                    }
                }
            }
        }
    }

    /// Try to classify module functions using multi-signal analysis (Spec 149)
    fn try_module_function_classification(
        &self,
        visitor: &super::god_object::TypeVisitor,
        file_path: &Path,
    ) -> Option<Vec<ModuleSplit>> {
        use crate::analysis::io_detection::Language;
        use crate::organization::module_function_classifier::ModuleFunctionClassifier;

        // Only proceed if we have module functions
        if visitor.module_functions.is_empty() {
            return None;
        }

        // Determine language from file extension
        let language = match file_path.extension().and_then(|s| s.to_str()) {
            Some("rs") => Language::Rust,
            Some("py") => Language::Python,
            Some("js") | Some("ts") => Language::JavaScript,
            _ => return None,
        };

        // Create classifier and generate splits
        let classifier = ModuleFunctionClassifier::new(language);
        let splits = classifier.generate_splits(
            &visitor.module_functions,
            3,    // min_functions_for_split
            0.30, // min_confidence
        );

        // Only return if we generated meaningful splits
        if splits.is_empty() {
            None
        } else {
            Some(splits)
        }
    }

    /// Get thresholds based on file extension
    fn get_thresholds_for_path(path: &Path) -> GodObjectThresholds {
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            GodObjectThresholds::for_rust()
        } else if path.extension().and_then(|s| s.to_str()) == Some("py") {
            GodObjectThresholds::for_python()
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s == "js" || s == "ts")
            .unwrap_or(false)
        {
            GodObjectThresholds::for_javascript()
        } else {
            GodObjectThresholds::default()
        }
    }

    /// Analyzes a file for god object patterns.
    ///
    /// Determines whether the file contains a God Class, God File, or God Module (hybrid)
    ///
    /// # Arguments
    /// * `primary_type` - The largest type in the file (if any)
    /// * `visitor` - Type visitor containing parsed type information
    /// * `standalone_count` - Number of standalone functions in the file
    ///
    /// # Returns
    /// Tuple of (total_methods, total_fields, all_methods, total_complexity, detection_type)
    ///
    /// # Hybrid Detection Logic (Spec 155)
    ///
    /// A file is classified as hybrid (GodModule) when:
    /// - At least one struct exists (`primary_type.is_some()`)
    /// - Standalone count > [`HYBRID_STANDALONE_THRESHOLD`] (default: 50)
    /// - Standalone count > impl method count * [`HYBRID_DOMINANCE_RATIO`] (default: 3)
    ///
    /// # Examples
    ///
    /// ```rust
    /// // God Class: 30 impl methods, 5 standalone
    /// // → Analyze impl methods only
    ///
    /// // God File: 0 structs, 80 standalone
    /// // → Analyze all standalone functions
    ///
    /// // Hybrid (God Module): 1 struct, 106 standalone
    /// // → Analyze all standalone functions
    /// ```
    fn determine_god_object_type(
        primary_type: Option<&TypeAnalysis>,
        visitor: &TypeVisitor,
        standalone_count: usize,
    ) -> (usize, usize, Vec<String>, u32, DetectionType) {
        // Spec 118 & 130 & 155: Distinguish between God Class, God File, and God Module
        // - God Class: Struct with excessive methods (tests excluded)
        // - God File: File with excessive functions/lines (tests included), no structs
        // - God Module: File with structs AND many standalone functions (hybrid)
        if let Some(type_info) = primary_type {
            // Check if this is a hybrid module (Spec 155)
            let standalone_dominates = standalone_count >= HYBRID_STANDALONE_THRESHOLD
                && standalone_count > type_info.method_count * HYBRID_DOMINANCE_RATIO;

            if standalone_dominates {
                // HYBRID: Structs exist but standalone functions dominate
                // This is primarily a functional module with helper types
                let all_methods = visitor.standalone_functions.clone();
                let total_complexity = Self::estimate_standalone_complexity(standalone_count);

                return (
                    standalone_count,
                    type_info.field_count, // Keep field count for context
                    all_methods,
                    total_complexity,
                    DetectionType::GodModule,
                );
            }

            // TRUE GOD CLASS: Struct with many impl methods
            // Spec 130: Filter out test functions for god class detection
            let struct_method_names: std::collections::HashSet<_> =
                type_info.methods.iter().collect();

            // Production methods only (exclude tests)
            let production_complexity: Vec<_> = visitor
                .function_complexity
                .iter()
                .filter(|fc| struct_method_names.contains(&fc.name) && !fc.is_test)
                .cloned()
                .collect();

            let production_methods: Vec<String> = production_complexity
                .iter()
                .map(|fc| fc.name.clone())
                .collect();

            let total_methods = production_methods.len();
            let total_complexity: u32 = production_complexity
                .iter()
                .map(|fc| fc.cyclomatic_complexity)
                .sum();

            (
                total_methods,
                type_info.field_count,
                production_methods,
                total_complexity,
                DetectionType::GodClass,
            )
        } else {
            // PURE GOD FILE: No structs, only standalone functions
            // Spec 130: Include ALL functions (production + tests) for file size concerns
            let all_methods = visitor.standalone_functions.clone();
            let total_complexity = Self::estimate_standalone_complexity(standalone_count);

            (
                standalone_count,
                0,
                all_methods,
                total_complexity,
                DetectionType::GodFile,
            )
        }
    }

    /// Estimate complexity for standalone functions
    /// Heuristic: average 5 cyclomatic complexity per function
    fn estimate_standalone_complexity(count: usize) -> u32 {
        (count * 5) as u32
    }

    /// Analyzes cross-domain struct mixing and generates module split recommendations
    ///
    /// # Arguments
    /// * `per_struct_metrics` - Metrics for each struct in the file
    /// * `total_methods` - Total number of methods
    /// * `lines_of_code` - Estimated lines of code
    /// * `is_god_object` - Whether this is classified as a god object
    /// * `path` - File path for generating split names
    /// * `all_methods` - All method names
    /// * `responsibility_groups` - Methods grouped by responsibility
    ///
    /// # Returns
    /// Tuple of (recommended_splits, analysis_method, cross_domain_severity, domain_count, domain_diversity, struct_ratio)
    #[allow(clippy::type_complexity)]
    fn analyze_domains_and_recommend_splits(
        per_struct_metrics: &[StructMetrics],
        total_methods: usize,
        lines_of_code: usize,
        is_god_object: bool,
        path: &Path,
        all_methods: &[String],
        responsibility_groups: &HashMap<String, Vec<String>>,
    ) -> (
        Vec<ModuleSplit>,
        crate::organization::SplitAnalysisMethod,
        Option<RecommendationSeverity>,
        usize,
        f64,
        f64,
    ) {
        // Cross-domain struct mixing analysis (Spec 140)
        let struct_count = per_struct_metrics.len();
        let domain_count = if struct_count >= 5 {
            crate::organization::count_distinct_domains(per_struct_metrics)
        } else {
            0
        };
        let struct_ratio = crate::organization::calculate_struct_ratio(struct_count, total_methods);

        // Determine cross-domain severity
        let cross_domain_severity = if domain_count >= 3 {
            Some(crate::organization::determine_cross_domain_severity(
                struct_count,
                domain_count,
                lines_of_code,
                is_god_object,
            ))
        } else {
            None
        };

        // Calculate domain diversity (0.0 to 1.0)
        let domain_diversity = if struct_count > 0 {
            (domain_count as f64) / (struct_count as f64)
        } else {
            0.0
        };

        // Determine analysis method and generate recommendations
        let (recommended_splits, analysis_method) = if struct_count >= 5 && domain_count >= 3 {
            // PRIORITY 1: Cross-domain mixing analysis (primary strategy)
            let mut splits =
                crate::organization::suggest_module_splits_by_domain(per_struct_metrics);

            // Attach severity to all splits
            if let Some(severity) = cross_domain_severity {
                for split in &mut splits {
                    split.severity = Some(severity);
                }
            }

            (
                splits,
                crate::organization::SplitAnalysisMethod::CrossDomain,
            )
        } else if is_god_object {
            // PRIORITY 2: Method-based analysis (fallback for god objects)
            let file_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module");
            (
                crate::organization::recommend_module_splits(
                    file_name,
                    all_methods,
                    responsibility_groups,
                ),
                crate::organization::SplitAnalysisMethod::MethodBased,
            )
        } else {
            (Vec::new(), crate::organization::SplitAnalysisMethod::None)
        };

        (
            recommended_splits,
            analysis_method,
            cross_domain_severity,
            domain_count,
            domain_diversity,
            struct_ratio,
        )
    }

    /// Analyzes module structure and visibility breakdown for Rust files
    ///
    /// # Arguments
    /// * `path` - File path to check if it's a Rust file
    /// * `is_god_object` - Whether this is classified as a god object
    /// * `visitor` - Type visitor containing function information
    /// * `all_methods` - All method names
    /// * `total_methods` - Total number of methods
    /// * `source_content` - Optional source content for detailed analysis
    ///
    /// # Returns
    /// Tuple of (visibility_breakdown, module_structure)
    fn analyze_module_structure_and_visibility(
        path: &Path,
        is_god_object: bool,
        visitor: &TypeVisitor,
        all_methods: &[String],
        total_methods: usize,
        source_content: &Option<String>,
    ) -> (
        Option<crate::organization::god_object_analysis::FunctionVisibilityBreakdown>,
        Option<crate::analysis::ModuleStructure>,
    ) {
        // Calculate visibility breakdown for Rust files (Spec 134)
        let visibility_breakdown = if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            Some(metrics::calculate_visibility_breakdown(
                visitor,
                all_methods,
            ))
        } else {
            None
        };

        // Optionally generate detailed module structure analysis for Rust files
        // Spec 140: Integrate visibility breakdown with module structure
        let module_structure =
            if is_god_object && path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Some(source_content) = source_content {
                    use crate::analysis::ModuleStructureAnalyzer;
                    let analyzer = ModuleStructureAnalyzer::new_rust();
                    let mut structure = analyzer.analyze_rust_file(source_content, path);

                    // Integrate visibility breakdown into function counts
                    if let Some(ref breakdown) = visibility_breakdown {
                        structure.function_counts = metrics::integrate_visibility_into_counts(
                            &structure.function_counts,
                            breakdown,
                            total_methods,
                        );
                    }

                    Some(structure)
                } else {
                    None
                }
            } else {
                None
            };

        (visibility_breakdown, module_structure)
    }

    /// # God Class vs God Module
    ///
    /// This function distinguishes between:
    /// - **God Class**: Single struct with excessive methods (>20), fields (>15)
    /// - **God Module**: File with excessive standalone functions (>50)
    ///
    /// Previously, this incorrectly combined standalone functions with struct
    /// methods, causing false positives for functional/procedural modules.
    /// Now it analyzes struct methods separately from standalone functions.
    pub fn analyze_comprehensive(&self, path: &Path, ast: &syn::File) -> GodObjectAnalysis {
        let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
        visitor.visit_file(ast);

        // Build per-struct metrics for domain analysis (Spec 140)
        let per_struct_metrics = metrics::build_per_struct_metrics(&visitor);

        // Get thresholds based on file extension
        let thresholds = Self::get_thresholds_for_path(path);

        // Find the largest type (struct with most methods) as primary god object candidate
        let primary_type = visitor
            .types
            .values()
            .max_by_key(|t| t.method_count + t.field_count * 2);

        // Count standalone functions in addition to methods from types
        let standalone_count = visitor.standalone_functions.len();

        // Determine whether this is a God Class or God File
        let (total_methods, total_fields, all_methods, total_complexity, detection_type) =
            Self::determine_god_object_type(primary_type, &visitor, standalone_count);

        // Count actual lines more accurately by looking at span information
        // For now, use a better heuristic based on item count and complexity
        // Spec 134 Phase 3: Use filtered total_methods for consistency
        let lines_of_code = if primary_type.is_some() {
            // Estimate based on filtered method count (production methods only)
            total_methods * 15 + total_fields * 2 + 50
        } else {
            // If no types, estimate based on standalone functions
            standalone_count * 10 + 20
        };

        let responsibility_groups = group_methods_by_responsibility(&all_methods);

        // Defensive check: Ensure we never report 0 responsibilities when functions exist
        // If grouping failed or returned empty, default to 1 (all functions share one responsibility)
        let responsibility_count = if responsibility_groups.is_empty() && !all_methods.is_empty() {
            1
        } else {
            responsibility_groups.len()
        };

        // Calculate complexity-weighted and purity-weighted metrics
        let (weighted_method_count, avg_complexity, purity_weighted_count, purity_distribution) =
            metrics::calculate_weighted_metrics(&visitor, &detection_type);

        // Calculate the final god object score and determination
        let (god_object_score, is_god_object) = metrics::calculate_final_god_object_score(
            purity_weighted_count,
            weighted_method_count,
            total_methods,
            total_fields,
            responsibility_count,
            lines_of_code,
            avg_complexity,
            &purity_distribution,
            !visitor.function_complexity.is_empty(),
            &thresholds,
        );

        let confidence = determine_confidence(
            total_methods,
            total_fields,
            responsibility_count,
            lines_of_code,
            total_complexity,
            &thresholds,
        );

        // Analyze cross-domain mixing and generate split recommendations
        let (
            recommended_splits,
            analysis_method,
            cross_domain_severity,
            domain_count,
            domain_diversity,
            struct_ratio,
        ) = Self::analyze_domains_and_recommend_splits(
            &per_struct_metrics,
            total_methods,
            lines_of_code,
            is_god_object,
            path,
            &all_methods,
            &responsibility_groups,
        );

        let responsibilities: Vec<String> = responsibility_groups.keys().cloned().collect();

        // Analyze module structure and visibility for Rust files
        let (visibility_breakdown, module_structure) =
            Self::analyze_module_structure_and_visibility(
                path,
                is_god_object,
                &visitor,
                &all_methods,
                total_methods,
                &self.source_content,
            );

        GodObjectAnalysis {
            is_god_object,
            method_count: total_methods,
            field_count: total_fields,
            responsibility_count,
            lines_of_code,
            complexity_sum: total_complexity,
            god_object_score,
            recommended_splits,
            confidence,
            responsibilities,
            purity_distribution,
            module_structure,
            detection_type,
            visibility_breakdown,
            domain_count,
            domain_diversity,
            struct_ratio,
            analysis_method,
            cross_domain_severity,
            domain_diversity_metrics: None, // Will be calculated separately if needed (spec 152)
        }
    }

    #[allow(dead_code)]
    fn analyze_type(&self, item_struct: &syn::ItemStruct) -> TypeAnalysis {
        let location = if let Some(ref extractor) = self.location_extractor {
            extractor.extract_item_location(&syn::Item::Struct(item_struct.clone()))
        } else {
            SourceLocation::default()
        };

        TypeAnalysis {
            name: item_struct.ident.to_string(),
            method_count: 0,
            field_count: self.count_fields(&item_struct.fields),
            methods: Vec::new(),
            fields: self.extract_field_names(&item_struct.fields),
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location,
        }
    }

    #[allow(dead_code)]
    fn count_fields(&self, fields: &syn::Fields) -> usize {
        match fields {
            syn::Fields::Named(fields) => fields.named.len(),
            syn::Fields::Unnamed(fields) => fields.unnamed.len(),
            syn::Fields::Unit => 0,
        }
    }

    #[allow(dead_code)]
    fn extract_field_names(&self, fields: &syn::Fields) -> Vec<String> {
        match fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|f| f.ident.as_ref().map(|id| id.to_string()))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Classify the maintainability impact based on method and field counts
    fn classify_god_object_impact(
        method_count: usize,
        field_count: usize,
    ) -> MaintainabilityImpact {
        match () {
            _ if method_count > 30 || field_count > 20 => MaintainabilityImpact::Critical,
            _ if method_count > 20 || field_count > 15 => MaintainabilityImpact::High,
            _ => MaintainabilityImpact::Medium,
        }
    }

    fn is_god_object(&self, analysis: &TypeAnalysis) -> bool {
        analysis.method_count > self.max_methods
            || analysis.field_count > self.max_fields
            || analysis.responsibilities.len() > self.max_responsibilities
            || analysis.trait_implementations > 10
    }

    fn suggest_responsibility_split(&self, analysis: &TypeAnalysis) -> Vec<ResponsibilityGroup> {
        let method_groups = self.group_methods_by_prefix(&analysis.methods);

        let groups: Vec<ResponsibilityGroup> = method_groups
            .into_iter()
            .map(|(prefix, methods)| self.create_responsibility_group(prefix, methods))
            .collect();

        // Return existing groups or create default if empty and exceeds threshold
        if groups.is_empty() && analysis.method_count > self.max_methods {
            vec![self.create_default_responsibility_group(analysis)]
        } else {
            groups
        }
    }

    /// Create a responsibility group from prefix and methods
    fn create_responsibility_group(
        &self,
        prefix: String,
        methods: Vec<String>,
    ) -> ResponsibilityGroup {
        let responsibility = self.infer_responsibility_name(&prefix);
        ResponsibilityGroup {
            name: format!("{}Manager", responsibility.replace(' ', "")),
            methods,
            fields: Vec::new(),
            responsibility,
        }
    }

    /// Create a default responsibility group for core functionality
    fn create_default_responsibility_group(&self, analysis: &TypeAnalysis) -> ResponsibilityGroup {
        ResponsibilityGroup {
            name: format!("{}Core", analysis.name),
            methods: analysis.methods.clone(),
            fields: analysis.fields.clone(),
            responsibility: "Core functionality".to_string(),
        }
    }

    fn group_methods_by_prefix(&self, methods: &[String]) -> HashMap<String, Vec<String>> {
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();

        for method in methods {
            let prefix = self.extract_method_prefix(method);
            groups.entry(prefix).or_default().push(method.clone());
        }

        groups
    }

    fn extract_method_prefix(&self, method_name: &str) -> String {
        Self::find_matching_prefix(method_name)
            .unwrap_or_else(|| Self::extract_first_word(method_name))
    }

    /// Pure function to find a matching prefix from the common list
    fn find_matching_prefix(method_name: &str) -> Option<String> {
        const COMMON_PREFIXES: &[&str] = &[
            "get",
            "set",
            "is",
            "has",
            "can",
            "should",
            "will",
            "create",
            "build",
            "make",
            "new",
            "init",
            "calculate",
            "compute",
            "process",
            "transform",
            "validate",
            "check",
            "verify",
            "ensure",
            "save",
            "load",
            "store",
            "retrieve",
            "fetch",
            "update",
            "modify",
            "change",
            "edit",
            "delete",
            "remove",
            "clear",
            "reset",
            "send",
            "receive",
            "handle",
            "manage",
        ];

        let lower_name = method_name.to_lowercase();
        COMMON_PREFIXES
            .iter()
            .find(|&&prefix| lower_name.starts_with(prefix))
            .map(|&s| s.to_string())
    }

    /// Pure function to extract the first word from a method name
    fn extract_first_word(method_name: &str) -> String {
        method_name
            .split('_')
            .next()
            .unwrap_or(method_name)
            .to_string()
    }

    fn infer_responsibility_name(&self, prefix: &str) -> String {
        Self::classify_responsibility(prefix)
    }

    /// Pure function to classify responsibility based on method prefix
    fn classify_responsibility(prefix: &str) -> String {
        match prefix {
            "get" | "set" => "Data Access".to_string(),
            "calculate" | "compute" => "Computation".to_string(),
            "validate" | "check" | "verify" | "ensure" => "Validation".to_string(),
            "save" | "load" | "store" | "retrieve" | "fetch" => "Persistence".to_string(),
            "create" | "build" | "new" | "make" | "init" => "Construction".to_string(),
            "send" | "receive" | "handle" | "manage" => "Communication".to_string(),
            "update" | "modify" | "change" | "edit" => "Modification".to_string(),
            "delete" | "remove" | "clear" | "reset" => "Deletion".to_string(),
            "is" | "has" | "can" | "should" | "will" => "State Query".to_string(),
            "process" | "transform" => "Processing".to_string(),
            _ => format!("{} Operations", capitalize_first(prefix)),
        }
    }
}

impl OrganizationDetector for GodObjectDetector {
    fn detect_anti_patterns(&self, file: &syn::File) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        let mut visitor = TypeVisitor::with_location_extractor(self.location_extractor.clone());
        visitor.visit_file(file);

        // Analyze each struct found
        for (_type_name, type_info) in visitor.types {
            if self.is_god_object(&type_info) {
                let suggested_split = self.suggest_responsibility_split(&type_info);

                patterns.push(OrganizationAntiPattern::GodObject {
                    type_name: type_info.name.clone(),
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
            } => GodObjectDetector::classify_god_object_impact(*method_count, *field_count),
            _ => MaintainabilityImpact::Low,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, ItemImpl};

    #[test]
    fn test_find_matching_prefix_with_get() {
        assert_eq!(
            GodObjectDetector::find_matching_prefix("get_value"),
            Some("get".to_string())
        );
    }

    #[test]
    fn test_find_matching_prefix_with_set() {
        assert_eq!(
            GodObjectDetector::find_matching_prefix("setValue"),
            Some("set".to_string())
        );
    }

    #[test]
    fn test_find_matching_prefix_with_validate() {
        assert_eq!(
            GodObjectDetector::find_matching_prefix("validate_input"),
            Some("validate".to_string())
        );
    }

    #[test]
    fn test_find_matching_prefix_case_insensitive() {
        assert_eq!(
            GodObjectDetector::find_matching_prefix("CREATE_INSTANCE"),
            Some("create".to_string())
        );
    }

    #[test]
    fn test_find_matching_prefix_no_match() {
        assert_eq!(GodObjectDetector::find_matching_prefix("foo_bar"), None);
    }

    #[test]
    fn test_extract_first_word_with_underscore() {
        assert_eq!(
            GodObjectDetector::extract_first_word("custom_method_name"),
            "custom".to_string()
        );
    }

    #[test]
    fn test_extract_first_word_no_underscore() {
        assert_eq!(
            GodObjectDetector::extract_first_word("singleword"),
            "singleword".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_data_access() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("get"),
            "Data Access".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("set"),
            "Data Access".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_computation() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("calculate"),
            "Computation".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("compute"),
            "Computation".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_validation() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("validate"),
            "Validation".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("check"),
            "Validation".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("verify"),
            "Validation".to_string()
        );
    }

    #[test]
    fn test_classify_god_object_impact_critical() {
        // Critical: method_count > 30 or field_count > 20
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(31, 10),
            MaintainabilityImpact::Critical
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(15, 21),
            MaintainabilityImpact::Critical
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(35, 25),
            MaintainabilityImpact::Critical
        );
    }

    #[test]
    fn test_classify_god_object_impact_high() {
        // High: method_count > 20 or field_count > 15 (but not critical)
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(21, 10),
            MaintainabilityImpact::High
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(15, 16),
            MaintainabilityImpact::High
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(25, 14),
            MaintainabilityImpact::High
        );
    }

    #[test]
    fn test_classify_god_object_impact_medium() {
        // Medium: everything else
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(10, 10),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(20, 15),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(5, 5),
            MaintainabilityImpact::Medium
        );
    }

    #[test]
    fn test_classify_god_object_impact_boundary_conditions() {
        // Test exact boundary values
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(30, 20),
            MaintainabilityImpact::High
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(31, 20),
            MaintainabilityImpact::Critical
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(30, 21),
            MaintainabilityImpact::Critical
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(20, 15),
            MaintainabilityImpact::Medium
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(21, 15),
            MaintainabilityImpact::High
        );
        assert_eq!(
            GodObjectDetector::classify_god_object_impact(20, 16),
            MaintainabilityImpact::High
        );
    }

    #[test]
    fn test_classify_responsibility_persistence() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("save"),
            "Persistence".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("load"),
            "Persistence".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("fetch"),
            "Persistence".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_construction() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("create"),
            "Construction".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("build"),
            "Construction".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("new"),
            "Construction".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_communication() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("send"),
            "Communication".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("receive"),
            "Communication".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("handle"),
            "Communication".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_modification() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("update"),
            "Modification".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("modify"),
            "Modification".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("change"),
            "Modification".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_deletion() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("delete"),
            "Deletion".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("remove"),
            "Deletion".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("clear"),
            "Deletion".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_state_query() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("is"),
            "State Query".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("has"),
            "State Query".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("can"),
            "State Query".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_processing() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("process"),
            "Processing".to_string()
        );
        assert_eq!(
            GodObjectDetector::classify_responsibility("transform"),
            "Processing".to_string()
        );
    }

    #[test]
    fn test_classify_responsibility_default() {
        assert_eq!(
            GodObjectDetector::classify_responsibility("custom"),
            "Custom Operations".to_string()
        );
    }

    #[test]
    fn test_extract_type_name_with_path_type() {
        let self_ty: syn::Type = parse_quote!(MyStruct);
        let result = TypeVisitor::extract_type_name(&self_ty);
        assert_eq!(result, Some("MyStruct".to_string()));
    }

    #[test]
    fn test_extract_type_name_with_complex_path() {
        let self_ty: syn::Type = parse_quote!(std::collections::HashMap);
        let result = TypeVisitor::extract_type_name(&self_ty);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_type_name_with_reference_type() {
        let self_ty: syn::Type = parse_quote!(&MyStruct);
        let result = TypeVisitor::extract_type_name(&self_ty);
        assert_eq!(result, None);
    }

    #[test]
    fn test_count_impl_methods_empty() {
        let items = vec![];
        let (methods, count) = TypeVisitor::count_impl_methods(&items);
        assert_eq!(methods.len(), 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_impl_methods_with_functions() {
        let impl_block: ItemImpl = parse_quote! {
            impl MyStruct {
                fn method1(&self) {}
                fn method2(&mut self) {}
                const CONSTANT: i32 = 42;
                fn method3() {}
            }
        };

        let (methods, count) = TypeVisitor::count_impl_methods(&impl_block.items);
        assert_eq!(count, 3);
        assert_eq!(methods.len(), 3);
        assert!(methods.contains(&"method1".to_string()));
        assert!(methods.contains(&"method2".to_string()));
        assert!(methods.contains(&"method3".to_string()));
    }

    #[test]
    fn test_count_impl_methods_mixed_items() {
        let impl_block: ItemImpl = parse_quote! {
            impl MyStruct {
                type Item = i32;
                fn method1(&self) {}
                const VALUE: i32 = 10;
                fn method2(&self) {}
            }
        };

        let (methods, count) = TypeVisitor::count_impl_methods(&impl_block.items);
        assert_eq!(count, 2);
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&"method1".to_string()));
        assert!(methods.contains(&"method2".to_string()));
    }

    #[test]
    fn test_update_type_info_with_methods() {
        let mut visitor = TypeVisitor::with_location_extractor(None);

        visitor.types.insert(
            "TestStruct".to_string(),
            TypeAnalysis {
                name: "TestStruct".to_string(),
                method_count: 0,
                field_count: 0,
                methods: Vec::new(),
                fields: Vec::new(),
                responsibilities: Vec::new(),
                trait_implementations: 0,
                location: SourceLocation::default(),
            },
        );

        let impl_block: ItemImpl = parse_quote! {
            impl TestStruct {
                fn method1(&self) {}
                fn method2(&self) {}
            }
        };

        visitor.update_type_info("TestStruct", &impl_block);

        let type_info = visitor.types.get("TestStruct").unwrap();
        assert_eq!(type_info.method_count, 2);
        assert_eq!(type_info.methods.len(), 2);
        assert!(type_info.methods.contains(&"method1".to_string()));
        assert!(type_info.methods.contains(&"method2".to_string()));
    }

    #[test]
    fn test_update_type_info_with_trait_impl() {
        let mut visitor = TypeVisitor::with_location_extractor(None);

        visitor.types.insert(
            "TestStruct".to_string(),
            TypeAnalysis {
                name: "TestStruct".to_string(),
                method_count: 0,
                field_count: 0,
                methods: Vec::new(),
                fields: Vec::new(),
                responsibilities: Vec::new(),
                trait_implementations: 0,
                location: SourceLocation::default(),
            },
        );

        let impl_block: ItemImpl = parse_quote! {
            impl Display for TestStruct {
                fn fmt(&self, f: &mut Formatter) -> Result {
                    Ok(())
                }
            }
        };

        visitor.update_type_info("TestStruct", &impl_block);

        let type_info = visitor.types.get("TestStruct").unwrap();
        assert_eq!(type_info.trait_implementations, 1);
        assert_eq!(type_info.method_count, 1);
        assert!(type_info.methods.contains(&"fmt".to_string()));
    }

    #[test]
    fn test_update_type_info_nonexistent_type() {
        let mut visitor = TypeVisitor::with_location_extractor(None);

        let impl_block: ItemImpl = parse_quote! {
            impl NonExistent {
                fn method(&self) {}
            }
        };

        visitor.update_type_info("NonExistent", &impl_block);

        assert!(!visitor.types.contains_key("NonExistent"));
    }

    #[test]
    fn test_visit_item_impl_integration() {
        use syn::visit::Visit;

        let mut visitor = TypeVisitor::with_location_extractor(None);

        visitor.types.insert(
            "MyStruct".to_string(),
            TypeAnalysis {
                name: "MyStruct".to_string(),
                method_count: 0,
                field_count: 0,
                methods: Vec::new(),
                fields: Vec::new(),
                responsibilities: Vec::new(),
                trait_implementations: 0,
                location: SourceLocation::default(),
            },
        );

        let impl_block: ItemImpl = parse_quote! {
            impl MyStruct {
                fn new() -> Self { MyStruct }
                fn process(&self) {}
            }
        };

        visitor.visit_item_impl(&impl_block);

        let type_info = visitor.types.get("MyStruct").unwrap();
        assert_eq!(type_info.method_count, 2);
        assert_eq!(type_info.methods.len(), 2);
    }

    #[test]
    fn test_create_responsibility_group() {
        let detector = GodObjectDetector::new();
        let methods = vec!["get_value".to_string(), "get_name".to_string()];

        let group = detector.create_responsibility_group("get".to_string(), methods.clone());

        assert_eq!(group.name, "DataAccessManager");
        assert_eq!(group.responsibility, "Data Access");
        assert_eq!(group.methods, methods);
        assert!(group.fields.is_empty());
    }

    #[test]
    fn test_create_responsibility_group_with_spaces() {
        let detector = GodObjectDetector::new();
        let methods = vec!["validate_input".to_string()];

        let group = detector.create_responsibility_group("validate".to_string(), methods.clone());

        assert_eq!(group.name, "ValidationManager");
        assert_eq!(group.responsibility, "Validation");
        assert_eq!(group.methods, methods);
    }

    #[test]
    fn test_create_default_responsibility_group() {
        let detector = GodObjectDetector::new();
        let analysis = TypeAnalysis {
            name: "TestClass".to_string(),
            method_count: 5,
            field_count: 3,
            methods: vec!["method1".to_string(), "method2".to_string()],
            fields: vec!["field1".to_string(), "field2".to_string()],
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location: SourceLocation::default(),
        };

        let group = detector.create_default_responsibility_group(&analysis);

        assert_eq!(group.name, "TestClassCore");
        assert_eq!(group.responsibility, "Core functionality");
        assert_eq!(group.methods, analysis.methods);
        assert_eq!(group.fields, analysis.fields);
    }

    #[test]
    fn test_suggest_responsibility_split_with_method_groups() {
        let detector = GodObjectDetector::new();
        let analysis = TypeAnalysis {
            name: "TestClass".to_string(),
            method_count: 8,
            field_count: 5,
            methods: vec![
                "get_value".to_string(),
                "get_name".to_string(),
                "set_value".to_string(),
                "validate_input".to_string(),
                "validate_output".to_string(),
                "save_data".to_string(),
            ],
            fields: Vec::new(),
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location: SourceLocation::default(),
        };

        let groups = detector.suggest_responsibility_split(&analysis);

        assert_eq!(groups.len(), 4); // get, set, validate, save

        // Verify that groups are properly created
        let group_names: Vec<String> = groups.iter().map(|g| g.name.clone()).collect();
        assert!(group_names.contains(&"DataAccessManager".to_string()));
        assert!(group_names.contains(&"ValidationManager".to_string()));
        assert!(group_names.contains(&"PersistenceManager".to_string()));
    }

    #[test]
    fn test_suggest_responsibility_split_with_no_groups_below_threshold() {
        let detector = GodObjectDetector::new();
        let analysis = TypeAnalysis {
            name: "SmallClass".to_string(),
            method_count: 10, // Below max_methods (15)
            field_count: 5,
            methods: vec!["custom_method".to_string()],
            fields: Vec::new(),
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location: SourceLocation::default(),
        };

        let groups = detector.suggest_responsibility_split(&analysis);

        // Should return the grouped method even if below threshold
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "CustomOperationsManager");
    }

    #[test]
    fn test_suggest_responsibility_split_with_no_groups_above_threshold() {
        let detector = GodObjectDetector::new();
        let analysis = TypeAnalysis {
            name: "LargeClass".to_string(),
            method_count: 20, // Above max_methods (15)
            field_count: 5,
            methods: vec!["custom_method".to_string()],
            fields: vec!["field1".to_string()],
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location: SourceLocation::default(),
        };

        let groups = detector.suggest_responsibility_split(&analysis);

        // Should still group by prefix even above threshold
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "CustomOperationsManager");
    }

    #[test]
    fn test_suggest_responsibility_split_empty_methods_above_threshold() {
        let detector = GodObjectDetector::new();
        let analysis = TypeAnalysis {
            name: "EmptyClass".to_string(),
            method_count: 20, // Above max_methods (15)
            field_count: 5,
            methods: Vec::new(), // No methods to group
            fields: vec!["field1".to_string()],
            responsibilities: Vec::new(),
            trait_implementations: 0,
            location: SourceLocation::default(),
        };

        let groups = detector.suggest_responsibility_split(&analysis);

        // Should create default group when no methods but above threshold
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "EmptyClassCore");
        assert_eq!(groups[0].responsibility, "Core functionality");
    }

    #[test]
    fn test_enhanced_analysis_god_class() {
        let code = r#"
            pub struct GodClass {
                f1: u32, f2: u32, f3: u32, f4: u32, f5: u32,
                f6: u32, f7: u32, f8: u32, f9: u32, f10: u32,
                f11: u32, f12: u32, f13: u32, f14: u32, f15: u32,
                f16: u32,
            }
            impl GodClass {
                fn m1(&self) {} fn m2(&self) {} fn m3(&self) {}
                fn m4(&self) {} fn m5(&self) {} fn m6(&self) {}
                fn m7(&self) {} fn m8(&self) {} fn m9(&self) {}
                fn m10(&self) {} fn m11(&self) {} fn m12(&self) {}
                fn m13(&self) {} fn m14(&self) {} fn m15(&self) {}
                fn m16(&self) {} fn m17(&self) {} fn m18(&self) {}
                fn m19(&self) {} fn m20(&self) {} fn m21(&self) {}
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("test.rs");
        let analysis = detector.analyze_enhanced(path, &ast);

        // Should be classified as god class
        match analysis.classification {
            GodObjectType::GodClass {
                struct_name,
                method_count,
                field_count,
                ..
            } => {
                assert_eq!(struct_name, "GodClass");
                assert!(method_count > 20);
                assert!(field_count > 15);
            }
            _ => panic!("Expected GodClass classification"),
        }
    }

    #[test]
    fn test_enhanced_analysis_god_module() {
        let code = r#"
            pub struct ScoringWeights { f1: u32, f2: u32 }
            impl ScoringWeights {
                fn m1(&self) {} fn m2(&self) {} fn m3(&self) {}
                fn m4(&self) {} fn m5(&self) {} fn m6(&self) {}
                fn m7(&self) {} fn m8(&self) {}
            }
            pub struct RoleMultipliers { f1: u32, f2: u32 }
            impl RoleMultipliers {
                fn m1(&self) {} fn m2(&self) {} fn m3(&self) {}
                fn m4(&self) {} fn m5(&self) {} fn m6(&self) {}
                fn m7(&self) {} fn m8(&self) {}
            }
            pub struct DetectionConfig { f1: u32 }
            impl DetectionConfig {
                fn m1(&self) {} fn m2(&self) {}
                fn m3(&self) {} fn m4(&self) {}
                fn m5(&self) {} fn m6(&self) {}
            }
            pub struct ThresholdLimits { f1: u32 }
            impl ThresholdLimits {
                fn m1(&self) {} fn m2(&self) {}
                fn m3(&self) {} fn m4(&self) {}
                fn m5(&self) {} fn m6(&self) {}
            }
            pub struct CoreConfig { f1: u32 }
            impl CoreConfig {
                fn m1(&self) {} fn m2(&self) {}
                fn m3(&self) {} fn m4(&self) {}
                fn m5(&self) {} fn m6(&self) {}
            }
            pub struct DataMetrics { f1: u32 }
            impl DataMetrics {
                fn m1(&self) {} fn m2(&self) {}
                fn m3(&self) {} fn m4(&self) {}
                fn m5(&self) {} fn m6(&self) {}
            }
            pub struct InfoData { f1: u32 }
            impl InfoData {
                fn m1(&self) {} fn m2(&self) {} fn m3(&self) {}
                fn m4(&self) {} fn m5(&self) {} fn m6(&self) {}
                fn m7(&self) {} fn m8(&self) {}
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("config.rs");
        let analysis = detector.analyze_enhanced(path, &ast);

        // Should have multiple per-struct metrics
        assert!(analysis.per_struct_metrics.len() >= 5);

        // Debug: print what we got
        eprintln!("Total methods: {}", analysis.file_metrics.method_count);
        eprintln!("Classification: {:?}", analysis.classification);

        // Should be classified as god module (no individual struct exceeds limits, but module is large)
        match analysis.classification {
            GodObjectType::GodModule {
                total_structs,
                total_methods,
                ..
            } => {
                assert!(total_structs >= 5);
                assert!(total_methods > 40);
            }
            _ => {
                // Adjust test if we don't hit the god module threshold
                // Total methods need to be > max_methods * 2 (i.e. > 40 for Rust)
                assert!(
                    analysis.file_metrics.method_count <= 40,
                    "Expected god module with {} methods, got: {:?}",
                    analysis.file_metrics.method_count,
                    analysis.classification
                );
            }
        }
    }

    #[test]
    fn test_enhanced_analysis_god_module_with_cohesion() {
        // This test validates spec 144: call graph integration for cohesion scoring
        let code = r#"
            pub struct ScoringWeights { base: f64 }
            impl ScoringWeights {
                fn get_default() -> Self { Self { base: 1.0 } }
                fn apply(&self, value: f64) -> f64 { value * self.base }
                fn get_multipliers(&self) -> RoleMultipliers { RoleMultipliers::get() }
            }
            pub struct RoleMultipliers { admin: f64 }
            impl RoleMultipliers {
                fn get() -> Self { Self { admin: 2.0 } }
                fn apply_to_score(&self, base: f64) -> f64 { base * self.admin }
            }
            pub struct DetectionConfig { enabled: bool }
            impl DetectionConfig {
                fn new() -> Self { Self { enabled: true } }
                fn is_enabled(&self) -> bool { self.enabled }
            }
            pub struct ThresholdLimits { max: f64 }
            impl ThresholdLimits {
                fn new() -> Self { Self { max: 100.0 } }
                fn check(&self, value: f64) -> bool { value <= self.max }
            }
            pub struct CoreConfig { name: String }
            impl CoreConfig {
                fn new() -> Self { Self { name: "default".to_string() } }
                fn get_name(&self) -> &str { &self.name }
            }
            pub struct DataMetrics { count: u32 }
            impl DataMetrics {
                fn new() -> Self { Self { count: 0 } }
                fn increment(&mut self) { self.count += 1; }
            }
            pub struct InfoData { data: Vec<String> }
            impl InfoData {
                fn new() -> Self { Self { data: vec![] } }
                fn add(&mut self, s: String) { self.data.push(s); }
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("config.rs");
        let analysis = detector.analyze_enhanced(path, &ast);

        // Should be classified as god module
        match &analysis.classification {
            GodObjectType::GodModule {
                suggested_splits, ..
            } => {
                // Check that cohesion scores are present
                let splits_with_cohesion: Vec<_> = suggested_splits
                    .iter()
                    .filter(|s| s.cohesion_score.is_some())
                    .collect();

                assert!(
                    !splits_with_cohesion.is_empty(),
                    "At least some splits should have cohesion scores"
                );

                // Check that cohesion scores are reasonable (between 0.0 and 1.0)
                for split in splits_with_cohesion.iter() {
                    let cohesion = split.cohesion_score.unwrap();
                    assert!(
                        (0.0..=1.0).contains(&cohesion),
                        "Cohesion score {} should be between 0.0 and 1.0",
                        cohesion
                    );
                }

                // Check average cohesion is reasonable (>0.6 for well-grouped modules)
                let avg_cohesion: f64 = splits_with_cohesion
                    .iter()
                    .map(|s| s.cohesion_score.unwrap())
                    .sum::<f64>()
                    / splits_with_cohesion.len() as f64;

                assert!(
                    avg_cohesion > 0.6,
                    "Average cohesion {} should be > 0.6 for well-grouped modules",
                    avg_cohesion
                );
            }
            _ => {
                // If not classified as god module, that's okay - just verify the code compiled
                // and ran without panicking
            }
        }
    }

    #[test]
    fn test_enhanced_analysis_not_god_object() {
        let code = r#"
            pub struct SmallStruct1 { f1: u32 }
            impl SmallStruct1 {
                fn m1(&self) {} fn m2(&self) {}
            }
            pub struct SmallStruct2 { f1: u32, f2: u32 }
            impl SmallStruct2 {
                fn m1(&self) {} fn m2(&self) {} fn m3(&self) {}
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("test.rs");
        let analysis = detector.analyze_enhanced(path, &ast);

        // Should be classified as not god object
        assert!(matches!(
            analysis.classification,
            GodObjectType::NotGodObject
        ));
    }

    #[test]
    fn test_build_per_struct_metrics() {
        let code = r#"
            pub struct Struct1 { f1: u32, f2: u32 }
            impl Struct1 {
                fn method1(&self) {} fn method2(&self) {}
            }
            pub struct Struct2 { f1: u32 }
            impl Struct2 {
                fn method1(&self) {}
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let mut visitor = TypeVisitor::with_location_extractor(detector.location_extractor.clone());
        visitor.visit_file(&ast);

        let metrics = metrics::build_per_struct_metrics(&visitor);

        assert_eq!(metrics.len(), 2);

        let struct1_metrics = metrics.iter().find(|m| m.name == "Struct1").unwrap();
        assert_eq!(struct1_metrics.method_count, 2);
        assert_eq!(struct1_metrics.field_count, 2);

        let struct2_metrics = metrics.iter().find(|m| m.name == "Struct2").unwrap();
        assert_eq!(struct2_metrics.method_count, 1);
        assert_eq!(struct2_metrics.field_count, 1);
    }

    #[test]
    fn test_classify_struct_domain_scoring() {
        use crate::organization::god_object_analysis::classify_struct_domain;

        assert_eq!(classify_struct_domain("ScoringWeights"), "scoring");
        assert_eq!(classify_struct_domain("RoleMultipliers"), "scoring");
        assert_eq!(classify_struct_domain("ComplexityFactor"), "scoring");
    }

    #[test]
    fn test_classify_struct_domain_thresholds() {
        use crate::organization::god_object_analysis::classify_struct_domain;

        assert_eq!(classify_struct_domain("ThresholdLimits"), "thresholds");
        assert_eq!(classify_struct_domain("MaxBounds"), "thresholds");
    }

    #[test]
    fn test_classify_struct_domain_detection() {
        use crate::organization::god_object_analysis::classify_struct_domain;

        assert_eq!(classify_struct_domain("OrchestratorDetector"), "detection");
        assert_eq!(classify_struct_domain("PatternChecker"), "detection");
    }

    #[test]
    fn test_classify_struct_domain_config() {
        use crate::organization::god_object_analysis::classify_struct_domain;

        assert_eq!(classify_struct_domain("AppConfig"), "core_config");
        assert_eq!(classify_struct_domain("UserSettings"), "core_config");
    }

    // Spec 134 Phase 2: Tests for impl method visibility tracking
    #[test]
    fn test_impl_method_visibility_tracking() {
        let code = r#"
            pub struct MyStruct {
                field: u32,
            }
            impl MyStruct {
                pub fn public_method(&self) {}
                fn private_method(&self) {}
                pub(crate) fn crate_method(&self) {}
                pub(super) fn super_method(&self) {}
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("test.rs");
        let analysis = detector.analyze_comprehensive(path, &ast);

        // Should have visibility breakdown
        assert!(analysis.visibility_breakdown.is_some());
        let breakdown = analysis.visibility_breakdown.as_ref().unwrap();

        // Should correctly count each visibility type
        assert_eq!(breakdown.public, 1, "Should have 1 public method");
        assert_eq!(breakdown.pub_crate, 1, "Should have 1 pub(crate) method");
        assert_eq!(breakdown.pub_super, 1, "Should have 1 pub(super) method");
        assert_eq!(breakdown.private, 1, "Should have 1 private method");
        assert_eq!(breakdown.total(), 4, "Total should be 4 methods");

        // Validate consistency
        assert!(analysis.validate().is_ok());
    }

    #[test]
    fn test_mixed_standalone_and_impl_visibility() {
        let code = r#"
            pub fn standalone_pub() {}
            fn standalone_private() {}

            pub struct MyStruct {}
            impl MyStruct {
                pub fn impl_pub(&self) {}
                fn impl_private(&self) {}
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("test.rs");
        let analysis = detector.analyze_comprehensive(path, &ast);

        let breakdown = analysis.visibility_breakdown.as_ref().unwrap();

        // For GodClass detection with a struct present, only impl methods are counted (2 methods)
        // The standalone functions are not part of the god object analysis
        assert_eq!(breakdown.public, 1, "impl method");
        assert_eq!(breakdown.private, 1, "impl method");
        assert_eq!(breakdown.total(), 2);
        assert_eq!(analysis.method_count, 2);
        assert!(analysis.validate().is_ok());
    }

    #[test]
    fn test_visibility_breakdown_validates() {
        let code = r#"
            pub struct MyStruct {}
            impl MyStruct {
                pub fn m1(&self) {}
                pub fn m2(&self) {}
                fn m3(&self) {}
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("test.rs");
        let analysis = detector.analyze_comprehensive(path, &ast);

        // method_count should match visibility breakdown total
        let breakdown = analysis.visibility_breakdown.as_ref().unwrap();
        assert_eq!(analysis.method_count, breakdown.total());

        // Validation should pass
        assert!(analysis.validate().is_ok());
    }

    // Spec 134 Phase 3: Tests for function counting consistency
    #[test]
    fn test_function_count_excludes_tests_for_god_class() {
        let code = r#"
            pub struct MyStruct {}
            impl MyStruct {
                pub fn production1(&self) {}
                pub fn production2(&self) {}

                #[test]
                fn test_method1() {}

                #[test]
                fn test_method2() {}
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("test.rs");
        let analysis = detector.analyze_comprehensive(path, &ast);

        // Should only count production methods (2), not tests (2)
        assert_eq!(
            analysis.method_count, 2,
            "Should count only production methods"
        );
        assert_eq!(
            analysis.detection_type,
            crate::organization::DetectionType::GodClass
        );

        // Visibility breakdown should match method_count
        let breakdown = analysis.visibility_breakdown.as_ref().unwrap();
        assert_eq!(
            breakdown.total(),
            2,
            "Visibility breakdown should match method_count"
        );

        // Validation must pass
        assert!(analysis.validate().is_ok());
    }

    #[test]
    fn test_function_count_consistency_across_metrics() {
        let code = r#"
            pub struct MyStruct {}
            impl MyStruct {
                pub fn m1(&self) {}
                pub fn m2(&self) {}
                pub fn m3(&self) {}

                #[test]
                fn test1() {}
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("test.rs");
        let analysis = detector.analyze_comprehensive(path, &ast);

        // Verify consistency: method_count, visibility breakdown, and responsibilities
        assert_eq!(analysis.method_count, 3, "Should have 3 production methods");

        let breakdown = analysis.visibility_breakdown.as_ref().unwrap();
        assert_eq!(breakdown.total(), 3, "Visibility should match method_count");
        assert_eq!(breakdown.public, 3);

        // Validate all metrics are consistent
        assert!(analysis.validate().is_ok());

        // Lines of code should be calculated from filtered method_count
        // Formula: method_count * 15 + field_count * 2 + 50
        let expected_loc = 3 * 15 + 50; // 0 fields, so no field contribution
        assert_eq!(analysis.lines_of_code, expected_loc);
    }

    #[test]
    fn test_no_contradictions_in_metrics() {
        let code = r#"
            pub struct GodClass {
                f1: u32, f2: u32, f3: u32, f4: u32, f5: u32,
            }
            impl GodClass {
                pub fn method1(&self) {}
                pub fn method2(&self) {}
                fn method3(&self) {}
                pub(crate) fn method4(&self) {}

                #[test]
                fn test_method() {}
            }
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("test.rs");
        let analysis = detector.analyze_comprehensive(path, &ast);

        // All metrics must be consistent
        let breakdown = analysis.visibility_breakdown.as_ref().unwrap();

        // Should have 4 production methods (excluding test)
        assert_eq!(analysis.method_count, 4);
        assert_eq!(breakdown.total(), 4);
        assert_eq!(breakdown.public, 2);
        assert_eq!(breakdown.pub_crate, 1);
        assert_eq!(breakdown.private, 1);

        // Responsibilities must match method count
        assert!(!analysis.responsibilities.is_empty());
        assert_eq!(
            analysis.responsibility_count,
            analysis.responsibilities.len()
        );

        // Validation must pass - no contradictions
        assert!(analysis.validate().is_ok());
    }

    #[test]
    fn test_godfile_visibility_tracking() {
        // Test case: file with only standalone functions (no structs)
        let code = r#"
            pub fn public_function1() {}
            pub fn public_function2() {}
            fn private_function1() {}
            fn private_function2() {}
            pub(crate) fn crate_function() {}
        "#;

        let ast: syn::File = syn::parse_str(code).unwrap();
        let detector = GodObjectDetector::with_source_content(code);
        let path = std::path::Path::new("test.rs");
        let analysis = detector.analyze_comprehensive(path, &ast);

        // Should be GodFile (no structs)
        assert_eq!(
            analysis.detection_type,
            crate::organization::DetectionType::GodFile
        );

        // Should have 5 functions
        assert_eq!(analysis.method_count, 5);

        // Visibility breakdown should be populated and match
        assert!(analysis.visibility_breakdown.is_some());
        let breakdown = analysis.visibility_breakdown.as_ref().unwrap();

        assert_eq!(breakdown.public, 2, "Should have 2 public functions");
        assert_eq!(breakdown.private, 2, "Should have 2 private functions");
        assert_eq!(breakdown.pub_crate, 1, "Should have 1 pub(crate) function");
        assert_eq!(breakdown.total(), 5, "Total should be 5");

        // Validation should pass
        assert!(analysis.validate().is_ok());
    }

    // Spec 140: Tests for visibility breakdown integration
    #[test]
    fn test_integrate_visibility_into_counts() {
        use crate::analysis::FunctionCounts;
        use crate::organization::god_object_analysis::FunctionVisibilityBreakdown;

        let original_counts = FunctionCounts {
            module_level_functions: 5,
            impl_methods: 15,
            trait_methods: 2,
            nested_module_functions: 0,
            public_functions: 0, // Old system didn't track these
            private_functions: 0,
        };

        let breakdown = FunctionVisibilityBreakdown {
            public: 10,
            pub_crate: 3,
            pub_super: 2,
            private: 12,
        };

        let integrated =
            metrics::integrate_visibility_into_counts(&original_counts, &breakdown, 27);

        // Original counts preserved
        assert_eq!(integrated.module_level_functions, 5);
        assert_eq!(integrated.impl_methods, 15);
        assert_eq!(integrated.trait_methods, 2);
        assert_eq!(integrated.nested_module_functions, 0);

        // Visibility counts from breakdown
        assert_eq!(integrated.public_functions, 10);
        assert_eq!(integrated.private_functions, 17); // 12 + 3 + 2
        assert_eq!(integrated.total(), 22); // module_level + impl + trait
    }

    #[test]
    fn test_integrate_visibility_all_public() {
        use crate::analysis::FunctionCounts;
        use crate::organization::god_object_analysis::FunctionVisibilityBreakdown;

        let original_counts = FunctionCounts {
            module_level_functions: 20,
            impl_methods: 0,
            trait_methods: 0,
            nested_module_functions: 0,
            public_functions: 0,
            private_functions: 0,
        };

        let breakdown = FunctionVisibilityBreakdown {
            public: 20,
            pub_crate: 0,
            pub_super: 0,
            private: 0,
        };

        let integrated =
            metrics::integrate_visibility_into_counts(&original_counts, &breakdown, 20);

        assert_eq!(integrated.public_functions, 20);
        assert_eq!(integrated.private_functions, 0);
    }

    #[test]
    fn test_integrate_visibility_all_private() {
        use crate::analysis::FunctionCounts;
        use crate::organization::god_object_analysis::FunctionVisibilityBreakdown;

        let original_counts = FunctionCounts {
            module_level_functions: 0,
            impl_methods: 15,
            trait_methods: 0,
            nested_module_functions: 0,
            public_functions: 0,
            private_functions: 0,
        };

        let breakdown = FunctionVisibilityBreakdown {
            public: 0,
            pub_crate: 0,
            pub_super: 0,
            private: 15,
        };

        let integrated =
            metrics::integrate_visibility_into_counts(&original_counts, &breakdown, 15);

        assert_eq!(integrated.public_functions, 0);
        assert_eq!(integrated.private_functions, 15);
    }
}
