//! Mapping pattern detection
//!
//! Detects pure mapping patterns and calculates adjusted complexity.

use crate::complexity::pure_mapping_patterns::{
    calculate_adjusted_complexity, MappingPatternConfig, MappingPatternDetector,
    MappingPatternResult,
};

/// Detect mapping patterns and calculate adjusted complexity (spec 118)
pub fn detect_mapping_pattern(
    block: &syn::Block,
    cyclomatic: u32,
    cognitive: u32,
) -> (MappingPatternResult, Option<f64>) {
    let function_body = quote::quote!(#block).to_string();
    let detector = MappingPatternDetector::new(MappingPatternConfig::default());
    let result = detector.analyze_function(&function_body, cyclomatic);

    let adjusted = result
        .is_pure_mapping
        .then(|| calculate_adjusted_complexity(cyclomatic, cognitive, &result));

    (result, adjusted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_detect_simple_block() {
        let block: syn::Block = parse_quote!({
            let x = 1;
            x + 2
        });
        let (result, _adjusted) = detect_mapping_pattern(&block, 1, 0);
        // Simple blocks are not pure mapping patterns
        assert!(!result.is_pure_mapping);
    }
}
