// Integration tests for spec 109: Fix Test Calculation Inconsistency in Recommendations
//
// These tests verify that ACTION text and detailed steps always show identical test counts,
// preventing the bug where ACTION says "Add 3 tests" but steps say "Write 11 tests".

use debtmap::priority::scoring::test_calculation::{
    calculate_tests_needed, validate_recommendation_consistency, ComplexityTier,
};

#[test]
fn test_recommendation_consistency_cyclo_33() {
    // Reproduce the original bug scenario from spec 109:
    // Function with cyclomatic complexity 33 and 66.1% coverage
    //
    // The bug: ACTION said "Add 3 tests" but steps said "Write 11 tests"
    // The fix: Both should now say ~12 tests (ceil(33 × 0.339) = 12)

    let cyclomatic = 33;
    let coverage = 0.661;

    let test_rec = calculate_tests_needed(cyclomatic, coverage, None);

    // Should use High tier (31-50) with linear formula
    // 33 × (1 - 0.661) = 33 × 0.339 = 11.187 → ceil() = 12 tests
    assert_eq!(
        test_rec.count, 12,
        "Cyclo 33 with 66.1% coverage should need 12 tests (not 3!)"
    );

    // Verify it's using the High complexity tier
    assert!(
        test_rec.rationale.contains("High complexity"),
        "Should use High complexity tier for cyclo=33"
    );

    // Verify formula is linear (not sqrt)
    assert!(
        test_rec.formula_used.contains("cyclomatic × coverage_gap"),
        "Should use linear formula for High tier"
    );

    // Simulate generating a recommendation with this test count
    let action = format!(
        "Add {} tests for {}% coverage gap, then refactor complexity {} into smaller functions",
        test_rec.count,
        ((1.0 - coverage) * 100.0).round() as u32,
        cyclomatic
    );

    let steps = vec![
        format!("Step 1: Analyze the {} execution paths", cyclomatic),
        format!(
            "Step 2: Currently ~{} of {} branches are uncovered ({}% coverage)",
            (cyclomatic as f64 * (1.0 - coverage)).round() as u32,
            cyclomatic,
            (coverage * 100.0).round() as u32
        ),
        format!(
            "Step 3: Write {} tests to cover critical uncovered branches first",
            test_rec.count
        ),
    ];

    // This validation should pass (all counts are consistent)
    #[cfg(debug_assertions)]
    {
        let result = validate_recommendation_consistency(&action, &steps);
        assert!(
            result.is_ok(),
            "Recommendation should be consistent, got error: {:?}",
            result
        );
    }
}

#[test]
fn test_all_complexity_tiers_generate_consistent_recommendations() {
    // Test that all complexity tiers generate internally consistent recommendations

    let test_cases = vec![
        (5, 0.6, ComplexityTier::Simple, "simple function"),
        (15, 0.5, ComplexityTier::Moderate, "moderate complexity"),
        (33, 0.661, ComplexityTier::High, "high complexity (spec 109 case)"),
        (45, 0.4, ComplexityTier::High, "high complexity near boundary"),
        (60, 0.5, ComplexityTier::Extreme, "extreme complexity"),
    ];

    for (cyclo, coverage, tier, description) in test_cases {
        let test_rec = calculate_tests_needed(cyclo, coverage, Some(tier));

        // Generate ACTION and steps using the same test count
        let action = format!(
            "Add {} tests for coverage improvement",
            test_rec.count
        );

        let steps = vec![
            format!("Step 1: Analyze {} branches", cyclo),
            format!(
                "Step 2: Write {} tests to achieve target coverage",
                test_rec.count
            ),
        ];

        // Validate consistency
        #[cfg(debug_assertions)]
        {
            let result = validate_recommendation_consistency(&action, &steps);
            assert!(
                result.is_ok(),
                "Inconsistent recommendation for {}: {:?}",
                description,
                result
            );
        }
    }
}

#[test]
#[cfg(debug_assertions)]
fn test_detect_inconsistent_recommendations() {
    // Verify that validation catches inconsistencies

    // Case 1: ACTION says 3, steps say 11 (the original bug)
    let bad_action = "Add 3 tests for coverage gap";
    let bad_steps = vec![
        "Step 1: Analyze branches".to_string(),
        "Step 2: Write 11 tests to cover uncovered paths".to_string(),
    ];

    let result = validate_recommendation_consistency(bad_action, &bad_steps);
    assert!(
        result.is_err(),
        "Should detect inconsistency between 3 and 11 tests"
    );
    assert!(result.unwrap_err().contains("spec 109"));

    // Case 2: Multiple different counts in steps
    let bad_action2 = "Add 5 tests for coverage";
    let bad_steps2 = vec![
        "Step 1: Write 3 tests for edge cases".to_string(),
        "Step 2: Write 5 tests for main paths".to_string(),
    ];

    let result2 = validate_recommendation_consistency(bad_action2, &bad_steps2);
    assert!(result2.is_err(), "Should detect inconsistency in steps");
}

#[test]
fn test_boundary_cases_consistency() {
    // Test boundary cases at complexity tier transitions

    let boundary_cases = vec![
        (10, 0.5, "Simple/Moderate boundary - simple side"),
        (11, 0.5, "Simple/Moderate boundary - moderate side"),
        (30, 0.5, "Moderate/High boundary - moderate side"),
        (31, 0.5, "Moderate/High boundary - high side"),
        (50, 0.5, "High/Extreme boundary - high side"),
        (51, 0.5, "High/Extreme boundary - extreme side"),
    ];

    for (cyclo, coverage, description) in boundary_cases {
        let test_rec = calculate_tests_needed(cyclo, coverage, None);

        // Ensure the count is reasonable (not 0, not absurdly high)
        assert!(
            test_rec.count > 0,
            "{}: should recommend at least 1 test",
            description
        );
        assert!(
            test_rec.count < cyclo + 10,
            "{}: test count should be reasonable (got {})",
            description,
            test_rec.count
        );

        // Ensure formula is documented
        assert!(
            !test_rec.formula_used.is_empty(),
            "{}: should document formula used",
            description
        );
    }
}

#[test]
fn test_zero_coverage_gap_no_tests_needed() {
    // Functions with 100% coverage shouldn't recommend tests

    let test_cases = vec![
        (5, 1.0, "simple"),
        (15, 1.0, "moderate"),
        (33, 1.0, "high (spec 109 cyclo)"),
        (60, 1.0, "extreme"),
    ];

    for (cyclo, coverage, description) in test_cases {
        let test_rec = calculate_tests_needed(cyclo, coverage, None);

        assert_eq!(
            test_rec.count, 0,
            "{}: full coverage should need 0 tests",
            description
        );
        assert_eq!(test_rec.formula_used, "fully_covered");
    }
}

#[test]
fn test_spec_109_exact_scenario_reproducibility() {
    // This test documents the EXACT scenario from spec 109 for future reference
    //
    // Context: User reported that debtmap showed:
    //   ACTION: "Add 3 tests for 34% coverage gap, then refactor complexity 33 into 14 functions"
    //   STEP 2: "Currently ~11 of 33 branches are uncovered (66% coverage)"
    //   STEP 3: "Write 11 tests to cover critical uncovered branches first"
    //
    // The issue: 3 != 11 (obvious contradiction)
    //
    // Expected fix: Both should show the same number (~12 tests)

    let cyclo = 33;
    let coverage_percent = 0.661; // 66.1% coverage
    let coverage_gap = 1.0 - coverage_percent; // 33.9% gap

    // Calculate using the unified test calculation
    let test_rec = calculate_tests_needed(cyclo, coverage_percent, None);

    // Expected: ceil(33 × 0.339) = ceil(11.187) = 12 tests
    assert_eq!(test_rec.count, 12);

    // Calculate uncovered branches (should be ~11)
    let uncovered_branches = (cyclo as f64 * coverage_gap).round() as u32;
    assert_eq!(uncovered_branches, 11);

    // Note: The test count (12) and uncovered branches (11) are different because:
    // - Uncovered branches: floor/round of 33 × 0.339 = 11
    // - Tests needed: ceil of 33 × 0.339 = 12
    //
    // This is correct! We need 12 tests to cover 11 uncovered branches (conservative rounding)
    // The bug was NOT about this 11 vs 12 difference (that's fine).
    // The bug was about ACTION saying "3 tests" which came from who-knows-where!

    // Both ACTION and steps should use test_rec.count (12), not uncovered_branches (11)
    let action = format!(
        "Add {} tests for {}% coverage gap",
        test_rec.count,
        (coverage_gap * 100.0).round() as u32
    );

    let steps = vec![
        format!(
            "Currently ~{} of {} branches are uncovered ({}% coverage)",
            uncovered_branches,
            cyclo,
            (coverage_percent * 100.0).round() as u32
        ),
        format!(
            "Write {} tests to cover critical uncovered branches first",
            test_rec.count // Use test count, not uncovered count!
        ),
    ];

    #[cfg(debug_assertions)]
    {
        let result = validate_recommendation_consistency(&action, &steps);
        assert!(result.is_ok(), "Spec 109 fix should pass validation");
    }
}

#[test]
fn test_realistic_recommendation_generation() {
    // Test end-to-end recommendation generation for various real-world scenarios

    struct Scenario {
        name: &'static str,
        cyclo: u32,
        coverage: f64,
        expected_min_tests: u32,
        expected_max_tests: u32,
    }

    let scenarios = vec![
        Scenario {
            name: "Well-tested simple function",
            cyclo: 5,
            coverage: 0.8,
            expected_min_tests: 1,
            expected_max_tests: 2,
        },
        Scenario {
            name: "Untested moderate function",
            cyclo: 20,
            coverage: 0.0,
            expected_min_tests: 8,
            expected_max_tests: 10,
        },
        Scenario {
            name: "Partially tested high complexity (spec 109)",
            cyclo: 33,
            coverage: 0.661,
            expected_min_tests: 11,
            expected_max_tests: 12,
        },
        Scenario {
            name: "Extreme complexity needs property tests",
            cyclo: 70,
            coverage: 0.5,
            expected_min_tests: 10,
            expected_max_tests: 20, // Includes property test recommendations
        },
    ];

    for scenario in scenarios {
        let test_rec = calculate_tests_needed(scenario.cyclo, scenario.coverage, None);

        assert!(
            test_rec.count >= scenario.expected_min_tests
                && test_rec.count <= scenario.expected_max_tests,
            "{}: expected {}-{} tests, got {}",
            scenario.name,
            scenario.expected_min_tests,
            scenario.expected_max_tests,
            test_rec.count
        );

        // Verify recommendation is actionable
        assert!(!test_rec.formula_used.is_empty());
        assert!(!test_rec.rationale.is_empty());
    }
}
