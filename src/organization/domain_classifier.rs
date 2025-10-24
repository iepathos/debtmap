/// Enhanced domain classification for struct-based grouping.
///
/// This module provides sophisticated pattern matching to classify structs into semantic
/// domains (scoring, thresholds, detection, output, etc.). This enables intelligent
/// module split recommendations that align with actual domain boundaries.
/// Classify a struct into a semantic domain based on naming patterns and method signatures.
///
/// Uses pattern matching on struct names to infer domain responsibility.
/// Patterns are ordered from most specific to most general.
///
/// # Arguments
///
/// * `struct_name` - The name of the struct to classify
/// * `methods` - Method names belonging to the struct (used as secondary signal)
///
/// # Returns
///
/// A domain string such as "scoring", "thresholds", "detection", etc.
pub fn classify_struct_domain_enhanced(struct_name: &str, methods: &[String]) -> String {
    let lower = struct_name.to_lowercase();

    // Specific patterns first (most to least specific)
    // Note: Order matters! More specific patterns should come before more general ones
    if matches_scoring_pattern(&lower) {
        return "scoring".to_string();
    }
    if matches_threshold_pattern(&lower) {
        return "thresholds".to_string();
    }
    if matches_detection_pattern(&lower) {
        return "detection".to_string();
    }
    if matches_output_pattern(&lower) {
        return "output".to_string();
    }
    if matches_language_pattern(&lower) {
        return "languages".to_string();
    }
    // Context pattern should come before error pattern to avoid false matches
    // (e.g., "PatternRule" should match "context" not "error_handling")
    if matches_context_pattern(&lower) {
        return "context".to_string();
    }
    if matches_error_pattern(&lower) {
        return "error_handling".to_string();
    }
    if matches_io_pattern(&lower) {
        return "io".to_string();
    }
    if matches_config_pattern(&lower) {
        return "config".to_string();
    }

    // Analyze method names as secondary signal
    let method_pattern = infer_domain_from_methods(methods);
    if !method_pattern.is_empty() {
        return method_pattern;
    }

    // Default to utilities
    "utilities".to_string()
}

/// Check if struct name matches scoring/weight domain patterns.
fn matches_scoring_pattern(name: &str) -> bool {
    name.contains("scoring")
        || name.contains("weight")
        || name.contains("multiplier")
        || name.contains("factor")
}

/// Check if struct name matches threshold/limit domain patterns.
fn matches_threshold_pattern(name: &str) -> bool {
    name.contains("threshold") || name.contains("limit") || name.contains("bound")
}

/// Check if struct name matches detection/analysis domain patterns.
fn matches_detection_pattern(name: &str) -> bool {
    name.contains("detection")
        || name.contains("detector")
        || name.contains("checker")
        || name.contains("analyzer")
}

/// Check if struct name matches output/formatting domain patterns.
fn matches_output_pattern(name: &str) -> bool {
    name.contains("display")
        || name.contains("output")
        || name.contains("format")
        || name.contains("render")
        || name.contains("print")
}

/// Check if struct name matches language-specific domain patterns.
fn matches_language_pattern(name: &str) -> bool {
    name.contains("language")
        || name.contains("rust")
        || name.contains("python")
        || name.contains("javascript")
        || name.contains("typescript")
}

/// Check if struct name matches error handling domain patterns.
fn matches_error_pattern(name: &str) -> bool {
    name.contains("error") || name.contains("severity")
}

/// Check if struct name matches context/rule domain patterns.
fn matches_context_pattern(name: &str) -> bool {
    name.contains("context") || name.contains("rule") || name.contains("matcher")
}

/// Check if struct name matches I/O domain patterns.
fn matches_io_pattern(name: &str) -> bool {
    name.contains("reader")
        || name.contains("writer")
        || name.contains("loader")
        || name.contains("saver")
}

/// Check if struct name matches config domain patterns.
fn matches_config_pattern(name: &str) -> bool {
    name.contains("config") || name.contains("settings") || name.contains("options")
}

/// Infer domain from method names when struct name is ambiguous.
///
/// This provides a secondary signal for classification when the struct name
/// doesn't clearly indicate its domain.
fn infer_domain_from_methods(methods: &[String]) -> String {
    if methods.is_empty() {
        return String::new();
    }

    let method_names: Vec<String> = methods.iter().map(|m| m.to_lowercase()).collect();

    // Count domain-specific method patterns
    let mut domain_counts: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();

    for method in &method_names {
        if method.contains("detect") || method.contains("check") || method.contains("analyze") {
            *domain_counts.entry("detection").or_insert(0) += 1;
        }
        if method.contains("score") || method.contains("weight") || method.contains("calculate") {
            *domain_counts.entry("scoring").or_insert(0) += 1;
        }
        if method.contains("validate") || method.contains("threshold") || method.contains("limit") {
            *domain_counts.entry("thresholds").or_insert(0) += 1;
        }
        if method.contains("format") || method.contains("display") || method.contains("print") {
            *domain_counts.entry("output").or_insert(0) += 1;
        }
        if method.contains("read")
            || method.contains("write")
            || method.contains("load")
            || method.contains("save")
        {
            *domain_counts.entry("io").or_insert(0) += 1;
        }
    }

    // Return most common domain, if any pattern is strong enough (>30% of methods)
    if let Some((domain, count)) = domain_counts.iter().max_by_key(|(_, &count)| count) {
        if *count as f64 / methods.len() as f64 > 0.3 {
            return domain.to_string();
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_scoring_structs() {
        assert_eq!(
            classify_struct_domain_enhanced("ScoringWeights", &[]),
            "scoring"
        );
        assert_eq!(
            classify_struct_domain_enhanced("RoleMultipliers", &[]),
            "scoring"
        );
        assert_eq!(
            classify_struct_domain_enhanced("ComplexityFactor", &[]),
            "scoring"
        );
        assert_eq!(
            classify_struct_domain_enhanced("WeightConfig", &[]),
            "scoring"
        );
    }

    #[test]
    fn test_classify_threshold_structs() {
        assert_eq!(
            classify_struct_domain_enhanced("ThresholdsConfig", &[]),
            "thresholds"
        );
        assert_eq!(
            classify_struct_domain_enhanced("ValidationLimits", &[]),
            "thresholds"
        );
        assert_eq!(
            classify_struct_domain_enhanced("MaxBounds", &[]),
            "thresholds"
        );
        assert_eq!(
            classify_struct_domain_enhanced("SizeThreshold", &[]),
            "thresholds"
        );
    }

    #[test]
    fn test_classify_detection_structs() {
        assert_eq!(
            classify_struct_domain_enhanced("PatternDetector", &[]),
            "detection"
        );
        assert_eq!(
            classify_struct_domain_enhanced("GodObjectChecker", &[]),
            "detection"
        );
        assert_eq!(
            classify_struct_domain_enhanced("ComplexityAnalyzer", &[]),
            "detection"
        );
        assert_eq!(
            classify_struct_domain_enhanced("DebtDetector", &[]),
            "detection"
        );
    }

    #[test]
    fn test_classify_output_structs() {
        assert_eq!(
            classify_struct_domain_enhanced("OutputFormatter", &[]),
            "output"
        );
        assert_eq!(
            classify_struct_domain_enhanced("DisplayConfig", &[]),
            "output"
        );
        assert_eq!(
            classify_struct_domain_enhanced("RenderOptions", &[]),
            "output"
        );
        assert_eq!(
            classify_struct_domain_enhanced("PrintSettings", &[]),
            "output"
        );
    }

    #[test]
    fn test_classify_language_structs() {
        assert_eq!(
            classify_struct_domain_enhanced("RustConfig", &[]),
            "languages"
        );
        assert_eq!(
            classify_struct_domain_enhanced("PythonSettings", &[]),
            "languages"
        );
        assert_eq!(
            classify_struct_domain_enhanced("LanguageSupport", &[]),
            "languages"
        );
    }

    #[test]
    fn test_classify_error_structs() {
        assert_eq!(
            classify_struct_domain_enhanced("ErrorConfig", &[]),
            "error_handling"
        );
        assert_eq!(
            classify_struct_domain_enhanced("SeverityLevel", &[]),
            "error_handling"
        );
        assert_eq!(
            classify_struct_domain_enhanced("ErrorHandler", &[]),
            "error_handling"
        );
    }

    #[test]
    fn test_classify_context_structs() {
        assert_eq!(
            classify_struct_domain_enhanced("AnalysisContext", &[]),
            "context"
        );
        assert_eq!(
            classify_struct_domain_enhanced("RuleMatcher", &[]),
            "context"
        );
        assert_eq!(
            classify_struct_domain_enhanced("PatternRule", &[]),
            "context"
        );
    }

    #[test]
    fn test_classify_io_structs() {
        assert_eq!(classify_struct_domain_enhanced("FileReader", &[]), "io");
        assert_eq!(classify_struct_domain_enhanced("DataWriter", &[]), "io");
        assert_eq!(classify_struct_domain_enhanced("ConfigLoader", &[]), "io");
        assert_eq!(classify_struct_domain_enhanced("StateSaver", &[]), "io");
    }

    #[test]
    fn test_classify_config_structs() {
        assert_eq!(classify_struct_domain_enhanced("AppConfig", &[]), "config");
        assert_eq!(
            classify_struct_domain_enhanced("GlobalSettings", &[]),
            "config"
        );
        assert_eq!(
            classify_struct_domain_enhanced("UserOptions", &[]),
            "config"
        );
    }

    #[test]
    fn test_classify_unknown_struct() {
        // Should classify unknown structs as utilities
        assert_eq!(
            classify_struct_domain_enhanced("MyCustomStruct", &[]),
            "utilities"
        );
        assert_eq!(classify_struct_domain_enhanced("Helper", &[]), "utilities");
    }

    #[test]
    fn test_ambiguous_struct_uses_methods() {
        // When struct name is ambiguous, use method names as signal
        let methods = vec![
            "detect_pattern".to_string(),
            "check_violation".to_string(),
            "analyze_code".to_string(),
        ];
        assert_eq!(
            classify_struct_domain_enhanced("Analyzer", &methods),
            "detection"
        );

        let scoring_methods = vec![
            "calculate_score".to_string(),
            "apply_weight".to_string(),
            "compute_total".to_string(),
        ];
        assert_eq!(
            classify_struct_domain_enhanced("Processor", &scoring_methods),
            "scoring"
        );
    }

    #[test]
    fn test_method_domain_inference_weak_signal() {
        // If method pattern is weak (<30%), should return empty
        let mixed_methods = vec![
            "detect_issue".to_string(),
            "format_output".to_string(),
            "validate_input".to_string(),
            "load_data".to_string(),
            "helper_fn".to_string(),
        ];
        assert_eq!(
            classify_struct_domain_enhanced("GenericUtil", &mixed_methods),
            "utilities"
        );
    }

    #[test]
    fn test_empty_methods() {
        assert_eq!(
            classify_struct_domain_enhanced("UnknownStruct", &[]),
            "utilities"
        );
    }
}
