//! Function metadata extraction
//!
//! Pure functions for extracting metadata from function signatures.

use super::test_detection::is_test_function;
use crate::analyzers::purity_detector::PurityDetector;
use crate::analyzers::rust::types::FunctionMetadata;
use crate::config::get_entropy_config;

/// Extract metadata from a function
pub fn extract_function_metadata(name: &str, item_fn: &syn::ItemFn) -> FunctionMetadata {
    FunctionMetadata {
        is_test: is_test_function(name, item_fn),
        visibility: extract_visibility(&item_fn.vis),
        entropy_score: calculate_entropy_if_enabled(&item_fn.block),
        purity_info: detect_purity(item_fn),
    }
}

/// Extract visibility string from syn::Visibility
pub fn extract_visibility(vis: &syn::Visibility) -> Option<String> {
    match vis {
        syn::Visibility::Public(_) => Some("pub".to_string()),
        syn::Visibility::Restricted(restricted) => {
            if restricted.path.is_ident("crate") {
                Some("pub(crate)".to_string())
            } else {
                Some(format!("pub({})", quote::quote!(#restricted.path)))
            }
        }
        syn::Visibility::Inherited => None,
    }
}

/// Calculate entropy score if enabled
pub fn calculate_entropy_if_enabled(
    block: &syn::Block,
) -> Option<crate::complexity::entropy_core::EntropyScore> {
    if get_entropy_config().enabled {
        let mut old_analyzer = crate::complexity::entropy::EntropyAnalyzer::new();
        let old_score = old_analyzer.calculate_entropy(block);

        Some(crate::complexity::entropy_core::EntropyScore {
            token_entropy: old_score.token_entropy,
            pattern_repetition: old_score.pattern_repetition,
            branch_similarity: old_score.branch_similarity,
            effective_complexity: old_score.effective_complexity,
            unique_variables: old_score.unique_variables,
            max_nesting: old_score.max_nesting,
            dampening_applied: old_score.dampening_applied,
        })
    } else {
        None
    }
}

/// Detect purity of a function
pub fn detect_purity(
    item_fn: &syn::ItemFn,
) -> (Option<bool>, Option<f32>, Option<crate::core::PurityLevel>) {
    let mut detector = PurityDetector::new();
    let analysis = detector.is_pure_function(item_fn);
    (
        Some(analysis.is_pure),
        Some(analysis.confidence),
        Some(analysis.purity_level),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_extract_visibility_public() {
        let vis: syn::Visibility = parse_quote! { pub };
        assert_eq!(extract_visibility(&vis), Some("pub".to_string()));
    }

    #[test]
    fn test_extract_visibility_pub_crate() {
        let vis: syn::Visibility = parse_quote! { pub(crate) };
        assert_eq!(extract_visibility(&vis), Some("pub(crate)".to_string()));
    }

    #[test]
    fn test_extract_visibility_pub_super() {
        let vis: syn::Visibility = parse_quote! { pub(super) };
        let result = extract_visibility(&vis);
        assert!(result.is_some());
        assert!(result.unwrap().starts_with("pub("));
    }

    #[test]
    fn test_extract_visibility_inherited() {
        let vis: syn::Visibility = parse_quote! {};
        assert_eq!(extract_visibility(&vis), None);
    }

    #[test]
    fn test_detect_purity_pure_function() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        };
        let (is_pure, confidence, _purity_level) = detect_purity(&item_fn);
        assert!(is_pure.is_some());
        assert!(confidence.is_some());
        if let (Some(pure), Some(conf)) = (is_pure, confidence) {
            if pure {
                assert!(conf > 0.5);
            }
        }
    }

    #[test]
    fn test_detect_purity_impure_function() {
        let item_fn: syn::ItemFn = parse_quote! {
            fn print_value(x: i32) {
                println!("Value: {}", x);
            }
        };
        let (is_pure, confidence, _purity_level) = detect_purity(&item_fn);
        assert!(is_pure.is_some());
        assert!(confidence.is_some());
        if let Some(pure) = is_pure {
            assert!(!pure);
        }
    }
}
