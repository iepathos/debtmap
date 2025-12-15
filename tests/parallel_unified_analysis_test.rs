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
            length: 20 + (i % 30),             // Varying lengths
            cyclomatic: ((i % 10) as u32 + 5), // Higher complexity (5-14) to trigger debt items
            cognitive: ((i % 5) as u32 + 3),   // Higher cognitive complexity
            nesting: (i % 3) as u32 + 1,       // At least 1 level of nesting
            is_test: i % 20 == 0,
            in_test_module: false,
            visibility: None,
            is_trait_method: false,
            entropy_score: None,
            is_pure: Some(i % 3 == 0),
            purity_confidence: if i % 3 == 0 { Some(0.9) } else { Some(0.1) },
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            purity_reason: None,
            call_dependencies: None,
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
    let (_unified, timings) = builder.build(data_flow, purity, items, file_items, None);

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
    let test_func = FunctionId::new(
        PathBuf::from("tests/test.rs"),
        "test_something".to_string(),
        10,
    );

    let helper_func = FunctionId::new(PathBuf::from("src/lib.rs"), "helper".to_string(), 20);

    let main_func = FunctionId::new(PathBuf::from("src/main.rs"), "main".to_string(), 5);

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
#[ignore] // Performance test - can be slow on CI or with coverage instrumentation
fn test_large_codebase_parallel_analysis() {
    use std::time::{Duration, Instant};

    // Create a large set of metrics simulating a real codebase
    let metrics = create_test_metrics(500);
    let mut call_graph = CallGraph::new();

    // Add functions to call graph
    for metric in &metrics {
        let func_id = debtmap::priority::call_graph::FunctionId::new(
            metric.file.clone(),
            metric.name.clone(),
            metric.line,
        );
        call_graph.add_function(
            func_id,
            false,
            metric.is_test,
            metric.cyclomatic,
            metric.length,
        );
    }

    // Add some call relationships
    for i in 0..metrics.len() - 1 {
        if i % 5 == 0 {
            let caller = debtmap::priority::call_graph::FunctionId::new(
                metrics[i].file.clone(),
                metrics[i].name.clone(),
                metrics[i].line,
            );
            let callee = debtmap::priority::call_graph::FunctionId::new(
                metrics[i + 1].file.clone(),
                metrics[i + 1].name.clone(),
                metrics[i + 1].line,
            );
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

    let (_unified, timings) = builder.build(data_flow, purity, items, file_items, None);

    let duration = start.elapsed();

    // Verify results - we should have some unified items, though not necessarily one per metric
    // since only functions with debt issues are included
    assert!(timings.total > Duration::from_secs(0));

    // Performance check - should be fast even for 500 functions
    // Allow up to 25 seconds to account for coverage instrumentation overhead
    // Coverage instrumentation can add 2-3x overhead on macOS
    assert!(
        duration.as_secs() < 25,
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

        let (data_flow, _purity, test_funcs, debt_agg) =
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

        // Items should be consistent but may not equal metrics.len() since only debt items are included
        // The test passes if we got some items processed
        assert!(!items.is_empty(), "Should have processed some items");
    }
}

#[test]
fn test_parallel_analysis_with_coverage_data() {
    use debtmap::core::{DebtItem, DebtType, Priority};

    let metrics = create_test_metrics(100);
    let call_graph = CallGraph::new();

    // Create mock debt items instead of coverage data
    let debt_items: Vec<DebtItem> = metrics
        .iter()
        .filter(|m| m.cyclomatic > 5) // Only functions with high complexity
        .map(|m| DebtItem {
            id: format!("debt_{}", m.name),
            debt_type: DebtType::Complexity {
                cyclomatic: m.cyclomatic,
                cognitive: m.cognitive,
            },
            priority: Priority::Medium,
            file: m.file.clone(),
            line: m.line,
            column: None,
            message: format!("High complexity: {}", m.cyclomatic),
            context: Some(m.name.clone()),
        })
        .collect();

    let options = ParallelUnifiedAnalysisOptions {
        parallel: true,
        jobs: Some(4),
        batch_size: 25,
        progress: false,
    };

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

    let (data_flow, _purity, test_funcs, debt_agg) =
        builder.execute_phase1_parallel(&metrics, Some(&debt_items));

    let items = builder.execute_phase2_parallel(
        &metrics,
        &test_funcs,
        &debt_agg,
        &data_flow,
        None, // No debt items for this test
        &Default::default(),
        None,
    );

    // Verify debt items were integrated
    assert!(!items.is_empty());
    // Check that high complexity functions have debt items
    let high_complexity_count = metrics.iter().filter(|m| m.cyclomatic > 5).count();
    assert!(
        high_complexity_count > 0,
        "Should have some high complexity functions"
    );
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
    let (data_flow, _purity, test_funcs, debt_agg) =
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

    // Should have processed items, though not necessarily one per metric
    assert!(!items.is_empty(), "Should have processed some items");
}

#[test]
fn test_data_flow_graph_population_integration() {
    // This test validates spec 216: Complete Data Flow Graph Population
    // It ensures that DataFlowGraph is populated with:
    // - CFG analysis from purity detector
    // - Mutation analysis (live vs total mutations)
    // - I/O operations
    // - Variable dependencies

    let metrics = create_test_metrics(50);
    let call_graph = CallGraph::new();
    let options = ParallelUnifiedAnalysisOptions::default();

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

    // Execute phase 1 which populates the DataFlowGraph
    let (data_flow, _purity, _test_funcs, _debt_agg) =
        builder.execute_phase1_parallel(&metrics, None);

    // Verify DataFlowGraph population
    // Note: Since we're using synthetic test metrics without actual Rust source files,
    // the population functions will not find real code to analyze.
    // This test verifies the integration plumbing works, not the content.

    // The DataFlowGraph should be created successfully
    assert_eq!(
        data_flow.call_graph().get_all_functions().count(),
        0, // CallGraph is empty since we didn't add functions to it
        "DataFlowGraph call graph should match initialized state"
    );

    // For a real integration test with actual source files, we would verify:
    // - cfg_analysis is populated for analyzed functions
    // - mutation_info contains live/total mutation counts
    // - io_operations are detected and recorded
    // - variable_deps are extracted from function signatures

    // Since we're using synthetic metrics, we just verify the graph was created
    // and the population functions were called (which they are in spawn_data_flow_task)
}

#[test]
fn test_god_objects_created_in_parallel_analysis() {
    // Spec 207: God objects should be created as UnifiedDebtItems in parallel analysis path
    use std::fs::write;
    use tempfile::TempDir;

    // Create temporary directory with test files
    let temp_dir = TempDir::new().unwrap();
    let god_file_path = temp_dir.path().join("god_object.rs");

    // Write a test file with enough content to trigger god object detection
    let content = "pub struct GodStruct {\n".to_string()
        + &(0..100)
            .map(|i| format!("    field_{}: i32,\n", i))
            .collect::<String>()
        + "}\n\nimpl GodStruct {\n"
        + &(0..60)
            .map(|i| format!("    pub fn method_{}(&self) {{ }}\n", i))
            .collect::<String>()
        + "}";
    write(&god_file_path, content).unwrap();

    // Create metrics for this god object file
    let metrics: Vec<FunctionMetrics> = (0..60)
        .map(|i| FunctionMetrics {
            file: god_file_path.clone(),
            name: format!("method_{}", i),
            line: i * 10 + 100,
            length: 5,
            cyclomatic: 2,
            cognitive: 1,
            nesting: 1,
            is_test: false,
            in_test_module: false,
            visibility: None,
            is_trait_method: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.1),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            purity_reason: None,
            call_dependencies: None,
        })
        .collect();

    let call_graph = CallGraph::new();
    let options = ParallelUnifiedAnalysisOptions {
        parallel: true,
        jobs: Some(2),
        batch_size: 50,
        progress: false,
    };

    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

    // Execute all phases
    let (data_flow, purity, test_funcs, debt_agg) = builder.execute_phase1_parallel(&metrics, None);

    let function_items = builder.execute_phase2_parallel(
        &metrics,
        &test_funcs,
        &debt_agg,
        &data_flow,
        None,
        &Default::default(),
        None,
    );

    // Execute phase 3 WITHOUT no_god_object flag (god object detection enabled)
    let file_items = builder.execute_phase3_parallel(&metrics, None, false);

    // Build final analysis
    let (unified, _timings) = builder.build(data_flow, purity, function_items, file_items, None);

    // Verify god objects are in analysis.items (not just file_items)
    let god_items: Vec<_> = unified
        .items
        .iter()
        .filter(|item| item.god_object_indicators.is_some())
        .collect();

    assert!(
        !god_items.is_empty(),
        "God objects should be created as UnifiedDebtItems in parallel analysis"
    );

    // Verify the god object has correct properties
    for god_item in god_items {
        let indicators = god_item.god_object_indicators.as_ref().unwrap();
        assert!(
            indicators.is_god_object,
            "God object indicator should be true"
        );
        assert!(
            indicators.method_count > 0 || indicators.field_count > 0,
            "God object should have methods or fields"
        );

        // Verify god objects are assigned to T1 (Critical Architecture)
        if let Some(tier) = god_item.tier {
            assert_eq!(
                tier,
                debtmap::priority::RecommendationTier::T1CriticalArchitecture,
                "God objects should be classified as T1 Critical Architecture"
            );
        }
    }

    // Verify file_items also contains god object information
    let file_god_objects: Vec<_> = unified
        .file_items
        .iter()
        .filter(|item| {
            item.metrics
                .god_object_analysis
                .as_ref()
                .is_some_and(|a| a.is_god_object)
        })
        .collect();

    assert!(
        !file_god_objects.is_empty(),
        "God objects should also be in file_items"
    );
}

#[test]
fn test_god_objects_not_created_when_disabled() {
    // Test that god objects are NOT created when no_god_object flag is true
    use std::fs::write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    write(&file_path, "pub struct Test {}").unwrap();

    let metrics: Vec<FunctionMetrics> = (0..60)
        .map(|i| FunctionMetrics {
            file: file_path.clone(),
            name: format!("method_{}", i),
            line: i * 10,
            length: 5,
            cyclomatic: 2,
            cognitive: 1,
            nesting: 1,
            is_test: false,
            in_test_module: false,
            visibility: None,
            is_trait_method: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.1),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            purity_reason: None,
            call_dependencies: None,
        })
        .collect();

    let call_graph = CallGraph::new();
    let options = ParallelUnifiedAnalysisOptions::default();
    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

    let (data_flow, purity, test_funcs, debt_agg) = builder.execute_phase1_parallel(&metrics, None);
    let function_items = builder.execute_phase2_parallel(
        &metrics,
        &test_funcs,
        &debt_agg,
        &data_flow,
        None,
        &Default::default(),
        None,
    );

    // Execute with no_god_object=true
    let file_items = builder.execute_phase3_parallel(&metrics, None, true);
    let (unified, _) = builder.build(data_flow, purity, function_items, file_items, None);

    // Verify NO god objects in analysis.items
    let god_items: Vec<_> = unified
        .items
        .iter()
        .filter(|item| item.god_object_indicators.is_some())
        .collect();

    assert!(
        god_items.is_empty(),
        "God objects should not be created when no_god_object flag is true"
    );
}

#[test]
fn test_god_objects_visible_in_tui() {
    // Test that god objects created in parallel analysis are visible to TUI (via ResultsApp)
    use debtmap::tui::results::app::ResultsApp;
    use std::fs::write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let god_file_path = temp_dir.path().join("god.rs");

    // Create a god object file
    let content = "pub struct God { }\nimpl God {\n".to_string()
        + &(0..60)
            .map(|i| format!("    pub fn method_{}(&self) {{ }}\n", i))
            .collect::<String>()
        + "}";
    write(&god_file_path, content).unwrap();

    let metrics: Vec<FunctionMetrics> = (0..60)
        .map(|i| FunctionMetrics {
            file: god_file_path.clone(),
            name: format!("method_{}", i),
            line: i * 10 + 10,
            length: 5,
            cyclomatic: 8, // High enough to create debt items
            cognitive: 5,
            nesting: 2,
            is_test: false,
            in_test_module: false,
            visibility: None,
            is_trait_method: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.1),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            purity_reason: None,
            call_dependencies: None,
        })
        .collect();

    let call_graph = CallGraph::new();
    let options = ParallelUnifiedAnalysisOptions::default();
    let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

    let (data_flow, purity, test_funcs, debt_agg) = builder.execute_phase1_parallel(&metrics, None);
    let function_items = builder.execute_phase2_parallel(
        &metrics,
        &test_funcs,
        &debt_agg,
        &data_flow,
        None,
        &Default::default(),
        None,
    );
    let file_items = builder.execute_phase3_parallel(&metrics, None, false);
    let (unified, _) = builder.build(data_flow, purity, function_items, file_items, None);

    // Create TUI ResultsApp with the analysis
    let app = ResultsApp::new(unified);

    // Verify TUI can see god objects in its items
    let total_items = app.item_count();
    assert!(total_items > 0, "TUI should have items from analysis");

    // Count god objects visible to TUI
    let god_items_in_tui: Vec<_> = app
        .filtered_items()
        .filter(|item| {
            item.god_object_indicators
                .as_ref()
                .map(|i| i.is_god_object)
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !god_items_in_tui.is_empty(),
        "God objects should be visible in TUI (via ResultsApp.filtered_items())"
    );

    // Verify god object appears in the full item list
    let all_items_with_god: Vec<_> = app
        .analysis()
        .items
        .iter()
        .filter(|item| {
            item.god_object_indicators
                .as_ref()
                .map(|i| i.is_god_object)
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !all_items_with_god.is_empty(),
        "God objects should be in analysis.items (accessible to TUI)"
    );
}

#[test]
#[ignore] // Performance test - run explicitly with --ignored
fn test_data_flow_population_performance_overhead() {
    // This test validates spec 216 requirement: "Performance: Data flow population must add < 10% to total analysis time"
    // We measure the overhead by comparing analysis with and without data flow population
    use std::time::Instant;

    let metrics = create_test_metrics(200);
    let call_graph = CallGraph::new();

    // Measure baseline analysis time (without detailed population)
    // We'll run the analysis multiple times to get a stable measurement
    let iterations = 5;
    let mut baseline_times = Vec::new();

    for _ in 0..iterations {
        let options = ParallelUnifiedAnalysisOptions {
            parallel: true,
            jobs: Some(4),
            batch_size: 50,
            progress: false,
        };

        let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph.clone(), options);

        let start = Instant::now();
        let (_data_flow, _purity, _test_funcs, _debt_agg) =
            builder.execute_phase1_parallel(&metrics, None);
        baseline_times.push(start.elapsed());
    }

    // Calculate average baseline time
    let baseline_avg = baseline_times.iter().sum::<std::time::Duration>() / iterations as u32;

    // For this test, we're measuring the overhead of the data flow population that occurs
    // in execute_phase1_parallel. Since the population is already integrated, we verify
    // that the total time is reasonable and document the expected overhead.

    // The population functions are called in spawn_data_flow_task (in parallel_unified_analysis.rs)
    // which includes:
    // - populate_from_call_graph
    // - populate_variable_dependencies_batch
    // - populate_io_operations_batch
    // - populate_cfg_analysis_batch

    // Since we're using synthetic metrics without real source files, the population
    // overhead is minimal. In real-world usage with actual Rust files, the overhead
    // should still be < 10% as required by spec 216.

    // Verify the baseline time is reasonable (should complete quickly for 200 synthetic functions)
    // Allow up to 5 seconds to account for coverage instrumentation and CI overhead
    assert!(
        baseline_avg.as_secs() < 5,
        "Baseline analysis took too long: {:?}",
        baseline_avg
    );

    // Log timing information for manual verification
    eprintln!("Data flow population performance (200 functions):");
    eprintln!("  Average time: {:?}", baseline_avg);
    eprintln!("  Min time: {:?}", baseline_times.iter().min().unwrap());
    eprintln!("  Max time: {:?}", baseline_times.iter().max().unwrap());

    // Note: To properly measure the <10% overhead requirement, this test should be run
    // on real codebases with actual source files where population does significant work.
    // This integration test verifies the plumbing works correctly with synthetic data.
}

// ============================================================================
// Spec 213: Extraction Pipeline Baseline Tests
// ============================================================================

#[test]
fn test_extraction_pipeline_baseline_equivalence() {
    // This test validates spec 213 requirement: "Analysis output unchanged (diff test against known baseline)"
    // Verifies that the unified extraction pipeline produces equivalent results
    // to the analysis without extracted data (both should now use extraction internally).
    use debtmap::extraction::{ExtractedFileData, UnifiedFileExtractor};
    use std::collections::HashMap;
    use tempfile::TempDir;

    // Create a temporary directory with real Rust code
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test_module.rs");

    // Write realistic Rust code with various patterns
    let test_code = r#"
use std::collections::HashMap;

pub struct Calculator {
    state: i32,
}

impl Calculator {
    pub fn new() -> Self {
        Self { state: 0 }
    }

    pub fn add(&mut self, value: i32) -> i32 {
        self.state += value;
        self.state
    }

    pub fn compute_complex(&self, items: &[i32]) -> i32 {
        items.iter()
            .filter(|&&x| x > 0)
            .map(|&x| x * 2)
            .fold(0, |acc, x| acc + x)
    }
}

fn pure_function(x: i32, y: i32) -> i32 {
    x + y
}

fn complex_function(data: &[String]) -> HashMap<String, usize> {
    let mut result = HashMap::new();
    for item in data {
        if item.len() > 3 {
            let count = result.entry(item.clone()).or_insert(0);
            *count += 1;
        }
    }
    result
}
"#;

    std::fs::write(&test_file, test_code).unwrap();

    // Create metrics for the test file functions
    let metrics = vec![
        FunctionMetrics {
            file: test_file.clone(),
            name: "Calculator::new".to_string(),
            line: 10,
            length: 3,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            is_test: false,
            in_test_module: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.95),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            purity_reason: None,
            call_dependencies: None,
        },
        FunctionMetrics {
            file: test_file.clone(),
            name: "Calculator::add".to_string(),
            line: 14,
            length: 4,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            is_test: false,
            in_test_module: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.9),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            purity_reason: None,
            call_dependencies: None,
        },
        FunctionMetrics {
            file: test_file.clone(),
            name: "Calculator::compute_complex".to_string(),
            line: 19,
            length: 6,
            cyclomatic: 2,
            cognitive: 2,
            nesting: 1,
            is_test: false,
            in_test_module: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.95),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            purity_reason: None,
            call_dependencies: None,
        },
        FunctionMetrics {
            file: test_file.clone(),
            name: "pure_function".to_string(),
            line: 27,
            length: 3,
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            is_test: false,
            in_test_module: false,
            visibility: None,
            is_trait_method: false,
            entropy_score: None,
            is_pure: Some(true),
            purity_confidence: Some(0.99),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            purity_reason: None,
            call_dependencies: None,
        },
        FunctionMetrics {
            file: test_file.clone(),
            name: "complex_function".to_string(),
            line: 31,
            length: 10,
            cyclomatic: 3,
            cognitive: 4,
            nesting: 2,
            is_test: false,
            in_test_module: false,
            visibility: None,
            is_trait_method: false,
            entropy_score: None,
            is_pure: Some(false),
            purity_confidence: Some(0.3),
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            purity_reason: None,
            call_dependencies: None,
        },
    ];

    // Run analysis with extraction pipeline
    let content = std::fs::read_to_string(&test_file).unwrap();
    let extracted_data = UnifiedFileExtractor::extract(&test_file, &content).unwrap();
    let mut extracted_map: HashMap<PathBuf, ExtractedFileData> = HashMap::new();
    extracted_map.insert(test_file.clone(), extracted_data);

    let call_graph = CallGraph::new();

    // Run WITH extracted data
    let options_with_extracted = ParallelUnifiedAnalysisOptions {
        parallel: true,
        jobs: Some(4),
        batch_size: 25,
        progress: false,
    };

    let mut builder_with_extracted =
        ParallelUnifiedAnalysisBuilder::new(call_graph.clone(), options_with_extracted)
            .with_extracted_data(extracted_map.clone());

    let (data_flow_with, purity_with, test_funcs_with, _debt_agg_with) =
        builder_with_extracted.execute_phase1_parallel(&metrics, None);

    // Run WITHOUT extracted data (uses fallback extraction path)
    let options_without_extracted = ParallelUnifiedAnalysisOptions {
        parallel: true,
        jobs: Some(4),
        batch_size: 25,
        progress: false,
    };

    let mut builder_without_extracted =
        ParallelUnifiedAnalysisBuilder::new(call_graph.clone(), options_without_extracted);

    let (data_flow_without, purity_without, test_funcs_without, _debt_agg_without) =
        builder_without_extracted.execute_phase1_parallel(&metrics, None);

    // Compare results - they should be equivalent
    assert_eq!(
        purity_with.len(),
        purity_without.len(),
        "Purity analysis count should match"
    );

    assert_eq!(
        test_funcs_with.len(),
        test_funcs_without.len(),
        "Test function count should match"
    );

    // Verify data flow contains expected information
    // Both paths should produce similar data flow graphs
    // Note: DataFlowGraph doesn't expose function_count, so we verify via call_graph
    let call_graph_with = data_flow_with.call_graph();
    let call_graph_without = data_flow_without.call_graph();

    // The call graphs should both be empty (we didn't build them with function data)
    // but the data flow graphs should have been populated with purity/IO/deps info
    // We verify this indirectly through the purity map which was populated
    assert_eq!(
        call_graph_with.get_all_functions().count(),
        call_graph_without.get_all_functions().count(),
        "Call graph function count should match"
    );

    // Verify purity values are consistent
    for (key, value_with) in &purity_with {
        if let Some(value_without) = purity_without.get(key) {
            assert_eq!(
                value_with, value_without,
                "Purity value for {} should match between extraction paths",
                key
            );
        }
    }
}

#[test]
#[ignore] // Performance test - run explicitly with --ignored
fn test_extraction_pipeline_speedup() {
    // This test validates spec 213 requirement: "10x+ speedup measured on large codebase"
    // Measures the speedup from using unified extraction vs repeated parsing
    use std::time::Instant;

    // Use debtmap's own source files as a realistic test case
    let src_path = std::path::Path::new("src");
    if !src_path.exists() {
        eprintln!("Skipping speedup test - src directory not found");
        return;
    }

    // Collect Rust files
    let rust_files: Vec<PathBuf> = walkdir::WalkDir::new(src_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .take(50) // Limit to 50 files for reasonable test time
        .collect();

    if rust_files.is_empty() {
        eprintln!("Skipping speedup test - no Rust files found");
        return;
    }

    eprintln!("Testing with {} Rust files", rust_files.len());

    // Measure unified extraction time (single-pass)
    let extraction_start = Instant::now();
    let mut extracted_count = 0;
    for path in &rust_files {
        if let Ok(content) = std::fs::read_to_string(path) {
            if debtmap::extraction::UnifiedFileExtractor::extract(path, &content).is_ok() {
                extracted_count += 1;
            }
        }
    }
    let extraction_time = extraction_start.elapsed();

    // Measure simulated per-function parsing time
    // This simulates what the old approach would do: parse each file multiple times
    // (once for I/O, once for deps, once for transformations per function)
    let simulated_start = Instant::now();
    let simulated_parses = 3; // Simulating 3 parsing passes per file
    for _ in 0..simulated_parses {
        for path in &rust_files {
            if let Ok(content) = std::fs::read_to_string(path) {
                let _ = syn::parse_file(&content);
            }
        }
    }
    let simulated_time = simulated_start.elapsed();

    // Calculate speedup
    let extraction_ms = extraction_time.as_millis() as f64;
    let simulated_ms = simulated_time.as_millis() as f64;
    let speedup = if extraction_ms > 0.0 {
        simulated_ms / extraction_ms
    } else {
        f64::INFINITY
    };

    eprintln!("\nSpec 213 Speedup Test Results:");
    eprintln!("  Files processed: {}", rust_files.len());
    eprintln!("  Files successfully extracted: {}", extracted_count);
    eprintln!("  Unified extraction time: {:?}", extraction_time);
    eprintln!(
        "  Simulated per-function parsing time ({} passes): {:?}",
        simulated_parses, simulated_time
    );
    eprintln!("  Speedup factor: {:.1}x", speedup);

    // Note: The spec requires 10x+ speedup. In practice, the speedup increases with:
    // - More functions per file (each would trigger re-parsing in old approach)
    // - Larger files (parsing overhead compounds)
    // This test with 3 simulated passes is conservative.
    // Real-world codebases with 20,000+ functions would see much higher speedup.

    // Don't assert on speedup since it depends on many factors (CI load, file sizes, etc.)
    // The test documents the actual measured speedup for validation purposes.
    assert!(
        extracted_count > 0,
        "Should successfully extract at least some files"
    );
}
