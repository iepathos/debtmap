//! Shared heuristic god object detection for fallback cases.
//!
//! This module provides simple text-based heuristics for detecting god objects
//! when full AST analysis is not available (e.g., non-Rust files, fallback cases).
//!
//! # Design (Spec 212 - Consolidation)
//!
//! The heuristics in this module are extracted from duplicate implementations
//! across file_analyzer.rs and parallel_unified_analysis.rs to provide a single
//! source of truth for fallback detection logic.

use crate::organization::god_object::scoring::calculate_god_object_score_weighted;
use crate::organization::god_object::{
    DetectionType, GodObjectAnalysis, GodObjectConfidence, GodObjectThresholds, SplitAnalysisMethod,
};

use std::collections::HashMap;

/// Thresholds for simple heuristic-based detection.
pub const HEURISTIC_MAX_FUNCTIONS: usize = 50;
pub const HEURISTIC_MAX_LINES: usize = 2000;
pub const HEURISTIC_MAX_FIELDS: usize = 30;

/// Pure heuristic god object detection for simple/fallback cases.
///
/// Used when AST analysis is not available (non-Rust files) or as a fallback
/// when primary analysis doesn't detect a potential god object.
///
/// # Arguments
///
/// * `function_count` - Number of functions detected (can be from regex matching)
/// * `line_count` - Total lines in the file
/// * `field_count` - Estimated field count (can be from regex matching)
/// * `complexity_sum` - Optional total complexity (0 if not available)
///
/// # Returns
///
/// `Some(GodObjectAnalysis)` if heuristics indicate a god object, `None` otherwise.
pub fn fallback_god_object_heuristics(
    function_count: usize,
    line_count: usize,
    field_count: usize,
    complexity_sum: u32,
) -> Option<GodObjectAnalysis> {
    let is_god_object = function_count > HEURISTIC_MAX_FUNCTIONS
        || line_count > HEURISTIC_MAX_LINES
        || field_count > HEURISTIC_MAX_FIELDS;

    if !is_god_object {
        return None;
    }

    let thresholds = GodObjectThresholds::default();

    // Calculate average complexity for weighted scoring
    let avg_complexity = if function_count > 0 && complexity_sum > 0 {
        complexity_sum as f64 / function_count as f64
    } else {
        5.0 // Default moderate complexity when not available
    };

    // Estimate responsibility count based on function count
    let estimated_resp_count = (function_count / 10).clamp(1, 10);
    let responsibilities: Vec<String> = (1..=estimated_resp_count)
        .map(|i| format!("responsibility_{}", i))
        .collect();
    let responsibility_method_counts: HashMap<String, usize> = responsibilities
        .iter()
        .map(|r| (r.clone(), function_count / estimated_resp_count))
        .collect();

    // Use weighted scoring for consistency (Spec 212)
    let god_object_score = calculate_god_object_score_weighted(
        function_count as f64,
        field_count,
        estimated_resp_count,
        line_count,
        avg_complexity,
        &thresholds,
    );

    Some(GodObjectAnalysis {
        is_god_object: true,
        method_count: function_count,
        weighted_method_count: None,
        field_count,
        responsibility_count: estimated_resp_count,
        lines_of_code: line_count,
        complexity_sum,
        god_object_score: god_object_score.max(0.0),
        recommended_splits: Vec::new(),
        confidence: GodObjectConfidence::Possible,
        responsibilities,
        responsibility_method_counts,
        purity_distribution: None,
        module_structure: None,
        detection_type: DetectionType::GodFile,
        struct_name: None,
        struct_line: None,
        struct_location: None,
        visibility_breakdown: None,
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
        complexity_metrics: None,   // Spec 211
        trait_method_summary: None, // Spec 217
    })
}

/// Text-based heuristic detection from raw file content.
///
/// Uses regex-like matching to count functions, fields, and lines from raw text.
/// Suitable for non-Rust files or when AST parsing is not available.
///
/// # Arguments
///
/// * `content` - Raw file content as string
///
/// # Returns
///
/// `Some(GodObjectAnalysis)` if heuristics indicate a god object, `None` otherwise.
pub fn detect_from_content(content: &str) -> Option<GodObjectAnalysis> {
    let line_count = content.lines().count();

    // Count functions across multiple languages
    let function_count = content.matches("fn ").count()
        + content.matches("def ").count()
        + content.matches("function ").count();

    // Estimate field count (rough heuristic)
    let field_count = content.matches("pub ").count() + content.matches("self.").count() / 3;

    fallback_god_object_heuristics(function_count, line_count, field_count, 0)
}

/// Heuristic fallback with optional preserved analysis data.
///
/// Used when primary analysis exists but didn't flag a god object, yet heuristic
/// thresholds are met. Preserves responsibilities and other data from the primary
/// analysis while creating a god object result.
///
/// # Arguments
///
/// * `function_count` - Number of functions
/// * `line_count` - Total lines in the file
/// * `complexity_sum` - Total complexity sum
/// * `existing_analysis` - Optional existing analysis to preserve responsibilities from
///
/// # Returns
///
/// `Some(GodObjectAnalysis)` if heuristics indicate a god object, `None` otherwise.
pub fn fallback_with_preserved_analysis(
    function_count: usize,
    line_count: usize,
    complexity_sum: u32,
    existing_analysis: Option<&GodObjectAnalysis>,
) -> Option<GodObjectAnalysis> {
    // Check if heuristic thresholds are met
    if function_count <= HEURISTIC_MAX_FUNCTIONS && line_count <= HEURISTIC_MAX_LINES {
        return None;
    }

    let thresholds = GodObjectThresholds::default();

    // Calculate average complexity for weighted scoring
    let avg_complexity = if function_count > 0 && complexity_sum > 0 {
        complexity_sum as f64 / function_count as f64
    } else {
        5.0 // Default moderate complexity
    };

    // Try to preserve responsibilities from existing analysis
    let (responsibilities, responsibility_method_counts, responsibility_count) =
        if let Some(analysis) = existing_analysis {
            if !analysis.responsibilities.is_empty() {
                (
                    analysis.responsibilities.clone(),
                    analysis.responsibility_method_counts.clone(),
                    analysis.responsibility_count,
                )
            } else {
                estimate_responsibilities(function_count)
            }
        } else {
            estimate_responsibilities(function_count)
        };

    // Use weighted scoring for consistency (Spec 212)
    let god_object_score = calculate_god_object_score_weighted(
        function_count as f64,
        0, // field_count not available in this context
        responsibility_count,
        line_count,
        avg_complexity,
        &thresholds,
    );

    Some(GodObjectAnalysis {
        is_god_object: true,
        method_count: function_count,
        weighted_method_count: None,
        field_count: 0,
        responsibility_count,
        lines_of_code: line_count,
        complexity_sum,
        god_object_score: god_object_score.max(0.0),
        recommended_splits: Vec::new(),
        confidence: GodObjectConfidence::Probable,
        responsibilities,
        responsibility_method_counts,
        purity_distribution: None,
        module_structure: None,
        detection_type: DetectionType::GodFile,
        struct_name: None,
        struct_line: None,
        struct_location: None,
        visibility_breakdown: None,
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
        complexity_metrics: None,   // Spec 211
        trait_method_summary: None, // Spec 217
    })
}

/// Helper: Estimate responsibilities based on function count.
fn estimate_responsibilities(
    function_count: usize,
) -> (Vec<String>, HashMap<String, usize>, usize) {
    let estimated_resp_count = (function_count / 10).clamp(1, 10);
    let responsibilities: Vec<String> = (1..=estimated_resp_count)
        .map(|i| format!("responsibility_{}", i))
        .collect();
    let responsibility_method_counts: HashMap<String, usize> = responsibilities
        .iter()
        .map(|r| (r.clone(), function_count / estimated_resp_count))
        .collect();
    (
        responsibilities,
        responsibility_method_counts,
        estimated_resp_count,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_file_not_god_object() {
        let result = fallback_god_object_heuristics(10, 500, 5, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_many_functions_is_god_object() {
        let result = fallback_god_object_heuristics(60, 1000, 10, 0);
        assert!(result.is_some());
        let analysis = result.unwrap();
        assert!(analysis.is_god_object);
        assert_eq!(analysis.method_count, 60);
    }

    #[test]
    fn test_many_lines_is_god_object() {
        let result = fallback_god_object_heuristics(20, 3000, 10, 0);
        assert!(result.is_some());
        let analysis = result.unwrap();
        assert!(analysis.is_god_object);
        assert_eq!(analysis.lines_of_code, 3000);
    }

    #[test]
    fn test_many_fields_is_god_object() {
        let result = fallback_god_object_heuristics(20, 500, 40, 0);
        assert!(result.is_some());
        let analysis = result.unwrap();
        assert!(analysis.is_god_object);
        assert_eq!(analysis.field_count, 40);
    }

    #[test]
    fn test_detect_from_content() {
        let content = r#"
fn foo() {}
fn bar() {}
fn baz() {}
pub struct Thing { pub field: i32 }
"#;
        // This small file should not be a god object
        let result = detect_from_content(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_from_content_large() {
        // Create content with many functions
        let functions: String = (0..60).map(|i| format!("fn func_{}() {{}}\n", i)).collect();
        let result = detect_from_content(&functions);
        assert!(result.is_some());
    }

    #[test]
    fn test_responsibilities_estimated() {
        let result = fallback_god_object_heuristics(100, 3000, 20, 0).unwrap();
        // 100 functions / 10 = 10 responsibilities
        assert_eq!(result.responsibility_count, 10);
        assert_eq!(result.responsibilities.len(), 10);
    }
}
