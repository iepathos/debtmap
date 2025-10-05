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
        assert!(
            score.final_score >= 50.0,
            "Zero coverage functions should score at least 50.0, got {}",
            score.final_score
        );
    }

    #[test]
    fn test_zero_coverage_prioritization() {
        // Test spec 98: Functions with 0% coverage get 10x boost
        let func = create_test_function_for_coverage();
        let call_graph = CallGraph::new();
        
        let lcov = create_coverage_function(&func, 0, 0.0);
        let score = calculate_unified_priority(&func, &call_graph, Some(&lcov), None);
        
        assert_zero_coverage_boost(&score);
    }
    
    fn create_coverage_function(func: &FunctionMetrics, execution_count: u64, coverage_percentage: f64) -> LcovData {
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
        assert!(
            score_low.final_score > score_mid.final_score * 2.0,
            "10% coverage ({}) should score much higher than 50% coverage ({})",
            score_low.final_score,
            score_mid.final_score
        );
    }

    #[test]
    fn test_low_coverage_prioritization() {
        // Test spec 98: Functions with <20% coverage get 5x boost
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