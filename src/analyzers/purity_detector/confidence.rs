//! Confidence score calculation
//!
//! Pure functions for calculating confidence scores in purity analysis.

use super::types::{LocalMutation, UpvalueMutation};

/// Parameters for confidence calculation
pub struct ConfidenceParams<'a> {
    pub has_side_effects: bool,
    pub has_io_operations: bool,
    pub has_unsafe_blocks: bool,
    pub modifies_external_state: bool,
    pub accesses_external_state: bool,
    pub local_mutations: &'a [LocalMutation],
    pub upvalue_mutations: &'a [UpvalueMutation],
    pub unknown_macros_count: usize,
    pub has_pure_unsafe: bool,
}

/// Calculate confidence score for purity analysis
pub fn calculate_confidence_score(params: &ConfidenceParams) -> f32 {
    let mut confidence: f32 = 1.0;

    // Reduce confidence if we only access external state
    if params.accesses_external_state && !params.modifies_external_state {
        confidence *= 0.8;
    }

    // Reduce confidence for upvalue mutations (closures)
    if !params.upvalue_mutations.is_empty() {
        confidence *= 0.85;
    }

    // High confidence for simple local mutations
    if !params.local_mutations.is_empty() && params.local_mutations.len() < 5 {
        confidence *= 0.95;
    }

    // Reduce confidence for unknown macros (conservative approach)
    for _ in 0..params.unknown_macros_count {
        confidence *= 0.95;
    }

    // Reduce confidence for pure unsafe operations (Spec 161)
    if params.has_pure_unsafe {
        confidence *= 0.85;
    }

    // If no impurities detected and no confidence-reducing factors, set high confidence
    if !params.has_side_effects
        && !params.has_io_operations
        && !params.has_unsafe_blocks
        && !params.modifies_external_state
        && !params.has_pure_unsafe
        && !params.accesses_external_state
        && params.unknown_macros_count == 0
        && params.local_mutations.is_empty()
        && params.upvalue_mutations.is_empty()
    {
        confidence = 0.95;
    }

    confidence.clamp(0.5, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params<'a>() -> ConfidenceParams<'a> {
        ConfidenceParams {
            has_side_effects: false,
            has_io_operations: false,
            has_unsafe_blocks: false,
            modifies_external_state: false,
            accesses_external_state: false,
            local_mutations: &[],
            upvalue_mutations: &[],
            unknown_macros_count: 0,
            has_pure_unsafe: false,
        }
    }

    #[test]
    fn test_high_confidence_for_pure() {
        let params = default_params();
        let confidence = calculate_confidence_score(&params);
        assert!(confidence >= 0.9);
    }

    #[test]
    fn test_reduced_confidence_for_external_access() {
        let mut params = default_params();
        params.accesses_external_state = true;
        let confidence = calculate_confidence_score(&params);
        assert!(confidence < 0.95);
    }

    #[test]
    fn test_reduced_confidence_for_pure_unsafe() {
        let mut params = default_params();
        params.has_pure_unsafe = true;
        let confidence = calculate_confidence_score(&params);
        assert!(confidence < 0.9);
    }

    #[test]
    fn test_reduced_confidence_for_unknown_macros() {
        let mut params = default_params();
        params.unknown_macros_count = 3;
        let confidence = calculate_confidence_score(&params);
        assert!(confidence < 0.95);
    }
}
