//! Path classification for purity analysis
//!
//! Contains pure functions for classifying paths as constants, pure calls, etc.

use super::constants::{
    KNOWN_CONSTANT_PREFIXES, KNOWN_CONSTANT_SUFFIXES, KNOWN_PURE_STD_FUNCTIONS, PURE_METHOD_NAMES,
};
use super::types::PathPurity;

/// Check if a method call is a known pure standard library function (Spec 261)
pub fn is_known_pure_call(method_name: &str, receiver_type: Option<&str>) -> bool {
    let full_name = match receiver_type {
        Some(ty) => format!("{}::{}", ty, method_name),
        None => method_name.to_string(),
    };

    KNOWN_PURE_STD_FUNCTIONS
        .iter()
        .any(|pure_fn| full_name.ends_with(pure_fn) || pure_fn.ends_with(&full_name))
}

/// Check if a method name alone matches a known pure method (Spec 261)
pub fn is_known_pure_method(method_name: &str) -> bool {
    PURE_METHOD_NAMES.contains(&method_name)
}

/// Check if a path string represents a known constant
pub fn is_known_constant(path_str: &str) -> bool {
    // Check prefixes (std :: i32 ::, core :: u64 ::, etc.)
    for prefix in KNOWN_CONSTANT_PREFIXES {
        if path_str.starts_with(prefix) {
            return true;
        }
    }

    // Check suffixes (:: MAX, :: MIN, etc.)
    for suffix in KNOWN_CONSTANT_SUFFIXES {
        if path_str.ends_with(suffix) {
            return true;
        }
    }

    false
}

/// Check if a string is in SCREAMING_CASE (likely a constant)
pub fn is_screaming_case(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_uppercase() || c == '_' || c.is_numeric())
        && s.chars().any(|c| c.is_alphabetic())
}

/// Check if a string is in PascalCase (likely an enum variant)
pub fn is_pascal_case(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_uppercase())
        && !s.chars().all(|c| c.is_uppercase() || c == '_')
        && s.chars().any(|c| c.is_lowercase())
}

/// Classify a path for purity analysis
pub fn classify_path_purity(path_str: &str) -> PathPurity {
    // 1. Check known constants
    if is_known_constant(path_str) {
        return PathPurity::Constant;
    }

    // 2. Check for SCREAMING_CASE (likely constant)
    // Extract last segment after ::
    let last_segment = path_str.rsplit("::").next().unwrap_or(path_str).trim();

    if is_screaming_case(last_segment) {
        return PathPurity::ProbablyConstant;
    }

    // 3. Check for enum variants (PascalCase after ::)
    // Common patterns: Option::None, Result::Ok, MyEnum::Variant
    if is_pascal_case(last_segment) {
        // Additional check: if it looks like an enum variant pattern
        // (path contains :: and ends with PascalCase identifier)
        let segments: Vec<&str> = path_str.split("::").map(|s| s.trim()).collect();
        if segments.len() >= 2 {
            // Last segment is PascalCase - likely an enum variant
            return PathPurity::ProbablyConstant;
        }
    }

    // 4. Default: unknown, conservative
    PathPurity::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_known_pure_call_option_map() {
        assert!(is_known_pure_call("map", Some("Option")));
        assert!(is_known_pure_call("and_then", Some("Option")));
        assert!(is_known_pure_call("unwrap_or", Some("Option")));
    }

    #[test]
    fn test_is_known_pure_call_result_methods() {
        assert!(is_known_pure_call("map", Some("Result")));
        assert!(is_known_pure_call("map_err", Some("Result")));
        assert!(is_known_pure_call("and_then", Some("Result")));
        assert!(is_known_pure_call("is_ok", Some("Result")));
    }

    #[test]
    fn test_is_known_pure_call_iterator_methods() {
        assert!(is_known_pure_call("map", Some("Iterator")));
        assert!(is_known_pure_call("filter", Some("Iterator")));
        assert!(is_known_pure_call("fold", Some("Iterator")));
        assert!(is_known_pure_call("collect", Some("Iterator")));
        assert!(is_known_pure_call("sum", Some("Iterator")));
    }

    #[test]
    fn test_is_known_pure_method_without_receiver() {
        assert!(is_known_pure_method("map"));
        assert!(is_known_pure_method("filter"));
        assert!(is_known_pure_method("collect"));
        assert!(is_known_pure_method("len"));
        assert!(is_known_pure_method("is_empty"));
        assert!(is_known_pure_method("clone"));
    }

    #[test]
    fn test_is_known_pure_method_unknown() {
        // These should NOT be considered known pure
        assert!(!is_known_pure_method("println"));
        assert!(!is_known_pure_method("write"));
        assert!(!is_known_pure_method("push")); // Mutation method
        assert!(!is_known_pure_method("insert")); // Mutation method
    }

    #[test]
    fn test_is_screaming_case() {
        assert!(is_screaming_case("MAX"));
        assert!(is_screaming_case("MIN_VALUE"));
        assert!(is_screaming_case("MAX_SIZE"));
        assert!(!is_screaming_case("maxValue"));
        assert!(!is_screaming_case("MaxValue"));
        assert!(!is_screaming_case(""));
    }

    #[test]
    fn test_is_pascal_case() {
        assert!(is_pascal_case("Option"));
        assert!(is_pascal_case("MyEnum"));
        assert!(is_pascal_case("SomeValue"));
        assert!(!is_pascal_case("MAX"));
        assert!(!is_pascal_case("max_value"));
        assert!(!is_pascal_case(""));
    }

    #[test]
    fn test_classify_path_purity_constant() {
        assert_eq!(
            classify_path_purity("std :: i32 :: MAX"),
            PathPurity::Constant
        );
        assert_eq!(
            classify_path_purity("core :: u64 :: MIN"),
            PathPurity::Constant
        );
    }

    #[test]
    fn test_classify_path_purity_probably_constant() {
        assert_eq!(
            classify_path_purity("config :: MAX_SIZE"),
            PathPurity::ProbablyConstant
        );
        assert_eq!(
            classify_path_purity("Option :: None"),
            PathPurity::ProbablyConstant
        );
    }

    #[test]
    fn test_classify_path_purity_unknown() {
        assert_eq!(
            classify_path_purity("external :: get_value"),
            PathPurity::Unknown
        );
    }
}
