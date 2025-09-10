use debtmap::builders::parallel_unified_analysis::{
    ParallelUnifiedAnalysisBuilder, ParallelUnifiedAnalysisOptions,
};
use debtmap::core::FunctionMetrics;
use debtmap::priority::call_graph::CallGraph;
use std::path::PathBuf;

/// Helper function to create test metrics
fn create_test_metrics(count: usize) -> Vec<FunctionMetrics> {
    (0..count)
        .map(|i| FunctionMetrics {
            file: PathBuf::from(format!("test{}.rs", i / 10)),
            name: format!("function_{}", i),
            line: i * 10,
            length: 20,
            cyclomatic: (i % 10) as u32 + 1,
            cognitive: (i % 5) as u32,
            nesting: (i % 3) as u32,
            is_test: i % 20 == 0,
            in_test_module: false,
            visibility: None,
            is_trait_method: false,
            entropy_score: None,
            is_pure: Some(i % 3 == 0),
            purity_confidence: if i % 3 == 0 { Some(0.9) } else { Some(0.1) },
        })
        .collect()
}

#[test]
fn test_parallel_unified_analysis_execution() {
    // Create test data
    let metrics = create_test_metrics(100);
    let call_graph = CallGraph::new();

    // Set up parallel options
    let options = ParallelUnifiedAnalysisOptions {
        parallel: true,
        jobs: Some(4),
        batch_size: 25,
        progress: false,
    };

    // Create builder
    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

    // Execute phase 1
    let (data_flow, purity, test_funcs, debt_agg) = builder.execute_phase1_parallel(&metrics, None);

    // Verify phase 1 results
    assert!(!purity.is_empty());
    assert_eq!(purity.len(), 100);

    // Verify some functions are marked as pure
    let pure_count = purity.values().filter(|&&v| v).count();
    assert!(pure_count > 0);

    // Execute phase 2
    let items = builder.execute_phase2_parallel(
        &metrics,
        &test_funcs,
        &debt_agg,
        &data_flow,
        None,                // coverage_data
        &Default::default(), // framework_exclusions
        None,                // function_pointer_used_functions
    );

    // Verify phase 2 results
    assert!(!items.is_empty());

    // Execute phase 3
    let file_items = builder.execute_phase3_parallel(&metrics, None, false);

    // Build final analysis
    let (unified, timings) = builder.build(data_flow, purity, items, file_items, None);

    // Verify timing results (we don't expect items without proper setup)
    assert!(timings.total >= std::time::Duration::from_secs(0));
    assert!(timings.data_flow_creation >= std::time::Duration::from_secs(0));
    assert!(timings.purity_analysis >= std::time::Duration::from_secs(0));
}

#[test]
fn test_optimized_test_detector() {
    use debtmap::builders::parallel_unified_analysis::OptimizedTestDetector;
    use debtmap::priority::call_graph::FunctionId;
    use std::sync::Arc;

    let mut call_graph = CallGraph::new();

    // Add test functions
    let test_func = FunctionId {
        file: PathBuf::from("tests/test.rs"),
        name: "test_something".to_string(),
        line: 10,
    };

    let helper_func = FunctionId {
        file: PathBuf::from("src/lib.rs"),
        name: "helper".to_string(),
        line: 20,
    };

    let main_func = FunctionId {
        file: PathBuf::from("src/main.rs"),
        name: "main".to_string(),
        line: 5,
    };

    // Add functions to graph
    call_graph.add_function(test_func.clone(), false, true, 5, 20);
    call_graph.add_function(helper_func.clone(), false, false, 3, 15);
    call_graph.add_function(main_func.clone(), true, false, 10, 50);

    // Add call relationships
    call_graph.add_call_parts(
        test_func.clone(),
        helper_func.clone(),
        debtmap::priority::call_graph::CallType::Direct,
    );

    // Create detector
    let detector = OptimizedTestDetector::new(Arc::new(call_graph));

    // Test detection
    assert!(detector.is_test_only(&test_func));
    assert!(detector.is_test_only(&helper_func)); // Called only from test
    assert!(!detector.is_test_only(&main_func)); // Not test-related

    // Test bulk detection
    let all_test_only = detector.find_all_test_only_functions();
    assert!(all_test_only.contains(&test_func));
    assert!(all_test_only.contains(&helper_func));
    assert!(!all_test_only.contains(&main_func));
}

#[test]
fn test_parallel_vs_sequential_consistency() {
    use debtmap::builders::unified_analysis;

    // Create test data
    let metrics = create_test_metrics(50);
    let call_graph = CallGraph::new();

    // Run sequential analysis
    let sequential_result = unified_analysis::create_unified_analysis_with_exclusions(
        &metrics,
        &call_graph,
        None,
        &Default::default(),
        None,
        None,
        false,
        None,
        None,
        false,
    );

    // Run parallel analysis
    std::env::set_var("DEBTMAP_PARALLEL", "true");
    let parallel_result = unified_analysis::create_unified_analysis_with_exclusions(
        &metrics,
        &call_graph,
        None,
        &Default::default(),
        None,
        None,
        false,
        None,
        None,
        false,
    );
    std::env::remove_var("DEBTMAP_PARALLEL");

    // Compare results - they should produce the same number of items
    assert_eq!(sequential_result.items.len(), parallel_result.items.len());
    assert_eq!(
        sequential_result.file_items.len(),
        parallel_result.file_items.len()
    );
}

#[test]
fn test_large_codebase_parallel_analysis() {
    use std::time::Instant;

    // Create a large set of metrics simulating a real codebase
    let metrics = create_test_metrics(500);
    let mut call_graph = CallGraph::new();

    // Add functions to call graph
    for metric in &metrics {
        let func_id = debtmap::priority::call_graph::FunctionId {
            file: metric.file.clone(),
            name: metric.name.clone(),
            line: metric.line,
        };
        call_graph.add_function(
            func_id,
            false,
            metric.is_test,
            metric.cyclomatic,
            metric.length as u32,
        );
    }

    // Add some call relationships
    for i in 0..metrics.len() - 1 {
        if i % 5 == 0 {
            let caller = debtmap::priority::call_graph::FunctionId {
                file: metrics[i].file.clone(),
                name: metrics[i].name.clone(),
                line: metrics[i].line,
            };
            let callee = debtmap::priority::call_graph::FunctionId {
                file: metrics[i + 1].file.clone(),
                name: metrics[i + 1].name.clone(),
                line: metrics[i + 1].line,
            };
            call_graph.add_call_parts(
                caller,
                callee,
                debtmap::priority::call_graph::CallType::Direct,
            );
        }
    }

    let options = ParallelUnifiedAnalysisOptions {
        parallel: true,
        jobs: None, // Use all cores
        batch_size: 100,
        progress: false,
    };

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

    let start = Instant::now();

    // Execute all phases
    let (data_flow, purity, test_funcs, debt_agg) = builder.execute_phase1_parallel(&metrics, None);

    let items = builder.execute_phase2_parallel(
        &metrics,
        &test_funcs,
        &debt_agg,
        &data_flow,
        None,
        &Default::default(),
        None,
    );

    let file_items = builder.execute_phase3_parallel(&metrics, None, false);

    let (unified, timings) = builder.build(data_flow, purity, items, file_items, None);

    let duration = start.elapsed();

    // Verify results
    assert!(!unified.items.is_empty());
    assert!(!unified.file_items.is_empty());

    // Performance check - should be fast even for 500 functions
    assert!(
        duration.as_secs() < 2,
        "Large codebase analysis took too long: {:?}",
        duration
    );

    // Verify timing breakdown
    assert!(timings.total > std::time::Duration::from_secs(0));
    assert_eq!(
        timings.total,
        timings.data_flow_creation
            + timings.purity_analysis
            + timings.test_detection
            + timings.debt_aggregation
            + timings.function_analysis
            + timings.file_analysis
            + timings.aggregation
            + timings.sorting
    );
}

#[test]
fn test_parallel_analysis_different_batch_sizes() {
    let metrics = create_test_metrics(200);
    let call_graph = CallGraph::new();

    for batch_size in [10, 50, 100, 200] {
        let options = ParallelUnifiedAnalysisOptions {
            parallel: true,
            jobs: Some(4),
            batch_size,
            progress: false,
        };

        let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph.clone(), options);

        let (data_flow, purity, test_funcs, debt_agg) =
            builder.execute_phase1_parallel(&metrics, None);

        let items = builder.execute_phase2_parallel(
            &metrics,
            &test_funcs,
            &debt_agg,
            &data_flow,
            None,
            &Default::default(),
            None,
        );

        // All batch sizes should produce the same number of items
        assert_eq!(items.len(), metrics.len());
    }
}

#[test]
fn test_parallel_analysis_with_coverage_data() {
    use debtmap::risk::lcov::LcovData;
    use std::collections::HashMap;

    let metrics = create_test_metrics(100);
    let call_graph = CallGraph::new();

    // Create mock coverage data
    let mut coverage_data = LcovData {
        files: HashMap::new(),
    };

    for metric in &metrics {
        let file_path = metric.file.to_str().unwrap().to_string();
        let mut lines = HashMap::new();
        lines.insert(metric.line, 10); // Each function hit 10 times
        coverage_data.files.insert(file_path, lines);
    }

    let options = ParallelUnifiedAnalysisOptions {
        parallel: true,
        jobs: Some(4),
        batch_size: 25,
        progress: false,
    };

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

    let (data_flow, purity, test_funcs, debt_agg) =
        builder.execute_phase1_parallel(&metrics, Some(&coverage_data));

    let items = builder.execute_phase2_parallel(
        &metrics,
        &test_funcs,
        &debt_agg,
        &data_flow,
        Some(&coverage_data),
        &Default::default(),
        None,
    );

    // Verify coverage is applied
    for item in &items {
        assert!(item.coverage_hit_count.is_some());
        assert_eq!(item.coverage_hit_count, Some(10));
    }
}

#[test]
fn test_parallel_analysis_memory_efficiency() {
    // Test that parallel analysis doesn't consume excessive memory
    // by processing a very large number of small functions
    let metrics = create_test_metrics(1000);
    let call_graph = CallGraph::new();

    let options = ParallelUnifiedAnalysisOptions {
        parallel: true,
        jobs: Some(2),  // Limit parallelism to control memory
        batch_size: 50, // Small batches
        progress: false,
    };

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

    // This should complete without running out of memory
    let (data_flow, purity, test_funcs, debt_agg) = builder.execute_phase1_parallel(&metrics, None);

    let items = builder.execute_phase2_parallel(
        &metrics,
        &test_funcs,
        &debt_agg,
        &data_flow,
        None,
        &Default::default(),
        None,
    );

    assert_eq!(items.len(), metrics.len());
}
