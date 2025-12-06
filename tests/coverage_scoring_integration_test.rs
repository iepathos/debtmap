/// Integration test for spec 122: Coverage scoring invariant validation
///
/// This test validates the acceptance criteria:
/// "Total debt score with coverage ≤ total debt score without coverage"
///
/// Coverage data should REDUCE total debt scores by dampening well-tested functions,
/// never increase them. This ensures coverage encourages better testing practices.
use debtmap::priority::coverage_propagation::TransitiveCoverage;
use debtmap::priority::scoring::calculation::{
    calculate_base_score_no_coverage, calculate_base_score_with_coverage_multiplier,
    calculate_complexity_factor, calculate_coverage_multiplier, calculate_dependency_factor,
};
use debtmap::priority::{
    CallGraph, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedAnalysis,
    UnifiedAnalysisUtils, UnifiedDebtItem, UnifiedScore,
};
use std::path::PathBuf;

/// Create a test debt item with specified coverage
fn create_debt_item(
    name: &str,
    coverage_pct: f64,
    complexity: u32,
    dependencies: usize,
) -> UnifiedDebtItem {
    let complexity_factor = calculate_complexity_factor(complexity as f64);
    let dependency_factor = calculate_dependency_factor(dependencies);
    let coverage_multiplier = calculate_coverage_multiplier(coverage_pct);

    let final_score = calculate_base_score_with_coverage_multiplier(
        coverage_multiplier,
        complexity_factor,
        dependency_factor,
    );

    UnifiedDebtItem {
        location: Location {
            file: PathBuf::from("src/test.rs"),
            function: name.to_string(),
            line: 100,
        },
        debt_type: DebtType::TestingGap {
            coverage: coverage_pct,
            cyclomatic: complexity,
            cognitive: complexity,
        },
        unified_score: UnifiedScore {
            complexity_factor,
            coverage_factor: coverage_multiplier * 10.0, // For display compatibility
            dependency_factor,
            role_multiplier: 1.0,
            final_score,
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
            purity_factor: None,
            refactorability_factor: None,
            pattern_factor: None,
        },
        function_role: FunctionRole::PureLogic,
        recommendation: debtmap::priority::ActionableRecommendation {
            primary_action: "Add tests".to_string(),
            rationale: "Testing gap".to_string(),
            implementation_steps: vec![],
            related_items: vec![],
            steps: None,
            estimated_effort_hours: None,
        },
        expected_impact: ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
        },
        transitive_coverage: Some(TransitiveCoverage {
            direct: coverage_pct,
            transitive: coverage_pct,
            propagated_from: vec![],
            uncovered_lines: vec![],
        }),
        file_context: None,
        upstream_dependencies: dependencies,
        downstream_dependencies: 0,
        upstream_callers: vec![],
        downstream_callees: vec![],
        nesting_depth: 1,
        function_length: 20,
        cyclomatic_complexity: complexity,
        cognitive_complexity: complexity,
        is_pure: Some(true),
        purity_confidence: Some(0.9),
        purity_level: None,
        entropy_details: None,
        entropy_adjusted_cyclomatic: None,
        entropy_adjusted_cognitive: None,
        entropy_dampening_factor: None,
        god_object_indicators: None,
        tier: None,
        function_context: None,
        context_confidence: None,
        contextual_recommendation: None,
        pattern_analysis: None,
        context_multiplier: None,
        context_type: None,
        language_specific: None, // spec 190
        detected_pattern: None,
        contextual_risk: None, // spec 203
        file_line_count: None,
    }
}

#[test]
fn test_coverage_scoring_invariant_single_item() {
    // Test with a single item: score with coverage should be ≤ score without coverage
    let complexity = 15;
    let dependencies = 5;

    // Same item, two coverage levels
    let untested = create_debt_item("untested_fn", 0.0, complexity, dependencies);
    let well_tested = create_debt_item("tested_fn", 0.9, complexity, dependencies);

    // Coverage should reduce the score (higher coverage = lower score)
    assert!(
        well_tested.unified_score.final_score <= untested.unified_score.final_score,
        "Well-tested function should have lower score. Tested: {}, Untested: {}",
        well_tested.unified_score.final_score,
        untested.unified_score.final_score
    );
}

#[test]
fn test_coverage_scoring_invariant_total_analysis() {
    // Create analysis WITHOUT coverage data
    let call_graph_no_cov = CallGraph::new();
    let mut analysis_no_coverage = UnifiedAnalysis::new(call_graph_no_cov);
    analysis_no_coverage.has_coverage_data = false;

    // Add items with NO coverage information (should use base scoring)
    for i in 0..10 {
        let complexity = 10 + i * 2;
        let dependencies = i;

        // When no coverage data, we calculate base score differently
        let complexity_factor = calculate_complexity_factor(complexity as f64);
        let dependency_factor = calculate_dependency_factor(dependencies);
        let base_score = calculate_base_score_no_coverage(complexity_factor, dependency_factor);

        let item = UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("src/test.rs"),
                function: format!("function_{}", i),
                line: 100 + i,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: complexity as u32,
                cognitive: complexity as u32,
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor,
                coverage_factor: 0.0, // No coverage data
                dependency_factor,
                role_multiplier: 1.0,
                final_score: base_score,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: debtmap::priority::ActionableRecommendation {
                primary_action: "Refactor".to_string(),
                rationale: "High complexity".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None, // No coverage
            file_context: None,
            upstream_dependencies: dependencies,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 3,
            function_length: 20,
            cyclomatic_complexity: complexity as u32,
            cognitive_complexity: complexity as u32,
            is_pure: Some(true),
            purity_confidence: Some(0.9),
            purity_level: None,
            entropy_details: None,
            entropy_adjusted_cyclomatic: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None, // spec 203
            file_line_count: None,
        };
        analysis_no_coverage.add_item(item);
    }

    // Create analysis WITH coverage data (same functions, various coverage levels)
    let call_graph_with_cov = CallGraph::new();
    let mut analysis_with_coverage = UnifiedAnalysis::new(call_graph_with_cov);
    analysis_with_coverage.has_coverage_data = true;

    for i in 0..10 {
        let complexity = 10 + i * 2;
        let dependencies = i;
        // Vary coverage: some well-tested, some not
        let coverage = (i as f64) / 10.0; // 0%, 10%, 20%, ..., 90%

        let item = create_debt_item(
            &format!("function_{}", i),
            coverage,
            complexity as u32,
            dependencies,
        );
        analysis_with_coverage.add_item(item);
    }

    // Calculate total scores
    let total_without_coverage: f64 = analysis_no_coverage
        .items
        .iter()
        .map(|item| item.unified_score.final_score)
        .sum();

    let total_with_coverage: f64 = analysis_with_coverage
        .items
        .iter()
        .map(|item| item.unified_score.final_score)
        .sum();

    // INVARIANT: Total debt score WITH coverage should be ≤ total WITHOUT coverage
    // Because coverage dampens scores for well-tested code
    assert!(
        total_with_coverage <= total_without_coverage,
        "Total debt score with coverage ({:.2}) should be ≤ total without coverage ({:.2}). \
         Coverage should only reduce scores, never increase them.",
        total_with_coverage,
        total_without_coverage
    );

    // Additional validation: scores should be substantially different
    // (not just equal due to a bug)
    let score_reduction = total_without_coverage - total_with_coverage;
    assert!(
        score_reduction > 0.0,
        "Coverage data should meaningfully reduce total score. Reduction: {:.2}",
        score_reduction
    );
}

#[test]
fn test_coverage_multiplier_dampens_scores() {
    // Unit test: Verify that the coverage multiplier actually dampens scores
    let complexity_factor = calculate_complexity_factor(20.0);
    let dependency_factor = calculate_dependency_factor(10);

    // Score with no coverage (100% of base)
    let score_no_coverage = calculate_base_score_with_coverage_multiplier(
        1.0, // 0% coverage = full multiplier
        complexity_factor,
        dependency_factor,
    );

    // Score with partial coverage (50% dampened)
    let score_partial_coverage = calculate_base_score_with_coverage_multiplier(
        0.5, // 50% coverage = half multiplier
        complexity_factor,
        dependency_factor,
    );

    // Score with full coverage (maximally dampened)
    let score_full_coverage = calculate_base_score_with_coverage_multiplier(
        0.0, // 100% coverage = no multiplier
        complexity_factor,
        dependency_factor,
    );

    // Verify ordering: no_coverage > partial > full
    assert!(
        score_no_coverage > score_partial_coverage,
        "Partial coverage should dampen score below no coverage"
    );
    assert!(
        score_partial_coverage > score_full_coverage,
        "Full coverage should dampen score below partial coverage"
    );
    assert!(
        score_full_coverage < 1.0,
        "Full coverage should result in near-zero score"
    );
}

#[test]
fn test_coverage_multiplier_mathematical_properties() {
    // Property test: Coverage multiplier is monotonically decreasing
    let coverage_levels = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
    let multipliers: Vec<f64> = coverage_levels
        .iter()
        .map(|&cov| calculate_coverage_multiplier(cov))
        .collect();

    // Verify monotonic decrease: higher coverage = lower multiplier
    for i in 1..multipliers.len() {
        assert!(
            multipliers[i] <= multipliers[i - 1],
            "Coverage multiplier should decrease with coverage. \
             At {}% coverage: {}, at {}% coverage: {}",
            coverage_levels[i - 1] * 100.0,
            multipliers[i - 1],
            coverage_levels[i] * 100.0,
            multipliers[i]
        );
    }

    // Boundary conditions
    assert_eq!(
        calculate_coverage_multiplier(0.0),
        1.0,
        "0% coverage should have full multiplier (1.0)"
    );
    assert_eq!(
        calculate_coverage_multiplier(1.0),
        0.0,
        "100% coverage should have zero multiplier (0.0)"
    );
}
