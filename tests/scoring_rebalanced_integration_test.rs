//! Integration test for rebalanced scoring algorithm (Spec 136)
//!
//! This test validates that the rebalanced scoring algorithm correctly prioritizes
//! actual code quality issues (complexity + coverage gaps) over pure file size concerns.

use debtmap::core::FunctionMetrics;
use debtmap::priority::scoring::rebalanced::{DebtScore, ScoreWeights};
use debtmap::priority::DebtType;
use std::path::PathBuf;

#[test]
fn test_complexity_and_coverage_outweigh_size() {
    // Create synthetic test cases that mirror real-world patterns

    // Case 1: Complex untested function (should score HIGH)
    let complex_untested = create_test_function("complex_untested", 42, 77, 150);
    let complex_score = DebtScore::calculate(
        &complex_untested,
        &DebtType::TestingGap {
            coverage: 0.38,
            cyclomatic: 42,
            cognitive: 77,
        },
        &ScoreWeights::default(),
    );

    // Case 2: Large but simple well-tested function (should score LOW)
    let large_simple = create_test_function("large_simple", 3, 5, 2000);
    let large_score = DebtScore::calculate(
        &large_simple,
        &DebtType::Risk {
            risk_score: 0.2,
            factors: vec!["Long function".to_string()],
        },
        &ScoreWeights::default(),
    );

    // Case 3: Moderate complexity with low coverage (should score MEDIUM-HIGH)
    let moderate_untested = create_test_function("moderate_untested", 20, 35, 100);
    let moderate_score = DebtScore::calculate(
        &moderate_untested,
        &DebtType::TestingGap {
            coverage: 0.25,
            cyclomatic: 20,
            cognitive: 35,
        },
        &ScoreWeights::default(),
    );

    // Validation: Complex + untested should score much higher than large simple
    assert!(
        complex_score.total > large_score.total * 1.5,
        "Complex untested (score={:.1}) should score significantly higher than large simple (score={:.1})",
        complex_score.total,
        large_score.total
    );

    // Validation: Moderate complexity + coverage gap should score higher than large simple
    assert!(
        moderate_score.total > large_score.total,
        "Moderate untested (score={:.1}) should score higher than large simple (score={:.1})",
        moderate_score.total,
        large_score.total
    );

    // Validation: Complex untested should be Critical or High severity
    assert!(
        matches!(
            complex_score.severity,
            debtmap::priority::scoring::rebalanced::Severity::Critical
                | debtmap::priority::scoring::rebalanced::Severity::High
        ),
        "Complex untested should be Critical or High, got {:?}",
        complex_score.severity
    );

    // Validation: Large simple should be Low or Medium severity
    assert!(
        matches!(
            large_score.severity,
            debtmap::priority::scoring::rebalanced::Severity::Low
                | debtmap::priority::scoring::rebalanced::Severity::Medium
        ),
        "Large simple should be Low or Medium, got {:?}",
        large_score.severity
    );

    // Print diagnostic information
    println!("\n=== Scoring Results ===");
    println!(
        "Complex untested: score={:.1}, severity={:?}",
        complex_score.total, complex_score.severity
    );
    println!("  Components: {:?}", complex_score.components);
    println!(
        "Large simple: score={:.1}, severity={:?}",
        large_score.total, large_score.severity
    );
    println!("  Components: {:?}", large_score.components);
    println!(
        "Moderate untested: score={:.1}, severity={:?}",
        moderate_score.total, moderate_score.severity
    );
    println!("  Components: {:?}", moderate_score.components);
}

#[test]
fn test_preset_weights_behavior() {
    let complex_func = create_test_function("complex", 25, 40, 150);
    let debt_type = DebtType::TestingGap {
        coverage: 0.3,
        cyclomatic: 25,
        cognitive: 40,
    };

    // Test different presets
    let balanced_score = DebtScore::calculate(&complex_func, &debt_type, &ScoreWeights::balanced());
    let quality_score =
        DebtScore::calculate(&complex_func, &debt_type, &ScoreWeights::quality_focused());
    let size_score = DebtScore::calculate(&complex_func, &debt_type, &ScoreWeights::size_focused());
    let testing_score = DebtScore::calculate(
        &complex_func,
        &debt_type,
        &ScoreWeights::test_coverage_focused(),
    );

    // Quality-focused should score higher than balanced for coverage gaps
    assert!(
        quality_score.total >= balanced_score.total,
        "Quality-focused should prioritize coverage gaps"
    );

    // Test-coverage-focused should have highest coverage component weight
    assert!(
        testing_score.components.coverage_score >= quality_score.components.coverage_score,
        "Test-coverage preset should emphasize coverage the most"
    );

    println!("\n=== Preset Comparisons ===");
    println!(
        "Balanced: total={:.1}, coverage_score={:.1}",
        balanced_score.total, balanced_score.components.coverage_score
    );
    println!(
        "Quality-focused: total={:.1}, coverage_score={:.1}",
        quality_score.total, quality_score.components.coverage_score
    );
    println!(
        "Size-focused: total={:.1}, size_score={:.1}",
        size_score.total, size_score.components.size_score
    );
    println!(
        "Test-coverage: total={:.1}, coverage_score={:.1}",
        testing_score.total, testing_score.components.coverage_score
    );
}

#[test]
fn test_additive_bonus_for_complex_untested() {
    let weights = ScoreWeights::default();

    // High complexity + low coverage (should get bonus)
    let complex_untested = create_test_function("complex_untested", 20, 35, 100);
    let complex_score = DebtScore::calculate(
        &complex_untested,
        &DebtType::TestingGap {
            coverage: 0.4,
            cyclomatic: 20,
            cognitive: 35,
        },
        &weights,
    );

    // Low complexity + low coverage (no bonus)
    let simple_untested = create_test_function("simple_untested", 5, 8, 50);
    let simple_score = DebtScore::calculate(
        &simple_untested,
        &DebtType::TestingGap {
            coverage: 0.4,
            cyclomatic: 5,
            cognitive: 8,
        },
        &weights,
    );

    // The bonus should be additive, not multiplicative
    let coverage_diff =
        complex_score.components.coverage_score - simple_score.components.coverage_score;

    assert!(
        (10.0..=20.0).contains(&coverage_diff),
        "Bonus for complex untested should be additive (10-20 points), got {:.1}",
        coverage_diff
    );

    println!("\n=== Additive Bonus Validation ===");
    println!(
        "Complex untested coverage score: {:.1}",
        complex_score.components.coverage_score
    );
    println!(
        "Simple untested coverage score: {:.1}",
        simple_score.components.coverage_score
    );
    println!("Bonus difference: {:.1}", coverage_diff);
}

#[test]
fn test_structural_issues_god_objects() {
    let func = create_test_function("god_object_method", 15, 25, 100);

    let god_object_score = DebtScore::calculate(
        &func,
        &DebtType::GodObject {
            methods: 45,
            fields: 25,
            responsibilities: 8,
            god_object_score: 3.5,
        },
        &ScoreWeights::default(),
    );

    // God objects should have significant structural score
    assert!(
        god_object_score.components.structural_score > 30.0,
        "God objects should have high structural score, got {:.1}",
        god_object_score.components.structural_score
    );

    // Should be at least Medium severity
    assert!(
        !matches!(
            god_object_score.severity,
            debtmap::priority::scoring::rebalanced::Severity::Low
        ),
        "God objects should be at least Medium severity, got {:?}",
        god_object_score.severity
    );

    println!("\n=== God Object Scoring ===");
    println!("Total score: {:.1}", god_object_score.total);
    println!(
        "Structural score: {:.1}",
        god_object_score.components.structural_score
    );
    println!("Severity: {:?}", god_object_score.severity);
}

#[test]
fn test_score_normalization_range() {
    // Test various scenarios to ensure scores stay in 0-200 range

    let scenarios = vec![
        (
            "max_complexity",
            50,
            100,
            300,
            DebtType::ComplexityHotspot {
                cyclomatic: 50,
                cognitive: 100,
                adjusted_cyclomatic: None,
            },
        ),
        (
            "max_coverage_gap",
            30,
            50,
            200,
            DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 30,
                cognitive: 50,
            },
        ),
        (
            "minimal",
            2,
            3,
            20,
            DebtType::Risk {
                risk_score: 0.1,
                factors: vec!["Minor issue".to_string()],
            },
        ),
    ];

    for (name, cyclomatic, cognitive, length, debt_type) in scenarios {
        let func = create_test_function(name, cyclomatic, cognitive, length);
        let score = DebtScore::calculate(&func, &debt_type, &ScoreWeights::default());

        assert!(
            score.total >= 0.0 && score.total <= 200.0,
            "Score for {} should be in 0-200 range, got {:.1}",
            name,
            score.total
        );
    }
}

// Helper function to create test function metrics
fn create_test_function(
    name: &str,
    cyclomatic: u32,
    cognitive: u32,
    length: usize,
) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from("test.rs"),
        line: 1,
        cyclomatic,
        cognitive,
        nesting: (cognitive / 10).min(5),
        length,
        is_test: false,
        visibility: Some("pub".to_string()),
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: Some(false),
        purity_confidence: Some(0.5),
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_reason: None,
        call_dependencies: None,
        purity_level: None,
    }
}
