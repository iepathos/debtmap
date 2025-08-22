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
    // Test high pattern repetition leading to reduction
    let base_complexity = 25;
    let entropy_score = create_entropy_score(
        0.7, // Normal entropy
        0.9, // Very high repetition (above threshold)
        0.4, // Normal branch similarity
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // High repetition should reduce complexity
    assert!(result < base_complexity);
    assert!(result >= base_complexity / 2); // Safety cap
}

#[test]
fn test_apply_entropy_dampening_low_entropy() {
    // Test low token entropy leading to reduction
    let base_complexity = 30;
    let entropy_score = create_entropy_score(
        0.2, // Very low entropy (below threshold)
        0.4, // Normal repetition
        0.4, // Normal branch similarity
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // Low entropy should reduce complexity
    assert!(result < base_complexity);
    assert!(result >= base_complexity / 2);
}

#[test]
fn test_apply_entropy_dampening_high_branch_similarity() {
    // Test high branch similarity leading to reduction
    let base_complexity = 18;
    let entropy_score = create_entropy_score(
        0.6,  // Normal entropy
        0.4,  // Normal repetition
        0.85, // Very high branch similarity (above threshold)
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // High branch similarity should reduce complexity
    assert!(result < base_complexity);
    assert!(result >= base_complexity / 2);
}

#[test]
fn test_apply_entropy_dampening_combined_factors() {
    // Test multiple dampening factors combined
    let base_complexity = 40;
    let entropy_score = create_entropy_score(
        0.3, // Low entropy
        0.8, // High repetition
        0.8, // High branch similarity
    );

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // Multiple factors should reduce complexity significantly
    assert!(result < base_complexity);
    assert!(result >= base_complexity / 2); // Still respects safety cap
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
    // Test that reduction is graduated based on how much values exceed thresholds
    let base_complexity = 20;

    // Slightly above threshold
    let slight_excess = create_entropy_score(0.6, 0.65, 0.65);
    let result_slight = apply_entropy_dampening(base_complexity, &slight_excess);

    // Significantly above threshold
    let high_excess = create_entropy_score(0.6, 0.95, 0.95);
    let result_high = apply_entropy_dampening(base_complexity, &high_excess);

    // Higher excess should lead to more reduction
    assert!(result_high < result_slight);
    assert!(result_slight <= base_complexity);
}

#[test]
fn test_apply_entropy_dampening_weight_application() {
    // Test that config weight affects the dampening
    let base_complexity = 25;

    // Create score that would trigger dampening
    let entropy_score = create_entropy_score(0.2, 0.8, 0.8);

    let result = apply_entropy_dampening(base_complexity, &entropy_score);

    // Result should be dampened but within expected bounds
    assert!(result < base_complexity);
    assert!(result >= (base_complexity as f64 * 0.5) as u32);
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
