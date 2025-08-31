use debtmap::complexity::entropy::{apply_entropy_dampening, EntropyScore};

/// Helper function to create an EntropyScore with specified values
fn create_entropy_score(
    token_entropy: f64,
    pattern_repetition: f64,
    branch_similarity: f64,
) -> EntropyScore {
    EntropyScore {
        token_entropy,
        pattern_repetition,
        branch_similarity,
        effective_complexity: 0.0,
        unique_variables: 0,
        max_nesting: 0,
        dampening_applied: 0.0,
    }
}

#[test]
fn test_apply_entropy_dampening_disabled_config() {
    // When entropy is disabled, should return base complexity unchanged
    let base_complexity = 20;
    let entropy_score = create_entropy_score(0.5, 0.5, 0.5);

    // Note: This test depends on the config state
    // If enabled in config, the test behavior will differ
    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // Result should be either unchanged or dampened based on config
    assert!(result <= base_complexity);
    assert!(result >= base_complexity / 2); // Never reduce by more than 50%
}

#[test]
fn test_apply_entropy_dampening_no_reduction() {
    // Test case where no dampening factors apply (all below thresholds)
    let base_complexity = 15;
    let entropy_score = create_entropy_score(
        0.8, // High entropy (above typical threshold)
        0.3, // Low repetition (below typical threshold)
        0.3, // Low branch similarity (below typical threshold)
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // With no reduction factors, complexity should remain close to base
    // Allow for small weight adjustments based on config
    assert!(result <= base_complexity);
    assert!(result >= (base_complexity as f64 * 0.7) as u32);
}

#[test]
fn test_apply_entropy_dampening_high_repetition() {
    // Per spec 68: Only low entropy (< 0.2) causes dampening, not high repetition alone
    let base_complexity = 25;
    let entropy_score = create_entropy_score(
        0.7, // Normal entropy (> 0.2, so NO dampening per spec 68)
        0.9, // Very high repetition (ignored when entropy > 0.2)
        0.4, // Normal branch similarity
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // With entropy > 0.2, no dampening should occur per spec 68
    assert_eq!(result, base_complexity);
}

#[test]
fn test_apply_entropy_dampening_low_entropy() {
    // Test low token entropy leading to reduction (spec 68)
    let base_complexity = 30;
    let entropy_score = create_entropy_score(
        0.1, // Very low entropy (< 0.2 threshold, triggers dampening)
        0.4, // Normal repetition
        0.4, // Normal branch similarity
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // Low entropy (0.1) should reduce complexity
    // Per spec 68: dampening_factor = 0.5 + 0.5 * (0.1/0.2) = 0.75
    // So result should be 30 * 0.75 = 22.5, rounded to 22 or 23
    assert!(result < base_complexity);
    assert!(result >= base_complexity / 2); // Never less than 50%
    assert!(result >= 22 && result <= 23);
}

#[test]
fn test_apply_entropy_dampening_high_branch_similarity() {
    // Per spec 68: Only low entropy (< 0.2) causes dampening
    let base_complexity = 18;
    let entropy_score = create_entropy_score(
        0.6,  // Normal entropy (> 0.2, so NO dampening)
        0.4,  // Normal repetition
        0.85, // Very high branch similarity (ignored when entropy > 0.2)
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // With entropy > 0.2, no dampening should occur
    assert_eq!(result, base_complexity);
}

#[test]
fn test_apply_entropy_dampening_combined_factors() {
    // Per spec 68: Only entropy < 0.2 matters, other factors ignored
    let base_complexity = 40;
    let entropy_score = create_entropy_score(
        0.3, // Entropy > 0.2, so NO dampening despite other factors
        0.8, // High repetition (ignored)
        0.8, // High branch similarity (ignored)
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // With entropy > 0.2, no dampening occurs regardless of other factors
    assert_eq!(result, base_complexity);
}

#[test]
fn test_apply_entropy_dampening_maximum_reduction() {
    // Test extreme case with all factors maxed
    let base_complexity = 100;
    let entropy_score = create_entropy_score(
        0.0, // Minimum entropy
        1.0, // Maximum repetition
        1.0, // Maximum branch similarity
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // Should apply significant reduction and respect 50% safety cap
    assert!(result <= base_complexity);
    assert!(result >= base_complexity / 2);
}

#[test]
fn test_apply_entropy_dampening_small_complexity() {
    // Test with small base complexity values
    let base_complexity = 3;
    let entropy_score = create_entropy_score(
        0.1, // Low entropy
        0.9, // High repetition
        0.9, // High branch similarity
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // Even with reductions, should maintain minimum complexity
    assert!(result >= 1);
    assert!(result <= base_complexity);
}

#[test]
fn test_apply_entropy_dampening_boundary_values() {
    // Test edge cases and boundary values

    // Zero complexity (edge case)
    let result_zero = apply_entropy_dampening(0, &create_entropy_score(0.5, 0.5, 0.5));
    assert_eq!(result_zero, 0);

    // Maximum u32 value
    let max_complexity = u32::MAX;
    let result_max = apply_entropy_dampening(max_complexity, &create_entropy_score(0.5, 0.5, 0.5));
    assert!(result_max <= max_complexity);
    assert!(result_max >= max_complexity / 2);
}

#[test]
fn test_apply_entropy_dampening_graduated_reduction() {
    // Test graduated reduction based on entropy level (spec 68)
    let base_complexity = 20;

    // Just below threshold (more dampening)
    let low_entropy = create_entropy_score(0.1, 0.65, 0.65);
    let result_low = apply_entropy_dampening(base_complexity, &low_entropy);

    // Very low entropy (maximum dampening)
    let very_low_entropy = create_entropy_score(0.0, 0.95, 0.95);
    let result_very_low = apply_entropy_dampening(base_complexity, &very_low_entropy);

    // Lower entropy should lead to more reduction
    // 0.0 entropy = 50% dampening, 0.1 entropy = 75% preserved
    assert!(result_very_low < result_low);
    assert_eq!(result_very_low, base_complexity / 2); // 50% for 0.0 entropy
    assert!(result_low == 15); // 75% of 20 for 0.1 entropy
}

#[test]
fn test_apply_entropy_dampening_weight_application() {
    // Test entropy at the boundary (spec 68)
    let base_complexity = 25;

    // Entropy at exactly 0.2 threshold - NO dampening
    let entropy_score = create_entropy_score(0.2, 0.8, 0.8);

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // At threshold, no dampening should occur
    assert_eq!(result, base_complexity);
}

#[test]
fn test_apply_entropy_dampening_preserves_relative_order() {
    // Test that relative complexity ordering is preserved
    let entropy_score = create_entropy_score(0.3, 0.7, 0.7);

    let complexity_low = 10;
    let complexity_mid = 20;
    let complexity_high = 30;

    let result_low = apply_entropy_dampening(complexity_low, &entropy_score);
    let result_mid = apply_entropy_dampening(complexity_mid, &entropy_score);
    let result_high = apply_entropy_dampening(complexity_high, &entropy_score);

    // Relative order should be preserved
    assert!(result_low <= result_mid);
    assert!(result_mid <= result_high);
}
