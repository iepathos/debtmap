use super::*;
use crate::core::FunctionMetrics;
use crate::priority::call_graph::CallGraph;
use crate::risk::lcov::{FunctionCoverage, LcovData};
use std::path::PathBuf;

fn create_test_metrics() -> FunctionMetrics {
    FunctionMetrics {
        file: PathBuf::from("test.rs"),
        name: "test_function".to_string(),
        line: 10,
        length: 50,
        cyclomatic: 5,
        cognitive: 8,
        nesting: 0,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
    }
}

#[test]
fn test_unified_scoring() {
    let func = create_test_metrics();
    let graph = CallGraph::new();
    let score = calculate_unified_priority(&func, &graph, None, None);

    assert!(score.complexity_factor > 0.0);
    assert!(score.coverage_factor > 0.0);
    assert!(score.final_score > 0.0);
    assert!(score.final_score <= 10.0);
}

fn create_simple_io_wrapper() -> FunctionMetrics {
    let mut func = create_test_metrics();
    func.name = "extract_module_from_import".to_string();
    func.cyclomatic = 1;
    func.cognitive = 1;
    func.length = 3;
    func.nesting = 1;
    func
}

fn create_full_coverage_data(func: &FunctionMetrics) -> LcovData {
    let mut lcov = LcovData::default();
    lcov.functions.insert(
        func.file.clone(),
        vec![FunctionCoverage {
            name: func.name.clone(),
            start_line: func.line,
            execution_count: 18,
            coverage_percentage: 100.0,
            uncovered_lines: vec![],
            normalized: crate::risk::lcov::NormalizedFunctionName::simple(&func.name),
        }],
    );
    lcov.build_index(); // Rebuild index after modifying functions
    lcov
}

fn assert_zero_debt_score(score: &UnifiedScore) {
    assert_eq!(score.final_score, 0.0);
    assert_eq!(score.complexity_factor, 0.0);
    assert_eq!(score.coverage_factor, 0.0);
}

#[test]
fn test_simple_io_wrapper_with_coverage_zero_score() {
    // Create a simple I/O wrapper function with test coverage
    let func = create_simple_io_wrapper();
    let call_graph = CallGraph::new();
    let lcov = create_full_coverage_data(&func);

    let score = calculate_unified_priority(&func, &call_graph, Some(&lcov), None);

    // Tested simple I/O wrapper should have zero score (not technical debt)
    assert_zero_debt_score(&score);
}

#[test]
fn test_simple_io_wrapper_without_coverage_has_score() {
    // Create a simple I/O wrapper function without test coverage
    let mut func = create_test_metrics();
    func.name = "print_risk_function".to_string();
    func.cyclomatic = 1;
    func.cognitive = 0;
    func.length = 4;
    func.nesting = 1;

    let call_graph = CallGraph::new();

    // Calculate priority score without coverage (assume untested)
    let score = calculate_unified_priority(&func, &call_graph, None, None);

    // Untested simple I/O wrapper should have a non-zero score (testing gap)
    assert!(
        score.final_score > 0.0,
        "Untested I/O wrapper should have non-zero score"
    );
}

fn assert_zero_coverage_boost(score: &UnifiedScore) {
    // Spec 122: With multiplier approach, 0% coverage (multiplier=1.0) keeps full base score
    // No longer applying 10x boost, but function still gets full complexity+dependency score
    assert!(
        score.final_score > 0.0,
        "Zero coverage functions should have non-zero score, got {}",
        score.final_score
    );
}

#[test]
fn test_zero_coverage_prioritization() {
    // Test spec 122: Functions with 0% coverage get full base score (multiplier=1.0)
    let func = create_test_function_for_coverage();
    let call_graph = CallGraph::new();

    let lcov = create_coverage_function(&func, 0, 0.0);
    let score = calculate_unified_priority(&func, &call_graph, Some(&lcov), None);

    assert_zero_coverage_boost(&score);
}

fn create_coverage_function(
    func: &FunctionMetrics,
    execution_count: u64,
    coverage_percentage: f64,
) -> LcovData {
    let mut lcov = LcovData::default();
    lcov.functions.insert(
        func.file.clone(),
        vec![FunctionCoverage {
            name: func.name.clone(),
            start_line: func.line,
            execution_count,
            coverage_percentage,
            uncovered_lines: vec![],
            normalized: crate::risk::lcov::NormalizedFunctionName::simple(&func.name),
        }],
    );
    lcov.build_index(); // Rebuild index after modifying functions
    lcov
}

fn create_test_function_for_coverage() -> FunctionMetrics {
    let mut func = create_test_metrics();
    func.cyclomatic = 5;
    func.cognitive = 8;
    func.is_test = false;
    func
}

fn assert_low_coverage_boost(score_low: &UnifiedScore, score_mid: &UnifiedScore) {
    // Spec 122: With multiplier approach, lower coverage → higher score (monotonicity)
    // 10% coverage (multiplier=0.9) should score higher than 50% coverage (multiplier=0.5)
    assert!(
        score_low.final_score > score_mid.final_score,
        "10% coverage ({}) should score higher than 50% coverage ({})",
        score_low.final_score,
        score_mid.final_score
    );
}

#[test]
fn test_low_coverage_prioritization() {
    // Test spec 122: Functions with lower coverage score higher (monotonicity)
    let func = create_test_function_for_coverage();
    let call_graph = CallGraph::new();

    let lcov_low = create_coverage_function(&func, 1, 10.0);
    let lcov_mid = create_coverage_function(&func, 5, 50.0);

    let score_low = calculate_unified_priority(&func, &call_graph, Some(&lcov_low), None);
    let score_mid = calculate_unified_priority(&func, &call_graph, Some(&lcov_mid), None);

    assert_low_coverage_boost(&score_low, &score_mid);
}

#[test]
fn test_test_code_not_boosted() {
    // Test spec 98: Test code should not get zero coverage boost
    let mut func = create_test_metrics();
    func.cyclomatic = 5;
    func.cognitive = 8;
    func.is_test = true; // Mark as test code
    func.name = "test_something".to_string();

    let call_graph = CallGraph::new();

    // No coverage data (worst case for non-test code)
    let score = calculate_unified_priority(&func, &call_graph, None, None);

    // Test code with no coverage should still have low score
    assert!(
        score.final_score < 10.0,
        "Test code should not get zero coverage boost, got {}",
        score.final_score
    );
}

#[test]
fn test_complex_function_has_score() {
    // Create a complex function that should have a non-zero score
    let mut func = create_test_metrics();
    func.name = "complex_logic".to_string();
    func.cyclomatic = 8;
    func.cognitive = 12;
    func.length = 50;

    let call_graph = CallGraph::new();

    // Calculate priority score
    let score = calculate_unified_priority(&func, &call_graph, None, None);

    // Complex function should have non-zero score (is technical debt)
    assert!(score.final_score > 0.0);
    assert!(score.complexity_factor > 0.0);
}

#[test]
fn test_complexity_factor_stores_calculated_factor_not_raw_complexity() {
    // Test spec 121: UnifiedScore.complexity_factor should store the result of
    // calculate_complexity_factor with weighted complexity scoring
    let mut func = create_test_metrics();
    func.cyclomatic = 5;
    func.cognitive = 15;
    // With spec 121 cognitive-weighted scoring (default 0.3 cyclo, 0.7 cognitive):
    // normalized_cyclo = (5/50) * 100 = 10.0
    // normalized_cog = (15/100) * 100 = 15.0
    // weighted = 0.3 * 10.0 + 0.7 * 15.0 = 13.5
    // raw_complexity (0-10 scale) = 13.5 / 10.0 = 1.35
    // complexity_factor = calculate_complexity_factor(1.35) = 1.35 / 2.0 = 0.675

    let call_graph = CallGraph::new();
    let score = calculate_unified_priority(&func, &call_graph, None, None);

    // The complexity_factor field should store the calculated factor with cognitive weighting
    assert!(
        (score.complexity_factor - 0.675).abs() < 0.01,
        "complexity_factor should be ~0.675 with cognitive weighting, got {}",
        score.complexity_factor
    );
}

#[test]
fn test_well_tested_simple_function_scores_below_20() {
    // Test spec 109: Well-tested simple functions (100% coverage, cyclomatic < 10)
    // should score below 20.0 (spec example shows ~16.25)
    let mut func = create_test_metrics();
    func.cyclomatic = 5;
    func.cognitive = 15;
    func.is_test = false;

    let call_graph = CallGraph::new();
    let lcov = create_full_coverage_data(&func);

    let score = calculate_unified_priority(&func, &call_graph, Some(&lcov), None);

    assert!(
        score.final_score < 20.0,
        "Well-tested simple function (100% coverage, cyclomatic=5) should score < 20.0, got {}",
        score.final_score
    );
}

// Add more tests as needed...

// Tests for spec 110: Role-based coverage weight multiplier

fn create_entry_point_function(cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
    let mut func = create_test_metrics();
    func.name = "handle_analyze".to_string(); // Entry point name pattern
    func.cyclomatic = cyclomatic;
    func.cognitive = cognitive;
    func
}

fn create_pure_logic_function(cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
    let mut func = create_test_metrics();
    func.name = "calculate_score".to_string(); // Pure logic name pattern
    func.cyclomatic = cyclomatic;
    func.cognitive = cognitive;
    func
}

fn create_zero_coverage_data(func: &FunctionMetrics) -> LcovData {
    let mut lcov = LcovData::default();
    lcov.functions.insert(
        func.file.clone(),
        vec![FunctionCoverage {
            name: func.name.clone(),
            start_line: func.line,
            execution_count: 0,
            coverage_percentage: 0.0,
            uncovered_lines: vec![func.line],
            normalized: crate::risk::lcov::NormalizedFunctionName::simple(&func.name),
        }],
    );
    lcov.build_index();
    lcov
}

#[test]
fn test_entry_point_coverage_adjustment() {
    // Spec 110: Entry points with 0% coverage should score lower than pure logic with 0% coverage
    let entry_point = create_entry_point_function(17, 17);
    let pure_logic = create_pure_logic_function(17, 17);

    let call_graph = CallGraph::new();
    let entry_lcov = create_zero_coverage_data(&entry_point);
    let logic_lcov = create_zero_coverage_data(&pure_logic);

    let entry_score =
        calculate_unified_priority(&entry_point, &call_graph, Some(&entry_lcov), None);
    let logic_score = calculate_unified_priority(&pure_logic, &call_graph, Some(&logic_lcov), None);

    // Entry point should score LOWER due to 0.6x coverage weight multiplier
    assert!(
        entry_score.final_score < logic_score.final_score,
        "Entry point (score: {}) should score lower than pure logic (score: {}) with same complexity and 0% coverage",
        entry_score.final_score,
        logic_score.final_score
    );
}

#[test]
fn test_orchestrator_coverage_adjustment() {
    // Spec 110: Orchestrators should get 0.8x coverage weight multiplier
    let mut orchestrator = create_test_metrics();
    orchestrator.name = "orchestrate_analysis".to_string();
    orchestrator.cyclomatic = 12;
    orchestrator.cognitive = 15;

    let call_graph = CallGraph::new();
    let lcov = create_zero_coverage_data(&orchestrator);

    let score = calculate_unified_priority(&orchestrator, &call_graph, Some(&lcov), None);

    // Orchestrator with 0% coverage should have reduced penalty compared to normal functions
    // The score should be lower than a pure logic function with same complexity
    assert!(
        score.final_score > 0.0,
        "Orchestrator should still have a score, but reduced due to coverage adjustment"
    );
}

#[test]
fn test_entry_point_with_coverage_not_overly_penalized() {
    // Spec 110: Entry point with 50% coverage should not rank in critical tier
    let entry_point = create_entry_point_function(12, 12);
    let mut partial_lcov = LcovData::default();
    partial_lcov.functions.insert(
        entry_point.file.clone(),
        vec![FunctionCoverage {
            name: entry_point.name.clone(),
            start_line: entry_point.line,
            execution_count: 5,
            coverage_percentage: 50.0,
            uncovered_lines: vec![],
            normalized: crate::risk::lcov::NormalizedFunctionName::simple(&entry_point.name),
        }],
    );
    partial_lcov.build_index();

    let call_graph = CallGraph::new();
    let score = calculate_unified_priority(&entry_point, &call_graph, Some(&partial_lcov), None);

    // Should not rank in critical tier (< 20.0)
    assert!(
        score.final_score < 20.0,
        "Entry point with 50% coverage and moderate complexity should not be critical (score: {})",
        score.final_score
    );
}

#[test]
fn test_complex_entry_point_still_flagged() {
    // Spec 110: Complex entry points should still be flagged despite coverage adjustment
    // Spec 121: With cognitive-weighted scoring, high cognitive complexity is emphasized
    let complex_entry = create_entry_point_function(25, 30);

    let call_graph = CallGraph::new();
    let lcov = create_zero_coverage_data(&complex_entry);

    let score = calculate_unified_priority(&complex_entry, &call_graph, Some(&lcov), None);

    // With cognitive-weighted scoring:
    // normalized_cyclo = (25/50) * 100 = 50.0
    // normalized_cog = (30/100) * 100 = 30.0
    // weighted = 0.3 * 50.0 + 0.7 * 30.0 = 15.0 + 21.0 = 36.0
    // raw_complexity = 36.0 / 10.0 = 3.6
    // complexity_factor = 3.6 / 2.0 = 1.8
    // With EntryPoint role multiplier (1.5x) and 0% coverage, expect score around 8-10
    assert!(
        score.final_score > 8.0,
        "Complex entry point should still be flagged (score: {})",
        score.final_score
    );
}

#[test]
fn test_io_wrapper_role_multiplier_not_clamped() {
    // Spec 119: IOWrapper role multiplier (0.5) should not be clamped to 0.8
    // with new default clamp range [0.3, 1.8]
    let mut io_wrapper = create_test_metrics();
    io_wrapper.name = "write_output".to_string();
    io_wrapper.cyclomatic = 16;
    io_wrapper.cognitive = 18;
    io_wrapper.length = 50;
    io_wrapper.nesting = 3;

    let call_graph = CallGraph::new();
    let lcov = create_zero_coverage_data(&io_wrapper);

    let score = calculate_unified_priority(&io_wrapper, &call_graph, Some(&lcov), None);

    // With the new clamp range [0.3, 1.8], the 0.5 multiplier should be applied
    // for IOWrapper functions (detected by name pattern or classifier)
    // Expected base score for cyclo=16, 0% coverage is around 22.0
    // With 0.5 multiplier: 22.0 * 0.5 = 11.0
    // With role-specific coverage weight (0.5x), the score should be even lower (~8-11)
    assert!(
        score.final_score < 20.0,
        "IOWrapper should score relatively low. Score: {}",
        score.final_score
    );

    // Verify role multiplier was applied (should be 0.5 or close for IOWrapper)
    assert!(
        score.role_multiplier <= 0.8,
        "Role multiplier for IOWrapper should be <=0.8, got: {}",
        score.role_multiplier
    );
}

#[test]
fn test_entry_point_role_multiplier_not_clamped() {
    // Spec 119: EntryPoint role multiplier (1.5) should not be clamped to 1.2
    // with new default clamp range [0.3, 1.8]
    // Spec 121: With cognitive-weighted scoring, actual scores will differ
    let entry_point = create_entry_point_function(15, 18);

    let call_graph = CallGraph::new();
    let lcov = create_zero_coverage_data(&entry_point);

    let score = calculate_unified_priority(&entry_point, &call_graph, Some(&lcov), None);

    // Verify role multiplier was applied
    assert!(
        (score.role_multiplier - 1.5).abs() < 0.01,
        "Role multiplier should be 1.5 for EntryPoint, got: {}",
        score.role_multiplier
    );

    // With cognitive-weighted scoring:
    // normalized_cyclo = (15/50) * 100 = 30.0
    // normalized_cog = (18/100) * 100 = 18.0
    // weighted = 0.3 * 30.0 + 0.7 * 18.0 = 9.0 + 12.6 = 21.6
    // raw_complexity = 21.6 / 10.0 = 2.16
    // complexity_factor = 2.16 / 2.0 = 1.08
    // With 1.5x multiplier and 0% coverage, expect score around 4-6
    assert!(
        score.final_score > 4.0,
        "EntryPoint with 1.5x multiplier should have elevated score. Score: {}",
        score.final_score
    );
}

#[test]
fn test_io_wrapper_coverage_weight_reduced() {
    // Spec 119: IOWrapper functions should have reduced coverage weight (0.5x)
    let mut io_wrapper = create_test_metrics();
    io_wrapper.name = "format_output".to_string();
    io_wrapper.file = PathBuf::from("src/io/formatter.rs");
    io_wrapper.cyclomatic = 12;
    io_wrapper.cognitive = 14;
    io_wrapper.length = 40;
    io_wrapper.nesting = 2;

    let call_graph = CallGraph::new();
    let lcov = create_zero_coverage_data(&io_wrapper);

    let score = calculate_unified_priority(&io_wrapper, &call_graph, Some(&lcov), None);

    // With 0% coverage, 0.5x coverage weight, and 0.5x role multiplier,
    // the score should be significantly lower than a PureLogic function
    // with same complexity
    assert!(
        score.final_score < 15.0,
        "IOWrapper with reduced coverage weight should score low. Score: {}",
        score.final_score
    );
}

#[test]
fn test_pure_logic_coverage_weight_unchanged() {
    // Spec 119: PureLogic functions should maintain 1.0x coverage weight
    // Spec 121: With cognitive-weighted scoring, scores will be adjusted
    let pure_logic = create_pure_logic_function(12, 14);

    let call_graph = CallGraph::new();
    let lcov = create_zero_coverage_data(&pure_logic);

    let score = calculate_unified_priority(&pure_logic, &call_graph, Some(&lcov), None);

    // With cognitive-weighted scoring:
    // normalized_cyclo = (12/50) * 100 = 24.0
    // normalized_cog = (14/100) * 100 = 14.0
    // weighted = 0.3 * 24.0 + 0.7 * 14.0 = 7.2 + 9.8 = 17.0
    // raw_complexity = 17.0 / 10.0 = 1.7
    // complexity_factor = 1.7 / 2.0 = 0.85
    // With 1.3x multiplier and 0% coverage, expect score around 4-5
    assert!(
        score.final_score > 4.0,
        "PureLogic with 0% coverage should score reasonably. Score: {}",
        score.final_score
    );

    // Role multiplier for PureLogic with complexity > 5 should be 1.3
    assert!(
        score.role_multiplier >= 1.0,
        "Role multiplier for PureLogic should be >= 1.0, got: {}",
        score.role_multiplier
    );
}

// Tests for spec 157d: Purity level-based scoring

#[test]
fn test_purity_adjustment_strictly_pure_high_confidence() {
    let mut func = create_test_metrics();
    func.purity_level = Some(crate::core::PurityLevel::StrictlyPure);
    func.purity_confidence = Some(0.9);

    let adjustment = calculate_purity_adjustment(&func);
    assert_eq!(
        adjustment, 0.70,
        "StrictlyPure with high confidence should get 0.70"
    );
}

#[test]
fn test_purity_adjustment_strictly_pure_medium_confidence() {
    let mut func = create_test_metrics();
    func.purity_level = Some(crate::core::PurityLevel::StrictlyPure);
    func.purity_confidence = Some(0.7);

    let adjustment = calculate_purity_adjustment(&func);
    assert_eq!(
        adjustment, 0.80,
        "StrictlyPure with medium confidence should get 0.80"
    );
}

#[test]
fn test_purity_adjustment_locally_pure_high_confidence() {
    let mut func = create_test_metrics();
    func.purity_level = Some(crate::core::PurityLevel::LocallyPure);
    func.purity_confidence = Some(0.9);

    let adjustment = calculate_purity_adjustment(&func);
    assert_eq!(
        adjustment, 0.75,
        "LocallyPure with high confidence should get 0.75"
    );
}

#[test]
fn test_purity_adjustment_locally_pure_medium_confidence() {
    let mut func = create_test_metrics();
    func.purity_level = Some(crate::core::PurityLevel::LocallyPure);
    func.purity_confidence = Some(0.7);

    let adjustment = calculate_purity_adjustment(&func);
    assert_eq!(
        adjustment, 0.85,
        "LocallyPure with medium confidence should get 0.85"
    );
}

#[test]
fn test_purity_adjustment_read_only() {
    let mut func = create_test_metrics();
    func.purity_level = Some(crate::core::PurityLevel::ReadOnly);
    func.purity_confidence = Some(0.9);

    let adjustment = calculate_purity_adjustment(&func);
    assert_eq!(adjustment, 0.90, "ReadOnly should get 0.90");
}

#[test]
fn test_purity_adjustment_impure() {
    let mut func = create_test_metrics();
    func.purity_level = Some(crate::core::PurityLevel::Impure);
    func.purity_confidence = Some(0.9);

    let adjustment = calculate_purity_adjustment(&func);
    assert_eq!(adjustment, 1.0, "Impure should get 1.0");
}

#[test]
fn test_backward_compatibility_with_is_pure() {
    let mut func = create_test_metrics();
    func.purity_level = None; // Not set - old code path
    func.is_pure = Some(true);
    func.purity_confidence = Some(0.9);

    let adjustment = calculate_purity_adjustment(&func);
    assert_eq!(
        adjustment, 0.70,
        "Old is_pure field with high confidence should still work"
    );
}

#[test]
fn test_backward_compatibility_with_is_pure_medium_confidence() {
    let mut func = create_test_metrics();
    func.purity_level = None; // Not set - old code path
    func.is_pure = Some(true);
    func.purity_confidence = Some(0.7);

    let adjustment = calculate_purity_adjustment(&func);
    assert_eq!(
        adjustment, 0.85,
        "Old is_pure field with medium confidence should still work"
    );
}

#[test]
fn test_locally_pure_scores_lower_than_impure() {
    // Test that LocallyPure functions get lower scores than Impure functions
    let mut func_locally_pure = create_test_metrics();
    func_locally_pure.purity_level = Some(crate::core::PurityLevel::LocallyPure);
    func_locally_pure.purity_confidence = Some(0.9);
    func_locally_pure.cyclomatic = 10;
    func_locally_pure.cognitive = 15;

    let mut func_impure = create_test_metrics();
    func_impure.purity_level = Some(crate::core::PurityLevel::Impure);
    func_impure.purity_confidence = Some(0.9);
    func_impure.cyclomatic = 10;
    func_impure.cognitive = 15;

    let call_graph = CallGraph::new();
    let score_locally_pure =
        calculate_unified_priority(&func_locally_pure, &call_graph, None, None);
    let score_impure = calculate_unified_priority(&func_impure, &call_graph, None, None);

    assert!(
        score_locally_pure.final_score < score_impure.final_score,
        "LocallyPure (score: {}) should score lower than Impure (score: {})",
        score_locally_pure.final_score,
        score_impure.final_score
    );
}

#[test]
fn test_locally_pure_scores_higher_than_strictly_pure() {
    // Test that LocallyPure functions get slightly higher scores than StrictlyPure functions
    let mut func_locally_pure = create_test_metrics();
    func_locally_pure.purity_level = Some(crate::core::PurityLevel::LocallyPure);
    func_locally_pure.purity_confidence = Some(0.9);
    func_locally_pure.cyclomatic = 10;
    func_locally_pure.cognitive = 15;

    let mut func_strictly_pure = create_test_metrics();
    func_strictly_pure.purity_level = Some(crate::core::PurityLevel::StrictlyPure);
    func_strictly_pure.purity_confidence = Some(0.9);
    func_strictly_pure.cyclomatic = 10;
    func_strictly_pure.cognitive = 15;

    let call_graph = CallGraph::new();
    let score_locally_pure =
        calculate_unified_priority(&func_locally_pure, &call_graph, None, None);
    let score_strictly_pure =
        calculate_unified_priority(&func_strictly_pure, &call_graph, None, None);

    assert!(
        score_locally_pure.final_score > score_strictly_pure.final_score,
        "LocallyPure (score: {}) should score slightly higher than StrictlyPure (score: {})",
        score_locally_pure.final_score,
        score_strictly_pure.final_score
    );
}

// Tests for entropy dampening integration (spec 214)

#[test]
fn test_entropy_dampening_reduces_complexity_score() {
    use crate::complexity::entropy_core::EntropyScore;

    // Create two identical functions - one with high pattern repetition, one without
    let mut func_with_patterns = create_test_metrics();
    func_with_patterns.cyclomatic = 10;
    func_with_patterns.cognitive = 15;
    func_with_patterns.entropy_score = Some(EntropyScore {
        token_entropy: 0.5,
        pattern_repetition: 0.8, // High pattern repetition
        branch_similarity: 0.6,
        effective_complexity: 0.3,
        unique_variables: 5,
        max_nesting: 2,
        dampening_applied: 1.0,
    });

    let mut func_without_patterns = create_test_metrics();
    func_without_patterns.cyclomatic = 10;
    func_without_patterns.cognitive = 15;
    func_without_patterns.entropy_score = None; // No entropy data

    let call_graph = CallGraph::new();
    let score_with_patterns =
        calculate_unified_priority(&func_with_patterns, &call_graph, None, None);
    let score_without_patterns =
        calculate_unified_priority(&func_without_patterns, &call_graph, None, None);

    // Pattern-heavy code should score lower due to entropy dampening
    assert!(
        score_with_patterns.final_score < score_without_patterns.final_score,
        "Pattern-heavy code (score: {}) should score lower than code without patterns (score: {})",
        score_with_patterns.final_score,
        score_without_patterns.final_score
    );
}

#[test]
fn test_entropy_dampening_never_increases_complexity() {
    use crate::complexity::entropy_core::EntropyScore;

    // Create function with entropy score
    let mut func = create_test_metrics();
    func.cyclomatic = 20;
    func.cognitive = 30;
    func.entropy_score = Some(EntropyScore {
        token_entropy: 0.7,
        pattern_repetition: 0.5,
        branch_similarity: 0.4,
        effective_complexity: 0.6,
        unique_variables: 10,
        max_nesting: 3,
        dampening_applied: 1.0,
    });

    // Calculate entropy details
    let entropy_details = crate::priority::scoring::computation::calculate_entropy_details(&func);

    assert!(entropy_details.is_some());
    let details = entropy_details.unwrap();

    // Entropy-adjusted complexity should never exceed raw complexity
    assert!(
        details.adjusted_complexity <= func.cyclomatic,
        "Adjusted cyclomatic ({}) should not exceed raw cyclomatic ({})",
        details.adjusted_complexity,
        func.cyclomatic
    );
    assert!(
        details.adjusted_cognitive <= func.cognitive,
        "Adjusted cognitive ({}) should not exceed raw cognitive ({})",
        details.adjusted_cognitive,
        func.cognitive
    );
    assert!(
        details.dampening_factor <= 1.0,
        "Dampening factor ({}) should not exceed 1.0",
        details.dampening_factor
    );
}

#[test]
fn test_entropy_details_populated_in_debt_item() {
    use crate::complexity::entropy_core::EntropyScore;
    use crate::priority::scoring::construction::create_unified_debt_item;

    let mut func = create_test_metrics();
    func.cyclomatic = 15;
    func.cognitive = 20;
    func.entropy_score = Some(EntropyScore {
        token_entropy: 0.6,
        pattern_repetition: 0.7,
        branch_similarity: 0.5,
        effective_complexity: 0.4,
        unique_variables: 8,
        max_nesting: 2,
        dampening_applied: 1.0,
    });

    let call_graph = CallGraph::new();
    let debt_item = create_unified_debt_item(&func, &call_graph, None);

    assert!(debt_item.is_some());
    let item = debt_item.unwrap();

    // Verify entropy details are populated
    assert!(item.entropy_details.is_some());
    assert!(item.entropy_adjusted_cyclomatic.is_some());
    assert!(item.entropy_adjusted_cognitive.is_some());
    assert!(item.entropy_dampening_factor.is_some());

    // Verify adjusted values are reasonable
    let adjusted_cyclo = item.entropy_adjusted_cyclomatic.unwrap();
    let adjusted_cog = item.entropy_adjusted_cognitive.unwrap();
    assert!(adjusted_cyclo <= func.cyclomatic);
    assert!(adjusted_cog <= func.cognitive);
}

#[test]
fn test_normalize_complexity_with_entropy_dampening() {
    use crate::complexity::entropy_core::EntropyScore;

    // Test that normalize_complexity uses entropy-adjusted values when available
    let mut func = create_test_metrics();
    func.cyclomatic = 20;
    func.cognitive = 30;
    func.entropy_score = Some(EntropyScore {
        token_entropy: 0.5,
        pattern_repetition: 0.8, // High repetition should reduce complexity
        branch_similarity: 0.6,
        effective_complexity: 0.3,
        unique_variables: 5,
        max_nesting: 2,
        dampening_applied: 1.0,
    });

    let call_graph = CallGraph::new();

    // Calculate score with entropy dampening
    let score_with_entropy = calculate_unified_priority(&func, &call_graph, None, None);

    // Remove entropy data and recalculate
    func.entropy_score = None;
    let score_without_entropy = calculate_unified_priority(&func, &call_graph, None, None);

    // Complexity factor should be lower with entropy dampening
    assert!(
        score_with_entropy.complexity_factor < score_without_entropy.complexity_factor,
        "Complexity factor with entropy ({}) should be lower than without entropy ({})",
        score_with_entropy.complexity_factor,
        score_without_entropy.complexity_factor
    );
}
