// ============================================================================
// PURE CORE: Function name matching logic (100% testable, no I/O)
// ============================================================================

use std::collections::HashSet;

/// Match confidence level for function name matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchConfidence {
    None = 0,
    Low = 1,    // Fuzzy/substring match
    Medium = 2, // Variant match
    High = 3,   // Exact match
}

/// Pure function: Generate all name variants for matching
///
/// Produces variants by stripping qualifiers, generics, and lifetimes.
/// Returns variants in order of specificity (exact → most general).
///
/// # Examples
/// ```
/// use debtmap::risk::function_name_matching::generate_function_name_variants;
///
/// let variants = generate_function_name_variants("Type::method<T>");
/// assert_eq!(variants, vec![
///     "Type::method<T>",  // Original
///     "Type::method",     // Without generics
///     "method<T>",        // Method with generics
///     "method",           // Method name only
/// ]);
/// ```
pub fn generate_function_name_variants(name: &str) -> Vec<String> {
    let mut variants = Vec::with_capacity(4);

    // Always include original
    variants.push(name.to_string());

    // Strip generics: func<T> → func
    if let Some(without_generics) = name.split('<').next() {
        if without_generics != name && !without_generics.is_empty() {
            variants.push(without_generics.to_string());
        }
    }

    // Extract method name: Type::method → method
    if let Some(method_name) = name.rsplit("::").next() {
        if method_name != name && !method_name.is_empty() {
            // Add method name with its generics if present
            if !variants.contains(&method_name.to_string()) {
                variants.push(method_name.to_string());
            }

            // Also strip generics from method name
            if let Some(method_no_generics) = method_name.split('<').next() {
                if method_no_generics != method_name && !method_no_generics.is_empty() {
                    variants.push(method_no_generics.to_string());
                }
            }
        }
    }

    // Deduplicate while preserving order
    let mut seen = HashSet::new();
    variants.retain(|v| seen.insert(v.clone()));

    variants
}

/// Pure function: Extract parent function from closure name
///
/// Detects {{closure}} pattern and extracts parent function name.
///
/// # Examples
/// ```
/// use debtmap::risk::function_name_matching::extract_closure_parent;
///
/// assert_eq!(
///     extract_closure_parent("async_fn::{{closure}}"),
///     Some("async_fn".to_string())
/// );
/// assert_eq!(
///     extract_closure_parent("process::{{closure}}#0"),
///     Some("process".to_string())
/// );
/// assert_eq!(extract_closure_parent("regular_function"), None);
/// ```
pub fn extract_closure_parent(name: &str) -> Option<String> {
    if !name.contains("{{closure}}") {
        return None;
    }

    name.split("::{{closure}}")
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Pure function: Check if function names match with confidence level
///
/// Returns tuple of (matches: bool, confidence: MatchConfidence).
///
/// # Matching Strategy
/// 1. Exact match → High confidence
/// 2. Closure parent attribution → High confidence
/// 3. Variant match (method name, no generics) → Medium confidence
/// 4. Fuzzy substring match → Low confidence
/// 5. No match → None confidence
///
/// # Examples
/// ```
/// use debtmap::risk::function_name_matching::{function_names_match, MatchConfidence};
///
/// let (matches, confidence) = function_names_match("foo", "foo");
/// assert!(matches);
/// assert_eq!(confidence, MatchConfidence::High);
///
/// let (matches, confidence) = function_names_match("Type::method", "method");
/// assert!(matches);
/// assert_eq!(confidence, MatchConfidence::Medium);
/// ```
pub fn function_names_match(query: &str, lcov: &str) -> (bool, MatchConfidence) {
    // Exact match - highest confidence
    if query == lcov {
        return (true, MatchConfidence::High);
    }

    // Check closure parent attribution
    if let Some(parent) = extract_closure_parent(lcov) {
        if query == parent {
            return (true, MatchConfidence::High);
        }
    }

    // Check if query is closure and lcov matches its parent
    if let Some(parent) = extract_closure_parent(query) {
        if parent == lcov {
            return (true, MatchConfidence::High);
        }
    }

    // Generate variants for both query and LCOV
    let query_variants = generate_function_name_variants(query);
    let lcov_variants = generate_function_name_variants(lcov);

    // Variant match - medium confidence
    for qv in &query_variants {
        for lv in &lcov_variants {
            if qv == lv {
                return (true, MatchConfidence::Medium);
            }
        }
    }

    // Fuzzy match - low confidence
    // Check if one name contains the other
    if query.contains(lcov) || lcov.contains(query) {
        return (true, MatchConfidence::Low);
    }

    (false, MatchConfidence::None)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Variant Generation Tests
    // ========================================================================

    #[test]
    fn test_generate_variants_simple() {
        let variants = generate_function_name_variants("simple_func");
        assert_eq!(variants, vec!["simple_func"]);
    }

    #[test]
    fn test_generate_variants_type_method() {
        let variants = generate_function_name_variants("Type::method");
        assert!(variants.contains(&"Type::method".to_string()));
        assert!(variants.contains(&"method".to_string()));
        assert_eq!(variants.len(), 2);
    }

    #[test]
    fn test_generate_variants_with_generics() {
        let variants = generate_function_name_variants("process<T, U>");
        assert!(variants.contains(&"process<T, U>".to_string()));
        assert!(variants.contains(&"process".to_string()));
        assert_eq!(variants.len(), 2);
    }

    #[test]
    fn test_generate_variants_type_method_with_generics() {
        let variants = generate_function_name_variants("Type::method<T>");
        assert!(variants.contains(&"Type::method<T>".to_string()));
        assert!(variants.contains(&"Type::method".to_string()));
        assert!(variants.contains(&"method<T>".to_string()));
        assert!(variants.contains(&"method".to_string()));
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_generate_variants_nested_path() {
        let variants = generate_function_name_variants("crate::module::Type::method<T>");
        assert!(variants.contains(&"method".to_string()));
        assert!(variants.contains(&"method<T>".to_string()));
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn test_generate_variants_nested_path_no_generics() {
        let variants = generate_function_name_variants("crate::module::Type::method");
        assert!(variants.contains(&"crate::module::Type::method".to_string()));
        assert!(variants.contains(&"method".to_string()));
        assert_eq!(variants.len(), 2);
    }

    #[test]
    fn test_generate_variants_empty_string() {
        let variants = generate_function_name_variants("");
        assert_eq!(variants, vec![""]);
    }

    #[test]
    fn test_generate_variants_unicode() {
        let variants = generate_function_name_variants("测试函数");
        assert!(variants.contains(&"测试函数".to_string()));
        assert_eq!(variants.len(), 1);
    }

    #[test]
    fn test_generate_variants_very_long_name() {
        let long_name = "a".repeat(1000);
        let variants = generate_function_name_variants(&long_name);
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0], long_name);
    }

    #[test]
    fn test_generate_variants_special_characters() {
        let variants = generate_function_name_variants("func_with_$special");
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0], "func_with_$special");
    }

    // ========================================================================
    // Closure Parent Extraction Tests
    // ========================================================================

    #[test]
    fn test_extract_closure_parent_basic() {
        assert_eq!(
            extract_closure_parent("async_fn::{{closure}}"),
            Some("async_fn".to_string())
        );
    }

    #[test]
    fn test_extract_closure_parent_numbered() {
        assert_eq!(
            extract_closure_parent("process::{{closure}}#0"),
            Some("process".to_string())
        );
    }

    #[test]
    fn test_extract_closure_parent_nested() {
        assert_eq!(
            extract_closure_parent("module::Type::method::{{closure}}"),
            Some("module::Type::method".to_string())
        );
    }

    #[test]
    fn test_extract_closure_parent_regular_function() {
        assert_eq!(extract_closure_parent("regular_function"), None);
    }

    #[test]
    fn test_extract_closure_parent_empty_parent() {
        assert_eq!(extract_closure_parent("::{{closure}}"), None);
    }

    // ========================================================================
    // Function Name Matching Tests
    // ========================================================================

    #[test]
    fn test_function_names_match_exact() {
        let (matches, confidence) = function_names_match("foo", "foo");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::High);
    }

    #[test]
    fn test_function_names_match_variant() {
        let (matches, confidence) = function_names_match("Type::method", "method");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::Medium);
    }

    #[test]
    fn test_function_names_match_variant_reverse() {
        let (matches, confidence) = function_names_match("method", "Type::method");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::Medium);
    }

    #[test]
    fn test_function_names_match_closure() {
        let (matches, confidence) = function_names_match("async_fn", "async_fn::{{closure}}");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::High);
    }

    #[test]
    fn test_function_names_match_closure_reverse() {
        let (matches, confidence) = function_names_match("async_fn::{{closure}}", "async_fn");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::High);
    }

    #[test]
    fn test_function_names_match_with_generics() {
        let (matches, confidence) = function_names_match("process<T>", "process");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::Medium);
    }

    #[test]
    fn test_function_names_match_fuzzy_contains() {
        let (matches, confidence) =
            function_names_match("RecursiveDetector::visit_expr", "visit_expr");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::Medium);
    }

    #[test]
    fn test_function_names_match_no_match() {
        let (matches, confidence) = function_names_match("foo", "bar");
        assert!(!matches);
        assert_eq!(confidence, MatchConfidence::None);
    }

    #[test]
    fn test_function_names_match_trait_impl() {
        let (matches, confidence) = function_names_match("Visitor::visit_expr", "visit_expr");
        assert!(matches);
        assert_eq!(confidence, MatchConfidence::Medium);
    }

    // ========================================================================
    // Property-Based Tests
    // ========================================================================

    #[test]
    fn variant_generation_never_panics_sample() {
        // Sample of potentially problematic inputs
        let long_name = "a".repeat(1000);
        let test_cases = vec![
            "",
            "a",
            ":::",
            "<<<",
            ">>>",
            "a::b::c::d::e",
            "func<>",
            "{{closure}}",
            "测试",
            &long_name,
        ];

        for name in test_cases {
            let _ = generate_function_name_variants(name);
        }
    }

    #[test]
    fn original_always_in_variants() {
        let test_cases = vec!["simple", "Type::method", "func<T>", "a::b::c"];

        for name in test_cases {
            let variants = generate_function_name_variants(name);
            assert!(
                variants.contains(&name.to_string()),
                "Original name '{}' not in variants: {:?}",
                name,
                variants
            );
        }
    }

    #[test]
    fn matching_is_reflexive() {
        let test_cases = vec![
            "simple",
            "Type::method",
            "func<T>",
            "async_fn::{{closure}}",
            "测试",
        ];

        for name in test_cases {
            let (matches, _) = function_names_match(name, name);
            assert!(matches, "Name '{}' should match itself", name);
        }
    }
}
