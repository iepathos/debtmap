//! God object adapter for analyzing extracted data for god object patterns.
//!
//! This module provides pure analysis functions that detect god object patterns
//! from `ExtractedFileData` without requiring additional file parsing.
//!
//! # Design (Spec 197 - Per-Struct Analysis)
//!
//! All functions in this module are pure (no I/O, no parsing). They perform O(n)
//! analysis where n is the number of functions and structs being analyzed.
//!
//! ## Key Design Decisions
//!
//! - **Per-struct analysis**: Each struct is analyzed independently with its own
//!   impl blocks, rather than aggregating metrics across the entire file.
//! - **Behavioral categorization**: Uses `classifier::group_methods_by_responsibility`
//!   to classify methods by behavior (Parsing, Validation, etc.) instead of using
//!   type/trait names as responsibilities.
//! - **Composable pipeline**: Pure helper functions that can be unit tested independently.

use crate::extraction::types::{ExtractedFileData, ExtractedImplData, ExtractedStructData};
use crate::organization::god_object::classifier::{
    calculate_weighted_count_from_names, group_methods_by_responsibility, is_cohesive_struct,
};
use crate::organization::god_object::scoring::calculate_god_object_score_weighted;
use crate::organization::god_object::{
    classify_all_methods, DetectionType, FunctionVisibilityBreakdown, GodObjectAnalysis,
    GodObjectThresholds, KnownTraitRegistry, SplitAnalysisMethod, TraitImplInfo,
    TraitMethodSummary,
};
use crate::priority::score_types::Score0To100;
use std::collections::HashMap;
use std::path::Path;

/// Internal struct for per-struct metric calculation.
#[derive(Debug)]
struct StructMetrics {
    field_count: usize,
    method_count: usize,
    /// Spec 209: Weighted method count accounting for accessor/boilerplate
    _weighted_method_count: f64,
    method_names: Vec<String>,
    /// Average complexity of methods (from extracted function data if available)
    avg_complexity: f64,
    /// Total complexity sum for all methods
    complexity_sum: u32,
    /// Spec 217: Trait implementations for this struct
    trait_impls: Vec<TraitImplInfo>,
    /// Spec 217: Trait-adjusted weighted count
    trait_weighted_count: f64,
}

/// Pure function: build mapping from type names to their impl blocks.
///
/// Constructs a HashMap allowing O(1) lookup of impl blocks by type name.
/// This enables efficient per-struct analysis without nested loops.
fn build_impl_map(impls: &[ExtractedImplData]) -> HashMap<String, Vec<&ExtractedImplData>> {
    let mut map: HashMap<String, Vec<&ExtractedImplData>> = HashMap::new();
    for impl_block in impls {
        map.entry(impl_block.type_name.clone())
            .or_default()
            .push(impl_block);
    }
    map
}

/// Pure function: extract trait implementation info from impl blocks.
///
/// Spec 217: Detects trait implementations for a struct to distinguish
/// trait-mandated methods from self-chosen methods.
fn extract_trait_impls(
    impl_blocks: &[&ExtractedImplData],
    registry: &KnownTraitRegistry,
) -> Vec<TraitImplInfo> {
    impl_blocks
        .iter()
        .filter_map(|impl_block| {
            impl_block.trait_name.as_ref().map(|trait_name| {
                let method_names: Vec<String> =
                    impl_block.methods.iter().map(|m| m.name.clone()).collect();
                let category = registry.categorize_trait(trait_name);
                TraitImplInfo::new(trait_name.clone(), method_names, category)
            })
        })
        .collect()
}

/// Pure function: calculate metrics for a single struct.
///
/// Aggregates method counts, names, and complexity from all impl blocks for this struct.
/// Looks up complexity from extracted function data using qualified names.
///
/// Spec 209: Also calculates weighted method count based on accessor/boilerplate classification.
/// Spec 217: Calculates trait-weighted count for more accurate god object detection.
fn calculate_struct_metrics(
    struct_data: &ExtractedStructData,
    impl_blocks: &[&ExtractedImplData],
    extracted: &ExtractedFileData,
) -> StructMetrics {
    let registry = KnownTraitRegistry::default();
    let method_count: usize = impl_blocks.iter().map(|i| i.methods.len()).sum();

    let method_names: Vec<String> = impl_blocks
        .iter()
        .flat_map(|i| i.methods.iter().map(|m| m.name.clone()))
        .collect();

    // Spec 209: Calculate weighted method count
    // Accessors and boilerplate contribute less to the god object score
    let weighted_method_count =
        calculate_weighted_count_from_names(method_names.iter().map(String::as_str));

    // Spec 217: Extract trait implementations and calculate trait-weighted count
    let trait_impls = extract_trait_impls(impl_blocks, &registry);
    let classified_methods = classify_all_methods(&method_names, &trait_impls, &registry);
    let trait_summary = TraitMethodSummary::from_classifications(&classified_methods);

    // Use the minimum of accessor-weighted and trait-weighted counts
    // This gives benefit of both Spec 209 and Spec 217
    let trait_weighted_count = trait_summary.weighted_count.min(weighted_method_count);

    // Look up complexity from extracted functions using qualified names
    // Methods in impl blocks should match "TypeName::method_name" pattern
    let complexity_sum: u32 = impl_blocks
        .iter()
        .flat_map(|impl_block| {
            impl_block.methods.iter().filter_map(|method| {
                let qualified = format!("{}::{}", impl_block.type_name, method.name);
                extracted
                    .functions
                    .iter()
                    .find(|f| f.qualified_name == qualified || f.name == method.name)
                    .map(|f| f.cyclomatic)
            })
        })
        .sum();

    let avg_complexity = if method_count > 0 {
        complexity_sum as f64 / method_count as f64
    } else {
        0.0
    };

    StructMetrics {
        field_count: struct_data.fields.len(),
        method_count,
        trait_impls,
        trait_weighted_count,
        _weighted_method_count: weighted_method_count,
        method_names,
        avg_complexity,
        complexity_sum,
    }
}

/// Pure function: determine if struct qualifies as god object based on metrics.
///
/// Returns true if any threshold is exceeded:
/// - Weighted method count > max_methods (Spec 209/217)
/// - Field count > max_fields
/// - Responsibility count > max_traits
///
/// Spec 209: Uses weighted method count instead of raw count, so structs
/// with many accessor/boilerplate methods are less likely to trigger.
/// Spec 217: Uses trait-weighted count if available, so structs with many
/// trait-mandated methods are less likely to trigger.
fn is_god_object_candidate(
    metrics: &StructMetrics,
    responsibilities: &HashMap<String, Vec<String>>,
    thresholds: &GodObjectThresholds,
) -> bool {
    // Spec 209/217: Use trait-weighted count (minimum of accessor and trait weighting)
    metrics.trait_weighted_count > thresholds.max_methods as f64
        || metrics.field_count > thresholds.max_fields
        || responsibilities.len() > thresholds.max_traits
}

/// Pure function: build GodObjectAnalysis from struct metrics and responsibilities.
///
/// Uses the weighted scoring algorithm for consistency with detector.rs (Spec 212).
/// Spec 209: Uses weighted method count for more accurate scoring.
/// Spec 214: Uses production LOC (excluding test code) for scoring.
/// Spec 217: Includes trait method summary for recommendations.
fn build_god_object_analysis(
    struct_data: &ExtractedStructData,
    metrics: &StructMetrics,
    responsibilities: HashMap<String, Vec<String>>,
    production_lines: usize,
    thresholds: &GodObjectThresholds,
) -> GodObjectAnalysis {
    // Use weighted scoring algorithm from scoring.rs for consistency (Spec 212)
    // Spec 209/217: Use trait-weighted count for scoring
    // Spec 214: Use production LOC for scoring to avoid penalizing well-tested code
    let god_object_score = calculate_god_object_score_weighted(
        metrics.trait_weighted_count, // Spec 217: Use trait-weighted count
        metrics.field_count,
        responsibilities.len(),
        production_lines,
        metrics.avg_complexity,
        thresholds,
    );

    // Build responsibility method counts
    let responsibility_method_counts: HashMap<String, usize> = responsibilities
        .iter()
        .map(|(k, v)| (k.clone(), v.len()))
        .collect();

    // Convert responsibilities to Vec<String> (just the keys)
    let responsibility_names: Vec<String> = responsibilities.keys().cloned().collect();

    // Determine confidence based on violation count
    // Spec 214: Use production LOC for confidence determination
    let confidence = crate::organization::god_object::classifier::determine_confidence(
        metrics.method_count,
        metrics.field_count,
        responsibility_names.len(),
        production_lines,
        metrics.complexity_sum,
        thresholds,
    );

    // Spec 217: Build trait method summary
    let registry = KnownTraitRegistry::default();
    let classified_methods =
        classify_all_methods(&metrics.method_names, &metrics.trait_impls, &registry);
    let trait_method_summary = TraitMethodSummary::from_classifications(&classified_methods);
    let has_trait_methods = trait_method_summary.mandated_count > 0;

    GodObjectAnalysis {
        is_god_object: true,
        method_count: metrics.method_count,
        weighted_method_count: Some(metrics.trait_weighted_count), // Spec 217: Show trait-weighted count
        field_count: metrics.field_count,
        responsibility_count: responsibility_names.len(),
        lines_of_code: production_lines, // Spec 214: Use production LOC
        complexity_sum: metrics.complexity_sum,
        god_object_score: Score0To100::new(god_object_score),
        recommended_splits: vec![],
        confidence,
        responsibilities: responsibility_names,
        responsibility_method_counts,
        purity_distribution: None,
        module_structure: None,
        detection_type: DetectionType::GodClass,
        struct_name: Some(struct_data.name.clone()),
        struct_line: Some(struct_data.line),
        struct_location: None,
        visibility_breakdown: None, // Would need per-struct visibility tracking
        domain_count: 0,
        domain_diversity: 0.0,
        struct_ratio: 0.0,
        analysis_method: SplitAnalysisMethod::None,
        cross_domain_severity: None,
        domain_diversity_metrics: None,
        aggregated_entropy: None,
        aggregated_error_swallowing_count: None,
        aggregated_error_swallowing_patterns: None,
        layering_impact: None,
        anti_pattern_report: None,
        complexity_metrics: None, // Spec 211
        trait_method_summary: if has_trait_methods {
            Some(trait_method_summary)
        } else {
            None
        }, // Spec 217
    }
}

/// Analyze extracted file data for god object patterns - per-struct analysis.
///
/// This is a pure function with no file I/O. It implements Spec 197's per-struct
/// analysis approach, returning a separate `GodObjectAnalysis` for each struct
/// that qualifies as a god object.
///
/// # Arguments
///
/// * `_path` - Path to the file being analyzed (unused, kept for API consistency)
/// * `extracted` - Extracted file data
///
/// # Returns
///
/// Vec of `GodObjectAnalysis`, one per qualifying struct. Empty if no god objects found.
///
/// # Design
///
/// 1. Build impl-to-struct mapping (O(n))
/// 2. For each struct, calculate metrics from its impl blocks
/// 3. Use behavioral categorization for responsibilities
/// 4. Filter to only structs exceeding thresholds
///
/// For files with no structs but many standalone functions, falls back to
/// file-level analysis (GodFile/GodModule detection).
pub fn analyze_god_objects(_path: &Path, extracted: &ExtractedFileData) -> Vec<GodObjectAnalysis> {
    analyze_god_objects_with_thresholds(_path, extracted, &GodObjectThresholds::default())
}

/// Analyze with custom thresholds - per-struct analysis.
pub fn analyze_god_objects_with_thresholds(
    _path: &Path,
    extracted: &ExtractedFileData,
    thresholds: &GodObjectThresholds,
) -> Vec<GodObjectAnalysis> {
    let impl_map = build_impl_map(&extracted.impls);

    // Analyze each struct independently
    let struct_results: Vec<GodObjectAnalysis> = extracted
        .structs
        .iter()
        .filter_map(|struct_data| {
            // Get impl blocks for this struct
            let impl_blocks = impl_map
                .get(&struct_data.name)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            // Skip DTOs (structs with no methods)
            if impl_blocks.is_empty() || impl_blocks.iter().all(|i| i.methods.is_empty()) {
                return None;
            }

            // Calculate metrics for THIS struct only (with complexity lookup)
            let metrics = calculate_struct_metrics(struct_data, impl_blocks, extracted);

            // Spec 206: Cohesion gate - skip structs with high domain cohesion
            // A struct like "CrossModuleTracker" where methods align with the
            // "module/tracker" domain is cohesive, not a god object
            if is_cohesive_struct(&struct_data.name, &metrics.method_names) {
                return None;
            }

            // Classify responsibilities by behavioral categories
            let responsibilities = group_methods_by_responsibility(&metrics.method_names);

            // Check if this struct qualifies as god object
            if !is_god_object_candidate(&metrics, &responsibilities, thresholds) {
                return None;
            }

            // Build analysis for this god object
            // Spec 214: Use production LOC (excluding test code)
            Some(build_god_object_analysis(
                struct_data,
                &metrics,
                responsibilities,
                extracted.production_lines(),
                thresholds,
            ))
        })
        .collect();

    // If we found god object structs, return them
    if !struct_results.is_empty() {
        return struct_results;
    }

    // Fallback: file-level analysis for standalone functions
    // (GodFile/GodModule when no structs but many standalone functions)
    analyze_file_level(extracted, thresholds)
        .map(|a| vec![a])
        .unwrap_or_default()
}

/// Backward-compatible wrapper: returns single highest-scoring god object.
///
/// This maintains API compatibility with existing code while using the new
/// per-struct analysis under the hood.
pub fn analyze_god_object(path: &Path, extracted: &ExtractedFileData) -> Option<GodObjectAnalysis> {
    analyze_god_objects(path, extracted)
        .into_iter()
        .max_by(|a, b| {
            a.god_object_score
                .value()
                .partial_cmp(&b.god_object_score.value())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// File-level analysis for when no structs are present.
///
/// Detects GodFile (no structs, many functions) and GodModule (structs exist
/// but standalone functions dominate).
/// Spec 214: Uses production LOC (excluding test code) for scoring.
fn analyze_file_level(
    extracted: &ExtractedFileData,
    thresholds: &GodObjectThresholds,
) -> Option<GodObjectAnalysis> {
    let total_methods: usize = extracted.impls.iter().map(|i| i.methods.len()).sum();
    let total_standalone = extracted.functions.len();
    let total_fields: usize = extracted.structs.iter().map(|s| s.fields.len()).sum();

    // Need significant function count for file-level detection
    if total_standalone < 50 {
        return None;
    }

    // Determine detection type
    let detection_type = determine_detection_type(extracted, total_methods, total_standalone);

    // Only proceed for GodFile or GodModule
    if detection_type == DetectionType::GodClass {
        return None;
    }

    let method_count = total_standalone + total_methods;

    // Collect all function names for behavioral categorization
    let all_function_names: Vec<String> = extracted
        .functions
        .iter()
        .map(|f| f.name.clone())
        .chain(
            extracted
                .impls
                .iter()
                .flat_map(|i| i.methods.iter().map(|m| m.name.clone())),
        )
        .collect();

    let responsibilities = group_methods_by_responsibility(&all_function_names);
    let responsibility_method_counts: HashMap<String, usize> = responsibilities
        .iter()
        .map(|(k, v)| (k.clone(), v.len()))
        .collect();
    let responsibility_names: Vec<String> = responsibilities.keys().cloned().collect();

    // Calculate weighted method count (Spec 209)
    // Also apply pure function weighting: standalone functions have no `self` access,
    // so they're inherently pure helpers (0.2 weight each per Spec 213)
    let name_weighted_count =
        calculate_weighted_count_from_names(all_function_names.iter().map(String::as_str));

    // Standalone functions are pure by definition (no self parameter)
    // Apply pure function weight (0.2) to standalone functions
    // Impl methods use name-based weighting
    let standalone_pure_weighted = total_standalone as f64 * 0.2;
    let impl_method_names: Vec<String> = extracted
        .impls
        .iter()
        .flat_map(|i| i.methods.iter().map(|m| m.name.clone()))
        .collect();
    let impl_weighted =
        calculate_weighted_count_from_names(impl_method_names.iter().map(String::as_str));

    // Use the lower of name-based weighting or pure function weighting
    // This gives benefit of both Spec 209 (accessor detection) and Spec 213 (pure functions)
    let weighted_method_count = (standalone_pure_weighted + impl_weighted).min(name_weighted_count);

    let complexity_sum: u32 = extracted.functions.iter().map(|f| f.cyclomatic).sum();
    let visibility_breakdown = build_visibility_breakdown(extracted);

    // Calculate average complexity for weighted scoring
    let avg_complexity = if method_count > 0 {
        complexity_sum as f64 / method_count as f64
    } else {
        0.0
    };

    // Spec 214: Use production LOC for scoring and confidence
    let production_lines = extracted.production_lines();

    let confidence = crate::organization::god_object::classifier::determine_confidence(
        method_count,
        total_fields,
        responsibility_names.len(),
        production_lines,
        complexity_sum,
        thresholds,
    );

    // Use weighted scoring algorithm for consistency (Spec 212)
    // Spec 209/213: Use weighted method count instead of raw count
    // Spec 214: Use production LOC to avoid penalizing well-tested code
    let god_score = calculate_god_object_score_weighted(
        weighted_method_count,
        total_fields,
        responsibility_names.len(),
        production_lines,
        avg_complexity,
        thresholds,
    );

    // Only show weighted count if there's meaningful adjustment (>10% reduction)
    let weighted_method_count_display = if weighted_method_count < method_count as f64 * 0.9 {
        Some(weighted_method_count)
    } else {
        None
    };

    Some(GodObjectAnalysis {
        is_god_object: true,
        method_count,
        weighted_method_count: weighted_method_count_display,
        field_count: total_fields,
        responsibility_count: responsibility_names.len(),
        lines_of_code: production_lines, // Spec 214: Use production LOC
        complexity_sum,
        god_object_score: Score0To100::new(god_score),
        recommended_splits: vec![],
        confidence,
        responsibilities: responsibility_names,
        responsibility_method_counts,
        purity_distribution: None,
        module_structure: None,
        detection_type,
        struct_name: None,
        struct_line: None,
        struct_location: None,
        visibility_breakdown: Some(visibility_breakdown),
        domain_count: 0,
        domain_diversity: 0.0,
        struct_ratio: calculate_struct_ratio(extracted),
        analysis_method: SplitAnalysisMethod::None,
        cross_domain_severity: None,
        domain_diversity_metrics: None,
        aggregated_entropy: None,
        aggregated_error_swallowing_count: None,
        aggregated_error_swallowing_patterns: None,
        layering_impact: None,
        anti_pattern_report: None,
        complexity_metrics: None,   // Spec 211
        trait_method_summary: None, // Spec 217 (file-level analysis doesn't detect traits yet)
    })
}

/// Determine the type of god object detected.
fn determine_detection_type(
    extracted: &ExtractedFileData,
    impl_methods: usize,
    standalone_functions: usize,
) -> DetectionType {
    let has_structs = !extracted.structs.is_empty();

    if !has_structs && standalone_functions > 50 {
        DetectionType::GodFile
    } else if has_structs && standalone_functions > 50 && standalone_functions > impl_methods * 3 {
        DetectionType::GodModule
    } else {
        DetectionType::GodClass
    }
}

/// Build function visibility breakdown.
fn build_visibility_breakdown(extracted: &ExtractedFileData) -> FunctionVisibilityBreakdown {
    let mut breakdown = FunctionVisibilityBreakdown::new();

    for func in &extracted.functions {
        match func.visibility.as_deref() {
            Some("pub") => breakdown.public += 1,
            Some("pub(crate)") => breakdown.pub_crate += 1,
            Some("pub(super)") => breakdown.pub_super += 1,
            _ => breakdown.private += 1,
        }
    }

    // Also count impl methods
    for impl_block in &extracted.impls {
        for method in &impl_block.methods {
            if method.is_public {
                breakdown.public += 1;
            } else {
                breakdown.private += 1;
            }
        }
    }

    breakdown
}

/// Calculate struct ratio (structs / total functions).
fn calculate_struct_ratio(extracted: &ExtractedFileData) -> f64 {
    let total_funcs = extracted.functions.len()
        + extracted
            .impls
            .iter()
            .map(|i| i.methods.len())
            .sum::<usize>();

    if total_funcs == 0 {
        0.0
    } else {
        extracted.structs.len() as f64 / total_funcs as f64
    }
}

/// Analyze multiple files for god objects.
pub fn analyze_all_files(
    extracted: &HashMap<std::path::PathBuf, ExtractedFileData>,
) -> Vec<(std::path::PathBuf, GodObjectAnalysis)> {
    extracted
        .iter()
        .filter_map(|(path, data)| {
            analyze_god_object(path, data).map(|analysis| (path.clone(), analysis))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::types::{
        ExtractedFileData, ExtractedFunctionData, ExtractedImplData, ExtractedStructData,
        FieldInfo, MethodInfo, PurityAnalysisData,
    };
    use std::path::PathBuf;

    fn create_test_function(name: &str, line: usize) -> ExtractedFunctionData {
        ExtractedFunctionData {
            name: name.to_string(),
            qualified_name: name.to_string(),
            line,
            end_line: line + 10,
            length: 10,
            cyclomatic: 5,
            cognitive: 3,
            nesting: 2,
            entropy_score: None,
            purity_analysis: PurityAnalysisData::pure(),
            io_operations: vec![],
            parameter_names: vec![],
            transformation_patterns: vec![],
            calls: vec![],
            is_test: false,
            is_async: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
            in_test_module: false,
        }
    }

    fn create_large_file() -> ExtractedFileData {
        let mut functions: Vec<ExtractedFunctionData> = (0..30)
            .map(|i| create_test_function(&format!("func_{}", i), i * 20))
            .collect();

        // Add some with visibility variations
        functions[5].visibility = Some("pub(crate)".to_string());
        functions[6].visibility = None; // private

        ExtractedFileData {
            path: PathBuf::from("src/god_object.rs"),
            functions,
            structs: vec![ExtractedStructData {
                name: "BigStruct".to_string(),
                line: 1,
                fields: (0..10)
                    .map(|i| FieldInfo {
                        name: format!("field_{}", i),
                        type_str: "String".to_string(),
                        is_public: false,
                    })
                    .collect(),
                is_public: true,
            }],
            impls: vec![ExtractedImplData {
                type_name: "BigStruct".to_string(),
                trait_name: None,
                methods: (0..25)
                    .map(|i| MethodInfo {
                        name: format!("method_{}", i),
                        line: 100 + i * 10,
                        is_public: i % 2 == 0,
                    })
                    .collect(),
                line: 50,
            }],
            imports: vec![],
            total_lines: 2500,
            detected_patterns: vec![],
            test_lines: 0, // Spec 214
        }
    }

    #[test]
    fn test_small_file_not_god_object() {
        let file_data = ExtractedFileData {
            path: PathBuf::from("src/small.rs"),
            functions: vec![create_test_function("foo", 1)],
            structs: vec![],
            impls: vec![],
            imports: vec![],
            total_lines: 50,
            detected_patterns: vec![],
            test_lines: 0, // Spec 214
        };

        let result = analyze_god_object(&file_data.path, &file_data);

        assert!(result.is_none());
    }

    #[test]
    fn test_large_file_is_god_object() {
        let file_data = create_large_file();
        let result = analyze_god_object(&file_data.path, &file_data);

        assert!(result.is_some());
        let analysis = result.unwrap();
        assert!(analysis.is_god_object);
        assert_eq!(analysis.lines_of_code, 2500);
    }

    #[test]
    fn test_detection_type_god_class() {
        let file_data = create_large_file();
        let result = analyze_god_object(&file_data.path, &file_data).unwrap();

        // Has structs and impl methods > standalone, should be GodClass
        assert_eq!(result.detection_type, DetectionType::GodClass);
    }

    #[test]
    fn test_detection_type_god_file() {
        let mut file_data = create_large_file();
        file_data.structs.clear();
        file_data.impls.clear();
        file_data.functions = (0..60)
            .map(|i| create_test_function(&format!("standalone_{}", i), i * 10))
            .collect();

        let result = analyze_god_object(&file_data.path, &file_data).unwrap();

        assert_eq!(result.detection_type, DetectionType::GodFile);
    }

    #[test]
    fn test_detection_type_god_module() {
        let mut file_data = create_large_file();
        // Keep structs but add many standalone functions
        file_data.functions = (0..60)
            .map(|i| create_test_function(&format!("standalone_{}", i), i * 10))
            .collect();
        // Clear impl methods to make standalone > impl * 3
        file_data.impls.clear();

        let result = analyze_god_object(&file_data.path, &file_data).unwrap();

        assert_eq!(result.detection_type, DetectionType::GodModule);
    }

    #[test]
    fn test_visibility_breakdown_file_level() {
        // Visibility breakdown is only available for file-level analysis (GodFile/GodModule)
        let mut file_data = create_large_file();
        file_data.structs.clear();
        file_data.impls.clear();
        file_data.functions = (0..60)
            .map(|i| create_test_function(&format!("standalone_{}", i), i * 10))
            .collect();

        let result = analyze_god_object(&file_data.path, &file_data).unwrap();
        // File-level analysis should have visibility breakdown
        let breakdown = result.visibility_breakdown.unwrap();
        assert!(breakdown.public > 0);
    }

    #[test]
    fn test_behavioral_responsibilities() {
        // Test that responsibilities are behavioral categories, not type names
        let file_data = ExtractedFileData {
            path: PathBuf::from("src/big_struct.rs"),
            functions: vec![],
            structs: vec![ExtractedStructData {
                name: "BigStruct".to_string(),
                line: 1,
                fields: (0..20)
                    .map(|i| FieldInfo {
                        name: format!("field_{}", i),
                        type_str: "String".to_string(),
                        is_public: false,
                    })
                    .collect(),
                is_public: true,
            }],
            impls: vec![ExtractedImplData {
                type_name: "BigStruct".to_string(),
                trait_name: None,
                methods: vec![
                    MethodInfo {
                        name: "parse_json".to_string(),
                        line: 100,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "validate_input".to_string(),
                        line: 110,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "render_output".to_string(),
                        line: 120,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "handle_event".to_string(),
                        line: 130,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "transform_data".to_string(),
                        line: 140,
                        is_public: true,
                    },
                    // Add more methods to exceed threshold
                    MethodInfo {
                        name: "parse_xml".to_string(),
                        line: 150,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "validate_schema".to_string(),
                        line: 160,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "render_view".to_string(),
                        line: 170,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "handle_click".to_string(),
                        line: 180,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "transform_record".to_string(),
                        line: 190,
                        is_public: true,
                    },
                    // More to ensure it qualifies
                    MethodInfo {
                        name: "get_data".to_string(),
                        line: 200,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_value".to_string(),
                        line: 210,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "save_state".to_string(),
                        line: 220,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "load_config".to_string(),
                        line: 230,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "create_instance".to_string(),
                        line: 240,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "build_object".to_string(),
                        line: 250,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "send_message".to_string(),
                        line: 260,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "receive_response".to_string(),
                        line: 270,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "filter_results".to_string(),
                        line: 280,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "search_database".to_string(),
                        line: 290,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "process_request".to_string(),
                        line: 300,
                        is_public: true,
                    },
                ],
                line: 50,
            }],
            imports: vec![],
            total_lines: 500,
            detected_patterns: vec![],
            test_lines: 0, // Spec 214
        };

        let results = analyze_god_objects(&file_data.path, &file_data);
        assert!(!results.is_empty(), "Should detect god object");

        let result = &results[0];
        // Should have behavioral categories, not type names
        let has_behavioral = result.responsibilities.iter().any(|r| {
            r == "Parsing"
                || r == "Validation"
                || r == "Rendering"
                || r == "Event Handling"
                || r == "Transformation"
                || r == "Data Access"
                || r == "Persistence"
                || r == "Construction"
                || r == "Communication"
                || r == "Filtering"
                || r == "Processing"
        });
        assert!(
            has_behavioral,
            "Responsibilities should be behavioral categories, got: {:?}",
            result.responsibilities
        );
    }

    #[test]
    fn test_per_struct_analysis_multiple_structs() {
        // Spec 197: Each struct should be analyzed independently
        let file_data = ExtractedFileData {
            path: PathBuf::from("src/multi_struct.rs"),
            functions: vec![],
            structs: vec![
                ExtractedStructData {
                    name: "SmallDTO".to_string(),
                    line: 1,
                    fields: vec![
                        FieldInfo {
                            name: "id".to_string(),
                            type_str: "u64".to_string(),
                            is_public: false,
                        },
                        FieldInfo {
                            name: "name".to_string(),
                            type_str: "String".to_string(),
                            is_public: false,
                        },
                    ],
                    is_public: true,
                },
                ExtractedStructData {
                    name: "GodClass".to_string(),
                    line: 100,
                    fields: (0..20)
                        .map(|i| FieldInfo {
                            name: format!("field_{}", i),
                            type_str: "String".to_string(),
                            is_public: false,
                        })
                        .collect(),
                    is_public: true,
                },
            ],
            impls: vec![
                // No impl for SmallDTO (it's a DTO)
                ExtractedImplData {
                    type_name: "GodClass".to_string(),
                    trait_name: None,
                    methods: (0..25)
                        .map(|i| MethodInfo {
                            name: format!("handle_request_{}", i),
                            line: 200 + i * 10,
                            is_public: true,
                        })
                        .collect(),
                    line: 150,
                },
            ],
            imports: vec![],
            total_lines: 800,
            detected_patterns: vec![],
            test_lines: 0, // Spec 214
        };

        let results = analyze_god_objects(&file_data.path, &file_data);

        // Should only have GodClass, not SmallDTO
        assert_eq!(results.len(), 1, "Should only detect one god object");
        assert_eq!(results[0].struct_name, Some("GodClass".to_string()));
        assert_eq!(results[0].struct_line, Some(100));
        // Methods should only count GodClass methods
        assert_eq!(results[0].method_count, 25);
    }

    #[test]
    fn test_dto_skipped() {
        // DTOs (structs with no impl methods) should not be flagged
        let file_data = ExtractedFileData {
            path: PathBuf::from("src/dto.rs"),
            functions: vec![],
            structs: vec![ExtractedStructData {
                name: "DataOnly".to_string(),
                line: 1,
                fields: vec![
                    FieldInfo {
                        name: "id".to_string(),
                        type_str: "u64".to_string(),
                        is_public: true,
                    },
                    FieldInfo {
                        name: "name".to_string(),
                        type_str: "String".to_string(),
                        is_public: true,
                    },
                ],
                is_public: true,
            }],
            impls: vec![], // No impl blocks
            imports: vec![],
            total_lines: 50,
            detected_patterns: vec![],
            test_lines: 0, // Spec 214
        };

        let results = analyze_god_objects(&file_data.path, &file_data);
        assert!(
            results.is_empty(),
            "DTOs should not be flagged as god objects"
        );
    }

    #[test]
    fn test_struct_name_and_line_correct() {
        // Spec 197: struct_name should identify the actual god object, not first struct
        let file_data = ExtractedFileData {
            path: PathBuf::from("src/ordered.rs"),
            functions: vec![],
            structs: vec![
                ExtractedStructData {
                    name: "FirstButSmall".to_string(),
                    line: 1,
                    fields: vec![FieldInfo {
                        name: "x".to_string(),
                        type_str: "i32".to_string(),
                        is_public: false,
                    }],
                    is_public: true,
                },
                ExtractedStructData {
                    name: "ActualGodObject".to_string(),
                    line: 200,
                    fields: (0..20)
                        .map(|i| FieldInfo {
                            name: format!("field_{}", i),
                            type_str: "String".to_string(),
                            is_public: false,
                        })
                        .collect(),
                    is_public: true,
                },
            ],
            impls: vec![
                ExtractedImplData {
                    type_name: "FirstButSmall".to_string(),
                    trait_name: None,
                    methods: vec![MethodInfo {
                        name: "get_x".to_string(),
                        line: 10,
                        is_public: true,
                    }],
                    line: 5,
                },
                ExtractedImplData {
                    type_name: "ActualGodObject".to_string(),
                    trait_name: None,
                    methods: (0..30)
                        .map(|i| MethodInfo {
                            name: format!("process_{}", i),
                            line: 250 + i * 5,
                            is_public: true,
                        })
                        .collect(),
                    line: 220,
                },
            ],
            imports: vec![],
            total_lines: 600,
            detected_patterns: vec![],
            test_lines: 0, // Spec 214
        };

        let results = analyze_god_objects(&file_data.path, &file_data);
        assert_eq!(results.len(), 1);
        // Should be ActualGodObject, not FirstButSmall
        assert_eq!(results[0].struct_name, Some("ActualGodObject".to_string()));
        assert_eq!(results[0].struct_line, Some(200));
    }

    #[test]
    fn test_backward_compatible_wrapper() {
        // analyze_god_object should return highest-scoring god object
        let file_data = create_large_file();
        let result = analyze_god_object(&file_data.path, &file_data);

        assert!(result.is_some());
        let analysis = result.unwrap();
        assert!(analysis.is_god_object);
        assert_eq!(analysis.struct_name, Some("BigStruct".to_string()));
    }

    #[test]
    fn test_analyze_all_files() {
        let mut extracted = HashMap::new();

        // Add a small file (not god object)
        extracted.insert(
            PathBuf::from("small.rs"),
            ExtractedFileData::empty(PathBuf::from("small.rs")),
        );

        // Add a large file (god object) - use the path from create_large_file()
        let large_file = create_large_file();
        let large_path = large_file.path.clone();
        extracted.insert(large_path.clone(), large_file);

        let results = analyze_all_files(&extracted);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, large_path);
    }

    #[test]
    fn test_struct_ratio() {
        let mut file_data = create_large_file();
        let ratio = calculate_struct_ratio(&file_data);
        assert!(ratio > 0.0);
        assert!(ratio < 1.0);

        // Empty file
        file_data.functions.clear();
        file_data.impls.clear();
        let ratio_empty = calculate_struct_ratio(&file_data);
        assert_eq!(ratio_empty, 0.0);
    }

    // =========================================================================
    // Spec 209: Accessor and Boilerplate Method Detection Tests
    // =========================================================================

    #[test]
    fn test_accessor_heavy_struct_not_flagged() {
        // Spec 209: A struct with many accessors but few substantive methods
        // should NOT be flagged as a god object because the weighted count
        // should be low (accessors have weight 0.1, setters 0.3).
        //
        // This tests the raw count vs weighted count scenario:
        // - 25 raw methods would exceed the threshold (20)
        // - But weighted count = 0.0 + 10*0.1 + 10*0.3 + 3*1.0 = 7.0 (below threshold)
        let file_data = ExtractedFileData {
            path: PathBuf::from("src/data_container.rs"),
            functions: vec![],
            structs: vec![ExtractedStructData {
                name: "DataContainer".to_string(),
                line: 1,
                fields: (0..10)
                    .map(|i| FieldInfo {
                        name: format!("field_{}", i),
                        type_str: "String".to_string(),
                        is_public: false,
                    })
                    .collect(),
                is_public: true,
            }],
            impls: vec![ExtractedImplData {
                type_name: "DataContainer".to_string(),
                trait_name: None,
                methods: vec![
                    // 1 Boilerplate (weight 0.0)
                    MethodInfo {
                        name: "new".to_string(),
                        line: 10,
                        is_public: true,
                    },
                    // 10 Trivial accessors (weight 0.1 each = 1.0 total)
                    MethodInfo {
                        name: "get_field_0".to_string(),
                        line: 20,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_1".to_string(),
                        line: 21,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_2".to_string(),
                        line: 22,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_3".to_string(),
                        line: 23,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_4".to_string(),
                        line: 24,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_5".to_string(),
                        line: 25,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_6".to_string(),
                        line: 26,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_7".to_string(),
                        line: 27,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_8".to_string(),
                        line: 28,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_9".to_string(),
                        line: 29,
                        is_public: true,
                    },
                    // 10 Simple accessors (weight 0.3 each = 3.0 total)
                    MethodInfo {
                        name: "set_field_0".to_string(),
                        line: 30,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_1".to_string(),
                        line: 31,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_2".to_string(),
                        line: 32,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_3".to_string(),
                        line: 33,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_4".to_string(),
                        line: 34,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_5".to_string(),
                        line: 35,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_6".to_string(),
                        line: 36,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_7".to_string(),
                        line: 37,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_8".to_string(),
                        line: 38,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_9".to_string(),
                        line: 39,
                        is_public: true,
                    },
                    // 3 Substantive methods (weight 1.0 each = 3.0 total)
                    // All "build_" which goes to same "construction" responsibility
                    MethodInfo {
                        name: "build_output".to_string(),
                        line: 40,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "build_summary".to_string(),
                        line: 41,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "build_report".to_string(),
                        line: 42,
                        is_public: true,
                    },
                ],
                line: 5,
            }],
            imports: vec![],
            total_lines: 200,
            detected_patterns: vec![],
            test_lines: 0, // Spec 214
        };

        // Total: 24 raw methods (would exceed threshold of 20)
        // Weighted: 0.0 + 10*0.1 + 10*0.3 + 3*1.0 = 7.0 (below threshold)
        // Responsibilities: mostly Data Access (get/set) + Construction (build)
        // = 2-3 responsibility categories (below max_traits of 5)
        let results = analyze_god_objects(&file_data.path, &file_data);
        assert!(
            results.is_empty(),
            "Struct with 24 methods (mostly accessors) should NOT be flagged. \
             Weighted count is ~7.0 (below 20 threshold)."
        );
    }

    #[test]
    fn test_substantive_heavy_struct_flagged() {
        // A struct with mostly substantive methods SHOULD be flagged
        let file_data = ExtractedFileData {
            path: PathBuf::from("src/god_class.rs"),
            functions: vec![],
            structs: vec![ExtractedStructData {
                name: "GodClass".to_string(),
                line: 1,
                fields: (0..10)
                    .map(|i| FieldInfo {
                        name: format!("field_{}", i),
                        type_str: "String".to_string(),
                        is_public: false,
                    })
                    .collect(),
                is_public: true,
            }],
            impls: vec![ExtractedImplData {
                type_name: "GodClass".to_string(),
                trait_name: None,
                // 25 substantive methods (weight 1.0 each = 25.0 total)
                methods: (0..25)
                    .map(|i| MethodInfo {
                        name: format!("process_item_{}", i),
                        line: 10 + i * 10,
                        is_public: true,
                    })
                    .collect(),
                line: 5,
            }],
            imports: vec![],
            total_lines: 500,
            detected_patterns: vec![],
            test_lines: 0, // Spec 214
        };

        // Weighted count = 25.0 (all substantive), exceeds threshold of 20
        let results = analyze_god_objects(&file_data.path, &file_data);
        assert!(
            !results.is_empty(),
            "Struct with 25 substantive methods SHOULD be flagged as god object"
        );
        assert_eq!(results[0].struct_name, Some("GodClass".to_string()));
    }

    #[test]
    fn test_pure_accessor_struct_not_flagged() {
        // A struct with ONLY accessor/boilerplate methods should never be flagged
        let file_data = ExtractedFileData {
            path: PathBuf::from("src/dto.rs"),
            functions: vec![],
            structs: vec![ExtractedStructData {
                name: "DataTransferObject".to_string(),
                line: 1,
                fields: (0..10)
                    .map(|i| FieldInfo {
                        name: format!("field_{}", i),
                        type_str: "String".to_string(),
                        is_public: false,
                    })
                    .collect(),
                is_public: true,
            }],
            impls: vec![ExtractedImplData {
                type_name: "DataTransferObject".to_string(),
                trait_name: None,
                methods: vec![
                    // 1 boilerplate
                    MethodInfo {
                        name: "new".to_string(),
                        line: 10,
                        is_public: true,
                    },
                    // 20 trivial accessors (weight = 20 * 0.1 = 2.0)
                    MethodInfo {
                        name: "get_field_0".to_string(),
                        line: 20,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_1".to_string(),
                        line: 21,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_2".to_string(),
                        line: 22,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_3".to_string(),
                        line: 23,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_4".to_string(),
                        line: 24,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_5".to_string(),
                        line: 25,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_6".to_string(),
                        line: 26,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_7".to_string(),
                        line: 27,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_8".to_string(),
                        line: 28,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "get_field_9".to_string(),
                        line: 29,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_0".to_string(),
                        line: 30,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_1".to_string(),
                        line: 31,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_2".to_string(),
                        line: 32,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_3".to_string(),
                        line: 33,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_4".to_string(),
                        line: 34,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_5".to_string(),
                        line: 35,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_6".to_string(),
                        line: 36,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_7".to_string(),
                        line: 37,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_8".to_string(),
                        line: 38,
                        is_public: true,
                    },
                    MethodInfo {
                        name: "set_field_9".to_string(),
                        line: 39,
                        is_public: true,
                    },
                ],
                line: 5,
            }],
            imports: vec![],
            total_lines: 100,
            detected_patterns: vec![],
            test_lines: 0, // Spec 214
        };

        // 21 methods raw, but weighted = 0.0 + 10*0.1 + 10*0.3 = 4.0
        // Should NOT trigger god object
        let results = analyze_god_objects(&file_data.path, &file_data);
        assert!(
            results.is_empty(),
            "Struct with 21 pure accessor methods should NOT be flagged. \
             Weighted count should be ~4.0, not 21."
        );
    }

    #[test]
    fn test_weighted_count_calculation() {
        // Verify the weighted count math directly
        use crate::organization::god_object::classifier::calculate_weighted_count_from_names;

        let methods = [
            "new",      // 0.0 (boilerplate)
            "get_a",    // 0.1 (trivial)
            "get_b",    // 0.1 (trivial)
            "set_a",    // 0.3 (simple)
            "set_b",    // 0.3 (simple)
            "process",  // 1.0 (substantive)
            "validate", // 1.0 (substantive)
        ];

        let weighted = calculate_weighted_count_from_names(methods.iter().copied());
        // Expected: 0.0 + 0.1 + 0.1 + 0.3 + 0.3 + 1.0 + 1.0 = 2.8
        assert!(
            (weighted - 2.8).abs() < 0.01,
            "Expected 2.8, got {}",
            weighted
        );
    }
}
