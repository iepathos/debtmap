//! # God Object Detection Predicates (Pure Functions)
//!
//! Pure boolean functions for god object detection.
//!
//! All predicates are:
//! - Pure: No side effects, deterministic
//! - Composable: Can be combined with && and ||
//! - Testable: No mocks needed

use super::thresholds::GodObjectThresholds;
use super::types::GodObjectConfidence;

/// Check if method count exceeds threshold.
pub fn exceeds_method_threshold(count: usize, threshold: usize) -> bool {
    count > threshold
}

/// Check if field count exceeds threshold.
pub fn exceeds_field_threshold(count: usize, threshold: usize) -> bool {
    count > threshold
}

/// Check if responsibility count exceeds threshold.
pub fn exceeds_responsibility_threshold(count: usize, threshold: usize) -> bool {
    count > threshold
}

/// Check if lines of code exceeds threshold.
pub fn exceeds_lines_threshold(count: usize, threshold: usize) -> bool {
    count > threshold
}

/// Check if complexity sum exceeds threshold.
pub fn exceeds_complexity_threshold(complexity: u32, threshold: u32) -> bool {
    complexity > threshold
}

/// Check if trait implementation count exceeds threshold.
pub fn exceeds_trait_threshold(count: usize, threshold: usize) -> bool {
    count > threshold
}

/// Check if counts indicate a god object.
pub fn is_god_object(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    thresholds: &GodObjectThresholds,
) -> bool {
    method_count > thresholds.max_methods
        || field_count > thresholds.max_fields
        || responsibility_count > thresholds.max_traits
}

/// Check if a type is a god object based on comprehensive metrics.
pub fn is_god_object_comprehensive(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    trait_count: usize,
    thresholds: &GodObjectThresholds,
) -> bool {
    method_count > thresholds.max_methods
        || field_count > thresholds.max_fields
        || responsibility_count > thresholds.max_traits
        || trait_count > 10
}

/// Check if struct count and domain count indicate cross-domain mixing.
pub fn is_hybrid_god_module(struct_count: usize, domain_count: usize) -> bool {
    // A hybrid god module has many structs (>15) across multiple domains (>3)
    // Ratio check: struct_count > domain_count * 3 means each domain has >3 structs
    struct_count > 15 && domain_count > 3 && struct_count > domain_count * 3
}

/// Check if a module split should be recommended based on method count.
pub fn should_recommend_split(method_count: usize, min_methods: usize) -> bool {
    method_count > min_methods
}

/// Check if method name suggests it's a pure method (no side effects).
pub fn is_pure_method_name(method_name: &str) -> bool {
    let pure_prefixes = [
        "get_",
        "is_",
        "has_",
        "can_",
        "should_",
        "calculate_",
        "compute_",
    ];
    pure_prefixes
        .iter()
        .any(|prefix| method_name.starts_with(prefix))
}

/// Check if method name suggests it performs I/O operations.
pub fn has_io_indicators(method_name: &str) -> bool {
    let io_keywords = [
        "read", "write", "print", "open", "close", "fetch", "load", "save",
    ];
    io_keywords
        .iter()
        .any(|keyword| method_name.contains(keyword))
}

/// Check if confidence level indicates definite god object.
pub fn is_definite_god_object(confidence: &GodObjectConfidence) -> bool {
    matches!(confidence, GodObjectConfidence::Definite)
}

/// Check if confidence level indicates probable god object.
pub fn is_probable_god_object(confidence: &GodObjectConfidence) -> bool {
    matches!(
        confidence,
        GodObjectConfidence::Probable | GodObjectConfidence::Definite
    )
}

/// Check if confidence level indicates possible god object.
pub fn is_possible_god_object(confidence: &GodObjectConfidence) -> bool {
    !matches!(confidence, GodObjectConfidence::NotGodObject)
}

/// Check if struct ratio indicates struct-heavy file.
pub fn is_struct_heavy(struct_ratio: f64, threshold: f64) -> bool {
    struct_ratio > threshold
}

/// Check if domain diversity indicates cross-domain mixing.
pub fn has_cross_domain_mixing(domain_count: usize, min_domains: usize) -> bool {
    domain_count >= min_domains
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exceeds_method_threshold() {
        assert!(exceeds_method_threshold(20, 15));
        assert!(!exceeds_method_threshold(10, 15));
        assert!(!exceeds_method_threshold(15, 15)); // boundary: equal is not exceeding
    }

    #[test]
    fn test_exceeds_field_threshold() {
        assert!(exceeds_field_threshold(20, 15));
        assert!(!exceeds_field_threshold(10, 15));
        assert!(!exceeds_field_threshold(15, 15));
    }

    #[test]
    fn test_exceeds_responsibility_threshold() {
        assert!(exceeds_responsibility_threshold(8, 5));
        assert!(!exceeds_responsibility_threshold(3, 5));
        assert!(!exceeds_responsibility_threshold(5, 5));
    }

    #[test]
    fn test_exceeds_lines_threshold() {
        assert!(exceeds_lines_threshold(1500, 1000));
        assert!(!exceeds_lines_threshold(500, 1000));
        assert!(!exceeds_lines_threshold(1000, 1000));
    }

    #[test]
    fn test_exceeds_complexity_threshold() {
        assert!(exceeds_complexity_threshold(250, 200));
        assert!(!exceeds_complexity_threshold(150, 200));
        assert!(!exceeds_complexity_threshold(200, 200));
    }

    #[test]
    fn test_exceeds_trait_threshold() {
        assert!(exceeds_trait_threshold(12, 10));
        assert!(!exceeds_trait_threshold(8, 10));
        assert!(!exceeds_trait_threshold(10, 10));
    }

    #[test]
    fn test_is_god_object_methods_exceeded() {
        let thresholds = GodObjectThresholds::default();
        assert!(is_god_object(25, 10, 3, &thresholds)); // methods > 20
        assert!(!is_god_object(15, 10, 3, &thresholds)); // all within limits
    }

    #[test]
    fn test_is_god_object_fields_exceeded() {
        let thresholds = GodObjectThresholds::default();
        assert!(is_god_object(15, 20, 3, &thresholds)); // fields > 15
    }

    #[test]
    fn test_is_god_object_responsibilities_exceeded() {
        let thresholds = GodObjectThresholds::default();
        assert!(is_god_object(15, 10, 8, &thresholds)); // responsibilities > 5
    }

    #[test]
    fn test_is_god_object_boundaries() {
        let thresholds = GodObjectThresholds::default();
        // Exactly at thresholds should NOT be god object
        assert!(!is_god_object(20, 15, 5, &thresholds));
        // One over any threshold should trigger
        assert!(is_god_object(21, 15, 5, &thresholds));
        assert!(is_god_object(20, 16, 5, &thresholds));
        assert!(is_god_object(20, 15, 6, &thresholds));
    }

    #[test]
    fn test_is_god_object_comprehensive_trait_threshold() {
        let thresholds = GodObjectThresholds::default();
        // Exceeds trait implementation threshold (>10)
        assert!(is_god_object_comprehensive(15, 10, 3, 12, &thresholds));
        // Within all limits
        assert!(!is_god_object_comprehensive(15, 10, 3, 8, &thresholds));
    }

    #[test]
    fn test_is_hybrid_god_module() {
        // 60 structs, 15 domains: 60 > 15*3 (45), so true
        assert!(is_hybrid_god_module(60, 15));
        // 60 structs, 25 domains: 60 < 25*3 (75), so false
        assert!(!is_hybrid_god_module(60, 25));
        // Below struct count threshold
        assert!(!is_hybrid_god_module(10, 3));
        // Below domain count threshold
        assert!(!is_hybrid_god_module(20, 2));
    }

    #[test]
    fn test_is_hybrid_god_module_boundaries() {
        // Exactly 15 structs, 3 domains
        assert!(!is_hybrid_god_module(15, 3)); // not > 15
                                               // 16 structs, 3 domains: 16 > 3*3 (9), and 16 > 15, and 3 domains
        assert!(!is_hybrid_god_module(16, 3)); // domains not > 3
                                               // 16 structs, 4 domains: 16 > 4*3 (12), and 16 > 15, and 4 > 3
        assert!(is_hybrid_god_module(16, 4));
    }

    #[test]
    fn test_should_recommend_split() {
        assert!(should_recommend_split(10, 5));
        assert!(!should_recommend_split(5, 5)); // boundary
        assert!(!should_recommend_split(3, 5));
    }

    #[test]
    fn test_is_pure_method_name() {
        assert!(is_pure_method_name("get_value"));
        assert!(is_pure_method_name("is_valid"));
        assert!(is_pure_method_name("has_permission"));
        assert!(is_pure_method_name("can_execute"));
        assert!(is_pure_method_name("should_retry"));
        assert!(is_pure_method_name("calculate_sum"));
        assert!(is_pure_method_name("compute_hash"));
        assert!(!is_pure_method_name("set_value"));
        assert!(!is_pure_method_name("update_state"));
    }

    #[test]
    fn test_has_io_indicators() {
        assert!(has_io_indicators("read_file"));
        assert!(has_io_indicators("write_data"));
        assert!(has_io_indicators("print_output"));
        assert!(has_io_indicators("open_connection"));
        assert!(has_io_indicators("close_stream"));
        assert!(has_io_indicators("fetch_url"));
        assert!(has_io_indicators("load_config"));
        assert!(has_io_indicators("save_state"));
        assert!(!has_io_indicators("calculate_sum"));
        assert!(!has_io_indicators("validate_input"));
    }

    #[test]
    fn test_is_definite_god_object() {
        assert!(is_definite_god_object(&GodObjectConfidence::Definite));
        assert!(!is_definite_god_object(&GodObjectConfidence::Probable));
        assert!(!is_definite_god_object(&GodObjectConfidence::Possible));
        assert!(!is_definite_god_object(&GodObjectConfidence::NotGodObject));
    }

    #[test]
    fn test_is_probable_god_object() {
        assert!(is_probable_god_object(&GodObjectConfidence::Definite));
        assert!(is_probable_god_object(&GodObjectConfidence::Probable));
        assert!(!is_probable_god_object(&GodObjectConfidence::Possible));
        assert!(!is_probable_god_object(&GodObjectConfidence::NotGodObject));
    }

    #[test]
    fn test_is_possible_god_object() {
        assert!(is_possible_god_object(&GodObjectConfidence::Definite));
        assert!(is_possible_god_object(&GodObjectConfidence::Probable));
        assert!(is_possible_god_object(&GodObjectConfidence::Possible));
        assert!(!is_possible_god_object(&GodObjectConfidence::NotGodObject));
    }

    #[test]
    fn test_is_struct_heavy() {
        assert!(is_struct_heavy(0.5, 0.3));
        assert!(!is_struct_heavy(0.2, 0.3));
        assert!(!is_struct_heavy(0.3, 0.3)); // boundary
    }

    #[test]
    fn test_has_cross_domain_mixing() {
        assert!(has_cross_domain_mixing(5, 3));
        assert!(has_cross_domain_mixing(3, 3)); // boundary: >= includes equal
        assert!(!has_cross_domain_mixing(2, 3));
    }

    // Predicate composition tests
    #[test]
    fn test_predicate_composition_and() {
        let thresholds = GodObjectThresholds::default();
        // Both conditions true
        assert!(
            exceeds_method_threshold(25, thresholds.max_methods)
                && exceeds_field_threshold(20, thresholds.max_fields)
        );
        // One condition false
        assert!(
            !(exceeds_method_threshold(25, thresholds.max_methods)
                && exceeds_field_threshold(10, thresholds.max_fields))
        );
    }

    #[test]
    fn test_predicate_composition_or() {
        let thresholds = GodObjectThresholds::default();
        // At least one condition true
        assert!(
            exceeds_method_threshold(25, thresholds.max_methods)
                || exceeds_field_threshold(10, thresholds.max_fields)
        );
        // Both conditions false
        assert!(
            !(exceeds_method_threshold(10, thresholds.max_methods)
                || exceeds_field_threshold(10, thresholds.max_fields))
        );
    }

    #[test]
    fn test_predicate_composition_complex() {
        let thresholds = GodObjectThresholds::default();
        // Complex condition: (methods OR fields) AND responsibilities
        let is_complex_god_object = (exceeds_method_threshold(25, thresholds.max_methods)
            || exceeds_field_threshold(20, thresholds.max_fields))
            && exceeds_responsibility_threshold(8, thresholds.max_traits);
        assert!(is_complex_god_object);
    }

    // Determinism tests
    #[test]
    fn test_predicates_are_deterministic() {
        // Same inputs should always produce same outputs
        assert_eq!(
            exceeds_method_threshold(20, 15),
            exceeds_method_threshold(20, 15)
        );
        assert_eq!(
            is_pure_method_name("get_value"),
            is_pure_method_name("get_value")
        );
        assert_eq!(
            has_io_indicators("read_file"),
            has_io_indicators("read_file")
        );
    }
}
