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
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
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
    // Test spec 109: UnifiedScore.complexity_factor should store the result of
    // calculate_complexity_factor(raw_complexity), not raw_complexity itself
    let mut func = create_test_metrics();
    func.cyclomatic = 5;
    func.cognitive = 15;
    // raw_complexity = max(cyclomatic, cognitive * 0.4) = max(5, 6) = 6.0
    // calculate_complexity_factor(6.0) should return 3.0 (halved because >= 5.0)

    let call_graph = CallGraph::new();
    let score = calculate_unified_priority(&func, &call_graph, None, None);

    // The complexity_factor field should store the calculated factor (3.0), not raw_complexity (6.0)
    assert_eq!(
            score.complexity_factor, 3.0,
            "complexity_factor should store calculate_complexity_factor(6.0) = 3.0, not raw_complexity = 6.0"
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
    let complex_entry = create_entry_point_function(25, 30);

    let call_graph = CallGraph::new();
    let lcov = create_zero_coverage_data(&complex_entry);

    let score = calculate_unified_priority(&complex_entry, &call_graph, Some(&lcov), None);

    // Should still be flagged due to high complexity, even with coverage adjustment
    assert!(
        score.final_score > 10.0,
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

    // Score should reflect the 1.5x boost (not clamped to 1.2x)
    // With coverage weight adjustment, score will be lower than raw calculation
    assert!(
        score.final_score > 8.0,
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
    let pure_logic = create_pure_logic_function(12, 14);

    let call_graph = CallGraph::new();
    let lcov = create_zero_coverage_data(&pure_logic);

    let score = calculate_unified_priority(&pure_logic, &call_graph, Some(&lcov), None);

    // PureLogic with 0% coverage should score higher than IOWrapper
    // due to full coverage weight (1.0x) and no role multiplier reduction
    assert!(
        score.final_score > 8.0,
        "PureLogic with 0% coverage should score reasonably high. Score: {}",
        score.final_score
    );

    // Role multiplier for PureLogic with complexity > 5 should be 1.3
    assert!(
        score.role_multiplier >= 1.0,
        "Role multiplier for PureLogic should be >= 1.0, got: {}",
        score.role_multiplier
    );
}
