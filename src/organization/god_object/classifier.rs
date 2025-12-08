//! # God Object Classification (Pure Transformations)
//!
//! Pure functions for classifying and grouping god objects.
//!
//! All functions in this module are:
//! - Pure: No side effects, deterministic outputs
//! - Composable: Can be combined and chained
//! - Testable: No mocks or I/O needed

use std::collections::HashMap;

use super::classification_types::{ClassificationResult, SignalType};
use super::thresholds::GodObjectThresholds;
use super::types::GodObjectConfidence;
use crate::organization::confidence::MINIMUM_CONFIDENCE;

/// Determine confidence level from score and metrics.
///
/// Maps threshold violations to confidence levels:
/// - 5 violations → Definite
/// - 3-4 violations → Probable
/// - 1-2 violations → Possible
/// - 0 violations → NotGodObject
///
/// # Arguments
///
/// * `method_count` - Number of methods in the type
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `complexity_sum` - Sum of cyclomatic complexity
/// * `thresholds` - Threshold configuration
///
/// # Returns
///
/// Confidence level based on number of threshold violations
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{GodObjectThresholds, GodObjectConfidence};
/// use debtmap::organization::god_object::classifier::determine_confidence;
///
/// let thresholds = GodObjectThresholds::default();
/// let confidence = determine_confidence(30, 20, 8, 1500, 300, &thresholds);
/// assert_eq!(confidence, GodObjectConfidence::Definite);
/// ```
pub fn determine_confidence(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_sum: u32,
    thresholds: &GodObjectThresholds,
) -> GodObjectConfidence {
    let mut violations = 0;

    if method_count > thresholds.max_methods {
        violations += 1;
    }
    if field_count > thresholds.max_fields {
        violations += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violations += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violations += 1;
    }
    if complexity_sum > thresholds.max_complexity {
        violations += 1;
    }

    match violations {
        5 => GodObjectConfidence::Definite,
        3..=4 => GodObjectConfidence::Probable,
        1..=2 => GodObjectConfidence::Possible,
        _ => GodObjectConfidence::NotGodObject,
    }
}

/// Infer method responsibility domain from name and optional body.
///
/// This is a pure classification function that analyzes method names to determine
/// their likely responsibility category. Returns `None` category if confidence
/// is below the minimum threshold.
///
/// # Arguments
///
/// * `method_name` - The name of the method to classify
/// * `method_body` - Optional method body for deeper analysis (currently unused)
///
/// # Returns
///
/// A `ClassificationResult` with:
/// - `category`: `Some(name)` if confidence ≥ threshold, `None` otherwise
/// - `confidence`: Score from 0.0 to 1.0
/// - `signals_used`: Which signals contributed to the classification
///
/// # Confidence Thresholds
///
/// - Recognized categories: 0.85 (high confidence)
/// - Domain fallback: 0.45 (low confidence, rejected by MINIMUM_CONFIDENCE of 0.50)
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::infer_responsibility_with_confidence;
///
/// // High confidence classification
/// let result = infer_responsibility_with_confidence("parse_json", None);
/// assert!(result.category.is_some());
/// assert!(result.confidence >= 0.50);
///
/// // Low confidence - refused classification
/// let result = infer_responsibility_with_confidence("helper", None);
/// // May return None if confidence too low
/// ```
///
/// # Implementation
///
/// Currently uses name-based heuristics as the primary signal.
/// Future enhancements will integrate:
/// - I/O detection (weight: 0.40)
/// - Call graph analysis (weight: 0.30)
/// - Type signatures (weight: 0.15)
/// - Purity analysis (weight: 0.10)
pub fn infer_responsibility_with_confidence(
    method_name: &str,
    _method_body: Option<&str>,
) -> ClassificationResult {
    use crate::organization::BehavioralCategorizer;

    let category = BehavioralCategorizer::categorize_method(method_name);
    let category_name = category.display_name();

    // Assign confidence based on category type
    let confidence = match category {
        crate::organization::BehaviorCategory::Domain(_) => 0.45, // Below threshold
        crate::organization::BehaviorCategory::Utilities => 0.75, // Good confidence for utilities
        _ => 0.85, // High confidence for recognized patterns
    };

    // Apply confidence thresholds
    if confidence < MINIMUM_CONFIDENCE {
        log::debug!(
            "Low confidence classification for method '{}': confidence {:.2} below minimum {:.2}",
            method_name,
            confidence,
            MINIMUM_CONFIDENCE
        );
        return ClassificationResult {
            category: None,
            confidence,
            signals_used: vec![SignalType::NameHeuristic],
        };
    }

    ClassificationResult {
        category: Some(category_name),
        confidence,
        signals_used: vec![SignalType::NameHeuristic],
    }
}

/// Group methods by inferred responsibility domain.
///
/// This is a pure transformation that categorizes methods based on their names.
/// Methods with low confidence classification are grouped as "unclassified".
///
/// # Arguments
///
/// * `methods` - List of method names to classify
///
/// # Returns
///
/// HashMap mapping responsibility categories to lists of methods
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::group_methods_by_responsibility;
///
/// let methods = vec![
///     "parse_json".to_string(),
///     "format_output".to_string(),
///     "validate_input".to_string(),
/// ];
/// let groups = group_methods_by_responsibility(&methods);
/// assert!(groups.contains_key("Parsing"));
/// ```
pub fn group_methods_by_responsibility(methods: &[String]) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for method in methods {
        let result = infer_responsibility_with_confidence(method, None);

        // If confidence is too low (None category), keep method in original location
        // by assigning it to "unclassified" group
        let responsibility = result
            .category
            .unwrap_or_else(|| "unclassified".to_string());

        groups
            .entry(responsibility)
            .or_default()
            .push(method.clone());
    }

    groups
}

/// Pure function: analyzes function name and returns primary responsibility category.
///
/// Reuses existing behavioral categorization infrastructure to provide uniform
/// responsibility analysis across all debt items (not just god objects).
///
/// # Arguments
///
/// * `function_name` - Name of the function to analyze
///
/// # Returns
///
/// * `Some(category)` - If behavioral category can be inferred with high confidence (>= 0.7)
/// * `None` - If function name doesn't clearly indicate a behavioral pattern
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::analyze_function_responsibility;
///
/// assert_eq!(analyze_function_responsibility("validate_email"), Some("Validation".to_string()));
/// assert_eq!(analyze_function_responsibility("parse_json"), Some("Parsing".to_string()));
/// assert_eq!(analyze_function_responsibility("get_user"), Some("Data Access".to_string()));
/// assert_eq!(analyze_function_responsibility("do_stuff"), None); // Low confidence
/// ```
///
/// # Stillwater Principle: Pure Core
///
/// This function is pure - same input always gives same output, no side effects.
/// Responsibility inference happens once during analysis, not during rendering.
pub fn analyze_function_responsibility(function_name: &str) -> Option<String> {
    // Reuse existing inference with confidence threshold
    let result = infer_responsibility_with_confidence(function_name, None);

    // Only return category if confidence meets threshold (>= 0.7)
    // Threshold of 0.7 matches existing god object analysis patterns
    // The infer_responsibility_with_confidence function uses MINIMUM_CONFIDENCE (0.50)
    // but we want higher confidence for universal responsibility analysis
    if result.confidence >= 0.7 {
        result.category
    } else {
        None
    }
}

/// Classify struct into a domain based on naming patterns.
///
/// Pure function that extracts semantic domain from struct names:
/// - `*Weight`, `*Multiplier`, `*Factor`, `*Scoring` → "scoring"
/// - `*Threshold`, `*Limit`, `*Bound` → "thresholds"
/// - `*Detection`, `*Detector`, `*Checker` → "detection"
/// - `*Config`, `*Settings`, `*Options` → "core_config"
/// - `*Data`, `*Info`, `*Metrics` → "data"
/// - Default: Extract first meaningful word from name
///
/// # Arguments
///
/// * `struct_name` - Name of the struct to classify
///
/// # Returns
///
/// Domain name as a string
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::classify_struct_domain;
///
/// assert_eq!(classify_struct_domain("ThresholdConfig"), "thresholds");
/// assert_eq!(classify_struct_domain("ScoringWeight"), "scoring");
/// assert_eq!(classify_struct_domain("DetectorSettings"), "detection");
/// ```
pub fn classify_struct_domain(struct_name: &str) -> String {
    let lower = struct_name.to_lowercase();

    if lower.contains("weight")
        || lower.contains("multiplier")
        || lower.contains("factor")
        || lower.contains("scoring")
    {
        "scoring".to_string()
    } else if lower.contains("threshold") || lower.contains("limit") || lower.contains("bound") {
        "thresholds".to_string()
    } else if lower.contains("detection") || lower.contains("detector") || lower.contains("checker")
    {
        "detection".to_string()
    } else if lower.contains("config") || lower.contains("settings") || lower.contains("options") {
        "core_config".to_string()
    } else if lower.contains("data") || lower.contains("info") || lower.contains("metrics") {
        "data".to_string()
    } else {
        // Extract first meaningful word from struct name as domain
        extract_domain_from_name(struct_name)
    }
}

/// Extract domain name from struct/type name by taking first meaningful word.
///
/// Handles both camelCase/PascalCase and snake_case naming conventions.
///
/// # Arguments
///
/// * `name` - Struct or type name
///
/// # Returns
///
/// Extracted domain name
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::extract_domain_from_name;
///
/// assert_eq!(extract_domain_from_name("UserProfile"), "User");
/// assert_eq!(extract_domain_from_name("user_data"), "user");
/// ```
pub fn extract_domain_from_name(name: &str) -> String {
    // First try snake_case extraction
    if name.contains('_') {
        return name
            .split('_')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Core".to_string());
    }

    // Handle camelCase and PascalCase
    let mut domain = String::new();
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            break;
        }
        domain.push(c);
    }

    if !domain.is_empty() {
        domain
    } else {
        "Core".to_string()
    }
}

/// Count distinct semantic domains in struct list.
///
/// Pure aggregation function that counts unique domain classifications.
///
/// # Arguments
///
/// * `structs` - List of struct metrics
///
/// # Returns
///
/// Number of distinct domains
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::count_distinct_domains;
/// use debtmap::organization::god_object::StructMetrics;
///
/// let structs = vec![
///     StructMetrics {
///         name: "ThresholdConfig".to_string(),
///         line_span: (0, 10),
///         method_count: 2,
///         field_count: 5,
///         responsibilities: vec!["configuration".to_string()],
///     },
///     StructMetrics {
///         name: "ScoringWeight".to_string(),
///         line_span: (11, 20),
///         method_count: 3,
///         field_count: 4,
///         responsibilities: vec!["calculation".to_string()],
///     },
/// ];
/// assert_eq!(count_distinct_domains(&structs), 2);
/// ```
pub fn count_distinct_domains(structs: &[super::types::StructMetrics]) -> usize {
    use std::collections::HashSet;
    let domains: HashSet<String> = structs
        .iter()
        .map(|s| classify_struct_domain(&s.name))
        .collect();
    domains.len()
}

/// Calculate struct-to-function ratio.
///
/// Pure computation that measures how struct-heavy a module is.
/// Returns 0.0 if total_functions is 0 to avoid division by zero.
///
/// # Arguments
///
/// * `struct_count` - Number of structs in the module
/// * `total_functions` - Total number of functions
///
/// # Returns
///
/// Ratio of structs to functions (0.0 if no functions)
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::calculate_struct_ratio;
///
/// assert_eq!(calculate_struct_ratio(8, 10), 0.8);
/// assert_eq!(calculate_struct_ratio(10, 0), 0.0);
/// ```
pub fn calculate_struct_ratio(struct_count: usize, total_functions: usize) -> f64 {
    if total_functions == 0 {
        return 0.0;
    }
    (struct_count as f64) / (total_functions as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_mapping_definite() {
        let thresholds = GodObjectThresholds::default();
        // All 5 thresholds violated
        let confidence = determine_confidence(30, 20, 8, 1500, 300, &thresholds);
        assert_eq!(confidence, GodObjectConfidence::Definite);
    }

    #[test]
    fn test_confidence_mapping_probable() {
        let thresholds = GodObjectThresholds::default();
        // 3 thresholds violated (methods, fields, responsibilities)
        let confidence = determine_confidence(30, 20, 8, 500, 100, &thresholds);
        assert_eq!(confidence, GodObjectConfidence::Probable);
    }

    #[test]
    fn test_confidence_mapping_possible() {
        let thresholds = GodObjectThresholds::default();
        // 2 thresholds violated (methods, fields)
        let confidence = determine_confidence(30, 20, 3, 500, 100, &thresholds);
        assert_eq!(confidence, GodObjectConfidence::Possible);
    }

    #[test]
    fn test_confidence_mapping_not_god_object() {
        let thresholds = GodObjectThresholds::default();
        // No thresholds violated
        let confidence = determine_confidence(10, 8, 3, 500, 100, &thresholds);
        assert_eq!(confidence, GodObjectConfidence::NotGodObject);
    }

    #[test]
    fn test_group_methods_by_responsibility_basic() {
        let methods = vec![
            "parse_json".to_string(),
            "format_output".to_string(),
            "validate_input".to_string(),
        ];
        let groups = group_methods_by_responsibility(&methods);
        assert!(groups.contains_key("Parsing"));
        assert!(groups.contains_key("Rendering"));
        assert!(groups.contains_key("Validation"));
    }

    #[test]
    fn test_group_methods_by_responsibility_unclassified() {
        let methods = vec!["foo".to_string()]; // Single-word method with no pattern
        let groups = group_methods_by_responsibility(&methods);
        // Low confidence methods go to "unclassified"
        assert!(groups.contains_key("unclassified"));
    }

    #[test]
    fn test_classify_struct_domain_scoring() {
        assert_eq!(classify_struct_domain("ScoringWeight"), "scoring");
        assert_eq!(classify_struct_domain("MultiplicandFactor"), "scoring");
    }

    #[test]
    fn test_classify_struct_domain_thresholds() {
        assert_eq!(classify_struct_domain("ThresholdConfig"), "thresholds");
        assert_eq!(classify_struct_domain("LimitSettings"), "thresholds");
    }

    #[test]
    fn test_classify_struct_domain_detection() {
        assert_eq!(classify_struct_domain("DetectorModule"), "detection");
        assert_eq!(classify_struct_domain("CheckerSystem"), "detection");
    }

    #[test]
    fn test_classify_struct_domain_config() {
        assert_eq!(classify_struct_domain("ConfigOptions"), "core_config");
        assert_eq!(classify_struct_domain("SystemSettings"), "core_config");
    }

    #[test]
    fn test_classify_struct_domain_data() {
        assert_eq!(classify_struct_domain("DataStructure"), "data");
        assert_eq!(classify_struct_domain("MetricsInfo"), "data");
    }

    #[test]
    fn test_classify_struct_domain_fallback() {
        // Should extract first word
        assert_eq!(classify_struct_domain("UserProfile"), "User");
        assert_eq!(classify_struct_domain("OrderProcessor"), "Order");
    }

    #[test]
    fn test_extract_domain_from_name_camel_case() {
        assert_eq!(extract_domain_from_name("UserProfile"), "User");
        assert_eq!(extract_domain_from_name("OrderManager"), "Order");
    }

    #[test]
    fn test_extract_domain_from_name_snake_case() {
        assert_eq!(extract_domain_from_name("user_profile"), "user");
        assert_eq!(extract_domain_from_name("order_data"), "order");
    }

    #[test]
    fn test_extract_domain_from_name_empty() {
        assert_eq!(extract_domain_from_name(""), "Core");
    }

    #[test]
    fn test_count_distinct_domains() {
        use super::super::types::StructMetrics;
        let structs = vec![
            StructMetrics {
                name: "ThresholdConfig".to_string(),
                line_span: (0, 10),
                method_count: 2,
                field_count: 5,
                responsibilities: vec!["configuration".to_string()],
            },
            StructMetrics {
                name: "ThresholdValidator".to_string(),
                line_span: (11, 20),
                method_count: 3,
                field_count: 4,
                responsibilities: vec!["validation".to_string()],
            },
            StructMetrics {
                name: "ScoringWeight".to_string(),
                line_span: (21, 30),
                method_count: 4,
                field_count: 3,
                responsibilities: vec!["calculation".to_string()],
            },
        ];
        // Should identify 2 domains: "thresholds" and "scoring"
        assert_eq!(count_distinct_domains(&structs), 2);
    }

    #[test]
    fn test_calculate_struct_ratio_normal() {
        assert_eq!(calculate_struct_ratio(10, 20), 0.5);
        assert_eq!(calculate_struct_ratio(15, 10), 1.5);
    }

    #[test]
    fn test_calculate_struct_ratio_zero_functions() {
        assert_eq!(calculate_struct_ratio(10, 0), 0.0);
    }

    #[test]
    fn test_calculate_struct_ratio_zero_structs() {
        assert_eq!(calculate_struct_ratio(0, 10), 0.0);
    }

    // Property-based tests using proptest
    use proptest::prelude::*;

    proptest! {
        /// Verify classification is idempotent - same input always produces same output
        #[test]
        fn prop_classification_idempotent(method_name in "[a-z_]{1,20}") {
            let r1 = infer_responsibility_with_confidence(&method_name, None);
            let r2 = infer_responsibility_with_confidence(&method_name, None);
            prop_assert_eq!(r1.category, r2.category);
            prop_assert_eq!(r1.confidence, r2.confidence);
            prop_assert_eq!(r1.signals_used, r2.signals_used);
        }

        /// Verify struct domain classification is idempotent
        #[test]
        fn prop_struct_domain_classification_idempotent(struct_name in "[A-Z][a-zA-Z0-9_]{1,30}") {
            let d1 = classify_struct_domain(&struct_name);
            let d2 = classify_struct_domain(&struct_name);
            prop_assert_eq!(d1, d2);
        }

        /// Verify confidence calculation is idempotent
        #[test]
        fn prop_confidence_calculation_idempotent(
            method_count in 0usize..100,
            field_count in 0usize..50,
            responsibility_count in 0usize..20,
            lines_of_code in 0usize..5000,
            complexity_sum in 0u32..1000
        ) {
            let thresholds = GodObjectThresholds::default();
            let c1 = determine_confidence(method_count, field_count, responsibility_count, lines_of_code, complexity_sum, &thresholds);
            let c2 = determine_confidence(method_count, field_count, responsibility_count, lines_of_code, complexity_sum, &thresholds);
            prop_assert_eq!(c1, c2);
        }

        /// Verify confidence levels map correctly based on violation count
        #[test]
        fn prop_confidence_violations_mapping(
            method_count in 0usize..100,
            field_count in 0usize..50,
            responsibility_count in 0usize..20,
            lines_of_code in 0usize..5000,
            complexity_sum in 0u32..1000
        ) {
            let thresholds = GodObjectThresholds::default();
            let confidence = determine_confidence(method_count, field_count, responsibility_count, lines_of_code, complexity_sum, &thresholds);

            // Count violations
            let mut violations = 0;
            if method_count > thresholds.max_methods { violations += 1; }
            if field_count > thresholds.max_fields { violations += 1; }
            if responsibility_count > thresholds.max_traits { violations += 1; }
            if lines_of_code > thresholds.max_lines { violations += 1; }
            if complexity_sum > thresholds.max_complexity { violations += 1; }

            // Verify mapping matches spec
            match violations {
                5 => prop_assert_eq!(confidence, GodObjectConfidence::Definite),
                3..=4 => prop_assert_eq!(confidence, GodObjectConfidence::Probable),
                1..=2 => prop_assert_eq!(confidence, GodObjectConfidence::Possible),
                _ => prop_assert_eq!(confidence, GodObjectConfidence::NotGodObject),
            }
        }

        /// Verify struct ratio calculation is always non-negative
        #[test]
        fn prop_struct_ratio_non_negative(struct_count in 0usize..100, total_functions in 0usize..200) {
            let ratio = calculate_struct_ratio(struct_count, total_functions);
            prop_assert!(ratio >= 0.0);
        }

        /// Verify struct ratio calculation handles zero functions gracefully
        #[test]
        fn prop_struct_ratio_zero_functions(struct_count in 0usize..100) {
            let ratio = calculate_struct_ratio(struct_count, 0);
            prop_assert_eq!(ratio, 0.0);
        }
    }

    // Spec 254: Universal Responsibility Analysis Tests
    #[test]
    fn test_analyze_function_responsibility_validation() {
        assert_eq!(
            analyze_function_responsibility("validate_email"),
            Some("Validation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("check_bounds"),
            Some("Validation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("verify_signature"),
            Some("Validation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("is_valid"),
            Some("Validation".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_parsing() {
        assert_eq!(
            analyze_function_responsibility("parse_json"),
            Some("Parsing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("read_config"),
            Some("Parsing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("extract_data"),
            Some("Parsing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("decode_message"),
            Some("Parsing".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_data_access() {
        assert_eq!(
            analyze_function_responsibility("get_user"),
            Some("Data Access".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("set_property"),
            Some("Data Access".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("fetch_record"),
            Some("Data Access".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("retrieve_data"),
            Some("Data Access".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_rendering() {
        assert_eq!(
            analyze_function_responsibility("render_view"),
            Some("Rendering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("draw_chart"),
            Some("Rendering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("paint_canvas"),
            Some("Rendering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("format_output"),
            Some("Rendering".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_construction() {
        assert_eq!(
            analyze_function_responsibility("create_instance"),
            Some("Construction".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("build_object"),
            Some("Construction".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("make_widget"),
            Some("Construction".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_filtering() {
        assert_eq!(
            analyze_function_responsibility("filter_results"),
            Some("Filtering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("select_items"),
            Some("Filtering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("find_matches"),
            Some("Filtering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("search_database"),
            Some("Filtering".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_transformation() {
        assert_eq!(
            analyze_function_responsibility("transform_data"),
            Some("Transformation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("convert_to_json"),
            Some("Transformation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("map_values"),
            Some("Transformation".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_communication() {
        assert_eq!(
            analyze_function_responsibility("send_message"),
            Some("Communication".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("receive_data"),
            Some("Communication".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("transmit_packet"),
            Some("Communication".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("notify_observers"),
            Some("Communication".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_persistence() {
        assert_eq!(
            analyze_function_responsibility("save_state"),
            Some("Persistence".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("load_config"),
            Some("Persistence".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_event_handling() {
        assert_eq!(
            analyze_function_responsibility("handle_keypress"),
            Some("Event Handling".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("on_mouse_down"),
            Some("Event Handling".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("dispatch_event"),
            Some("Event Handling".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_processing() {
        assert_eq!(
            analyze_function_responsibility("process_request"),
            Some("Processing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("execute_task"),
            Some("Processing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("run_pipeline"),
            Some("Processing".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_low_confidence() {
        // Generic names should return None due to low confidence
        // Note: "process" and "handle" are recognized patterns (Processing/Event Handling)
        // so we test truly generic names
        assert_eq!(analyze_function_responsibility("do_something"), None);
        assert_eq!(analyze_function_responsibility("helper"), None);
        assert_eq!(analyze_function_responsibility("utils"), None);
        assert_eq!(analyze_function_responsibility("foo"), None);
        assert_eq!(analyze_function_responsibility("bar"), None);
    }

    #[test]
    fn test_analyze_function_responsibility_purity() {
        // Pure function: same input = same output
        let result1 = analyze_function_responsibility("validate_input");
        let result2 = analyze_function_responsibility("validate_input");
        assert_eq!(result1, result2);
        assert_eq!(result1, Some("Validation".to_string()));

        // Test multiple times to ensure determinism
        for _ in 0..10 {
            assert_eq!(
                analyze_function_responsibility("parse_json"),
                Some("Parsing".to_string())
            );
        }
    }

    #[test]
    fn test_analyze_function_responsibility_empty_string() {
        // Edge case: empty string
        assert_eq!(analyze_function_responsibility(""), None);
    }

    #[test]
    fn test_analyze_function_responsibility_lifecycle() {
        assert_eq!(
            analyze_function_responsibility("initialize_system"),
            Some("Lifecycle".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("cleanup"),
            Some("Lifecycle".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_state_management() {
        assert_eq!(
            analyze_function_responsibility("update_state"),
            Some("State Management".to_string())
        );
    }
}
