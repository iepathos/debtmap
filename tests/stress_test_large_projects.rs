#[cfg(test)]
mod stress_tests {
    use debtmap::builders::parallel_unified_analysis::{
        ParallelUnifiedAnalysisBuilder, ParallelUnifiedAnalysisOptions,
    };
    use debtmap::core::FunctionMetrics;
    use debtmap::priority::call_graph::{CallGraph, CallType, FunctionId};
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    /// Create a large, realistic codebase simulation
    fn create_large_codebase_metrics(
        num_files: usize,
        funcs_per_file: usize,
    ) -> Vec<FunctionMetrics> {
        let mut metrics = Vec::new();

        for file_idx in 0..num_files {
            let file_path =
                PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx));

            for func_idx in 0..funcs_per_file {
                let is_test = func_idx == 0 && file_idx % 10 == 0; // Some test files
                let name = if is_test {
                    format!("test_function_{}", func_idx)
                } else {
                    format!("function_{}_{}", file_idx, func_idx)
                };

                metrics.push(FunctionMetrics {
                    file: file_path.clone(),
                    name,
                    line: func_idx * 50 + 10,
                    length: 10 + (func_idx % 40), // Varying function lengths
                    cyclomatic: 1 + (func_idx % 15) as u32, // Varying complexity
                    cognitive: (func_idx % 10) as u32,
                    nesting: (func_idx % 4) as u32,
                    is_test,
                    in_test_module: is_test,
                    visibility: Some("pub".to_string()),
                    is_trait_method: func_idx % 20 == 0,
                    entropy_score: Some(debtmap::complexity::entropy_core::EntropyScore {
                        token_entropy: 0.5 + (func_idx as f64 * 0.01) % 0.5,
                        pattern_repetition: 0.2,
                        branch_similarity: 0.3,
                        effective_complexity: func_idx as f64 * 0.7,
                        unique_variables: 5,
                        max_nesting: 2,
                        dampening_applied: 0.9,
                    }),
                    is_pure: Some(func_idx % 3 != 0),
                    purity_confidence: Some(if func_idx % 3 != 0 { 0.8 } else { 0.2 }),
                    detected_patterns: None,
                    upstream_callers: None,
                    downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
                });
            }
        }

        metrics
    }

    /// Create a complex call graph with realistic relationships
    fn create_complex_call_graph(metrics: &[FunctionMetrics]) -> CallGraph {
        let mut call_graph = CallGraph::new();

        // Add all functions to the graph
        for metric in metrics {
            let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

            call_graph.add_function(
                func_id,
                metric
                    .visibility
                    .as_ref()
                    .map(|v| v == "pub")
                    .unwrap_or(false),
                metric.is_test,
                metric.cyclomatic,
                metric.length,
            );
        }

        // Add realistic call relationships
        // Each function calls 0-5 other functions
        for i in 0..metrics.len() {
            let caller = FunctionId::new(
                metrics[i].file.clone(),
                metrics[i].name.clone(),
                metrics[i].line,
            );

            // Create some call relationships
            let num_calls = i % 6; // 0-5 calls per function
            for j in 0..num_calls {
                let callee_idx = (i + j + 1) % metrics.len();
                let callee = FunctionId::new(
                    metrics[callee_idx].file.clone(),
                    metrics[callee_idx].name.clone(),
                    metrics[callee_idx].line,
                );

                call_graph.add_call_parts(
                    caller.clone(),
                    callee,
                    if j % 2 == 0 {
                        CallType::Direct
                    } else {
                        CallType::Delegate // Using Delegate instead of non-existent Indirect
                    },
                );
            }
        }

        call_graph
    }

    #[test]
    #[ignore] // Run with: cargo test stress_test_1000_files -- --ignored
    fn stress_test_1000_files() {
        let metrics = create_large_codebase_metrics(1000, 10); // 1000 files, 10 functions each = 10,000 functions
        let call_graph = create_complex_call_graph(&metrics);

        let options = ParallelUnifiedAnalysisOptions {
            parallel: true,
            jobs: None, // Use all available cores
            batch_size: 100,
            progress: false,
        };

        let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

        let start = Instant::now();

        // Execute all phases
        let (data_flow, _purity, test_funcs, debt_agg) =
            builder.execute_phase1_parallel(&metrics, None);
        let phase1_time = start.elapsed();

        let items = builder.execute_phase2_parallel(
            &metrics,
            &test_funcs,
            &debt_agg,
            &data_flow,
            None,
            &Default::default(),
            None,
        );
        let phase2_time = start.elapsed() - phase1_time;

        let file_items = builder.execute_phase3_parallel(&metrics, None, false);
        let phase3_time = start.elapsed() - phase1_time - phase2_time;

        let (unified, _timings) = builder.build(data_flow, _purity, items, file_items, None);
        let total_time = start.elapsed();

        // Assertions
        assert_eq!(
            unified.items.len(),
            10000,
            "Should analyze all 10,000 functions"
        );
        assert_eq!(
            unified.file_items.len(),
            1000,
            "Should have results for all 1000 files"
        );

        // Performance assertions - should complete in reasonable time
        assert!(
            total_time < Duration::from_secs(10),
            "1000-file analysis took too long: {:?}",
            total_time
        );

        println!("1000-file stress test results:");
        println!("  Total time: {:?}", total_time);
        println!("  Phase 1 (initialization): {:?}", phase1_time);
        println!("  Phase 2 (function analysis): {:?}", phase2_time);
        println!("  Phase 3 (file analysis): {:?}", phase3_time);
        println!("  Items analyzed: {}", unified.items.len());
        println!("  Files analyzed: {}", unified.file_items.len());
    }

    #[test]
    #[ignore] // Run with: cargo test stress_test_5000_files -- --ignored
    fn stress_test_5000_files() {
        let metrics = create_large_codebase_metrics(5000, 10); // 5000 files, 10 functions each = 50,000 functions
        let call_graph = create_complex_call_graph(&metrics);

        let options = ParallelUnifiedAnalysisOptions {
            parallel: true,
            jobs: None,      // Use all available cores
            batch_size: 200, // Larger batches for better efficiency
            progress: false,
        };

        let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

        let start = Instant::now();

        // Execute all phases
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
        let file_items = builder.execute_phase3_parallel(&metrics, None, false);
        let (unified, _timings) = builder.build(data_flow, _purity, items, file_items, None);

        let total_time = start.elapsed();

        // Assertions
        assert_eq!(
            unified.items.len(),
            50000,
            "Should analyze all 50,000 functions"
        );
        assert_eq!(
            unified.file_items.len(),
            5000,
            "Should have results for all 5000 files"
        );

        // Performance assertion - should complete in reasonable time even for huge codebases
        assert!(
            total_time < Duration::from_secs(60),
            "5000-file analysis took too long: {:?}",
            total_time
        );

        println!("5000-file stress test completed in {:?}", total_time);
    }

    #[test]
    #[ignore] // Run with: cargo test stress_test_memory_pressure -- --ignored
    fn stress_test_memory_pressure() {
        // Test with limited parallelism to ensure memory efficiency
        let metrics = create_large_codebase_metrics(2000, 20); // 40,000 functions
        let call_graph = create_complex_call_graph(&metrics);

        let options = ParallelUnifiedAnalysisOptions {
            parallel: true,
            jobs: Some(2),  // Limit to 2 threads to reduce memory pressure
            batch_size: 50, // Small batches
            progress: false,
        };

        let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

        // Should complete without excessive memory usage
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
        let file_items = builder.execute_phase3_parallel(&metrics, None, false);
        let (unified, _) = builder.build(data_flow, _purity, items, file_items, None);

        assert_eq!(unified.items.len(), 40000);
        println!("Memory pressure test passed with 40,000 functions");
    }

    #[test]
    #[ignore] // Run with: cargo test stress_test_highly_connected_graph -- --ignored
    fn stress_test_highly_connected_graph() {
        // Test with a highly interconnected call graph
        let metrics = create_large_codebase_metrics(100, 100); // 10,000 functions in 100 files
        let mut call_graph = CallGraph::new();

        // Add all functions
        for metric in &metrics {
            let func_id = FunctionId::new(metric.file.clone(), metric.name.clone(), metric.line);

            call_graph.add_function(
                func_id,
                true,
                metric.is_test,
                metric.cyclomatic,
                metric.length,
            );
        }

        // Create a highly connected graph - each function calls many others
        for i in 0..metrics.len() {
            let caller = FunctionId::new(
                metrics[i].file.clone(),
                metrics[i].name.clone(),
                metrics[i].line,
            );

            // Each function calls up to 20 others
            for j in 1..=20 {
                let callee_idx = (i + j * 97) % metrics.len(); // Use prime for distribution
                let callee = FunctionId::new(
                    metrics[callee_idx].file.clone(),
                    metrics[callee_idx].name.clone(),
                    metrics[callee_idx].line,
                );

                call_graph.add_call_parts(caller.clone(), callee, CallType::Direct);
            }
        }

        let options = ParallelUnifiedAnalysisOptions {
            parallel: true,
            jobs: None,
            batch_size: 100,
            progress: false,
        };

        let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph, options);

        let start = Instant::now();

        // This should handle the complex graph efficiently
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

        let duration = start.elapsed();

        assert_eq!(items.len(), 10000);
        assert!(
            duration < Duration::from_secs(30),
            "Highly connected graph analysis took too long: {:?}",
            duration
        );

        println!(
            "Highly connected graph (10k nodes, 200k edges) analyzed in {:?}",
            duration
        );
    }

    #[test]
    #[ignore] // Run with: cargo test stress_test_performance_scaling -- --ignored
    fn stress_test_performance_scaling() {
        // Test that performance scales appropriately with different job counts
        let metrics = create_large_codebase_metrics(500, 20); // 10,000 functions
        let call_graph = create_complex_call_graph(&metrics);

        let mut results = Vec::new();

        for jobs in [1, 2, 4, 8, 16] {
            let options = ParallelUnifiedAnalysisOptions {
                parallel: true,
                jobs: Some(jobs),
                batch_size: 100,
                progress: false,
            };

            let mut builder = ParallelUnifiedAnalysisBuilder::new(call_graph.clone(), options);

            let start = Instant::now();

            let (data_flow, _purity, test_funcs, debt_agg) =
                builder.execute_phase1_parallel(&metrics, None);
            let _items = builder.execute_phase2_parallel(
                &metrics,
                &test_funcs,
                &debt_agg,
                &data_flow,
                None,
                &Default::default(),
                None,
            );
            builder.execute_phase3_parallel(&metrics, None, false);

            let duration = start.elapsed();
            results.push((jobs, duration));

            println!("Jobs: {}, Time: {:?}", jobs, duration);
        }

        // Verify that more jobs generally means faster analysis
        // (though not necessarily linear due to overhead)
        let single_thread_time = results[0].1;
        let multi_thread_time = results[2].1; // 4 threads

        assert!(
            multi_thread_time < single_thread_time,
            "Parallel execution should be faster than sequential"
        );

        // Should see at least 2x speedup with 4 threads
        let speedup = single_thread_time.as_secs_f64() / multi_thread_time.as_secs_f64();
        assert!(
            speedup > 2.0,
            "Expected at least 2x speedup with 4 threads, got {:.2}x",
            speedup
        );
    }
}
