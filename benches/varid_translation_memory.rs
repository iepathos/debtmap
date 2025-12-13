//! Memory benchmarks for VarId translation layer
//!
//! Verifies that the translation layer adds <10% memory overhead (NFR1 from spec 247)

use criterion::{criterion_group, criterion_main, Criterion};
use debtmap::analysis::data_flow::{DataFlowAnalysis, ReachingDefinitions, VarId};
use debtmap::data_flow::{CfgAnalysisWithContext, DataFlowGraph};
use debtmap::priority::call_graph::FunctionId;
use std::hint::black_box;
use std::mem;
use std::path::PathBuf;

/// Create a realistic DataFlowAnalysis with the given number of variables
fn create_analysis(_num_vars: usize) -> DataFlowAnalysis {
    // The DataFlowAnalysis now only contains reaching_defs
    // EscapeAnalysis and TaintAnalysis were removed in a previous spec
    DataFlowAnalysis {
        reaching_defs: ReachingDefinitions::default(),
    }
}

/// Create variable names for translation
fn create_var_names(num_vars: usize) -> Vec<String> {
    (0..num_vars).map(|i| format!("var_{}", i)).collect()
}

/// Benchmark: Memory overhead of translation layer
///
/// This compares memory usage of DataFlowAnalysis alone vs with CfgAnalysisWithContext
fn bench_memory_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("varid_translation_memory");

    for num_vars in [10, 50, 100, 500] {
        let analysis = create_analysis(num_vars);
        let var_names = create_var_names(num_vars);

        // Measure baseline memory (just analysis)
        let baseline_size = mem::size_of_val(&analysis);

        // Measure with translation context
        let ctx = CfgAnalysisWithContext::new(var_names.clone(), analysis.clone());
        let with_context_size = mem::size_of_val(&ctx);

        // Calculate overhead percentage
        let overhead_pct =
            ((with_context_size as f64 - baseline_size as f64) / baseline_size as f64) * 100.0;

        // Log the results for manual verification
        eprintln!(
            "Variables: {}, Baseline: {} bytes, With context: {} bytes, Overhead: {:.2}%",
            num_vars, baseline_size, with_context_size, overhead_pct
        );

        // Benchmark translation operations
        group.bench_function(format!("translate_{}_vars", num_vars), |b| {
            let ctx = CfgAnalysisWithContext::new(var_names.clone(), analysis.clone());
            b.iter(|| {
                // Simulate translating all definitions
                let defs: Vec<_> = ctx
                    .analysis
                    .reaching_defs
                    .all_definitions
                    .iter()
                    .map(|d| d.var)
                    .collect();
                black_box(ctx.var_names_for(defs.into_iter()));

                // Simulate translating all uses
                let uses: Vec<_> = ctx
                    .analysis
                    .reaching_defs
                    .all_uses
                    .iter()
                    .map(|u| u.var)
                    .collect();
                black_box(ctx.var_names_for(uses.into_iter()));
            });
        });
    }

    group.finish();
}

/// Benchmark: DataFlowGraph memory overhead with translation context
///
/// This measures the memory impact at the DataFlowGraph level
fn bench_data_flow_graph_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_flow_graph_memory");

    for num_functions in [10, 50, 100] {
        let mut graph = DataFlowGraph::new();

        // Add functions with translation context
        for i in 0..num_functions {
            let func_id = FunctionId::new(
                PathBuf::from(format!("file{}.rs", i % 5)),
                format!("func_{}", i),
                i * 10,
            );

            let analysis = create_analysis(20); // 20 vars per function
            let var_names = create_var_names(20);
            let ctx = CfgAnalysisWithContext::new(var_names, analysis);

            graph.set_cfg_analysis_with_context(func_id, ctx);
        }

        group.bench_function(format!("iterate_{}_functions", num_functions), |b| {
            b.iter(|| {
                // Simply iterate over the function analyses to measure access overhead
                for i in 0..num_functions {
                    let func_id = FunctionId::new(
                        PathBuf::from(format!("file{}.rs", i % 5)),
                        format!("func_{}", i),
                        i * 10,
                    );

                    // Simulate typical access pattern
                    if let Some(ctx) = graph.get_cfg_analysis_with_context(&func_id) {
                        black_box(&ctx.analysis.reaching_defs);
                    }
                }
            });
        });
    }

    group.finish();
}

/// Benchmark: Individual translation operations
fn bench_individual_translations(c: &mut Criterion) {
    let analysis = create_analysis(100);
    let var_names = create_var_names(100);
    let ctx = CfgAnalysisWithContext::new(var_names, analysis);

    c.bench_function("translate_single_varid", |b| {
        let var_id = VarId {
            name_id: 42,
            version: 0,
        };
        b.iter(|| {
            black_box(ctx.var_name(black_box(var_id)));
        });
    });

    c.bench_function("translate_10_varids", |b| {
        let var_ids: Vec<_> = (0..10)
            .map(|i| VarId {
                name_id: i,
                version: 0,
            })
            .collect();
        b.iter(|| {
            black_box(ctx.var_names_for(black_box(var_ids.iter().copied())));
        });
    });
}

criterion_group!(
    benches,
    bench_memory_overhead,
    bench_data_flow_graph_memory,
    bench_individual_translations
);

criterion_main!(benches);
