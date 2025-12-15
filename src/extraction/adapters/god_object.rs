//! God object adapter for analyzing extracted data for god object patterns.
//!
//! This module provides pure analysis functions that detect god object patterns
//! from `ExtractedFileData` without requiring additional file parsing.
//!
//! # Design
//!
//! All functions in this module are pure (no I/O, no parsing). They perform O(n)
//! analysis where n is the number of functions and structs being analyzed.

use crate::extraction::types::ExtractedFileData;
use crate::organization::god_object::{
    DetectionType, FunctionVisibilityBreakdown, GodObjectAnalysis, GodObjectConfidence,
    SplitAnalysisMethod,
};
use crate::priority::score_types::Score0To100;
use std::collections::HashMap;
use std::path::Path;

/// Thresholds for god object detection.
#[derive(Debug, Clone)]
pub struct GodObjectThresholds {
    /// Minimum methods to consider as potential god object
    pub min_methods: usize,
    /// Minimum lines to consider as potential god object
    pub min_lines: usize,
    /// Maximum methods before classification
    pub method_threshold: usize,
    /// Maximum lines before classification
    pub line_threshold: usize,
}

impl Default for GodObjectThresholds {
    fn default() -> Self {
        Self {
            min_methods: 20,
            min_lines: 500,
            method_threshold: 50,
            line_threshold: 2000,
        }
    }
}

/// Analyze extracted file data for god object patterns.
///
/// This is a pure function with no file I/O.
///
/// # Arguments
///
/// * `path` - Path to the file being analyzed
/// * `extracted` - Extracted file data
///
/// # Returns
///
/// `Some(GodObjectAnalysis)` if the file qualifies as a god object, `None` otherwise.
pub fn analyze_god_object(path: &Path, extracted: &ExtractedFileData) -> Option<GodObjectAnalysis> {
    analyze_with_thresholds(path, extracted, &GodObjectThresholds::default())
}

/// Analyze with custom thresholds.
pub fn analyze_with_thresholds(
    _path: &Path,
    extracted: &ExtractedFileData,
    thresholds: &GodObjectThresholds,
) -> Option<GodObjectAnalysis> {
    let total_methods: usize = extracted.impls.iter().map(|i| i.methods.len()).sum();
    let total_standalone = extracted.functions.len();
    let total_fields: usize = extracted.structs.iter().map(|s| s.fields.len()).sum();

    // Check minimum thresholds
    if total_methods + total_standalone < thresholds.min_methods
        && extracted.total_lines < thresholds.min_lines
    {
        return None;
    }

    // Determine detection type
    let detection_type = determine_detection_type(extracted, total_methods, total_standalone);

    // Calculate god object score
    // For GodClass, use impl methods if available, otherwise use standalone functions
    let method_count = match detection_type {
        DetectionType::GodClass => {
            if total_methods > 0 {
                total_methods
            } else {
                total_standalone
            }
        }
        DetectionType::GodFile | DetectionType::GodModule => total_standalone + total_methods,
    };

    let method_score = (method_count as f64 / thresholds.method_threshold as f64 * 50.0).min(50.0);
    let loc_score =
        (extracted.total_lines as f64 / thresholds.line_threshold as f64 * 50.0).min(50.0);
    let god_score = method_score + loc_score;

    // Check if it qualifies as a god object
    let is_god_object = god_score > 50.0
        || method_count > thresholds.method_threshold
        || extracted.total_lines > thresholds.line_threshold;

    if !is_god_object {
        return None;
    }

    // Extract responsibilities from impl blocks and function names
    let responsibilities = extract_responsibilities(extracted);
    let responsibility_method_counts = count_responsibility_methods(&responsibilities, extracted);

    // Calculate complexity sum
    let complexity_sum: u32 = extracted.functions.iter().map(|f| f.cyclomatic).sum();

    // Build visibility breakdown
    let visibility_breakdown = build_visibility_breakdown(extracted);

    // Determine confidence
    let confidence = determine_confidence(god_score, method_count, extracted.total_lines);

    Some(GodObjectAnalysis {
        is_god_object: true,
        method_count,
        field_count: total_fields,
        responsibility_count: responsibilities.len(),
        lines_of_code: extracted.total_lines,
        complexity_sum,
        god_object_score: Score0To100::new(god_score),
        recommended_splits: vec![],
        confidence,
        responsibilities: responsibilities.clone(),
        responsibility_method_counts,
        purity_distribution: None,
        module_structure: None,
        detection_type,
        struct_name: extracted.structs.first().map(|s| s.name.clone()),
        struct_line: extracted.structs.first().map(|s| s.line),
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

/// Extract responsibility names from impl blocks and function patterns.
fn extract_responsibilities(extracted: &ExtractedFileData) -> Vec<String> {
    let mut responsibilities = Vec::new();

    // Get responsibilities from impl blocks
    for impl_block in &extracted.impls {
        let name = impl_block
            .trait_name
            .clone()
            .unwrap_or_else(|| impl_block.type_name.clone());

        if !responsibilities.contains(&name) {
            responsibilities.push(name);
        }
    }

    // If no impl blocks, infer from function name prefixes
    if responsibilities.is_empty() {
        let prefixes = extract_function_prefixes(&extracted.functions);
        responsibilities.extend(prefixes);
    }

    // Ensure at least one responsibility if there are methods
    if responsibilities.is_empty() && !extracted.functions.is_empty() {
        responsibilities.push("General".to_string());
    }

    responsibilities
}

/// Extract common function name prefixes as potential responsibilities.
fn extract_function_prefixes(
    functions: &[crate::extraction::types::ExtractedFunctionData],
) -> Vec<String> {
    let mut prefix_counts: HashMap<String, usize> = HashMap::new();

    for func in functions {
        if let Some(prefix) = extract_prefix(&func.name) {
            *prefix_counts.entry(prefix).or_insert(0) += 1;
        }
    }

    // Return prefixes that appear multiple times
    prefix_counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(prefix, _)| to_pascal_case(&prefix))
        .collect()
}

/// Extract prefix from a function name (e.g., "handle_request" -> "handle").
fn extract_prefix(name: &str) -> Option<String> {
    let parts: Vec<&str> = name.split('_').collect();
    if parts.len() >= 2 {
        Some(parts[0].to_string())
    } else {
        None
    }
}

/// Convert snake_case to PascalCase.
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Count methods per responsibility.
fn count_responsibility_methods(
    responsibilities: &[String],
    extracted: &ExtractedFileData,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();

    for impl_block in &extracted.impls {
        let name = impl_block
            .trait_name
            .clone()
            .unwrap_or_else(|| impl_block.type_name.clone());

        *counts.entry(name).or_insert(0) += impl_block.methods.len();
    }

    // For responsibilities inferred from prefixes, count matching functions
    for resp in responsibilities {
        if !counts.contains_key(resp) {
            let prefix = resp.to_lowercase();
            let count = extracted
                .functions
                .iter()
                .filter(|f| f.name.starts_with(&prefix))
                .count();
            if count > 0 {
                counts.insert(resp.clone(), count);
            }
        }
    }

    counts
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

/// Determine confidence level based on metrics.
fn determine_confidence(score: f64, methods: usize, lines: usize) -> GodObjectConfidence {
    if score > 80.0 && methods > 80 && lines > 3000 {
        GodObjectConfidence::Definite
    } else if score > 60.0 && (methods > 50 || lines > 2000) {
        GodObjectConfidence::Probable
    } else if score > 40.0 {
        GodObjectConfidence::Possible
    } else {
        GodObjectConfidence::NotGodObject
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
    fn test_visibility_breakdown() {
        let file_data = create_large_file();
        let result = analyze_god_object(&file_data.path, &file_data).unwrap();

        let breakdown = result.visibility_breakdown.unwrap();
        assert!(breakdown.public > 0);
        assert!(breakdown.private > 0);
    }

    #[test]
    fn test_responsibility_extraction() {
        let mut file_data = create_large_file();
        file_data.impls = vec![
            ExtractedImplData {
                type_name: "BigStruct".to_string(),
                trait_name: Some("Display".to_string()),
                methods: vec![MethodInfo {
                    name: "fmt".to_string(),
                    line: 100,
                    is_public: true,
                }],
                line: 50,
            },
            ExtractedImplData {
                type_name: "BigStruct".to_string(),
                trait_name: None,
                methods: (0..20)
                    .map(|i| MethodInfo {
                        name: format!("method_{}", i),
                        line: 200 + i * 10,
                        is_public: true,
                    })
                    .collect(),
                line: 150,
            },
        ];

        let result = analyze_god_object(&file_data.path, &file_data).unwrap();

        assert!(result.responsibilities.contains(&"Display".to_string()));
        assert!(result.responsibilities.contains(&"BigStruct".to_string()));
    }

    #[test]
    fn test_confidence_levels() {
        assert_eq!(
            determine_confidence(85.0, 100, 4000),
            GodObjectConfidence::Definite
        );
        assert_eq!(
            determine_confidence(65.0, 55, 2500),
            GodObjectConfidence::Probable
        );
        assert_eq!(
            determine_confidence(45.0, 25, 800),
            GodObjectConfidence::Possible
        );
        assert_eq!(
            determine_confidence(30.0, 15, 300),
            GodObjectConfidence::NotGodObject
        );
    }

    #[test]
    fn test_custom_thresholds() {
        let file_data = ExtractedFileData {
            path: PathBuf::from("src/medium.rs"),
            functions: (0..15)
                .map(|i| create_test_function(&format!("func_{}", i), i * 10))
                .collect(),
            structs: vec![],
            impls: vec![],
            imports: vec![],
            total_lines: 400,
        };

        // Default thresholds - not a god object
        let result_default = analyze_god_object(&file_data.path, &file_data);
        assert!(result_default.is_none());

        // Custom lower thresholds
        let custom = GodObjectThresholds {
            min_methods: 10,
            min_lines: 300,
            method_threshold: 20,
            line_threshold: 1000,
        };

        let result_custom = analyze_with_thresholds(&file_data.path, &file_data, &custom);
        assert!(result_custom.is_some());
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
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("handle"), "Handle");
        assert_eq!(to_pascal_case("process_data"), "ProcessData");
        assert_eq!(to_pascal_case("get"), "Get");
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
}
