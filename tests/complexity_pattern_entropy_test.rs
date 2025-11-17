use debtmap::priority::complexity_patterns::{ComplexityMetrics, ComplexityPattern};

/// Test that chaotic pattern detection uses token_entropy, not effective_complexity
#[test]
fn chaotic_pattern_uses_token_entropy_not_effective_complexity() {
    let metrics = ComplexityMetrics {
        cyclomatic: 15,
        cognitive: 50,
        nesting: 3,
        entropy_score: Some(0.6), // This should be token_entropy
        state_signals: None,
        coordinator_signals: None,
    };

    let pattern = ComplexityPattern::detect(&metrics);

    if let ComplexityPattern::ChaoticStructure { entropy, .. } = pattern {
        // Verify the entropy value matches what we passed
        assert!(
            (entropy - 0.6).abs() < 0.01,
            "Pattern should use provided entropy value, got {}",
            entropy
        );
    } else {
        panic!("Expected ChaoticStructure pattern for entropy 0.6");
    }
}

/// Test that entropy below threshold doesn't trigger chaotic pattern
#[test]
fn below_threshold_not_chaotic() {
    let metrics = ComplexityMetrics {
        cyclomatic: 15,
        cognitive: 50,
        nesting: 3,
        entropy_score: Some(0.44), // Just below 0.45 threshold
        state_signals: None,
        coordinator_signals: None,
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(
        !matches!(pattern, ComplexityPattern::ChaoticStructure { .. }),
        "entropy 0.44 should not trigger chaotic pattern"
    );
}

/// Test that entropy exactly at threshold triggers chaotic pattern
#[test]
fn at_threshold_is_chaotic() {
    let metrics = ComplexityMetrics {
        cyclomatic: 15,
        cognitive: 50,
        nesting: 3,
        entropy_score: Some(0.45), // Exactly at threshold
        state_signals: None,
        coordinator_signals: None,
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(
        matches!(pattern, ComplexityPattern::ChaoticStructure { .. }),
        "entropy 0.45 should trigger chaotic pattern"
    );
}

/// Test that chaotic pattern takes precedence over other patterns
#[test]
fn chaotic_pattern_takes_precedence() {
    // Metrics that would trigger HighNesting if not for high entropy
    let metrics = ComplexityMetrics {
        cyclomatic: 12,
        cognitive: 50,            // ratio = 4.17 > 3.0
        nesting: 5,               // >= 4
        entropy_score: Some(0.6), // >= 0.45, should trigger chaotic
        state_signals: None,
        coordinator_signals: None,
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(
        matches!(pattern, ComplexityPattern::ChaoticStructure { .. }),
        "chaotic pattern should take precedence over high nesting"
    );
}

/// Test that None entropy allows other patterns to be detected
#[test]
fn none_entropy_allows_other_patterns() {
    let metrics = ComplexityMetrics {
        cyclomatic: 12,
        cognitive: 50,       // ratio = 4.17 > 3.0
        nesting: 5,          // >= 4
        entropy_score: None, // No entropy data
        state_signals: None,
        coordinator_signals: None,
    };

    let pattern = ComplexityPattern::detect(&metrics);
    assert!(
        matches!(pattern, ComplexityPattern::HighNesting { .. }),
        "should detect high nesting when entropy is None"
    );
}
