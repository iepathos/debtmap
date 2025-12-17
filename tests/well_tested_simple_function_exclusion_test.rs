/// Integration test for spec 109: Well-tested simple functions should not appear in top 10
///
/// This test verifies that well-tested simple functions (>80% coverage, cyclomatic < 10)
/// don't appear in the top 10 technical debt recommendations.
use debtmap::builders::parallel_unified_analysis::{
    ParallelUnifiedAnalysisBuilder, ParallelUnifiedAnalysisOptions,
};
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::CallGraph;
use debtmap::priority::UnifiedAnalysisQueries;
use debtmap::risk::lcov::{FunctionCoverage, LcovData};
use std::path::PathBuf;

#[test]
fn test_well_tested_simple_functions_excluded_from_top_10() {
    // Create a mix of functions:
    // - Some well-tested simple functions (should NOT be in top 10)
    // - Some complex/untested functions (should be in top 10)

    let test_dir = PathBuf::from("test_project");

    // Create test functions
    let mut functions = vec![];
    let mut coverage_data = LcovData::default();

    // Add 5 well-tested simple functions (these should NOT be in top 10)
    for i in 0..5 {
        let func = FunctionMetrics {
            file: test_dir.join("simple.rs"),
            name: format!("simple_well_tested_{}", i),
            line: 10 + i * 10,
            length: 8,
            cyclomatic: 3, // Low complexity
            cognitive: 5,
            nesting: 1,
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
            composition_metrics: None,
            language_specific: None,
            purity_reason: None,
            call_dependencies: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        };

        // Add 100% coverage for these functions
        coverage_data
            .functions
            .entry(func.file.clone())
            .or_insert_with(Vec::new)
            .push(FunctionCoverage {
                name: func.name.clone(),
                start_line: func.line,
                execution_count: 20,
                coverage_percentage: 100.0,
                uncovered_lines: vec![],
                normalized: debtmap::risk::lcov::NormalizedFunctionName::simple(&func.name),
            });

        functions.push(func);
    }

    // Add 10 complex/untested functions (these SHOULD be in top 10)
    for i in 0..10 {
        let func = FunctionMetrics {
            file: test_dir.join("complex.rs"),
            name: format!("complex_untested_{}", i),
            line: 10 + i * 20,
            length: 100,
            cyclomatic: 15, // High complexity
            cognitive: 25,
            nesting: 4,
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
            composition_metrics: None,
            language_specific: None,
            purity_reason: None,
            call_dependencies: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
        };

        // No coverage for these functions (untested)

        functions.push(func);
    }

    // Build analysis with coverage data
    let call_graph = CallGraph::new();
    let options = ParallelUnifiedAnalysisOptions {
        parallel: false,
        jobs: Some(1),
        batch_size: 100,
        progress: false,
    };

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

    // Phase 1: Execute parallel initialization
    let (data_flow, purity, test_funcs, debt_agg) =
        builder.execute_phase1_parallel(&functions, None);

    // Phase 2: Execute function analysis with coverage data
    let items = builder.execute_phase2_parallel(
        &functions,
        &test_funcs,
        &debt_agg,
        &data_flow,
        Some(&coverage_data),
        &Default::default(),
        None,
    );

    // Phase 3: File-level analysis
    let file_items = builder.execute_phase3_parallel(&functions, Some(&coverage_data), false);

    // Build final analysis
    let (analysis, _timings) =
        builder.build(data_flow, purity, items, file_items, Some(&coverage_data));

    // Get top 10 recommendations
    let recommendations = analysis.get_top_priorities(10);

    // Verify that none of the well-tested simple functions are in the top 10
    let simple_in_top_10: Vec<_> = recommendations
        .iter()
        .filter(|item| item.location.function.starts_with("simple_well_tested_"))
        .collect();

    assert!(
        simple_in_top_10.is_empty(),
        "Well-tested simple functions should not appear in top 10, but found: {:?}",
        simple_in_top_10
            .iter()
            .map(|item| &item.location.function)
            .collect::<Vec<_>>()
    );

    // Verify that complex/untested functions dominate the top 10
    let complex_in_top_10: Vec<_> = recommendations
        .iter()
        .filter(|item| item.location.function.starts_with("complex_untested_"))
        .collect();

    assert!(
        complex_in_top_10.len() == 10,
        "Top 10 should be dominated by complex/untested functions, but only found {} of them",
        complex_in_top_10.len()
    );
}

#[test]
fn test_well_tested_simple_function_has_low_score() {
    // This is a unit-level check that complements the integration test above
    // Verify that a single well-tested simple function gets a low score

    let func = FunctionMetrics {
        file: PathBuf::from("test.rs"),
        name: "simple_tested".to_string(),
        line: 10,
        length: 8,
        cyclomatic: 5,
        cognitive: 15,
        nesting: 1,
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
        composition_metrics: None,
        language_specific: None,
        purity_reason: None,
        call_dependencies: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis: None,
    };

    let mut coverage_data = LcovData::default();
    coverage_data.functions.insert(
        func.file.clone(),
        vec![FunctionCoverage {
            name: func.name.clone(),
            start_line: func.line,
            execution_count: 15,
            coverage_percentage: 100.0,
            uncovered_lines: vec![],
            normalized: debtmap::risk::lcov::NormalizedFunctionName::simple(&func.name),
        }],
    );

    let call_graph = CallGraph::new();
    let options = ParallelUnifiedAnalysisOptions {
        parallel: false,
        jobs: Some(1),
        batch_size: 100,
        progress: false,
    };

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);
    let functions = vec![func.clone()];

    // Phase 1: Execute parallel initialization
    let (data_flow, purity, test_funcs, debt_agg) =
        builder.execute_phase1_parallel(&functions, None);

    // Phase 2: Execute function analysis with coverage data
    let items = builder.execute_phase2_parallel(
        &functions,
        &test_funcs,
        &debt_agg,
        &data_flow,
        Some(&coverage_data),
        &Default::default(),
        None,
    );

    // Phase 3: File-level analysis
    let file_items = builder.execute_phase3_parallel(&functions, Some(&coverage_data), false);

    // Build final analysis
    let (analysis, _timings) =
        builder.build(data_flow, purity, items, file_items, Some(&coverage_data));

    let recommendations = analysis.get_top_priorities(10);

    // Should have the function in results
    let simple_func = recommendations
        .iter()
        .find(|item| item.location.function == "simple_tested");

    if let Some(item) = simple_func {
        // Score should be below 20.0 (as per spec 109)
        assert!(
            item.unified_score.final_score.value() < 20.0,
            "Well-tested simple function should score < 20.0, got {}",
            item.unified_score.final_score.value()
        );
    }
}
